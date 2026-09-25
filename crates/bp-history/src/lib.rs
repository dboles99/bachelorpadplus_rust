//! Crash-safe recovery journal.
//!
//! specs.md section 6: an edit journal and session recovery, so that work
//! survives a crash or a power cut between saves.
//!
//! **This writes document content to disk.** That is the entire point, and it
//! is also the thing to be careful about: a recovery journal is a copy of
//! your unsaved work sitting outside the file you were editing.
//!
//! The document's security profile governs it (ADR-0011, ADR-0020), and
//! `checkpoint` takes that policy rather than assuming one. Two things follow
//! that are easy to miss:
//!
//! * **A refusal deletes what is already there.** Tightening a profile has to
//!   remove what the looser one wrote, or the journal would report itself as
//!   protected while this morning's plaintext sat beside it.
//! * **An encrypted journal is sealed with the document's own passphrase**
//!   and filed under a digest of its path (ADR-0022), so it is recovered at
//!   *unlock* time -- nothing prompts for a passphrase at startup, because a
//!   program asking for one unbidden is the habit phishing depends on. A
//!   document with no passphrase gets no journal, and is told what fixes
//!   that.
//!
//! Under the default profile the journal is plaintext in the user's own
//! config directory, and `Journal::location` exists so the UI can say where.
//!
//! Two properties matter more than speed here:
//!
//! * A checkpoint is written atomically, so a crash *during* a checkpoint
//!   cannot corrupt the previous one.
//! * A journal is discarded only after a successful save. Deleting it any
//!   earlier would open a window in which neither the file nor the journal
//!   holds the work.
//!
//! Several copies of the program share one recovery directory, so each
//! journal owns a directory of its own inside it, locked for as long as the
//! journal lives. Another instance's checkpoints are recoverable only once
//! that lock is free -- that is, once the instance is gone -- and are never
//! offered or deleted while it is running. `Journal` explains why a lock and
//! not a pid. What this crate does not do is decide *what to ask the user*:
//! it reports what is recoverable and what is held elsewhere, and the shell
//! words the prompt.
//!
//! [`profiles`] is the other thing this crate keeps per document between
//! sessions: the security profile each document was given, filed under the
//! same path digest as the sealed journal, so that a Confidential document
//! reopens Confidential rather than lapsing to Standard.

#![forbid(unsafe_code)]

pub mod profiles;

pub use profiles::{DocumentKey, ProfileStore};

use std::collections::BTreeMap;
use std::collections::hash_map::RandomState;
use std::fs::{File, OpenOptions, TryLockError};
use std::hash::{BuildHasher, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use bp_security::Recovery;
use serde::{Deserialize, Serialize};

/// Crate identity used by workspace smoke tests and diagnostics.
pub const CRATE_NAME: &str = "bp-history";

/// One document's unsaved state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// The file this belonged to, if it had one. `None` for an unsaved
    /// document that never had a name.
    pub path: Option<PathBuf>,
    /// A label for the recovery prompt, so the user is not asked about
    /// "document 3".
    pub name: String,
    pub text: String,
    /// Seconds since the Unix epoch. A plain integer rather than a timestamp
    /// type so the journal format does not depend on a crate's serialisation.
    pub written_at: u64,
}

/// What a checkpoint attempt did.
///
/// More than a `bool`, because "not written" has several meanings and only
/// some are worth telling the user about: a profile that forbids a journal is
/// working as asked, while one that cannot journal because the document has
/// no passphrase is something they can fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Written {
    Yes,
    Refused(Refusal),
}

/// Why a checkpoint was not written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The document's profile disables the recovery journal outright.
    ProfileForbidsIt,
    /// The profile requires an encrypted journal and this document has no
    /// passphrase, because it is not an encrypted document.
    ///
    /// The journal is sealed with the document's own passphrase (ADR-0022),
    /// so a plaintext document under a strict profile has no key to use. The
    /// answer is actionable, which is why it carries a notice: encrypting the
    /// document turns recovery back on.
    NoPassphrase,
    /// The profile requires an encrypted journal and the document has no
    /// stable name to file one under -- it has never been saved.
    ///
    /// An unsaved document's journal is keyed by nothing, so a later session
    /// could not find it even with the passphrase. Refusing is honest;
    /// writing one would be work the user could never get back.
    NeverSaved,
}

impl Refusal {
    /// What to tell the user, if anything.
    ///
    /// A profile doing what it was set to do is not news. A profile that
    /// cannot be honoured is, and staying silent about it would leave
    /// somebody believing their work is being journalled when it is not.
    #[must_use]
    pub const fn notice(self) -> Option<&'static str> {
        match self {
            Self::ProfileForbidsIt => None,
            // Both of these are actionable, which is the test for whether
            // they are worth interrupting the user about.
            Self::NoPassphrase => Some(
                "this profile keeps the recovery journal encrypted -- encrypt \
                 this document (Security menu) to turn recovery back on",
            ),
            Self::NeverSaved => Some(
                "this profile cannot journal a document that has never been \
                 saved; save it first",
            ),
        }
    }
}

/// Where checkpoints are kept, and which of them are this instance's to
/// manage.
///
/// # Instances
///
/// Every running copy of the program shares one recovery directory, and a
/// second copy is the ordinary case rather than an exotic one: registering
/// as the default editor means double-clicking a second file starts a second
/// process. Before instances existed, the second process read the first
/// one's live checkpoints as a crashed session's work, offered to "recover"
/// them, and deleted them when the offer was declined. Document ids restart
/// at 1 in every process as well, so even a session that had genuinely
/// ended had its checkpoint 1 overwritten or discarded by the next session's
/// document 1.
///
/// So each journal writes into a directory of its own,
/// `instance-<pid>-<nonce>/`, and holds an exclusive lock on the sibling file
/// `instance-<pid>-<nonce>.lock` for as long as it lives. The lock is taken
/// at the first *write*, not at construction: a journal that has written
/// nothing has nothing to protect, and keeping `new` free of side effects
/// means constructing one -- which the shell and its tests do freely --
/// never creates a directory.
///
/// Whether another instance is alive is asked of the operating system rather
/// than inferred. `std` cannot portably say whether a pid is running without
/// `unsafe`, and a pid would be the wrong question anyway: pids are reused,
/// so a crashed session's pid may belong to an unrelated process by the next
/// launch. What `std` can say, on both platforms, is whether a file lock is
/// held -- `File::try_lock` is `flock` on Linux and `LockFileEx` on
/// Windows -- and both kernels release that lock when its holder exits,
/// however it exits. A lock that can be taken therefore belonged to an
/// instance that is gone, and its checkpoints are recoverable. The pid and
/// start time written beside the lock are for a person reading the
/// directory, never for the decision.
///
/// Offering a dead instance's work *claims* it: [`Journal::pending`] keeps
/// the dead instance's lock until the work is discarded, the claim is
/// released with [`Journal::keep_for_later`], or this journal is dropped.
/// That settles which window offers a crashed session's work when two start
/// at once -- selecting several files and pressing Enter does exactly that --
/// so one of them cannot delete what the other has just restored. Every
/// other call takes such a lock only for its own duration: a window that has
/// been open all day must not quietly hoard the work of one that crashed an
/// hour ago, which it will never offer, from the window the user opens next.
///
/// Where the answer is unknowable -- a file system that does not support
/// locking, or a lock file that cannot be opened -- the other instance is
/// treated as alive. That errs towards offering too little rather than
/// deleting too much: work that is not offered is still on disk, while work
/// that is deleted is gone.
///
/// Checkpoints written before instances existed sit directly in the
/// directory. They are recoverable like a dead instance's, claimed through
/// `legacy.lock`; a copy of the *old* program still running would not honour
/// that lock, which is a one-release transition risk rather than a design.
#[derive(Debug, Clone)]
pub struct Journal {
    dir: PathBuf,
    /// Shared by clones: a clone is the same instance, not a second one.
    instance: Arc<Mutex<Instance>>,
}

