//! Where the product keeps what is not settings, and how it gets there from
//! where it used to be.
//!
//! Three files and directories used to hang off the configuration directory:
//! the recent-files list, the recovery journal and the security history.
//! On Windows that directory is `%APPDATA%`, which **roams** -- it is copied
//! to every machine the user signs into -- and all three hold machine-specific
//! absolute paths or machine-specific work. `bp-platform` has a name for each
//! of them, and this module is where the product finally asks for it:
//!
//! | What | Kind | Why |
//! | --- | --- | --- |
//! | recent files | [`DirKind::State`] | a per-machine convenience list |
//! | security history | [`DirKind::State`] | a log of this machine's events |
//! | recovery journal | [`DirKind::Data`] | unsaved work -- not recomputable, not disposable |
//!
//! The journal is `Data` rather than `State` because `bp-platform` says so in
//! as many words, and because the XDG specification reserves `$XDG_STATE_HOME`
//! for things "not important or portable enough" for the data directory.
//! Unsaved work is the most important thing this product writes.
//!
//! # Moving is not free, and one of the three holds unsaved work
//!
//! A user upgrading from a build that used the old locations has files there,
//! and the recovery journal may hold the only copy of something they typed.
//! So each old location is **adopted** rather than abandoned: renamed into the
//! new one at startup, as a whole. [`adopt`] carries the rules, and the one
//! that matters is the last -- **when the move fails, the old location is
//! used**, for this session and every one after it until a move succeeds. A
//! product that loses a recovery journal because `%LOCALAPPDATA%` sat on a
//! different volume from `%APPDATA%` has swapped a roaming nuisance for data
//! loss.
//!
//! The paths are computed from an [`EnvSnapshot`], like `config_path_in`, so
//! both platforms' answers are assertable from either leg of CI. Only
//! [`Locations::for_host`] touches the filesystem.

use std::io;
use std::path::{Path, PathBuf};

use bp_platform::dirs::{self, DirKind};

use crate::{EnvSnapshot, Platform};

/// The recovery journal's directory name, under the data directory.
pub const RECOVERY_DIR: &str = "recovery";

/// The security history's file name, under the state directory.
pub const SECURITY_HISTORY_FILE: &str = "security-history.log";

/// A name inside one of the product's directories.
///
/// Joined with the named platform's separator, for the reason
/// `beside_the_config_file` gives: `PathBuf::join` would build the Windows
/// answer with `/` on the Linux leg.
pub(crate) fn inside(
    platform: Platform,
    kind: DirKind,
    env: &EnvSnapshot,
    name: &str,
) -> Option<PathBuf> {
    let directory = dirs::directory(platform, kind, env)?;
    Some(PathBuf::from(bp_platform::paths::join(
        platform,
        directory.to_str()?,
        &[name],
    )))
}

/// Where the recovery journal lives.
#[must_use]
pub fn recovery_dir_in(platform: Platform, env: &EnvSnapshot) -> Option<PathBuf> {
    inside(platform, DirKind::Data, env, RECOVERY_DIR)
}

/// Where the security history lives.
#[must_use]
pub fn security_history_path_in(platform: Platform, env: &EnvSnapshot) -> Option<PathBuf> {
    inside(platform, DirKind::State, env, SECURITY_HISTORY_FILE)
}

/// Where each of the three used to live: beside `config.toml`.
///
/// Kept as a function rather than deleted, because [`adopt`] needs it for as
/// long as anybody may be upgrading from a build that wrote there.
#[must_use]
pub fn legacy_path_in(platform: Platform, env: &EnvSnapshot, name: &str) -> Option<PathBuf> {
    crate::beside_the_config_file(platform, env, name)
}

/// The three locations the shell needs, resolved and adopted.
#[derive(Debug)]
pub struct Locations {
    /// The recent-files list. `None` when the environment names no state
    /// directory, in which case the list is kept in memory only.
    pub recent: Option<PathBuf>,
    /// The recovery journal's directory.
    pub recovery: Option<PathBuf>,
    /// The security history.
    pub security_history: Option<PathBuf>,
    /// Anything [`adopt`] could not move, to be reported rather than lost.
    pub left_behind: Vec<Adoption>,
}

