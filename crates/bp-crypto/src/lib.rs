//! The `.bpadx` encrypted document format (ADR-0021).
//!
//! **This crate composes cryptography; it implements none.** Argon2id comes
//! from `argon2`, the AEADs from `chacha20poly1305` and `aes-gcm`, randomness
//! from the OS. ADR-0011 draws the line exactly there: designing the envelope
//! is ours to do, and writing a cipher is not.
//!
//! It also decides no policy. Whether a document *should* be encrypted is
//! `bp-security`'s question (ADR-0020); neither crate depends on the other.
//!
//! ## The shape of a document
//!
//! ```text
//! header  magic, version, suite, KDF cost, salt, chunk size, nonce prefix
//! chunk 0 u32 length, then ciphertext+tag
//! chunk 1 ...
//! ```
//!
//! Each chunk is sealed separately, and its additional authenticated data is
//! **the whole header, the chunk's index, and whether it is the last**. Each
//! of those three prevents a specific attack, and leaving any one out gives a
//! format that still decrypts perfectly while being silently wrong:
//!
//! * without the header, the suite or the KDF cost can be edited to downgrade
//!   the document;
//! * without the index, chunks can be reordered or duplicated and every one
//!   still authenticates;
//! * without the last-chunk flag, the file can be truncated and the remainder
//!   still authenticates -- data loss that looks like a successful decrypt.
//!
//! ## One thing this cannot do
//!
//! **Tell a wrong passphrase from a corrupted file.** Both are an
//! authentication failure, and distinguishing them would mean trusting
//! something outside the authenticated data. [`CryptoError::CannotOpen`] says
//! both, deliberately.

#![forbid(unsafe_code)]

use aes_gcm::Aes256Gcm;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::XChaCha20Poly1305;
use zeroize::{Zeroize, Zeroizing};

mod header;
mod sign;

pub use header::{
    DEFAULT_CHUNK_SIZE, Header, KdfParams, MAX_ITERATIONS, MAX_LANES, MAX_MEMORY_KIB, UnlockCost,
};
use header::{MAGIC, MAX_CHUNK_SIZE, SALT_LEN, VERSION};
pub use sign::{
    DocumentHash, HashAlgorithm, SIGNATURE_LEN, SIGNING_KEY_LEN, SignError, Signature, SigningKey,
    VERIFYING_KEY_LEN, VerifyingKey, hash_document, sign_document, verify_document,
};

/// Crate identity used by workspace smoke tests and diagnostics.
pub const CRATE_NAME: &str = "bp-crypto";

/// Which AEAD sealed a document.
///
/// Recorded per document rather than fixed, so the one not chosen today
/// remains available without a format break (ADR-0021).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Suite {
    /// The default. A 192-bit nonce makes a random per-file prefix safe
    /// without ceremony, and it is constant-time in software on any CPU --
    /// including those without AES instructions, where a software AES leaks
    /// timing.
    #[default]
    XChaCha20Poly1305,
    /// Hardware-accelerated almost everywhere, and the answer to compliance
    /// questions. The 96-bit nonce leaves less room, which the chunk counter
    /// accounts for.
    Aes256Gcm,
}

impl Suite {
    pub(crate) const fn tag(self) -> u8 {
        match self {
            Self::XChaCha20Poly1305 => 1,
            Self::Aes256Gcm => 2,
        }
    }

    pub(crate) fn from_tag(tag: u8) -> Result<Self, CryptoError> {
        match tag {
            1 => Ok(Self::XChaCha20Poly1305),
            2 => Ok(Self::Aes256Gcm),
            other => Err(CryptoError::UnsupportedSuite(other)),
        }
    }

    pub(crate) const fn nonce_len(self) -> usize {
        match self {
            Self::XChaCha20Poly1305 => 24,
            Self::Aes256Gcm => 12,
        }
    }

    /// How many random bytes lead each nonce. The remaining eight carry the
    /// chunk index, so a nonce cannot repeat within a file.
    pub(crate) const fn nonce_prefix_len(self) -> usize {
        self.nonce_len() - 8
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::XChaCha20Poly1305 => "XChaCha20-Poly1305",
            Self::Aes256Gcm => "AES-256-GCM",
        }
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum CryptoError {
    #[error("this is not a BachelorPad+ encrypted document")]
    NotBpadx,

    #[error(
        "this document was written by a newer BachelorPad+ (format {found}, \
         this build understands {supported})"
    )]
    UnsupportedVersion { found: u16, supported: u16 },

    #[error("unknown encryption suite {0}; this document needs a newer BachelorPad+")]
    UnsupportedSuite(u8),

    #[error("unknown key-derivation method {0}; this document needs a newer BachelorPad+")]
    UnsupportedKdf(u8),

    /// The one failure that cannot be diagnosed further. See the crate docs.
    #[error("wrong passphrase, or this document has been altered or damaged")]
    CannotOpen,

    #[error("this document is incomplete -- it was truncated or only partly copied")]
    Truncated,

    #[error("this document's structure is damaged")]
    Corrupt,

    #[error(
        "this document asks for an unreasonable amount of work to open \
         ({memory_kib} KiB, {iterations} passes, {lanes} lanes; the most this \
         build will do is {max_mib} MiB, {MAX_ITERATIONS} passes and \
         {MAX_LANES} lanes) and was refused",
        max_mib = MAX_MEMORY_KIB / 1024
    )]
    UnreasonableCost {
        memory_kib: u32,
        iterations: u32,
        lanes: u32,
    },

    /// Within the ceiling but above four times the default cost, and
    /// [`open`] was used rather than [`open_confirmed`]. Not a verdict on the
    /// document: the caller has to ask the user before paying for it.
    #[error(
        "this document asks for an unusually expensive unlock ({memory_kib} KiB, \
         {iterations} passes), which has to be agreed to before it is attempted"
    )]
    UnconfirmedCost {
        memory_kib: u32,
        iterations: u32,
        lanes: u32,
    },

    #[error("this document declares an unreasonable chunk size ({0} bytes) and was refused")]
    UnreasonableChunkSize(u32),

    #[error("the passphrase is empty")]
    EmptyPassphrase,

    #[error("could not read the system random number generator: {0}")]
    NoRandomness(String),

    #[error("key derivation failed: {0}")]
    Kdf(String),
}

