//! Turning a document back into bytes: the inverse of [`load`].
//!
//! [`load`]: crate::load
//!
//! This lives here, next to `load`, because it is the same decision seen from
//! the other side. While it lived in the Slint shell as a `pub(crate)`
//! function, no test outside a window could reach the code that decides what
//! actually lands on the user's disk -- the cross-crate tests could prove the
//! *model* lossless and could not touch the writer. The property that makes
//! the pair worth having is stated and tested below: write then load returns
//! what you wrote.

use std::borrow::Cow;

use bp_core::{Encoding, LineEnding, Unrepresentable, encode_windows_1252};
use thiserror::Error;

use crate::utf16::{self, Endian};

/// Why [`try_encode`] could not produce bytes.
///
/// One variant today, and only because there is one encoding that cannot
/// hold every `char`. An enum rather than [`Unrepresentable`] itself so that
/// a second legacy code page, or a second way to fail, is an addition the
/// shell's `match` is told about rather than a signature change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum EncodeError {
    /// The document holds a character the encoding has no bytes for.
    ///
    /// The shell's cue to ask "save as UTF-8 instead?" -- not to write a
    /// `?` in its place, which is how a user loses a character without
    /// being told. The offset is into the text as passed, before any line
    /// ending conversion, so it is a position in the editor's own buffer.
    #[error("{0}; UTF-8 can hold it")]
    Unrepresentable(#[from] Unrepresentable),
}

/// What [`encode`] does with the line breaks the text already has.
///
/// A choice rather than a fixed behaviour because the two callers want
/// opposite things and both are right. Saving a file the user opened should
/// return the bytes they were given; applying an explicit "convert to CRLF"
/// should rewrite every break. Folding those into one silent rule -- which is
/// what the shell did -- means a mixed-ending document is quietly rewritten
/// on every save, and nothing can tell that it happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineEndingPolicy {
    /// Write the text's line breaks through unchanged, byte for byte.
    ///
    /// The default, because it is the only policy under which `encode` is a
    /// true inverse of `load` for every document. A file that was mixed when
    /// it was opened is still mixed when it is saved.
    #[default]
    Preserve,
    /// Rewrite every line break to this convention.
    ///
    /// **This is lossy on purpose, and only for the minority convention in a
    /// mixed document.** An LF break in a document saved as CRLF becomes
    /// CRLF; there is no way to get it back. Use
    /// [`LineEndingSurvey::breaks_rewritten_by`] before calling to find out
    /// how many breaks that will be -- zero for the overwhelmingly common
    /// case of a document that already uses one convention throughout.
    ///
    /// A bare carriage return is data, not a break, and is never rewritten.
    /// That has one visible consequence, reported by
    /// [`LineEndingSurvey::converges_to`]: converting `"\r\r\n"` to LF leaves
    /// the bare `\r` in front of the new `\n`, which reads back as CRLF.
    ConvertTo(LineEnding),
}

/// A count of the line-break conventions present in a document.
///
/// Exists so that "saving this will converge it" is something a caller can
/// *know* rather than infer. `bp_core::LineEnding::detect` answers "which
/// convention is this document", by majority, and throws the minority away in
/// the process; the status bar needs no more than that. Deciding whether to
/// warn before a save needs the counts themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineEndingSurvey {
    /// Bare line feeds -- `\n` not preceded by `\r`.
    pub lf: usize,
    /// Carriage-return line-feed pairs.
    pub crlf: usize,
    /// Carriage returns with no line feed after them.
    ///
    /// Counted and reported, never treated as a line break: `LineEnding` has
    /// no variant for classic Mac endings, so calling a bare `\r` a break
    /// would mean converting it to something, and converting a `\r` that was
    /// really data -- in a paste from a terminal, say -- is exactly the
    /// silent rewrite this module exists to avoid. They pass through
    /// untouched under every policy.
    ///
    /// Since decision D6 a bare `\r` *does* end a line everywhere lines are
    /// counted or shown -- the rope, Ln/Col, Go to Line, search -- so this
    /// module is now the one place that treats it as data. That is a
    /// conversion policy rather than a line count, and it stays until
    /// `LineEnding` can name a classic Mac file; see DECISIONS.md.
    pub lone_cr: usize,
    /// Bare carriage returns that sit immediately before a line break, as in
    /// `"\r\r\n"` -- the fingerprint of a document some other tool has
    /// already converted once.
    ///
    /// A subset of [`lone_cr`](Self::lone_cr), broken out because it is the
    /// one thing that stops a conversion to LF from converging: rewriting the
    /// break to `\n` leaves the untouched `\r` sitting in front of it, which
    /// on reload is a CRLF again. See [`converges_to`](Self::converges_to).
    pub cr_before_break: usize,
}

