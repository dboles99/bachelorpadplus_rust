//! The security profile each document was given, remembered between sessions.
//!
//! A user marks a document Confidential, closes it, and opens it again next
//! week. Without this module it comes back Standard: the recovery journal is
//! written in plaintext again, clipboard history is kept again, and nothing
//! on screen says the protection has gone. ADR-0020 says a security control
//! that quietly weakens itself is worse than an absent one, because the user
//! has been told it is on. This store is what stops the profile lapsing.
//!
//! # What is written down
//!
//! One small JSON file, `{ "version": 1, "documents": { <key>: <Security> } }`,
//! where the key is a digest of the document's canonical path and the value
//! is `bp_security::Security` exactly as it serialises. Nothing else: no
//! path, no file name, no timestamp, no count of how often it was opened.
//!
//! * **Only settings other than Standard are stored.** Standard is the
//!   *absence* of an entry, so setting a document back to Standard deletes
//!   its entry rather than recording "Standard" -- a list of documents that
//!   were once protected is itself something worth not keeping. A
//!   consequence worth knowing: absence means "whatever the default profile
//!   is", so under a stricter default (`default_profile` in
//!   `config.toml`) an explicit Standard does not survive a restart and the
//!   document comes back at the default. That errs towards strictness,
//!   which is the direction to err in.
//! * **Custom policies are stored too, axis by axis.** `Security` already
//!   serialises a `Custom` policy in full, so scoping this to named profiles
//!   would have saved nothing and silently dropped the one setting a user
//!   had to build by hand. The price is forward compatibility: a policy
//!   written by a later build with an eighth axis will not parse here. Such
//!   an entry is kept verbatim and written back untouched, so an older build
//!   never destroys what a newer one wrote, and in the meantime it reads as
//!   [`Profile::Maximum`] -- the one profile at least as strict as any policy
//!   on every axis. An entry that cannot be read must not become Standard.
//!
//! # Why a digest, and what it does not hide
//!
//! The point of a Confidential profile is to write down as little as possible
//! about the document, so the store must not become a list of confidential
//! file names. Each entry is therefore keyed by the same digest the sealed
//! recovery journal uses for its file names (ADR-0022,
//! `bp_crypto::stable_name`: SHA-256, truncated to 128 bits), taken over the
//! document's canonical absolute path.
//!
//! **The digest is not a secret, and nothing here treats it as one.** It is a
//! deterministic function of the path, with no key and no salt, so anybody
//! holding this file can *confirm a guess*: hash `/home/ana/Payroll 2026.xlsx`
//! and look for it. Paths are very guessable -- home directory, `Documents`,
//! a date-stamped name -- so against someone who already suspects which file
//! matters, this hides nothing. What it does do is stop the file reading as a
//! list: someone who opens it learns how many documents carry a non-default
//! profile and which profile each has, but not which documents they are
//! unless they can guess the paths.
//!
//! A salt would not change that. It would have to be stored beside the
//! digests to be usable, and anyone holding the file holds the salt; it
//! would defeat only tables precomputed across many machines, at the cost of
//! a second thing whose loss forgets every entry at once.
//!
//! # Metadata::Disabled, and why the entry is written anyway
//!
//! Confidential and Maximum both say `metadata: Disabled` -- "nothing about
//! the document is recorded". An entry here is, strictly, something recorded
//! about the document. It is written anyway, deliberately:
//!
//! * the entry is what makes the profile stick. Refusing to write it would
//!   honour the letter of the metadata axis by breaking every other axis on
//!   the next open: the document would come back Standard, with a plaintext
//!   journal and a clipboard history, and nothing would say so. That is the
//!   silent lapse ADR-0020 forbids, and it is strictly worse for the user
//!   than a digest in the state directory;
//! * the metadata axis is defined by its reader, `bp-storage`, and governs
//!   what is recorded about a document's *contents* -- titles, summaries, an
//!   index of what the user writes about. This entry records the user's
//!   instruction about how to treat the document, not anything derived from
//!   it;
//! * what is written is the minimum that can do the job: a digest and the
//!   setting, with no name, no time and no history, and it is deleted
//!   outright when the document goes back to Standard.
//!
//! The trade-off is stated rather than hidden: someone holding this file who
//! can guess a document's path learns that it was marked Confidential. The
//! same reasoning covers Privacy Mode, which clamps `metadata` to Disabled
//! for the session: the store records the document's own setting, never the
//! clamped policy, because Privacy Mode is a session override and must not
//! be written into a document's future.
//!
//! # Paths, and moving documents
//!
//! The key is taken over the canonical absolute path, so a relative spelling,
//! a spelling through `.` or `..`, and one through a symbolic link all name
//! the same entry. A document that does not exist yet -- the target of a
//! Save As, looked up before it is written -- canonicalises its deepest
//! existing ancestor and appends the rest, so it gets the key it will have
//! once it exists. On Windows the canonical form is the verbatim `\\?\` path
//! with the case the filesystem reports for the parts that exist; a part that
//! does not exist yet keeps the case it was typed in, which is the case the
//! file will be created with.
//!
//! **The entry follows the path, not the document.** Like the sealed journal
//! (ADR-0022), there is no stable identifier the filesystem provides across
//! renames, so:
//!
//! * a Save As writes the tab's setting for the new path with
//!   [`ProfileStore::set`], and leaves the old path's entry alone -- the old
//!   file is still there, holding the same text, and still deserves its
//!   profile;
//! * a move, where the old file is gone, uses [`ProfileStore::rename`];
//! * a document moved or deleted outside the editor leaves its entry behind.
//!   It is never pruned, because a digest cannot be turned back into a path
//!   to check. It is small, and it says nothing a guess could not.
//!
//! # Failure
//!
//! A missing file is an empty store. A file that cannot be read or parsed is
//! also an empty store, plus a notice for the shell to show: opening a
//! document must never fail because this file is damaged, and an empty store
//! means every document opens at the default profile until set again -- which
//! is what the notice says. A file written by a newer format version is read
//! as best it can be and **never overwritten**: [`ProfileStore::save`] refuses
//! rather than destroy entries this build cannot understand.
//!
//! Writes are atomic: a uniquely named temporary is created beside the file
//! (`create_new`, so it can never be an existing file or a planted symbolic
//! link that points somewhere else), flushed to disk and renamed over the
//! target. The rename replaces a symbolic link at the target rather than
//! writing through it, so a save never lands outside the directory it was
//! given. Several copies of the program share one file, so a save merges this
//! store's changes onto what is on disk at that moment rather than replacing
//! it with this copy's view.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{OpenOptions, TryLockError};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use bp_security::{Profile, Security};
use serde_json::{Map, Value};

