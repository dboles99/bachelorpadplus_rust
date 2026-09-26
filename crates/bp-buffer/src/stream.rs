//! Reading a file as text without ever holding the file.
//!
//! `bp-search` owns what a match *is* -- literal or regex, case, word
//! boundaries -- and nothing here duplicates a byte of it. What streaming
//! needs from a buffer crate is the other half: a sequence of `&str` windows
//! that between them cover the file exactly once, each small enough to hand
//! to a matcher, with enough overlap that a match straddling two windows is
//! still found, and enough bookkeeping that a hit inside a window can be
//! reported as a line number in the file.
//!
//! Two boundary defects live here, and both are silent:
//!
//! - **A window split mid-character.** UTF-8 is 1--4 bytes; cutting a 1 MiB
//!   read at a fixed offset lands inside a character roughly three times in
//!   four for CJK text. `String::from_utf8` on that fails, and the tempting
//!   fixes -- lossy decode, or trusting the offset -- corrupt the text or
//!   panic. The incomplete tail is carried to the next read instead.
//! - **A match split mid-window.** Searched independently, neither half
//!   matches, and the hit vanishes with nothing to indicate it. Each window
//!   therefore repeats the tail of the previous one, and
//!   [`StreamWindow::is_new_match`] states the exact rule for not reporting
//!   the same hit twice.
//!
//! Windows begin and end on line breaks wherever the text allows, so that a
//! line number computed inside a window is a line number in the file. A file
//! that is one enormous line is the case that forbids making that a
//! guarantee, which is why [`StreamWindow::starts_mid_line`] exists rather
//! than being assumed away.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::large::LargeFileError;

/// Bytes of file text handed to the matcher at a time.
///
/// 1 MiB because it is two orders of magnitude larger than any pattern
/// anyone types -- so the overlap is noise against it -- and two orders of
/// magnitude smaller than the files this exists for, so the peak stays flat.
pub const DEFAULT_WINDOW_BYTES: usize = 1024 * 1024;

/// The smallest window the reader will honour.
///
/// Below this the UTF-8 carry-over and the line-alignment trim stop being
/// rounding errors and start being most of the window, and progress per read
/// gets bad enough to matter. A caller asking for less gets this.
pub const MIN_WINDOW_BYTES: usize = 4096;

/// How much of each window repeats the previous one, at minimum.
///
/// 4 KiB: any match shorter than this cannot fall between two windows, and
/// nothing a person types into a find box comes close. It is also 0.4% of the
/// default window, which is what makes the repeated scanning free.
///
/// A minimum rather than an exact figure: the repeat is extended backwards to
/// the nearest line start within another `overlap_bytes`, so that windows
/// begin on lines. The extension is capped at twice this, so a bound remains
/// a bound.
pub const DEFAULT_OVERLAP_BYTES: usize = 4096;

/// Ceiling on the overlap, so a caller cannot turn streaming into quadratic
/// re-scanning by asking for an overlap near the window size.
pub const MAX_OVERLAP_BYTES: usize = 64 * 1024;

/// One window of file text, and everything needed to place a hit inside it.
///
/// Byte offsets and line numbers, never absolute *character* offsets: a
/// character offset cannot be known without decoding every byte before it,
/// which is exactly the walk streaming exists to avoid. Offsets *within*
/// [`Self::text`] are characters, matching the rest of this crate and
/// `bp_search::Match::range`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamWindow {
    /// Byte offset in the file at which [`Self::text`] begins.
    pub byte_offset: u64,
    /// Leading bytes of [`Self::text`] that the previous window already
    /// covered.
    pub overlap_bytes: usize,
    /// The same prefix measured in characters, because matches are reported
    /// in characters and comparing the two units is how an off-by-a-few bug
    /// gets into a dedupe rule.
    pub overlap_chars: usize,
    /// 0-based line of the file that [`Self::byte_offset`] falls on, matching
    /// [`crate::Buffer::line_start`]. Add one for display.
    pub first_line: usize,
    /// True when [`Self::byte_offset`] is not the start of a line -- only
    /// possible when a single line is longer than the window.
    pub starts_mid_line: bool,
    /// The window's text. Valid UTF-8, never split mid-character.
    pub text: String,
}