impl LineEndingSurvey {
    /// Count the line breaks in `text`.
    ///
    /// One pass, byte-wise. `\r` and `\n` are ASCII, so they cannot occur
    /// inside a multi-byte UTF-8 sequence and a byte scan cannot land in the
    /// middle of a character.
    pub fn of(text: &str) -> Self {
        let mut survey = Self::default();
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\n' => {
                    survey.lf += 1;
                    i += 1;
                }
                b'\r' if bytes.get(i + 1) == Some(&b'\n') => {
                    survey.crlf += 1;
                    i += 2;
                }
                b'\r' => {
                    survey.lone_cr += 1;
                    // `\r\r\n`: this bare CR abuts the CRLF after it.
                    if bytes.get(i + 1) == Some(&b'\r') && bytes.get(i + 2) == Some(&b'\n') {
                        survey.cr_before_break += 1;
                    }
                    i += 1;
                }
                _ => i += 1,
            }
        }
        survey
    }

    /// Whether both conventions occur, so some break disagrees with whatever
    /// the document is declared to be.
    ///
    /// A lone `\r` never makes a document mixed, because it is not a break.
    pub const fn is_mixed(self) -> bool {
        self.lf > 0 && self.crlf > 0
    }

    /// Total number of line breaks, of either convention.
    pub const fn breaks(self) -> usize {
        self.lf + self.crlf
    }

    /// The convention the document is, by majority.
    ///
    /// Contractually identical to `bp_core::LineEnding::detect` -- a property
    /// test pins the two together, because two answers to "what is this file"
    /// in one program is a defect waiting to be found by a user. [`None`] for
    /// a document with no line break at all, which the caller resolves to
    /// `LineEnding::platform_default`.
    pub const fn majority(self) -> Option<LineEnding> {
        if self.crlf == 0 && self.lf == 0 {
            None
        } else if self.crlf >= self.lf {
            // Ties go to CRLF, matching `detect`.
            Some(LineEnding::CrLf)
        } else {
            Some(LineEnding::Lf)
        }
    }

    /// How many line breaks [`LineEndingPolicy::ConvertTo`] would rewrite.
    ///
    /// Zero means the conversion is a no-op and `encode` is an exact inverse
    /// of `load` under either policy. Anything above zero is the number of
    /// the user's line breaks that saving is about to change, which is the
    /// number a warning would want to quote.
    pub const fn breaks_rewritten_by(self, target: LineEnding) -> usize {
        match target {
            LineEnding::Lf => self.crlf,
            LineEnding::CrLf => self.lf,
        }
    }

    /// Whether [`LineEndingPolicy::ConvertTo`] will actually leave the
    /// document using only `target`.
    ///
    /// Almost always yes, and false in exactly one situation: converting to
    /// LF a document that contains `"\r\r\n"`. The bare carriage return is
    /// data, so it stays; the `\r\n` after it becomes `\n`; and the two are
    /// then adjacent, which *is* a CRLF. The crate will not delete a byte the
    /// user typed in order to make its own postcondition come true, so it
    /// reports the situation instead. Saving such a document as LF repeatedly
    /// consumes one carriage return each time, which is why a caller that
    /// converts should check this rather than assume.
    pub const fn converges_to(self, target: LineEnding) -> bool {
        match target {
            LineEnding::Lf => self.cr_before_break == 0,
            // Converting to CRLF cannot fail this way: every `\n` it writes
            // already has its own `\r`, so an extra one in front is still
            // just a bare CR followed by a CRLF.
            LineEnding::CrLf => true,
        }
    }
}