/// The format this build reads and writes.
///
/// Bumped only for a change an older build would misread. Adding an axis to
/// `Policy` is not one: an older build keeps such an entry verbatim.
pub const FORMAT_VERSION: u64 = 1;

/// The key an entry is filed under: a digest of a canonical path.
///
/// Public so the shell and tests can see two spellings map to one key,
/// without ever seeing the path it came from written down.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentKey(String);

impl DocumentKey {
    /// The key for `path`, resolved against the process's current directory
    /// when it is relative.
    ///
    /// Never fails: a path that cannot be canonicalised at all -- no current
    /// directory, nothing on it exists -- is keyed on its lexically cleaned
    /// absolute form, which is still stable for as long as the path is.
    #[must_use]
    pub fn of(path: &Path) -> Self {
        let base = std::env::current_dir().unwrap_or_default();
        Self::of_in(path, &base)
    }

    /// [`of`](Self::of) with the base for a relative path given, so a test
    /// can exercise relative spellings without changing the process's
    /// current directory under every other test running beside it.
    #[must_use]
    pub fn of_in(path: &Path, base: &Path) -> Self {
        let canonical = canonical(path, base);
        // The platform's own bytes rather than a lossy string: two distinct
        // non-UTF-8 paths must not share an entry because both lost the same
        // byte to U+FFFD. The encoding differs between platforms, and that
        // does not matter -- this file is per-machine state and never roams.
        Self(bp_crypto::stable_name(
            canonical.as_os_str().as_encoded_bytes(),
        ))
    }

    /// The digest as it is written in the file.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The canonical absolute form of `path`, whether or not it exists.
///
/// Walks up to the deepest ancestor the filesystem can canonicalise, then
/// appends what was left over. The leftover part does not exist, so it holds
/// no symbolic links and can be cleaned lexically without changing what it
/// names.
fn canonical(path: &Path, base: &Path) -> PathBuf {
    // `join` keeps an absolute `path` as it is, and on Windows gives a
    // root-relative `\x` the drive of `base`.
    let absolute = base.join(path);

    let mut existing = absolute.as_path();
    let mut rest: Vec<OsString> = Vec::new();
    loop {
        if let Ok(resolved) = std::fs::canonicalize(existing) {
            return push_cleaned(resolved, rest.iter().rev());
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_owned());
                existing = parent;
            }
            // `file_name` is `None` for a path ending in `..`; take that
            // step lexically rather than giving up on the whole path.
            (Some(parent), None) => {
                rest.push(OsString::from(".."));
                existing = parent;
            }
            (None, _) => break,
        }
    }
    // Nothing on the path exists, not even its root. Clean it lexically so
    // at least the spellings that differ only in `.` and `..` agree.
    push_cleaned(PathBuf::new(), std::iter::once(&absolute.into_os_string()))
}

