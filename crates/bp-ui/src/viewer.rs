//! The large-file viewer (release plan M1-06, owner decision U3).
//!
//! A file past the editable limit used to be refused. It now opens here: a
//! read-only tab that shows the file a screenful at a time and never loads
//! it. `bp_buffer::LargeFile` does the reading -- a sparse line index and one
//! chunk at a time, ADR-0027's chunked reads rather than a memory map -- and
//! this module decides what is on screen and keeps everything slow off the
//! UI thread.
//!
//! Three kinds of work, and where each runs:
//!
//! * **Drawing** reads the visible lines on the UI thread, and only lines the
//!   index already knows the end of. Reading forty lines costs forty lines;
//!   reading into the part of the file nobody has indexed yet could cost the
//!   file, so the view does not go there until the index has.
//! * **Indexing** -- finding where every line starts, so Ctrl+End and Go to
//!   Line can reach the end of a 2 GB log -- runs on a [`Background`] with a
//!   `LargeFile` of its own. It shares what it has found every quarter of a
//!   second, and the finished index is adopted whole.
//! * **Finding** streams the file past the matcher through
//!   `bp_buffer::WindowReader`, also on a [`Background`]. Closing the find
//!   bar stops it; a newer search supersedes it.
//!
//! Nothing here reaches the recovery journal, the clipboard history or a
//! save: a viewer tab has no rope, so there is no text for any of them to
//! take, and `AppState` refuses the write paths by name as well
//! (`refuse_read_only`), so that a path nobody thought of cannot write an
//! empty buffer over the file.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bp_buffer::{INDEX_BUDGET_BYTES, LargeFile, LineIndex, StreamWindow, WindowReader};
use bp_core::DocumentId;

use crate::state::AppState;
use crate::worker::Background;
use crate::{AppWindow, EditorRow, SelectionBox};

/// How often the indexing thread hands over what it has found so far.
///
/// A quarter of a second: often enough that scrolling down a file still
/// being indexed feels like the file arriving, rarely enough that copying a
/// half-megabyte index is noise against the reading.
const SHARE_EVERY: Duration = Duration::from_millis(250);

/// How far Left and Right move the view, in columns.
const SIDEWAYS_STEP: usize = 4;

/// Said once per viewer tab, the first time a key would have edited it.
///
/// Once, because the tenth keypress has already been told; a status bar that
/// repeats itself per key is a status bar nobody reads.
pub(crate) const TYPING_REFUSED: &str =
    "Read-only: this file is open in the viewer, so typing changes nothing";

/// Why a document is in the viewer rather than the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reason {
    /// Past the editable limit (`editable_limit_mib`).
    TooLarge { bytes: u64, limit: u64 },
    /// `--readonly` on the command line.
    ReadOnly,
    /// `--large-file` on the command line.
    Asked,
}

impl Reason {
    /// The banner above the text. States the fact and, for a file that is
    /// too large, both numbers -- "too large" alone invites "it is only 40
    /// megabytes".
    pub(crate) fn banner(self) -> String {
        match self {
            Self::TooLarge { bytes, limit } => format!(
                "Read-only: this file is too large to edit ({}; the limit is {})",
                size(bytes),
                size(limit)
            ),
            Self::ReadOnly => {
                "Opened read-only (--readonly): nothing here can be changed or saved".to_owned()
            }
            Self::Asked => "Read-only: opened in the large-file viewer (--large-file)".to_owned(),
        }
    }
}

/// How the command line asked for files to be opened.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum OpenAs {
    /// The ordinary way: editable, or the viewer if the file is too large.
    #[default]
    Normal,
    /// `--readonly`.
    ReadOnly,
    /// `--large-file`.
    Viewer,
}

/// A byte count as the banner says it: whole MiB when it is whole -- the
/// limit always is -- and otherwise tenths, rounded up. Up, so a file one
/// byte past a 32 MiB limit reads "32.1 MiB" and never "32 MiB; the limit is
/// 32 MiB", which would contradict itself.
fn size(bytes: u64) -> String {
    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = 1024 * MIB;
    let (unit, name) = if bytes >= GIB {
        (GIB, "GiB")
    } else {
        (MIB, "MiB")
    };
    if bytes.is_multiple_of(unit) {
        return format!("{} {name}", bytes / unit);
    }
    let tenths = (u128::from(bytes) * 10).div_ceil(u128::from(unit));
    format!("{}.{} {name}", tenths / 10, tenths % 10)
}

/// A number with thousands separators, for line counts in the millions.
fn grouped(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Where a match is: a 0-based line and a range of characters within it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Hit {
    pub(crate) line: usize,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// How a search ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Outcome {
    Found { hit: Hit, wrapped: bool },
    NotFound,
    Failed(String),
    Stopped,
}

/// A Go to Line waiting for the index to reach its line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pending {
    Line(usize),
    End,
}

/// What a Go to Line did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Landed {
    /// On the line asked for.
    There,
    /// Past the end: on the last line, which is this many lines in.
    PastEnd(usize),
    /// The index has not reached it yet; it happens when it does.
    Waiting,
}

/// One screenful, ready for the window.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Frame {
    pub(crate) banner: String,
    /// Gutter number and text, one per visible row.
    pub(crate) rows: Vec<(String, String)>,
    /// Row and visual columns of the highlighted match, if it is on screen.
    pub(crate) highlight: Option<(usize, usize, usize)>,
    /// Row and visual column of the caret, if it is on screen.
    pub(crate) caret: Option<(usize, usize)>,
    /// Where the scrollbar's thumb is, 0 at the top and 1 at the bottom.
    pub(crate) thumb_position: f32,
    /// How much of the track it covers.
    pub(crate) thumb_size: f32,
    /// Digits in the largest line number the file is expected to have, so
    /// the gutter is wide enough before the user scrolls down to the numbers
    /// that need it, and does not change width as they do.
    pub(crate) gutter_columns: usize,
}

/// One document open in the viewer.
pub(crate) struct Viewer {
    file: LargeFile,
    reason: Reason,
    /// First line on screen, 0-based.
    pub(crate) top: usize,
    /// The caret's line, 0-based: what Ln shows and where Find starts.
    pub(crate) line: usize,
    /// Columns scrolled off the left edge.
    pub(crate) left: usize,
    hit: Option<Hit>,
    told_read_only: bool,
    pending: Option<Pending>,
    indexing: Background<Result<LineIndex, String>>,
    /// Bytes the indexing thread has got through, for the status bar.
    indexed: Arc<AtomicU64>,
    /// What the indexing thread has found so far, newest only.
    shared: Arc<Mutex<Option<LineIndex>>>,
    search: Background<Outcome>,
    /// Bytes the search has got through, for the find bar.
    scanned: Arc<AtomicU64>,
    /// Why the view cannot show the file, when it cannot.
    pub(crate) failure: Option<String>,
    /// Bytes of text per search window. The default outside tests; a test
    /// lowers it to put a match across a window boundary in a small file.
    pub(crate) window_bytes: usize,
}

impl Drop for Viewer {
    /// Closing the tab stops its threads. Their results would be thrown away
    /// anyway; stopping them gives the disk back.
    fn drop(&mut self) {
        self.indexing.stop();
        self.search.stop();
    }
}