impl StreamWindow {
    /// Whether a match found in this window is new, or was already reported
    /// against the previous one.
    ///
    /// The rule is *end past the overlap*, not *start past the overlap*, and
    /// the difference is a real defect. A match contained entirely in the
    /// repeated prefix was seen whole last time -- skip it. A match that
    /// starts in the prefix and runs past it was **truncated** last time and
    /// therefore never matched at all -- report it. Testing `start` alone
    /// silently drops exactly the matches the overlap exists to catch.
    ///
    /// `range` is in characters, as `bp_search::Match::range` gives it.
    pub fn is_new_match(&self, range: &Range<usize>) -> bool {
        range.end > self.overlap_chars
    }

    /// Turn a 1-based line number within this window -- what
    /// `bp_search::Match::line` reports -- into a 0-based line of the file.
    ///
    /// Correct only when the window starts at a line boundary, which is why
    /// [`Self::starts_mid_line`] is exposed: for a window cut inside a giant
    /// line, its first "line" is a fragment, and a caller reporting it as
    /// line N would be off by however many lines it never saw.
    pub fn absolute_line(&self, line_in_window: usize) -> usize {
        self.first_line + line_in_window.saturating_sub(1)
    }
}

/// Produces [`StreamWindow`]s over a file, holding a bounded amount of it.
///
/// An `Iterator` of `Result`, rather than a method returning everything,
/// because the whole point is that "everything" is never in memory at once --
/// and because an I/O error halfway through a 2 GB file has to reach the
/// caller as an error, not as a short read that looks like the end.
#[derive(Debug)]
pub struct WindowReader {
    path: PathBuf,
    file: File,
    len: u64,
    window_bytes: usize,
    overlap_bytes: usize,
    /// Scratch for one read. Never larger than `window_bytes`.
    buf: Vec<u8>,
    /// Bytes read but not yet emitted: an incomplete character at the end of
    /// a read, and whatever the line-alignment trim gave back.
    carry: Vec<u8>,
    /// The tail of the previous window, to be repeated at the head of the
    /// next one.
    prefix: String,
    /// 0-based line at the start of `prefix`.
    prefix_line: usize,
    prefix_mid_line: bool,
    /// File offset just past the last byte emitted. `prefix` occupies the
    /// bytes immediately before it; `carry` the bytes immediately after.
    emitted_end: u64,
    finished: bool,
    /// Replace bytes that are not UTF-8 rather than stopping at them. See
    /// [`WindowReader::substituting_invalid`].
    substitute: bool,
}

/// What a byte that is not UTF-8 becomes in a window from a reader that is
/// [substituting](WindowReader::substituting_invalid): ASCII SUB, the control
/// character that exists to stand in for one that could not be read.
///
/// One byte for one byte, which is the point. A three-byte U+FFFD would move
/// every offset after it, and [`StreamWindow::byte_offset`] and the line
/// arithmetic both count bytes of the file. Nobody types SUB into a find
/// box, so it matches nothing a person searches for.
pub const INVALID_BYTE_SEARCHED: u8 = 0x1A;

impl WindowReader {
    /// Stream a file with the default window and overlap.
    pub fn open(path: &Path) -> Result<Self, LargeFileError> {
        Self::with_config(path, DEFAULT_WINDOW_BYTES, DEFAULT_OVERLAP_BYTES)
    }

    /// Stream a file with a chosen window and minimum overlap.
    ///
    /// Both are clamped rather than rejected: these are tuning numbers, and a
    /// caller that passes zero wants a sensible default far more than it
    /// wants an error it will `unwrap`. The overlap is additionally clamped
    /// to a quarter of the window, because the line-alignment extension can
    /// double it and an overlap approaching the window size means every byte
    /// is scanned many times.
    pub fn with_config(
        path: &Path,
        window_bytes: usize,
        overlap_bytes: usize,
    ) -> Result<Self, LargeFileError> {
        let window_bytes = window_bytes.max(MIN_WINDOW_BYTES);
        let overlap_bytes = overlap_bytes.min(MAX_OVERLAP_BYTES).min(window_bytes / 4);
        let file = File::open(path).map_err(|source| LargeFileError::io(path, source))?;
        let len = file
            .metadata()
            .map_err(|source| LargeFileError::io(path, source))?
            .len();
        Ok(Self {
            path: path.to_owned(),
            file,
            len,
            window_bytes,
            overlap_bytes,
            buf: Vec::with_capacity(window_bytes),
            carry: Vec::new(),
            prefix: String::new(),
            prefix_line: 0,
            prefix_mid_line: false,
            emitted_end: 0,
            finished: false,
            substitute: false,
        })
    }

