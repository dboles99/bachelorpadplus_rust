//! Writing the user's choices back to `config.toml`.
//!
//! Owns [`Settings`] -- the handful of values the user chooses inside the app
//! (View menu, `Tools > Settings...`) -- and [`save`], which puts them into the
//! same `config.toml` that [`crate::resolve`] reads. Deliberately does not own
//! reading: there is one reader, and a saved setting comes back through it at
//! the file's place in the precedence order like any hand-written one.
//!
//! # One file, edited in place
//!
//! The alternative was a second, app-owned `settings.toml` beside
//! `config.toml`. It was rejected because every precedence order it allows is
//! a trap: below `config.toml`, a user who ever wrote `theme = "Green"` by
//! hand finds that picking another theme in the menu silently stops working
//! after a restart; above it, their hand edits silently stop working instead.
//! One file has one answer, and it is the one "Open Config Folder" shows them.
//!
//! So a save edits `config.toml` in place and touches only the keys it was
//! given. The spans come from `toml`'s own parser (`toml::de::DeTable`), so a
//! value is located exactly -- a `#` inside a quoted theme name is not a
//! comment -- and everything outside those spans is copied through byte for
//! byte: comments, blank lines, key order, unknown keys, tables, line endings.
//! A key the file does not have yet goes after the last top-level key, never
//! into a `[table]`, where it would mean something else.
//!
//! Then the result is **checked, not trusted**: it is parsed again and must
//! equal the original document with exactly the new values set. A file this
//! simple rule cannot edit safely -- one that is not valid TOML, or that
//! spells an owned key as a table -- is refused with a [`SaveError`] and left
//! exactly as it was. A setting that does not persist is a message in the
//! status bar; a user's hand-written file mangled is not recoverable.
//!
//! # Precedence
//!
//! Unchanged: command line, environment, `config.toml`, defaults. A saved
//! setting *is* a `config.toml` setting. The consequence for the shell: save
//! what the user changed ([`Settings::between`]), never a snapshot of the
//! resolved [`Config`], which carries `--font-size` and `BACHELORPAD_THEME`
//! and would write a one-off override into the file for good.

use std::fmt;
use std::io::Write as _;
use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::{
    Config, MAX_EDITABLE_LIMIT_MIB, MAX_FONT_SIZE, MAX_TAB_WIDTH, MIN_EDITABLE_LIMIT_MIB,
    MIN_FONT_SIZE, MIN_TAB_WIDTH, SecurityProfile,
};

/// What a new `config.toml` starts with, so the first person to open it knows
/// that the app wrote it and that editing it is safe.
const HEADER: &str = "\
# BachelorPad+ settings.
#
# The editor writes what you choose in the app (the View menu, Tools >
# Settings) into this file. Anything you add by hand -- comments included --
# is kept when it does. Command-line flags and BACHELORPAD_* variables
# override what is here for one run without changing it.
";

/// The settings the user chooses in the app, each `None` unless it is to be
/// written.
///
/// A patch rather than a whole configuration, because what reaches the file
/// has to be only what the user chose -- see the module header on
/// precedence. `renderer` and `log` are not here: they are diagnostics, set
/// by hand. Neither is `show_planned_menus`, a switch for agents and the
/// owner that the app must never write into a user's file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub theme: Option<String>,
    /// Zoom: Ctrl+= and Ctrl+- change this.
    pub font_size: Option<u8>,
    pub tab_width: Option<u8>,
    pub indent_spaces: Option<bool>,
    pub word_wrap: Option<bool>,
    pub line_numbers: Option<bool>,
    pub clipboard_record_other_programs: Option<bool>,
    pub default_profile: Option<SecurityProfile>,
    pub editor_view: Option<bool>,
    pub editable_limit_mib: Option<u8>,
}