/// Append `parts` to `onto`, dropping `.` and resolving `..` lexically.
fn push_cleaned<'a>(mut onto: PathBuf, parts: impl Iterator<Item = &'a OsString>) -> PathBuf {
    for part in parts {
        for component in Path::new(part).components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    onto.pop();
                }
                other => onto.push(other.as_os_str()),
            }
        }
    }
    onto
}

/// What this copy of the program has changed since it last read the file.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Change {
    Set(Security),
    Remove,
    /// An entry this build cannot read, moved here by `rename`. Replayed as
    /// a value rather than parsed, so a newer build's policy survives.
    Keep(Value),
}

/// What a file on disk turned out to hold.
enum OnDisk {
    /// Nothing there. Not worth a notice.
    Absent,
    /// A file this build understands, perhaps with entries it could not
    /// read, which are carried as they were.
    Read {
        entries: BTreeMap<DocumentKey, Security>,
        kept: BTreeMap<DocumentKey, Value>,
    },
    /// A later format. What could be salvaged is carried, and the file is
    /// never written over.
    Newer {
        version: u64,
        kept: BTreeMap<DocumentKey, Value>,
    },
    /// Could not be read or parsed at all.
    Damaged(String),
}

/// Each document's security setting, as far as this machine remembers it.
///
/// Held by the shell for the session: [`load`](Self::load) at startup,
/// [`get`](Self::get) on every open, [`set`](Self::set) and
/// [`save`](Self::save) whenever the user changes a profile. The file is
/// small enough that saving on every change costs nothing worth measuring.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileStore {
    entries: BTreeMap<DocumentKey, Security>,
    /// Entries that did not parse as a `Security` -- most likely a policy
    /// from a newer build. Written back untouched; read as Maximum.
    kept: BTreeMap<DocumentKey, Value>,
    /// Changes not yet saved, merged onto the file as it is at save time.
    changes: BTreeMap<DocumentKey, Change>,
}

impl ProfileStore {
    /// An empty store, for when there is nowhere to keep one. Settings made
    /// in it last for the session.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the store at `path`.
    ///
    /// Never fails. The second value is a notice for the status bar when the
    /// file was there but could not be used in full; a missing file is the
    /// ordinary first run and has no notice.
    #[must_use]
    pub fn load(path: &Path) -> (Self, Option<String>) {
        match read(path) {
            OnDisk::Absent => (Self::new(), None),
            OnDisk::Read { entries, kept } => {
                let notice = (!kept.is_empty()).then(|| {
                    format!(
                        "{} saved document profile{} could not be read and will be treated \
                         as Maximum until set again",
                        kept.len(),
                        if kept.len() == 1 { "" } else { "s" },
                    )
                });
                let store = Self {
                    entries,
                    kept,
                    changes: BTreeMap::new(),
                };
                (store, notice)
            }
            OnDisk::Newer { version, kept } => {
                let notice = format!(
                    "saved document profiles were written by a newer version (format \
                     {version}); documents that have one open as Maximum, and changes made \
                     now will not be saved"
                );
                let store = Self {
                    entries: BTreeMap::new(),
                    kept,
                    changes: BTreeMap::new(),
                };
                (store, Some(notice))
            }
            OnDisk::Damaged(why) => (
                Self::new(),
                Some(format!(
                    "saved document profiles could not be read ({why}); documents open with \
                     the default profile until one is set again"
                )),
            ),
        }
    }

    /// The saved setting for the document at `document`, if it has one.
    ///
    /// `None` means the default profile applies. An entry this build cannot
    /// read answers [`Profile::Maximum`] rather than `None`: it was *some*
    /// setting other than Standard, and Maximum is the only answer certain to
    /// be at least as strict as whatever it was.
    #[must_use]
    pub fn get(&self, document: &Path) -> Option<Security> {
        self.get_key(&DocumentKey::of(document))
    }

    /// [`get`](Self::get) by key.
    #[must_use]
    pub fn get_key(&self, key: &DocumentKey) -> Option<Security> {
        if let Some(security) = self.entries.get(key) {
            return Some(*security);
        }
        self.kept
            .contains_key(key)
            .then_some(Security::Named(Profile::Maximum))
    }

    /// Remember `security` for the document at `document`.
    ///
    /// Standard removes the entry: Standard is the absence of one. A `Custom`
    /// policy is stored even when its axes happen to equal Standard's,
    /// because the user chose "Custom" and should see it again.
    pub fn set(&mut self, document: &Path, security: Security) {
        self.set_key(DocumentKey::of(document), security);
    }

