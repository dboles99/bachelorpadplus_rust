//! Encoding and line-ending model, and the one legacy code page's table.
//!
//! Both are surfaced in the status bar (specs.md section 3) and both are
//! round-trip properties of a document: opening a CRLF file and saving it
//! must not silently convert it to LF.
//!
//! The Windows-1252 table lives here rather than in `bp-files` because it is
//! pure data about an [`Encoding`] variant, with no filesystem in it, and a
//! fuzz target (release plan M4-05) wants it without the I/O. This module
//! deliberately does not *detect* Windows-1252 -- no byte sequence proves a
//! file is in it, so choosing it is a policy (`bp_files::load` falls back to
//! it only when UTF-8 and UTF-16 have failed), not a property of the bytes.

/// Text encoding of a document as it exists on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Encoding {
    #[default]
    Utf8,
    /// UTF-8 with a leading byte-order mark. Tracked separately because the
    /// BOM must be written back if it was there when the file was opened.
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    /// Windows code page 1252, the "ANSI" of every Western-European Windows
    /// install and what Notepad wrote before it defaulted to UTF-8.
    ///
    /// Also what a Latin-1 (ISO-8859-1) file on Linux is read as: the two
    /// agree everywhere except 0x80-0x9F, where Latin-1 has C1 controls no
    /// text file contains and 1252 has the euro sign and smart quotes. So
    /// one variant, one label, on both operating systems.
    ///
    /// Unlike the others this cannot hold every `char`; see
    /// [`encode_windows_1252`].
    Windows1252,
}

impl Encoding {
    /// Detect from a leading byte-order mark. Absent a BOM this returns
    /// [`Encoding::Utf8`]; it does not attempt statistical detection.
    pub fn detect_bom(bytes: &[u8]) -> Self {
        if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            Self::Utf8Bom
        } else if bytes.starts_with(&[0xFF, 0xFE]) {
            Self::Utf16Le
        } else if bytes.starts_with(&[0xFE, 0xFF]) {
            Self::Utf16Be
        } else {
            Self::Utf8
        }
    }

    /// The BOM to write back for this encoding, if any.
    pub const fn bom(self) -> &'static [u8] {
        match self {
            Self::Utf8 => &[],
            Self::Utf8Bom => &[0xEF, 0xBB, 0xBF],
            Self::Utf16Le => &[0xFF, 0xFE],
            Self::Utf16Be => &[0xFE, 0xFF],
            // Code pages have no mark; that is why detecting one is a guess.
            Self::Windows1252 => &[],
        }
    }

    /// Status-bar label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf8Bom => "UTF-8 BOM",
            Self::Utf16Le => "UTF-16 LE",
            Self::Utf16Be => "UTF-16 BE",
            Self::Windows1252 => "Windows-1252",
        }
    }

    /// Whether every `char` has a representation in this encoding.
    ///
    /// False only for [`Encoding::Windows1252`]. A caller that is about to
    /// write a document into an encoding for which this is false must be
    /// prepared for [`Unrepresentable`].
    pub const fn is_unicode(self) -> bool {
        !matches!(self, Self::Windows1252)
    }
}

/// What bytes 0x80-0x9F mean in Windows-1252, indexed from 0x80.
///
/// The WHATWG Encoding Standard's `index-windows-1252`, which is Microsoft's
/// `bestfit1252` table with its five holes filled. 0x81, 0x8D, 0x8F, 0x90 and
/// 0x9D are unassigned by Microsoft; WHATWG maps each to the C1 control with
/// the same number (U+0081 and so on), and so does this. The alternative --
/// refusing those bytes, or decoding them to U+FFFD -- would make a file that
/// is otherwise plain 1252 unopenable over one stray byte, or open it and
/// write the damage back on save. With the holes filled the table is a
/// bijection between the 256 bytes and 256 chars, so decoding is total and
/// every decoded document encodes back to exactly the bytes it came from.
///
/// Every other byte is itself: 0x00-0x7F is ASCII and 0xA0-0xFF is Latin-1,
/// which Unicode's first 256 code points copy.
const WINDOWS_1252_HIGH: [char; 32] = [
    '\u{20AC}', // 0x80 euro sign
    '\u{0081}', // 0x81 unassigned: C1 control, per WHATWG
    '\u{201A}', // 0x82 single low-9 quotation mark
    '\u{0192}', // 0x83 f with hook
    '\u{201E}', // 0x84 double low-9 quotation mark
    '\u{2026}', // 0x85 horizontal ellipsis
    '\u{2020}', // 0x86 dagger
    '\u{2021}', // 0x87 double dagger
    '\u{02C6}', // 0x88 modifier circumflex
    '\u{2030}', // 0x89 per mille sign
    '\u{0160}', // 0x8A S with caron
    '\u{2039}', // 0x8B single left-pointing angle quotation mark
    '\u{0152}', // 0x8C OE ligature
    '\u{008D}', // 0x8D unassigned: C1 control, per WHATWG
    '\u{017D}', // 0x8E Z with caron
    '\u{008F}', // 0x8F unassigned: C1 control, per WHATWG
    '\u{0090}', // 0x90 unassigned: C1 control, per WHATWG
    '\u{2018}', // 0x91 left single quotation mark
    '\u{2019}', // 0x92 right single quotation mark
    '\u{201C}', // 0x93 left double quotation mark
    '\u{201D}', // 0x94 right double quotation mark
    '\u{2022}', // 0x95 bullet
    '\u{2013}', // 0x96 en dash
    '\u{2014}', // 0x97 em dash
    '\u{02DC}', // 0x98 small tilde
    '\u{2122}', // 0x99 trade mark sign
    '\u{0161}', // 0x9A s with caron
    '\u{203A}', // 0x9B single right-pointing angle quotation mark
    '\u{0153}', // 0x9C oe ligature
    '\u{009D}', // 0x9D unassigned: C1 control, per WHATWG
    '\u{017E}', // 0x9E z with caron
    '\u{0178}', // 0x9F Y with diaeresis
];