/// Turn document text into the bytes to write to disk, or say which
/// character the encoding cannot hold.
///
/// The exact inverse of [`load`](crate::load) under
/// [`LineEndingPolicy::Preserve`]: for any text this crate can hold and any
/// Unicode `encoding`, writing these bytes and loading them back returns the
/// same text and the same encoding. Under [`LineEndingPolicy::ConvertTo`] it
/// is an inverse only when [`LineEndingSurvey::breaks_rewritten_by`] is zero
/// -- deliberately, and that asymmetry is pinned by test rather than left to
/// be discovered.
///
/// Fails only for [`Encoding::Windows1252`], the one encoding that cannot
/// hold every `char`, and then with [`EncodeError::Unrepresentable`] naming
/// the first character it cannot hold. Nothing is ever substituted.
///
/// The byte-order mark is written whenever the encoding carries one, which is
/// what makes reopening the file report the encoding it was opened with. Two
/// consequences worth knowing, both properties of detecting an encoding from
/// bytes rather than of this function, and in both the bytes are exact and
/// only the model shifts:
///
/// - A document whose *first character* is U+FEFF and whose encoding is plain
///   [`Encoding::Utf8`] produces bytes indistinguishable from a UTF-8 BOM, so
///   reloading reports `Utf8Bom` and one character shorter.
/// - Windows-1252 has no mark, and `load` reads anything that is valid UTF-8
///   as UTF-8. So a 1252 document that is all ASCII reloads as the same text
///   labelled UTF-8 -- harmless, the bytes are the same in both -- and one
///   whose 1252 bytes happen to spell valid UTF-8 ("Ã©" is the bytes of "é")
///   reloads as that UTF-8. What does hold for every file: one that `load`
///   accepts, saved unchanged, is the bytes it was.
pub fn try_encode(
    text: &str,
    encoding: Encoding,
    line_endings: LineEndingPolicy,
) -> Result<Vec<u8>, EncodeError> {
    let body = with_line_endings(text, line_endings);
    encode_body(&body, encoding).map_err(|in_body| {
        // Report the position in the text the caller holds. Line breaks are
        // ASCII, which every encoding holds, so a converted copy fails on the
        // same character -- but at an offset shifted by whatever conversion
        // did before it. Recomputing costs a second pass on the error path
        // only; a save that succeeds never pays it.
        let in_text = match body {
            Cow::Borrowed(_) => None,
            Cow::Owned(_) => encode_body(text, encoding).err(),
        };
        EncodeError::Unrepresentable(in_text.unwrap_or(in_body))
    })
}

/// Turn document text into the bytes to write to disk.
///
/// [`try_encode`] for callers that cannot ask the user anything, and the API
/// the shell's save path was built on, kept so that adding Windows-1252 did
/// not change a signature under it. For every Unicode encoding the two are
/// identical, and so is everything [`try_encode`] says about inverses.
///
/// For a [`Encoding::Windows1252`] document holding a character the code
/// page cannot, this writes the document as plain UTF-8 instead. Of the
/// options for a function that must return bytes, that is the one that
/// loses nothing: every character survives, and reopening the file detects
/// UTF-8 and reports it. Writing `?` for the character -- what the Windows
/// API does -- loses it silently. A caller that can prompt should call
/// [`try_encode`] and ask before switching.
pub fn encode(text: &str, encoding: Encoding, line_endings: LineEndingPolicy) -> Vec<u8> {
    let body = with_line_endings(text, line_endings);
    match encode_body(&body, encoding) {
        Ok(bytes) => bytes,
        Err(_) => body.as_bytes().to_vec(),
    }
}

/// Apply the line-ending policy, borrowing when there is nothing to change.
fn with_line_endings(text: &str, line_endings: LineEndingPolicy) -> Cow<'_, str> {
    match line_endings {
        LineEndingPolicy::Preserve => Cow::Borrowed(text),
        LineEndingPolicy::ConvertTo(target) => match convert(text, target) {
            Some(converted) => Cow::Owned(converted),
            // Nothing to change: keep the borrow rather than copying the
            // whole document, which is the common case for every save.
            None => Cow::Borrowed(text),
        },
    }
}

/// The mark, then the text in `encoding`.
fn encode_body(body: &str, encoding: Encoding) -> Result<Vec<u8>, Unrepresentable> {
    let mut out = encoding.bom().to_vec();
    match (encoding, Endian::of(encoding)) {
        (_, Some(endian)) => out.extend_from_slice(&utf16::encode(body, endian)),
        (Encoding::Windows1252, None) => out.extend_from_slice(&encode_windows_1252(body)?),
        (_, None) => out.extend_from_slice(body.as_bytes()),
    }
    Ok(out)
}