/// This journal's identity and the locks it holds.
#[derive(Debug)]
struct Instance {
    /// `instance-<pid>-<nonce>`: names both the directory and its lock.
    name: String,
    /// The lock on this instance's own lock file, once something has been
    /// written. Dropping it -- or the process ending -- is what makes this
    /// instance's checkpoints recoverable by the next one.
    own: Option<File>,
    /// Dead owners' checkpoints this journal has claimed, keyed by the path
    /// of their lock file.
    claimed: BTreeMap<PathBuf, Claim>,
}

#[derive(Debug)]
struct Claim {
    /// Held for the claim's lifetime; the value is never read.
    _lock: File,
    place: Place,
}

/// A directory of checkpoints and the lock that says whose they are.
#[derive(Debug, Clone)]
struct Place {
    files: PathBuf,
    lock: PathBuf,
    /// The journal's own root, holding checkpoints from before instances.
    /// Its files may be removed but the directory itself never is.
    legacy: bool,
}

/// Every place with checkpoints in it, sorted by who may touch them.
struct Survey {
    /// This instance's own directory first, then everything it has claimed.
    mine: Vec<Place>,
    /// Places held by an instance that is still running.
    elsewhere: Vec<Place>,
    /// Lock files claimed by this survey rather than before it.
    fresh: Vec<PathBuf>,
}

const INSTANCE_PREFIX: &str = "instance-";
const LEGACY_LOCK: &str = "legacy.lock";
/// Diagnostic only: see the type-level docs for why liveness is never
/// decided from it.
const OWNER_FILE: &str = "owner";

impl Journal {
    /// Use `dir` for checkpoints.
    ///
    /// Touches nothing on disk. The instance lock is taken by the first
    /// checkpoint actually written.
    pub fn new(dir: PathBuf) -> Self {
        let instance = Instance {
            name: instance_name(),
            own: None,
            claimed: BTreeMap::new(),
        };
        Self {
            dir,
            instance: Arc::new(Mutex::new(instance)),
        }
    }

    /// The directory checkpoints are written to, for showing the user.
    pub fn location(&self) -> &Path {
        &self.dir
    }

    /// The instance state, even if a panic elsewhere poisoned the mutex:
    /// the map of held locks is still accurate, and refusing to journal
    /// because of an unrelated panic would cost the user their recovery.
    fn instance(&self) -> MutexGuard<'_, Instance> {
        self.instance.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn own_dir(&self) -> PathBuf {
        self.dir.join(&self.instance().name)
    }

    /// This instance's directory, taking its lock first if this is the
    /// first write.
    ///
    /// The lock file is created before the directory, and a surveying
    /// instance finds owners by their directories, so it can never see a
    /// directory whose lock nobody has yet had the chance to take.
    fn ensure_own(&self) -> std::io::Result<PathBuf> {
        let mut instance = self.instance();
        let own_dir = self.dir.join(&instance.name);
        if instance.own.is_some() {
            return Ok(own_dir);
        }
        std::fs::create_dir_all(&self.dir)?;
        let lock_path = self.dir.join(format!("{}.lock", instance.name));
        let lock = open_lock(&lock_path)?;
        match lock.try_lock() {
            Ok(()) => instance.own = Some(lock),
            // Somebody else holds a lock named after this instance: a nonce
            // collision, or someone meddling. Writing into a directory
            // another instance may discard would be the bug this exists to
            // prevent.
            Err(TryLockError::WouldBlock) => {
                return Err(std::io::Error::other(
                    "another instance holds this journal's lock",
                ));
            }
            // A file system without locks. Journalling unlocked keeps the
            // work on disk; other instances will treat it as held elsewhere
            // (see the type docs), which never deletes it.
            Err(TryLockError::Error(_)) => {}
        }
        std::fs::create_dir_all(&own_dir)?;
        let owner = format!(
            "pid {}\nstarted {}\ninstance {}\n",
            std::process::id(),
            now_unix(),
            instance.name
        );
        std::fs::write(own_dir.join(OWNER_FILE), owner)?;
        Ok(own_dir)
    }

    /// Sort every place with checkpoints into this instance's own, those it
    /// can claim, and those another running instance holds.
    ///
    /// **Looking is claiming.** A dead instance's lock that can be taken is
    /// kept, so that the answer does not change between this call and
    /// whatever the caller does with it. Only [`Journal::pending`] keeps it
    /// beyond that; everything else goes through [`Journal::glance`].
    fn survey(&self) -> Survey {
        let mut instance = self.instance();
        let own = Place {
            files: self.dir.join(&instance.name),
            lock: self.dir.join(format!("{}.lock", instance.name)),
            legacy: false,
        };
        let mut survey = Survey {
            mine: vec![own],
            elsewhere: Vec::new(),
            fresh: Vec::new(),
        };
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return survey;
        };