/// How a document should be sealed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SealOptions {
    pub suite: Suite,
    pub kdf: KdfParams,
    pub chunk_size: u32,
}

impl Default for SealOptions {
    fn default() -> Self {
        Self {
            suite: Suite::default(),
            kdf: KdfParams::default(),
            chunk_size: DEFAULT_CHUNK_SIZE,
        }
    }
}

/// Bytes an AEAD adds to a chunk: the authentication tag.
///
/// The same for both suites -- Poly1305 and GCM both produce 16 bytes -- which
/// is why one constant serves. A suite with a different tag size would need
/// this to move onto `Suite`.
const TAG_OVERHEAD: usize = 16;

/// A derived key, wiped when it goes out of scope.
///
/// `Zeroizing` rather than discipline: an early return that forgot to clear
/// this would leave the document's key in freed memory, and there are several
/// early returns below.
type Key = Zeroizing<[u8; 32]>;

fn derive(passphrase: &str, salt: &[u8], params: KdfParams) -> Result<Key, CryptoError> {
    params.validate()?;
    let argon = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(params.memory_kib, params.iterations, params.lanes, Some(32))
            .map_err(|e| CryptoError::Kdf(e.to_string()))?,
    );

    let mut key = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(passphrase.as_bytes(), salt, key.as_mut())
        .map_err(|e| CryptoError::Kdf(e.to_string()))?;
    Ok(key)
}

fn random(bytes: &mut [u8]) -> Result<(), CryptoError> {
    getrandom::fill(bytes).map_err(|e| CryptoError::NoRandomness(e.to_string()))
}

/// The additional authenticated data for one chunk.
///
/// Header, index, last-chunk flag. All three, always -- see the crate docs for
/// what each one prevents.
fn aad(header_bytes: &[u8], index: u64, last: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(header_bytes.len() + 9);
    out.extend_from_slice(header_bytes);
    out.extend_from_slice(&index.to_be_bytes());
    out.push(u8::from(last));
    out
}

/// The nonce for one chunk: the file's random prefix, then the index.
fn nonce(prefix: &[u8], index: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(prefix.len() + 8);
    out.extend_from_slice(prefix);
    out.extend_from_slice(&index.to_be_bytes());
    out
}

fn seal_chunk(
    suite: Suite,
    key: &Key,
    nonce: &[u8],
    aad: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let payload = Payload {
        msg: plaintext,
        aad,
    };
    match suite {
        Suite::XChaCha20Poly1305 => XChaCha20Poly1305::new(key.as_ref().into())
            .encrypt(nonce.into(), payload)
            .map_err(|_| CryptoError::CannotOpen),
        Suite::Aes256Gcm => Aes256Gcm::new(key.as_ref().into())
            .encrypt(nonce.into(), payload)
            .map_err(|_| CryptoError::CannotOpen),
    }
}

fn open_chunk(
    suite: Suite,
    key: &Key,
    nonce: &[u8],
    aad: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let payload = Payload {
        msg: ciphertext,
        aad,
    };
    match suite {
        Suite::XChaCha20Poly1305 => XChaCha20Poly1305::new(key.as_ref().into())
            .decrypt(nonce.into(), payload)
            .map_err(|_| CryptoError::CannotOpen),
        Suite::Aes256Gcm => Aes256Gcm::new(key.as_ref().into())
            .decrypt(nonce.into(), payload)
            .map_err(|_| CryptoError::CannotOpen),
    }
}