/// The character Windows-1252 byte `byte` stands for.
const fn windows_1252_char(byte: u8) -> char {
    match byte {
        0x80..=0x9F => WINDOWS_1252_HIGH[(byte - 0x80) as usize],
        // ASCII and Latin-1 are the first 256 code points, unchanged.
        _ => byte as char,
    }
}

/// The Windows-1252 byte for `c`, or [`None`] if the code page has none.
fn windows_1252_byte(c: char) -> Option<u8> {
    match u32::from(c) {
        // Checked before the table because it is nearly every character in
        // a Western text, and the table search is the slow path.
        code @ (0x00..=0x7F | 0xA0..=0xFF) => u8::try_from(code).ok(),
        _ => WINDOWS_1252_HIGH
            .iter()
            .position(|&high| high == c)
            .and_then(|i| u8::try_from(i).ok())
            .map(|i| 0x80 | i),
    }
}

/// Decode Windows-1252 bytes into text.
///
/// Total: every byte sequence decodes, and nothing is ever substituted --
/// see the table's note on the five unassigned bytes. That is what makes
/// the fallback in `bp_files::load` safe to offer: a 1252 file opened and
/// saved unchanged is byte-for-byte what it was.
///
/// The flip side of total is that it is never *wrong* in a way this function
/// can see: a UTF-8 file decoded here is readable mojibake ("Ã©" for "é"),
/// not an error. Deciding that a file is 1252 is the caller's job, and the
/// caller must try UTF-8 first.
pub fn decode_windows_1252(bytes: &[u8]) -> String {
    // Every byte is one char of at most three UTF-8 bytes; ASCII-heavy text
    // is the norm, so reserve for that and let the rare growth reallocate.
    let mut text = String::with_capacity(bytes.len());
    text.extend(bytes.iter().map(|&b| windows_1252_char(b)));
    text
}

/// Encode text as Windows-1252, or say which character cannot be.
///
/// Refuses rather than writing `?` for the character, which is what the
/// Windows API does by default and how a user saving a document with one
/// arrow in it loses the arrow without being told. The error names the
/// first offending character and where it is, so the shell can offer to
/// switch the document to UTF-8 instead.
///
/// The exact inverse of [`decode_windows_1252`] for every byte sequence.
pub fn encode_windows_1252(text: &str) -> Result<Vec<u8>, Unrepresentable> {
    let mut bytes = Vec::with_capacity(text.len());
    for (byte_offset, character) in text.char_indices() {
        match windows_1252_byte(character) {
            Some(byte) => bytes.push(byte),
            None => {
                return Err(Unrepresentable {
                    encoding: Encoding::Windows1252,
                    character,
                    byte_offset,
                });
            }
        }
    }
    Ok(bytes)
}

/// A character that the target encoding has no bytes for.
///
/// Carries enough for a prompt that a user can act on -- which character,
/// and where -- rather than "cannot encode".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unrepresentable {
    /// The encoding that was asked for.
    pub encoding: Encoding,
    /// The first character in the text that it cannot hold.
    pub character: char,
    /// Where that character starts, as a byte offset into the UTF-8 text --
    /// the unit the editor's cursor positions are in, so the shell can put
    /// the caret on it.
    pub byte_offset: usize,
}

impl std::fmt::Display for Unrepresentable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\"{}\" (U+{:04X}) at byte {} cannot be written as {}",
            self.character,
            u32::from(self.character),
            self.byte_offset,
            self.encoding.label()
        )
    }
}

impl std::error::Error for Unrepresentable {}

/// Line-ending convention of a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
}

impl LineEnding {
    /// The convention a new document gets on this platform.
    pub const fn platform_default() -> Self {
        if cfg!(windows) { Self::CrLf } else { Self::Lf }
    }