impl Locations {
    /// The paths for `platform` and `env`, without touching the filesystem.
    #[must_use]
    pub fn in_env(platform: Platform, env: &EnvSnapshot) -> Self {
        Self {
            recent: crate::recent::recent_path_in(platform, env),
            recovery: recovery_dir_in(platform, env),
            security_history: security_history_path_in(platform, env),
            left_behind: Vec::new(),
        }
    }

    /// The locations on this machine, with each old one adopted.
    ///
    /// The edge, and the only function here with side effects: it may rename
    /// files and directories. Call it once, at startup, before anything reads
    /// the journal -- never from a test, which should build a `Locations` of
    /// its own under a temporary directory.
    #[must_use]
    pub fn for_host() -> Self {
        let env = EnvSnapshot::from_environment();
        let platform = Platform::HOST;
        let mut locations = Self::in_env(platform, &env);

        for (slot, name) in [
            (&mut locations.recent, crate::recent::FILE_NAME),
            (&mut locations.recovery, RECOVERY_DIR),
            (&mut locations.security_history, SECURITY_HISTORY_FILE),
        ] {
            let (Some(current), Some(legacy)) =
                (slot.as_ref(), legacy_path_in(platform, &env, name))
            else {
                continue;
            };
            let adoption = adopt(&legacy, current);
            *slot = Some(adoption.path_to_use(current).to_path_buf());
            if adoption.needs_reporting() {
                locations.left_behind.push(adoption);
            }
        }
        locations
    }
}

/// What [`adopt`] did.
#[derive(Debug)]
pub enum Adoption {
    /// Nothing was at the old location. The new one is used.
    NothingToMove,
    /// The old location was renamed to the new one.
    Moved,
    /// Both locations hold something. The new one is used and the old one is
    /// left exactly as it was -- merging two recovery journals whose file
    /// names may collide is not something to do unasked.
    BothOccupied {
        /// The location nothing will read any more.
        legacy: PathBuf,
    },
    /// The rename failed. The old location is used, and nothing was lost.
    KeptLegacy {
        /// The location still in use.
        legacy: PathBuf,
        /// Why the move did not happen.
        error: io::Error,
    },
}

impl Adoption {
    /// The path to read and write from now on.
    #[must_use]
    pub fn path_to_use<'a>(&'a self, current: &'a Path) -> &'a Path {
        match self {
            Self::KeptLegacy { legacy, .. } => legacy,
            Self::NothingToMove | Self::Moved | Self::BothOccupied { .. } => current,
        }
    }

    /// Whether a person should hear about this.
    ///
    /// Only the two outcomes where something is not where the product will
    /// look for it next time. A successful move is invisible by design.
    #[must_use]
    pub const fn needs_reporting(&self) -> bool {
        matches!(self, Self::BothOccupied { .. } | Self::KeptLegacy { .. })
    }
}

/// Move whatever is at `legacy` to `current`, if that is safe.
///
/// Works on a file or a directory, and moves it whole with one rename --
/// never file by file, which would leave half a journal in each place if it
/// stopped partway. The rules, in order:
///
/// 1. Nothing at `legacy`: nothing to do.
/// 2. Something at `current` already: use it, and leave `legacy` untouched.
///    An *empty directory* at `current` does not count, and is replaced --
///    the journal creates its directory before it has anything to put in it,
///    so an empty one is what a first launch of this build leaves behind.
/// 3. Otherwise rename. If that fails -- a different volume, a permission, a
///    file held open by another copy of the program -- keep using `legacy`.
///
/// Rule 3 is the reason this returns a path to use rather than a success
/// flag: the caller has no decision to make.
///
/// "Nothing at `legacy`" means the filesystem *said* nothing is there. An
/// old location that cannot be examined -- a permission, an I/O error -- is
/// kept and reported rather than taken as absent: absent would switch to the
/// new location silently, the next launch would find both occupied, and an
/// old journal would never be adopted at all.
pub fn adopt(legacy: &Path, current: &Path) -> Adoption {
    if legacy == current {
        return Adoption::NothingToMove;
    }
    match std::fs::symlink_metadata(legacy) {
        Ok(_) => {}
        Err(error) if is_absence(&error) => return Adoption::NothingToMove,
        Err(error) => {
            return Adoption::KeptLegacy {
                legacy: legacy.to_path_buf(),
                error,
            };
        }
    }
    if is_occupied(current) {
        return Adoption::BothOccupied {
            legacy: legacy.to_path_buf(),
        };
    }

    let moved = (|| {
        if let Some(parent) = current.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if current.is_dir() {
            // Empty, by `is_occupied`; `rename` over an empty directory is
            // allowed on Linux and refused on Windows, so remove it first
            // and let both platforms take the same path.
            std::fs::remove_dir(current)?;
        }
        std::fs::rename(legacy, current)
    })();

    match moved {
        Ok(()) => Adoption::Moved,
        Err(error) => after_a_failed_move(legacy, current, error),
    }
}