impl Settings {
    /// Whatever differs between two resolved configurations.
    ///
    /// How the shell turns "the user just changed something" into a save:
    /// resolve at startup, apply the user's change to a copy, save the
    /// difference. A command-line override is the same on both sides, so it
    /// never reaches the file. A theme going back to `None` is not expressed
    /// -- the app has no way to ask for "no theme", only for another one.
    #[must_use]
    pub fn between(before: &Config, after: &Config) -> Self {
        fn changed<T: PartialEq + Clone>(before: &T, after: &T) -> Option<T> {
            (before != after).then(|| after.clone())
        }
        Self {
            theme: after.theme.clone().filter(|_| before.theme != after.theme),
            font_size: changed(&before.font_size, &after.font_size),
            tab_width: changed(&before.tab_width, &after.tab_width),
            indent_spaces: changed(&before.indent_spaces, &after.indent_spaces),
            word_wrap: changed(&before.word_wrap, &after.word_wrap),
            line_numbers: changed(&before.line_numbers, &after.line_numbers),
            clipboard_record_other_programs: changed(
                &before.clipboard_record_other_programs,
                &after.clipboard_record_other_programs,
            ),
            default_profile: changed(&before.default_profile, &after.default_profile),
            editor_view: changed(&before.editor_view, &after.editor_view),
            editable_limit_mib: changed(&before.editable_limit_mib, &after.editable_limit_mib),
        }
    }

    /// Whether there is nothing to write.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The keys to write and their values, in a fixed order, each checked
    /// against the same bounds [`crate::resolve`] holds a file to -- a value
    /// the reader would refuse must never be written for it to refuse.
    fn assignments(&self) -> Result<Vec<(&'static str, toml::Value)>, SaveError> {
        fn ranged(
            key: &'static str,
            value: Option<u8>,
            min: u8,
            max: u8,
        ) -> Result<Option<(&'static str, toml::Value)>, SaveError> {
            match value {
                None => Ok(None),
                Some(n) if (min..=max).contains(&n) => {
                    Ok(Some((key, toml::Value::Integer(i64::from(n)))))
                }
                Some(n) => Err(SaveError::InvalidValue {
                    key,
                    value: n.to_string(),
                }),
            }
        }
        let switch =
            |key: &'static str, value: Option<bool>| value.map(|b| (key, toml::Value::Boolean(b)));

        let theme = match &self.theme {
            None => None,
            // A control character in a theme name is not a theme; a blank one
            // would read back as a theme called nothing.
            Some(t) if t.trim().is_empty() || t.chars().any(char::is_control) => {
                return Err(SaveError::InvalidValue {
                    key: "theme",
                    value: t.clone(),
                });
            }
            Some(t) => Some(("theme", toml::Value::String(t.clone()))),
        };

        Ok([
            theme,
            ranged("font_size", self.font_size, MIN_FONT_SIZE, MAX_FONT_SIZE)?,
            ranged("tab_width", self.tab_width, MIN_TAB_WIDTH, MAX_TAB_WIDTH)?,
            switch("indent_spaces", self.indent_spaces),
            switch("word_wrap", self.word_wrap),
            switch("line_numbers", self.line_numbers),
            switch(
                "clipboard_record_other_programs",
                self.clipboard_record_other_programs,
            ),
            self.default_profile
                .map(|p| ("default_profile", toml::Value::String(p.name().to_owned()))),
            switch("editor_view", self.editor_view),
            ranged(
                "editable_limit_mib",
                self.editable_limit_mib,
                MIN_EDITABLE_LIMIT_MIB,
                MAX_EDITABLE_LIMIT_MIB,
            )?,
        ]
        .into_iter()
        .flatten()
        .collect())
    }
}