/// Rewrite every line break to `target`, or [`None`] if there is nothing to
/// rewrite.
///
/// A single left-to-right pass rather than the `replace("\r\n", "\n")` then
/// `replace('\n', "\r\n")` pair the shell used. The two are *behaviourally*
/// identical -- `str::replace` is itself a single non-overlapping left-to-
/// right scan, so it handles `"\r\r\n"` exactly the way this does, and every
/// property below passes against either. The reason for one pass is cost: the
/// pair allocates a whole intermediate copy of the document on the way to
/// CRLF, on every save. The [`None`] return is the same economy -- a
/// conversion with nothing to convert should not copy the document at all.
fn convert(text: &str, target: LineEnding) -> Option<String> {
    let survey = LineEndingSurvey::of(text);
    if survey.breaks_rewritten_by(target) == 0 {
        return None;
    }

    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len() + survey.breaks());
    let mut copied = 0;
    let mut i = 0;
    while i < bytes.len() {
        let width = match bytes[i] {
            b'\n' => 1,
            b'\r' if bytes.get(i + 1) == Some(&b'\n') => 2,
            _ => {
                i += 1;
                continue;
            }
        };
        out.push_str(&text[copied..i]);
        out.push_str(target.as_str());
        i += width;
        copied = i;
    }
    out.push_str(&text[copied..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SaveOptions, atomic_write, load};
    use proptest::prelude::*;
    use tempfile::TempDir;

    /// The encodings that can hold any text. Windows-1252 is tested on its
    /// own below, because it cannot, and because it has no mark to say it
    /// was chosen.
    const UNICODE_ENCODINGS: [Encoding; 4] = [
        Encoding::Utf8,
        Encoding::Utf8Bom,
        Encoding::Utf16Le,
        Encoding::Utf16Be,
    ];

    /// Write bytes into a fresh temporary directory and load them back.
    ///
    /// Never the checkout: every test here writes a file, and a test that
    /// writes into the working tree has already cost this project documents.
    fn round_trip(bytes: &[u8]) -> (TempDir, crate::LoadedFile) {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("document.txt");
        atomic_write(&path, bytes, SaveOptions::default()).expect("atomic_write");
        let loaded = load(&path).expect("load");
        (dir, loaded)
    }

    // --- generators ------------------------------------------------------

    /// Text built from the tokens that break round trips: both line breaks, a
    /// bare carriage return, two- and three-byte characters, an astral one,
    /// and a tab. The alphabet is visible here, so a failure report leaks
    /// nothing about any real document.
    fn document() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            prop_oneof![
                Just("a"),
                Just("word "),
                Just("\n"),
                Just("\r\n"),
                Just("\r"),
                Just("é"),
                Just("中"),
                Just("\t"),
                Just("😀"),
            ],
            0..30,
        )
        .prop_map(|parts| parts.concat())
    }

    /// The Unicode encodings; see [`UNICODE_ENCODINGS`].
    fn encoding() -> impl Strategy<Value = Encoding> {
        prop_oneof![
            Just(Encoding::Utf8),
            Just(Encoding::Utf8Bom),
            Just(Encoding::Utf16Le),
            Just(Encoding::Utf16Be),
        ]
    }

    fn line_ending() -> impl Strategy<Value = LineEnding> {
        prop_oneof![Just(LineEnding::Lf), Just(LineEnding::CrLf)]
    }

    // --- the properties --------------------------------------------------

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        /// **The headline property: `encode` is the inverse of `load`.** For
        /// any text, in any encoding this crate supports, under the
        /// preserving policy, writing and reading back returns the same text,
        /// the same encoding, and the line ending the text actually has.
        ///
        /// This is the property that could not exist while the encoder lived
        /// in the shell, and the reason for moving it.
        #[test]
        fn writing_then_loading_returns_what_was_written(
            text in document(),
            encoding in encoding(),
        ) {
            // The one documented exception: leading U+FEFF in plain UTF-8 is
            // a BOM once it is bytes. The generator cannot produce it, but
            // saying so here is cheaper than a reader wondering.
            prop_assume!(!text.starts_with('\u{FEFF}'));

            let bytes = encode(&text, encoding, LineEndingPolicy::Preserve);
            let (_dir, loaded) = round_trip(&bytes);

            prop_assert_eq!(&loaded.text, &text);
            prop_assert_eq!(loaded.encoding, encoding);
            prop_assert_eq!(
                loaded.line_ending,
                LineEnding::detect(&text).unwrap_or_default()
            );
            prop_assert_eq!(loaded.bytes_on_disk, bytes.len() as u64);
            prop_assert_eq!(
                try_encode(&text, encoding, LineEndingPolicy::Preserve),
                Ok(bytes),
                "for a Unicode encoding the two entry points must never differ"
            );
        }

        /// **Any file that loads saves back as the bytes it was.** The
        /// guarantee the Windows-1252 fallback rests on, stated over
        /// arbitrary bytes rather than text: whatever `load` accepted, in
        /// whatever encoding it settled on, `try_encode` under that encoding
        /// reproduces it exactly. Random bytes are mostly not UTF-8, so most
        /// cases that load are exercising the fallback.
        #[test]
        fn every_file_that_loads_saves_back_to_the_bytes_it_was(
            bytes in proptest::collection::vec(any::<u8>(), 0..48),
        ) {
            let dir = tempfile::tempdir().expect("temp dir");
            let path = dir.path().join("arbitrary.bin");
            atomic_write(&path, &bytes, SaveOptions::default()).expect("atomic_write");

            if let Ok(loaded) = load(&path) {
                prop_assert_eq!(
                    try_encode(&loaded.text, loaded.encoding, LineEndingPolicy::Preserve),
                    Ok(bytes)
                );
            }
        }

        /// **The survey agrees with `bp_core`.** Two answers to "what line
        /// ending is this file" in one program is a defect waiting for a
        /// user to find it.
        #[test]
        fn majority_agrees_with_line_ending_detect(text in document()) {
            prop_assert_eq!(
                LineEndingSurvey::of(&text).majority(),
                LineEnding::detect(&text)
            );
        }

        /// **Converting converges, and changes nothing else.** After a
        /// conversion every break is the target, and the number of breaks and
        /// of bare carriage returns is exactly what it was: a conversion may
        /// rewrite a break, never add, drop or merge one.
        #[test]
        fn converting_converges_without_losing_breaks(
            text in document(),
            target in line_ending(),
        ) {
            let before = LineEndingSurvey::of(&text);
            let bytes = encode(&text, Encoding::Utf8, LineEndingPolicy::ConvertTo(target));
            let (_dir, loaded) = round_trip(&bytes);
            let after = LineEndingSurvey::of(&loaded.text);

            // Nothing but line-break characters may move: strip every `\r`
            // and `\n` from both and what is left must be identical, so a
            // conversion can never eat a letter, a tab or an emoji.
            let strip = |s: &str| s.replace(['\r', '\n'], "");
            prop_assert_eq!(strip(&loaded.text), strip(&text), "a non-break character moved");
            prop_assert_eq!(after.breaks(), before.breaks(), "a break was added or lost");

            // Convergence is claimed only where `converges_to` claims it, and
            // *wherever* it claims it. Checking both halves is what stops the
            // predicate from becoming a rubber stamp that always says no.
            prop_assert_eq!(
                after.breaks_rewritten_by(target) == 0,
                before.converges_to(target),
                "converges_to mispredicted the outcome"
            );

            if before.converges_to(target) {
                prop_assert_eq!(after.lone_cr, before.lone_cr, "a bare CR was touched");
                if before.breaks() > 0 {
                    prop_assert_eq!(loaded.line_ending, target);
                }
            }
        }

        /// **The asymmetry, pinned as an equivalence.** Converting is an
        /// inverse of `load` *exactly* when there was nothing to convert. Not
        /// "usually", not "unless mixed" -- the survey predicts it, both
        /// directions, every time.
        ///
        /// This is the honest statement of the behaviour the shell had and
        /// could not describe: a save under `ConvertTo` rewrites the minority
        /// convention, and `breaks_rewritten_by` says in advance how many.
        #[test]
        fn converting_is_lossless_exactly_when_nothing_needs_converting(
            text in document(),
            target in line_ending(),
        ) {
            let predicted = LineEndingSurvey::of(&text).breaks_rewritten_by(target);
            let bytes = encode(&text, Encoding::Utf8, LineEndingPolicy::ConvertTo(target));
            let (_dir, loaded) = round_trip(&bytes);

            prop_assert_eq!(
                loaded.text == text,
                predicted == 0,
                "survey predicted {} rewrites; text {} unchanged",
                predicted,
                if loaded.text == text { "was" } else { "was not" }
            );
        }

        /// **Converting is idempotent.** Saving a converged document again
        /// must be a no-op, or a document would drift a little further from
        /// what the user wrote on every save -- the failure mode nobody
        /// notices until a diff is unreadable.
        #[test]
        fn converting_twice_is_converting_once(
            text in document(),
            target in line_ending(),
        ) {
            // Only where the conversion converges. Where it does not -- a
            // document holding "\r\r\n", converted to LF -- a second pass
            // does change it again, and that drift is stated on
            // `converges_to` and pinned by example, not hidden by a weaker
            // property here.
            prop_assume!(LineEndingSurvey::of(&text).converges_to(target));

            let policy = LineEndingPolicy::ConvertTo(target);
            let once = encode(&text, Encoding::Utf8, policy);
            let text_once = String::from_utf8(once.clone()).expect("utf8");
            let twice = encode(&text_once, Encoding::Utf8, policy);
            prop_assert_eq!(twice, once);
        }

        /// **The survey partitions the text.** Every `\r` and `\n` in the
        /// document is accounted for by exactly one counter, so no break can
        /// be invisible to a caller deciding whether to warn.
        #[test]
        fn the_survey_accounts_for_every_carriage_return_and_line_feed(text in document()) {
            let s = LineEndingSurvey::of(&text);
            prop_assert_eq!(
                s.lf + s.crlf,
                text.matches('\n').count(),
                "line feeds unaccounted for"
            );
            prop_assert_eq!(
                s.crlf + s.lone_cr,
                text.matches('\r').count(),
                "carriage returns unaccounted for"
            );
        }
    }

    // --- examples, boundaries and failure modes --------------------------

    #[test]
    fn the_empty_document_is_just_the_byte_order_mark() {
        for encoding in UNICODE_ENCODINGS {
            let bytes = encode("", encoding, LineEndingPolicy::Preserve);
            assert_eq!(bytes, encoding.bom(), "{}", encoding.label());

            let (_dir, loaded) = round_trip(&bytes);
            assert_eq!(loaded.text, "");
            assert_eq!(loaded.encoding, encoding);
            assert_eq!(loaded.line_ending, LineEnding::platform_default());
        }
    }

    #[test]
    fn utf16_writes_the_byte_order_mark_it_promises() {
        assert_eq!(
            encode("hi", Encoding::Utf16Le, LineEndingPolicy::Preserve),
            b"\xFF\xFEh\0i\0"
        );
        assert_eq!(
            encode("hi", Encoding::Utf16Be, LineEndingPolicy::Preserve),
            b"\xFE\xFF\0h\0i"
        );
    }

    #[test]
    fn a_utf8_bom_is_written_back_and_a_plain_utf8_file_gains_nothing() {
        assert_eq!(
            encode("hi", Encoding::Utf8Bom, LineEndingPolicy::Preserve),
            b"\xEF\xBB\xBFhi"
        );
        assert_eq!(
            encode("hi", Encoding::Utf8, LineEndingPolicy::Preserve),
            b"hi"
        );
    }

    #[test]
    fn preserving_leaves_a_mixed_document_mixed() {
        // Three LF, two CRLF: the majority answer must not become an
        // instruction to rewrite the minority.
        let text = "crlf\r\nlf one\nlf two\nlf three\ncrlf\r\n";
        let bytes = encode(text, Encoding::Utf8, LineEndingPolicy::Preserve);

        assert_eq!(bytes, text.as_bytes());
        let survey = LineEndingSurvey::of(text);
        assert!(survey.is_mixed());
        assert_eq!(survey.majority(), Some(LineEnding::Lf));
        assert_eq!(survey.breaks_rewritten_by(LineEnding::Lf), 2);
        assert_eq!(survey.breaks_rewritten_by(LineEnding::CrLf), 3);
    }

    #[test]
    fn converting_a_mixed_document_rewrites_exactly_the_predicted_breaks() {
        let text = "crlf\r\nlf one\nlf two\nlf three\ncrlf\r\n";
        let converted = encode(
            text,
            Encoding::Utf8,
            LineEndingPolicy::ConvertTo(LineEnding::Lf),
        );
        assert_eq!(
            String::from_utf8(converted).unwrap(),
            "crlf\nlf one\nlf two\nlf three\ncrlf\n",
            "the two CRLF the survey named are the two that changed"
        );
    }

    #[test]
    fn a_bare_carriage_return_is_data_not_a_line_break() {
        let text = "a\rb\nc";
        for target in [LineEnding::Lf, LineEnding::CrLf] {
            let out = encode(text, Encoding::Utf8, LineEndingPolicy::ConvertTo(target));
            let out = String::from_utf8(out).unwrap();
            assert_eq!(out, format!("a\rb{}c", target.as_str()));
        }
        let survey = LineEndingSurvey::of(text);
        assert_eq!(survey.lone_cr, 1);
        assert_eq!(survey.cr_before_break, 0, "this CR is nowhere near a break");
        assert!(!LineEndingSurvey::of("a\rb").is_mixed());
    }

    #[test]
    fn a_carriage_return_abutting_a_break_is_the_one_case_that_cannot_converge() {
        // "\r\r\n" is a bare CR -- data -- followed by a CRLF. Converting to
        // LF rewrites the break and leaves the CR, and the two are then
        // adjacent, which is a CRLF again. The only alternative is deleting a
        // byte the user typed, which this crate will not do; so it reports
        // the situation instead of pretending.
        let survey = LineEndingSurvey::of("a\r\r\nb");
        assert_eq!(survey.crlf, 1);
        assert_eq!(survey.lone_cr, 1);
        assert_eq!(survey.cr_before_break, 1);
        assert!(!survey.converges_to(LineEnding::Lf));
        assert!(survey.converges_to(LineEnding::CrLf));

        let out = encode(
            "a\r\r\nb",
            Encoding::Utf8,
            LineEndingPolicy::ConvertTo(LineEnding::Lf),
        );
        assert_eq!(String::from_utf8(out).unwrap(), "a\r\nb");

        // And the drift a caller heeding `converges_to` avoids: converting
        // again eats the carriage return the first pass left behind.
        let again = encode(
            "a\r\nb",
            Encoding::Utf8,
            LineEndingPolicy::ConvertTo(LineEnding::Lf),
        );
        assert_eq!(String::from_utf8(again).unwrap(), "a\nb");

        // Converting the same document to CRLF is a clean no-op, which is why
        // `converges_to` answers per target rather than per document.
        let out = encode(
            "a\r\r\nb",
            Encoding::Utf8,
            LineEndingPolicy::ConvertTo(LineEnding::CrLf),
        );
        assert_eq!(String::from_utf8(out).unwrap(), "a\r\r\nb");
    }

    #[test]
    fn an_ordinary_document_converges_to_either_convention() {
        for text in ["a\nb\r\nc", "a\nb", "a\r\nb", "no breaks", "", "a\rb"] {
            let survey = LineEndingSurvey::of(text);
            assert!(survey.converges_to(LineEnding::Lf), "{text:?}");
            assert!(survey.converges_to(LineEnding::CrLf), "{text:?}");
        }
    }

    #[test]
    fn an_empty_survey_has_no_majority_and_nothing_to_rewrite() {
        let s = LineEndingSurvey::of("");
        assert_eq!(s, LineEndingSurvey::default());
        assert_eq!(s.majority(), None);
        assert!(!s.is_mixed());
        assert_eq!(s.breaks_rewritten_by(LineEnding::Lf), 0);
        assert_eq!(s.breaks_rewritten_by(LineEnding::CrLf), 0);
    }

    #[test]
    fn a_tie_goes_to_crlf_just_as_detect_does() {
        let text = "a\r\nb\nc";
        assert_eq!(
            LineEndingSurvey::of(text).majority(),
            Some(LineEnding::CrLf)
        );
        assert_eq!(LineEnding::detect(text), Some(LineEnding::CrLf));
    }

    #[test]
    fn astral_characters_survive_every_encoding() {
        // The character a naive UTF-16 cast truncates, and the one a
        // surrogate-pair bug splits.
        let text = "😀 é 中\n";
        for encoding in UNICODE_ENCODINGS {
            let bytes = encode(text, encoding, LineEndingPolicy::Preserve);
            let (_dir, loaded) = round_trip(&bytes);
            assert_eq!(loaded.text, text, "{}", encoding.label());
            assert_eq!(loaded.encoding, encoding);
        }
    }

    #[test]
    fn a_leading_zero_width_no_break_space_in_plain_utf8_is_read_back_as_a_bom() {
        // The documented limitation, tested so it is a known behaviour rather
        // than a surprise. The bytes are exact; the model shifts.
        let text = "\u{FEFF}hi";
        let bytes = encode(text, Encoding::Utf8, LineEndingPolicy::Preserve);
        assert_eq!(bytes, "\u{FEFF}hi".as_bytes());

        let (_dir, loaded) = round_trip(&bytes);
        assert_eq!(loaded.encoding, Encoding::Utf8Bom);
        assert_eq!(loaded.text, "hi");

        // In UTF-16 it round-trips exactly, because the mark is not ambiguous
        // with the first character there.
        let bytes = encode(text, Encoding::Utf16Le, LineEndingPolicy::Preserve);
        let (_dir, loaded) = round_trip(&bytes);
        assert_eq!(loaded.encoding, Encoding::Utf16Le);
        assert_eq!(loaded.text, text);
    }

    // --- Windows-1252 ------------------------------------------------------

    /// The fixture M1-10 names: an accent, the euro sign, all four smart
    /// quotes, CRLF -- opened and saved unchanged.
    const NOTEPAD_ANSI: &[u8] = b"Caf\xE9 \x805\r\n\x93quoted\x94 and \x91single\x92\r\n";

    #[test]
    fn a_windows_1252_file_saves_back_to_exactly_the_bytes_it_was_opened_from() {
        let (_dir, loaded) = round_trip(NOTEPAD_ANSI);
        assert_eq!(loaded.encoding, Encoding::Windows1252);

        for policy in [
            LineEndingPolicy::Preserve,
            LineEndingPolicy::ConvertTo(loaded.line_ending),
        ] {
            assert_eq!(
                try_encode(&loaded.text, loaded.encoding, policy).as_deref(),
                Ok(NOTEPAD_ANSI),
                "{policy:?}"
            );
            assert_eq!(encode(&loaded.text, loaded.encoding, policy), NOTEPAD_ANSI);
        }
    }

    #[test]
    fn a_character_1252_cannot_hold_is_refused_with_its_position_not_replaced() {
        let err = try_encode(
            "naïve → x",
            Encoding::Windows1252,
            LineEndingPolicy::Preserve,
        )
        .unwrap_err();

        let EncodeError::Unrepresentable(what) = err;
        assert_eq!(what.character, '→');
        assert_eq!(what.byte_offset, "naïve ".len());
        assert_eq!(what.encoding, Encoding::Windows1252);
        assert!(err.to_string().contains("UTF-8"), "{err}");
    }

    #[test]
    fn the_refused_position_is_in_the_callers_text_not_the_converted_copy() {
        // Converting to CRLF inserts a byte before the arrow; the shell
        // needs the offset in the buffer it holds.
        let text = "a\nb\nc → d";
        let err = try_encode(
            text,
            Encoding::Windows1252,
            LineEndingPolicy::ConvertTo(LineEnding::CrLf),
        )
        .unwrap_err();

        let EncodeError::Unrepresentable(what) = err;
        assert_eq!(what.byte_offset, text.find('→').unwrap());
    }

    #[test]
    fn encode_without_a_prompt_writes_utf8_rather_than_lose_a_character() {
        let text = "€5 → £4\r\n";

        let bytes = encode(text, Encoding::Windows1252, LineEndingPolicy::Preserve);

        assert_eq!(bytes, text.as_bytes(), "no `?` where the arrow was");
        let (_dir, loaded) = round_trip(&bytes);
        assert_eq!(loaded.text, text);
        assert_eq!(
            loaded.encoding,
            Encoding::Utf8,
            "and reopening says what was actually written"
        );
    }

    #[test]
    fn a_windows_1252_document_whose_bytes_are_valid_utf8_reads_back_as_utf8() {
        // The documented limit of a code page with no mark (risk R13), pinned
        // so it is known rather than found. All-ASCII is the common, harmless
        // case: the same text, labelled UTF-8. "Ã©" is the rare harmful one:
        // its 1252 bytes, C3 A9, are "é" in UTF-8. In both the bytes on disk
        // are exactly what 1252 says they should be.
        let (_dir, ascii) = round_trip(&encode(
            "plain",
            Encoding::Windows1252,
            LineEndingPolicy::Preserve,
        ));
        assert_eq!(ascii.text, "plain");
        assert_eq!(ascii.encoding, Encoding::Utf8);

        let bytes = encode("Ã©", Encoding::Windows1252, LineEndingPolicy::Preserve);
        assert_eq!(bytes, b"\xC3\xA9");
        let (_dir, mojibake) = round_trip(&bytes);
        assert_eq!(mojibake.text, "é");
        assert_eq!(mojibake.encoding, Encoding::Utf8);
    }

    #[test]
    fn the_empty_windows_1252_document_is_no_bytes_at_all() {
        assert_eq!(
            try_encode("", Encoding::Windows1252, LineEndingPolicy::Preserve),
            Ok(Vec::new())
        );
    }

    #[test]
    fn the_default_policy_preserves() {
        assert_eq!(LineEndingPolicy::default(), LineEndingPolicy::Preserve);
    }
}