/// What a failed rename actually left behind.
///
/// Two copies of the program launched together -- a file association
/// opening several files at once does exactly this -- both see the old
/// journal and both try to move it. One wins. The other's rename fails
/// because its source is gone, and keeping "the old location" would point it
/// at a directory that no longer exists: it would miss the moved checkpoints
/// and start a second journal there that the next launch never reads. So the
/// state is looked at again rather than assumed: the old location gone and
/// the new one occupied means the move happened, just not here.
fn after_a_failed_move(legacy: &Path, current: &Path, error: io::Error) -> Adoption {
    let legacy_gone = matches!(std::fs::symlink_metadata(legacy), Err(e) if is_absence(&e));
    if legacy_gone && is_occupied(current) {
        Adoption::Moved
    } else {
        Adoption::KeptLegacy {
            legacy: legacy.to_path_buf(),
            error,
        }
    }
}

/// Whether an error from examining a path means nothing is there.
///
/// `NotADirectory` counts: a path under something that is a file cannot
/// hold anything either.
fn is_absence(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
    )
}

/// Whether `path` holds anything worth not overwriting.
fn is_occupied(path: &Path) -> bool {
    match std::fs::symlink_metadata(path) {
        Err(_) => false,
        Ok(metadata) if metadata.is_dir() => std::fs::read_dir(path)
            .map(|mut entries| entries.next().is_some())
            // A directory that cannot be listed is not one to replace.
            .unwrap_or(true),
        Ok(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::tests::{text, whole_env};
    use tempfile::tempdir;

    // --- where each thing goes -------------------------------------------

    fn starts_in(platform: Platform, path: Option<PathBuf>, kind: DirKind) -> bool {
        let env = whole_env();
        text(path).starts_with(&text(dirs::directory(platform, kind, &env)))
    }

    #[test]
    fn nothing_the_shell_keeps_per_machine_is_put_in_a_roaming_directory() {
        // The defect this module exists to fix, stated as a property of every
        // location on every platform: none of them may land in a directory
        // `bp-platform` says roams.
        let env = whole_env();
        for &platform in Platform::ALL {
            let locations = Locations::in_env(platform, &env);
            for path in [
                locations.recent,
                locations.recovery,
                locations.security_history,
            ] {
                let path = text(path);
                for &kind in DirKind::ALL {
                    if kind.roams(platform) {
                        let roaming = text(dirs::directory(platform, kind, &env));
                        assert!(
                            !path.starts_with(&roaming),
                            "{platform:?}: {path:?} is inside the roaming {} directory",
                            kind.token()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn each_location_is_in_the_directory_bp_platform_names_for_it() {
        let env = whole_env();
        for &platform in Platform::ALL {
            let locations = Locations::in_env(platform, &env);
            assert!(starts_in(platform, locations.recent, DirKind::State));
            assert!(starts_in(
                platform,
                locations.security_history,
                DirKind::State
            ));
            assert!(
                starts_in(platform, locations.recovery, DirKind::Data),
                "{platform:?}: unsaved work belongs with data, not state"
            );
        }
    }

    #[test]
    fn the_windows_answers_are_the_local_app_data_folders() {
        let env = whole_env();
        let locations = Locations::in_env(Platform::Windows, &env);
        assert!(text(locations.recovery).ends_with(r"\bachelorpad\data\recovery"));
        assert!(text(locations.recent).ends_with(r"\bachelorpad\state\recent.toml"));
        assert!(
            text(locations.security_history).ends_with(r"\bachelorpad\state\security-history.log")
        );
    }

    #[test]
    fn the_legacy_locations_are_beside_the_config_file() {
        // What `adopt` moves *from*. If this drifts, an upgrade silently
        // starts with an empty journal and the old one is never looked at.
        let env = whole_env();
        for &platform in Platform::ALL {
            let config = text(crate::config_path_in(platform, &env));
            let directory = config
                .rsplit_once(platform.separator())
                .expect("a directory")
                .0;
            for name in [
                crate::recent::FILE_NAME,
                RECOVERY_DIR,
                SECURITY_HISTORY_FILE,
            ] {
                let legacy = text(legacy_path_in(platform, &env, name));
                assert_eq!(legacy, format!("{directory}{}{name}", platform.separator()));
            }
        }
    }

    #[test]
    fn an_empty_environment_yields_no_locations_rather_than_guesses() {
        for &platform in Platform::ALL {
            let locations = Locations::in_env(platform, &EnvSnapshot::default());
            assert_eq!(locations.recent, None);
            assert_eq!(locations.recovery, None);
            assert_eq!(locations.security_history, None);
        }
    }

    // --- adopting what an older build left --------------------------------

    /// A recovery directory holding one checkpoint, the shape that matters.
    fn a_journal_in(dir: &Path) -> PathBuf {
        let journal = dir.join("old").join(RECOVERY_DIR);
        fs::create_dir_all(&journal).unwrap();
        fs::write(journal.join("1.checkpoint"), b"unsaved work").unwrap();
        journal
    }

    #[test]
    fn a_journal_moves_whole_and_nothing_is_left_at_the_old_location() {
        let dir = tempdir().unwrap();
        let legacy = a_journal_in(dir.path());
        let current = dir.path().join("new").join("data").join(RECOVERY_DIR);

        let adoption = adopt(&legacy, &current);

        assert!(matches!(adoption, Adoption::Moved), "{adoption:?}");
        assert_eq!(adoption.path_to_use(&current), current);
        assert_eq!(
            fs::read(current.join("1.checkpoint")).unwrap(),
            b"unsaved work"
        );
        assert!(!legacy.exists(), "a move must not leave a second copy");
    }

    #[test]
    fn a_single_file_moves_too() {
        let dir = tempdir().unwrap();
        let legacy = dir.path().join("recent.toml");
        fs::write(&legacy, b"paths = []\n").unwrap();
        let current = dir.path().join("state").join("recent.toml");

        assert!(matches!(adopt(&legacy, &current), Adoption::Moved));
        assert_eq!(fs::read(&current).unwrap(), b"paths = []\n");
    }

    #[test]
    fn nothing_at_the_old_location_is_nothing_to_do() {
        let dir = tempdir().unwrap();
        let current = dir.path().join("new");
        let adoption = adopt(&dir.path().join("absent"), &current);
        assert!(matches!(adoption, Adoption::NothingToMove));
        assert!(!current.exists(), "adopting nothing created something");
    }

    #[test]
    fn an_empty_new_directory_is_replaced_by_the_old_journal() {
        // What a first launch of this build leaves if it ran before the move
        // was attempted: the journal makes its directory eagerly.
        let dir = tempdir().unwrap();
        let legacy = a_journal_in(dir.path());
        let current = dir.path().join("new");
        fs::create_dir_all(&current).unwrap();

        assert!(matches!(adopt(&legacy, &current), Adoption::Moved));
        assert!(current.join("1.checkpoint").exists());
    }

    #[test]
    fn when_both_hold_something_neither_is_touched() {
        let dir = tempdir().unwrap();
        let legacy = a_journal_in(dir.path());
        let current = dir.path().join("new");
        fs::create_dir_all(&current).unwrap();
        fs::write(current.join("1.checkpoint"), b"newer work").unwrap();

        let adoption = adopt(&legacy, &current);

        assert!(
            matches!(&adoption, Adoption::BothOccupied { legacy: l } if *l == legacy),
            "{adoption:?}"
        );
        assert!(
            adoption.needs_reporting(),
            "the old journal must be mentioned"
        );
        assert_eq!(
            fs::read(current.join("1.checkpoint")).unwrap(),
            b"newer work"
        );
        assert_eq!(
            fs::read(legacy.join("1.checkpoint")).unwrap(),
            b"unsaved work",
            "the old journal must survive untouched"
        );
    }

    #[test]
    fn a_move_that_fails_keeps_using_the_old_location_and_loses_nothing() {
        // The rule that makes moving a journal safe at all. A parent that is
        // a *file* stands in for a different volume or a refused permission:
        // the move cannot happen, and the journal must still be found.
        let dir = tempdir().unwrap();
        let legacy = a_journal_in(dir.path());
        let blocker = dir.path().join("not-a-directory");
        fs::write(&blocker, b"").unwrap();
        let current = blocker.join(RECOVERY_DIR);

        let adoption = adopt(&legacy, &current);

        assert!(
            matches!(&adoption, Adoption::KeptLegacy { .. }),
            "{adoption:?}"
        );
        assert_eq!(adoption.path_to_use(&current), legacy);
        assert!(adoption.needs_reporting());
        assert_eq!(
            fs::read(legacy.join("1.checkpoint")).unwrap(),
            b"unsaved work"
        );
    }

    #[test]
    fn an_old_location_that_cannot_be_examined_is_kept_not_taken_as_absent() {
        // A NUL byte makes every platform refuse to look, without needing a
        // permission the test runner (often root) would ignore. What matters
        // is the class: an error that is not "nothing is there".
        let dir = tempdir().unwrap();
        let legacy = PathBuf::from("unreadable\0journal");
        let current = dir.path().join("new");

        let adoption = adopt(&legacy, &current);

        assert!(
            matches!(&adoption, Adoption::KeptLegacy { legacy: l, .. } if *l == legacy),
            "{adoption:?}"
        );
        assert!(adoption.needs_reporting());
        assert!(!current.exists(), "nothing may be created in its place");
    }

    #[test]
    fn a_rename_that_lost_a_race_uses_what_the_winner_moved() {
        // Another copy of the program moved the journal between this one's
        // checks and its rename: the source is gone, the destination holds
        // the checkpoints. The old location must not be chosen.
        let dir = tempdir().unwrap();
        let legacy = dir.path().join("old").join(RECOVERY_DIR);
        let current = dir.path().join("new");
        fs::create_dir_all(&current).unwrap();
        fs::write(current.join("1.checkpoint"), b"moved by the other copy").unwrap();

        let adoption =
            after_a_failed_move(&legacy, &current, io::Error::from(io::ErrorKind::NotFound));

        assert!(matches!(adoption, Adoption::Moved), "{adoption:?}");
        assert_eq!(adoption.path_to_use(&current), current);
    }

    #[test]
    fn a_rename_that_failed_with_the_journal_still_in_place_keeps_it() {
        // The other half of the recheck: the ordinary failure is untouched.
        let dir = tempdir().unwrap();
        let legacy = a_journal_in(dir.path());
        let current = dir.path().join("new");

        let adoption =
            after_a_failed_move(&legacy, &current, io::Error::other("cross-device link"));

        assert!(
            matches!(&adoption, Adoption::KeptLegacy { .. }),
            "{adoption:?}"
        );
        assert_eq!(adoption.path_to_use(&current), legacy);
    }

    #[test]
    fn a_successful_move_is_not_worth_mentioning() {
        assert!(!Adoption::Moved.needs_reporting());
        assert!(!Adoption::NothingToMove.needs_reporting());
    }
}