/// Why a save did not happen. In every case the file on disk is as it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    /// A value outside what the reader accepts. A shell bug, not a user one:
    /// the controls should not have offered it.
    InvalidValue { key: &'static str, value: String },
    /// The existing file could not be read, so it cannot be edited without
    /// losing what is in it.
    Unreadable { path: PathBuf, error: String },
    /// The existing file is not valid TOML. It is probably mid-edit in
    /// another window, and overwriting it would throw that edit away.
    Malformed { error: String },
    /// The file spells a setting as a table (`[word_wrap]`, or
    /// `word_wrap.x = 1`), which has no single value to replace.
    Conflict { key: String },
    /// The edited file did not read back as the original with only the new
    /// values set. Nothing was written. A defect in the editing rule, kept as
    /// an error so it can never become a mangled file.
    Unverified,
    /// Writing or renaming the file failed.
    Unwritable { path: PathBuf, error: String },
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValue { key, value } => {
                write!(f, "'{value}' is not a valid {key}; setting not saved")
            }
            Self::Unreadable { path, error } => {
                write!(
                    f,
                    "cannot read {}: {error}; setting not saved",
                    path.display()
                )
            }
            Self::Malformed { error } => write!(
                f,
                "config.toml is not valid TOML ({error}); fix it and the setting will be saved"
            ),
            Self::Conflict { key } => write!(
                f,
                "config.toml defines '{key}' as a table; setting not saved"
            ),
            Self::Unverified => write!(
                f,
                "config.toml could not be updated without changing other settings; setting not saved"
            ),
            Self::Unwritable { path, error } => {
                write!(
                    f,
                    "cannot write {}: {error}; setting not saved",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for SaveError {}

/// Write `settings` into the config file at `path`, keeping everything else
/// in it.
///
/// Creates the file (and its directory) if there is none. A no-op when
/// `settings` is empty or already matches the file, so a save on every View
/// change does not rewrite an unchanged file. Atomic: written beside the
/// target, flushed, and renamed over it, so a crash mid-save leaves the old
/// file rather than half of the new one. A symlinked `config.toml` -- a
/// dotfiles checkout -- is written through, not replaced by a plain file.
pub fn save(path: &Path, settings: &Settings) -> Result<(), SaveError> {
    if settings.is_empty() {
        return Ok(());
    }
    let target = through_symlink(path);
    let existing = match std::fs::read_to_string(&target) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            return Err(SaveError::Unreadable {
                path: target,
                error: e.to_string(),
            });
        }
    };
    let text = edit_config(existing.as_deref(), settings)?;
    if existing.as_deref() == Some(text.as_str()) {
        return Ok(());
    }
    write_atomically(&target, &text).map_err(|e| SaveError::Unwritable {
        path: target.clone(),
        error: e.to_string(),
    })
}

/// The pure half of [`save`]: `existing` is the file's contents, or `None`
/// if there is no file yet, and the result is what the file should hold.
///
/// Exposed so the editing rule can be tested, and shown, without a disk.
pub fn edit_config(existing: Option<&str>, settings: &Settings) -> Result<String, SaveError> {
    let assignments = settings.assignments()?;
    let Some(text) = existing else {
        let mut fresh = HEADER.to_owned();
        fresh.push('\n');
        for (key, value) in &assignments {
            fresh.push_str(&format!("{key} = {value}\n"));
        }
        return Ok(fresh);
    };
    if assignments.is_empty() {
        return Ok(text.to_owned());
    }

    let malformed = |e: toml::de::Error| SaveError::Malformed {
        error: e.message().to_owned(),
    };
    let before: toml::Table = toml::from_str(text).map_err(malformed)?;
    let document = toml::de::DeTable::parse(text).map_err(malformed)?;
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };

    // Replace the values the file already has; collect the rest to add.
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    let mut additions = String::new();
    for (key, value) in &assignments {
        let found = document
            .get_ref()
            .iter()
            .find(|(k, _)| AsRef::<str>::as_ref(k.get_ref()) == *key);
        match found {
            // `key = value`: the key's text ends before the value's begins.
            // A dotted key or a `[header]` reports spans that overlap, and
            // has no one value to replace.
            Some((k, v)) if k.span().end <= v.span().start => {
                edits.push((v.span(), value.to_string()));
            }
            Some(_) => {
                return Err(SaveError::Conflict {
                    key: (*key).to_owned(),
                });
            }
            None => additions.push_str(&format!("{key} = {value}{newline}")),
        }
    }
    if !additions.is_empty() {
        edits.push(insertion(text, &document, newline, additions));
    }

    edits.sort_by_key(|(range, _)| range.start);
    let mut out = String::with_capacity(text.len() + 256);
    let mut copied = 0;
    for (range, replacement) in edits {
        out.push_str(text.get(copied..range.start).ok_or(SaveError::Unverified)?);
        out.push_str(&replacement);
        copied = range.end;
    }
    out.push_str(text.get(copied..).ok_or(SaveError::Unverified)?);

    // Checked, not trusted: the file must read back as the original with
    // exactly the new values set, or nothing is written.
    let mut expected = before;
    for (key, value) in assignments {
        expected.insert(key.to_owned(), value);
    }
    match toml::from_str::<toml::Table>(&out) {
        Ok(after) if after == expected => Ok(out),
        _ => Err(SaveError::Unverified),
    }
}

