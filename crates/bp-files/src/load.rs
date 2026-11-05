//! Reading a file into a document.
//!
//! This module owns turning bytes on disk into editable text, and refusing
//! -- before reading -- a file too large for the editable widget to hold. It
//! deliberately does not serve large files some other way: the read-only
//! viewer is `bp_buffer::LargeFile`, and choosing between the two is the
//! shell's job once this has said which side of the line a file is on.

use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use bp_buffer::{HUGE_FILE_BYTES, SizeClass};
use bp_core::{Encoding, LineEnding, decode_windows_1252};
use thiserror::Error;

use crate::utf16::{self, Endian, Utf16Error};

/// The largest file [`load`] will read into an editable document, in bytes.
///
/// 32 MiB is release-plan decision 8 (U3), a product decision rather than a
/// measurement, and it sits between the two measured thresholds in
/// `bp-buffer` (ADR-0027):
///
/// - up to [`bp_buffer::LARGE_FILE_BYTES`] (8 MiB) a file is
///   [`SizeClass::Normal`] and opens within the startup budget;
/// - from there to this limit it still loads, but is [`SizeClass::Large`]
///   and [`LoadedFile::size_class`] says so, so the shell can warn that
///   whole-document tools may be slow;
/// - past this limit it is refused with [`LoadError::TooLarge`]. The rope
///   could hold it -- that ceiling is [`HUGE_FILE_BYTES`], 192 MiB -- but the
///   editable widget takes the whole text as one string and lays it all out,
///   and that, not the rope, is what hangs on a file this size.
///
/// A limit passed to [`load_with_limit`] is clamped to [`HUGE_FILE_BYTES`],
/// because past that size nothing is ever read whole, whatever a setting
/// says.
///
/// The comparison is inclusive, as `SizeClass`'s is: a file of exactly this
/// size loads.
pub const EDITABLE_LIMIT_BYTES: u64 = 32 * 1024 * 1024;

/// Why a file could not be turned into a document.
///
/// Every variant names the file and says what is wrong with it rather than
/// what the code was doing. "os error 32" and "invalid utf-16" are not things
/// a user can act on; "byte 4097 is half a character" is.
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("cannot read {}: {source}", .path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Not text in any encoding this crate reads. Since the Windows-1252
    /// fallback, that means a UTF-8 byte-order mark followed by invalid
    /// UTF-8, or bytes that are not UTF-8 and contain a NUL -- a binary
    /// file. `encoding` is what was tried, which is why the message still
    /// reads "not valid UTF-8": 1252 was deliberately not attempted.
    #[error("{} is not valid {}", .path.display(), .encoding.label())]
    InvalidText { path: PathBuf, encoding: Encoding },

    /// The body of a UTF-16 file is an odd number of bytes, so its last code
    /// unit is cut in half. Refused rather than dropping the stray byte: a
    /// file this size is truncated or corrupt, and opening it would invite
    /// the user to save the truncation back over whatever survived.
    #[error(
        "{} is {} but its text is {bytes} bytes -- an odd length, so the file is truncated or is not really {}",
        .path.display(), .encoding.label(), .encoding.label()
    )]
    TruncatedUtf16 {
        path: PathBuf,
        encoding: Encoding,
        bytes: usize,
    },

    /// A surrogate code unit with no partner. There is no character to
    /// produce for it -- only U+FFFD, and substituting one would hand back a
    /// document that looks intact and writes the damage back on the next
    /// save.
    #[error(
        "{} is {} but has an unpaired surrogate at character {at}, so it is not valid text",
        .path.display(), .encoding.label()
    )]
    UnpairedSurrogate {
        path: PathBuf,
        encoding: Encoding,
        /// Index of the offending code unit within the text, after the
        /// byte-order mark.
        at: usize,
    },

    /// The file is past the editable limit, so it was not read whole.
    /// Refused rather than attempted: the attempt is what hangs the editor
    /// or runs it out of memory, taking every other open document with it.
    /// The shell opens such a file in its read-only viewer instead
    /// (`bp_buffer::LargeFile`), so this message is seen only where that
    /// cannot be done.
    #[error(
        "{} is {} -- too large to edit here (the limit is {})",
        .path.display(), Size(*.bytes), Size(*.limit)
    )]
    TooLarge {
        path: PathBuf,
        /// Size as best known. For a file that grew past the limit while it
        /// was being read, or a special file that reports no size, this is
        /// a lower bound: reading stopped one byte past the limit.
        bytes: u64,
        /// The limit it was measured against, after clamping to
        /// [`HUGE_FILE_BYTES`].
        limit: u64,
    },
}

