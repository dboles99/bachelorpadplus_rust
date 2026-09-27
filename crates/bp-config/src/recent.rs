//! The recently-opened list.
//!
//! State rather than configuration -- the user never edits it by hand -- so
//! it lives in the state directory, not beside `config.toml`, and a corrupt
//! or missing one is simply an empty list. Losing a convenience list is not
//! worth a message, let alone a failure to start.
//!
//! It used to live beside `config.toml`, which on Windows roams, and a list of
//! one machine's absolute paths is exactly what a roaming directory must not
//! carry. [`crate::locations`] moves an old one across.
//!
//! Paths only. This file records *which* documents were opened, never
//! anything about their contents (ADR-0011).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use bp_platform::dirs::DirKind;

use crate::{EnvSnapshot, Platform};

/// How many entries to keep. Long enough to be useful, short enough that the
/// File menu stays a menu.
pub const MAX_RECENT: usize = 10;

pub(crate) const FILE_NAME: &str = "recent.toml";

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recent {
    #[serde(default)]
    paths: Vec<PathBuf>,
}

impl Recent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// Record a path as most-recently used.
    ///
    /// Moves an existing entry to the front rather than duplicating it, so
    /// reopening a file does not push everything else out of the list.
    pub fn push(&mut self, path: &Path) {
        self.paths.retain(|p| p != path);
        self.paths.insert(0, path.to_path_buf());
        self.paths.truncate(MAX_RECENT);
    }

    /// Drop entries that no longer exist.
    ///
    /// Called before display: offering to open a file that was deleted or is
    /// on an unplugged drive is worse than not offering it.
    pub fn prune_missing(&mut self) {
        self.paths.retain(|p| p.exists());
    }

    pub fn parse(text: &str) -> Self {
        // A corrupt list is an empty list. There is nothing here worth
        // interrupting the user over.
        toml::from_str(text).unwrap_or_default()
    }

    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_default()
    }
}

/// Where the recent list is stored, given a platform and an environment.
///
/// The state directory: per-machine, and never the roaming one. The edge that
/// resolves it for this machine is [`crate::Locations::for_host`], which also
/// moves a list an older build left beside `config.toml`.
#[must_use]
pub fn recent_path_in(platform: Platform, env: &EnvSnapshot) -> Option<PathBuf> {
    crate::locations::inside(platform, DirKind::State, env, FILE_NAME)
}

/// Read the recent list from `path`, pruning entries that have gone away.
///
/// A path rather than a lookup, so the caller decides where the list lives
/// and a test can hand it a temporary one. Resolving it here meant every
/// test that opened a file wrote into the developer's real list.
pub fn load_recent(path: &Path) -> Recent {
    let mut recent = std::fs::read_to_string(path)
        .map(|text| Recent::parse(&text))
        .unwrap_or_default();
    recent.prune_missing();
    recent
}