/// Where new top-level keys go, as an empty range to "replace" with them.
///
/// After the line holding the last top-level `key = value`, so they join the
/// user's own settings. With none, before the first `[table]` -- and before
/// any comment lines directly above it, which describe the table -- because a
/// key below a header belongs to that table. With no table either, at the end.
fn insertion(
    text: &str,
    document: &toml::Spanned<toml::de::DeTable<'_>>,
    newline: &str,
    block: String,
) -> (Range<usize>, String) {
    let last_value_end = document
        .get_ref()
        .iter()
        .filter(|(k, v)| k.span().start <= v.span().start)
        .map(|(_, v)| v.span().end)
        .max();

    let at = |pos: usize, text_after: String| (pos..pos, text_after);
    if let Some(end) = last_value_end {
        return match text.get(end..).and_then(|rest| rest.find('\n')) {
            Some(i) => at(end + i + 1, block),
            // The last line has no newline to put the new keys after.
            None => at(text.len(), format!("{newline}{block}")),
        };
    }

    let mut offset = 0;
    let mut comments_from = None;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start_matches(['\u{feff}', ' ', '\t']);
        if trimmed.starts_with('[') {
            // A byte-order mark stays first: Notepad writes one, and one
            // anywhere else is not TOML.
            let bom = if text.starts_with('\u{feff}') { 3 } else { 0 };
            let pos = comments_from.unwrap_or(offset).max(bom);
            return at(pos, format!("{block}{newline}"));
        }
        if trimmed.starts_with('#') {
            comments_from.get_or_insert(offset);
        } else {
            comments_from = None;
        }
        offset += line.len();
    }
    if text.is_empty() || text.ends_with('\n') {
        at(text.len(), block)
    } else {
        at(text.len(), format!("{newline}{block}"))
    }
}

/// The file a symlink points at, or `path` itself.
///
/// Renaming over a symlink replaces the link, which would quietly detach a
/// dotfiles-managed `config.toml` from its repository.
fn through_symlink(path: &Path) -> PathBuf {
    let is_link = std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink());
    if is_link {
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    }
}