    /// Replace each byte that is not UTF-8 with [`INVALID_BYTE_SEARCHED`]
    /// instead of ending the stream with [`LargeFileError::InvalidUtf8`].
    ///
    /// For searching a file that is only being *read*: a log with one stray
    /// byte in its first megabyte would otherwise be unsearchable past it.
    /// Never for anything that writes -- the refusal is what stops a lossy
    /// decode being saved back over the original.
    pub fn substituting_invalid(mut self) -> Self {
        self.substitute = true;
        self
    }

    /// Begin at `byte_offset`, which must be the start of 0-based line
    /// `line`, instead of at the start of the file.
    ///
    /// So "find the next match" from the middle of a 2 GB file reads forward
    /// from there rather than from byte zero. The caller vouches for the
    /// pairing -- [`crate::LargeFile::line_start`] is where it comes from --
    /// because checking it would mean counting every line before it, which is
    /// the walk this exists to skip. Only before the first window; after
    /// that the stream has a position of its own and this changes nothing.
    pub fn start_at(&mut self, byte_offset: u64, line: usize) -> Result<(), LargeFileError> {
        if self.emitted_end != 0 || self.finished || !self.prefix.is_empty() {
            return Ok(());
        }
        self.file
            .seek(SeekFrom::Start(byte_offset))
            .map_err(|source| LargeFileError::io(&self.path, source))?;
        self.emitted_end = byte_offset;
        self.prefix_line = line;
        self.prefix_mid_line = false;
        Ok(())
    }

    /// The file being streamed.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Size of the file when the stream was opened. The denominator for a
    /// progress display; not re-read, because a file changing under a stream
    /// is a reload, not a progress update.
    pub fn len_bytes(&self) -> u64 {
        self.len
    }

    /// Bytes of the file already delivered. Rises monotonically to
    /// [`Self::len_bytes`].
    pub fn bytes_delivered(&self) -> u64 {
        self.emitted_end
    }

    /// The minimum overlap actually in use, after clamping.
    ///
    /// Worth asking for: it is the longest match guaranteed to be found
    /// across a window boundary, so a caller searching for something longer
    /// knows it must raise it. A given window's actual repeat is on the
    /// window ([`StreamWindow::overlap_bytes`]) and lies between this and
    /// twice this.
    pub fn overlap_bytes(&self) -> usize {
        self.overlap_bytes
    }

    /// The window size actually in use, after clamping.
    pub fn window_bytes(&self) -> usize {
        self.window_bytes
    }

    /// Bytes of heap this reader holds between calls.
    ///
    /// Capacities, not lengths, and it excludes the window most recently
    /// handed to the caller -- that one belongs to them. Use
    /// [`Self::memory_bound_bytes`] for the peak including it.
    pub fn resident_bytes(&self) -> usize {
        self.buf.capacity() + self.carry.capacity() + self.prefix.capacity()
    }

    /// The peak this reader will not exceed, whatever the file's size.
    ///
    /// Read buffer, plus the largest carry the line-alignment trim can leave
    /// (half a window, plus up to three bytes of a split character), plus the
    /// repeated prefix, plus the one window handed out. The file's length
    /// does not appear in it, and that is the whole claim.
    pub fn memory_bound_bytes(&self) -> usize {
        self.window_bytes                                // read buffer
            + self.window_bytes / 2 + 4                  // carry after a trim
            + 2 * self.overlap_bytes                     // repeated prefix
            + 2 * self.overlap_bytes + self.window_bytes // the window handed out
    }