/// A byte count as a person would say it. Private because the only
/// consumer is the [`LoadError::TooLarge`] message.
struct Size(u64);

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const UNITS: [(u64, &str); 3] = [(1 << 30, "GiB"), (1 << 20, "MiB"), (1 << 10, "KiB")];
        for (unit, name) in UNITS {
            if self.0 >= unit {
                // Integer tenths rather than a float: exact, and u128 so the
                // multiply cannot overflow on any u64.
                let tenths = u128::from(self.0) * 10 / u128::from(unit);
                return write!(f, "{}.{} {name}", tenths / 10, tenths % 10);
            }
        }
        write!(f, "{} bytes", self.0)
    }
}

/// A file read from disk, with the properties the status bar reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedFile {
    pub text: String,
    /// Encoding as found on disk. Saving must write it back the same way --
    /// opening a BOM'd file and saving it without the BOM is data loss as far
    /// as some downstream tools are concerned.
    ///
    /// [`Encoding::Windows1252`] is an inference rather than a finding: the
    /// bytes were not UTF-8, so they were read as the code page. The shell
    /// should show it, so a user with a file in some other code page can
    /// see why the accents are wrong.
    pub encoding: Encoding,
    pub line_ending: LineEnding,
    /// Size on disk, before BOM stripping or decoding.
    pub bytes_on_disk: u64,
}

impl LoadedFile {
    /// The file's size class, so the shell can warn about a
    /// [`SizeClass::Large`] file that loaded but will make whole-document
    /// tools slow.
    ///
    /// A method rather than a field, because it is a pure function of
    /// [`Self::bytes_on_disk`] and a second copy could only disagree with
    /// it. A loaded file is never [`SizeClass::Huge`]: the editable limit is
    /// clamped to that threshold.
    pub fn size_class(&self) -> SizeClass {
        SizeClass::of(self.bytes_on_disk)
    }
}

/// Read a text file, detecting its encoding and line endings.
///
/// UTF-16 in either byte order is decoded, not refused. Windows tools have
/// emitted UTF-16 with a byte-order mark for decades, so refusing meant a
/// Windows editor that could not open a large class of Windows files. What
/// the original refusal was protecting against -- a lossy decode the user
/// then saves back over their original -- is protected against instead by
/// [`LoadError::TruncatedUtf16`] and [`LoadError::UnpairedSurrogate`]:
/// nothing is ever substituted, so anything that loads is exactly what was on
/// disk and [`crate::encode`] writes it back byte for byte.
///
/// UTF-16 is recognised only by its byte-order mark. A UTF-16 file without
/// one is indistinguishable from binary without statistical guessing, and a
/// wrong guess is the mangling this crate refuses.
///
/// A file with no byte-order mark that is not valid UTF-8 is read as
/// [`Encoding::Windows1252`] -- the legacy "ANSI" `.txt` that is most of what
/// a Notepad user has, and on Linux the Latin-1 file, which reads the same
/// way. Refusing those meant a Notepad replacement that could not open
/// Notepad's files. The order is the safeguard (release-plan risk R13):
/// UTF-8 is tried first and any valid UTF-8 is taken as UTF-8, because the
/// odds of a real 1252 text with a non-ASCII byte also being valid UTF-8 are
/// negligible, while reading a UTF-8 file as 1252 would turn every "é" into
/// "Ã©". The fallback is not a guess that can lose data: the 1252 decoder is
/// total and substitutes nothing, so a file opened this way and saved
/// unchanged is byte-for-byte what it was, and [`LoadedFile::encoding`] says
/// what was assumed so the status bar can show it.
///
/// Two cases still refuse with [`LoadError::InvalidText`]. A file whose
/// UTF-8 byte-order mark is followed by invalid UTF-8 -- the mark is a
/// declaration, and this is damaged UTF-8, not a code page. And a file that
/// is not UTF-8 and contains a NUL byte: no text editor writes NUL into a
/// 1252 file, while executables, images, archives and BOM-less UTF-16 are
/// full of them, and "opening" a binary as a page of accented gibberish
/// invites the user to save it back.
///
/// A file larger than [`EDITABLE_LIMIT_BYTES`] is refused with
/// [`LoadError::TooLarge`] without being read; see [`load_with_limit`].
pub fn load(path: &Path) -> Result<LoadedFile, LoadError> {
    load_with_limit(path, EDITABLE_LIMIT_BYTES)
}