/// Write beside `path` and rename over it.
///
/// Flushed before the rename, unlike the recent list: losing a convenience
/// list to a power cut is shrugged off, losing every setting is not. The old
/// file's permissions carry over, so a `config.toml` a user made private
/// stays private. The process id in the temporary name keeps two running
/// editors from writing into each other's half-finished file.
fn write_atomically(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(format!(".{}.tmp", std::process::id()));
    let temporary = PathBuf::from(temporary);

    let result = (|| {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        if let Ok(meta) = std::fs::metadata(path) {
            std::fs::set_permissions(&temporary, meta.permissions())?;
        }
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Env, Notice, load_from, resolve};

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    /// Every setting, each at a valid value that is not its default.
    fn everything() -> Settings {
        Settings {
            theme: Some("Green".to_owned()),
            font_size: Some(20),
            tab_width: Some(8),
            indent_spaces: Some(true),
            word_wrap: Some(true),
            line_numbers: Some(false),
            clipboard_record_other_programs: Some(true),
            default_profile: Some(SecurityProfile::Confidential),
            editor_view: Some(true),
            editable_limit_mib: Some(64),
        }
    }

    fn read_back(path: &Path) -> (Config, Vec<Notice>) {
        let loaded = load_from(Some(path), &Env::default(), &[]);
        (loaded.config, loaded.notices)
    }

    #[test]
    fn every_setting_saved_is_the_setting_resolved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bachelorpad").join("config.toml");
        save(&path, &everything()).unwrap();

        let (config, notices) = read_back(&path);
        assert!(
            notices.is_empty(),
            "a saved file must read clean: {notices:?}"
        );
        let expected = Config {
            theme: Some("Green".to_owned()),
            font_size: 20,
            tab_width: 8,
            indent_spaces: true,
            word_wrap: true,
            line_numbers: false,
            clipboard_record_other_programs: true,
            default_profile: SecurityProfile::Confidential,
            editor_view: true,
            editable_limit_mib: 64,
            ..Config::default()
        };
        assert_eq!(config, expected);
        assert_eq!(
            Settings::between(&Config::default(), &expected),
            everything(),
            "the test's own two lists of settings must agree, or a new one was missed"
        );
    }

    #[test]
    fn saving_again_replaces_the_value_rather_than_adding_a_second() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        save(&path, &everything()).unwrap();
        save(
            &path,
            &Settings {
                font_size: Some(11),
                word_wrap: Some(false),
                ..Settings::default()
            },
        )
        .unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.matches("font_size").count(), 1, "{text}");
        let (config, notices) = read_back(&path);
        assert!(notices.is_empty(), "a duplicate key would be malformed");
        assert_eq!(config.font_size, 11);
        assert!(!config.word_wrap);
        assert!(config.editor_view, "settings not in the second save stay");
    }

    #[test]
    fn a_hand_written_comment_survives_a_save() {
        let before = "\
# My editor, my rules.
theme = \"Dark\"   # easier at night
font_size = 12

# Things I am trying out.
my_own_key = [1, 2]
";
        let after = edit_config(
            Some(before),
            &Settings {
                theme: Some("Light".to_owned()),
                word_wrap: Some(true),
                ..Settings::default()
            },
        )
        .unwrap();

        assert_eq!(
            after,
            "\
# My editor, my rules.
theme = \"Light\"   # easier at night
font_size = 12

# Things I am trying out.
my_own_key = [1, 2]
word_wrap = true
",
            "only the value changes and the new key joins the others; every comment, blank line and unknown key stays"
        );
    }

    #[test]
    fn a_hash_inside_a_quoted_value_is_not_mistaken_for_a_comment() {
        let before = "theme = \"Solar # 2\" # the # in the name is real\n";
        let after = edit_config(
            Some(before),
            &Settings {
                theme: Some("Dark".to_owned()),
                ..Settings::default()
            },
        )
        .unwrap();
        assert_eq!(after, "theme = \"Dark\" # the # in the name is real\n");
    }

    #[test]
    fn a_new_setting_goes_above_the_users_tables_not_into_one() {
        let before = "\
font_size = 12

# Plugin settings, not the editor's.
[plugins]
word_wrap_style = \"soft\"
";
        let after = edit_config(
            Some(before),
            &Settings {
                word_wrap: Some(true),
                ..Settings::default()
            },
        )
        .unwrap();
        assert_eq!(
            after,
            "\
font_size = 12
word_wrap = true

# Plugin settings, not the editor's.
[plugins]
word_wrap_style = \"soft\"
"
        );
        let (config, _) = resolve(Some(&after), &Env::default(), &[]);
        assert!(config.word_wrap, "a key under [plugins] would not be read");
    }

    #[test]
    fn a_file_of_only_tables_gets_new_settings_above_the_first_and_its_comment() {
        let before = "# About the table.\n[plugins]\nx = 1\n";
        let after = edit_config(
            Some(before),
            &Settings {
                line_numbers: Some(false),
                ..Settings::default()
            },
        )
        .unwrap();
        assert_eq!(
            after,
            "line_numbers = false\n\n# About the table.\n[plugins]\nx = 1\n"
        );
    }

    #[test]
    fn a_file_without_a_final_newline_gains_one_before_the_new_key() {
        for before in ["font_size = 12", "# just a comment"] {
            let after = edit_config(
                Some(before),
                &Settings {
                    editor_view: Some(true),
                    ..Settings::default()
                },
            )
            .unwrap();
            assert_eq!(after, format!("{before}\neditor_view = true\n"));
        }
    }

    #[test]
    fn windows_line_endings_are_kept() {
        let before = "# written in Notepad\r\nfont_size = 12\r\n";
        let after = edit_config(
            Some(before),
            &Settings {
                font_size: Some(16),
                tab_width: Some(2),
                ..Settings::default()
            },
        )
        .unwrap();
        assert_eq!(
            after,
            "# written in Notepad\r\nfont_size = 16\r\ntab_width = 2\r\n"
        );
    }

    #[test]
    fn a_byte_order_mark_stays_at_the_start() {
        let before = "\u{feff}[plugins]\nx = 1\n";
        let after = edit_config(
            Some(before),
            &Settings {
                word_wrap: Some(true),
                ..Settings::default()
            },
        )
        .unwrap();
        assert!(after.starts_with("\u{feff}word_wrap = true\n"), "{after:?}");
    }

    #[test]
    fn a_first_save_writes_an_explanatory_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new").join("config.toml");
        save(
            &path,
            &Settings {
                word_wrap: Some(true),
                ..Settings::default()
            },
        )
        .unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with(HEADER), "{text}");
        assert!(text.ends_with("\nword_wrap = true\n"), "{text}");
    }

    #[test]
    fn an_empty_save_neither_creates_nor_touches_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        save(&path, &Settings::default()).unwrap();
        assert!(!path.exists(), "nothing chosen, nothing written");

        std::fs::write(&path, "font_size =   12  # odd spacing\n").unwrap();
        save(
            &path,
            &Settings {
                font_size: Some(12),
                ..Settings::default()
            },
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "font_size =   12  # odd spacing\n",
            "the value was already right; the file must not be rewritten"
        );
    }

    #[test]
    fn a_malformed_file_is_left_alone_rather_than_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let broken = "# half-way through an edit\ntheme = = \n";
        std::fs::write(&path, broken).unwrap();

        let err = save(&path, &everything()).unwrap_err();
        assert!(matches!(err, SaveError::Malformed { .. }), "{err:?}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), broken);
    }

    #[test]
    fn a_setting_spelt_as_a_table_is_refused_rather_than_mangled() {
        for before in ["word_wrap.soft = true\n", "[word_wrap]\nsoft = true\n"] {
            let err = edit_config(
                Some(before),
                &Settings {
                    word_wrap: Some(true),
                    ..Settings::default()
                },
            )
            .unwrap_err();
            assert_eq!(
                err,
                SaveError::Conflict {
                    key: "word_wrap".to_owned()
                },
                "{before:?}"
            );
        }
    }

    #[test]
    fn a_value_the_reader_would_refuse_is_never_written() {
        let refused = [
            Settings {
                font_size: Some(MAX_FONT_SIZE + 1),
                ..Settings::default()
            },
            Settings {
                tab_width: Some(0),
                ..Settings::default()
            },
            Settings {
                editable_limit_mib: Some(0),
                ..Settings::default()
            },
            Settings {
                editable_limit_mib: Some(MAX_EDITABLE_LIMIT_MIB + 1),
                ..Settings::default()
            },
            Settings {
                theme: Some("  ".to_owned()),
                ..Settings::default()
            },
            Settings {
                theme: Some("Dark\n".to_owned()),
                ..Settings::default()
            },
        ];
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        for settings in refused {
            let err = save(&path, &settings).unwrap_err();
            assert!(matches!(err, SaveError::InvalidValue { .. }), "{err:?}");
            assert!(!path.exists(), "{settings:?} reached the disk");
        }
    }

    #[test]
    fn the_command_line_still_beats_a_saved_setting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        save(&path, &everything()).unwrap();

        let loaded = load_from(
            Some(&path),
            &Env::default(),
            &args(&["--font-size=30", "--theme=Light"]),
        );
        assert_eq!(loaded.config.font_size, 30);
        assert_eq!(loaded.config.theme.as_deref(), Some("Light"));
        assert!(
            loaded.config.word_wrap,
            "the rest of the file still applies"
        );
    }

    #[test]
    fn the_environment_still_beats_a_saved_setting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        save(&path, &everything()).unwrap();

        let env = Env {
            tab_width: Some("3".to_owned()),
            ..Env::default()
        };
        let loaded = load_from(Some(&path), &env, &[]);
        assert_eq!(loaded.config.tab_width, 3);
    }

    #[test]
    fn a_command_line_override_is_never_saved_into_the_file() {
        // The shell's side of the precedence rule: it saves the difference
        // the user made, and `--font-size=30` is on both sides of it.
        let (before, _) = resolve(None, &Env::default(), &args(&["--font-size=30"]));
        let mut after = before.clone();
        after.word_wrap = true;

        assert_eq!(
            Settings::between(&before, &after),
            Settings {
                word_wrap: Some(true),
                ..Settings::default()
            }
        );
    }

    #[test]
    fn nothing_is_left_beside_the_file_after_a_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        save(&path, &everything()).unwrap();
        save(
            &path,
            &Settings {
                font_size: Some(9),
                ..Settings::default()
            },
        )
        .unwrap();

        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["config.toml"], "a temporary file was left behind");
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_config_is_written_through_not_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("dotfiles-config.toml");
        std::fs::write(&real, "# tracked in git\n").unwrap();
        let link = dir.path().join("config.toml");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        save(
            &link,
            &Settings {
                word_wrap: Some(true),
                ..Settings::default()
            },
        )
        .unwrap();

        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink(),
            "the link must still be a link"
        );
        assert_eq!(
            std::fs::read_to_string(&real).unwrap(),
            "# tracked in git\nword_wrap = true\n"
        );
    }
}