    /// Detect from content by majority.
    ///
    /// A file with no line break at all yields [`None`], which the caller
    /// should resolve to [`Self::platform_default`]. Reporting a guess for an
    /// empty file would put a wrong value in the status bar.
    pub fn detect(text: &str) -> Option<Self> {
        let crlf = text.matches("\r\n").count();
        // Every CRLF also contains an LF, so subtract to get bare LFs.
        let lf = text.matches('\n').count() - crlf;
        match (crlf, lf) {
            (0, 0) => None,
            _ if crlf >= lf => Some(Self::CrLf),
            _ => Some(Self::Lf),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }

    /// Status-bar label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::CrLf => "CRLF",
        }
    }
}

impl Default for LineEnding {
    fn default() -> Self {
        Self::platform_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_utf8_bom() {
        assert_eq!(
            Encoding::detect_bom(&[0xEF, 0xBB, 0xBF, b'h', b'i']),
            Encoding::Utf8Bom
        );
        assert_eq!(Encoding::detect_bom(b"hi"), Encoding::Utf8);
        // A truncated BOM prefix must not be mistaken for one.
        assert_eq!(Encoding::detect_bom(&[0xEF, 0xBB]), Encoding::Utf8);
    }

    #[test]
    fn empty_and_unbroken_text_has_no_detectable_line_ending() {
        assert_eq!(LineEnding::detect(""), None);
        assert_eq!(LineEnding::detect("no breaks here"), None);
    }

    #[test]
    fn detects_by_majority() {
        assert_eq!(LineEnding::detect("a\r\nb\r\nc"), Some(LineEnding::CrLf));
        assert_eq!(LineEnding::detect("a\nb\nc"), Some(LineEnding::Lf));
        // Mixed, LF-dominant.
        assert_eq!(LineEnding::detect("a\r\nb\nc\nd"), Some(LineEnding::Lf));
        // Mixed, CRLF-dominant.
        assert_eq!(LineEnding::detect("a\r\nb\r\nc\nd"), Some(LineEnding::CrLf));
    }

    #[test]
    fn windows_1252_decodes_the_characters_notepad_users_actually_have() {
        // "café – “€5” ‘ok’" as Notepad's ANSI would have saved it.
        let bytes = b"caf\xE9 \x96 \x93\x805\x94 \x91ok\x92";
        assert_eq!(decode_windows_1252(bytes), "café – “€5” ‘ok’");
    }

    #[test]
    fn every_byte_decodes_and_encodes_back_to_itself() {
        let all: Vec<u8> = (0..=u8::MAX).collect();

        let text = decode_windows_1252(&all);

        assert_eq!(text.chars().count(), 256, "one char per byte, none dropped");
        assert!(
            !text.contains('\u{FFFD}'),
            "decoding must never substitute, or saving writes the damage back"
        );
        assert_eq!(
            encode_windows_1252(&text).as_deref(),
            Ok(&all[..]),
            "the table must be a bijection, or an opened file saved unchanged \
             is not the file that was opened"
        );
    }

    #[test]
    fn the_five_unassigned_bytes_are_the_c1_controls_whatwg_gives_them() {
        for byte in [0x81_u8, 0x8D, 0x8F, 0x90, 0x9D] {
            let text = decode_windows_1252(&[byte]);
            assert_eq!(text, char::from(byte).to_string(), "byte {byte:#04X}");
            assert_eq!(encode_windows_1252(&text), Ok(vec![byte]));
        }
    }

    #[test]
    fn a_c1_control_that_1252_reassigned_cannot_be_encoded() {
        // U+0080 is a C1 control in Latin-1, but byte 0x80 is the euro sign
        // in 1252, so the control has nowhere to go.
        assert_eq!(
            encode_windows_1252("\u{0080}"),
            Err(Unrepresentable {
                encoding: Encoding::Windows1252,
                character: '\u{0080}',
                byte_offset: 0,
            })
        );
        assert_eq!(encode_windows_1252("€"), Ok(vec![0x80]));
    }

    #[test]
    fn encoding_refuses_the_first_character_1252_cannot_hold_and_says_where() {
        let err = encode_windows_1252("é → 中").unwrap_err();

        assert_eq!(err.character, '→');
        assert_eq!(err.byte_offset, "é ".len(), "the first, not the last");
        assert_eq!(err.encoding, Encoding::Windows1252);
        assert_eq!(
            err.to_string(),
            "\"→\" (U+2192) at byte 3 cannot be written as Windows-1252"
        );
    }

    #[test]
    fn windows_1252_is_labelled_the_same_on_every_os_and_has_no_mark() {
        assert_eq!(Encoding::Windows1252.label(), "Windows-1252");
        assert_eq!(Encoding::Windows1252.bom(), b"");
        assert!(!Encoding::Windows1252.is_unicode());
        for unicode in [
            Encoding::Utf8,
            Encoding::Utf8Bom,
            Encoding::Utf16Le,
            Encoding::Utf16Be,
        ] {
            assert!(unicode.is_unicode(), "{}", unicode.label());
        }
    }

    #[test]
    fn crlf_is_not_double_counted_as_lf() {
        // The subtraction in `detect` is the whole reason this passes.
        assert_eq!(LineEnding::detect("a\r\nb"), Some(LineEnding::CrLf));
    }
}