    /// [`set`](Self::set) by key.
    pub fn set_key(&mut self, key: DocumentKey, security: Security) {
        if security == Security::Named(Profile::Standard) {
            self.remove_key(key);
            return;
        }
        self.kept.remove(&key);
        self.entries.insert(key.clone(), security);
        self.changes.insert(key, Change::Set(security));
    }

    /// Forget the document at `document`, so it opens at the default.
    pub fn remove(&mut self, document: &Path) {
        self.remove_key(DocumentKey::of(document));
    }

    /// [`remove`](Self::remove) by key.
    pub fn remove_key(&mut self, key: DocumentKey) {
        self.entries.remove(&key);
        self.kept.remove(&key);
        self.changes.insert(key, Change::Remove);
    }

    /// The document at `from` is now at `to`, and `from` is gone.
    ///
    /// For a move, not a Save As: after a Save As the old file still exists
    /// with the same text, and removing its entry would reopen it at Standard.
    /// A Save As calls [`set`](Self::set) on the new path instead.
    ///
    /// When `from` has no entry, whatever `to` had is removed too: the file
    /// now at `to` is the one that was at `from`, at the default profile.
    /// An unreadable entry at `from` moves as it is.
    pub fn rename(&mut self, from: &Path, to: &Path) {
        let (from, to) = (DocumentKey::of(from), DocumentKey::of(to));
        if from == to {
            return;
        }
        let security = self.entries.get(&from).copied();
        let kept = self.kept.get(&from).cloned();
        self.remove_key(from);
        match (security, kept) {
            (Some(security), _) => self.set_key(to, security),
            (None, Some(value)) => {
                // Carried verbatim, so a newer build's policy survives the
                // move, and recorded as a change of its own so the merge at
                // save time replaces whatever `to` had on disk.
                self.entries.remove(&to);
                self.kept.insert(to.clone(), value.clone());
                self.changes.insert(to, Change::Keep(value));
            }
            (None, None) => self.remove_key(to),
        }
    }

    /// How many documents have a saved setting.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len() + self.kept.len()
    }

    /// Whether no document has a saved setting.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Write the store to `path`, merging with whatever is there now.
    ///
    /// Another copy of the program may have saved since this one loaded, so
    /// the file is read again and only this copy's changes are applied to
    /// it. Two copies changing *different* documents both keep their change;
    /// the same document, the later save wins, which is what the user did
    /// last.
    ///
    /// # Errors
    ///
    /// Fails, writing nothing, when the file on disk is from a newer format
    /// -- overwriting it would destroy entries this build cannot read -- and
    /// when the directory or the file cannot be written. A failure is a
    /// status-bar line for the shell, never a reason to refuse the profile:
    /// it still applies for the session.
    pub fn save(&mut self, path: &Path) -> io::Result<()> {
        let Some(directory) = path.parent() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a document profile store needs a parent directory",
            ));
        };
        if !directory.as_os_str().is_empty() {
            std::fs::create_dir_all(directory)?;
        }
        let lock_path = path.with_extension("lock");
        let mut lock_options = OpenOptions::new();
        lock_options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            lock_options.mode(0o600);
        }
        let lock = lock_options.open(lock_path)?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "another process is saving document profiles",
                ));
            }
            Err(TryLockError::Error(e)) => return Err(e),
        }
        let (mut entries, mut kept) = match read(path) {
            OnDisk::Absent => (BTreeMap::new(), BTreeMap::new()),
            OnDisk::Read { entries, kept } => (entries, kept),
            OnDisk::Newer { version, .. } => {
                return Err(io::Error::other(format!(
                    "document profiles were saved by a newer version (format {version}) \
                     and are left untouched"
                )));
            }
            // Already reported at load. What this copy holds is the best
            // account there is, and it is not worth keeping a damaged file
            // about protected documents around for its own sake.
            OnDisk::Damaged(_) => (self.entries.clone(), self.kept.clone()),
        };

        // Replayed onto the file as it is now, not onto this copy's view of
        // it: an entry another copy removed since this one loaded stays
        // removed unless this copy changed it too.
        for (key, change) in &self.changes {
            match change {
                Change::Set(security) => {
                    kept.remove(key);
                    entries.insert(key.clone(), *security);
                }
                Change::Remove => {
                    kept.remove(key);
                    entries.remove(key);
                }
                Change::Keep(value) => {
                    entries.remove(key);
                    kept.insert(key.clone(), value.clone());
                }
            }
        }

        write_atomically(path, &render(&entries, &kept)?)?;
        self.entries = entries;
        self.kept = kept;
        self.changes.clear();
        Ok(())
    }
}

/// Read and classify the file at `path`.
fn read(path: &Path) -> OnDisk {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return OnDisk::Absent,
        Err(e) => return OnDisk::Damaged(e.to_string()),
    };
    parse(&text)
}

