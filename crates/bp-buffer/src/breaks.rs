//! What ends a line: LF, CRLF or a bare CR, and nothing else (decision D6).
//!
//! The Notepad rule. A bare CR is what a classic Mac file, or a program that
//! drew a progress bar, leaves behind, and every editor a user is likely to
//! compare this one with starts a new line there. Form feed, vertical tab,
//! U+0085, U+2028 and U+2029 do not end a line: Unicode says they may, but no
//! editor people use treats them so, and a document that happens to contain
//! one would otherwise gain lines nobody can see the reason for.
//!
//! One definition, used by the rope (built with `ropey`'s `cr_lines` and
//! without `unicode_lines`, see `Cargo.toml`), by the large-file index and
//! viewer, and by every count of lines in this crate. `bp-search` and the
//! other crates that number lines without depending on this one carry the
//! same rule, and `tests/integration` holds them to it: two definitions of a
//! line are how Find in Files came to jump to the wrong one.
//!
//! Byte-level throughout. CR and LF are single bytes below `0x80`, so neither
//! can hide inside a multi-byte UTF-8 character and a scan over raw bytes
//! cannot be misled by one.

/// Bytes of the line break starting at byte `at`: 2 for CRLF, 1 for LF or a
/// bare CR, 0 if there is none there.
#[must_use]
pub fn break_len(bytes: &[u8], at: usize) -> usize {
    match (bytes.get(at), bytes.get(at + 1)) {
        (Some(b'\r'), Some(b'\n')) => 2,
        (Some(b'\n' | b'\r'), _) => 1,
        _ => 0,
    }
}

/// Line breaks in `bytes`. A CRLF is one.
#[must_use]
pub fn count(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .enumerate()
        .filter(|&(i, &b)| b == b'\n' || (b == b'\r' && bytes.get(i + 1) != Some(&b'\n')))
        .count()
}

/// The 1-based line that byte `offset` of `bytes` falls on.
///
/// Looks past `offset` for the one case that needs it: the LF of a CRLF is
/// on the line its CR ends, as it is in the rope, so a CR just before
/// `offset` does not count when the LF is at `offset`.
#[must_use]
pub fn line_of(bytes: &[u8], offset: usize) -> usize {
    let head = bytes.get(..offset).unwrap_or(bytes);
    head.iter()
        .enumerate()
        .filter(|&(i, &b)| b == b'\n' || (b == b'\r' && bytes.get(i + 1) != Some(&b'\n')))
        .count()
        + 1
}

/// `line` without the one line break it may end with.
#[must_use]
pub fn strip(line: &str) -> &str {
    line.strip_suffix("\r\n")
        .or_else(|| line.strip_suffix('\n'))
        .or_else(|| line.strip_suffix('\r'))
        .unwrap_or(line)
}

/// Each line's byte offset and its text without the break, in order.
///
/// Shaped like `str::lines`, which callers used before: no entry for the
/// empty line after a trailing break, and none at all for empty text. The
/// difference is the rule, not the shape -- `str::lines` does not end a line
/// at a bare CR.
#[must_use]
pub fn spans(text: &str) -> Spans<'_> {
    Spans { text, at: 0 }
}

/// The lines of `text` without their breaks: [`spans`] without the offsets.
pub fn lines(text: &str) -> impl Iterator<Item = &str> {
    spans(text).map(|(_, line)| line)
}

/// Iterator returned by [`spans`].
#[derive(Debug, Clone)]
pub struct Spans<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Iterator for Spans<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.at;
        let rest = self.text.get(start..).filter(|rest| !rest.is_empty())?;
        let bytes = rest.as_bytes();
        match bytes.iter().position(|&b| b == b'\n' || b == b'\r') {
            Some(end) => {
                self.at = start + end + break_len(bytes, end);
                // `end` is a CR or LF byte, and so a character boundary.
                Some((start, rest.get(..end).unwrap_or(rest)))
            }
            None => {
                self.at = self.text.len();
                Some((start, rest))
            }
        }
    }
}

/// The last line start `s` with `lo <= s <= hi` that `bytes` alone proves.
///
/// An LF proves one. A CR proves one only when the byte after it is present
/// and is not an LF: a CR as the last byte of what is in hand may be the
/// first half of a CRLF, and a line start between the two is a position on
/// no line at all. The offset is always a character boundary, since the byte
/// before it is ASCII.
pub(crate) fn last_line_start(bytes: &[u8], lo: usize, hi: usize) -> Option<usize> {
    (lo.max(1)..=hi.min(bytes.len()))
        .rev()
        .find(|&s| match bytes.get(s - 1) {
            Some(b'\n') => true,
            Some(b'\r') => bytes.get(s).is_some_and(|&next| next != b'\n'),
            _ => false,
        })
}

/// Finds line starts in text that arrives in pieces.
///
/// A CRLF split across two pieces is the case this exists for: the CR alone
/// looks like a bare CR, and deciding on it before the next piece arrives
/// would count one break as two. So a CR at the end of a piece is held until
/// the next byte is known, and [`Self::finish`] settles one left at the end.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Scanner {
    pending_cr: bool,
}