impl Viewer {
    /// Open `path` for viewing and start indexing it.
    ///
    /// Reads one chunk on this thread before handing the rest to the
    /// worker, so a file of a few hundred kilobytes -- `--readonly` on a
    /// note -- is on screen at once rather than a tick later.
    pub(crate) fn open(path: &Path, reason: Reason) -> Result<Self, String> {
        let mut file = LargeFile::open(path)
            .map_err(|e| e.to_string())?
            .substituting_invalid();
        file.index_more(bp_buffer::CHUNK_BYTES as u64)
            .map_err(|e| e.to_string())?;

        let indexed = Arc::new(AtomicU64::new(file.progress().indexed_bytes));
        let shared = Arc::new(Mutex::new(None));
        let mut indexing = Background::new();
        if !file.progress().complete {
            let (path, progress, latest) =
                (path.to_owned(), Arc::clone(&indexed), Arc::clone(&shared));
            indexing.start(move |stop| index_in_background(&path, stop, &progress, &latest));
        }

        Ok(Self {
            file,
            reason,
            top: 0,
            line: 0,
            left: 0,
            hit: None,
            told_read_only: false,
            pending: None,
            indexing,
            indexed,
            shared,
            search: Background::new(),
            scanned: Arc::new(AtomicU64::new(0)),
            failure: None,
            window_bytes: bp_buffer::DEFAULT_WINDOW_BYTES,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        self.file.path()
    }

    pub(crate) fn reason(&self) -> Reason {
        self.reason
    }

    #[cfg(test)]
    pub(crate) fn hit(&self) -> Option<Hit> {
        self.hit
    }

    /// Whether the whole file has been indexed, so its line count is known.
    pub(crate) fn complete(&self) -> bool {
        self.file.progress().complete
    }

    /// Lines the view may show: those whose end the index has seen. The
    /// last line counted in an unfinished index may run on into bytes not
    /// yet read, and drawing it would read them here.
    pub(crate) fn reachable(&self) -> usize {
        let progress = self.file.progress();
        if progress.complete {
            progress.lines
        } else {
            progress.lines.saturating_sub(1)
        }
    }

    /// How many lines the file will turn out to have: the count once it is
    /// known, and until then the lines found so far scaled up by the bytes
    /// not yet read. An estimate, used only to size the gutter.
    fn expected_lines(&self) -> usize {
        let progress = self.file.progress();
        if progress.complete || progress.indexed_bytes == 0 {
            return progress.lines;
        }
        let scaled = u128::from(self.file.len_bytes()) * progress.lines as u128
            / u128::from(progress.indexed_bytes);
        usize::try_from(scaled).unwrap_or(usize::MAX)
    }

    /// Whether a thread is still working for this viewer.
    pub(crate) fn busy(&self) -> bool {
        self.indexing.busy() || self.search.busy()
    }

    pub(crate) fn searching(&self) -> bool {
        self.search.busy()
    }

    /// How far indexing has got, as a whole percentage.
    fn indexed_percent(&self) -> u64 {
        let total = self.file.len_bytes().max(1);
        let done = self
            .indexed
            .load(Ordering::Relaxed)
            .max(self.file.progress().indexed_bytes);
        (done.min(total) * 100) / total
    }

    /// How far the running search has got, as a whole percentage.
    pub(crate) fn search_percent(&self) -> u64 {
        let total = self.file.len_bytes().max(1);
        (self.scanned.load(Ordering::Relaxed).min(total) * 100) / total
    }

    /// Ln for the status bar, and the line count once there is one.
    pub(crate) fn position_label(&self) -> String {
        let at = grouped(self.line + 1);
        if self.complete() {
            format!("Ln {at} of {}", grouped(self.file.progress().lines))
        } else {
            format!("Ln {at} · indexing lines, {}%", self.indexed_percent())
        }
    }

    /// The key that would have edited was pressed. Returns the message to
    /// show, the first time only.
    pub(crate) fn refuse_typing(&mut self) -> Option<&'static str> {
        if self.told_read_only {
            return None;
        }
        self.told_read_only = true;
        Some(TYPING_REFUSED)
    }

    /// The first line that can be at the top with `rows` on screen.
    fn max_top(&self, rows: usize) -> usize {
        self.reachable().saturating_sub(rows.max(1))
    }

    /// Move the caret to `line`, scrolling as little as possible to show it.
    pub(crate) fn put_caret(&mut self, line: usize, rows: usize) {
        let rows = rows.max(1);
        self.line = line.min(self.reachable().saturating_sub(1));
        if self.line < self.top {
            self.top = self.line;
        } else if self.line >= self.top + rows {
            self.top = self.line + 1 - rows;
        }
        self.top = self.top.min(self.max_top(rows));
    }

    /// Move the caret by `delta` lines.
    pub(crate) fn move_caret(&mut self, delta: isize, rows: usize) {
        let line = self.line.saturating_add_signed(delta);
        self.put_caret(line, rows);
    }

    /// Scroll the view by `delta` lines, leaving the caret where it is --
    /// what a wheel does.
    pub(crate) fn scroll(&mut self, delta: isize, rows: usize) {
        self.top = self
            .top
            .saturating_add_signed(delta)
            .min(self.max_top(rows));
    }

    /// Put the top of the view `fraction` of the way down, from the
    /// scrollbar.
    pub(crate) fn scrub(&mut self, fraction: f32, rows: usize) {
        let max = self.max_top(rows);
        let at = (fraction.clamp(0.0, 1.0) * max as f32).round();
        // Rounded and clamped just above, so the cast cannot truncate.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let at = at as usize;
        self.top = at.min(max);
        self.line = self.line.clamp(self.top, self.top + rows.max(1) - 1);
    }

    /// Scroll sideways by `delta` columns.
    pub(crate) fn sideways(&mut self, delta: isize) {
        self.left = self
            .left
            .saturating_add_signed(delta)
            .min(bp_buffer::MAX_DISPLAY_LINE_BYTES);
    }

    /// Go to a 0-based line, or wait for the index to reach it.
    pub(crate) fn go_to(&mut self, line: usize, rows: usize) -> Landed {
        self.hit = None;
        if line < self.reachable() {
            self.pending = None;
            self.put_caret(line, rows);
            self.centre(rows);
            return Landed::There;
        }
        if self.complete() {
            self.pending = None;
            self.put_caret(line, rows);
            return Landed::PastEnd(self.file.progress().lines);
        }
        // As far as is known now, and the rest when the index gets there.
        self.put_caret(line, rows);
        self.pending = Some(Pending::Line(line));
        Landed::Waiting
    }

    /// Ctrl+End: the last line, now or once the index has found it.
    pub(crate) fn go_to_end(&mut self, rows: usize) {
        self.put_caret(usize::MAX, rows);
        self.pending = (!self.complete()).then_some(Pending::End);
    }

    /// Put the caret's line a third of the way down, so what is around a
    /// match or a line jumped to is on screen with it.
    fn centre(&mut self, rows: usize) {
        let rows = rows.max(1);
        self.top = self.line.saturating_sub(rows / 3).min(self.max_top(rows));
    }

    /// Adopt whatever the threads have finished. Returns whether anything
    /// changed that the window should redraw, and what a finished search
    /// found for the find bar to say.
    pub(crate) fn poll(&mut self, rows: usize) -> (bool, Option<Outcome>) {
        let mut changed = false;
        let shared = self.shared.lock().ok().and_then(|mut slot| slot.take());
        if let Some(index) = shared {
            changed |= self.file.adopt_index(index);
        }
        match self.indexing.take() {
            Some(Ok(index)) => changed |= self.file.adopt_index(index),
            Some(Err(e)) => {
                self.failure = Some(e);
                changed = true;
            }
            None => {}
        }
        if changed {
            self.resolve_pending(rows);
        }

        let outcome = self.search.take();
        if let Some(Outcome::Found { hit, .. }) = &outcome {
            self.show_hit(*hit, rows);
        }
        (changed || outcome.is_some(), outcome)
    }

    /// A Go to Line or Ctrl+End that was waiting for the index.
    fn resolve_pending(&mut self, rows: usize) {
        match self.pending {
            Some(Pending::End) if self.complete() => {
                self.pending = None;
                self.put_caret(usize::MAX, rows);
            }
            Some(Pending::Line(line)) if line < self.reachable() || self.complete() => {
                self.pending = None;
                self.put_caret(line, rows);
                self.centre(rows);
            }
            _ => {}
        }
    }

    /// Scroll to a match and remember it, so Find Next starts after it.
    fn show_hit(&mut self, hit: Hit, rows: usize) {
        self.hit = Some(hit);
        self.pending = None;
        if hit.line >= self.reachable() {
            // Found by the search before the index got there: go as far as
            // is known, and the rest once it arrives.
            self.put_caret(hit.line, rows);
            self.pending = Some(Pending::Line(hit.line));
            return;
        }
        self.put_caret(hit.line, rows);
        self.centre(rows);
    }

    /// Start a search. `forward` for Find Next, `fresh` when the query
    /// itself changed -- a match at the caret then counts, where for Find
    /// Next it is the one being stepped past.
    pub(crate) fn find(&mut self, query: bp_search::Query, forward: bool, fresh: bool) -> String {
        if query.is_empty() {
            self.search.stop();
            self.hit = None;
            return String::new();
        }
        // A bad pattern is said at once, as the editor's find says it,
        // rather than after a thread has been started to find it out.
        if let Err(e) = bp_search::count("", &query) {
            self.search.stop();
            return e.to_string();
        }
        let from = match self.hit {
            Some(hit) => (hit.line, hit.start),
            None => (self.line, 0),
        };
        let job = SearchJob {
            path: self.file.path().to_owned(),
            // Where `from` starts is looked up on the worker, with a copy of
            // what is indexed so far: a match the search found past the
            // index would otherwise be indexed up to here, on this thread.
            index: self.file.index().clone(),
            query,
            forward,
            // A match on the caret's own line counts when there is no match
            // to step past yet, so Find Next from line 1 finds line 1's.
            inclusive: forward && (fresh || self.hit.is_none()),
            from,
            window_bytes: self.window_bytes,
        };
        self.scanned.store(0, Ordering::Relaxed);
        let scanned = Arc::clone(&self.scanned);
        self.search.start(move |stop| job.run(stop, &scanned));
        "searching… (Esc stops)".to_owned()
    }

    /// Stop a running search. Returns whether there was one.
    pub(crate) fn stop_search(&mut self) -> bool {
        let running = self.search.busy();
        self.search.stop();
        running
    }

    /// The visible lines and everything drawn with them.
    ///
    /// `columns` is how many characters fit across; `tab` the tab width.
    /// Does the reading for the frame, which is why it takes `&mut self`:
    /// at most `rows` lines, each capped, and only lines the index has
    /// already seen the end of.
    pub(crate) fn frame(&mut self, rows: usize, columns: usize, tab: usize) -> Frame {
        let rows = rows.max(1);
        let columns = columns.max(1);
        let reachable = self.reachable();
        let count = rows.min(reachable.saturating_sub(self.top));
        let mut frame = Frame {
            banner: self.reason.banner(),
            ..Frame::default()
        };
        if let Some(failure) = &self.failure {
            frame.banner = format!("{} -- {failure}", frame.banner);
        }

        let lines = if count == 0 {
            Vec::new()
        } else {
            match self.file.display_lines(self.top, count) {
                Ok(lines) => lines,
                Err(e) => {
                    frame.banner = format!("{} -- {e}", frame.banner);
                    Vec::new()
                }
            }
        };

        for (row, shown) in lines.iter().enumerate() {
            let mut text = shown.text.clone();
            if shown.truncated {
                text.push_str(" ⋯ (the viewer shows the first 64 KiB of a line)");
            }
            let visual = expand_tabs(&text, tab);
            // Plain digits, as the editor's gutter has them; the status bar
            // groups them, where there is room.
            frame.rows.push((
                (shown.line + 1).to_string(),
                visual.chars().skip(self.left).take(columns).collect(),
            ));

            if let Some(hit) = self.hit.filter(|h| h.line == shown.line) {
                let start = bp_editor::view::visual_column(&text, hit.start, tab);
                let end = bp_editor::view::visual_column(&text, hit.end.max(hit.start + 1), tab);
                let (start, end) = (
                    start.saturating_sub(self.left),
                    end.saturating_sub(self.left),
                );
                if end > 0 && start < columns {
                    frame.highlight = Some((row, start, end.min(columns)));
                }
            }
            if shown.line == self.line {
                let at = self
                    .hit
                    .filter(|h| h.line == shown.line)
                    .map_or(0, |h| bp_editor::view::visual_column(&text, h.start, tab));
                if at >= self.left && at - self.left <= columns {
                    frame.caret = Some((row, at - self.left));
                }
            }
        }

        let total = reachable.max(1);
        frame.gutter_columns = self.expected_lines().max(total).to_string().len();
        frame.thumb_size = (rows as f32 / total as f32).clamp(0.0, 1.0);
        let max_top = self.max_top(rows);
        frame.thumb_position = if max_top == 0 {
            0.0
        } else {
            (self.top as f32 / max_top as f32).clamp(0.0, 1.0)
        };
        frame
    }

    /// Scroll sideways so the match on the caret's line is on screen.
    pub(crate) fn reveal_hit(&mut self, columns: usize, tab: usize) {
        // Not past the index: reading there would index it, here.
        let Some(hit) = self.hit.filter(|h| h.line < self.reachable()) else {
            return;
        };
        let Ok(lines) = self.file.display_lines(hit.line, 1) else {
            return;
        };
        let Some(shown) = lines.first() else {
            return;
        };
        let start = bp_editor::view::visual_column(&shown.text, hit.start, tab);
        let end = bp_editor::view::visual_column(&shown.text, hit.end, tab);
        let columns = columns.max(1);
        if start < self.left || end > self.left + columns {
            self.left = start.saturating_sub(columns / 4);
        }
    }

    /// End without Ctrl: scroll sideways to the end of the caret's line.
    pub(crate) fn show_line_end(&mut self, columns: usize, tab: usize) {
        if self.line >= self.reachable() {
            return;
        }
        let Ok(lines) = self.file.display_lines(self.line, 1) else {
            return;
        };
        let width = lines
            .first()
            .map_or(0, |l| expand_tabs(&l.text, tab).chars().count());
        self.left = width.saturating_sub(columns.max(1).saturating_sub(1));
    }
}

/// Tabs to spaces, to the next multiple of `tab`, so a column on screen is a
/// column in the arithmetic.
fn expand_tabs(text: &str, tab: usize) -> String {
    if !text.contains('\t') {
        return text.to_owned();
    }
    let tab = tab.max(1);
    let mut out = String::with_capacity(text.len() + 16);
    let mut column = 0;
    for ch in text.chars() {
        if ch == '\t' {
            let next = (column / tab + 1) * tab;
            out.extend(std::iter::repeat_n(' ', next - column));
            column = next;
        } else {
            out.push(ch);
            column += 1;
        }
    }
    out
}

/// Index `path` to the end on a thread of its own, sharing progress as it
/// goes. What comes back is the whole index, for the viewer to adopt.
fn index_in_background(
    path: &Path,
    stop: &AtomicBool,
    indexed: &AtomicU64,
    shared: &Mutex<Option<LineIndex>>,
) -> Result<LineIndex, String> {
    let mut file = LargeFile::open(path).map_err(|e| e.to_string())?;
    let mut last_shared: Option<Instant> = None;
    loop {
        if stop.load(Ordering::Relaxed) {
            return Err("indexing stopped".to_owned());
        }
        let progress = file
            .index_more(INDEX_BUDGET_BYTES)
            .map_err(|e| e.to_string())?;
        indexed.store(progress.indexed_bytes, Ordering::Relaxed);
        if progress.complete {
            return Ok(file.index().clone());
        }
        if last_shared.is_none_or(|at| at.elapsed() >= SHARE_EVERY) {
            if let Ok(mut slot) = shared.lock() {
                *slot = Some(file.index().clone());
            }
            last_shared = Some(Instant::now());
        }
    }
}

/// One search, everything it needs to run on another thread.
struct SearchJob {
    path: PathBuf,
    /// The viewer's index when the search started.
    index: LineIndex,
    query: bp_search::Query,
    forward: bool,
    /// Whether a match exactly at `from` counts.
    inclusive: bool,
    /// Line and character column the search starts from.
    from: (usize, usize),
    window_bytes: usize,
}

/// What the visitor wants after seeing a match.
enum Next {
    Stop,
    Go,
}

impl SearchJob {
    fn run(self, stop: &AtomicBool, scanned: &AtomicU64) -> Outcome {
        let from_byte = match self.line_start() {
            Ok(byte) => byte,
            Err(e) => return Outcome::Failed(e),
        };
        let result = if self.forward {
            self.forwards(from_byte, stop, scanned)
        } else {
            self.backwards(from_byte, stop, scanned)
        };
        match result {
            Ok(outcome) => outcome,
            Err(e) => Outcome::Failed(e),
        }
    }