/// Decode bytes that have already been obtained by a caller.
///
/// Encrypted documents use this after authentication so their original
/// encoding is preserved instead of being forced through UTF-8.
pub fn decode_bytes(path: &Path, bytes: Vec<u8>) -> Result<LoadedFile, LoadError> {
    decode(path, bytes)
}

/// [`load`], against a limit other than [`EDITABLE_LIMIT_BYTES`].
///
/// For the shell to pass a limit from Settings, and for tests to exercise
/// the refusal without writing tens of megabytes. `limit` is clamped to
/// [`HUGE_FILE_BYTES`].
///
/// The size is checked twice. Once from metadata, before reading, so an
/// oversized file costs a `stat` rather than a read. And again while
/// reading, because metadata can be wrong: a log still being written grows
/// between the check and the read, and special files such as `/proc` entries
/// report a length of zero whatever they hold. The read stops one byte past
/// the limit, so either case is refused having read at most `limit + 1`
/// bytes, never the whole file.
pub fn load_with_limit(path: &Path, limit: u64) -> Result<LoadedFile, LoadError> {
    let limit = limit.min(HUGE_FILE_BYTES);
    let read_error = |source| LoadError::Read {
        path: path.to_owned(),
        source,
    };
    // Stat the handle rather than the path, so the size checked belongs to
    // the file read even if the path is replaced in between.
    let file = File::open(path).map_err(read_error)?;
    let stated = file.metadata().map_err(read_error)?.len();
    if stated > limit {
        return Err(LoadError::TooLarge {
            path: path.to_owned(),
            bytes: stated,
            limit,
        });
    }
    let bytes = read_capped(path, &file, stated, limit).map_err(|e| match e {
        LoadError::TooLarge { path, bytes, limit } => {
            // A growing file has usually grown further than the one byte
            // that proved it too large, so a fresh stat gives the message a
            // truer size. A special file still says zero, and the lower
            // bound stands.
            let now = file.metadata().map_or(0, |m| m.len());
            LoadError::TooLarge {
                path,
                bytes: bytes.max(now),
                limit,
            }
        }
        other => other,
    })?;
    decode(path, bytes)
}

/// Read at most `limit` bytes of `source`, refusing if there is more.
///
/// Separate from [`load_with_limit`] so a test can hand it a file that grows
/// after its size was checked, which the filesystem will not do on cue.
fn read_capped(
    path: &Path,
    source: impl Read,
    size_hint: u64,
    limit: u64,
) -> Result<Vec<u8>, LoadError> {
    // The hint is trusted only as far as the limit, so a lying stat cannot
    // make this allocate more than the read is allowed to fill.
    let capacity = usize::try_from(size_hint.min(limit)).unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    source
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| LoadError::Read {
            path: path.to_owned(),
            source,
        })?;
    let read = bytes.len() as u64;
    if read > limit {
        return Err(LoadError::TooLarge {
            path: path.to_owned(),
            bytes: read,
            limit,
        });
    }
    Ok(bytes)
}