        let mut legacy = false;
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
            if !is_dir {
                legacy |= is_checkpoint_file(&path) || is_temporary(&path);
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !name.starts_with(INSTANCE_PREFIX) || name == instance.name {
                continue;
            }
            let place = Place {
                lock: self.dir.join(format!("{name}.lock")),
                files: path,
                legacy: false,
            };
            instance.sort(place, &mut survey);
        }
        if legacy {
            let place = Place {
                files: self.dir.clone(),
                lock: self.dir.join(LEGACY_LOCK),
                legacy: true,
            };
            instance.sort(place, &mut survey);
        }
        survey
    }

    /// Survey, act, and release whatever the survey newly claimed.
    fn glance<T>(&self, act: impl FnOnce(&Survey) -> T) -> T {
        let survey = self.survey();
        let result = act(&survey);
        let mut instance = self.instance();
        for lock in &survey.fresh {
            instance.claimed.remove(lock);
        }
        result
    }

    /// Release claims on places that no longer hold any checkpoint, removing
    /// their directories and lock files.
    ///
    /// The directory goes before the lock is released, so no other instance
    /// can claim a place in the middle of its removal.
    fn tidy(&self) {
        let mut instance = self.instance();
        let empty: Vec<PathBuf> = instance
            .claimed
            .iter()
            .filter(|(_, claim)| !has_checkpoints(&claim.place.files))
            .map(|(lock, _)| lock.clone())
            .collect();
        for lock in empty {
            if let Some(claim) = instance.claimed.remove(&lock) {
                // Best effort throughout: an empty place left behind is
                // litter, not lost work, and the next survey tidies it.
                if !claim.place.legacy {
                    let _ = std::fs::remove_dir_all(&claim.place.files);
                }
                let _ = std::fs::remove_file(&lock);
                drop(claim);
            }
        }
    }

    fn file_for(&self, id: u64) -> PathBuf {
        self.own_dir().join(format!("{id}.json"))
    }

    /// Write a checkpoint for `id` if the document's policy permits it.
    ///
    /// Atomic: written to a temporary file and renamed, so a crash midway
    /// leaves the *previous* checkpoint intact rather than a truncated one.
    /// A journal that can be half-written is worse than no journal, because
    /// it looks recoverable.
    ///
    /// `id` is this instance's own document id, and it is only ever matched
    /// against this instance's checkpoints: another instance's document 1,
    /// live or dead, is out of reach.
    ///
    /// **A refusal deletes any checkpoint already on disk for `id`.** That is
    /// the whole point of asking: tightening a document's profile has to
    /// remove what the looser one wrote, or moving to Confidential would
    /// leave this morning's plaintext sitting in the recovery directory and
    /// report itself as protected.
    ///
    /// [`Recovery::Encrypted`] seals the checkpoint with `passphrase`, which
    /// is the document's own. Without one it is refused rather than
    /// downgraded: writing plaintext for a user who has been told it is
    /// encrypted is worse than keeping no journal at all (ADR-0020).
    ///
    /// The two forms are kept in step in *both* directions. Tightening a
    /// profile removes the plaintext journal, and loosening one removes the
    /// sealed journal -- the second is the easier to forget, and a sealed
    /// copy left behind would be offered at the next unlock holding older
    /// work.
    pub fn checkpoint(
        &self,
        id: u64,
        checkpoint: &Checkpoint,
        recovery: Recovery,
        passphrase: Option<&str>,
    ) -> std::io::Result<Written> {
        match recovery {
            Recovery::Plaintext => {}
            Recovery::Disabled => {
                self.discard(id)?;
                self.discard_sealed(checkpoint.path.as_deref())?;
                return Ok(Written::Refused(Refusal::ProfileForbidsIt));
            }
            Recovery::Encrypted => {
                // The plaintext form must go whatever happens next: this
                // document was journalled in clear under a looser profile,
                // and tightening it has to remove that.
                self.discard(id)?;
                // Passphrase first, path second. For a document that is
                // neither saved nor encrypted both are true, and "encrypt
                // this document" is the more useful thing to say -- Encrypt
                // Document is a Save As, so it fixes the missing path too.
                let Some(passphrase) = passphrase else {
                    self.discard_sealed(checkpoint.path.as_deref())?;
                    return Ok(Written::Refused(Refusal::NoPassphrase));
                };
                let Some(path) = checkpoint.path.as_deref() else {
                    return Ok(Written::Refused(Refusal::NeverSaved));
                };
                return self.checkpoint_sealed(path, checkpoint, passphrase);
            }
        }
        // A document journalled in clear must not also leave a sealed copy
        // from when its profile was stricter.
        self.discard_sealed(checkpoint.path.as_deref())?;
        let own_dir = self.ensure_own()?;
        let json = serde_json::to_vec_pretty(checkpoint)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let target = own_dir.join(format!("{id}.json"));
        let temp = target.with_extension("json.tmp");
        std::fs::write(&temp, &json)?;
        std::fs::rename(&temp, &target)?;
        Ok(Written::Yes)
    }

    /// Seal a checkpoint with the document's own passphrase.
    ///
    /// The whole `Checkpoint` goes inside the ciphertext, including the path
    /// and the name. Writing either in clear beside it would leave a list of
    /// which documents have unsaved work sitting in the recovery directory --
    /// close to the thing the strict profiles exist to prevent (ADR-0022).
    ///
    /// A dead instance's sealed copy for the same path is superseded once
    /// this one is safely on disk. The document was unlocked to get the
    /// passphrase, and unlocking is when that copy was offered, so keeping
    /// it would only offer older work at the next unlock. Only after the
    /// rename, so there is no moment with neither.
    fn checkpoint_sealed(
        &self,
        path: &Path,
        checkpoint: &Checkpoint,
        passphrase: &str,
    ) -> std::io::Result<Written> {
        let own_dir = self.ensure_own()?;
        let json = serde_json::to_vec(checkpoint)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let sealed = bp_crypto::seal(&json, passphrase, bp_crypto::SealOptions::default())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

        let name = sealed_name(path);
        let target = own_dir.join(&name);
        let temp = target.with_extension("bpadx.tmp");
        std::fs::write(&temp, &sealed)?;
        std::fs::rename(&temp, &target)?;

        self.glance(|survey| {
            for place in survey.mine.iter().skip(1) {
                remove_if_present(&place.files.join(&name))?;
            }
            self.tidy();
            Ok(Written::Yes)
        })
    }

    /// Remove the sealed checkpoint for `path`, if there is one, from this
    /// instance and from every dead instance -- never from a live one.
    fn discard_sealed(&self, path: Option<&Path>) -> std::io::Result<()> {
        let Some(path) = path else {
            return Ok(());
        };
        let name = sealed_name(path);
        self.glance(|survey| {
            for place in &survey.mine {
                remove_if_present(&place.files.join(&name))?;
            }
            self.tidy();
            Ok(())
        })
    }

    /// The unsaved work waiting for `path`, if any, given its passphrase.
    ///
    /// Called at unlock time: the user opens a `.bpadx`, gives its
    /// passphrase, and only then can this be read. That is why nothing
    /// prompts for a passphrase at startup -- a program asking for one
    /// unprompted is the habit that makes phishing work.
    ///
    /// Looks in this instance's journal and every dead instance's, and
    /// returns the newest. A live instance's copy is that window's work in
    /// progress, not something to recover.
    ///
    /// A wrong passphrase and a damaged journal are indistinguishable here
    /// for the same reason they are in `bp-crypto`, and neither is worth
    /// reporting: the caller has just successfully opened the document with
    /// this passphrase, so a failure means the journal is damaged, and a
    /// damaged journal is simply no journal.
    #[must_use]
    pub fn sealed_pending(&self, path: &Path, passphrase: &str) -> Option<Checkpoint> {
        let name = sealed_name(path);
        self.glance(|survey| {
            survey
                .mine
                .iter()
                .filter_map(|place| {
                    let bytes = std::fs::read(place.files.join(&name)).ok()?;
                    let plain = bp_crypto::open(&bytes, passphrase).ok()?;
                    serde_json::from_slice::<Checkpoint>(&plain).ok()
                })
                .max_by_key(|c| c.written_at)
        })
    }

    /// Forget the sealed checkpoint for `path`. Public because the shell
    /// discards it after a successful save, like the plaintext one.
    ///
    /// A live instance's copy is left alone: it is that window's to discard.
    pub fn discard_sealed_for(&self, path: &Path) -> std::io::Result<()> {
        self.discard_sealed(Some(path))
    }

    /// How many sealed checkpoints are waiting.
    ///
    /// Their contents cannot be read without the documents' passphrases, so
    /// this is a count and nothing more -- enough to tell the user that
    /// something is waiting, not enough to say what. Those held by another
    /// running instance are counted by [`Journal::held_elsewhere`] instead.
    #[must_use]
    pub fn sealed_count(&self) -> usize {
        self.glance(|survey| {
            survey
                .mine
                .iter()
                .map(|place| count_matching(&place.files, is_sealed_file))
                .sum()
        })
    }

    /// How many checkpoints, plain and sealed, belong to another instance
    /// that is still running.
    ///
    /// Not offered by [`Journal::pending`] and never deleted from here: they
    /// are documents open in another window, and the most the shell should
    /// do is say so.
    #[must_use]
    pub fn held_elsewhere(&self) -> usize {
        self.glance(|survey| {
            survey
                .elsewhere
                .iter()
                .map(|place| count_matching(&place.files, is_checkpoint_file))
                .sum()
        })
    }

    /// Forget this instance's checkpoint for `id`.
    ///
    /// Call this only after a successful save. Earlier, and there is a window
    /// where neither the file nor the journal holds the work.
    ///
    /// Only this instance's own: a dead instance's document with the same id
    /// is a different document, kept until the user decides about it.
    pub fn discard(&self, id: u64) -> std::io::Result<()> {
        remove_if_present(&self.file_for(id))
    }

    /// Every recoverable checkpoint, newest first: this instance's own and
    /// every dead instance's, but never a live instance's.
    ///
    /// The id is the one the owning session used, so ids from different
    /// sessions can repeat. It identifies nothing in *this* session, and
    /// passing it to [`Journal::discard`] would not reach a dead session's
    /// file -- that is what [`Journal::discard_recovered`] is for.
    ///
    /// Unreadable or corrupt entries are skipped rather than failing the
    /// whole scan: one bad file must not cost the user the other recoveries.
    pub fn pending(&self) -> Vec<(u64, Checkpoint)> {
        let mut found: Vec<(u64, Checkpoint)> = self
            .survey()
            .mine
            .iter()
            .flat_map(|place| read_checkpoints(&place.files))
            .collect();

        // Newest first: `Reverse` rather than a hand-rolled comparator, which
        // clippy rightly points out is the same thing with more rope.
        found.sort_by_key(|(_, c)| std::cmp::Reverse(c.written_at));
        found
    }

    /// Remove every plaintext checkpoint this instance may: its own and
    /// every dead instance's. Used when the user declines recovery.
    ///
    /// A live instance's checkpoints are untouched -- declining to recover
    /// a crash is not a decision about a window that is still open. Sealed
    /// checkpoints are untouched too, as before: they are recovered at
    /// unlock, not at startup.
    pub fn discard_all(&self) -> std::io::Result<()> {
        self.glance(|survey| self.discard_plaintext(&survey.mine))
    }

    /// Remove dead instances' plaintext checkpoints, keeping this instance's
    /// own.
    ///
    /// For after the user accepts recovery: restore what [`Journal::pending`]
    /// returned, checkpoint the restored documents under this instance, and
    /// only then call this. Discarding before that checkpoint would leave the
    /// work only in memory; never discarding would offer it again at every
    /// launch.
    pub fn discard_recovered(&self) -> std::io::Result<()> {
        self.glance(|survey| self.discard_plaintext(&survey.mine[1..]))
    }

    /// Let go of the dead instances' work [`Journal::pending`] offered,
    /// without discarding it -- the answer for "not now".
    ///
    /// The work stays on disk, and the next window to start offers it again,
    /// even while this one is still open. Holding the claim would leave that
    /// window reporting it as open elsewhere, which it is not.
    pub fn keep_for_later(&self) {
        self.instance().claimed.clear();
    }

    fn discard_plaintext(&self, places: &[Place]) -> std::io::Result<()> {
        for place in places {
            let Ok(entries) = std::fs::read_dir(&place.files) else {
                continue;
            };
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if checkpoint_id(&path).is_some() {
                    remove_if_present(&path)?;
                }
            }
        }
        self.tidy();
        Ok(())
    }

    /// Delete stray temporary files left by a crash mid-checkpoint -- this
    /// instance's and dead instances', never a live instance's, whose
    /// temporary file may be a checkpoint halfway through being written.
    pub fn clean_temporaries(&self) {
        self.glance(|survey| {
            for place in &survey.mine {
                let Ok(entries) = std::fs::read_dir(&place.files) else {
                    continue;
                };
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if is_temporary(&path) {
                        let _ = std::fs::remove_file(path);
                    }
                }
            }
        });
    }
}