    /// Where the line the search starts from begins.
    fn line_start(&self) -> Result<u64, String> {
        let mut file = LargeFile::open(&self.path).map_err(|e| e.to_string())?;
        file.adopt_index(self.index.clone());
        Ok(file
            .line_start(self.from.0)
            .map_err(|e| e.to_string())?
            .unwrap_or(0))
    }

    fn after(&self, hit: Hit) -> bool {
        let at = (hit.line, hit.start);
        if self.inclusive {
            at >= self.from
        } else {
            at > self.from
        }
    }

    /// From `from` to the end, then from the top back round to `from`.
    fn forwards(
        &self,
        from_byte: u64,
        stop: &AtomicBool,
        scanned: &AtomicU64,
    ) -> Result<Outcome, String> {
        let mut found = None;
        self.scan(from_byte, self.from.0, None, stop, scanned, |hit| {
            if self.after(hit) {
                found = Some(hit);
                Next::Stop
            } else {
                Next::Go
            }
        })?;
        if let Some(hit) = found {
            return Ok(Outcome::Found {
                hit,
                wrapped: false,
            });
        }
        if stop.load(Ordering::Relaxed) {
            return Ok(Outcome::Stopped);
        }
        self.scan(0, 0, Some(self.from.0), stop, scanned, |hit| {
            if !self.after(hit) {
                found = Some(hit);
            }
            Next::Stop
        })?;
        Ok(found.map_or(Outcome::NotFound, |hit| Outcome::Found {
            hit,
            wrapped: true,
        }))
    }