/// Turn bytes already read into a document.
fn decode(path: &Path, bytes: Vec<u8>) -> Result<LoadedFile, LoadError> {
    let bytes_on_disk = bytes.len() as u64;
    let marked = Encoding::detect_bom(&bytes);
    // `Encoding::Utf8` has an empty mark, so this covers all four cases.
    let body = &bytes[marked.bom().len()..];

    let (text, encoding) = match Endian::of(marked) {
        Some(endian) => {
            let text = utf16::decode(body, endian).map_err(|e| match e {
                Utf16Error::OddByteCount { bytes } => LoadError::TruncatedUtf16 {
                    path: path.to_owned(),
                    encoding: marked,
                    bytes,
                },
                Utf16Error::UnpairedSurrogate { at } => LoadError::UnpairedSurrogate {
                    path: path.to_owned(),
                    encoding: marked,
                    at,
                },
            })?;
            (text, marked)
        }
        None => match std::str::from_utf8(body) {
            Ok(text) => (text.to_owned(), marked),
            // Only with no mark: a UTF-8 BOM in front of invalid UTF-8 is
            // damaged UTF-8, and a NUL is binary. See `load`.
            Err(_) if marked == Encoding::Utf8 && !body.contains(&0) => {
                (decode_windows_1252(body), Encoding::Windows1252)
            }
            Err(_) => {
                return Err(LoadError::InvalidText {
                    path: path.to_owned(),
                    encoding: marked,
                });
            }
        },
    };

    let line_ending = LineEnding::detect(&text).unwrap_or_default();

    Ok(LoadedFile {
        text,
        encoding,
        line_ending,
        bytes_on_disk,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SaveOptions, atomic_write};
    use bp_buffer::LARGE_FILE_BYTES;
    use std::io::Write;
    use tempfile::tempdir;

    fn write_bytes(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        atomic_write(&path, bytes, SaveOptions::default()).unwrap();
        path
    }

    #[test]
    fn reads_plain_utf8() {
        let dir = tempdir().unwrap();
        let path = write_bytes(dir.path(), "a.txt", b"hello\nworld\n");

        let f = load(&path).unwrap();

        assert_eq!(f.text, "hello\nworld\n");
        assert_eq!(f.encoding, Encoding::Utf8);
        assert_eq!(f.line_ending, LineEnding::Lf);
        assert_eq!(f.bytes_on_disk, 12);
    }

    #[test]
    fn strips_a_utf8_bom_but_remembers_it() {
        let dir = tempdir().unwrap();
        let path = write_bytes(dir.path(), "b.txt", b"\xEF\xBB\xBFhello");

        let f = load(&path).unwrap();

        assert_eq!(f.text, "hello", "the BOM must not appear in the buffer");
        assert_eq!(
            f.encoding,
            Encoding::Utf8Bom,
            "but it must be remembered, so saving writes it back"
        );
        assert_eq!(f.bytes_on_disk, 8);
    }

    #[test]
    fn detects_crlf() {
        let dir = tempdir().unwrap();
        let path = write_bytes(dir.path(), "c.txt", b"a\r\nb\r\n");

        assert_eq!(load(&path).unwrap().line_ending, LineEnding::CrLf);
    }

    #[test]
    fn an_empty_file_gets_the_platform_line_ending() {
        let dir = tempdir().unwrap();
        let path = write_bytes(dir.path(), "d.txt", b"");

        let f = load(&path).unwrap();

        assert_eq!(f.text, "");
        assert_eq!(f.line_ending, LineEnding::platform_default());
    }

    #[test]
    fn decodes_utf16_in_both_byte_orders() {
        let dir = tempdir().unwrap();

        let le = write_bytes(dir.path(), "e-le.txt", b"\xFF\xFEh\0i\0");
        let f = load(&le).unwrap();
        assert_eq!(f.text, "hi", "the BOM must not appear in the buffer");
        assert_eq!(f.encoding, Encoding::Utf16Le);
        assert_eq!(f.bytes_on_disk, 6);

        let be = write_bytes(dir.path(), "e-be.txt", b"\xFE\xFF\0h\0i");
        let f = load(&be).unwrap();
        assert_eq!(f.text, "hi");
        assert_eq!(f.encoding, Encoding::Utf16Be);
    }

    #[test]
    fn a_lone_utf16_byte_order_mark_is_an_empty_document() {
        let dir = tempdir().unwrap();
        let path = write_bytes(dir.path(), "e2.txt", b"\xFF\xFE");

        let f = load(&path).unwrap();

        assert_eq!(f.text, "");
        assert_eq!(
            f.encoding,
            Encoding::Utf16Le,
            "an empty UTF-16 file is still a UTF-16 file, and saving it must \
             write the mark back"
        );
        assert_eq!(f.line_ending, LineEnding::platform_default());
    }

    #[test]
    fn refuses_a_truncated_utf16_file_rather_than_dropping_the_stray_byte() {
        let dir = tempdir().unwrap();
        let path = write_bytes(dir.path(), "e3.txt", b"\xFF\xFEh\0i");

        let err = load(&path).unwrap_err();

        assert!(matches!(
            err,
            LoadError::TruncatedUtf16 {
                encoding: Encoding::Utf16Le,
                bytes: 3,
                ..
            }
        ));
        assert!(err.to_string().contains("e3.txt"));
    }

    #[test]
    fn refuses_an_unpaired_surrogate_rather_than_substituting_u_fffd() {
        let dir = tempdir().unwrap();
        // "a" then a lone high surrogate.
        let path = write_bytes(dir.path(), "e4.txt", b"\xFF\xFEa\0\x00\xD8");

        let err = load(&path).unwrap_err();

        assert!(matches!(
            err,
            LoadError::UnpairedSurrogate {
                encoding: Encoding::Utf16Le,
                at: 1,
                ..
            }
        ));
        assert!(err.to_string().contains("e4.txt"));
    }

    #[test]
    fn utf16_line_endings_are_detected_after_decoding() {
        let dir = tempdir().unwrap();
        // "a\r\nb" little-endian.
        let path = write_bytes(dir.path(), "e5.txt", b"\xFF\xFEa\0\r\0\n\0b\0");

        let f = load(&path).unwrap();

        assert_eq!(f.text, "a\r\nb");
        assert_eq!(f.line_ending, LineEnding::CrLf);
    }

    /// This test used to be `rejects_invalid_utf8`, asserting refusal. The
    /// refusal is what M1-10 removes: a file that is not UTF-8 and has no
    /// mark is now read as Windows-1252, because that is what legacy
    /// Windows text is and refusing it left Notepad's files unopenable.
    /// The decode is total, so nothing is lost by reading instead of
    /// refusing; what is still refused is below.
    #[test]
    fn invalid_utf8_with_no_mark_is_read_as_windows_1252_rather_than_refused() {
        let dir = tempdir().unwrap();
        let path = write_bytes(dir.path(), "f.txt", b"ok \xC3\x28 bad");

        let f = load(&path).unwrap();

        assert_eq!(f.encoding, Encoding::Windows1252);
        assert_eq!(f.text, "ok Ã( bad");
    }

    #[test]
    fn a_windows_1252_file_loads_with_its_encoding_and_the_right_text() {
        let dir = tempdir().unwrap();
        // What Notepad's "ANSI" wrote: an accent, the euro sign (0x80) and
        // all four smart quotes (0x91-0x94), with Windows line endings.
        let path = write_bytes(
            dir.path(),
            "notepad.txt",
            b"Caf\xE9 \x805\r\n\x93quoted\x94 and \x91single\x92\r\n",
        );

        let f = load(&path).unwrap();

        assert_eq!(f.text, "Café €5\r\n“quoted” and ‘single’\r\n");
        assert_eq!(f.encoding, Encoding::Windows1252);
        assert_eq!(f.encoding.label(), "Windows-1252");
        assert_eq!(f.line_ending, LineEnding::CrLf);
        assert_eq!(
            f.bytes_on_disk, 32,
            "measured before decoding, one byte a character"
        );
    }

    #[test]
    fn a_utf8_file_with_an_accent_is_read_as_utf8_never_as_windows_1252() {
        let dir = tempdir().unwrap();
        let path = write_bytes(dir.path(), "utf8.txt", "Café “€5”\n".as_bytes());

        let f = load(&path).unwrap();

        assert_eq!(
            f.encoding,
            Encoding::Utf8,
            "UTF-8 is tried first; read as 1252 this would be \"CafÃ©\""
        );
        assert_eq!(f.text, "Café “€5”\n");
    }

    #[test]
    fn a_binary_file_is_still_refused_rather_than_read_as_windows_1252() {
        let dir = tempdir().unwrap();
        // A PNG's signature and first chunk header, and "hé" as UTF-16 LE
        // with no mark: neither is UTF-8, and both have NUL bytes, which is
        // what gives them away.
        let png = write_bytes(dir.path(), "image.png", b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR");
        let utf16 = write_bytes(dir.path(), "unmarked.txt", b"h\0\xE9\0");

        for path in [png, utf16] {
            let err = load(&path).unwrap_err();
            assert!(
                matches!(
                    err,
                    LoadError::InvalidText {
                        encoding: Encoding::Utf8,
                        ..
                    }
                ),
                "{} must be refused, got {err:?}",
                path.display()
            );
        }
    }

    #[test]
    fn invalid_utf8_after_a_utf8_mark_is_refused_rather_than_read_as_windows_1252() {
        let dir = tempdir().unwrap();
        let path = write_bytes(dir.path(), "damaged.txt", b"\xEF\xBB\xBFok \xC3\x28");

        assert!(
            matches!(
                load(&path).unwrap_err(),
                LoadError::InvalidText {
                    encoding: Encoding::Utf8Bom,
                    ..
                }
            ),
            "the mark declares UTF-8, so this is damaged UTF-8, not a code page"
        );
    }

    #[test]
    fn reports_a_missing_file_clearly() {
        let dir = tempdir().unwrap();
        let err = load(&dir.path().join("nope.txt")).unwrap_err();

        assert!(matches!(err, LoadError::Read { .. }));
        assert!(err.to_string().contains("nope.txt"));
    }

    #[test]
    fn round_trips_through_save() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("g.txt");
        let original = "line one\nline two\n";

        atomic_write(&path, original.as_bytes(), SaveOptions::default()).unwrap();

        assert_eq!(load(&path).unwrap().text, original);
    }

    /// A file of `len` bytes that costs no disk: `set_len` extends with a
    /// hole, which reads back as NUL bytes -- valid UTF-8, so a sparse file
    /// that is allowed through also loads.
    fn sparse(dir: &Path, name: &str, len: u64) -> PathBuf {
        let path = dir.join(name);
        File::create(&path).unwrap().set_len(len).unwrap();
        path
    }

    #[test]
    fn a_file_past_the_limit_is_refused_and_one_at_the_limit_loads() {
        let dir = tempdir().unwrap();
        let limit = 64;
        let at = sparse(dir.path(), "at.txt", limit);
        let over = sparse(dir.path(), "over.txt", limit + 1);

        let f = load_with_limit(&at, limit).unwrap();
        assert_eq!(
            f.bytes_on_disk, limit,
            "the limit is inclusive, as SizeClass's thresholds are"
        );

        let err = load_with_limit(&over, limit).unwrap_err();
        assert!(matches!(
            err,
            LoadError::TooLarge {
                bytes: 65,
                limit: 64,
                ..
            }
        ));
    }

    #[test]
    fn the_default_limit_refuses_a_sparse_file_just_past_it_without_reading_it() {
        let dir = tempdir().unwrap();
        let path = sparse(dir.path(), "big.log", EDITABLE_LIMIT_BYTES + 1);

        let err = load(&path).unwrap_err();

        assert!(matches!(
            err,
            LoadError::TooLarge { bytes, limit, .. }
                if bytes == EDITABLE_LIMIT_BYTES + 1 && limit == EDITABLE_LIMIT_BYTES
        ));
    }

    #[test]
    fn a_multi_gigabyte_file_is_refused_by_its_size_alone() {
        let dir = tempdir().unwrap();
        let path = sparse(dir.path(), "huge.log", 4 << 30);

        let err = load(&path).unwrap_err();

        assert!(matches!(err, LoadError::TooLarge { bytes, .. } if bytes == 4 << 30));
    }

    #[test]
    fn a_configured_limit_cannot_raise_the_ceiling_past_the_huge_threshold() {
        let dir = tempdir().unwrap();
        let path = sparse(dir.path(), "huge.log", HUGE_FILE_BYTES + 1);

        let err = load_with_limit(&path, u64::MAX).unwrap_err();

        assert!(
            matches!(err, LoadError::TooLarge { limit, .. } if limit == HUGE_FILE_BYTES),
            "past HUGE_FILE_BYTES nothing is read whole, whatever a setting says"
        );
    }

    #[test]
    fn the_refusal_names_the_file_its_size_and_the_limit() {
        let dir = tempdir().unwrap();
        let path = sparse(dir.path(), "server.log", 40 * 1024 * 1024);

        let message = load(&path).unwrap_err().to_string();

        assert!(message.contains("server.log"), "{message}");
        assert!(message.contains("40.0 MiB"), "{message}");
        assert!(message.contains("32.0 MiB"), "{message}");
        // What to do instead is the shell's to say: it opens the file in its
        // read-only viewer, and a message from here promising a viewer "to
        // come" outlived the viewer's arrival once already.
        assert!(!message.contains("coming"), "{message}");
    }

    #[test]
    fn a_file_that_grows_past_the_limit_after_its_size_is_checked_is_still_refused() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("growing.log");
        std::fs::write(&path, b"12345678").unwrap();
        let file = File::open(&path).unwrap();
        let stated = file.metadata().unwrap().len();

        // The writer appends between the stat and the read.
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(&[b'x'; 1000])
            .unwrap();

        let err = read_capped(&path, &file, stated, 16).unwrap_err();
        assert!(
            matches!(
                err,
                LoadError::TooLarge {
                    bytes: 17,
                    limit: 16,
                    ..
                }
            ),
            "the read must stop one byte past the limit rather than take the \
             whole file, got {err:?}"
        );
    }

    #[test]
    fn a_source_that_never_ends_is_refused_rather_than_read_forever() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("device");

        let err = read_capped(&path, io::repeat(b'a'), 0, 1024).unwrap_err();

        assert!(matches!(err, LoadError::TooLarge { bytes: 1025, .. }));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_proc_file_that_reports_no_size_is_still_held_to_the_limit() {
        let path = Path::new("/proc/self/status");
        assert_eq!(
            std::fs::metadata(path).unwrap().len(),
            0,
            "the premise: /proc reports zero whatever the entry holds"
        );

        let err = load_with_limit(path, 8).unwrap_err();

        assert!(matches!(
            err,
            LoadError::TooLarge {
                bytes: 9,
                limit: 8,
                ..
            }
        ));
    }

    #[test]
    fn a_small_file_is_normal_and_one_past_the_large_threshold_is_large() {
        let dir = tempdir().unwrap();
        let small = write_bytes(dir.path(), "small.txt", b"hello");
        let large = sparse(dir.path(), "large.txt", LARGE_FILE_BYTES + 1);

        assert_eq!(load(&small).unwrap().size_class(), SizeClass::Normal);
        assert_eq!(
            load(&large).unwrap().size_class(),
            SizeClass::Large,
            "between LARGE_FILE_BYTES and the editable limit a file loads, \
             and says so, so the shell can warn"
        );
    }

    #[test]
    fn sizes_read_as_a_person_would_say_them() {
        assert_eq!(Size(900).to_string(), "900 bytes");
        assert_eq!(Size(1536).to_string(), "1.5 KiB");
        assert_eq!(Size(EDITABLE_LIMIT_BYTES).to_string(), "32.0 MiB");
        assert_eq!(Size(4 << 30).to_string(), "4.0 GiB");
        assert_eq!(Size(u64::MAX).to_string(), "17179869183.9 GiB");
    }
}
