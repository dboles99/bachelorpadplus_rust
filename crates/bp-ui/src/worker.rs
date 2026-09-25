//! Work that must not run on the UI thread (release plan M1-07).
//!
//! Slint draws, and answers every key, on the one thread that runs its event
//! loop. Anything slow done there -- walking a folder of five thousand files
//! for Find in Files -- freezes the window for as long as it takes, with no
//! way to say so or to stop it.
//!
//! A [`Background`] runs one kind of job on a thread of its own. Three
//! things make that safe to use from the UI:
//!
//! * **Generations.** Each job is numbered, and only the newest job's result
//!   is ever handed back. A search superseded by another cannot land its
//!   stale hits over the new one's.
//! * **A stop flag** the job is given and should check. Starting a new job,
//!   or calling [`Background::stop`], raises it for the one before.
//! * **Results by channel, collected on the UI thread** ([`Background::take`]).
//!   Nothing crosses back by itself, so the application state -- which lives
//!   in an `Rc<RefCell<..>>` and is not `Send` -- is only ever touched on the
//!   thread that owns it. A UI timer polls while a job is running; the tests
//!   poll directly, with no event loop at all.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};

/// One kind of background job, newest wins.
pub(crate) struct Background<T> {
    sender: Sender<(u64, T)>,
    receiver: Receiver<(u64, T)>,
    /// The newest job's number. Results from any other are dropped.
    generation: u64,
    /// The newest job's stop flag, while it runs.
    running: Option<Arc<AtomicBool>>,
}

impl<T: Send + 'static> Background<T> {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            sender,
            receiver,
            generation: 0,
            running: None,
        }
    }

    /// Start `job` on a thread of its own, stopping whatever job came
    /// before. `job` is handed a flag that becomes true when it should give
    /// up; checking it is the job's business.
    pub(crate) fn start(&mut self, job: impl FnOnce(&AtomicBool) -> T + Send + 'static) {
        self.stop();
        self.generation += 1;
        let generation = self.generation;
        let stop = Arc::new(AtomicBool::new(false));
        self.running = Some(Arc::clone(&stop));
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = job(&stop);
            // Nobody listening means the window has gone; nothing to do.
            let _ = sender.send((generation, result));
        });
    }

    /// Ask the running job, if any, to give up. Its result, if it still
    /// sends one, is dropped: it is no longer the newest.
    pub(crate) fn stop(&mut self) {
        if let Some(flag) = self.running.take() {
            flag.store(true, Ordering::Relaxed);
            // Moving on a generation is what drops the late result.
            self.generation += 1;
        }
    }

    /// Whether a job has been started and has not yet been collected.
    pub(crate) fn busy(&self) -> bool {
        self.running.is_some()
    }

    /// The newest job's result, if it has arrived. Anything older is
    /// thrown away on the way past.
    pub(crate) fn take(&mut self) -> Option<T> {
        let mut newest = None;
        while let Ok((generation, result)) = self.receiver.try_recv() {
            if generation == self.generation {
                newest = Some(result);
            }
        }
        if newest.is_some() {
            self.running = None;
        }
        newest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Poll until a result arrives or a generous deadline passes.
    fn wait<T: Send + 'static>(background: &mut Background<T>) -> Option<T> {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if let Some(result) = background.take() {
                return Some(result);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        None
    }

    #[test]
    fn a_job_runs_off_this_thread_and_its_result_comes_back() {
        let mut background = Background::new();
        let here = std::thread::current().id();
        background.start(move |_| std::thread::current().id() != here);
        assert!(background.busy());
        assert_eq!(wait(&mut background), Some(true));
        assert!(!background.busy());
    }

    #[test]
    fn a_newer_job_wins_and_the_older_one_is_told_to_stop() {
        let mut background = Background::new();
        let (release, gate) = std::sync::mpsc::channel::<()>();
        let (told, heard) = std::sync::mpsc::channel::<bool>();
        background.start(move |stop| {
            // Held until the second job has been started.
            let _ = gate.recv();
            let _ = told.send(stop.load(Ordering::Relaxed));
            "old"
        });
        background.start(|_| "new");
        let _ = release.send(());

        assert_eq!(wait(&mut background), Some("new"));
        assert_eq!(heard.recv_timeout(Duration::from_secs(10)), Ok(true));
        // The old job's late result never surfaces.
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(background.take(), None);
    }

    #[test]
    fn a_stopped_job_hands_nothing_back() {
        let mut background = Background::new();
        let (release, gate) = std::sync::mpsc::channel::<()>();
        background.start(move |_| {
            let _ = gate.recv();
            1
        });
        background.stop();
        assert!(!background.busy());
        let _ = release.send(());
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(background.take(), None);
    }
}