/// Encrypt `plaintext` into a `.bpadx` document.
///
/// An empty passphrase is refused rather than accepted as a weak one: a
/// document that opens with no passphrase is not encrypted in any sense the
/// user would recognise, and offering it would be the lie ADR-0020 forbids
/// elsewhere.
pub fn seal(
    plaintext: &[u8],
    passphrase: &str,
    options: SealOptions,
) -> Result<Vec<u8>, CryptoError> {
    if passphrase.is_empty() {
        return Err(CryptoError::EmptyPassphrase);
    }
    options.kdf.validate()?;
    if !(header::MIN_CHUNK_SIZE..=MAX_CHUNK_SIZE).contains(&options.chunk_size) {
        return Err(CryptoError::UnreasonableChunkSize(options.chunk_size));
    }

    let mut salt = [0u8; SALT_LEN];
    random(&mut salt)?;
    let mut nonce_prefix = vec![0u8; options.suite.nonce_prefix_len()];
    random(&mut nonce_prefix)?;

    let header = Header {
        version: VERSION,
        suite: options.suite,
        kdf: options.kdf,
        salt,
        chunk_size: options.chunk_size,
        nonce_prefix,
    };
    let header_bytes = header.to_bytes();
    let key = derive(passphrase, &header.salt, header.kdf)?;

    let mut out = header_bytes.clone();
    let size = options.chunk_size as usize;

    // An empty document still gets one chunk. Without it there would be
    // nothing to authenticate, so an empty `.bpadx` would open under any
    // passphrase at all.
    let chunks: Vec<&[u8]> = if plaintext.is_empty() {
        vec![&[]]
    } else {
        plaintext.chunks(size).collect()
    };
    let last_index = chunks.len() - 1;

    for (index, chunk) in chunks.iter().enumerate() {
        let i = index as u64;
        let sealed = seal_chunk(
            options.suite,
            &key,
            &nonce(&header.nonce_prefix, i),
            &aad(&header_bytes, i, index == last_index),
            chunk,
        )?;
        let len = u32::try_from(sealed.len()).map_err(|_| CryptoError::Corrupt)?;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&sealed);
    }
    Ok(out)
}

/// Decrypt a `.bpadx` document whose declared cost is [`UnlockCost::Normal`].
///
/// A cost above four times the default is refused with
/// [`CryptoError::UnconfirmedCost`] before anything is derived, so a caller
/// that never asks the user -- the recovery journal, the audit log, the
/// signing-key store -- cannot be made to pay for one by an edited header.
/// The shell asks, and then calls [`open_confirmed`].
///
/// The plaintext is returned in a `Zeroizing` wrapper: this is document
/// content that was encrypted at rest, and handing it back in a plain `Vec`
/// would leave it in freed memory the moment the caller drops it.
pub fn open(document: &[u8], passphrase: &str) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    open_at(document, passphrase, false)
}

/// Decrypt a `.bpadx` document, paying any cost up to the ceiling.
///
/// For after the user has agreed to an [`UnlockCost::NeedsConfirmation`]
/// cost. The ceiling still holds: an [`UnlockCost::Refused`] document is
/// refused here exactly as by [`open`], whoever agreed to what.
pub fn open_confirmed(
    document: &[u8],
    passphrase: &str,
) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    open_at(document, passphrase, true)
}

/// The cost a document declares, read from its header without deriving
/// anything and without judging it.
///
/// Every other header field is checked as [`open`] would check it, so an
/// `Ok` here is a structurally sound header. The cost itself is untrusted:
/// classify it with [`KdfParams::unlock_cost`] before paying it.
pub fn declared_cost(document: &[u8]) -> Result<KdfParams, CryptoError> {
    Header::parse_declared(document).map(|(header, _)| header.kdf)
}

/// What opening `document` will cost (decision 15), read from its header
/// alone.
///
/// `Err` for a header that is damaged, unknown or not a `.bpadx` at all; a
/// cost past the ceiling is `Ok(UnlockCost::Refused(..))`, so the caller has
/// one place to learn the reason whichever way the cost fails.
pub fn unlock_cost(document: &[u8]) -> Result<UnlockCost, CryptoError> {
    declared_cost(document).map(KdfParams::unlock_cost)
}

fn open_at(
    document: &[u8],
    passphrase: &str,
    expensive_agreed: bool,
) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    let (header, header_len) = Header::parse(document)?;
    if !expensive_agreed && header.kdf.needs_confirmation() {
        return Err(CryptoError::UnconfirmedCost {
            memory_kib: header.kdf.memory_kib,
            iterations: header.kdf.iterations,
            lanes: header.kdf.lanes,
        });
    }
    let header_bytes = &document[..header_len];
    let key = derive(passphrase, &header.salt, header.kdf)?;

    // Wrapped and sized up front, both deliberately.
    //
    // `Zeroizing` from the start rather than only on the returned value:
    // growing a plain `Vec` reallocates, and each reallocation copies the
    // plaintext accumulated so far into a new buffer and frees the old one
    // *unwiped*. Wrapping only at the end would leave every earlier
    // allocation of a multi-chunk document lying in freed memory, which is
    // precisely the guarantee this function's documentation makes.
    //
    // Reserving the plaintext's maximum size means there are no reallocations
    // to leak through in the first place. It is bounded by the file's own
    // length, so a hostile header cannot use it to demand memory.
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::with_capacity(
        document.len().saturating_sub(header_len),
    ));
    let mut at = header_len;
    let mut index: u64 = 0;

    // What a single chunk may decrypt to, from the header the chunks
    // themselves authenticate. Without this the declared chunk size is
    // validated, authenticated, and then never used for anything -- which a
    // reader would reasonably assume constrains the framing.
    let max_ciphertext = (header.chunk_size as usize).saturating_add(TAG_OVERHEAD);

    loop {
        if at == document.len() {
            // Ran out of chunks without one claiming to be the last: the file
            // was cut short. Distinct from a damaged chunk, and the user can
            // act on the difference.
            return Err(CryptoError::Truncated);
        }
        // `checked_add` rather than `+`, matching `header::take`. `len` is
        // attacker-controlled up to `u32::MAX`, and on a 32-bit target a bare
        // addition wraps -- panicking in debug, and in release yielding a
        // bogus slice that happens to be in range.
        let end = at.checked_add(4).ok_or(CryptoError::Truncated)?;
        let len_bytes: [u8; 4] = document
            .get(at..end)
            .ok_or(CryptoError::Truncated)?
            .try_into()
            .map_err(|_| CryptoError::Truncated)?;
        at = end;

        let len = u32::from_le_bytes(len_bytes) as usize;
        if len > max_ciphertext {
            // Longer than the header says a chunk can be. The header is
            // authenticated, so this is either damage or a forgery attempt,
            // and either way there is nothing to decrypt.
            return Err(CryptoError::Corrupt);
        }
        let end = at.checked_add(len).ok_or(CryptoError::Truncated)?;
        // A hostile length must not make this allocate before anything has
        // authenticated. The slice is bounded by the file itself.
        let ciphertext = document.get(at..end).ok_or(CryptoError::Truncated)?;
        at = end;

        let last = at == document.len();
        match open_chunk(
            header.suite,
            &key,
            &nonce(&header.nonce_prefix, index),
            &aad(header_bytes, index, last),
            ciphertext,
        ) {
            Ok(mut chunk) => {
                out.extend_from_slice(&chunk);
                chunk.zeroize();
            }
            Err(e) => {
                // `out` wipes itself on drop, but doing it here says so at the
                // point it matters rather than relying on the reader knowing.
                out.zeroize();
                return Err(e);
            }
        }

        if last {
            return Ok(out);
        }
        index += 1;
    }
}