/// Classify the text of a store file.
fn parse(text: &str) -> OnDisk {
    let value: Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(e) => return OnDisk::Damaged(format!("not valid JSON: {e}")),
    };
    let Some(version) = value.get("version").and_then(Value::as_u64) else {
        return OnDisk::Damaged("no format version".to_owned());
    };
    let documents = match value.get("documents") {
        None => Map::new(),
        Some(Value::Object(documents)) => documents.clone(),
        Some(_) => return OnDisk::Damaged("documents is not a map".to_owned()),
    };

    if version > FORMAT_VERSION {
        let kept = documents
            .into_iter()
            .map(|(key, value)| (DocumentKey(key), value))
            .collect();
        return OnDisk::Newer { version, kept };
    }

    let mut entries = BTreeMap::new();
    let mut kept = BTreeMap::new();
    for (key, value) in documents {
        match serde_json::from_value::<Security>(value.clone()) {
            // A Standard entry is not one this build writes. Read as what it
            // says, and dropped at the next save by the same rule as `set`.
            Ok(Security::Named(Profile::Standard)) => {}
            Ok(security) => {
                entries.insert(DocumentKey(key), security);
            }
            Err(_) => {
                kept.insert(DocumentKey(key), value);
            }
        }
    }
    OnDisk::Read { entries, kept }
}

/// The file's text: pretty-printed, keys sorted, so it can be read by eye
/// and compared between two saves.
fn render(
    entries: &BTreeMap<DocumentKey, Security>,
    kept: &BTreeMap<DocumentKey, Value>,
) -> io::Result<String> {
    let mut sorted: BTreeMap<&str, Value> = BTreeMap::new();
    for (key, security) in entries {
        sorted.insert(key.as_str(), serde_json::to_value(security)?);
    }
    for (key, value) in kept {
        sorted.entry(key.as_str()).or_insert_with(|| value.clone());
    }
    // Built by hand rather than with `json!`, whose expansion unwraps. The
    // map keeps insertion order (the workspace enables `preserve_order`), so
    // inserting from the sorted map is what sorts the file.
    let documents: Map<String, Value> = sorted
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect();
    let mut file = Map::new();
    file.insert("version".to_owned(), Value::from(FORMAT_VERSION));
    file.insert("documents".to_owned(), Value::Object(documents));
    let mut text = serde_json::to_string_pretty(&Value::Object(file))?;
    text.push('\n');
    Ok(text)
}