impl Instance {
    /// Claim `place` if its owner is gone; otherwise note it as held.
    fn sort(&mut self, place: Place, survey: &mut Survey) {
        if self.claimed.contains_key(&place.lock) {
            survey.mine.push(place);
            return;
        }
        // Opening creates the lock file if it is missing. An owner creates
        // its lock before its directory, so a directory without one was
        // left half-removed or half-made by a crash, and either way nobody
        // is running there.
        let Ok(lock) = open_lock(&place.lock) else {
            survey.elsewhere.push(place);
            return;
        };
        match lock.try_lock() {
            Ok(()) => {
                let claim = Claim {
                    _lock: lock,
                    place: place.clone(),
                };
                self.claimed.insert(place.lock.clone(), claim);
                survey.fresh.push(place.lock.clone());
                survey.mine.push(place);
            }
            // Held by a running instance -- or unknowable, which is treated
            // the same way for the reason in `Journal`'s docs.
            Err(TryLockError::WouldBlock | TryLockError::Error(_)) => {
                survey.elsewhere.push(place);
            }
        }
    }
}

/// A name no other instance, in this process or another, will choose.
///
/// The pid is there for a person reading the directory. The nonce is what
/// makes it unique: `RandomState` is seeded from the operating system's
/// randomness once per process, which is enough to separate processes whose
/// pids have been reused, and the counter separates journals in one process.
fn instance_name() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    hasher.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
    );
    format!(
        "{INSTANCE_PREFIX}{}-{:016x}",
        std::process::id(),
        hasher.finish()
    )
}

fn open_lock(path: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

/// The file name of the sealed checkpoint for `path`.
///
/// Named after a digest of the document's path rather than a session id,
/// because an encrypted journal is found again at *unlock* time -- the user
/// opens the `.bpadx`, gives its passphrase, and only then can this be read.
/// A session id would be meaningless by then.
///
/// The digest is not a secret. See `bp_crypto::stable_name`.
fn sealed_name(path: &Path) -> String {
    let key = bp_crypto::stable_name(path.to_string_lossy().as_bytes());
    format!("{key}.bpadx")
}

/// `Some(id)` for a plaintext checkpoint's file name, `<id>.json`.
fn checkpoint_id(path: &Path) -> Option<u64> {
    if path.extension()? != "json" {
        return None;
    }
    path.file_stem()?.to_str()?.parse().ok()
}

fn is_sealed_file(path: &Path) -> bool {
    path.extension().is_some_and(|x| x == "bpadx")
}

fn is_checkpoint_file(path: &Path) -> bool {
    checkpoint_id(path).is_some() || is_sealed_file(path)
}

fn is_temporary(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "tmp")
}

fn count_matching(dir: &Path, matches: fn(&Path) -> bool) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|e| matches(&e.path()))
        .count()
}

fn has_checkpoints(dir: &Path) -> bool {
    count_matching(dir, is_checkpoint_file) > 0
}

fn read_checkpoints(dir: &Path) -> Vec<(u64, Checkpoint)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let id = checkpoint_id(&path)?;
            let text = std::fs::read_to_string(&path).ok()?;
            let checkpoint: Checkpoint = serde_json::from_str(&text).ok()?;
            Some((id, checkpoint))
        })
        .collect()
}