    /// Fill `buf` from `carry` plus fresh bytes, up to one window.
    ///
    /// Returns whether the file ended: a short fill can only mean end of
    /// file, because the loop keeps reading until it is full or a read
    /// returns nothing.
    fn fill(&mut self) -> Result<bool, LargeFileError> {
        self.buf.clear();
        self.buf.append(&mut self.carry);
        while self.buf.len() < self.window_bytes {
            let before = self.buf.len();
            self.buf.resize(self.window_bytes, 0);
            let read = self
                .file
                .read(&mut self.buf[before..])
                .map_err(|source| LargeFileError::io(&self.path, source))?;
            self.buf.truncate(before + read);
            if read == 0 {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// The next window, or `None` at the end of the file.
    ///
    /// Separate from the `Iterator` impl so a caller driving this from a
    /// background task can ask for exactly one window per tick without
    /// wrapping the iterator.
    pub fn next_window(&mut self) -> Result<Option<StreamWindow>, LargeFileError> {
        if self.finished {
            return Ok(None);
        }
        let at_eof = self.fill()?;

        // Decode as far as the bytes are valid. An incomplete character at
        // the end of a read is expected -- carry it. An invalid one, or an
        // incomplete one at end of file, is a real error: this crate refuses
        // to guess at bytes that are not text, for the same reason
        // `bp-files` refuses a lossy decode.
        if self.substitute {
            self.substitute_invalid(at_eof);
        }
        let valid = match std::str::from_utf8(&self.buf) {
            Ok(_) => self.buf.len(),
            Err(error) => {
                let valid = error.valid_up_to();
                if error.error_len().is_some() || at_eof {
                    self.finished = true;
                    return Err(LargeFileError::InvalidUtf8 {
                        path: self.path.clone(),
                        offset: self.emitted_end + valid as u64,
                    });
                }
                valid
            }
        };
        let text = std::str::from_utf8(&self.buf[..valid]).expect("validated above");

        // End on a line break when one is close enough to the end that
        // giving the rest back still leaves real progress. Without the
        // half-window floor, a file that is one enormous line would hand back
        // almost everything it read, forever. Failing that, never end on a
        // CR before the end of the file: its LF may be in the next read, and
        // a CRLF split between two windows is counted as two breaks.
        let mut take = text.len();
        if !at_eof {
            match crate::breaks::last_line_start(text.as_bytes(), self.window_bytes / 2, text.len())
            {
                Some(start) => take = start,
                None if text.ends_with('\r') => take -= 1,
                None => {}
            }
        }

        let emitted = &text[..take];
        if emitted.is_empty() {
            // Only the repeated prefix would be left, and that was already
            // delivered. Emitting it again would be a duplicate window.
            self.finished = true;
            return Ok(None);
        }
        let emitted_len = emitted.len();

        let mut window_text = String::with_capacity(self.prefix.len() + emitted_len);
        window_text.push_str(&self.prefix);
        window_text.push_str(emitted);

        let window = StreamWindow {
            byte_offset: self.emitted_end - self.prefix.len() as u64,
            overlap_bytes: self.prefix.len(),
            overlap_chars: self.prefix.chars().count(),
            first_line: self.prefix_line,
            starts_mid_line: self.prefix_mid_line,
            text: window_text,
        };

        self.carry = self.buf[take..].to_vec();
        self.emitted_end += emitted_len as u64;
        self.finished = at_eof;
        self.advance_prefix(&window, emitted_len);

        Ok(Some(window))
    }

    /// Overwrite every byte of `buf` that is not part of valid UTF-8 with
    /// [`INVALID_BYTE_SEARCHED`], in place. An incomplete sequence at the
    /// end is left for the next read to complete -- unless the file has
    /// ended, when nothing will.
    fn substitute_invalid(&mut self, at_eof: bool) {
        let mut from = 0;
        while let Err(error) = std::str::from_utf8(&self.buf[from..]) {
            let at = from + error.valid_up_to();
            let bad = match error.error_len() {
                Some(bad) => bad,
                None if at_eof => self.buf.len() - at,
                None => return,
            };
            for byte in &mut self.buf[at..at + bad] {
                *byte = INVALID_BYTE_SEARCHED;
            }
            from = at + bad;
        }
    }

    /// Choose the tail of `window` to repeat at the head of the next one.
    ///
    /// Two constraints pull against each other. The repeat must be at least
    /// `overlap_bytes` long, or a straddling match is lost; and it must start
    /// at a line boundary, or the next window's `first_line` describes the
    /// middle of a line and every line number derived from it is off by one.
    /// A line boundary is wherever the text puts it, so: take the minimum,
    /// extend backwards to the nearest line start within another
    /// `overlap_bytes`, and give up on alignment beyond that rather than let
    /// one giant line drag the repeat up to the size of the window.
    ///
    /// It may never reach back past what was newly emitted, or the stream
    /// would re-deliver its own prefix and stop advancing.
    fn advance_prefix(&mut self, window: &StreamWindow, emitted_len: usize) {
        let text = &window.text;
        let floor = text.len() - emitted_len;
        let want = self.overlap_bytes.min(emitted_len);
        let mut split = text.len() - want;
        while !text.is_char_boundary(split) {
            split += 1;
        }
        // How far back to look for a line start. Rounded up to a character
        // boundary, not merely clamped: `split - want` lands inside a
        // multi-byte character whenever the text has any, and slicing there
        // panics.
        let mut reach = split.saturating_sub(want).max(floor);
        while !text.is_char_boundary(reach) {
            reach += 1;
        }
        if let Some(start) = crate::breaks::last_line_start(text.as_bytes(), reach + 1, split) {
            split = start;
        }

        // Breaks on the rule the rest of the crate uses (decision D6). A CR
        // ending `head` whose LF opens the repeat is half of a CRLF: the next
        // window holds the LF and counts the pair there, once.
        let head = &text[..split];
        let pair_cut = head.ends_with('\r') && text.as_bytes().get(split) == Some(&b'\n');
        self.prefix_line =
            window.first_line + crate::breaks::count(head.as_bytes()) - usize::from(pair_cut);
        self.prefix_mid_line = if split == 0 {
            window.starts_mid_line
        } else {
            !(head.ends_with('\n') || (head.ends_with('\r') && !pair_cut))
        };
        self.prefix = text[split..].to_owned();
    }
}

impl Iterator for WindowReader {
    type Item = Result<StreamWindow, LargeFileError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_window().transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write(dir: &TempDir, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.path().join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(bytes).unwrap();
        file.sync_all().unwrap();
        path
    }

    /// Reassemble the file from its windows, dropping each window's repeated
    /// prefix. If the windows cover the file exactly once, this is the file.
    fn reassemble(path: &Path, window: usize, overlap: usize) -> String {
        let reader = WindowReader::with_config(path, window, overlap).unwrap();
        let mut out = String::new();
        for w in reader {
            let w = w.unwrap();
            out.push_str(&w.text[w.overlap_bytes..]);
        }
        out
    }

    #[test]
    fn the_windows_cover_the_file_exactly_once_at_every_window_size() {
        let dir = TempDir::new().unwrap();
        let corpus: &[&str] = &[
            "",
            "\n",
            "a",
            "short file\nwith three\nlines\n",
            "no trailing newline",
            "crlf\r\nand\r\nmore\r\n",
            "日本語のテキストと emoji 🙂🙃 mixed in\nsecond line ここ\n",
        ];
        for (n, text) in corpus.iter().enumerate() {
            let path = write(&dir, &format!("cover{n}.txt"), text.as_bytes());
            for window in [MIN_WINDOW_BYTES, MIN_WINDOW_BYTES + 7, 1 << 20] {
                for overlap in [0, 1, 64, MIN_WINDOW_BYTES] {
                    assert_eq!(
                        reassemble(&path, window, overlap),
                        **text,
                        "{text:?} at window {window} overlap {overlap}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_long_file_is_covered_exactly_once_at_every_window_size_around_the_boundary() {
        let dir = TempDir::new().unwrap();
        // Multi-byte characters, mixed line lengths, and a final line with no
        // newline: the combination that puts a character, a line break and a
        // window edge on top of each other.
        let mut text = String::new();
        for i in 0..4_000 {
            match i % 4 {
                0 => text.push_str("plain ascii line\n"),
                1 => text.push_str("日本語の行です\r\n"),
                2 => text.push_str("🙂🙃🙂🙃 emoji line\n"),
                _ => text.push_str(&"x".repeat(i % 97)),
            }
            if i % 4 == 3 {
                text.push('\n');
            }
        }
        text.push_str("last line with no newline");
        let path = write(&dir, "mixed.txt", text.as_bytes());

        for window in [
            MIN_WINDOW_BYTES,
            MIN_WINDOW_BYTES + 1,
            MIN_WINDOW_BYTES + 3,
            8191,
            8192,
            8193,
        ] {
            for overlap in [0, 1, 7, 512] {
                assert_eq!(
                    reassemble(&path, window, overlap),
                    text,
                    "window {window} overlap {overlap}"
                );
            }
        }
    }

    #[test]
    fn a_window_never_splits_a_character_even_when_every_boundary_would() {
        let dir = TempDir::new().unwrap();
        // Three-byte characters against a 4096-byte window: every window
        // boundary lands inside a character unless the carry-over works.
        let text: String = "日".repeat(20_000);
        let path = write(&dir, "cjk.txt", text.as_bytes());

        let mut reader = WindowReader::with_config(&path, MIN_WINDOW_BYTES, 0).unwrap();
        let mut rebuilt = String::new();
        let mut windows = 0;
        while let Some(w) = reader.next_window().unwrap() {
            assert!(!w.text.is_empty(), "a window must carry something");
            rebuilt.push_str(&w.text[w.overlap_bytes..]);
            windows += 1;
        }
        assert!(windows > 10, "the file must actually span many windows");
        assert_eq!(rebuilt, text);
    }

    #[test]
    fn a_match_across_a_window_boundary_is_reported_exactly_once() {
        let dir = TempDir::new().unwrap();
        // Filler with no line breaks, so windows cannot be line-aligned and
        // land on exact multiples of the window size. Needles are planted at
        // varying distances either side of those multiples, so some straddle
        // and some end exactly on one.
        let needle = "NEEDLE";
        let mut bytes = vec![b'a'; 200_000];
        let mut planted = Vec::new();
        for k in 1..40usize {
            let at = MIN_WINDOW_BYTES * k - (k % 6) - 1;
            bytes[at..at + needle.len()].copy_from_slice(needle.as_bytes());
            planted.push(at as u64);
        }
        let path = write(&dir, "needles.txt", &bytes);

        let reader = WindowReader::with_config(&path, MIN_WINDOW_BYTES, 64).unwrap();
        let mut found = Vec::new();
        for w in reader {
            let w = w.unwrap();
            for (byte, _) in w.text.match_indices(needle) {
                // Character and byte offsets coincide for this ASCII
                // fixture, which is why the dedupe rule can be exercised
                // directly against `match_indices`.
                let range = byte..byte + needle.len();
                if w.is_new_match(&range) {
                    found.push(w.byte_offset + byte as u64);
                }
            }
        }
        assert_eq!(found, planted, "every needle, once, in order");
    }

    #[test]
    fn line_numbers_survive_across_windows() {
        let dir = TempDir::new().unwrap();
        let text: String = (0..5_000).map(|i| format!("line {i}\n")).collect();
        let path = write(&dir, "lines.txt", text.as_bytes());

        let reader = WindowReader::with_config(&path, MIN_WINDOW_BYTES, 128).unwrap();
        let mut checked = 0;
        for w in reader {
            let w = w.unwrap();
            assert!(
                !w.starts_mid_line,
                "these lines are short, so alignment must hold"
            );
            // Line k of the window, 1-based, must be file line
            // `first_line + k - 1`, and the fixture says which line it is.
            for (k, line) in w.text.lines().enumerate() {
                let absolute = w.absolute_line(k + 1);
                assert_eq!(
                    line,
                    format!("line {absolute}"),
                    "window at byte {} line {k}",
                    w.byte_offset
                );
                checked += 1;
            }
        }
        assert!(checked >= 5_000, "only checked {checked} lines");
    }

    #[test]
    fn line_numbers_survive_crlf_and_multi_byte_lines() {
        let dir = TempDir::new().unwrap();
        let text: String = (0..3_000).map(|i| format!("行 {i} ここ\r\n")).collect();
        let path = write(&dir, "crlf.txt", text.as_bytes());

        let reader = WindowReader::with_config(&path, MIN_WINDOW_BYTES, 64).unwrap();
        let mut checked = 0;
        for w in reader {
            let w = w.unwrap();
            assert!(!w.starts_mid_line);
            for (k, line) in w.text.lines().enumerate() {
                // `str::lines` strips the whole CRLF, which is what makes a
                // window's line numbering comparable with the file's.
                let absolute = w.absolute_line(k + 1);
                assert_eq!(line, format!("行 {absolute} ここ"), "line {k}");
                checked += 1;
            }
        }
        assert!(checked >= 3_000, "only checked {checked} lines");
    }

    #[test]
    fn line_numbers_survive_bare_crs_and_crlfs_cut_between_windows() {
        // Decision D6: a bare CR ends a line, a CRLF is one break wherever a
        // window boundary falls. Several window sizes and overlaps, zero
        // among them, so boundaries land on every part of every ending.
        let dir = TempDir::new().unwrap();
        let mut text = String::new();
        for i in 0..4_000 {
            text.push_str(&format!("line {i}"));
            text.push_str(["\r", "\n", "\r\n"][i % 3]);
        }
        let path = write(&dir, "mixed.txt", text.as_bytes());

        for window in [MIN_WINDOW_BYTES, MIN_WINDOW_BYTES + 1, MIN_WINDOW_BYTES + 7] {
            for overlap in [0, 1, 64] {
                let reader = WindowReader::with_config(&path, window, overlap).unwrap();
                let mut checked = 0;
                for w in reader {
                    let w = w.unwrap();
                    // A one-byte overlap can rarely reach back to a line
                    // start, so a window may open on a fragment of its first
                    // line. Its number is still right; its text is partial.
                    assert!(
                        overlap == 1 || !w.starts_mid_line,
                        "window {window} overlap {overlap} lost its alignment"
                    );
                    let skip = usize::from(w.starts_mid_line);
                    for (k, line) in crate::breaks::lines(&w.text).enumerate().skip(skip) {
                        let absolute = w.absolute_line(k + 1);
                        assert_eq!(
                            line,
                            format!("line {absolute}"),
                            "window {window} overlap {overlap} at byte {}",
                            w.byte_offset
                        );
                        checked += 1;
                    }
                }
                assert!(checked >= 3_900, "only checked {checked} lines");
            }
        }
    }

    #[test]
    fn one_enormous_line_streams_and_says_it_is_mid_line() {
        let dir = TempDir::new().unwrap();
        let text = "x".repeat(300_000);
        let path = write(&dir, "one_line.txt", text.as_bytes());

        let mut reader = WindowReader::with_config(&path, MIN_WINDOW_BYTES, 32).unwrap();
        let first = reader.next_window().unwrap().unwrap();
        assert!(!first.starts_mid_line, "the first window starts at line 0");
        assert_eq!(first.first_line, 0);

        let mut count = 1;
        while let Some(w) = reader.next_window().unwrap() {
            assert!(w.starts_mid_line, "there is no line break to align to");
            assert_eq!(w.first_line, 0, "it is all line 0");
            count += 1;
        }
        assert!(count > 50, "only {count} windows for 300 KB at 4 KB each");
    }

    #[test]
    fn an_empty_file_yields_no_windows() {
        let dir = TempDir::new().unwrap();
        let path = write(&dir, "empty.txt", b"");
        let mut reader = WindowReader::open(&path).unwrap();
        assert_eq!(reader.next_window().unwrap(), None);
        assert_eq!(reader.next_window().unwrap(), None, "and stays ended");
    }

    #[test]
    fn invalid_utf8_is_an_error_with_the_offset_that_failed() {
        let dir = TempDir::new().unwrap();
        let mut bytes = b"good text here\n".to_vec();
        bytes.extend_from_slice(&[0xFF, 0xFE]);
        bytes.extend_from_slice(b"\nmore\n");
        let path = write(&dir, "bad.txt", &bytes);

        let mut reader = WindowReader::open(&path).unwrap();
        match reader.next_window() {
            Err(LargeFileError::InvalidUtf8 { offset, .. }) => assert_eq!(offset, 15),
            other => panic!("expected an InvalidUtf8 at 15, got {other:?}"),
        }
    }

    #[test]
    fn a_truncated_character_at_the_end_of_the_file_is_an_error_not_a_silent_drop() {
        let dir = TempDir::new().unwrap();
        // The first two bytes of a three-byte character, and nothing after.
        let path = write(&dir, "truncated.txt", &[0xE6, 0x97]);
        let mut reader = WindowReader::open(&path).unwrap();
        assert!(
            matches!(
                reader.next_window(),
                Err(LargeFileError::InvalidUtf8 { .. })
            ),
            "a file ending mid-character is malformed, not empty"
        );
    }

    #[test]
    fn memory_stays_bounded_over_a_file_far_larger_than_the_bound() {
        let dir = TempDir::new().unwrap();
        let block: String = (0..16_384).map(|i| format!("line {i} of text\n")).collect();
        let path = dir.path().join("big.txt");
        {
            let mut file = File::create(&path).unwrap();
            // ~48 MiB, more than ten times the reader's bound.
            for _ in 0..(48 * 1024 * 1024 / block.len() + 1) {
                file.write_all(block.as_bytes()).unwrap();
            }
            file.sync_all().unwrap();
        }
        let len = std::fs::metadata(&path).unwrap().len();

        let mut reader = WindowReader::open(&path).unwrap();
        let bound = reader.memory_bound_bytes();
        assert!(
            (len as usize) > bound * 10,
            "the fixture must dwarf the bound: {len} vs {bound}"
        );

        while let Some(w) = reader.next_window().unwrap() {
            let held = reader.resident_bytes() + w.text.capacity();
            assert!(held <= bound, "held {held} bytes, bound is {bound}");
        }
        assert_eq!(reader.bytes_delivered(), len, "and it read all of it");
    }

    #[test]
    fn the_window_and_overlap_are_clamped_rather_than_trusted() {
        let dir = TempDir::new().unwrap();
        let path = write(&dir, "clamp.txt", b"hello\n");
        let reader = WindowReader::with_config(&path, 0, usize::MAX).unwrap();
        assert_eq!(reader.window_bytes(), MIN_WINDOW_BYTES);
        assert_eq!(reader.overlap_bytes(), MIN_WINDOW_BYTES / 4);
        assert_eq!(reader.len_bytes(), 6);
    }

    #[test]
    fn opening_a_file_that_is_not_there_is_an_error_naming_it() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("absent.txt");
        let error = WindowReader::open(&path).unwrap_err();
        assert!(
            error.to_string().contains("absent.txt"),
            "the message must name the file: {error}"
        );
    }

    #[test]
    fn a_substituting_reader_keeps_every_offset_and_finds_text_past_a_bad_byte() {
        let dir = TempDir::new().unwrap();
        let mut bytes = Vec::new();
        for i in 0..2_000 {
            bytes.extend_from_slice(format!("line {i}\n").as_bytes());
            if i == 10 {
                bytes.extend_from_slice(b"bad \xFF\xC3 here\n");
            }
        }
        bytes.extend_from_slice(b"tail \xE2\x82");
        let path = write(&dir, "bad.txt", &bytes);

        assert!(
            WindowReader::with_config(&path, MIN_WINDOW_BYTES, 64)
                .unwrap()
                .any(|w| w.is_err()),
            "without substitution the stream refuses, as it always has"
        );

        let reader = WindowReader::with_config(&path, MIN_WINDOW_BYTES, 64)
            .unwrap()
            .substituting_invalid();
        let mut out = Vec::new();
        for w in reader {
            let w = w.unwrap();
            out.extend_from_slice(&w.text.as_bytes()[w.overlap_bytes..]);
        }
        assert_eq!(out.len(), bytes.len(), "one byte for one byte");
        let expected: Vec<u8> = bytes
            .iter()
            .map(|b| {
                if *b >= 0x80 {
                    INVALID_BYTE_SEARCHED
                } else {
                    *b
                }
            })
            .collect();
        assert_eq!(out, expected);
    }

    #[test]
    fn a_reader_started_at_a_line_reports_lines_and_offsets_in_the_file() {
        let dir = TempDir::new().unwrap();
        let text: String = (0..5_000).map(|i| format!("entry {i}\n")).collect();
        let path = write(&dir, "mid.txt", text.as_bytes());
        let line = 3_210;
        let offset = text.match_indices('\n').nth(line - 1).unwrap().0 as u64 + 1;

        let mut reader = WindowReader::with_config(&path, MIN_WINDOW_BYTES, 64).unwrap();
        reader.start_at(offset, line).unwrap();
        let first = reader.next_window().unwrap().unwrap();
        assert_eq!(first.byte_offset, offset);
        assert_eq!(first.first_line, line);
        assert!(first.text.starts_with("entry 3210\n"));

        let mut rest = first.text.clone();
        while let Some(w) = reader.next_window().unwrap() {
            let found = w
                .text
                .find("entry 4999")
                .map(|at| w.text[..at].matches('\n').count());
            if let Some(newlines) = found {
                assert_eq!(
                    w.first_line + newlines,
                    4_999,
                    "line arithmetic from a mid start"
                );
            }
            rest.push_str(&w.text[w.overlap_bytes..]);
        }
        assert_eq!(
            rest,
            text[offset as usize..],
            "from there to the end, exactly once"
        );
    }
}