    /// The last match before `from`; failing that, the last in the file.
    ///
    /// Streaming only runs forwards, so this reads from the top to `from`
    /// and keeps the last match it passed. It costs the file up to the
    /// caret rather than the few lines above it, which is why it is on a
    /// thread and can be stopped.
    fn backwards(
        &self,
        from_byte: u64,
        stop: &AtomicBool,
        scanned: &AtomicU64,
    ) -> Result<Outcome, String> {
        let from = self.from;
        let mut last = None;
        self.scan(0, 0, Some(from.0), stop, scanned, |hit| {
            if (hit.line, hit.start) < from {
                last = Some(hit);
                Next::Go
            } else {
                Next::Stop
            }
        })?;
        if let Some(hit) = last {
            return Ok(Outcome::Found {
                hit,
                wrapped: false,
            });
        }
        if stop.load(Ordering::Relaxed) {
            return Ok(Outcome::Stopped);
        }
        self.scan(from_byte, from.0, None, stop, scanned, |hit| {
            if (hit.line, hit.start) > from {
                last = Some(hit);
            }
            Next::Go
        })?;
        Ok(last.map_or(Outcome::NotFound, |hit| Outcome::Found {
            hit,
            wrapped: true,
        }))
    }

    /// Stream from `byte` (the start of `line`) and show `visit` every
    /// match in file order, until it says stop, the file ends, or a window
    /// starts past `until_line`.
    fn scan(
        &self,
        byte: u64,
        line: usize,
        until_line: Option<usize>,
        stop: &AtomicBool,
        scanned: &AtomicU64,
        mut visit: impl FnMut(Hit) -> Next,
    ) -> Result<(), String> {
        let overlap = self
            .query
            .pattern
            .len()
            .max(bp_buffer::DEFAULT_OVERLAP_BYTES);
        let mut reader = WindowReader::with_config(&self.path, self.window_bytes, overlap)
            .map_err(|e| e.to_string())?
            .substituting_invalid();
        reader.start_at(byte, line).map_err(|e| e.to_string())?;
        // The window before, and the column its text began at, for the
        // window after it when that one starts in the middle of a line.
        let mut previous: Option<(StreamWindow, usize)> = None;
        while let Some(window) = reader.next_window().map_err(|e| e.to_string())? {
            if stop.load(Ordering::Relaxed) {
                return Ok(());
            }
            let first_column = if window.starts_mid_line {
                previous.as_ref().map_or(0, |(before, column)| {
                    column_where_next_starts(before, *column, &window)
                })
            } else {
                0
            };
            scanned.store(reader.bytes_delivered(), Ordering::Relaxed);
            if until_line.is_some_and(|until| window.first_line > until) {
                return Ok(());
            }
            // Counting first is cheap and skips the per-window offset table
            // that `find_all` builds, for the windows with nothing in them.
            if bp_search::count(&window.text, &self.query).map_err(|e| e.to_string())? > 0 {
                let matches =
                    bp_search::find_all(&window.text, &self.query).map_err(|e| e.to_string())?;
                let mut columns = Columns::new(&window, first_column);
                for found in matches {
                    if !window.is_new_match(&found.range) {
                        continue;
                    }
                    let start = columns.of(found.range.start);
                    let hit = Hit {
                        line: window.absolute_line(found.line),
                        start,
                        end: start + (found.range.end - found.range.start),
                    };
                    if let Next::Stop = visit(hit) {
                        return Ok(());
                    }
                }
            }
            previous = Some((window, first_column));
        }
        Ok(())
    }
}

/// Character columns of matches within one window, walking it once.
///
/// Matches arrive in order, so the walk only ever moves forward; asking
/// each match's column from the start of the window would be quadratic on
/// a line full of matches.
struct Columns<'a> {
    chars: std::str::Chars<'a>,
    at: usize,
    line_start: usize,
    /// The column the window's first character sits at: 0 unless the window
    /// was cut inside a line longer than it.
    first_column: usize,
    on_first_line: bool,
}

impl<'a> Columns<'a> {
    fn new(window: &'a StreamWindow, first_column: usize) -> Self {
        Self {
            chars: window.text.chars(),
            at: 0,
            line_start: 0,
            first_column,
            on_first_line: true,
        }
    }

    /// The column of the character at `offset` in the window.
    fn of(&mut self, offset: usize) -> usize {
        while self.at < offset {
            let Some(ch) = self.chars.next() else {
                break;
            };
            self.at += 1;
            // A line ends at LF, CRLF or a bare CR (D6). A CRLF moves the
            // line start twice, landing after the LF, which is where it ends.
            if ch == '\n' || ch == '\r' {
                self.line_start = self.at;
                self.on_first_line = false;
            }
        }
        let column = offset.saturating_sub(self.line_start);
        if self.on_first_line {
            column + self.first_column
        } else {
            column
        }
    }
}

/// The column at which `next` begins, given the window before it and the
/// column that one began at.
///
/// Only asked when `next` starts in the middle of a line, which takes a line
/// longer than half a window. `next` repeats the tail of `before`, so it
/// begins that many characters short of `before`'s end, and the column there
/// is counted from `before`'s last line break -- or, if it had none, from
/// where `before` itself began.
fn column_where_next_starts(
    before: &StreamWindow,
    before_column: usize,
    next: &StreamWindow,
) -> usize {
    let total = before.text.chars().count();
    let at = total.saturating_sub(next.overlap_chars);
    let head: String = before.text.chars().take(at).collect();
    match head.rfind(['\n', '\r']) {
        Some(newline) => head[newline + 1..].chars().count(),
        None => before_column + at,
    }
}