fn remove_if_present(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        // Already gone is the desired state, not a failure.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Seconds since the Unix epoch, saturating at 0 if the clock is before it.
pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn journal() -> (TempDir, Journal) {
        let dir = TempDir::new().unwrap();
        let journal = Journal::new(dir.path().join("recovery"));
        (dir, journal)
    }

    fn checkpoint(text: &str) -> Checkpoint {
        Checkpoint {
            path: Some(PathBuf::from("/notes/a.txt")),
            name: "a.txt".to_owned(),
            text: text.to_owned(),
            written_at: now_unix(),
        }
    }

    #[test]
    fn a_checkpoint_round_trips() {
        let (_dir, journal) = journal();
        let entry = checkpoint("unsaved work");
        journal
            .checkpoint(1, &entry, Recovery::Plaintext, None)
            .unwrap();

        let pending = journal.pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0, 1);
        assert_eq!(pending[0].1, entry);
    }

    #[test]
    fn checkpointing_twice_replaces_rather_than_accumulates() {
        let (_dir, journal) = journal();
        journal
            .checkpoint(1, &checkpoint("first"), Recovery::Plaintext, None)
            .unwrap();
        journal
            .checkpoint(1, &checkpoint("second"), Recovery::Plaintext, None)
            .unwrap();

        let pending = journal.pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].1.text, "second");
    }

    #[test]
    fn discarding_is_idempotent() {
        let (_dir, journal) = journal();
        journal
            .checkpoint(1, &checkpoint("work"), Recovery::Plaintext, None)
            .unwrap();

        journal.discard(1).unwrap();
        assert!(journal.pending().is_empty());
        // Already gone is the desired state, not an error.
        journal.discard(1).unwrap();
    }

    #[test]
    fn pending_is_newest_first() {
        let (_dir, journal) = journal();
        let mut old = checkpoint("old");
        old.written_at = 1_000;
        let mut new = checkpoint("new");
        new.written_at = 2_000;

        journal
            .checkpoint(1, &old, Recovery::Plaintext, None)
            .unwrap();
        journal
            .checkpoint(2, &new, Recovery::Plaintext, None)
            .unwrap();

        let pending = journal.pending();
        assert_eq!(pending[0].1.text, "new");
        assert_eq!(pending[1].1.text, "old");
    }

    #[test]
    fn a_corrupt_entry_does_not_cost_the_others() {
        // One bad file must not lose the user their other recoveries.
        let (_dir, journal) = journal();
        journal
            .checkpoint(1, &checkpoint("good"), Recovery::Plaintext, None)
            .unwrap();
        std::fs::write(journal.location().join("2.json"), "{ not json").unwrap();
        std::fs::write(journal.location().join("notanumber.json"), "{}").unwrap();

        let pending = journal.pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].1.text, "good");
    }

    #[test]
    fn a_missing_directory_is_simply_nothing_to_recover() {
        let dir = TempDir::new().unwrap();
        let journal = Journal::new(dir.path().join("never-created"));
        assert!(journal.pending().is_empty());
        // And must not panic or create anything.
        assert!(!journal.location().exists());
    }

    #[test]
    fn checkpointing_creates_the_directory() {
        let (_dir, journal) = journal();
        assert!(!journal.location().exists());
        journal
            .checkpoint(1, &checkpoint("work"), Recovery::Plaintext, None)
            .unwrap();
        assert!(journal.location().exists());
    }

    #[test]
    fn a_crash_mid_checkpoint_leaves_the_previous_one_readable() {
        // The reason checkpoints are written to a temp file and renamed.
        let (_dir, journal) = journal();
        journal
            .checkpoint(1, &checkpoint("survivable"), Recovery::Plaintext, None)
            .unwrap();

        // Simulate the debris of an interrupted write.
        std::fs::write(journal.location().join("1.json.tmp"), "{ half writ").unwrap();

        let pending = journal.pending();
        assert_eq!(
            pending.len(),
            1,
            "the .tmp file must not be read as a checkpoint"
        );
        assert_eq!(pending[0].1.text, "survivable");

        journal.clean_temporaries();
        assert!(!journal.location().join("1.json.tmp").exists());
    }

    #[test]
    fn discard_all_clears_everything() {
        let (_dir, journal) = journal();
        journal
            .checkpoint(1, &checkpoint("a"), Recovery::Plaintext, None)
            .unwrap();
        journal
            .checkpoint(2, &checkpoint("b"), Recovery::Plaintext, None)
            .unwrap();

        journal.discard_all().unwrap();
        assert!(journal.pending().is_empty());
    }

    #[test]
    fn an_unnamed_document_can_still_be_recovered() {
        let (_dir, journal) = journal();
        let entry = Checkpoint {
            path: None,
            name: "Untitled".to_owned(),
            text: "notes with no file yet".to_owned(),
            written_at: now_unix(),
        };
        journal
            .checkpoint(7, &entry, Recovery::Plaintext, None)
            .unwrap();

        let pending = journal.pending();
        assert_eq!(pending[0].1.path, None);
        assert_eq!(pending[0].1.text, "notes with no file yet");
    }

    // --- instances --------------------------------------------------------
    //
    // Two `Journal`s in one test process stand for two running copies of the
    // program: the lock is per open file, not per process, on both
    // platforms. Dropping one stands for its process ending.

    /// Leave a crashed instance's directory behind, holding `checkpoints`,
    /// with or without its lock file.
    fn crashed_instance(
        location: &Path,
        name: &str,
        checkpoints: &[(u64, &Checkpoint)],
        lock: bool,
    ) {
        let files = location.join(name);
        std::fs::create_dir_all(&files).unwrap();
        for (id, entry) in checkpoints {
            let json = serde_json::to_vec(entry).unwrap();
            std::fs::write(files.join(format!("{id}.json")), json).unwrap();
        }
        if lock {
            std::fs::write(location.join(format!("{name}.lock")), "").unwrap();
        }
    }

    fn texts(pending: &[(u64, Checkpoint)]) -> Vec<&str> {
        let mut texts: Vec<&str> = pending.iter().map(|(_, c)| c.text.as_str()).collect();
        texts.sort_unstable();
        texts
    }

    #[test]
    fn a_second_instance_neither_offers_nor_discards_the_first_instances_live_work() {
        // The hazard this layout exists for, and it was real: before
        // instances, `offered` held window one's document and `discard_all`
        // deleted it, because the second window's startup prompt could not
        // tell a live session from a crashed one.
        let (_dir, first) = journal();
        let second = Journal::new(first.location().to_path_buf());
        first
            .checkpoint(
                1,
                &checkpoint("live in window one"),
                Recovery::Plaintext,
                None,
            )
            .unwrap();

        let offered = second.pending();
        second.discard_all().unwrap();
        second.discard(1).unwrap();

        assert!(
            offered.is_empty(),
            "offered another window's work: {offered:?}"
        );
        assert_eq!(second.held_elsewhere(), 1, "but it is known to exist");
        assert_eq!(
            texts(&first.pending()),
            ["live in window one"],
            "the live window's work was deleted"
        );
    }

    #[test]
    fn a_new_session_cannot_reach_a_dead_sessions_checkpoint_through_its_own_ids() {
        // Document ids restart at 1 in every session. Before instances, the
        // new session's clean document 1 discarded the old session's
        // checkpoint 1 within one autosave tick, and a dirty one overwrote
        // it -- so "keep for later" kept nothing.
        let (_dir, before) = journal();
        let location = before.location().to_path_buf();
        let kept = checkpoint("kept for later");
        before
            .checkpoint(1, &kept, Recovery::Plaintext, None)
            .unwrap();
        drop(before);

        let after = Journal::new(location);
        after
            .checkpoint(
                1,
                &checkpoint("the new document 1"),
                Recovery::Plaintext,
                None,
            )
            .unwrap();
        assert_eq!(
            texts(&after.pending()),
            ["kept for later", "the new document 1"],
            "the new session's checkpoint replaced the old one"
        );
        after.discard(1).unwrap();

        let pending = after.pending();
        assert_eq!(
            pending.len(),
            1,
            "the new session's discard reached the old session"
        );
        assert_eq!(pending[0], (1, kept), "and what survives is intact");
    }

    #[test]
    fn a_dead_instances_work_is_recovered_and_then_cleared_away_entirely() {
        let (_dir, crashed) = journal();
        let location = crashed.location().to_path_buf();
        crashed
            .checkpoint(4, &checkpoint("from the crash"), Recovery::Plaintext, None)
            .unwrap();
        drop(crashed);

        let next = Journal::new(location.clone());
        assert_eq!(texts(&next.pending()), ["from the crash"]);
        assert_eq!(next.held_elsewhere(), 0, "a dead instance holds nothing");

        next.discard_all().unwrap();
        assert!(next.pending().is_empty());
        let left: Vec<_> = std::fs::read_dir(&location).unwrap().collect();
        assert!(
            left.is_empty(),
            "the dead instance's directory and lock should go with its work: {left:?}"
        );
    }

    #[test]
    fn only_one_of_two_new_instances_claims_a_dead_instances_work() {
        // Selecting several files and pressing Enter starts several copies at
        // once. Were both to offer the same crash, one could delete what the
        // other had just restored.
        let (_dir, crashed) = journal();
        let location = crashed.location().to_path_buf();
        crashed
            .checkpoint(1, &checkpoint("orphan"), Recovery::Plaintext, None)
            .unwrap();
        drop(crashed);

        let first = Journal::new(location.clone());
        let second = Journal::new(location);
        assert_eq!(texts(&first.pending()), ["orphan"]);
        assert!(second.pending().is_empty(), "offered by both windows");
        assert_eq!(second.held_elsewhere(), 1);

        second.discard_all().unwrap();
        assert_eq!(
            texts(&first.pending()),
            ["orphan"],
            "deleted by the window that did not offer it"
        );

        // The claim ends with the window that made it.
        drop(first);
        assert_eq!(texts(&second.pending()), ["orphan"]);
    }

    #[test]
    fn work_kept_for_later_is_offered_again_at_the_next_launch() {
        let (_dir, crashed) = journal();
        let location = crashed.location().to_path_buf();
        crashed
            .checkpoint(1, &checkpoint("not now"), Recovery::Plaintext, None)
            .unwrap();
        drop(crashed);

        let asked = Journal::new(location.clone());
        assert_eq!(asked.pending().len(), 1);
        // Neither restored nor discarded: the session simply carries on.
        asked
            .checkpoint(1, &checkpoint("unrelated"), Recovery::Plaintext, None)
            .unwrap();
        asked.discard(1).unwrap();
        drop(asked);

        let later = Journal::new(location);
        assert_eq!(texts(&later.pending()), ["not now"]);
    }

    #[test]
    fn work_kept_for_later_is_offered_by_a_window_opened_while_this_one_is_still_open() {
        let (_dir, crashed) = journal();
        let location = crashed.location().to_path_buf();
        crashed
            .checkpoint(1, &checkpoint("not now"), Recovery::Plaintext, None)
            .unwrap();
        drop(crashed);

        let asked = Journal::new(location.clone());
        assert_eq!(asked.pending().len(), 1);
        asked.keep_for_later();

        let next = Journal::new(location);
        assert_eq!(
            texts(&next.pending()),
            ["not now"],
            "still claimed by a window that has already said not now"
        );
        assert_eq!(asked.held_elsewhere(), 1, "and now the next window has it");
    }

    #[test]
    fn a_window_open_all_day_does_not_hoard_a_crash_it_never_offered() {
        // Only the startup offer keeps a claim. Anything else that looks at
        // a dead instance -- every autosave does -- lets go again, or the
        // window the user opens after a crash would be told the crashed work
        // is open elsewhere.
        let (_dir, running) = journal();
        let location = running.location().to_path_buf();
        running
            .checkpoint(1, &checkpoint("mine"), Recovery::Plaintext, None)
            .unwrap();

        let crashed = Journal::new(location.clone());
        crashed
            .checkpoint(
                1,
                &checkpoint("lost in the crash"),
                Recovery::Plaintext,
                None,
            )
            .unwrap();
        drop(crashed);

        // An autosave tick, and a count, in the window that stayed open.
        running
            .checkpoint(1, &checkpoint("mine, later"), Recovery::Plaintext, None)
            .unwrap();
        let _ = running.sealed_count();
        running.clean_temporaries();

        let reopened = Journal::new(location);
        assert_eq!(texts(&reopened.pending()), ["lost in the crash"]);
        assert_eq!(reopened.held_elsewhere(), 1, "the open window's own work");
    }

    #[test]
    fn accepting_recovery_leaves_exactly_one_copy_of_the_restored_work() {
        // The restored document is checkpointed under the new instance before
        // the dead one's copy goes, so there is never a moment when only
        // memory holds it -- and afterwards there is one copy, not two
        // offered side by side at the next launch.
        let (_dir, crashed) = journal();
        let location = crashed.location().to_path_buf();
        crashed
            .checkpoint(3, &checkpoint("restore me"), Recovery::Plaintext, None)
            .unwrap();
        drop(crashed);

        let next = Journal::new(location.clone());
        let recovered = next.pending();
        next.checkpoint(1, &recovered[0].1, Recovery::Plaintext, None)
            .unwrap();
        next.discard_recovered().unwrap();
        assert_eq!(next.pending(), vec![(1, recovered[0].1.clone())]);
        drop(next);

        assert_eq!(texts(&Journal::new(location).pending()), ["restore me"]);
    }

    #[test]
    fn a_live_instances_sealed_journal_is_neither_offered_nor_discarded() {
        let (_dir, first) = journal();
        let second = Journal::new(first.location().to_path_buf());
        let entry = checkpoint("sealed in window one");
        let path = entry.path.clone().unwrap();
        first
            .checkpoint(1, &entry, Recovery::Encrypted, Some("hunter2"))
            .unwrap();

        assert_eq!(second.sealed_count(), 0);
        assert_eq!(second.sealed_pending(&path, "hunter2"), None);
        assert_eq!(second.held_elsewhere(), 1);
        // A save in the second window, and a plaintext checkpoint of the same
        // path there, both discard sealed copies -- but only their own.
        second.discard_sealed_for(&path).unwrap();
        second
            .checkpoint(1, &checkpoint("plain here"), Recovery::Plaintext, None)
            .unwrap();
        assert_eq!(
            first.sealed_pending(&path, "hunter2"),
            Some(entry.clone()),
            "the live window's sealed journal was discarded"
        );

        drop(first);
        assert_eq!(second.sealed_pending(&path, "hunter2"), Some(entry));
    }

    #[test]
    fn sealing_supersedes_the_dead_instances_copy_for_the_same_document() {
        // The single-slot behaviour sealed journals had before instances:
        // unlocking offered the dead copy, so once this instance has sealed
        // its own, the older one would only be offered again, holding less.
        let (_dir, crashed) = journal();
        let location = crashed.location().to_path_buf();
        let old = checkpoint("older");
        let path = old.path.clone().unwrap();
        crashed
            .checkpoint(1, &old, Recovery::Encrypted, Some("k"))
            .unwrap();
        drop(crashed);

        let next = Journal::new(location.clone());
        assert_eq!(next.sealed_pending(&path, "k"), Some(old));
        let mut new = checkpoint("newer");
        new.written_at += 1;
        next.checkpoint(9, &new, Recovery::Encrypted, Some("k"))
            .unwrap();
        assert_eq!(next.sealed_count(), 1, "the dead copy lingered");
        drop(next);

        assert_eq!(Journal::new(location).sealed_pending(&path, "k"), Some(new));
    }

    #[test]
    fn cleaning_temporaries_spares_a_live_instances_half_written_checkpoint() {
        // A `.tmp` in a live instance's directory may be a checkpoint
        // between its write and its rename; deleting it would make that
        // rename fail and lose the checkpoint.
        let (_dir, first) = journal();
        let second = Journal::new(first.location().to_path_buf());
        first
            .checkpoint(1, &checkpoint("work"), Recovery::Plaintext, None)
            .unwrap();
        let in_flight = first.own_dir().join("2.json.tmp");
        std::fs::write(&in_flight, "{ half writ").unwrap();

        second.clean_temporaries();
        assert!(
            in_flight.exists(),
            "a live instance's temporary was deleted"
        );

        drop(first);
        second.clean_temporaries();
        assert!(!in_flight.exists(), "a dead instance's debris was left");
    }

    #[test]
    fn a_journal_takes_its_lock_at_the_first_write_and_names_its_owner() {
        let (_dir, journal) = journal();
        let _ = journal.pending();
        assert!(
            !journal.location().exists(),
            "looking must not create anything"
        );

        journal
            .checkpoint(1, &checkpoint("work"), Recovery::Plaintext, None)
            .unwrap();
        let owner = std::fs::read_to_string(journal.own_dir().join(OWNER_FILE)).unwrap();
        assert!(
            owner.contains(&format!("pid {}", std::process::id())),
            "{owner}"
        );
    }

    // --- a crash while taking or dropping the lock ---

    #[test]
    fn an_instance_that_died_before_its_first_checkpoint_leaves_nothing_behind() {
        // Killed after taking its lock and making its directory, before it
        // wrote anything: nothing to offer, and nothing that lingers.
        let (_dir, journal) = journal();
        let location = journal.location().to_path_buf();
        crashed_instance(&location, "instance-1-dead", &[], true);
        std::fs::write(location.join("instance-1-dead").join(OWNER_FILE), "pid 1").unwrap();

        assert!(journal.pending().is_empty());
        assert_eq!(journal.held_elsewhere(), 0);
        journal.discard_all().unwrap();
        assert!(std::fs::read_dir(&location).unwrap().next().is_none());
    }

    #[test]
    fn a_dead_instances_checkpoint_is_recoverable_even_without_its_lock_file() {
        // An owner makes its lock before its directory, so a directory with
        // no lock was half-removed or half-made -- either way nobody is
        // running there, and the work in it is the user's.
        let (_dir, journal) = journal();
        let location = journal.location().to_path_buf();
        crashed_instance(
            &location,
            "instance-2-dead",
            &[(1, &checkpoint("lockless"))],
            false,
        );

        assert_eq!(texts(&journal.pending()), ["lockless"]);
    }

    #[test]
    fn a_lock_file_whose_directory_is_gone_is_harmless() {
        // Killed between removing a directory and removing its lock.
        let (_dir, journal) = journal();
        std::fs::create_dir_all(journal.location()).unwrap();
        std::fs::write(journal.location().join("instance-3-dead.lock"), "").unwrap();

        assert!(journal.pending().is_empty());
        assert_eq!(journal.held_elsewhere(), 0);
        journal
            .checkpoint(1, &checkpoint("fine"), Recovery::Plaintext, None)
            .unwrap();
        assert_eq!(journal.pending().len(), 1);
    }

    #[test]
    fn a_journal_written_before_instances_existed_is_still_recoverable() {
        // The layout this release replaces put checkpoints directly in the
        // recovery directory. A crash on the old version must still come
        // back on the new one.
        let (_dir, journal) = journal();
        let location = journal.location().to_path_buf();
        std::fs::create_dir_all(&location).unwrap();
        let old = checkpoint("from the old layout");
        std::fs::write(location.join("5.json"), serde_json::to_vec(&old).unwrap()).unwrap();

        assert_eq!(journal.pending(), vec![(5, old)]);
        journal.discard_all().unwrap();
        assert!(journal.pending().is_empty());
        assert!(location.exists(), "the recovery directory itself stays");
        assert!(
            !location.join(LEGACY_LOCK).exists(),
            "its claim should be tidied away"
        );
    }

    #[derive(Debug, Clone)]
    enum Op {
        Checkpoint(u64),
        Discard(u64),
        DiscardAll,
        DiscardRecovered,
        DiscardSealed,
        CleanTemporaries,
    }

    fn op() -> impl proptest::strategy::Strategy<Value = Op> {
        use proptest::prelude::*;
        prop_oneof![
            (1..4u64).prop_map(Op::Checkpoint),
            (1..4u64).prop_map(Op::Discard),
            Just(Op::DiscardAll),
            Just(Op::DiscardRecovered),
            Just(Op::DiscardSealed),
            Just(Op::CleanTemporaries),
        ]
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(48))]

        #[test]
        fn nothing_another_instance_does_touches_a_live_instances_checkpoints(
            ops in proptest::collection::vec(op(), 0..12)
        ) {
            let (_dir, live) = journal();
            let other = Journal::new(live.location().to_path_buf());
            let theirs = ["a1", "a2", "a3"];
            for (id, text) in (1..).zip(theirs) {
                live.checkpoint(id, &checkpoint(text), Recovery::Plaintext, None).unwrap();
            }

            for op in ops {
                match op {
                    Op::Checkpoint(id) => {
                        other.checkpoint(id, &checkpoint("b"), Recovery::Plaintext, None).unwrap();
                    }
                    Op::Discard(id) => other.discard(id).unwrap(),
                    Op::DiscardAll => other.discard_all().unwrap(),
                    Op::DiscardRecovered => other.discard_recovered().unwrap(),
                    Op::DiscardSealed => {
                        other.discard_sealed_for(Path::new("/notes/a.txt")).unwrap();
                    }
                    Op::CleanTemporaries => other.clean_temporaries(),
                }
                proptest::prop_assert!(
                    !texts(&other.pending()).iter().any(|t| t.starts_with('a')),
                    "offered a live window's work"
                );
            }
            let kept = live.pending();
            proptest::prop_assert_eq!(texts(&kept), theirs);

            // And once that window is gone, all of it is the other's to offer.
            drop(live);
            let offered = other.pending();
            let recovered: Vec<&str> =
                texts(&offered).into_iter().filter(|t| t.starts_with('a')).collect();
            proptest::prop_assert_eq!(recovered, theirs);
        }
    }

    // --- security policy --------------------------------------------------

    #[test]
    fn a_profile_that_disables_recovery_writes_nothing() {
        let (_dir, journal) = journal();

        let written = journal
            .checkpoint(1, &checkpoint("secret"), Recovery::Disabled, None)
            .unwrap();

        assert_eq!(written, Written::Refused(Refusal::ProfileForbidsIt));
        assert!(journal.pending().is_empty());
    }

    #[test]
    fn tightening_a_profile_deletes_what_the_looser_one_already_wrote() {
        // The whole point of asking the policy. Without this, moving a
        // document to Confidential leaves this morning's plaintext sitting in
        // the recovery directory while the profile reports it as protected --
        // which is worse than never having had a profile, because the user
        // has been told they are covered.
        let (_dir, journal) = journal();
        journal
            .checkpoint(
                1,
                &checkpoint("written while Standard"),
                Recovery::Plaintext,
                None,
            )
            .unwrap();
        assert_eq!(journal.pending().len(), 1, "the plaintext is on disk");

        journal
            .checkpoint(
                1,
                &checkpoint("now Confidential"),
                Recovery::Encrypted,
                None,
            )
            .unwrap();

        assert!(
            journal.pending().is_empty(),
            "tightening the profile must remove the earlier plaintext, not \
             merely stop adding to it"
        );
    }

    #[test]
    fn a_plaintext_document_under_a_strict_profile_is_told_what_to_do() {
        // The journal is sealed with the document's own passphrase, so a
        // plaintext document has no key. Refusing silently would leave the
        // user believing recovery was on; the notice says what turns it back
        // on.
        let (_dir, journal) = journal();

        let written = journal
            .checkpoint(1, &checkpoint("sensitive"), Recovery::Encrypted, None)
            .unwrap();

        assert_eq!(written, Written::Refused(Refusal::NoPassphrase));
        assert!(journal.pending().is_empty());
        assert!(
            Refusal::NoPassphrase
                .notice()
                .is_some_and(|n| n.contains("encrypt")),
            "the notice has to name the action that fixes it"
        );
    }

    #[test]
    fn a_never_saved_document_cannot_be_journalled_under_a_strict_profile() {
        // Its journal would be keyed by nothing, so a later session could not
        // find it even with the passphrase. Writing one would be work the user
        // could never get back.
        let (_dir, journal) = journal();
        let unsaved = Checkpoint {
            path: None,
            name: "Untitled".to_owned(),
            text: "unsaved".to_owned(),
            written_at: now_unix(),
        };

        let written = journal
            .checkpoint(1, &unsaved, Recovery::Encrypted, Some("hunter2"))
            .unwrap();

        assert_eq!(written, Written::Refused(Refusal::NeverSaved));
    }

    #[test]
    fn a_disabled_journal_is_silent_but_an_actionable_refusal_is_not() {
        // The distinction the refusals exist for: a profile doing what it was
        // set to do is not news, and one the user can fix is.
        assert_eq!(Refusal::ProfileForbidsIt.notice(), None);
        assert!(Refusal::NoPassphrase.notice().is_some());
        assert!(Refusal::NeverSaved.notice().is_some());
    }

    // --- sealed journals --------------------------------------------------

    #[test]
    fn an_encrypted_journal_round_trips_at_unlock_time() {
        let (_dir, journal) = journal();
        let entry = checkpoint("unsaved secret work");
        let path = entry.path.clone().unwrap();

        let written = journal
            .checkpoint(1, &entry, Recovery::Encrypted, Some("hunter2"))
            .unwrap();
        assert_eq!(written, Written::Yes);

        let recovered = journal
            .sealed_pending(&path, "hunter2")
            .expect("the journal is found by the document's path");
        assert_eq!(recovered, entry);
    }

    #[test]
    fn a_sealed_journal_does_not_appear_in_the_plain_pending_list() {
        // `pending` is what the startup prompt reads, and it must not offer to
        // recover something it cannot read.
        let (_dir, journal) = journal();
        journal
            .checkpoint(1, &checkpoint("secret"), Recovery::Encrypted, Some("k"))
            .unwrap();

        assert!(journal.pending().is_empty());
        assert_eq!(journal.sealed_count(), 1, "but it is known to exist");
    }

    #[test]
    fn a_sealed_journal_holds_the_path_and_name_inside_the_ciphertext() {
        // Writing either in clear beside it would leave a list of which
        // documents have unsaved work in the recovery directory -- close to the
        // thing the strict profiles exist to prevent.
        let (_dir, journal) = journal();
        let entry = checkpoint("secret work");
        journal
            .checkpoint(1, &entry, Recovery::Encrypted, Some("hunter2"))
            .unwrap();

        // Every file under the journal's own directory, not the tempdir
        // above it -- reading a directory as a file is a permission error on
        // Windows, which is how the first version of this test failed. Lock
        // files are only checked for emptiness: Windows refuses reads of a
        // locked file, and an empty file cannot leak anything.
        let mut dirs = vec![journal.location().to_path_buf()];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    dirs.push(path);
                    continue;
                }
                if path.extension().is_some_and(|x| x == "lock") {
                    assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);
                    continue;
                }
                let bytes = std::fs::read(&path).unwrap();
                for leak in [
                    b"a.txt".as_slice(),
                    b"notes".as_slice(),
                    b"secret".as_slice(),
                ] {
                    assert!(
                        !bytes.windows(leak.len()).any(|w| w == leak),
                        "{} appears in the recovery directory in clear",
                        String::from_utf8_lossy(leak)
                    );
                }
            }
        }
    }

    #[test]
    fn the_wrong_passphrase_recovers_nothing_rather_than_erroring() {
        // The caller has just opened the document with a passphrase, so a
        // failure here means the journal is damaged -- and a damaged journal is
        // simply no journal, not something to interrupt anyone about.
        let (_dir, journal) = journal();
        let entry = checkpoint("secret");
        let path = entry.path.clone().unwrap();
        journal
            .checkpoint(1, &entry, Recovery::Encrypted, Some("hunter2"))
            .unwrap();

        assert_eq!(journal.sealed_pending(&path, "wrong"), None);
    }

    #[test]
    fn loosening_a_profile_removes_the_sealed_journal() {
        // The mirror of the tightening case. A document that goes back to
        // Standard is journalled in clear, and the sealed copy must not linger
        // holding older work under a passphrase.
        let (_dir, journal) = journal();
        let entry = checkpoint("secret");
        journal
            .checkpoint(1, &entry, Recovery::Encrypted, Some("hunter2"))
            .unwrap();
        assert_eq!(journal.sealed_count(), 1);

        journal
            .checkpoint(1, &entry, Recovery::Plaintext, None)
            .unwrap();

        assert_eq!(journal.sealed_count(), 0, "the sealed copy lingered");
        assert_eq!(journal.pending().len(), 1);
    }

    #[test]
    fn disabling_recovery_removes_both_forms() {
        let (_dir, journal) = journal();
        let entry = checkpoint("secret");
        journal
            .checkpoint(1, &entry, Recovery::Encrypted, Some("hunter2"))
            .unwrap();

        journal
            .checkpoint(1, &entry, Recovery::Disabled, None)
            .unwrap();

        assert_eq!(journal.sealed_count(), 0);
        assert!(journal.pending().is_empty());
    }

    #[test]
    fn two_documents_get_two_sealed_journals() {
        // Keyed by path, so one document's journal must not overwrite
        // another's -- which a fixed name or a session id would do.
        let (_dir, journal) = journal();
        let mut a = checkpoint("first");
        a.path = Some(PathBuf::from("/notes/a.txt"));
        let mut b = checkpoint("second");
        b.path = Some(PathBuf::from("/notes/b.txt"));

        journal
            .checkpoint(1, &a, Recovery::Encrypted, Some("k"))
            .unwrap();
        journal
            .checkpoint(2, &b, Recovery::Encrypted, Some("k"))
            .unwrap();

        assert_eq!(journal.sealed_count(), 2);
        assert_eq!(
            journal
                .sealed_pending(&PathBuf::from("/notes/a.txt"), "k")
                .unwrap()
                .text,
            "first"
        );
    }

    #[test]
    fn refusing_a_document_leaves_other_documents_journals_alone() {
        // One document going Confidential must not lose another's unsaved
        // work. The refusal path calls `discard`, and discarding the wrong id
        // would be a data-loss bug wearing a security fix.
        let (_dir, journal) = journal();
        journal
            .checkpoint(1, &checkpoint("keep me"), Recovery::Plaintext, None)
            .unwrap();

        journal
            .checkpoint(2, &checkpoint("forget me"), Recovery::Disabled, None)
            .unwrap();

        let pending = journal.pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0, 1);
        assert_eq!(pending[0].1.text, "keep me");
    }
}