/// Replace `path` with `text` so that a reader sees the old file or the new
/// one, never half of either.
fn write_atomically(path: &Path, text: &str) -> io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);

    let (Some(directory), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a document profile store needs a file name inside a directory",
        ));
    };
    if !directory.as_os_str().is_empty() {
        std::fs::create_dir_all(directory)?;
    }

    // Beside the target, so the rename never crosses a volume, and unique to
    // this process and call, so two copies saving at once never share one.
    let mut temporary = OsString::from(".");
    temporary.push(name);
    temporary.push(format!(
        ".{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let temporary = directory.join(temporary);

    let written = (|| {
        let mut file = create_private(&temporary)?;
        file.write_all(text.as_bytes())?;
        // Flushed before the rename, or a crash just after it could leave
        // the new name pointing at a file the disk never received.
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    written
}

/// Create `path` afresh, readable by its owner only where that can be said.
///
/// `create_new` refuses anything already there, a symbolic link included,
/// so the temporary can never be a link planted to send the write elsewhere.
fn create_private(path: &Path) -> io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    // The file says which (guessable) documents are protected, so it is kept
    // from other users of the machine even under a permissive umask.
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use bp_security::{
        Clipboard, Embeddings, Metadata, Network, Policy, Recovery, TemporaryFiles, Zeroise,
    };
    use tempfile::tempdir;

    use super::*;

    const CONFIDENTIAL: Security = Security::Named(Profile::Confidential);

    fn odd_policy() -> Policy {
        Policy {
            recovery: Recovery::Disabled,
            clipboard: Clipboard::Persistent,
            metadata: Metadata::PathOnly,
            embeddings: Embeddings::None,
            network: Network::Denied,
            temporary_files: TemporaryFiles::Allowed,
            zeroise: Zeroise::On,
        }
    }

    fn names_in(dir: &Path) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn a_confidential_profile_is_there_after_a_restart() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("state").join("document-profiles.json");
        let document = dir.path().join("Report.txt");
        fs::write(&document, b"text").unwrap();

        let (mut store, notice) = ProfileStore::load(&store_path);
        assert_eq!(notice, None, "a first run is not worth a notice");
        store.set(&document, CONFIDENTIAL);
        store.save(&store_path).unwrap();

        let (reloaded, notice) = ProfileStore::load(&store_path);
        assert_eq!(notice, None);
        assert_eq!(reloaded.get(&document), Some(CONFIDENTIAL));
    }

    #[test]
    fn a_custom_policy_survives_a_restart_axis_by_axis() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("profiles.json");
        let document = dir.path().join("custom.txt");

        let mut store = ProfileStore::new();
        store.set(&document, Security::Custom(odd_policy()));
        store.save(&store_path).unwrap();

        assert_eq!(
            ProfileStore::load(&store_path).0.get(&document),
            Some(Security::Custom(odd_policy()))
        );
    }

    #[test]
    fn a_profile_that_forbids_metadata_is_still_written_down() {
        // The decision in the module docs, pinned: without the entry the
        // profile lapses to Standard on the next open, which is worse than
        // the digest the entry costs.
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("profiles.json");
        let document = dir.path().join("max.txt");
        assert_eq!(Profile::Maximum.policy().metadata, Metadata::Disabled);

        let mut store = ProfileStore::new();
        store.set(&document, Security::Named(Profile::Maximum));
        store.save(&store_path).unwrap();

        assert_eq!(
            ProfileStore::load(&store_path).0.get(&document),
            Some(Security::Named(Profile::Maximum))
        );
    }

    #[test]
    fn setting_standard_removes_the_entry_rather_than_recording_it() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("profiles.json");
        let document = dir.path().join("a.txt");

        let mut store = ProfileStore::new();
        store.set(&document, CONFIDENTIAL);
        store.save(&store_path).unwrap();
        store.set(&document, Security::Named(Profile::Standard));
        store.save(&store_path).unwrap();

        let (reloaded, _) = ProfileStore::load(&store_path);
        assert_eq!(reloaded.get(&document), None);
        assert!(reloaded.is_empty());
        let text = fs::read_to_string(&store_path).unwrap();
        assert!(!text.contains("Standard"), "{text}");
        assert!(
            !text.contains(DocumentKey::of(&document).as_str()),
            "a document once protected must leave no trace once it is not: {text}"
        );
    }

    #[test]
    fn the_file_contains_no_part_of_the_documents_path() {
        let dir = tempdir().unwrap();
        let folder = dir.path().join("Acquisition Plans");
        fs::create_dir_all(&folder).unwrap();
        let document = folder.join("Project_Nightingale_Board_Memo.txt");
        let store_path = dir.path().join("profiles.json");

        let mut store = ProfileStore::new();
        store.set(&document, CONFIDENTIAL);
        store.save(&store_path).unwrap();

        let text = fs::read_to_string(&store_path).unwrap();
        for fragment in [
            "Nightingale",
            "Board_Memo",
            "Acquisition",
            ".txt",
            dir.path().file_name().unwrap().to_str().unwrap(),
        ] {
            assert!(!text.contains(fragment), "{fragment:?} leaked into {text}");
        }
        assert!(text.contains("\"version\": 1"), "{text}");
        assert!(text.contains("Confidential"), "{text}");
    }

    #[test]
    fn a_corrupt_file_is_an_empty_store_and_a_notice() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("profiles.json");
        for damage in ["{ half writ", "", "[1, 2, 3]", "{\"documents\": {}}"] {
            fs::write(&store_path, damage).unwrap();
            let (store, notice) = ProfileStore::load(&store_path);
            assert!(store.is_empty(), "{damage:?}");
            let notice = notice.unwrap_or_default();
            assert!(
                notice.contains("default profile"),
                "{damage:?} gave {notice:?}"
            );
        }
    }

    #[test]
    fn a_corrupt_file_is_replaced_by_the_next_save() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("profiles.json");
        let document = dir.path().join("a.txt");
        fs::write(&store_path, "not json at all").unwrap();

        let (mut store, _) = ProfileStore::load(&store_path);
        store.set(&document, CONFIDENTIAL);
        store.save(&store_path).unwrap();

        let (reloaded, notice) = ProfileStore::load(&store_path);
        assert_eq!(notice, None);
        assert_eq!(reloaded.get(&document), Some(CONFIDENTIAL));
    }

    #[test]
    fn a_missing_file_is_an_empty_store_without_a_notice() {
        let dir = tempdir().unwrap();
        let (store, notice) = ProfileStore::load(&dir.path().join("absent.json"));
        assert!(store.is_empty());
        assert_eq!(notice, None);
    }

    #[test]
    fn two_different_documents_do_not_collide() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("profiles.json");
        let one = dir.path().join("one.txt");
        let two = dir.path().join("two.txt");
        assert_ne!(DocumentKey::of(&one), DocumentKey::of(&two));

        let mut store = ProfileStore::new();
        store.set(&one, CONFIDENTIAL);
        store.set(&two, Security::Named(Profile::Private));
        store.save(&store_path).unwrap();

        let (reloaded, _) = ProfileStore::load(&store_path);
        assert_eq!(reloaded.get(&one), Some(CONFIDENTIAL));
        assert_eq!(reloaded.get(&two), Some(Security::Named(Profile::Private)));
        assert_eq!(reloaded.get(&dir.path().join("three.txt")), None);
    }

    #[test]
    fn a_relative_and_an_absolute_spelling_of_one_file_share_an_entry() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir_all(&sub).unwrap();
        let absolute = dir.path().join("notes.txt");
        fs::write(&absolute, b"x").unwrap();

        let key = DocumentKey::of(&absolute);
        assert_eq!(DocumentKey::of_in(Path::new("notes.txt"), dir.path()), key);
        assert_eq!(
            DocumentKey::of_in(Path::new("./sub/../notes.txt"), dir.path()),
            key
        );
        assert_eq!(DocumentKey::of_in(Path::new("../notes.txt"), &sub), key);
    }

    #[cfg(unix)]
    #[test]
    fn a_spelling_through_a_symbolic_link_shares_the_entry_too() {
        let dir = tempdir().unwrap();
        let real = dir.path().join("real");
        fs::create_dir_all(&real).unwrap();
        fs::write(real.join("doc.txt"), b"x").unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        assert_eq!(
            DocumentKey::of(&link.join("doc.txt")),
            DocumentKey::of(&real.join("doc.txt"))
        );
    }

    #[test]
    fn a_document_that_does_not_exist_yet_keeps_its_key_once_it_does() {
        // The Save As case: a lookup or a `set` before the file is written
        // must name the entry the file will have afterwards -- including
        // when the temp directory is itself reached through a link, as it is
        // on macOS.
        let dir = tempdir().unwrap();
        let document = dir.path().join("later").join("new.txt");
        let before = DocumentKey::of(&document);
        assert_eq!(
            DocumentKey::of(&dir.path().join("later/./x/../new.txt")),
            before
        );

        fs::create_dir_all(document.parent().unwrap()).unwrap();
        fs::write(&document, b"x").unwrap();

        assert_eq!(DocumentKey::of(&document), before);
    }

    #[test]
    fn renaming_carries_the_entry_to_the_new_path() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("profiles.json");
        let from = dir.path().join("draft.txt");
        let to = dir.path().join("final.txt");

        let mut store = ProfileStore::new();
        store.set(&from, CONFIDENTIAL);
        store.save(&store_path).unwrap();
        store.rename(&from, &to);
        store.save(&store_path).unwrap();

        let (reloaded, _) = ProfileStore::load(&store_path);
        assert_eq!(reloaded.get(&to), Some(CONFIDENTIAL));
        assert_eq!(reloaded.get(&from), None, "the old path no longer holds it");
    }

    #[test]
    fn renaming_an_unprotected_document_over_a_protected_path_clears_it() {
        // The file now at `to` is the one that was at `from`, so it takes
        // `from`'s setting -- which here is the default.
        let dir = tempdir().unwrap();
        let from = dir.path().join("plain.txt");
        let to = dir.path().join("secret.txt");

        let mut store = ProfileStore::new();
        store.set(&to, CONFIDENTIAL);
        store.rename(&from, &to);

        assert_eq!(store.get(&to), None);
    }

    #[test]
    fn saving_leaves_no_temporary_file_behind() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("profiles.json");
        fs::write(&store_path, "{\"version\": 1, \"documents\": {}}").unwrap();

        let mut store = ProfileStore::new();
        store.set(&dir.path().join("a.txt"), CONFIDENTIAL);
        store.save(&store_path).unwrap();
        store.save(&store_path).unwrap();

        assert_eq!(names_in(dir.path()), ["profiles.json"]);
    }

    #[cfg(unix)]
    #[test]
    fn saving_over_a_symbolic_link_replaces_the_link_and_never_writes_through_it() {
        let dir = tempdir().unwrap();
        let elsewhere = tempdir().unwrap();
        let victim = elsewhere.path().join("victim");
        fs::write(&victim, b"untouched").unwrap();
        let store_path = dir.path().join("profiles.json");
        std::os::unix::fs::symlink(&victim, &store_path).unwrap();

        let mut store = ProfileStore::new();
        store.set(&dir.path().join("a.txt"), CONFIDENTIAL);
        // The link's target does not parse, so this is the damaged-file path.
        store.save(&store_path).unwrap();

        assert_eq!(fs::read(&victim).unwrap(), b"untouched");
        assert!(!fs::symlink_metadata(&store_path).unwrap().is_symlink());
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_readable_by_its_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("profiles.json");
        let mut store = ProfileStore::new();
        store.set(&dir.path().join("a.txt"), CONFIDENTIAL);
        store.save(&store_path).unwrap();

        let mode = fs::metadata(&store_path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn two_copies_of_the_program_saving_different_documents_both_keep_theirs() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("profiles.json");
        let one = dir.path().join("one.txt");
        let two = dir.path().join("two.txt");

        let (mut first, _) = ProfileStore::load(&store_path);
        let (mut second, _) = ProfileStore::load(&store_path);
        first.set(&one, CONFIDENTIAL);
        first.save(&store_path).unwrap();
        second.set(&two, Security::Named(Profile::Private));
        second.save(&store_path).unwrap();

        let (reloaded, _) = ProfileStore::load(&store_path);
        assert_eq!(reloaded.get(&one), Some(CONFIDENTIAL));
        assert_eq!(reloaded.get(&two), Some(Security::Named(Profile::Private)));
        assert_eq!(
            second.get(&one),
            Some(CONFIDENTIAL),
            "a save adopts the merge"
        );
    }

    #[test]
    fn an_entry_this_build_cannot_read_is_treated_as_maximum_and_kept() {
        // A policy from a later build with an axis this one has never heard
        // of. Reading it as Standard would be the silent lapse; dropping it
        // would destroy what the newer build wrote.
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("profiles.json");
        let document = dir.path().join("future.txt");
        let other = dir.path().join("other.txt");
        let key = DocumentKey::of(&document);
        fs::write(
            &store_path,
            format!(
                "{{\"version\": 1, \"documents\": {{\"{}\": {{\"Custom\": {{\"telepathy\": \"Off\"}}}}}}}}",
                key.as_str()
            ),
        )
        .unwrap();

        let (mut store, notice) = ProfileStore::load(&store_path);
        assert!(notice.unwrap_or_default().contains("Maximum"));
        assert_eq!(
            store.get(&document),
            Some(Security::Named(Profile::Maximum))
        );

        store.set(&other, CONFIDENTIAL);
        store.save(&store_path).unwrap();
        let text = fs::read_to_string(&store_path).unwrap();
        assert!(
            text.contains("telepathy"),
            "the unread entry was lost: {text}"
        );
    }

    #[test]
    fn an_unreadable_entry_moves_with_a_rename_and_replaces_what_was_there() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("profiles.json");
        let from = dir.path().join("future.txt");
        let to = dir.path().join("moved.txt");
        fs::write(
            &store_path,
            format!(
                "{{\"version\": 1, \"documents\": {{\"{}\": {{\"Custom\": {{\"telepathy\": \"Off\"}}}}, \"{}\": {{\"Named\": \"Private\"}}}}}}",
                DocumentKey::of(&from).as_str(),
                DocumentKey::of(&to).as_str(),
            ),
        )
        .unwrap();

        let (mut store, _) = ProfileStore::load(&store_path);
        store.rename(&from, &to);
        store.save(&store_path).unwrap();

        let (reloaded, _) = ProfileStore::load(&store_path);
        assert_eq!(reloaded.get(&from), None);
        assert_eq!(
            reloaded.get(&to),
            Some(Security::Named(Profile::Maximum)),
            "the moved entry, still unreadable, must not be replaced by Private from disk"
        );
        assert!(
            fs::read_to_string(&store_path)
                .unwrap()
                .contains("telepathy")
        );
    }

    #[test]
    fn a_file_from_a_newer_format_is_never_overwritten() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("profiles.json");
        let newer = "{\"version\": 99, \"documents\": {\"abc\": \"whatever\"}}";
        fs::write(&store_path, newer).unwrap();

        let (mut store, notice) = ProfileStore::load(&store_path);
        assert!(notice.unwrap_or_default().contains("newer version"));
        store.set(&dir.path().join("a.txt"), CONFIDENTIAL);

        assert!(store.save(&store_path).is_err());
        assert_eq!(fs::read_to_string(&store_path).unwrap(), newer);
        assert_eq!(names_in(dir.path()), ["profiles.json"]);
    }

    #[test]
    fn the_key_is_the_journal_digest_of_the_canonical_path() {
        // ADR-0022's scheme, reused rather than reinvented: 32 hex digits of
        // SHA-256 via `bp_crypto::stable_name`.
        let dir = tempdir().unwrap();
        let document = dir.path().join("a.txt");
        fs::write(&document, b"x").unwrap();
        let canonical = fs::canonicalize(&document).unwrap();

        let key = DocumentKey::of(&document);
        assert_eq!(
            key.as_str(),
            bp_crypto::stable_name(canonical.as_os_str().as_encoded_bytes())
        );
        assert_eq!(key.as_str().len(), 32);
        assert!(key.as_str().bytes().all(|b| b.is_ascii_hexdigit()));
    }
}