impl Scanner {
    /// Scan the next piece, whose first byte is at absolute offset `base`,
    /// reporting the offset of each line start found.
    pub(crate) fn feed(&mut self, piece: &[u8], base: u64, mut line_starts: impl FnMut(u64)) {
        for (i, &b) in (0u64..).zip(piece) {
            let at = base + i;
            if std::mem::take(&mut self.pending_cr) && b != b'\n' {
                line_starts(at);
            }
            match b {
                b'\n' => line_starts(at + 1),
                b'\r' => self.pending_cr = true,
                _ => {}
            }
        }
    }

    /// The text ended at `end`. A CR held back is a bare one, and like a
    /// trailing LF it starts an empty last line.
    pub(crate) fn finish(&mut self, end: u64, mut line_starts: impl FnMut(u64)) {
        if std::mem::take(&mut self.pending_cr) {
            line_starts(end);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lf_crlf_and_a_bare_cr_each_end_one_line() {
        assert_eq!(count(b"a\nb\r\nc\rd"), 3);
        assert_eq!(count(b"\r\n\r\n"), 2, "a CRLF is one break");
        assert_eq!(count(b"\r\r\n"), 2, "a bare CR, then a CRLF");
        assert_eq!(count(b"\n\r"), 2, "LF then CR is two, not a pair");
    }

    #[test]
    fn form_feed_vertical_tab_and_the_unicode_separators_do_not_end_a_line() {
        for text in ["a\x0Cb", "a\x0Bb", "a\u{0085}b", "a\u{2028}b", "a\u{2029}b"] {
            assert_eq!(count(text.as_bytes()), 0, "{text:?}");
            assert_eq!(lines(text).count(), 1, "{text:?}");
        }
    }

    #[test]
    fn spans_split_like_str_lines_but_on_every_break() {
        let text = "one\r\ntwo\rthree\nfour";
        let got: Vec<_> = spans(text).collect();
        assert_eq!(
            got,
            [(0, "one"), (5, "two"), (9, "three"), (15, "four")],
            "offsets must land on each line's first byte"
        );
        for plain in ["", "a", "a\n", "a\n\n", "\n", "a\r\nb\r\n"] {
            assert_eq!(
                lines(plain).collect::<Vec<_>>(),
                plain.lines().collect::<Vec<_>>(),
                "{plain:?}"
            );
        }
        assert_eq!(lines("a\rb\r").collect::<Vec<_>>(), ["a", "b"]);
    }

    #[test]
    fn line_of_agrees_with_the_rope_at_every_offset() {
        let text = "a\r\nb\rc\n\r\x0Cd\r";
        let rope = crate::Buffer::from_text(text);
        for offset in 0..=text.len() {
            assert_eq!(
                line_of(text.as_bytes(), offset),
                rope.position_of(offset).line,
                "offset {offset}"
            );
        }
    }

    #[test]
    fn strip_removes_exactly_one_break_of_any_kind() {
        assert_eq!(strip("a\r\n"), "a");
        assert_eq!(strip("a\n"), "a");
        assert_eq!(strip("a\r"), "a");
        assert_eq!(strip("a\n\n"), "a\n");
        assert_eq!(strip("a"), "a");
    }

    #[test]
    fn a_line_start_is_proved_only_where_the_break_is_certain() {
        assert_eq!(last_line_start(b"a\nb", 0, 3), Some(2));
        assert_eq!(last_line_start(b"a\rb", 0, 3), Some(2));
        assert_eq!(
            last_line_start(b"a\r", 0, 2),
            None,
            "the CR may yet be a CRLF"
        );
        assert_eq!(
            last_line_start(b"a\r\nb", 0, 4),
            Some(3),
            "after the pair, never between"
        );
        assert_eq!(last_line_start(b"a\nb\nc", 0, 3), Some(2), "at most hi");
        assert_eq!(last_line_start(b"a\nb\nc", 3, 5), Some(4));
        assert_eq!(last_line_start(b"a\nbc", 3, 5), None, "at least lo");
    }

    /// Every line start, the slow way, from the whole text at once.
    fn whole(text: &[u8]) -> Vec<u64> {
        let mut starts = vec![0];
        let mut at = 0;
        while at < text.len() {
            let len = break_len(text, at);
            at += len.max(1);
            if len > 0 {
                starts.push(at as u64);
            }
        }
        starts
    }

    #[test]
    fn the_scanner_agrees_with_a_whole_text_scan_however_the_text_is_cut() {
        // Every cut of a text dense with every kind of break, including a
        // CRLF split between two pieces and a CR as the very last byte.
        let text = b"a\r\nb\rc\n\r\r\n\rd\r";
        for cut in 0..=text.len() {
            let mut starts = vec![0];
            let mut scanner = Scanner::default();
            let (head, tail) = text.split_at(cut);
            scanner.feed(head, 0, |s| starts.push(s));
            scanner.feed(tail, cut as u64, |s| starts.push(s));
            scanner.finish(text.len() as u64, |s| starts.push(s));
            assert_eq!(starts, whole(text), "cut at {cut}");
        }
    }
}