/// A stable, filesystem-safe name derived from arbitrary bytes.
///
/// For naming recovery-journal files after the document they belong to,
/// without writing the path down (ADR-0022). SHA-256 truncated to 32 hex
/// characters -- ample against accidental collision, and short enough to stay
/// a sane filename.
///
/// **This is not a secret.** Anyone holding the recovery directory can test a
/// guessed path against it, so it hides *which* documents have unsaved work
/// only from someone who cannot guess their paths. That is the trade the
/// alternative -- writing the path in clear beside the ciphertext -- loses
/// outright.
///
/// Not used for key derivation anywhere. Keys come from Argon2id.
#[must_use]
pub fn stable_name(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().take(16).map(|b| format!("{b:02x}")).collect()
}

/// Whether `bytes` begins with the `.bpadx` magic.
///
/// Cheap enough to call on every file open, so the shell can route a document
/// to the passphrase prompt instead of showing the user its ciphertext.
#[must_use]
pub fn is_bpadx(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deliberately cheap key derivation.
    ///
    /// The real cost is 19 MiB and ~50 ms; multiplied across this module that
    /// is minutes of gate time for no extra coverage, because the cost
    /// parameter is not what any of these tests are about. The one test that
    /// *is* about cost uses real values.
    fn cheap() -> KdfParams {
        KdfParams {
            memory_kib: 8,
            iterations: 1,
            lanes: 1,
        }
    }

    fn options(suite: Suite) -> SealOptions {
        SealOptions {
            suite,
            kdf: cheap(),
            chunk_size: header::MIN_CHUNK_SIZE,
        }
    }

    const PASS: &str = "correct horse battery staple";

    #[test]
    fn a_document_round_trips_through_both_suites() {
        for suite in [Suite::XChaCha20Poly1305, Suite::Aes256Gcm] {
            let text = b"the quick brown fox";
            let sealed = seal(text, PASS, options(suite)).unwrap();

            assert!(is_bpadx(&sealed), "{} lost the magic", suite.name());
            assert_eq!(
                open(&sealed, PASS).unwrap().as_slice(),
                text,
                "{} did not round trip",
                suite.name()
            );
        }
    }

    #[test]
    fn the_ciphertext_does_not_contain_the_plaintext() {
        // The most basic thing a reviewer should be able to check, and the
        // one that would catch a catastrophic wiring mistake instantly.
        let text = b"SECRETSECRETSECRET";
        let sealed = seal(text, PASS, options(Suite::default())).unwrap();

        assert!(
            !sealed.windows(text.len()).any(|w| w == text),
            "the plaintext is sitting in the output"
        );
    }

    #[test]
    fn sealing_the_same_document_twice_gives_different_bytes() {
        // A fresh salt and nonce prefix each time. Identical output would mean
        // one of them was fixed, which makes the key reusable across documents
        // and the nonces repeat with it.
        let a = seal(b"same", PASS, options(Suite::default())).unwrap();
        let b = seal(b"same", PASS, options(Suite::default())).unwrap();

        assert_ne!(a, b, "output is deterministic; a salt or nonce is fixed");
    }

    #[test]
    fn a_wrong_passphrase_does_not_open_the_document() {
        let sealed = seal(b"secret", PASS, options(Suite::default())).unwrap();

        assert_eq!(
            open(&sealed, "wrong passphrase"),
            Err(CryptoError::CannotOpen)
        );
    }

    #[test]
    fn an_empty_passphrase_is_refused_rather_than_treated_as_a_weak_one() {
        // A document that opens with no passphrase is not encrypted in any
        // sense the user would recognise.
        assert_eq!(
            seal(b"secret", "", options(Suite::default())),
            Err(CryptoError::EmptyPassphrase)
        );
    }

    #[test]
    fn a_flipped_ciphertext_bit_is_caught() {
        for suite in [Suite::XChaCha20Poly1305, Suite::Aes256Gcm] {
            let mut sealed = seal(b"secret document", PASS, options(suite)).unwrap();
            let last = sealed.len() - 1;
            sealed[last] ^= 0x01;

            assert_eq!(
                open(&sealed, PASS),
                Err(CryptoError::CannotOpen),
                "{} accepted altered ciphertext",
                suite.name()
            );
        }
    }

    #[test]
    fn editing_the_suite_in_the_header_is_caught() {
        // The header is authenticated as additional data on every chunk, so an
        // edit to it cannot silently change how the document is read. A header
        // that were only *read* could be downgraded.
        let mut sealed = seal(b"secret", PASS, options(Suite::XChaCha20Poly1305)).unwrap();
        // Byte 8: magic (6) + version (2), then the suite tag.
        assert_eq!(sealed[8], Suite::XChaCha20Poly1305.tag());
        sealed[8] = Suite::Aes256Gcm.tag();

        // Rejected either as a structural mismatch (the nonce width no longer
        // matches the declared suite) or as a failed authentication. Both are
        // refusals; what matters is that it does not open.
        assert!(open(&sealed, PASS).is_err(), "a downgraded header opened");
    }

    #[test]
    fn editing_the_kdf_cost_in_the_header_is_caught() {
        // Lowering the cost would let an attacker who copied the file brute
        // force it more cheaply, if the reader honoured the edit.
        let mut sealed = seal(b"secret", PASS, options(Suite::default())).unwrap();
        // magic 6 + version 2 + suite 1 + kdf id 1 = 10, then memory_kib.
        // The document was sealed at `cheap()`'s 8 KiB, so the edit has to be
        // to a *different* legal value -- writing 8 back would test nothing,
        // which is what the first version of this test did.
        assert_eq!(u32::from_le_bytes(sealed[10..14].try_into().unwrap()), 8);
        sealed[10..14].copy_from_slice(&64u32.to_le_bytes());

        assert!(open(&sealed, PASS).is_err(), "an edited cost was honoured");
    }

    /// Split a sealed document into its header and its framed chunks.
    fn split(sealed: &[u8]) -> (Vec<u8>, Vec<Vec<u8>>) {
        let (_, header_len) = Header::parse(sealed).unwrap();
        let mut chunks = Vec::new();
        let mut at = header_len;
        while at < sealed.len() {
            let len = u32::from_le_bytes(sealed[at..at + 4].try_into().unwrap()) as usize;
            chunks.push(sealed[at..at + 4 + len].to_vec());
            at += 4 + len;
        }
        (sealed[..header_len].to_vec(), chunks)
    }

    #[test]
    fn reordering_chunks_is_caught() {
        // Without the chunk index in the additional data, every reordered
        // chunk would still authenticate and the document would open with its
        // contents rearranged -- corruption that looks like success.
        let text: Vec<u8> = (0..4096u32).flat_map(u32::to_le_bytes).collect();
        let sealed = seal(&text, PASS, options(Suite::default())).unwrap();
        let (header, chunks) = split(&sealed);
        assert!(
            chunks.len() >= 3,
            "need several chunks; got {}",
            chunks.len()
        );

        let mut swapped = header;
        swapped.extend_from_slice(&chunks[1]);
        swapped.extend_from_slice(&chunks[0]);
        for chunk in &chunks[2..] {
            swapped.extend_from_slice(chunk);
        }

        assert_eq!(open(&swapped, PASS), Err(CryptoError::CannotOpen));
    }

    #[test]
    fn duplicating_a_chunk_is_caught() {
        let text: Vec<u8> = (0..4096u32).flat_map(u32::to_le_bytes).collect();
        let sealed = seal(&text, PASS, options(Suite::default())).unwrap();
        let (header, chunks) = split(&sealed);

        let mut doubled = header;
        doubled.extend_from_slice(&chunks[0]);
        for chunk in &chunks {
            doubled.extend_from_slice(chunk);
        }

        assert!(open(&doubled, PASS).is_err());
    }

    #[test]
    fn truncating_the_document_is_caught() {
        // The attack the last-chunk flag exists for. Without it the remaining
        // chunks all authenticate and the document opens short -- silent data
        // loss presented as a successful decrypt.
        let text: Vec<u8> = (0..4096u32).flat_map(u32::to_le_bytes).collect();
        let sealed = seal(&text, PASS, options(Suite::default())).unwrap();
        let (header, chunks) = split(&sealed);

        let mut cut = header;
        for chunk in &chunks[..chunks.len() - 1] {
            cut.extend_from_slice(chunk);
        }

        assert_eq!(
            open(&cut, PASS),
            Err(CryptoError::CannotOpen),
            "a truncated document must not open with its remaining chunks"
        );
    }

    #[test]
    fn a_document_cut_mid_chunk_is_reported_as_incomplete() {
        // Distinct from damage: the user can act on "incomplete" by finding a
        // better copy.
        let sealed = seal(b"secret document", PASS, options(Suite::default())).unwrap();
        let cut = &sealed[..sealed.len() - 5];

        assert_eq!(open(cut, PASS), Err(CryptoError::Truncated));
    }

    #[test]
    fn a_file_that_is_not_bpadx_is_named_rather_than_guessed_at() {
        assert!(!is_bpadx(b"just some text"));
        assert_eq!(open(b"just some text", PASS), Err(CryptoError::NotBpadx));
    }

    #[test]
    fn a_newer_format_version_is_refused_by_name() {
        // "Could not open" tells the user nothing they can act on; "written by
        // a newer BachelorPad+" tells them to update.
        let mut sealed = seal(b"secret", PASS, options(Suite::default())).unwrap();
        sealed[6] = 99;
        sealed[7] = 0;

        assert_eq!(
            open(&sealed, PASS),
            Err(CryptoError::UnsupportedVersion {
                found: 99,
                supported: VERSION,
            })
        );
    }

    #[test]
    fn an_unknown_suite_is_refused_by_name() {
        let mut sealed = seal(b"secret", PASS, options(Suite::default())).unwrap();
        sealed[8] = 77;

        assert_eq!(open(&sealed, PASS), Err(CryptoError::UnsupportedSuite(77)));
    }

    #[test]
    fn an_absurd_work_factor_is_refused_rather_than_attempted() {
        // A hostile header asking for 64 GiB is a denial-of-service, not a
        // strong document. Refusing is the difference between a message and a
        // machine that stops responding.
        let mut sealed = seal(b"secret", PASS, options(Suite::default())).unwrap();
        let absurd = (64u32 * 1024 * 1024).to_le_bytes();
        sealed[10..14].copy_from_slice(&absurd);

        assert!(matches!(
            open(&sealed, PASS),
            Err(CryptoError::UnreasonableCost { .. })
        ));
    }

    #[test]
    fn an_absurd_chunk_size_is_refused() {
        let mut sealed = seal(b"secret", PASS, options(Suite::default())).unwrap();
        // magic 6 + version 2 + suite 1 + kdf 1 + cost 12 + salt 16 = 38.
        let absurd = u32::MAX.to_le_bytes();
        sealed[38..42].copy_from_slice(&absurd);

        assert!(matches!(
            open(&sealed, PASS),
            Err(CryptoError::UnreasonableChunkSize(_))
        ));
    }

    #[test]
    fn an_empty_document_is_still_authenticated() {
        // It gets one chunk holding nothing. Without it there would be nothing
        // to authenticate, and an empty `.bpadx` would open under any
        // passphrase at all.
        let sealed = seal(b"", PASS, options(Suite::default())).unwrap();

        assert!(open(&sealed, PASS).unwrap().is_empty());
        assert_eq!(open(&sealed, "wrong"), Err(CryptoError::CannotOpen));
    }

    #[test]
    fn a_document_spanning_many_chunks_round_trips_exactly() {
        let text: Vec<u8> = (0..50_000u32).map(|i| (i % 251) as u8).collect();
        let sealed = seal(&text, PASS, options(Suite::default())).unwrap();

        assert_eq!(open(&sealed, PASS).unwrap().as_slice(), text.as_slice());
    }

    #[test]
    fn a_document_around_one_chunk_long_round_trips() {
        // The boundary where "is this the last chunk" is decided.
        let size = header::MIN_CHUNK_SIZE as usize;
        for len in [size - 1, size, size + 1] {
            let text = vec![b'x'; len];
            let sealed = seal(&text, PASS, options(Suite::default())).unwrap();
            assert_eq!(
                open(&sealed, PASS).unwrap().as_slice(),
                text.as_slice(),
                "failed at {len} bytes"
            );
        }
    }

    #[test]
    fn the_default_work_factor_is_the_owasp_baseline() {
        // The one test that uses the real cost. ADR-0021 pins these numbers,
        // and lowering them silently would be a security change disguised as a
        // performance one.
        let kdf = KdfParams::default();
        assert_eq!(kdf.memory_kib, 19 * 1024);
        assert_eq!(kdf.iterations, 2);
        assert_eq!(kdf.lanes, 1);

        // And it genuinely works end to end at that cost, not only at the
        // cheap one the rest of this module uses.
        let sealed = seal(b"real cost", PASS, SealOptions::default()).unwrap();
        assert_eq!(open(&sealed, PASS).unwrap().as_slice(), b"real cost");
    }

    #[test]
    fn the_stored_parameters_are_what_a_reader_uses() {
        // The reason cost lives in the envelope: a document written at one
        // cost must open on a build whose default is another.
        let sealed = seal(
            b"written cheaply",
            PASS,
            SealOptions {
                kdf: cheap(),
                ..SealOptions::default()
            },
        )
        .unwrap();

        let (header, _) = Header::parse(&sealed).unwrap();
        assert_eq!(header.kdf, cheap());
        assert_eq!(open(&sealed, PASS).unwrap().as_slice(), b"written cheaply");
    }

    // --- decision 15: the ceiling and the question -----------------------

    /// A cost with one lane, for the boundary tests below.
    fn cost(memory_kib: u32, iterations: u32) -> KdfParams {
        KdfParams {
            memory_kib,
            iterations,
            lanes: 1,
        }
    }

    /// A cheaply sealed document with its declared cost rewritten.
    ///
    /// The rewrite breaks authentication, which is the point: these are the
    /// headers an attacker writes, and what is under test is whether the
    /// reader pays for them before it finds out.
    fn declaring(kdf: KdfParams) -> Vec<u8> {
        let mut sealed = seal(b"secret", PASS, options(Suite::default())).unwrap();
        // magic 6 + version 2 + suite 1 + kdf id 1 = 10.
        sealed[10..14].copy_from_slice(&kdf.memory_kib.to_le_bytes());
        sealed[14..18].copy_from_slice(&kdf.iterations.to_le_bytes());
        sealed[18..22].copy_from_slice(&kdf.lanes.to_le_bytes());
        sealed
    }

    /// Four times the default is 77,824 KiB of memory and 155,648 KiB-passes
    /// of work; this sits exactly on both.
    fn exactly_four_times() -> KdfParams {
        cost(4 * 19 * 1024, 2)
    }

    #[test]
    fn the_default_cost_opens_without_asking() {
        assert_eq!(KdfParams::default().unlock_cost(), UnlockCost::Normal);
    }

    #[test]
    fn exactly_four_times_the_default_opens_without_asking() {
        // "Above four times", so four times itself is normal -- by memory and
        // by work at once, and by work alone at the default's memory share.
        assert_eq!(exactly_four_times().unlock_cost(), UnlockCost::Normal);
        assert_eq!(cost(2 * 19 * 1024, 4).unlock_cost(), UnlockCost::Normal);
    }

    #[test]
    fn one_kib_pass_more_work_than_four_times_the_default_asks_first() {
        // 38,913 KiB at 4 passes: the memory is well inside four times the
        // default, the work is four KiB-passes over.
        assert!(matches!(
            cost(2 * 19 * 1024 + 1, 4).unlock_cost(),
            UnlockCost::NeedsConfirmation { .. }
        ));
        assert!(matches!(
            cost(4 * 19 * 1024 + 1, 2).unlock_cost(),
            UnlockCost::NeedsConfirmation { .. }
        ));
    }

    #[test]
    fn one_kib_more_memory_than_four_times_the_default_asks_first_even_at_one_pass() {
        // Half the default's work at 77,825 KiB and one pass, but more than
        // four times its memory: a small machine runs out of memory, not
        // patience, and it is asked about either.
        assert!(matches!(
            cost(4 * 19 * 1024 + 1, 1).unlock_cost(),
            UnlockCost::NeedsConfirmation { .. }
        ));
    }

    #[test]
    fn the_ceiling_itself_is_allowed_once_agreed_to_and_says_how_long_it_takes() {
        assert_eq!(
            cost(MAX_MEMORY_KIB, MAX_ITERATIONS).unlock_cost(),
            UnlockCost::NeedsConfirmation {
                memory_kib: 256 * 1024,
                iterations: 8,
                // 2 GiB-passes at the assumed 512 MiB-passes a second.
                estimate: std::time::Duration::from_secs(4),
            }
        );
        let widest = KdfParams {
            lanes: MAX_LANES,
            ..cost(MAX_MEMORY_KIB, MAX_ITERATIONS)
        };
        assert!(matches!(
            widest.unlock_cost(),
            UnlockCost::NeedsConfirmation { .. }
        ));
    }

    #[test]
    fn one_step_past_the_ceiling_is_refused_whoever_agrees() {
        for past in [
            cost(MAX_MEMORY_KIB + 1, 1),
            cost(8, MAX_ITERATIONS + 1),
            KdfParams {
                lanes: MAX_LANES + 1,
                ..cost(MAX_MEMORY_KIB, 1)
            },
            // The ceiling before decision 15, which took over a minute.
            cost(1024 * 1024, 64),
        ] {
            assert_eq!(
                past.unlock_cost(),
                UnlockCost::Refused(CryptoError::UnreasonableCost {
                    memory_kib: past.memory_kib,
                    iterations: past.iterations,
                    lanes: past.lanes,
                }),
                "{past:?} was not refused"
            );
            let document = declaring(past);
            assert!(matches!(
                open_confirmed(&document, PASS),
                Err(CryptoError::UnreasonableCost { .. })
            ));
        }
    }

    #[test]
    fn the_refusal_names_the_ceiling() {
        let message = cost(MAX_MEMORY_KIB + 1, 1).unlock_cost();
        let UnlockCost::Refused(error) = message else {
            unreachable!("past the ceiling is refused; see the test above");
        };
        let text = error.to_string();
        assert!(
            text.contains("256 MiB") && text.contains("8 passes"),
            "{text}"
        );
    }

    #[test]
    fn a_cost_past_the_ceiling_is_refused_in_microseconds_not_after_a_derivation() {
        // The whole point of a ceiling on an unauthenticated header: the
        // refusal has to come before the work, or it refuses nothing. A
        // derivation at even the lowest refused cost here takes seconds, so
        // a bound of milliseconds cannot be met by one.
        let document = declaring(cost(MAX_MEMORY_KIB + 1, MAX_ITERATIONS));

        let started = std::time::Instant::now();
        let classified = unlock_cost(&document);
        let refused = open_confirmed(&document, PASS);
        let elapsed = started.elapsed();

        assert!(matches!(classified, Ok(UnlockCost::Refused(_))));
        assert!(matches!(refused, Err(CryptoError::UnreasonableCost { .. })));
        assert!(
            elapsed < std::time::Duration::from_millis(10),
            "refusing took {elapsed:?}"
        );
    }

    #[test]
    fn an_unagreed_expensive_cost_is_refused_by_open_before_deriving() {
        // `open` is what every caller that cannot ask uses. The ceiling here
        // would take seconds to derive, so being refused in milliseconds
        // shows none of it was paid.
        let document = declaring(cost(MAX_MEMORY_KIB, MAX_ITERATIONS));

        let started = std::time::Instant::now();
        let refused = open(&document, PASS);
        let elapsed = started.elapsed();

        assert_eq!(
            refused,
            Err(CryptoError::UnconfirmedCost {
                memory_kib: MAX_MEMORY_KIB,
                iterations: MAX_ITERATIONS,
                lanes: 1,
            })
        );
        assert!(
            elapsed < std::time::Duration::from_millis(10),
            "refusing took {elapsed:?}"
        );
    }

    #[test]
    fn an_agreed_expensive_cost_is_paid_by_open_confirmed() {
        // The cheapest cost that asks: just over four times the default's
        // memory, at one pass, so this derives about twice the default's
        // work rather than four times it.
        let expensive = cost(4 * 19 * 1024 + 1, 1);
        let sealed = seal(
            b"strong",
            PASS,
            SealOptions {
                kdf: expensive,
                ..SealOptions::default()
            },
        )
        .unwrap();

        assert!(matches!(
            unlock_cost(&sealed),
            Ok(UnlockCost::NeedsConfirmation { .. })
        ));
        assert!(matches!(
            open(&sealed, PASS),
            Err(CryptoError::UnconfirmedCost { .. })
        ));
        assert_eq!(open_confirmed(&sealed, PASS).unwrap().as_slice(), b"strong");
    }

    #[test]
    fn sealing_never_writes_a_cost_the_ceiling_refuses() {
        // A writer that produces a file its own reader refuses is a defect
        // that arrives a day late.
        for past in [cost(MAX_MEMORY_KIB + 1, 1), cost(8, MAX_ITERATIONS + 1)] {
            assert!(matches!(
                seal(
                    b"x",
                    PASS,
                    SealOptions {
                        kdf: past,
                        ..SealOptions::default()
                    }
                ),
                Err(CryptoError::UnreasonableCost { .. })
            ));
        }
    }

    #[test]
    fn the_default_and_the_published_stronger_presets_are_within_the_ceiling() {
        // OWASP's equivalent set of Argon2id costs all sit at about the
        // default's work, so none of them asks.
        for owasp in [
            cost(46 * 1024, 1),
            cost(19 * 1024, 2),
            cost(12 * 1024, 3),
            cost(9 * 1024, 4),
            cost(7 * 1024, 5),
        ] {
            assert_eq!(owasp.unlock_cost(), UnlockCost::Normal, "{owasp:?}");
        }
        // RFC 9106's low-memory recommendation is within the ceiling, so it
        // can be written and read, and asks first: it is five times the
        // default's work. Its first recommendation, 2 GiB, is past the
        // ceiling and cannot be written at all.
        let rfc_low_memory = KdfParams {
            memory_kib: 64 * 1024,
            iterations: 3,
            lanes: 4,
        };
        assert!(rfc_low_memory.validate().is_ok());
        assert!(matches!(
            rfc_low_memory.unlock_cost(),
            UnlockCost::NeedsConfirmation { .. }
        ));
        assert!(
            KdfParams {
                memory_kib: 2 * 1024 * 1024,
                iterations: 1,
                lanes: 4,
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn the_declared_cost_is_read_without_being_judged() {
        let absurd = cost(1024 * 1024, 64);
        let document = declaring(absurd);

        assert_eq!(declared_cost(&document), Ok(absurd));
        assert!(matches!(unlock_cost(&document), Ok(UnlockCost::Refused(_))));
        assert_eq!(unlock_cost(b"plain text"), Err(CryptoError::NotBpadx));
    }

    proptest::proptest! {
        #[test]
        fn what_sealing_accepts_is_exactly_what_the_reader_does_not_refuse(
            memory_kib in 0u32..(MAX_MEMORY_KIB * 2),
            iterations in 0u32..(MAX_ITERATIONS * 2),
            lanes in 0u32..(MAX_LANES * 2),
        ) {
            // `seal` checks the cost with `validate`; the reader classifies
            // with `unlock_cost`. If they could disagree, a writer could
            // produce a file its own reader refuses.
            let kdf = KdfParams { memory_kib, iterations, lanes };
            proptest::prop_assert_eq!(
                kdf.validate().is_ok(),
                !matches!(kdf.unlock_cost(), UnlockCost::Refused(_))
            );
        }
    }

    #[test]
    fn every_byte_of_a_document_is_load_bearing() {
        // The property behind all the targeted tests above: there is no byte
        // that can be changed without the document refusing to open. If one
        // existed it would be a byte outside the authenticated data, which is
        // exactly where a format grows a hole.
        let sealed = seal(b"hello", PASS, options(Suite::default())).unwrap();

        for index in 0..sealed.len() {
            let mut altered = sealed.clone();
            altered[index] ^= 0xff;
            assert!(
                open(&altered, PASS).is_err(),
                "flipping byte {index} of {} still opened the document",
                sealed.len()
            );
        }
    }
}