// --- the application's side ----------------------------------------------

impl AppState {
    /// Open `path` in the viewer, as a tab of its own.
    pub(crate) fn open_in_viewer(&mut self, path: PathBuf, reason: Reason) {
        if let Some(refusal) = refuse_encoding(&path) {
            self.error = Some(refusal);
            return;
        }
        let viewer = match Viewer::open(&path, reason) {
            Ok(viewer) => viewer,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        let created = crate::state::created_at(&path);
        let id = self.workspace.open_path(path.clone(), created);
        self.give_default_profile(id);
        self.recall_profile(id, &path);
        if let Some(doc) = self.workspace.get_mut(id)
            && let Some(ending) = sniff_line_ending(&path)
        {
            doc.set_line_ending(ending);
        }
        self.viewers.insert(id, viewer);
        self.mark_in_step(id, &path);
        tracing::info!("opened a file in the read-only viewer ({reason:?})");
    }

    /// File ▸ Reload from Disk on a viewer tab: read the file again from
    /// scratch -- a log has usually grown -- and stay on the same line.
    pub(crate) fn reload_viewer(&mut self, id: DocumentId) {
        let rows = self.viewer_rows;
        let Some(old) = self.viewers.get(&id) else {
            return;
        };
        let (path, reason, line) = (old.path().to_owned(), old.reason(), old.line);
        match Viewer::open(&path, reason) {
            Ok(mut viewer) => {
                viewer.go_to(line, rows);
                self.viewers.insert(id, viewer);
                self.mark_in_step(id, &path);
            }
            Err(e) => self.error = Some(e),
        }
    }

    pub(crate) fn is_viewer(&self, id: DocumentId) -> bool {
        self.viewers.contains_key(&id)
    }

    pub(crate) fn viewer_active(&self) -> bool {
        self.workspace
            .active_id()
            .is_some_and(|id| self.is_viewer(id))
    }

    pub(crate) fn active_viewer(&self) -> Option<&Viewer> {
        self.viewers.get(&self.workspace.active_id()?)
    }

    pub(crate) fn active_viewer_mut(&mut self) -> Option<&mut Viewer> {
        let id = self.workspace.active_id()?;
        self.viewers.get_mut(&id)
    }

    /// Refuse to change or write `id` if it is a viewer tab, and say why.
    ///
    /// The write paths call this by name -- save, copy, encode, edit --
    /// rather than trusting the menus to be greyed: a viewer tab has no
    /// rope, so a save that got through would write an empty document over
    /// a file that is by definition large.
    pub(crate) fn refuse_read_only(&mut self, id: DocumentId) -> bool {
        if !self.is_viewer(id) {
            return false;
        }
        self.error = Some(format!(
            "Read-only: {} is open in the viewer, so it cannot be changed or saved",
            self.display_name(id)
        ));
        true
    }

    /// Whether any viewer still has a thread working, which is what keeps
    /// the window's poll running.
    pub(crate) fn viewers_busy(&self) -> bool {
        self.viewers.values().any(Viewer::busy)
    }

    /// Collect what the viewers' threads have finished. Returns whether the
    /// window should redraw.
    pub(crate) fn poll_viewers(&mut self) -> bool {
        let rows = self.viewer_rows;
        let (columns, tab) = (self.viewer_columns, self.indent.width);
        let active = self.workspace.active_id();
        let mut changed = false;
        let mut said = None;
        for (id, viewer) in &mut self.viewers {
            let (moved, outcome) = viewer.poll(rows);
            changed |= moved;
            if let Some(outcome) = outcome {
                if matches!(outcome, Outcome::Found { .. }) {
                    viewer.reveal_hit(columns, tab);
                }
                if Some(*id) == active {
                    said = Some(outcome);
                }
            }
        }
        if let Some(outcome) = said {
            self.find_status = match outcome {
                Outcome::Found { hit, wrapped } => {
                    let mut status = format!("line {}", grouped(hit.line + 1));
                    if wrapped {
                        status.push_str(" (wrapped round)");
                    }
                    status
                }
                Outcome::NotFound => "no matches".to_owned(),
                Outcome::Failed(e) => e,
                Outcome::Stopped => "stopped".to_owned(),
            };
        } else if let Some(viewer) = self.active_viewer()
            && viewer.searching()
        {
            self.find_status = format!("searching… {}% (Esc stops)", viewer.search_percent());
        }
        changed
    }

    /// Start a search in the active viewer. Returns whether there was one.
    pub(crate) fn viewer_find(
        &mut self,
        query: &bp_search::Query,
        forward: bool,
        fresh: bool,
    ) -> bool {
        let Some(viewer) = self.active_viewer_mut() else {
            return false;
        };
        let status = viewer.find(query.clone(), forward, fresh);
        self.find_status = status;
        true
    }

    /// Stop the active viewer's search, if one is running: the find bar was
    /// closed.
    pub(crate) fn stop_viewer_search(&mut self) {
        let stopped = self.active_viewer_mut().is_some_and(Viewer::stop_search);
        if stopped {
            self.find_status = "stopped".to_owned();
        }
    }

    /// Go to Line in the viewer. `line` is 1-based, as typed.
    pub(crate) fn viewer_go_to(&mut self, line: usize) {
        let rows = self.viewer_rows;
        let Some(viewer) = self.active_viewer_mut() else {
            return;
        };
        let landed = viewer.go_to(line.saturating_sub(1), rows);
        self.goto_status = match landed {
            Landed::There => String::new(),
            Landed::PastEnd(total) => format!("there are only {} lines", grouped(total)),
            Landed::Waiting => "still indexing; it goes there when it can".to_owned(),
        };
    }

    /// A key in the viewer. Returns whether the viewer claimed it; anything
    /// it does not must bubble, or Ctrl+W stops closing the tab.
    pub(crate) fn viewer_key(&mut self, text: &str, control: bool, alt: bool) -> bool {
        use bp_editor::Key;
        let (rows, columns, tab) = (
            self.viewer_rows.max(1),
            self.viewer_columns,
            self.indent.width,
        );
        let page = isize::try_from(rows.saturating_sub(1).max(1)).unwrap_or(1);
        let Some(key) = crate::editor_view::translate_key(text) else {
            return false;
        };
        let Some(viewer) = self.active_viewer_mut() else {
            return false;
        };
        let refused = match key {
            Key::Up => {
                viewer.move_caret(-1, rows);
                None
            }
            Key::Down => {
                viewer.move_caret(1, rows);
                None
            }
            Key::PageUp => {
                viewer.move_caret(-page, rows);
                None
            }
            Key::PageDown => {
                viewer.move_caret(page, rows);
                None
            }
            Key::Home if control => {
                viewer.put_caret(0, rows);
                viewer.left = 0;
                None
            }
            Key::End if control => {
                viewer.go_to_end(rows);
                None
            }
            Key::Home => {
                viewer.left = 0;
                None
            }
            Key::End => {
                viewer.show_line_end(columns, tab);
                None
            }
            Key::Left => {
                viewer.sideways(-isize::try_from(SIDEWAYS_STEP).unwrap_or(1));
                None
            }
            Key::Right => {
                viewer.sideways(isize::try_from(SIDEWAYS_STEP).unwrap_or(1));
                None
            }
            // Ctrl+A has nothing to select, and Insert nothing to overwrite.
            Key::Char('a' | 'A') if control && !alt => None,
            Key::Insert => None,
            // Cut, paste, undo and redo would all change the document.
            Key::Char('x' | 'X' | 'v' | 'V' | 'z' | 'Z' | 'y' | 'Y') if control && !alt => {
                viewer.refuse_typing()
            }
            // Typing, AltGr typing included, and the keys that edit.
            Key::Char(_) if !control || alt => viewer.refuse_typing(),
            Key::Backspace | Key::Delete | Key::Enter | Key::Tab if !control => {
                viewer.refuse_typing()
            }
            _ => return false,
        };
        if let Some(message) = refused {
            self.error = Some(message.to_owned());
        }
        true
    }
}

/// A file the viewer cannot show, and why.
///
/// UTF-16: the viewer reads bytes as UTF-8, and a UTF-16 file read that way
/// is every other character a NUL. And an encrypted `.bpadx`, which would be
/// ciphertext on screen -- and is opened, and unlocked, by the editor.
fn refuse_encoding(path: &Path) -> Option<String> {
    let head = crate::state::read_head(path).ok()?;
    if bp_crypto::is_bpadx(&head) {
        return Some(format!(
            "{} is encrypted; open it without --readonly or --large-file to unlock it",
            path.display()
        ));
    }
    let encoding = bp_core::Encoding::detect_bom(&head);
    matches!(
        encoding,
        bp_core::Encoding::Utf16Le | bp_core::Encoding::Utf16Be
    )
    .then(|| {
        format!(
            "{} is {}, which the large-file viewer cannot read yet; it reads UTF-8",
            path.display(),
            encoding.label()
        )
    })
}

/// The line ending the start of the file uses, for the status bar.
fn sniff_line_ending(path: &Path) -> Option<bp_core::LineEnding> {
    use std::io::Read as _;
    let mut head = vec![0u8; 64 * 1024];
    let read = std::fs::File::open(path)
        .and_then(|mut f| f.read(&mut head))
        .ok()?;
    bp_core::LineEnding::detect(&String::from_utf8_lossy(&head[..read]))
}

/// Draw the active viewer tab into the window.
pub(crate) fn draw(ui: &AppWindow, state: &mut AppState) {
    use std::rc::Rc;
    let (rows, columns, tab) = (state.viewer_rows, state.viewer_columns, state.indent.width);
    let Some(viewer) = state.active_viewer_mut() else {
        return;
    };
    let frame = viewer.frame(rows, columns, tab);
    let clamp = crate::editor_view::clamp_i32;
    ui.set_viewer_banner(frame.banner.into());
    let drawn: Vec<EditorRow> = frame
        .rows
        .into_iter()
        .map(|(number, text)| EditorRow {
            number: number.into(),
            text: text.into(),
        })
        .collect();
    ui.set_viewer_rows(Rc::new(slint::VecModel::from(drawn)).into());
    let boxes: Vec<SelectionBox> = frame
        .highlight
        .map(|(row, start, end)| SelectionBox {
            row: clamp(row),
            start: clamp(start),
            end: clamp(end),
        })
        .into_iter()
        .collect();
    ui.set_viewer_highlight(Rc::new(slint::VecModel::from(boxes)).into());
    let (caret_row, caret_column) = frame
        .caret
        .map_or((-1, 0), |(row, column)| (clamp(row), clamp(column)));
    ui.set_viewer_caret_row(caret_row);
    ui.set_viewer_caret_column(caret_column);
    ui.set_viewer_thumb_position(frame.thumb_position);
    ui.set_viewer_thumb_size(frame.thumb_size);
    ui.set_viewer_gutter_columns(clamp(frame.gutter_columns));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// `line 000001` .. `line NNNNNN`, one per line, with a trailing newline.
    fn numbered(lines: usize) -> String {
        (1..=lines).map(|i| format!("line {i:06}\n")).collect()
    }

    fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    /// A state whose editable limit is `limit` bytes, the lowered threshold
    /// that lets a test open "too large" files a person can read.
    fn limited(limit: u64) -> AppState {
        let mut state = AppState::new();
        state.editable_limit = limit;
        state.viewer_rows = 10;
        state
    }

    /// Poll until every viewer's threads are done, as the window's timer
    /// would, with a generous deadline.
    fn settle(state: &mut AppState) {
        let deadline = Instant::now() + Duration::from_secs(20);
        while state.viewers_busy() && Instant::now() < deadline {
            state.poll_viewers();
            std::thread::sleep(Duration::from_millis(2));
        }
        state.poll_viewers();
        assert!(!state.viewers_busy(), "the viewer's threads never finished");
    }

    fn viewer(state: &mut AppState) -> &mut Viewer {
        state.active_viewer_mut().expect("a viewer tab")
    }

    fn find(state: &mut AppState, pattern: &str) -> String {
        state.find(&bp_search::Query::literal(pattern));
        settle(state);
        state.find_status.clone()
    }

    fn step(state: &mut AppState, pattern: &str, forward: bool) -> String {
        state.viewer_find(&bp_search::Query::literal(pattern), forward, false);
        settle(state);
        state.find_status.clone()
    }

    #[test]
    fn a_file_over_the_limit_opens_in_a_viewer_tab_whose_banner_gives_both_sizes() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "app.log", &numbered(110_000));
        let mut state = limited(1024 * 1024);