/// Write the recent list to `path`, creating its directory if needed.
///
/// Failures are ignored: not remembering a filename is not worth an error in
/// front of someone who is trying to write.
///
/// Written beside the target and renamed over it, so a crash or a full disk
/// mid-write leaves the previous list rather than half of one. A torn file
/// would parse as empty -- `Recent::parse` forgives it -- and the user's whole
/// list would be gone for the sake of one entry. `std::fs::rename` replaces
/// the target on both platforms. Not `bp_files::atomic_write`: this crate
/// sits below `bp-files` and the list needs no read-back verification.
pub fn save_recent(path: &Path, recent: &Recent) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    if std::fs::write(&temporary, recent.to_toml()).is_err()
        || std::fs::rename(&temporary, path).is_err()
    {
        let _ = std::fs::remove_file(&temporary);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn most_recent_comes_first() {
        let mut r = Recent::new();
        r.push(&p("/a.txt"));
        r.push(&p("/b.txt"));
        assert_eq!(r.paths(), [p("/b.txt"), p("/a.txt")]);
    }

    #[test]
    fn reopening_moves_rather_than_duplicates() {
        let mut r = Recent::new();
        r.push(&p("/a.txt"));
        r.push(&p("/b.txt"));
        r.push(&p("/a.txt"));

        assert_eq!(
            r.paths(),
            [p("/a.txt"), p("/b.txt")],
            "reopening must not push everything else out"
        );
    }

    #[test]
    fn the_list_is_capped() {
        let mut r = Recent::new();
        for i in 0..(MAX_RECENT + 5) {
            r.push(&p(&format!("/f{i}.txt")));
        }
        assert_eq!(r.paths().len(), MAX_RECENT);
        assert_eq!(
            r.paths()[0],
            p(&format!("/f{}.txt", MAX_RECENT + 4)),
            "the newest survives"
        );
    }

    #[test]
    fn a_corrupt_list_is_an_empty_list() {
        // Never a reason to interrupt the user.
        assert_eq!(Recent::parse("this = = not toml"), Recent::new());
        assert_eq!(Recent::parse(""), Recent::new());
    }

    #[test]
    fn it_round_trips_through_toml() {
        let mut r = Recent::new();
        r.push(&p("/notes/One_17AUG2026.txt"));
        r.push(&p("/notes/Two_17AUG2026.md"));

        assert_eq!(Recent::parse(&r.to_toml()), r);
    }

    #[test]
    fn missing_entries_are_pruned() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.txt");
        std::fs::write(&real, "x").unwrap();

        let mut r = Recent::new();
        r.push(&dir.path().join("gone.txt"));
        r.push(&real);
        assert_eq!(r.paths().len(), 2);

        r.prune_missing();
        assert_eq!(r.paths(), [real], "a deleted file must not be offered");
    }

    #[test]
    fn it_records_paths_and_nothing_else() {
        // ADR-0011: this file says which documents were opened, never
        // anything about what they contain.
        let mut r = Recent::new();
        r.push(&p("/secret/Report_17AUG2026.txt"));
        let toml = r.to_toml();

        assert!(toml.contains("Report_17AUG2026.txt"));
        assert!(
            !toml.to_lowercase().contains("content"),
            "no content field may creep in"
        );
    }

    // --- where the list is kept ----------------------------------------
    //
    // Built from an `EnvSnapshot` here, so these read neither the real
    // environment nor the developer's own configuration directory.

    /// Split a path into (directory, file name) using the *named* platform's
    /// separator.
    ///
    /// `Path::parent` cannot do this job here: it would have to walk a Windows
    /// path while the test runs on Linux, where `\` is an ordinary filename
    /// character and the whole path is therefore one component. Splitting on
    /// the platform's own separator gives both legs the same answer.
    fn split(platform: Platform, path: &str) -> (&str, &str) {
        path.rsplit_once(platform.separator())
            .expect("a resolved path has a directory and a file name")
    }

    #[test]
    fn the_list_sits_in_the_state_directory_and_not_beside_the_config_file() {
        // Beside `config.toml` was where it used to be, and on Windows that
        // directory roams: a list of one machine's absolute paths arrived on
        // every other machine the user signed into.
        let env = crate::tests::whole_env();
        for &platform in Platform::ALL {
            let recent = crate::tests::text(recent_path_in(platform, &env));
            let config = crate::tests::text(crate::config_path_in(platform, &env));
            let (recent_dir, recent_name) = split(platform, &recent);
            let (config_dir, _) = split(platform, &config);

            assert_ne!(
                recent_dir, config_dir,
                "{platform:?}: the list must not sit beside the settings"
            );
            assert_eq!(
                recent_dir,
                crate::tests::text(bp_platform::dirs::directory(platform, DirKind::State, &env)),
            );
            assert_eq!(recent_name, FILE_NAME);
        }
    }

    #[test]
    fn a_list_round_trips_through_the_path_it_is_given() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("a.txt");
        std::fs::write(&existing, b"").unwrap();
        let path = dir.path().join("state").join(FILE_NAME);

        let mut recent = Recent::new();
        recent.push(&existing);
        save_recent(&path, &recent);

        assert_eq!(load_recent(&path), recent);
    }

    #[test]
    fn saving_replaces_the_list_and_leaves_no_temporary_behind() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("a.txt");
        std::fs::write(&existing, b"").unwrap();
        let path = dir.path().join(FILE_NAME);
        std::fs::write(&path, b"paths = []\n").unwrap();

        let mut recent = Recent::new();
        recent.push(&existing);
        save_recent(&path, &recent);

        assert_eq!(load_recent(&path), recent);
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert!(
            names.iter().all(|n| !n.to_string_lossy().ends_with(".tmp")),
            "{names:?}"
        );
    }

    #[test]
    fn a_missing_list_is_an_empty_one() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_recent(&dir.path().join("absent.toml")).is_empty());
    }

    #[test]
    fn a_relative_xdg_state_home_does_not_move_the_list_either() {
        // The same defect `config_path_in` once had: the list must not end up
        // beside the directory the editor happened to be launched from.
        for unusable in crate::tests::UNUSABLE_XDG {
            let env = EnvSnapshot {
                xdg_state_home: Some((*unusable).to_owned()),
                ..crate::tests::whole_env()
            };
            assert_eq!(
                crate::tests::text(recent_path_in(Platform::Linux, &env)),
                "/home/me/.local/state/bachelorpad/recent.toml",
                "{unusable:?} must be ignored"
            );
        }
    }

    #[test]
    fn an_empty_environment_yields_no_path_rather_than_a_guess() {
        for &platform in Platform::ALL {
            assert_eq!(recent_path_in(platform, &EnvSnapshot::default()), None);
        }
    }
}