        state.open(path.clone());

        assert!(state.error.is_none(), "{:?}", state.error);
        assert!(state.viewer_active());
        assert!(state.active_editor().is_none(), "the rope never saw it");
        assert!(state.recent.paths().contains(&path));
        let frame = viewer(&mut state).frame(10, 80, 4);
        assert_eq!(
            frame.banner,
            "Read-only: this file is too large to edit (1.3 MiB; the limit is 1 MiB)"
        );
        assert_eq!(frame.rows.len(), 10, "a screenful, and only that");
        assert_eq!(frame.rows[0], ("1".to_owned(), "line 000001".to_owned()));
        assert_eq!(
            frame.gutter_columns, 6,
            "room for line 110,001 from the start, before it has been indexed"
        );
    }

    #[test]
    fn a_file_within_the_limit_still_opens_in_the_editor() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "small.txt", &numbered(10));
        let mut state = limited(1024 * 1024);
        state.open(path);
        assert!(!state.viewer_active());
        assert!(state.active_editor().is_some());
    }

    #[test]
    fn readonly_opens_even_a_small_file_in_the_viewer_and_says_why() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "notes.txt", "hello\nworld\n");
        let mut state = AppState::new();

        state.open_as(path, OpenAs::ReadOnly);

        assert!(state.viewer_active());
        let frame = viewer(&mut state).frame(10, 80, 4);
        assert!(frame.banner.contains("--readonly"), "{}", frame.banner);
        let texts: Vec<&str> = frame.rows.iter().map(|(_, t)| t.as_str()).collect();
        assert_eq!(texts, ["hello", "world", ""]);
    }

    #[test]
    fn large_file_forces_the_viewer_for_a_file_small_enough_to_edit() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "data.jsonl", "{}\n{}\n");
        let mut state = AppState::new();
        state.open_as(path, OpenAs::Viewer);
        assert!(state.viewer_active());
        assert!(
            viewer(&mut state)
                .frame(5, 80, 4)
                .banner
                .contains("--large-file")
        );
    }

    #[test]
    fn startup_with_readonly_leaves_one_viewer_tab_and_no_blank_document() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "a.txt", "text\n");
        let mut state = AppState::new();
        state.start_up(None, &[path], OpenAs::ReadOnly);
        assert_eq!(state.workspace.len(), 1);
        assert!(state.viewer_active());
    }

    #[test]
    fn a_utf16_file_is_refused_by_name_rather_than_shown_as_noise() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wide.txt");
        std::fs::write(&path, [0xFF, 0xFE, b'h', 0, b'i', 0]).unwrap();
        let mut state = AppState::new();
        let tabs = state.workspace.len();

        state.open_as(path, OpenAs::ReadOnly);

        assert_eq!(state.workspace.len(), tabs);
        let error = state.error.clone().unwrap_or_default();
        assert!(error.contains("UTF-16"), "{error}");
    }

    #[test]
    fn typing_changes_nothing_and_says_read_only_once_not_per_key() {
        let dir = tempfile::tempdir().unwrap();
        let text = numbered(50);
        let path = write(dir.path(), "a.log", &text);
        let mut state = AppState::new();
        state.open_as(path.clone(), OpenAs::ReadOnly);
        let id = state.workspace.active_id().unwrap();

        assert!(state.viewer_key("x", false, false));
        assert_eq!(state.error.as_deref(), Some(TYPING_REFUSED));
        state.error = None;
        for key in ["y", "\u{8}", "\u{7f}", "\n"] {
            assert!(
                state.viewer_key(key, false, false),
                "{key:?} was not claimed"
            );
        }
        assert!(state.viewer_key("v", true, false), "Ctrl+V");
        assert_eq!(state.error, None, "said once, not per key");

        assert!(!state.is_dirty(id));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        // Ctrl+S and friends are the window's, and must bubble.
        assert!(!state.viewer_key("s", true, false));
    }

    #[test]
    fn saving_or_copying_a_viewer_tab_is_refused_and_the_file_is_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let text = numbered(100);
        let path = write(dir.path(), "keep.log", &text);
        let mut state = limited(64);
        state.open(path.clone());
        let id = state.workspace.active_id().unwrap();

        assert_eq!(
            state.save_document(id, None),
            crate::state::SaveResult::Failed
        );
        assert!(state.error.clone().unwrap().contains("Read-only"));
        let elsewhere = dir.path().join("copy.log");
        assert_eq!(
            state.save_document(id, Some(elsewhere.clone())),
            crate::state::SaveResult::Failed
        );
        assert_eq!(
            state.save_copy(id, &elsewhere),
            crate::state::SaveResult::Failed
        );
        state.set_encoding(bp_core::Encoding::Windows1252);
        state.set_line_ending(bp_core::LineEnding::CrLf);
        state.edit("replaced".to_owned());
        state.mark_edited();

        assert!(!state.is_dirty(id), "nothing may make it look unsaved");
        assert!(!elsewhere.exists());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    }

    #[test]
    fn the_recovery_journal_never_checkpoints_a_viewer_tab() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "a.log", &numbered(10));
        let mut state = AppState::new();
        state.open_as(path, OpenAs::ReadOnly);
        state.checkpoint_all();
        assert!(state.prepare_checkpoints().is_empty());
        assert!(state.journal.pending().is_empty());
    }

    #[test]
    fn go_to_line_lands_on_the_line_and_past_the_end_says_how_many() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "a.log", &numbered(5_000));
        let mut state = limited(1024);
        state.open(path);
        settle(&mut state);

        assert_eq!(state.go_to_line("4321"), None, "the viewer moves itself");
        assert_eq!(state.goto_status, "");
        assert_eq!(viewer(&mut state).line, 4_320);
        let frame = viewer(&mut state).frame(10, 80, 4);
        let caret = frame.caret.expect("the caret's line is on screen");
        assert_eq!(frame.rows[caret.0].1, "line 004321");
        assert!(state.cursor_label().starts_with("Ln 4,321 of 5,001"));

        state.go_to_line("999999");
        assert_eq!(state.goto_status, "there are only 5,001 lines");
        assert_eq!(viewer(&mut state).line, 5_000);
    }

    #[test]
    fn a_line_the_index_has_not_reached_is_gone_to_once_it_has() {
        let dir = tempfile::tempdir().unwrap();
        // Well past the one chunk read on the UI thread at open.
        let path = write(dir.path(), "big.log", &numbered(60_000));
        let mut state = limited(1024);
        state.open(path);
        assert!(!viewer(&mut state).complete());

        state.go_to_line("55555");
        assert!(
            state.goto_status.contains("indexing"),
            "{}",
            state.goto_status
        );
        settle(&mut state);

        assert_eq!(viewer(&mut state).line, 55_554);
        let frame = viewer(&mut state).frame(10, 80, 4);
        assert!(frame.rows.iter().any(|(_, t)| t == "line 055555"));
    }

    #[test]
    fn ctrl_end_reaches_the_last_line_and_ctrl_home_the_first() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "big.log", &numbered(60_000));
        let mut state = limited(1024);
        state.open(path);

        assert!(state.viewer_key(
            &char::from(slint::platform::Key::End).to_string(),
            true,
            false
        ));
        settle(&mut state);
        let frame = viewer(&mut state).frame(10, 80, 4);
        assert_eq!(
            frame.rows.last().unwrap().1,
            "",
            "the empty line after the last newline"
        );
        assert_eq!(frame.rows[frame.rows.len() - 2].1, "line 060000");
        assert!(frame.thumb_position > 0.99, "{}", frame.thumb_position);

        state.viewer_key(
            &char::from(slint::platform::Key::Home).to_string(),
            true,
            false,
        );
        let frame = viewer(&mut state).frame(10, 80, 4);
        assert_eq!(frame.rows[0].1, "line 000001");
        assert!(frame.thumb_position < 0.01);
    }

    #[test]
    fn page_down_and_the_wheel_move_through_the_file_a_screen_at_a_time() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "a.log", &numbered(1_000));
        let mut state = limited(1024);
        state.open(path);
        settle(&mut state);

        let page_down = char::from(slint::platform::Key::PageDown).to_string();
        state.viewer_key(&page_down, false, false);
        state.viewer_key(&page_down, false, false);
        assert_eq!(viewer(&mut state).line, 18, "two pages of nine");
        viewer(&mut state).scroll(100, 10);
        assert_eq!(viewer(&mut state).frame(10, 80, 4).rows[0].1, "line 000110");
        viewer(&mut state).scrub(1.0, 10);
        assert_eq!(viewer(&mut state).frame(10, 80, 4).rows[9].1, "");
    }

    #[test]
    fn find_runs_on_a_background_job_and_lands_on_the_match() {
        let dir = tempfile::tempdir().unwrap();
        let mut text = numbered(20_000);
        text.push_str("the needle is here\n");
        text.push_str(&numbered(100));
        let path = write(dir.path(), "a.log", &text);
        let mut state = limited(1024);
        state.open(path);

        state.find(&bp_search::Query::literal("needle"));
        // Started, not done: the call came back before the search did, and
        // the result arrives only through the poll.
        assert!(
            viewer(&mut state).searching(),
            "the search ran on this thread"
        );
        assert!(
            state.find_status.starts_with("searching"),
            "{}",
            state.find_status
        );
        assert_eq!(viewer(&mut state).hit(), None);

        settle(&mut state);
        assert_eq!(state.find_status, "line 20,001");
        let hit = viewer(&mut state).hit().unwrap();
        assert_eq!((hit.line, hit.start, hit.end), (20_000, 4, 10));
        let frame = viewer(&mut state).frame(10, 80, 4);
        let (row, start, end) = frame.highlight.expect("the match is highlighted");
        assert_eq!(frame.rows[row].1, "the needle is here");
        assert_eq!((start, end), (4, 10));
    }

    #[test]
    fn find_catches_a_match_that_straddles_a_window_boundary() {
        let dir = tempfile::tempdir().unwrap();
        // A line longer than half a window, so windows are cut inside it,
        // with the needle across the first cut.
        let window = bp_buffer::MIN_WINDOW_BYTES;
        let mut long = "x".repeat(window * 3);
        long.replace_range(window - 3..window + 3, "NEEDLE");
        let text = format!("{}{long}\n{}", numbered(3), numbered(3));
        let path = write(dir.path(), "a.log", &text);
        let mut state = limited(1024);
        state.open(path);
        viewer(&mut state).window_bytes = window;

        assert_eq!(find(&mut state, "needle"), "line 4");
        let hit = viewer(&mut state).hit().unwrap();
        assert_eq!((hit.line, hit.start), (3, window - 3));
    }

    #[test]
    fn find_next_steps_past_the_match_and_wraps_round_and_previous_goes_back() {
        let dir = tempfile::tempdir().unwrap();
        let mut text = String::new();
        for i in 0..3_000 {
            if i % 1_000 == 500 {
                text.push_str("marker marker\n");
            } else {
                text.push_str(&format!("filler {i}\n"));
            }
        }
        let path = write(dir.path(), "a.log", &text);
        let mut state = limited(1024);
        state.open(path);
        viewer(&mut state).window_bytes = bp_buffer::MIN_WINDOW_BYTES;

        let at = |state: &mut AppState| {
            let hit = viewer(state).hit().unwrap();
            (hit.line, hit.start)
        };
        assert_eq!(find(&mut state, "marker"), "line 501");
        assert_eq!(at(&mut state), (500, 0));
        step(&mut state, "marker", true);
        assert_eq!(at(&mut state), (500, 7), "the second on the same line");
        step(&mut state, "marker", true);
        assert_eq!(at(&mut state), (1_500, 0));
        step(&mut state, "marker", true);
        step(&mut state, "marker", true);
        step(&mut state, "marker", true);
        assert_eq!(at(&mut state), (2_500, 7));
        assert_eq!(step(&mut state, "marker", true), "line 501 (wrapped round)");
        assert_eq!(at(&mut state), (500, 0));

        assert_eq!(
            step(&mut state, "marker", false),
            "line 2,501 (wrapped round)"
        );
        assert_eq!(at(&mut state), (2_500, 7));
        step(&mut state, "marker", false);
        assert_eq!(at(&mut state), (2_500, 0));
        step(&mut state, "marker", false);
        assert_eq!(at(&mut state), (1_500, 7));
    }

    #[test]
    fn find_says_when_there_is_nothing_and_a_bad_pattern_at_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "a.log", &numbered(100));
        let mut state = limited(64);
        state.open(path);
        assert_eq!(find(&mut state, "absent"), "no matches");

        let bad = bp_search::Query {
            pattern: "(".to_owned(),
            regex: true,
            ..bp_search::Query::default()
        };
        state.find(&bad);
        assert!(
            !viewer(&mut state).searching(),
            "no thread for a bad pattern"
        );
        assert!(
            state.find_status.contains("regular expression"),
            "{}",
            state.find_status
        );
    }

    #[test]
    fn closing_the_find_bar_stops_a_search() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "a.log", &numbered(200_000));
        let mut state = limited(1024);
        state.open(path);
        viewer(&mut state).window_bytes = bp_buffer::MIN_WINDOW_BYTES;

        state.find(&bp_search::Query::literal("absent"));
        state.stop_viewer_search();
        assert!(!viewer(&mut state).searching());
        assert_eq!(state.find_status, "stopped");
    }

    #[test]
    fn bytes_that_are_not_utf8_show_as_replacement_characters_and_do_not_stop_a_search() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mixed.log");
        let mut bytes = b"first\nbad \xFF byte\n".to_vec();
        bytes.extend_from_slice(b"target line\n");
        std::fs::write(&path, &bytes).unwrap();
        let mut state = limited(8);
        state.open(path);

        let frame = viewer(&mut state).frame(10, 80, 4);
        assert_eq!(frame.rows[1].1, "bad \u{FFFD} byte");
        assert_eq!(find(&mut state, "target"), "line 3");
    }

    #[test]
    fn a_bare_cr_ends_a_line_for_find_as_it_does_for_the_rows() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "old-mac.log", "one\rtwo\r\nthree\rxy target\r");
        let mut state = limited(8);
        state.open(path);

        let frame = viewer(&mut state).frame(10, 80, 4);
        assert_eq!(frame.rows[3].1, "xy target");
        assert_eq!(find(&mut state, "target"), "line 4");
        let frame = viewer(&mut state).frame(10, 80, 4);
        let (_, start, _) = frame.highlight.expect("the match is on screen");
        assert_eq!(start, 3, "the column counts from the bare CR, not the LF");
    }

    #[test]
    fn tabs_are_expanded_and_the_view_scrolls_sideways_to_a_match() {
        let dir = tempfile::tempdir().unwrap();
        let text = format!("\tindented\n{}far\n", " ".repeat(200));
        let path = write(dir.path(), "a.log", &text);
        let mut state = limited(8);
        state.viewer_columns = 40;
        state.open(path);

        let frame = viewer(&mut state).frame(10, 40, 4);
        assert_eq!(frame.rows[0].1, "    indented");
        find(&mut state, "far");
        let frame = viewer(&mut state).frame(10, 40, 4);
        let (row, start, _) = frame.highlight.expect("scrolled to show it");
        assert_eq!(row, 1);
        assert!(start < 40);
    }

    #[test]
    fn closing_a_viewer_tab_drops_it_and_reload_reads_a_grown_file_again() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "grow.log", &numbered(10));
        let mut state = limited(8);
        state.open(path.clone());
        settle(&mut state);
        assert!(
            state.cursor_label().ends_with("of 11"),
            "{}",
            state.cursor_label()
        );

        std::fs::write(&path, numbered(20)).unwrap();
        state.reload();
        settle(&mut state);
        assert!(
            state.cursor_label().ends_with("of 21"),
            "{}",
            state.cursor_label()
        );

        let id = state.workspace.active_id().unwrap();
        state.close(id);
        assert!(state.viewers.is_empty());
    }

    #[test]
    fn every_row_that_would_change_the_document_is_greyed_with_a_reason() {
        use crate::menus::{self, action};
        let rows = menus::for_viewer(menus::edit(&[]));
        let find = |id: i32| rows.iter().find(|r| r.action == id).unwrap().clone();
        for id in [
            action::UNDO,
            action::PASTE,
            action::LINES_SORT_ASC,
            action::DUPLICATE_LINE,
        ] {
            let row = find(id);
            assert!(!row.enabled, "{} is live", row.label);
            assert_eq!(row.shortcut.as_str(), "read-only");
        }
        assert!(find(action::GO_TO_LINE).enabled, "Go to Line works");

        let file = menus::for_viewer(menus::file(
            false,
            true,
            &[],
            &crate::default_editor::Rows::default(),
        ));
        let row = |id: i32| file.iter().find(|r| r.action == id).unwrap().clone();
        assert!(!row(action::SAVE).enabled);
        assert!(!row(action::SAVE_AS).enabled);
        assert!(!row(action::SAVE_COPY).enabled);
        assert!(row(action::CLOSE_TAB).enabled);
        assert!(row(action::OPEN).enabled);
        assert!(
            row(action::RELOAD).enabled,
            "a growing log is worth reloading"
        );

        assert_eq!(
            menus::viewer_refusal(action::HASH_DOCUMENT),
            Some("not in the viewer")
        );
        assert_eq!(
            menus::viewer_refusal(action::ENCRYPT_DOCUMENT),
            Some("read-only")
        );
        assert_eq!(menus::viewer_refusal(action::ZOOM_IN), None);
    }

    #[test]
    fn an_encrypted_file_is_not_shown_as_ciphertext() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.bpadx");
        let mut bytes = b"BPADX\0".to_vec();
        bytes.extend_from_slice(&[0u8; 64]);
        std::fs::write(&path, bytes).unwrap();
        let mut state = AppState::new();
        let tabs = state.workspace.len();
        state.open_as(path, OpenAs::ReadOnly);
        assert_eq!(state.workspace.len(), tabs);
        assert!(
            state
                .error
                .clone()
                .unwrap_or_default()
                .contains("encrypted")
        );
    }
}
