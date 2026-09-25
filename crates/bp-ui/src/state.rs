//! Document and workspace state.
//!
//! Owns `AppState` and everything that answers questions about
//! document/workspace state: what the active document is, whether it is dirty,
//! how to save it, and how to label it in the status bar.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use bp_config::Recent;
use bp_core::{Document, DocumentId, Encoding, LineEnding, UNTITLED, Workspace};
use bp_files::{
    DiskState, EncodeError, FileStamp, LineEndingPolicy, SaveOptions, atomic_write,
    load_with_limit, try_encode,
};
use bp_formats::Format;
use bp_naming::SemanticName;
use bp_theme::ThemeId;
use time::OffsetDateTime;

use crate::menus::action;

/// How much of a document is enough to answer a question about its start.
///
/// Format detection sniffs the beginning, and `refresh` asks on every
/// keystroke. Copying a whole document to look at its first line would put
/// the document's size back into the typing path, which is the cost the rope
/// exists to remove.
pub(crate) const PREFIX_CHARS: usize = 4096;

/// How a save attempt ended.
///
/// Three cases, not two: a close-on-quit flow must not treat "needs a path"
/// or "the disk refused" as success and then discard the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SaveResult {
    Saved,
    /// No path yet. The caller should escalate to Save As.
    NeedsPath,
    /// Attempted and refused; `AppState::error` explains why.
    Failed,
    /// Nothing written: the document's encoding has no bytes for one of its
    /// characters. The caller offers UTF-8 instead (release plan M1-10);
    /// writing a `?` in its place is how a character is lost unannounced.
    Unrepresentable(bp_core::Unrepresentable),
}

/// What a Note-menu action decided to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NoteOutcome {
    /// Report something to the user.
    Show {
        title: String,
        body: String,
    },
    /// Offer to save under a suggested name.
    SaveAs,
    Nothing,
}

/// Whether the buffer text needs pushing back into the editor widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushText {
    /// The document changed underneath the widget: opened, switched, closed.
    Yes,
    /// The user typed. The widget already holds the text, and pushing it back
    /// would clone the whole document on every keystroke to no effect.
    No,
}

pub struct AppState {
    pub(crate) workspace: Workspace,
    /// One editor per document: the rope is the storage, and the caret,
    /// selection and undo stack belong to us.
    ///
    /// True whichever view is drawing. `TextInput` still owns *its* caret and
    /// undo when it is the one on screen, but the text it hands back lands
    /// here as a single undoable replacement, so there is one document.
    editors: HashMap<DocumentId, bp_editor::Editor>,
    /// Draw with the custom surface rather than Slint's `TextInput`.
    pub(crate) editor_view: bool,
    /// The visual row the surface is scrolled to.
    ///
    /// A line *and* a row within it, not a line alone: once a line can wrap,
    /// one taller than the window has to be scrollable through, and a
    /// line-only anchor could only jump over it.
    pub(crate) anchor: bp_editor::view::Anchor,
    /// How many rows fit in it. Slint measures and tells us.
    pub(crate) visible_rows: usize,
    /// How many characters fit across it, for wrapping. Slint measures this
    /// too -- it owns the font, and a column count derived from anything but
    /// the drawn advance would wrap in the wrong place.
    pub(crate) wrap_columns: usize,
    pub(crate) theme: ThemeId,
    /// Track the desktop's light/dark preference rather than staying put.
    ///
    /// A flag beside `theme` rather than a fifth `ThemeId`, because the theme
    /// in force is always one of the four either way -- this only says where
    /// it came from, and therefore whether a later preference change should
    /// move it. ADR-0009 keeps themes as data; this is not one.
    pub(crate) follow_system_theme: bool,
    /// What the desktop last said, or `None` if it will not say. Slint
    /// reports it; nothing here asks the platform directly.
    pub(crate) system_dark: Option<bool>,
    pub(crate) error: Option<String>,
    /// Cached gutter text, and the line count it was built for.
    ///
    /// Rebuilding this on every keystroke was the single largest avoidable
    /// cost in the typing path: it allocates proportionally to the document
    /// on each character typed, while the content only changes when a line is
    /// added or removed. `usize::MAX` is a sentinel meaning "never built",
    /// since 0 is a line count `max(1)` can never produce.
    pub(crate) gutter: String,
    gutter_lines: usize,
    pub(crate) show_gutter: bool,
    pub(crate) wrap_text: bool,
    /// How the Tab key indents, and how wide a tab is drawn.
    ///
    /// One value serving both, because they are the same number seen from two
    /// sides -- a document indented to eight columns and drawn at four does
    /// not look wrong, it looks like a different document.
    pub(crate) indent: bp_editor::Indent,
    /// Editor font size in points, and the only place it is decided.
    ///
    /// Held here rather than read back off the widget because both editor
    /// views draw at it and neither is authoritative -- `EditorSurface`
    /// measures its character advance from whatever it is given, so a size
    /// that lived in the view would put the caret arithmetic somewhere Rust
    /// cannot see. Config supplies the starting value; zoom moves it.
    pub(crate) font_size: u8,
    /// Whether the clipboard history records copies made in other programs
    /// (`clipboard_record_other_programs`). Off unless the user opted in:
    /// recording everything put whatever a password manager copied into the
    /// Edit menu's previews.
    pub(crate) record_other_programs: bool,
    /// Past this many bytes a file opens nowhere editable
    /// (`editable_limit_mib`, 32 MiB unless configured).
    pub(crate) editable_limit: u64,
    /// The profile a document gets when nothing else says otherwise
    /// (`default_profile`).
    pub(crate) default_profile: bp_security::Profile,
    /// Where settings chosen in the window are saved, and what the file last
    /// said. `None` when there is nowhere to save them -- a test, or an
    /// environment naming no configuration directory -- and then a change
    /// lasts for the session, as every change did before.
    pub(crate) settings: Option<SavedSettings>,
    /// The Settings sheet, while it is open.
    pub(crate) settings_sheet: Option<crate::settings_sheet::Sheet>,
    /// Whether the menus of promises are on the bar (`show_planned_menus`).
    pub(crate) show_planned_menus: bool,
    /// Where the log and crash reports go, for Tools ▸ Open Log Folder.
    pub(crate) log_dir: Option<PathBuf>,
    /// Each document's edit count (`Document::edits`) when its checkpoint
    /// was last written (release plan M1-08). A dirty document whose count
    /// has not moved already has a current checkpoint, and writing it again
    /// every interval cost a key derivation each time for an encrypted one.
    checkpointed: HashMap<DocumentId, u64>,
    /// What is drawing the window, in words, for Help ▸ About.
    pub(crate) renderer: String,
    /// Whether typing overwrites (the Insert key).
    ///
    /// The window's, not a document's: one key switches it and every tab
    /// follows, which is what the status bar's `OVR` promises. Each editor is
    /// told on the way to a command, like the indent and the wrap.
    pub(crate) overwrite: bool,
    /// What each document's file looked like when we last read or wrote it,
    /// so an edit made by another program can be noticed.
    stamps: HashMap<DocumentId, FileStamp>,
    pub(crate) recent: Recent,
    /// What `bp-config` could not move out of the old locations, held apart
    /// from `error` until startup has finished opening files -- `open` and
    /// `close` both clear `error`, and this is the only notice of a recovery
    /// journal the product no longer reads. See [`AppState::start_up`].
    left_behind: Option<String>,
    /// Where `recent` is kept. `None` keeps the list in memory only, which
    /// is what an environment naming no state directory gets.
    recent_file: Option<PathBuf>,
    /// Standing warning about the file on disk. Distinct from `error`, which
    /// reports something that just failed; this persists until resolved.
    pub(crate) disk_warning: Option<String>,
    /// Matches for the current find query, and which one is selected.
    pub(crate) matches: Vec<bp_search::Match>,
    match_index: usize,
    pub(crate) find_status: String,
    /// What the go-to-line bar is reporting. Separate from `error`, which is
    /// the status bar's: a bar with its own input owns its own message, or
    /// "there are only 42 lines" would appear at the far end of the window
    /// from the box it is about.
    pub(crate) goto_status: String,
    pub(crate) journal: bp_history::Journal,
    /// Where the security history is appended and read back.
    ///
    /// A field rather than a lookup at each use, so a test gets its own
    /// file. It is not configurable from outside: the production value is
    /// set once, from [`locations`], and nothing changes it.
    pub(crate) audit_path: PathBuf,
    /// Each document's security profile, kept between sessions (release plan
    /// M1-19), and where that is saved. `None` for the path when this system
    /// names no state directory: profiles then last for the session.
    pub(crate) profiles: bp_history::ProfileStore,
    pub(crate) profiles_path: Option<PathBuf>,
    /// The tab the context menu was opened on.
    ///
    /// Held rather than passed with the click, because opening the menu and
    /// choosing a row are two separate events -- by the time a row is
    /// clicked the pointer has moved and the tab under it may be a different
    /// one. `None` means the menu is not open, in which case the rows fall
    /// back to the active tab.
    pub(crate) tab_context: Option<DocumentId>,
    /// Passphrases for the encrypted documents open right now.
    ///
    /// Held for the session so that saving a `.bpadx` does not ask again on
    /// every Ctrl+S -- which would train the user to type it reflexively,
    /// which is worse than holding it. `Zeroizing` because these are wiped
    /// when a document closes rather than left in freed memory.
    ///
    /// A document with an entry here is encrypted; one without is not. That
    /// is the whole test, so there is no second flag to fall out of step.
    pub(crate) passphrases: HashMap<DocumentId, zeroize::Zeroizing<String>>,
    /// Privacy Mode: a session-wide override that can only tighten.
    ///
    /// On `AppState` rather than on a document, because that is what it is
    /// for -- one switch when you are about to share a screen, instead of
    /// auditing every open tab.
    pub(crate) privacy: bp_security::Privacy,
    /// What the passphrase bar is currently asking, if anything.
    pub(crate) ask: Option<crate::passphrase::Ask>,
    /// What the passphrase bar is reporting.
    pub(crate) passphrase_status: String,
    /// Cross-file search results, indexed by the row the user clicks.
    pub(crate) file_hits: Vec<bp_search::FileHit>,
    pub(crate) clips: bp_clipboard::History,
}

impl AppState {
    pub(crate) fn new() -> Self {
        let locations = locations();
        let mut state = Self::at(&locations);
        state.left_behind = describe_left_behind(&locations.left_behind);
        state
    }

    /// Add a notice beside whatever the status bar already says, never over
    /// it. Startup is the case that needs it: several things may each have
    /// something to report, and the last to speak used to erase the rest.
    pub(crate) fn add_notice(&mut self, notice: String) {
        self.error = Some(match self.error.take() {
            Some(earlier) => format!("{earlier}; {notice}"),
            None => notice,
        });
    }

    /// Open the files named on the command line, then say everything startup
    /// has to say.
    ///
    /// The first file replaces the empty document opened at startup, so
    /// `bachelorpad note.txt` shows one tab rather than an untouched Untitled
    /// beside it.
    ///
    /// The notices are composed *after* the files are open, and that order
    /// is the point. `open` and `close` both clear `error`, so a notice set
    /// before them vanished on the most common launch there is -- a file
    /// double-clicked, or opened through a file association. Every notice is
    /// kept: the configuration's, then anything left behind by a move, then
    /// whatever opening the files reported.
    pub(crate) fn start_up(&mut self, config_notice: Option<String>, files: &[PathBuf]) {
        if !files.is_empty() {
            let blank = self.workspace.active_id();
            for path in files {
                self.open(path.clone());
            }
            if let Some(blank) = blank
                && self.workspace.len() > 1
            {
                self.close(blank);
            }
        }

        let notices: Vec<String> = [config_notice, self.left_behind.take(), self.error.take()]
            .into_iter()
            .flatten()
            .collect();
        self.error = (!notices.is_empty()).then(|| notices.join("; "));
    }

    /// Take the user's configuration: the settings it holds, and where to
    /// save changes to them (release plan M1-03).
    ///
    /// Called before `start_up` opens anything, so every document opened at
    /// startup gets the configured default profile. The untitled document
    /// `new` made is given it here.
    pub(crate) fn configure(&mut self, config: &bp_config::Config, save_to: Option<PathBuf>) {
        self.show_gutter = config.line_numbers;
        self.wrap_text = config.word_wrap;
        self.indent = bp_editor::Indent {
            spaces: config.indent_spaces,
            width: usize::from(config.tab_width),
        };
        self.font_size = bp_config::zoom(config.font_size, 0);
        self.record_other_programs = config.clipboard_record_other_programs;
        self.editable_limit = config.editable_limit_bytes();
        self.default_profile = profile_named(config.default_profile);
        self.show_planned_menus = config.show_planned_menus;
        let ids: Vec<DocumentId> = self.workspace.iter().map(Document::id).collect();
        for id in ids {
            self.give_default_profile(id);
        }
        self.settings = save_to.map(|path| SavedSettings {
            path,
            baseline: config.clone(),
        });
    }

    /// Give a document that has just appeared the configured default
    /// profile. Straight onto the document rather than through
    /// `set_security`: a default is not a change the user made, and the
    /// security history records changes.
    fn give_default_profile(&mut self, id: DocumentId) {
        if self.default_profile == bp_security::Profile::Standard {
            return;
        }
        if let Some(doc) = self.workspace.get_mut(id) {
            doc.set_security(bp_security::Security::Named(self.default_profile));
        }
    }

    /// Give a document the profile the user last set on the file at `path`,
    /// if there is one. After `give_default_profile`, so a profile set on
    /// this very file beats the default for new ones.
    fn recall_profile(&mut self, id: DocumentId, path: &Path) {
        if let Some(security) = self.profiles.get(path)
            && let Some(doc) = self.workspace.get_mut(id)
        {
            doc.set_security(security);
        }
    }

    /// Keep `security` as the profile of the file at `path` for next time.
    ///
    /// The document's own setting, never the one Privacy Mode clamps it to:
    /// Privacy Mode is a session's override, and saving what it forced would
    /// leave a document stricter than the user ever set it once the mode is
    /// off. A failure to save is a status line; the profile still applies.
    fn keep_profile(&mut self, path: &Path, security: bp_security::Security) {
        self.profiles.set(path, security);
        if let Some(store) = &self.profiles_path
            && let Err(e) = self.profiles.save(store)
        {
            self.error = Some(format!(
                "this document's profile will not be remembered next time: {e}"
            ));
        }
    }

    /// Save a setting the user just chose, so it is there next time.
    ///
    /// `change` applies it to a copy of what the file last said, and only
    /// the difference is written (`bp_config::Settings::between`). Never a
    /// snapshot of everything in force: that would write a one-off
    /// `--font-size` from the command line into the file for good. A failure
    /// is a status-bar line, never fatal -- the setting still applies for
    /// this session.
    pub(crate) fn remember(&mut self, change: impl FnOnce(&mut bp_config::Config)) {
        let Some(saved) = &mut self.settings else {
            return;
        };
        let mut after = saved.baseline.clone();
        change(&mut after);
        let difference = bp_config::Settings::between(&saved.baseline, &after);
        match bp_config::save(&saved.path, &difference) {
            Ok(()) => saved.baseline = after,
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// A fresh state whose files live at `locations`.
    fn at(locations: &bp_config::Locations) -> Self {
        // A store that cannot be read is empty, and says so: documents then
        // open with the default profile, which the notice tells the user,
        // rather than the editor refusing to start over a lost preference.
        let (profiles, profiles_notice) = locations.profiles.as_deref().map_or(
            (bp_history::ProfileStore::new(), None),
            bp_history::ProfileStore::load,
        );
        let mut workspace = Workspace::new();
        let id = workspace.open_new(now());
        let mut editors = HashMap::new();
        editors.insert(id, bp_editor::Editor::default());
        Self {
            workspace,
            editors,
            editor_view: false,
            anchor: bp_editor::view::Anchor::default(),
            // Replaced by Slint's own measurement as soon as the surface has
            // a height; only Page Up before the first frame would see this.
            visible_rows: 30,
            // Replaced by Slint's measurement on the first layout pass, like
            // `visible_rows`. Only wrapping before the first frame sees this.
            wrap_columns: 80,
            theme: ThemeId::default(),
            follow_system_theme: false,
            system_dark: None,
            error: profiles_notice,
            gutter: String::new(),
            gutter_lines: usize::MAX,
            show_gutter: true,
            wrap_text: false,
            indent: bp_editor::Indent::default(),
            font_size: bp_config::DEFAULT_FONT_SIZE,
            record_other_programs: false,
            editable_limit: bp_files::EDITABLE_LIMIT_BYTES,
            default_profile: bp_security::Profile::Standard,
            settings: None,
            settings_sheet: None,
            show_planned_menus: false,
            log_dir: None,
            checkpointed: HashMap::new(),
            renderer: String::new(),
            overwrite: false,
            stamps: HashMap::new(),
            recent: locations
                .recent
                .as_deref()
                .map(bp_config::load_recent)
                .unwrap_or_default(),
            recent_file: locations.recent.clone(),
            left_behind: None,
            disk_warning: None,
            matches: Vec::new(),
            match_index: 0,
            find_status: String::new(),
            goto_status: String::new(),
            // Relative fallbacks for an environment naming no directory at
            // all, which is what these were before they had a home.
            journal: bp_history::Journal::new(
                locations
                    .recovery
                    .clone()
                    .unwrap_or_else(|| PathBuf::from(bp_config::locations::RECOVERY_DIR)),
            ),
            audit_path: locations
                .security_history
                .clone()
                .unwrap_or_else(|| PathBuf::from(bp_config::locations::SECURITY_HISTORY_FILE)),
            profiles,
            profiles_path: locations.profiles.clone(),
            passphrases: HashMap::new(),
            privacy: bp_security::Privacy::default(),
            ask: None,
            passphrase_status: String::new(),
            tab_context: None,
            file_hits: Vec::new(),
            clips: bp_clipboard::History::new(),
        }
    }

    /// Write a recovery checkpoint for every unsaved document that changed
    /// since its last one, and clear the journal for those that are now
    /// clean -- all on this thread.
    ///
    /// specs.md section 3 is emphatic that a checkpoint is not a save, and
    /// `bp-core` enforces that: `record_checkpoint` deliberately leaves the
    /// document dirty.
    ///
    /// The three steps below, run in a row. The autosave timer runs the
    /// middle one on a background thread instead (release plan M1-07), so a
    /// sealed checkpoint's key derivation never stalls the keyboard.
    pub(crate) fn checkpoint_all(&mut self) {
        let jobs = self.prepare_checkpoints();
        let journal = self.journal.clone();
        let written = write_checkpoints(&journal, jobs);
        self.apply_checkpoints(written);
    }

    /// The UI-thread half of a checkpoint pass: discard the journal of every
    /// clean document, and snapshot every dirty one whose checkpoint is not
    /// already current, each with a journal ticket.
    ///
    /// The ticket is what makes writing later, elsewhere, safe: a save or a
    /// close between now and the write discards the document's journal, and
    /// that discard cancels the ticket, so a snapshot taken before it can
    /// never land after it and be offered as "recovered" work the user
    /// already saved or threw away.
    pub(crate) fn prepare_checkpoints(&mut self) -> Vec<CheckpointJob> {
        let ids: Vec<DocumentId> = self.workspace.iter().map(Document::id).collect();
        let mut jobs = Vec::new();

        for id in ids {
            let Some(doc) = self.workspace.get(id) else {
                continue;
            };
            if !doc.is_dirty() {
                // Clean means the file holds the work; the journal must not
                // linger and offer to "recover" a stale copy.
                let _ = self.journal.discard(id.get());
                self.checkpointed.remove(&id);
                continue;
            }
            let edits = doc.edits();
            if self.checkpointed.get(&id) == Some(&edits) {
                continue;
            }

            let checkpoint = bp_history::Checkpoint {
                path: doc.path().map(Path::to_path_buf),
                name: doc.display_name().to_owned(),
                text: self.text_of(id).to_owned(),
                written_at: bp_history::now_unix(),
            };
            // The document's own profile, not a global setting: two tabs
            // open side by side can be governed differently, and the
            // stricter one must not be relaxed by the other being open.
            let recovery = doc.security().policy_under(self.privacy).recovery;
            // Its own passphrase, if it has one. A sealed journal is
            // encrypted with the document's key so that it can be recovered
            // at unlock time and never needs a prompt of its own (ADR-0022).
            let passphrase = self
                .passphrases
                .get(&id)
                .map(|p| zeroize::Zeroizing::new(p.to_string()));
            jobs.push(CheckpointJob {
                id,
                edits,
                ticket: self.journal.ticket(id.get()),
                checkpoint,
                recovery,
                passphrase,
            });
        }
        jobs
    }

    /// The UI-thread half after the write: record what reached the journal,
    /// and say so when a profile could not be honoured.
    pub(crate) fn apply_checkpoints(&mut self, written: Vec<CheckpointWritten>) {
        let at = now();
        for done in written {
            match done.result {
                Ok(bp_history::Written::Yes) => {
                    if let Some(doc) = self.workspace.get_mut(done.id) {
                        doc.record_checkpoint(at);
                    }
                    // The count the snapshot was taken at: typing since then
                    // leaves the record behind, and the next pass writes again.
                    self.checkpointed.insert(done.id, done.edits);
                }
                // A profile doing what it was set to do is not news; one that
                // cannot be honoured is. `notice` decides which this was.
                Ok(bp_history::Written::Refused(refusal)) => {
                    if let Some(message) = refusal.notice() {
                        self.error = Some(message.to_owned());
                    }
                }
                // A save or a close got there first; its discard stands.
                Ok(bp_history::Written::Superseded) | Err(_) => {}
            }
        }
    }

    /// Open recovered documents as unsaved tabs.
    pub(crate) fn restore(&mut self, entries: Vec<(u64, bp_history::Checkpoint)>) {
        for (_, entry) in entries {
            let id = match entry.path {
                Some(path) => {
                    let created = created_at(&path);
                    self.workspace.open_path(path, created)
                }
                None => self.workspace.open_new(now()),
            };
            self.give_default_profile(id);
            if let Some(path) = doc_path(&self.workspace, id) {
                self.recall_profile(id, &path);
            }
            self.editors.insert(id, bp_editor::Editor::new(&entry.text));
            // Recovered work is by definition not on disk yet.
            if let Some(doc) = self.workspace.get_mut(id) {
                doc.mark_modified();
            }
        }
    }

    /// Recompute matches for `query` against the active document.
    ///
    /// Returns the range to select, if there is one.
    pub(crate) fn find(&mut self, query: &bp_search::Query) -> Option<std::ops::Range<usize>> {
        self.match_index = 0;
        if query.is_empty() {
            self.matches.clear();
            self.find_status.clear();
            return None;
        }
        let text = self.active_text();
        match bp_search::find_all(&text, query) {
            Ok(found) => {
                self.matches = found;
                self.find_status = if self.matches.is_empty() {
                    "no matches".to_owned()
                } else {
                    format!("1 of {}", self.matches.len())
                };
                self.matches.first().map(|m| m.range.clone())
            }
            Err(e) => {
                // A half-typed regex is the normal case while typing, so this
                // reports rather than alarms.
                self.matches.clear();
                self.find_status = e.to_string();
                None
            }
        }
    }

    /// Step to the next or previous match, wrapping.
    pub(crate) fn step_match(&mut self, forward: bool) -> Option<std::ops::Range<usize>> {
        if self.matches.is_empty() {
            return None;
        }
        let len = self.matches.len();
        self.match_index = if forward {
            (self.match_index + 1) % len
        } else {
            (self.match_index + len - 1) % len
        };
        self.find_status = format!("{} of {len}", self.match_index + 1);
        Some(self.matches[self.match_index].range.clone())
    }

    /// Note that this document and its file are in step, and remember the
    /// file as recently used.
    fn mark_in_step(&mut self, id: DocumentId, path: &Path) {
        if let Some(stamp) = FileStamp::of(path) {
            self.stamps.insert(id, stamp);
        }
        self.disk_warning = None;
        // The recent list is a record *about* the document -- its path, on
        // disk, outliving the session -- which is exactly what
        // `Metadata::Disabled` forbids. Confidential and Maximum set it, and
        // Privacy Mode clamps every document to it, so one check covers all
        // three. It used to be written regardless, the one record in the
        // product that ignored the profile.
        let recordable = self.workspace.get(id).is_none_or(|doc| {
            doc.security().policy_under(self.privacy).metadata != bp_security::Metadata::Disabled
        });
        if !recordable {
            return;
        }
        self.recent.push(path);
        if let Some(file) = &self.recent_file {
            bp_config::save_recent(file, &self.recent);
        }
    }

    /// How the active document's file compares with what we last saw.
    fn disk_state(&self) -> Option<DiskState> {
        let id = self.workspace.active_id()?;
        let path = self.workspace.get(id)?.path()?;
        Some(bp_files::check(path, *self.stamps.get(&id)?))
    }

    /// Refresh the standing warning about the file on disk.
    pub(crate) fn poll_disk(&mut self) {
        self.disk_warning = match self.disk_state() {
            Some(DiskState::Modified) => {
                Some("⚠ changed on disk by another program — File ▸ Reload from Disk".to_owned())
            }
            Some(DiskState::Missing) => {
                Some("⚠ no longer on disk — Save will write it again".to_owned())
            }
            _ => None,
        };
    }

    pub(crate) fn recent_path(&self, index: usize) -> Option<PathBuf> {
        self.recent.paths().get(index).cloned()
    }

    /// Work out what a Note-menu action should do.
    ///
    /// Returns a decision rather than acting, so the extraction stays
    /// testable without a window and the dialogs stay in one place.
    pub(crate) fn note_action(&self, id: i32) -> NoteOutcome {
        let text = self.active_text();
        match id {
            action::NOTE_TITLE => bp_semantic::suggest_title(&text).map_or(
                NoteOutcome::Show {
                    title: "Suggest title".to_owned(),
                    body: "There is nothing in this document to take a title from.".to_owned(),
                },
                |title| NoteOutcome::Show {
                    title: "Suggested title".to_owned(),
                    body: format!("{title}\n\nUse Note ▸ Semantic Rename to save under this name."),
                },
            ),

            // A physical rename needs explicit approval (PROJECT_MEMORY), so
            // this offers a filename and lets the save dialog be the consent.
            action::NOTE_RENAME => NoteOutcome::SaveAs,

            action::NOTE_SUMMARY => {
                let summary = bp_semantic::summary(&text, 400);
                NoteOutcome::Show {
                    title: "Summary".to_owned(),
                    body: if summary.is_empty() {
                        "No prose to summarise.".to_owned()
                    } else {
                        summary
                    },
                }
            }

            action::NOTE_KEYWORDS => {
                let keywords = bp_semantic::keywords(&text, 12);
                NoteOutcome::Show {
                    title: "Keywords".to_owned(),
                    body: if keywords.is_empty() {
                        "No distinctive words found.".to_owned()
                    } else {
                        keywords.join(", ")
                    },
                }
            }

            action::NOTE_OUTLINE => {
                let outline = bp_semantic::outline(&text);
                NoteOutcome::Show {
                    title: "Outline".to_owned(),
                    body: if outline.is_empty() {
                        "No headings in this document.".to_owned()
                    } else {
                        outline
                            .iter()
                            .map(|h| {
                                format!(
                                    "{}{}  (line {})",
                                    "    ".repeat(h.level.saturating_sub(1)),
                                    h.text,
                                    h.line
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    },
                }
            }

            _ => NoteOutcome::Nothing,
        }
    }

    /// A filename suggested from the document's own content.
    ///
    /// Falls back to the document's title when there is nothing to extract,
    /// so Save As always has something to offer.
    fn semantic_filename(&self, id: DocumentId) -> Option<String> {
        let doc = self.workspace.get(id)?;
        let title = bp_semantic::suggest_title(&self.text_of(id))?;
        let extension = doc
            .path()
            .and_then(Path::extension)
            .and_then(|e| e.to_str())
            .unwrap_or("txt");
        Some(SemanticName::new(&title, doc.created_at().date(), extension).to_filename())
    }

    /// What the active document is, by extension then by content.
    pub(crate) fn format(&self) -> Format {
        let path = self.workspace.active().and_then(Document::path);
        // A prefix, not the document: this is asked on every refresh, and
        // detection only ever sniffs the beginning.
        bp_formats::detect(path, &self.active_prefix())
    }

    /// Replace the active document's text with the result of a data
    /// operation, leaving it unsaved.
    ///
    /// Applied as an ordinary edit: the user can undo it, and nothing reaches
    /// disk until they save.
    fn apply_to_active(&mut self, result: Result<String, bp_data::DataError>) {
        match result {
            Ok(text) => {
                self.error = None;
                self.edit(text);
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// Apply a YAML rewrite, and say what it cost or why it refused.
    ///
    /// Two things ADR-0023 requires the caller to say, and neither is
    /// `bp-data`'s to say for it.
    ///
    /// On success: `saphyr` parses YAML into data, so a comment -- which is
    /// not data -- has nothing to be put back from, and an alias is resolved
    /// into a copy of what it pointed at. Both preserve what the document
    /// *means* while changing what it says. The menu row warns before the
    /// click; this says it again after, because the status bar is what is on
    /// screen when the user looks at the result.
    ///
    /// On failure: that **nothing changed**. The library's own sentence
    /// follows verbatim rather than being re-worded here -- the three
    /// refusals ADR-0023 designs for each name their limit and where it was
    /// hit, and a shell that paraphrased them would be a second place for
    /// that wording to drift.
    fn apply_yaml(&mut self, result: Result<String, bp_data::DataError>, did: &str) {
        match result {
            Ok(text) => {
                self.edit(text);
                self.error = Some(format!(
                    "✓ {did} — comments were not kept and aliases were expanded; Ctrl+Z undoes it"
                ));
            }
            Err(e) => self.error = Some(format!("this YAML was left unchanged — {e}")),
        }
    }

    /// Apply a whole-document line operation.
    ///
    /// An ordinary edit, like the data operations: undoable, and nothing
    /// reaches disk until the user saves. The caret-dependent members of the
    /// family -- Duplicate Line, Move Line Up/Down -- are not here, because
    /// the caret is not readable until the custom editor view exists.
    pub(crate) fn run_line_action(&mut self, id: i32) {
        use bp_editor::lines::{self, Order};

        let text = self.active_text();
        let changed = match id {
            action::LINES_SORT_ASC => lines::sort(&text, Order::Ascending),
            action::LINES_SORT_DESC => lines::sort(&text, Order::Descending),
            action::LINES_DEDUPE => lines::remove_duplicates(&text),
            action::LINES_REVERSE => lines::reverse(&text),
            action::LINES_TRIM => lines::trim_trailing_whitespace(&text),
            _ => return,
        };
        self.error = None;
        self.edit(changed);
    }

    /// Apply a caret-dependent line operation: Duplicate Line, Move Line
    /// Up/Down.
    ///
    /// Unlike `run_line_action`, these move and duplicate the line the
    /// caret is on rather than rewriting the whole document, so they call
    /// `Editor` directly. Under `TextInput` the caret is the widget's, and
    /// the caller copies it in first (`follow_widget_caret`) and back out
    /// after -- release plan M1-04, which retired the "needs --editor-view"
    /// gate this used to have.
    pub(crate) fn run_line_edit(&mut self, id: i32) {
        // Compared rather than assumed: Duplicate Line always changes the
        // document, but Move Line Up/Down is a deliberate no-op at the
        // first or last line, and `bp-editor` reports that only through its
        // undo stack, not a return value. A no-op must not mark a clean
        // document dirty.
        let before = self.active_text();
        let Some(editor) = self.active_editor_mut() else {
            return;
        };
        match id {
            action::DUPLICATE_LINE => editor.duplicate_line(),
            action::MOVE_LINE_UP => editor.move_line_up(),
            action::MOVE_LINE_DOWN => editor.move_line_down(),
            _ => return,
        }
        if self.active_text() != before {
            self.mark_edited();
        }
    }

    /// Copy the `TextInput` view's caret, given as UTF-8 byte offsets, into
    /// the active editor, which counts characters.
    ///
    /// Everything that needs the caret -- the line operations, Insert
    /// Date/Time, the status bar's Ln/Col -- reads the editor's, so under the
    /// default view the widget's is copied in first. A no-op under the custom
    /// surface, where the editor's caret is the only one.
    pub(crate) fn follow_widget_caret(&mut self, anchor: i32, cursor: i32) {
        if self.editor_view {
            return;
        }
        let Some(editor) = self.active_editor_mut() else {
            return;
        };
        let to_char = |byte: i32| {
            editor
                .buffer()
                .byte_to_char(usize::try_from(byte).unwrap_or(0))
        };
        let (anchor, cursor) = (to_char(anchor), to_char(cursor));
        editor.select(anchor, cursor);
    }

    /// The active editor's selection as a character range, anchor first --
    /// the caret alone when nothing is selected. What the default view's
    /// caret is put back to after an operation replaced its text.
    pub(crate) fn caret_range(&self) -> Option<std::ops::Range<usize>> {
        let editor = self.active_editor()?;
        let cursor = editor.cursor();
        Some(match editor.selection() {
            Some(selected) if selected.end == cursor => selected,
            // Selected backwards: the anchor is the far end.
            Some(selected) => selected.end..selected.start,
            None => cursor..cursor,
        })
    }

    /// Character offset `char_idx` of the active document as a UTF-8 byte
    /// offset, for `TextInput`, which counts bytes.
    pub(crate) fn char_to_byte(&self, char_idx: usize) -> usize {
        self.active_editor()
            .map_or(char_idx, |editor| editor.buffer().char_to_byte(char_idx))
    }

    /// UTF-8 byte offset `byte` of the active document as a character
    /// offset.
    pub(crate) fn byte_to_char(&self, byte: usize) -> usize {
        self.active_editor()
            .map_or(byte, |editor| editor.buffer().byte_to_char(byte))
    }

    /// Run a data operation, reporting the outcome in the status bar.
    pub(crate) fn run_data_action(&mut self, id: i32) {
        let text = self.active_text().to_owned();
        match (id, self.format()) {
            (action::DATA_VALIDATE, Format::Json) => {
                self.error = Some(match bp_data::json_validate(&text) {
                    Ok(()) => "✓ valid JSON".to_owned(),
                    Err(e) => format!("invalid JSON — {e}"),
                });
            }
            (action::DATA_VALIDATE, Format::JsonLines) => {
                self.error = Some(bp_data::jsonl_validate(&text).summary());
            }
            (action::DATA_VALIDATE, Format::Toml) => {
                self.error = Some(match bp_data::toml_validate(&text) {
                    Ok(()) => "✓ valid TOML".to_owned(),
                    Err(e) => format!("invalid TOML — {e}"),
                });
            }
            // The document count is part of the answer rather than a nicety.
            // A YAML file is a *stream*: Format and Minify write every
            // document back and Convert to JSON refuses more than one, so the
            // first place a user learns there are three must not be the
            // refusal (ADR-0023).
            (action::DATA_VALIDATE, Format::Yaml) => {
                let count = bp_data::yaml_validate(&text)
                    .and_then(|()| bp_data::yaml_document_count(&text));
                self.error = Some(match count {
                    Ok(0) => "✓ valid YAML — no documents in this file".to_owned(),
                    Ok(1) => "✓ valid YAML".to_owned(),
                    Ok(n) => format!("✓ valid YAML — {n} documents"),
                    Err(e) => format!("invalid YAML — {e}"),
                });
            }
            (action::DATA_FORMAT, Format::Json) => {
                self.apply_to_active(bp_data::json_format(&text));
            }
            (action::DATA_SORT_KEYS, Format::Json) => {
                self.apply_to_active(bp_data::json_sort_keys(&text));
            }
            (action::DATA_FORMAT, Format::Toml) => match bp_data::toml_format(&text) {
                // Said after the fact as well as on the row, like YAML: the
                // document has changed in a way a person would not guess.
                Ok(formatted) => {
                    self.edit(formatted);
                    self.error = Some(
                        "✓ formatted — comments were not kept and keys were sorted; Ctrl+Z undoes it"
                            .to_owned(),
                    );
                }
                Err(e) => self.error = Some(format!("this TOML was left unchanged — {e}")),
            },
            (action::DATA_MINIFY, Format::Json) => {
                self.apply_to_active(bp_data::json_minify(&text));
            }
            (action::DATA_FORMAT, Format::Yaml) => {
                self.apply_yaml(bp_data::yaml_format(&text), "formatted");
            }
            (action::DATA_MINIFY, Format::Yaml) => {
                self.apply_yaml(bp_data::yaml_minify(&text), "rewritten in flow style");
            }
            (action::DATA_YAML_TO_JSON, _) => {
                self.apply_yaml(bp_data::yaml_to_json(&text), "converted to JSON");
            }
            // The one direction that loses nothing, so it is the one that
            // does not go through `apply_yaml`: every JSON value has a YAML
            // spelling, and there are no comments in JSON to drop.
            (action::DATA_JSON_TO_YAML, _) => {
                let converted = bp_data::json_to_yaml(&text);
                let ok = converted.is_ok();
                self.apply_to_active(converted);
                if ok {
                    self.error = Some("✓ converted to YAML".to_owned());
                }
            }
            (action::DATA_TO_JSONL, _) => self.apply_to_active(bp_data::json_to_jsonl(&text)),
            (action::DATA_TO_JSON, _) => self.apply_to_active(bp_data::jsonl_to_json(&text)),
            (action::DATA_REPORT, _) => {
                self.error = Some(match bp_data::delimited_report(&text) {
                    Ok(report) => report.summary(),
                    Err(e) => format!("could not read as a table — {e}"),
                });
            }
            (action::DATA_CSV_TO_JSON, _) => {
                self.apply_to_active(bp_data::delimited_to_json(&text));
            }
            (action::DATA_CSV_TO_JSONL, _) => {
                self.apply_to_active(bp_data::delimited_to_jsonl(&text));
            }
            (action::DATA_COLUMN_TYPES, _) => {
                self.error = Some(match bp_data::column_types(&text) {
                    Ok(columns) if columns.is_empty() => "no columns to report".to_owned(),
                    Ok(columns) => columns
                        .iter()
                        .map(bp_data::ColumnReport::summary)
                        .collect::<Vec<_>>()
                        .join(" │ "),
                    Err(e) => format!("could not read as a table — {e}"),
                });
            }
            // A row that does not apply to this format. The menu should not
            // have offered it; doing nothing is better than guessing.
            _ => {}
        }
    }

    /// Discard edits and re-read the active document from disk.
    pub(crate) fn reload(&mut self) {
        self.error = None;
        let Some(id) = self.workspace.active_id() else {
            return;
        };
        let Some(path) = self
            .workspace
            .get(id)
            .and_then(|d| d.path())
            .map(Path::to_path_buf)
        else {
            return;
        };
        match load_with_limit(&path, self.editable_limit) {
            Ok(file) => {
                if let Some(doc) = self.workspace.get_mut(id) {
                    doc.set_encoding(file.encoding);
                    doc.set_line_ending(file.line_ending);
                    // Back in step with disk, so the document is clean again.
                    doc.record_disk_save(now());
                }
                self.editors.insert(id, bp_editor::Editor::new(&file.text));
                self.mark_in_step(id, &path);
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// Change how the document will be written, and mark it unsaved.
    ///
    /// The bytes on disk no longer match the intent, which is exactly what
    /// "unsaved" means -- leaving it clean would hide a pending change.
    pub(crate) fn set_line_ending(&mut self, line_ending: LineEnding) {
        if let Some(doc) = self.workspace.active_mut()
            && doc.line_ending() != line_ending
        {
            doc.set_line_ending(line_ending);
            doc.mark_modified();
        }
    }

    pub(crate) fn set_encoding(&mut self, encoding: Encoding) {
        if let Some(id) = self.workspace.active_id() {
            self.set_encoding_of(id, encoding);
        }
    }

    /// Change a document's encoding, refusing one that cannot hold its text.
    ///
    /// Refused here, when the user chooses it, rather than discovered at the
    /// next save: Unicode holds everything, but Windows-1252 has 256
    /// characters, and the status bar says which one stands in the way and
    /// where, so the user can decide what to do about it.
    pub(crate) fn set_encoding_of(&mut self, id: DocumentId, encoding: Encoding) {
        if !encoding.is_unicode() {
            let text = self.text_of(id);
            if let Err(EncodeError::Unrepresentable(what)) =
                try_encode(&text, encoding, LineEndingPolicy::Preserve)
            {
                let line = text[..what.byte_offset].matches('\n').count() + 1;
                let current = self
                    .workspace
                    .get(id)
                    .map_or("unchanged", |d| d.encoding().label());
                self.error = Some(format!(
                    "still {current}: \"{}\" on line {line} has no {} form",
                    what.character,
                    encoding.label()
                ));
                return;
            }
        }
        if let Some(doc) = self.workspace.get_mut(id)
            && doc.encoding() != encoding
        {
            doc.set_encoding(encoding);
            doc.mark_modified();
        }
    }

    pub(crate) fn active_editor(&self) -> Option<&bp_editor::Editor> {
        self.workspace
            .active_id()
            .and_then(|id| self.editors.get(&id))
    }

    pub(crate) fn active_editor_mut(&mut self) -> Option<&mut bp_editor::Editor> {
        self.workspace
            .active_id()
            .and_then(|id| self.editors.get_mut(&id))
    }

    /// The active document as one string.
    ///
    /// Allocates: a rope is not contiguous, so "the whole document as text"
    /// costs a copy. That is the right trade -- saving, searching and format
    /// conversion each want the whole thing once, while editing wants none of
    /// it -- but it is why nothing on the typing path calls this.
    pub(crate) fn active_text(&self) -> String {
        self.active_editor()
            .map(bp_editor::Editor::text)
            .unwrap_or_default()
    }

    fn text_of(&self, id: DocumentId) -> String {
        self.editors
            .get(&id)
            .map(bp_editor::Editor::text)
            .unwrap_or_default()
    }

    /// The first [`PREFIX_CHARS`] characters of the active document.
    fn active_prefix(&self) -> String {
        self.active_editor().map_or_else(String::new, |editor| {
            editor
                .buffer()
                .slice(0..PREFIX_CHARS.min(editor.buffer().len_chars()))
        })
    }

    /// Whether the document has anything worth extracting a title from.
    ///
    /// Checks a prefix rather than the whole document, because this is asked
    /// on every refresh. A document longer than the prefix and made entirely
    /// of whitespace would answer wrongly, and is not worth a full scan.
    pub(crate) fn active_has_content(&self) -> bool {
        self.active_editor().is_some_and(|editor| {
            editor.buffer().len_chars() > PREFIX_CHARS || !self.active_prefix().trim().is_empty()
        })
    }

    /// Tab label for `id`, for use in prompts.
    pub(crate) fn display_name(&self, id: DocumentId) -> String {
        self.workspace
            .get(id)
            .map_or_else(|| UNTITLED.to_owned(), |d| d.display_name().to_owned())
    }

    pub(crate) fn is_dirty(&self, id: DocumentId) -> bool {
        self.workspace.get(id).is_some_and(Document::is_dirty)
    }

    /// Rebuild the gutter only when the line count actually changed.
    ///
    /// Returns `true` if the cache changed and the UI needs the new value.
    pub(crate) fn sync_gutter(&mut self) -> bool {
        // From the rope, which already counts the line after a trailing
        // newline -- unlike `str::lines()`, which ignores it and left every
        // file ending in one with a gutter shorter than the text beside it.
        let lines = self
            .active_editor()
            .map_or(1, |editor| editor.buffer().len_lines());
        if self.gutter_lines == lines {
            return false;
        }
        self.gutter_lines = lines;
        self.gutter.clear();
        for n in 1..=lines {
            if n > 1 {
                self.gutter.push('\n');
            }
            let _ = write!(self.gutter, "{n}");
        }
        true
    }

    pub(crate) fn new_document(&mut self) {
        self.error = None;
        let id = self.workspace.open_new(now());
        self.editors.insert(id, bp_editor::Editor::default());
        self.give_default_profile(id);
    }

    pub(crate) fn open(&mut self, path: PathBuf) {
        self.error = None;
        let path2 = path.clone();
        match load_with_limit(&path, self.editable_limit) {
            Ok(file) => {
                let id = self.workspace.open_path(path, created_at(&path2));
                self.give_default_profile(id);
                self.recall_profile(id, &path2);
                if let Some(doc) = self.workspace.get_mut(id) {
                    doc.set_encoding(file.encoding);
                    doc.set_line_ending(file.line_ending);
                }
                self.editors.insert(id, bp_editor::Editor::new(&file.text));
                self.mark_in_step(id, &path2);
                // Loaded, but big enough that whole-document tools -- Find,
                // the data operations, statistics -- will be noticeably slow.
                // Said once, at open, rather than left for the user to
                // discover as a hang. A file past the editable limit never
                // gets here: `load` refuses it, and the arm below shows why.
                if file.size_class() == bp_buffer::SizeClass::Large {
                    self.error = Some(format!(
                        "Large file ({} MiB): whole-document tools may be slow",
                        file.bytes_on_disk / (1024 * 1024)
                    ));
                }
            }
            // The error types already render a message naming the file and
            // what to do about it, which is exactly what the status bar wants.
            // That includes a file too large to edit, which opens no tab.
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// Outcome of a save attempt, so callers can tell the three cases apart.
    ///
    /// A close-on-quit flow must not treat "needs a path" or "the disk
    /// refused" as success and then discard the buffer.
    pub(crate) fn save_document(&mut self, id: DocumentId, path: Option<PathBuf>) -> SaveResult {
        self.error = None;
        let Some(doc) = self.workspace.get(id) else {
            return SaveResult::Saved;
        };

        let Some(target) = path.or_else(|| doc.path().map(Path::to_path_buf)) else {
            return SaveResult::NeedsPath;
        };

        // `Preserve`, not the document's declared line ending: a save must
        // return the bytes the user was given. Converting here is what made
        // every save quietly rewrite the minority convention in a mixed
        // document, with nothing on screen to say it had happened. Format ▸
        // LF / CRLF is where a conversion is asked for, and it asks first.
        //
        // Both copies -- the text and its encoded bytes -- are wiped when
        // dropped (release plan M1-20). For an encrypted document they are
        // the plaintext the seal exists to protect, and they are the two
        // copies this code owns outright; see SECURITY_MODEL.md for the
        // copies it cannot reach.
        let text = zeroize::Zeroizing::new(self.text_of(id));
        let bytes = match try_encode(&text, doc.encoding(), LineEndingPolicy::Preserve) {
            Ok(bytes) => zeroize::Zeroizing::new(bytes),
            Err(EncodeError::Unrepresentable(what)) => {
                self.error = Some(format!("not saved: {what}"));
                return SaveResult::Unrepresentable(what);
            }
        };

        // An encrypted document stays encrypted. The passphrase is held for
        // the session precisely so this does not ask again on every save --
        // a prompt on every Ctrl+S trains the user to type it without
        // reading, which is worse than holding it in memory.
        let bytes = match self.passphrases.get(&id) {
            None => bytes,
            Some(passphrase) => {
                match bp_crypto::seal(&bytes, passphrase, bp_crypto::SealOptions::default()) {
                    Ok(sealed) => zeroize::Zeroizing::new(sealed),
                    Err(e) => {
                        // Refused rather than falling back to plaintext. A
                        // save that silently wrote the document in clear
                        // would be the worst failure this program has.
                        self.error = Some(format!("not saved -- {e}"));
                        return SaveResult::Failed;
                    }
                }
            }
        };

        match atomic_write(&target, &bytes, SaveOptions::default()) {
            Ok(_) => {
                let mut moved_to = None;
                if let Some(doc) = self.workspace.get_mut(id) {
                    if doc.path() != Some(target.as_path()) {
                        moved_to = Some(doc.security());
                    }
                    doc.set_path(target.clone());
                    // Only now, after a verified write, is the document clean.
                    doc.record_disk_save(now());
                }
                // A first save or a Save As: the profile goes with the
                // document to its new file. The old file keeps its own entry
                // -- it still exists, with the same text in it.
                if let Some(security) = moved_to {
                    self.keep_profile(&target, security);
                }
                // Both forms of journal go once the work is on disk. The
                // sealed one is keyed by path rather than by session id, so
                // it has to be discarded by path.
                let _ = self.journal.discard(id.get());
                let _ = self.journal.discard_sealed_for(&target);
                self.checkpointed.remove(&id);
                // Re-stamp from what we just wrote, or our own save would
                // look like somebody else's change on the next poll.
                self.mark_in_step(id, &target);
                SaveResult::Saved
            }
            Err(e) => {
                self.error = Some(e.to_string());
                SaveResult::Failed
            }
        }
    }

    /// Write `id`'s current text to `target` without adopting it.
    ///
    /// Deliberately not a call into [`save_document`](Self::save_document)
    /// with a path. A copy is a different operation that happens to write the
    /// same bytes, and the three things it must *not* do are exactly the
    /// three that one does: it must not move the document's path, must not
    /// mark it clean, and must not touch the recent-files list or the disk
    /// stamp. Sharing the code would mean sharing all four behaviours and
    /// then subtracting three, which is how Save Copy quietly becomes
    /// Save As.
    ///
    /// Only the encoding is shared, because bytes written differently from a
    /// real save would make the copy a different file from the original.
    pub(crate) fn save_copy(&mut self, id: DocumentId, target: &Path) -> SaveResult {
        self.error = None;
        let Some(doc) = self.workspace.get(id) else {
            return SaveResult::Saved;
        };

        // The same policy as a real save, for the same reason the encoding is
        // shared: bytes written differently from the original would make the
        // copy a different file.
        let bytes = match try_encode(
            &self.text_of(id),
            doc.encoding(),
            LineEndingPolicy::Preserve,
        ) {
            Ok(bytes) => bytes,
            Err(e) => {
                self.error = Some(format!("copy not saved: {e}"));
                return SaveResult::Failed;
            }
        };

        match atomic_write(target, &bytes, SaveOptions::default()) {
            Ok(_) => {
                // Reported, because a write that leaves no trace anywhere in
                // the window is indistinguishable from one that did not
                // happen -- the tab does not change, and neither does the
                // save state.
                self.error = Some(format!("copy written to {}", target.display()));
                SaveResult::Saved
            }
            Err(e) => {
                self.error = Some(e.to_string());
                SaveResult::Failed
            }
        }
    }

    /// Report the active document's statistics into the status bar.
    ///
    /// A one-off on a menu click, which is what makes reading the whole
    /// document acceptable here: `active_text()` copies it, so this must
    /// never move onto the typing path or into `refresh` (see the trap in
    /// R011).
    pub(crate) fn report_statistics(&mut self) {
        let stats = bp_semantic::statistics(&self.active_text());
        self.error = Some(format!(
            "{} words, {} lines, {} paragraphs, {} characters ({} without spaces)",
            stats.words,
            stats.lines,
            stats.paragraphs,
            stats.characters,
            stats.characters_no_whitespace,
        ));
    }

    /// The active document as the bytes it would be written to disk as.
    ///
    /// The encoded form rather than the buffer, because a digest is a claim
    /// about a *file*: hashing the buffer would print a digest that
    /// `sha256sum` disagrees with for every document carrying a byte-order
    /// mark, and a user comparing the two would conclude their file had been
    /// tampered with.
    ///
    /// [`LineEndingPolicy::Preserve`], the same policy `save_document` uses,
    /// and the two have to agree: a digest taken under one policy and a file
    /// written under the other would differ for exactly the mixed-ending
    /// documents somebody takes a digest to settle.
    ///
    /// A one-off on a menu click, like `report_statistics` -- it copies the
    /// whole document twice over and must never move onto the typing path.
    pub(crate) fn active_bytes(&self) -> Result<Vec<u8>, String> {
        let Some(doc) = self.workspace.active() else {
            return Err("there is no document to read".to_owned());
        };
        // A document that cannot be encoded has no file form to hash, and a
        // digest of some other encoding's bytes would describe a file that
        // does not exist.
        try_encode(
            &self.active_text(),
            doc.encoding(),
            LineEndingPolicy::Preserve,
        )
        .map_err(|e| e.to_string())
    }

    /// Whether the active document differs from whatever is on disk.
    ///
    /// A never-saved document counts, which is why this is not simply
    /// `is_dirty` at the call site: "there is no file on disk" and "the file
    /// on disk is older than this" are the same fact as far as a digest or a
    /// signature over these bytes is concerned.
    fn active_differs_from_disk(&self) -> bool {
        self.workspace
            .active()
            .is_some_and(|doc| doc.is_dirty() || doc.path().is_none())
    }

    /// Scan the active document for credentials, reporting into the status
    /// bar and handing the findings back for a fuller listing.
    ///
    /// The findings are positions and classifications; `bp-secrets` refuses
    /// to carry the matched text and this returns nothing that would
    /// reintroduce it.
    pub(crate) fn scan_for_secrets(&mut self) -> Vec<bp_secrets::Finding> {
        let findings = bp_secrets::scan(&self.active_text());
        self.error = Some(secret_scan_summary(&findings));
        // The count only. Where the findings are is in the document the user
        // is looking at; putting the offsets in a file beside it would make
        // the history a map to the credentials -- which is the one thing
        // `bp-secrets` is built never to hold.
        self.record_security_event(bp_audit::Event::SecretScanFinished {
            findings: u32::try_from(findings.len()).unwrap_or(u32::MAX),
        });
        findings
    }

    /// Plan a redaction of whatever a scan of the active document finds.
    ///
    /// Scans, says what it found, and hands back a plan for the caller to get
    /// consent for -- redaction destroys text, so the shell asks before it
    /// happens rather than offering undo afterwards as the whole answer.
    ///
    /// `None` means there is nothing this can act on, and the status bar
    /// already says which of the reasons it is.
    pub(crate) fn plan_redaction(&mut self) -> Option<RedactionPlan> {
        let text = self.active_text();
        let findings = bp_secrets::scan(&text);
        let plan = RedactionPlan::from_scan(text, &findings);
        if plan.spans.is_empty() {
            self.error = Some(plan.refusal());
            return None;
        }
        self.error = Some(secret_scan_summary(&findings));
        Some(plan)
    }

    /// Say that the user declined a redaction they were shown.
    ///
    /// Not silence. Cancelling a file dialog leaves the status bar alone
    /// because nothing was claimed; here the bar is showing a scan summary
    /// and the user has just been asked a question, so the answer belongs on
    /// screen -- otherwise "I clicked Cancel" and "it did nothing" look the
    /// same.
    pub(crate) fn decline_redaction(&mut self, plan: &RedactionPlan) {
        self.error = Some(format!(
            "nothing was redacted — {} possible credential{} {} still in this document",
            plan.spans.len(),
            if plan.spans.len() == 1 { "" } else { "s" },
            if plan.spans.len() == 1 { "is" } else { "are" },
        ));
    }

    /// Carry out a redaction the user has agreed to, and check it.
    ///
    /// An ordinary edit, like a data operation: undoable, and nothing reaches
    /// disk until the user saves. That is the only honest shape for this.
    /// Redaction is irreversible *in the string it returns*, but the document
    /// is not the file, and quietly rewriting the file instead would destroy
    /// the user's only copy of text a scanner guessed about.
    ///
    /// Returns whether the document changed, so the caller knows whether to
    /// re-push the text into the widget.
    pub(crate) fn apply_redaction(&mut self, plan: &RedactionPlan) -> bool {
        // The offsets were measured when the question opened, and it is
        // answered later, from an event, with timers free to run in between.
        // ADR-0028 is built on the rule that stale
        // offsets destroy the wrong text and report success, and its own
        // `PastEnd` catches only the half of that which runs off the end --
        // an edit that left the length alone would slip straight through. So
        // the document is compared rather than trusted.
        if self.active_text() != plan.original {
            self.error = Some(
                concat!(
                    "nothing was redacted — the document changed while the ",
                    "dialog was open, so these positions no longer describe it; scan again"
                )
                .to_owned(),
            );
            return false;
        }

        let redacted = match bp_redaction::redact(&plan.original, &plan.spans, REDACTION_MODE) {
            Ok(redacted) => redacted,
            Err(e) => {
                self.error = Some(format!("nothing was redacted — {e}"));
                return false;
            }
        };

        // `verify` re-derives what was removed from the original rather than
        // being handed it, so checking costs nobody a variable holding a
        // secret. It answers in indices into `applied`; those become line
        // numbers here and never text, which is the whole reason the type
        // reports indices in the first place.
        let survivors = bp_redaction::verify(&plan.original, &plan.spans, &redacted.text)
            .ok()
            .map(|verification| {
                verification
                    .surviving
                    .iter()
                    .filter_map(|index| redacted.applied.get(*index))
                    .map(|applied| line_at(&plan.original, applied.start))
                    .collect::<Vec<usize>>()
            });

        let written = redacted.applied.len();
        let bytes_removed = plan.original.len().saturating_sub(redacted.text.len());
        self.edit(redacted.text);
        self.error = Some(plan.outcome(written, survivors.as_deref()));
        // How much went, never what. The removed text is the text somebody
        // chose to destroy, which makes it the most sensitive thing this
        // product ever handles -- `Event::RedactionApplied` carries a count
        // and a byte total for that reason and this passes it nothing else.
        self.record_security_event(bp_audit::Event::RedactionApplied {
            spans: u32::try_from(written).unwrap_or(u32::MAX),
            bytes_removed: bytes_removed as u64,
        });
        true
    }

    /// Report what identifying metadata the active document carries.
    ///
    /// Returns the body of the listing to show, or `None` when the status bar
    /// has already said everything there is to say.
    pub(crate) fn inspect_metadata(&mut self) -> Option<String> {
        let container = container_of(self.workspace.active().and_then(Document::path));
        let findings = bp_redaction::inspect(&self.active_text());
        self.error = Some(metadata_summary(&findings, container));
        (!findings.is_empty() || !container.hidden().is_empty())
            .then(|| metadata_report(&findings, container))
    }

    /// Take a digest of the active document.
    ///
    /// Returns the body of the report to show, or `None` when the document
    /// cannot be encoded -- in which case the status bar already says why.
    /// The digest is not put in the status bar: it elides, and half a
    /// checksum compares equal to nothing.
    pub(crate) fn hash_active_document(&mut self) -> Option<String> {
        let bytes = match self.active_bytes() {
            Ok(bytes) => bytes,
            Err(e) => {
                self.error = Some(format!("cannot hash this document — {e}"));
                return None;
            }
        };
        let digest = bp_crypto::hash_document(&bytes, bp_crypto::HashAlgorithm::Sha256);
        self.error = Some(format!(
            "{} taken over {} bytes",
            digest.algorithm().name(),
            bytes.len()
        ));
        Some(hash_report(
            &digest,
            bytes.len(),
            self.active_differs_from_disk(),
        ))
    }

    /// Check the active document against the `.sig` sidecar beside it.
    ///
    /// The sidecar is found by `bp_integrity::sidecar_path` -- `document.ext`
    /// is signed by `document.ext.sig` -- rather than asked for. Two callers
    /// that disagree about that name produce a document one half of the
    /// product believes is unsigned, which is why the convention is a
    /// function in one crate and not a rule left to each caller.
    ///
    /// `expect` chooses between the two questions ADR-0026 keeps apart.
    /// [`Expectation::AnySigner`](bp_integrity::Expectation::AnySigner)
    /// answers "has this changed since the key named in the sidecar signed
    /// it", which anybody who alters a document can pass by re-signing with a
    /// key of their own; naming a key answers "was it *this* signer", and is
    /// the only way `SignedByAnotherKey` can arise at all.
    ///
    /// Returns whether the check passed, so the caller knows whether naming a
    /// key could still change the answer. It can only turn a pass into
    /// `SignedByAnotherKey`; a failure is a failure whatever key is offered,
    /// because `verify_file` tests the signature against the sidecar's own
    /// key before it compares that key with anybody's expectation.
    pub(crate) fn verify_signature(&mut self, expect: &bp_integrity::Expectation) -> bool {
        let Some(path) = self
            .workspace
            .active()
            .and_then(Document::path)
            .map(Path::to_path_buf)
        else {
            self.error = Some(
                "cannot verify — this document has never been saved, so there is no file \
                 for a signature to sit beside"
                    .to_owned(),
            );
            return false;
        };

        // A missing *document* is an error and a missing *signature* is a
        // verdict -- and a failing one. `bp-integrity` draws that line, and
        // the shell does not get to soften it: an unsigned document must not
        // come back as anything but a failure, or deleting a file would be a
        // way to pass the check.
        let verification = match bp_integrity::verify_file(&path, expect) {
            Ok(verification) => verification,
            Err(e) => {
                self.error = Some(format!("cannot verify — {e}"));
                return false;
            }
        };

        let verified = verification.is_verified();
        // The flag rather than only the fact of a check: `Event` makes the
        // outcome part of the event for a reason, because a recorded
        // verification that omitted its answer reads as reassurance.
        self.record_security_event(bp_audit::Event::SignatureVerified { valid: verified });
        // `explain` rather than a second set of sentences here. Every verdict
        // words itself in `bp-integrity`, beside the logic that produces it,
        // so the status bar and any other surface say the same thing about
        // the same answer -- and so the five outcomes ADR-0026 exists to keep
        // apart stay five, rather than collapsing into "does not verify" on
        // the way to a screen.
        let mut message = format!(
            "{} {}",
            if verified { "✓" } else { "✗" },
            verification.explain()
        );

        if verified && *expect == bp_integrity::Expectation::AnySigner {
            // The caveat `Expectation::AnySigner` carries in its own doc
            // comment. Without it a tick reads as "this is from who you
            // think", which is a claim nothing here checked.
            message.push_str(
                " — no key was named, so this says only that the document has not changed \
                 since the key above signed it; anyone who alters a document can re-sign it \
                 with a key of their own",
            );
        }
        if self.active_differs_from_disk() {
            // The shell's own contribution, and the only sentence here it is
            // in a position to write: the verdict is about the file, and the
            // buffer on screen is not that file.
            message.push_str(
                " — this document has unsaved changes, so these are not the bytes anybody \
                 signed; the check was made against the file on disk",
            );
        }
        self.error = Some(message);
        verified
    }

    /// Read the public key a signature is claimed to have been made with.
    ///
    /// Reports its own failure, because "could not read the key" and "the
    /// signature does not match" are answers with different fixes and the
    /// second must never be shown for the first.
    pub(crate) fn expected_signer(&mut self, key_path: &Path) -> Option<bp_integrity::Expectation> {
        match read_verifying_key(key_path) {
            Ok(key) => Some(bp_integrity::Expectation::Key(key)),
            Err(e) => {
                self.error = Some(format!("could not read the key — {e}"));
                None
            }
        }
    }

    /// Insert a date or time stamp at the caret.
    ///
    /// The clock is read here rather than in `bp-naming`, which is pure and
    /// never reads one -- so the shell decides *when*, and the crate only
    /// decides how that instant is written.
    ///
    /// Caret work, so it needs the editor we own. Under `TextInput` there is
    /// no readable caret (ADR-0018) and the rows are disabled instead.
    pub(crate) fn insert_stamp(&mut self, index: usize) -> bool {
        let Some(stamp) = bp_naming::Stamp::all().get(index).copied() else {
            return false;
        };
        let text = bp_naming::render(stamp, now());
        let Some(editor) = self.active_editor_mut() else {
            return false;
        };
        editor.insert(&text);
        self.mark_edited();
        true
    }

    /// Say something only if nothing more important is already being said.
    ///
    /// Every caller of `record_security_event` has just finished an operation
    /// that put its own result in the status bar, and that result is what the
    /// user asked for. A notice about the *history* of the operation must not
    /// take the place of the operation's own answer -- "3 possible
    /// credentials" is the thing somebody clicked for, and losing it to a
    /// line about sealing would be the log making the product worse.
    fn note_quietly(&mut self, message: String) {
        if self.error.is_none() {
            self.error = Some(message);
        }
    }

    /// Write `event` to the security history, if the profile permits it.
    ///
    /// Never fails the operation it describes. A security capability that
    /// stopped working because its *log* could not be written would be a
    /// worse product than one whose log has a gap -- so a refusal or an I/O
    /// error becomes a status-bar line and the capability carries on. The
    /// one refusal worth a notice is `NoSealer`, which tells the user how to
    /// fix it (encrypt the document); `ProfileForbidsIt` is the profile
    /// doing its job and says nothing.
    ///
    /// The policy passed is the one *in force* -- `self.policy()` already
    /// resolves Privacy Mode -- because a caller that passes the document's
    /// own profile is a caller Privacy Mode does not reach.
    pub(crate) fn record_security_event(&mut self, event: bp_audit::Event) {
        let policy = self.policy();
        // Opened per event rather than held. `AuditLog::open` reads and
        // checks the existing file, which is what makes its sequence
        // trustworthy; a handle kept across a session would be a sequence
        // that stopped agreeing with the file the moment anything else
        // appended to it. Security events are rare enough to afford it.
        let sealer = self
            .workspace
            .active_id()
            .and_then(|id| self.passphrases.get(&id))
            .and_then(|p| bp_audit::PassphraseSealer::new(p).ok());

        let mut log = match bp_audit::AuditLog::open(
            self.audit_path.clone(),
            sealer.as_ref().map(|s| s as &dyn bp_audit::Sealer),
        ) {
            Ok(log) => log,
            Err(e) => {
                self.note_quietly(format!("security history unavailable -- {e}"));
                return;
            }
        };

        let document = self
            .workspace
            .active_id()
            .map(|id| bp_audit::DocumentId::new(id.get()));

        match log.append(
            now(),
            document,
            event,
            policy,
            sealer.as_ref().map(|s| s as &dyn bp_audit::Sealer),
        ) {
            Ok(bp_audit::Appended::Persisted(_)) => {}
            Ok(bp_audit::Appended::NotWritten(reason)) => {
                if let Some(notice) = reason.notice() {
                    self.note_quietly(notice.to_owned());
                }
            }
            Err(e) => self.note_quietly(format!("security history not written -- {e}")),
        }
    }

    /// The security history, as a report to show.
    ///
    /// Returns `None` when it cannot be read, having put the reason in the
    /// status bar -- a sealed history with no passphrase to hand is the
    /// common case, and it is a real answer rather than a failure.
    pub(crate) fn security_history(&mut self) -> Option<String> {
        let sealer = self
            .workspace
            .active_id()
            .and_then(|id| self.passphrases.get(&id))
            .and_then(|p| bp_audit::PassphraseSealer::new(p).ok());

        match bp_audit::AuditLog::open(
            self.audit_path.clone(),
            sealer.as_ref().map(|s| s as &dyn bp_audit::Sealer),
        ) {
            Ok(log) => {
                self.error = Some(format!("{} security events", log.records().len()));
                Some(crate::audit::history_report(&log))
            }
            Err(e) => {
                self.error = Some(format!("cannot read the security history -- {e}"));
                None
            }
        }
    }

    /// The active document's security policy.
    ///
    /// Falls back to the default when there is no active document, so callers
    /// never have to choose a policy for themselves -- picking one at a call
    /// site is how a permissive default gets applied to a document that asked
    /// for something stricter.
    /// Record text the user copied or cut in this window, if the active
    /// document's profile allows a history at all.
    pub(crate) fn record_copy(&mut self, text: &str) -> bool {
        let policy = self.policy().clipboard;
        self.clips
            .observe(text, policy, bp_clipboard::Origin::OpenDocument, false)
    }

    /// Whether a copy out of the active document should be kept out of the
    /// OS clipboard history and cloud sync.
    pub(crate) fn keep_clipboard_private(&self) -> bool {
        bp_clipboard::keep_out_of_os_history(&self.policy())
    }

    pub(crate) fn policy(&self) -> bp_security::Policy {
        // Through `policy_under`, always. Reading a document's own policy
        // directly is how a caller ends up outside Privacy Mode's reach.
        self.security().policy_under(self.privacy)
    }

    pub(crate) fn security(&self) -> bp_security::Security {
        self.workspace
            .active()
            .map_or_else(bp_security::Security::default, Document::security)
    }

    /// Give the active document a profile, and make the world match it.
    ///
    /// Setting a profile is not only a note on the document: tightening one
    /// obliges us to remove what the looser one already permitted. The
    /// journal is handled by the next checkpoint, which deletes a refused
    /// document's file; the clipboard is shared and has to be dealt with
    /// here.
    pub(crate) fn set_security(&mut self, security: bp_security::Security) {
        let Some(id) = self.workspace.active_id() else {
            return;
        };
        let Some(doc) = self.workspace.get_mut(id) else {
            return;
        };
        let previous = doc.set_security(security);
        if previous == security {
            return;
        }
        // The journal's form follows the profile, so the recorded one is no
        // longer current whatever happens below.
        self.checkpointed.remove(&id);
        // Remembered for the file, so reopening a Confidential document is
        // Confidential. An untitled document has no file to remember it
        // for yet; its first save does it.
        if let Some(path) = doc.path().map(Path::to_path_buf) {
            self.keep_profile(&path, security);
        }

        let policy = security.policy_under(self.privacy);
        if self.clips.enforce(policy.clipboard) {
            self.error = Some("clipboard history cleared to match this profile".to_owned());
        }

        // Recorded against the *new* profile, which is the one that decides
        // whether it may be kept. Recording a tightening under the old, looser
        // profile would write the line the user just asked to stop writing;
        // recording a loosening under the old, stricter one would lose the
        // single most interesting event a history can hold.
        self.record_security_event(bp_audit::Event::SecurityProfileChanged {
            from: bp_audit::ProfileLabel::of(&previous),
            to: bp_audit::ProfileLabel::of(&security),
        });

        // The journal for *this* document only. A refused checkpoint deletes
        // its file, so asking for one now is what turns the profile change
        // into an actual deletion rather than waiting up to the autosave
        // interval with plaintext still on disk.
        let entry = bp_history::Checkpoint {
            path: doc_path(&self.workspace, id),
            name: self.display_name(id),
            text: self.text_of(id).to_owned(),
            written_at: bp_history::now_unix(),
        };
        let passphrase = self.passphrases.get(&id).map(|p| p.to_string());
        if let Ok(bp_history::Written::Refused(refusal)) =
            self.journal
                .checkpoint(id.get(), &entry, policy.recovery, passphrase.as_deref())
            && let Some(message) = refusal.notice()
        {
            self.error = Some(message.to_owned());
        }
    }

    /// Whether `id` is an encrypted document.
    pub(crate) fn is_encrypted(&self, id: DocumentId) -> bool {
        self.passphrases.contains_key(&id)
    }

    /// Begin opening `path`, asking for a passphrase if it is encrypted.
    ///
    /// Read as bytes rather than through `bp_files::load`, which decodes text
    /// and would mangle ciphertext on the way in.
    ///
    /// An encrypted file does **not** become a tab until it is unlocked. A tab
    /// nobody can read is worse than no tab: it looks like an empty document,
    /// and saving it would write emptiness over the real one.
    pub(crate) fn open_maybe_encrypted(&mut self, path: PathBuf) -> bool {
        match std::fs::read(&path) {
            Ok(bytes) if bp_crypto::is_bpadx(&bytes) => {
                self.ask = Some(crate::passphrase::Ask::Unlock(path));
                self.passphrase_status.clear();
                true
            }
            // Not encrypted, or unreadable -- either way `open` handles it and
            // reports properly.
            _ => {
                self.open(path);
                false
            }
        }
    }

    /// Open every file in `paths`, as File ▸ Open with several chosen does.
    ///
    /// Returns whether the passphrase bar should open. Only one passphrase
    /// can be asked at a time, so the first encrypted file asks and any
    /// further ones are named in the status bar to open one by one --
    /// better than guessing that one passphrase fits them all. What each
    /// open reports is kept, rather than each wiping the last.
    pub(crate) fn open_many(&mut self, paths: Vec<PathBuf>) -> bool {
        let mut notices = Vec::new();
        let mut asking = false;
        let mut locked_later = Vec::new();
        for path in paths {
            let encrypted = std::fs::read(&path).is_ok_and(|bytes| bp_crypto::is_bpadx(&bytes));
            if encrypted && asking {
                locked_later.push(path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                ));
                continue;
            }
            asking |= self.open_maybe_encrypted(path);
            notices.extend(self.error.take());
        }
        if !locked_later.is_empty() {
            notices.push(format!(
                "also encrypted, open separately: {}",
                locked_later.join(", ")
            ));
        }
        self.error = (!notices.is_empty()).then(|| notices.join("; "));
        asking
    }

    /// Make the next tab active, or the previous one, wrapping at either
    /// end (Ctrl+Tab and Ctrl+Shift+Tab).
    pub(crate) fn cycle_tab(&mut self, forward: bool) {
        let ids: Vec<DocumentId> = self.workspace.iter().map(Document::id).collect();
        let Some(at) = self
            .workspace
            .active_id()
            .and_then(|active| ids.iter().position(|&id| id == active))
        else {
            return;
        };
        let next = if forward {
            (at + 1) % ids.len()
        } else {
            (at + ids.len() - 1) % ids.len()
        };
        self.workspace.set_active(ids[next]);
    }

    /// Answer whatever the passphrase bar was asking.
    ///
    /// Returns whether the bar should stay open: a wrong passphrase keeps it
    /// up with a message, because closing it would make a typo look like a
    /// refusal to open the file at all.
    pub(crate) fn answer_passphrase(&mut self, entered: &str) -> bool {
        use crate::passphrase::Ask;
        let Some(ask) = self.ask.take() else {
            return false;
        };

        match ask {
            Ask::Unlock(path) => self.unlock(path, entered),
            Ask::Set { id, target } => {
                if entered.is_empty() {
                    // `bp_crypto::seal` refuses this too, but saying so here
                    // means the user is told before typing it twice.
                    self.passphrase_status = "a passphrase is required".to_owned();
                    self.ask = Some(Ask::Set { id, target });
                    return true;
                }
                self.passphrase_status.clear();
                self.ask = Some(Ask::Confirm {
                    id,
                    target,
                    first: zeroize::Zeroizing::new(entered.to_owned()),
                });
                true
            }
            Ask::Confirm { id, target, first } => {
                if first.as_str() != entered {
                    // Back to the first question, not the second. Asking to
                    // confirm again would compare against a passphrase the
                    // user may have already decided was the mistake.
                    self.passphrase_status = "those did not match -- start again".to_owned();
                    self.ask = Some(Ask::Set { id, target });
                    return true;
                }
                self.encrypt(id, &target, &first)
            }
        }
    }

    /// Decrypt `path` and open it as a tab.
    fn unlock(&mut self, path: PathBuf, passphrase: &str) -> bool {
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) => {
                self.error = Some(e.to_string());
                return false;
            }
        };

        match bp_crypto::open(&bytes, passphrase) {
            Ok(plain) => {
                // Lossy rather than refusing: a document that decrypted and
                // authenticated is the user's own text, and refusing to show
                // it because one byte is not UTF-8 would strand it inside a
                // file only this program can open.
                let text = String::from_utf8_lossy(&plain).into_owned();
                let id = self.workspace.open_path(path.clone(), created_at(&path));
                self.give_default_profile(id);
                self.recall_profile(id, &path);

                // Unsaved work from a crashed session, if there is any. This
                // is the only moment it can be read: the journal is sealed
                // with this passphrase, which nothing had until now. Recovery
                // therefore never needs a prompt of its own -- a program
                // asking for a passphrase unprompted is the habit that makes
                // phishing work.
                let recovered = self.journal.sealed_pending(&path, passphrase);
                let text = match &recovered {
                    Some(checkpoint) => checkpoint.text.clone(),
                    None => text,
                };

                self.editors.insert(id, bp_editor::Editor::new(&text));
                self.passphrases
                    .insert(id, zeroize::Zeroizing::new(passphrase.to_owned()));
                self.mark_in_step(id, &path);
                self.passphrase_status.clear();

                if recovered.is_some() {
                    // Marked modified, because what is on screen is *not*
                    // what is in the file. Leaving it clean would let the
                    // user close the tab and lose the recovered work without
                    // being asked.
                    if let Some(doc) = self.workspace.get_mut(id) {
                        doc.mark_modified();
                    }
                    self.error = Some("recovered unsaved work from a previous session".to_owned());
                } else {
                    self.error = None;
                }
                false
            }
            Err(e) => {
                // The bar stays up. A wrong passphrase is the expected
                // failure, and it is indistinguishable from a damaged file --
                // see `bp_crypto`'s docs for why nothing can do better.
                self.passphrase_status = e.to_string();
                self.ask = Some(crate::passphrase::Ask::Unlock(path));
                true
            }
        }
    }

    /// Seal `id`'s text to `target` and let the tab adopt it.
    fn encrypt(&mut self, id: DocumentId, target: &Path, passphrase: &str) -> bool {
        let sealed = match bp_crypto::seal(
            self.text_of(id).as_bytes(),
            passphrase,
            bp_crypto::SealOptions::default(),
        ) {
            Ok(sealed) => sealed,
            Err(e) => {
                self.passphrase_status = e.to_string();
                return true;
            }
        };

        match atomic_write(target, &sealed, SaveOptions::default()) {
            Ok(_) => {
                // The tab adopts the encrypted file, so later saves stay
                // encrypted. Recording the passphrase is what makes the
                // document encrypted as far as the rest of the shell is
                // concerned -- there is no second flag.
                self.passphrases
                    .insert(id, zeroize::Zeroizing::new(passphrase.to_owned()));
                let mut security = None;
                if let Some(doc) = self.workspace.get_mut(id) {
                    doc.set_path(target.to_path_buf());
                    doc.record_disk_save(now());
                    security = Some(doc.security());
                }
                if let Some(security) = security {
                    self.keep_profile(target, security);
                }
                self.mark_in_step(id, target);
                self.passphrase_status.clear();
                self.error = Some(format!("encrypted to {}", target.display()));
                false
            }
            Err(e) => {
                self.passphrase_status = e.to_string();
                true
            }
        }
    }

    /// Turn Privacy Mode on or off, and make the world match.
    ///
    /// Switching it on has to *act*, not merely be recorded: a mode that only
    /// governed future writes would leave the clipboard history and the
    /// journals gathered a moment ago exactly where they were, which is the
    /// opposite of what somebody switching it on wants.
    pub(crate) fn set_privacy(&mut self, privacy: bp_security::Privacy) {
        if self.privacy == privacy {
            return;
        }
        self.privacy = privacy;
        // Every document's journal changes form (or goes) without an edit,
        // so none of the recorded checkpoints is current any more: turning
        // the mode off must journal unchanged work again at the next
        // interval, not wait for the next keystroke.
        self.checkpointed.clear();
        if !privacy.is_on() {
            // Turning it off restores each document's own profile. Nothing to
            // clean up -- the clamp only ever removed permissions.
            //
            // Recorded after the field changes, so the policy deciding whether
            // this line may be kept is the one now in force. Leaving Privacy
            // Mode is the event that re-enables everything it was switched on
            // to stop, which is exactly why it is worth a line.
            self.record_security_event(bp_audit::Event::PrivacyModeLeft);
            return;
        }
        self.record_security_event(bp_audit::Event::PrivacyModeEntered);

        let cleared = self.clips.enforce(self.policy().clipboard);
        // Every document, not only the active one: the mode is session-wide,
        // and a journal left behind for a background tab is exactly what it
        // was switched on to prevent.
        self.checkpoint_all();
        self.error = Some(if cleared {
            "Privacy Mode on -- clipboard history cleared and journals removed".to_owned()
        } else {
            "Privacy Mode on".to_owned()
        });
    }

    /// Which document the tab context menu's rows should act on.
    ///
    /// The right-clicked tab, or the active one when the menu was not opened
    /// from a tab -- so the same ids serve a keyboard route later without the
    /// rows changing meaning.
    pub(crate) fn tab_context_id(&self) -> Option<DocumentId> {
        self.tab_context.or_else(|| self.workspace.active_id())
    }

    /// What the tab context menu needs to know: how many other tabs are open,
    /// and whether the one under the pointer has a path to copy.
    pub(crate) fn tab_context_shape(&self) -> (usize, bool) {
        let target = self.tab_context_id();
        let others = self.workspace.len().saturating_sub(1);
        let has_path = target
            .and_then(|id| self.workspace.get(id))
            .and_then(Document::path)
            .is_some();
        (others, has_path)
    }

    /// Move the caret to the line the user typed, and say what happened.
    ///
    /// `Editor::go_to_line` moves somewhere sensible *and* reports whether
    /// the line existed, so a number past the end lands at the last line and
    /// says why rather than doing nothing and looking broken.
    ///
    /// Returns where the caret went, in characters, for `dispatch::select`
    /// to put either view's caret there.
    pub(crate) fn go_to_line(&mut self, text: &str) -> Option<std::ops::Range<usize>> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            self.goto_status.clear();
            return None;
        }
        // Parsed rather than trusted: `input-type: number` is a hint to the
        // keyboard, not a guarantee from the backend.
        let Ok(line) = trimmed.parse::<usize>() else {
            self.goto_status = "not a line number".to_owned();
            return None;
        };
        let editor = self.active_editor_mut()?;
        let existed = editor.go_to_line(line);
        let cursor = editor.cursor();
        let total = editor.buffer().len_lines();
        self.goto_status = if existed {
            String::new()
        } else {
            format!("there are only {total} lines")
        };
        Some(cursor..cursor)
    }

    /// True if `id`'s file changed underneath us since we last read or wrote
    /// it — meaning a save would overwrite somebody else's work.
    pub(crate) fn would_overwrite_external_change(&self, id: DocumentId) -> bool {
        let Some(path) = self.workspace.get(id).and_then(Document::path) else {
            return false;
        };
        let Some(stamp) = self.stamps.get(&id) else {
            return false;
        };
        bp_files::check(path, *stamp) == DiskState::Modified
    }

    /// Where a file dialog should open.
    ///
    /// The active document's own folder, else the user's Documents folder.
    /// Explicitly *not* the process working directory, which is wherever the
    /// binary happened to be launched from -- during testing that was a git
    /// checkout, and notes were saved straight into it.
    pub(crate) fn dialog_directory(&self) -> PathBuf {
        self.workspace
            .active()
            .and_then(Document::path)
            .and_then(Path::parent)
            .filter(|p| !p.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .or_else(documents_dir)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    /// A default filename for Save As, in the grammar from ADR-0003.
    ///
    /// Prefers a title extracted from the document's own content — that is
    /// the "Semantic Filing Apparatus" doing its job — and falls back to the
    /// document's title when there is nothing to extract.
    ///
    /// Free in the folder the dialog opens in (release plan M1-17): a name
    /// that is taken gets the grammar's collision suffix, so accepting the
    /// suggestion never offers to overwrite some other file. The document's
    /// own file does not count as taken -- saving over it is what saving is.
    pub(crate) fn suggested_filename(&self, id: DocumentId) -> String {
        let suggested = self.semantic_filename(id).unwrap_or_else(|| {
            self.workspace.get(id).map_or_else(
                || "Untitled.txt".to_owned(),
                |doc| SemanticName::new(doc.title(), doc.created_at().date(), "txt").to_filename(),
            )
        });
        let Some(name) = SemanticName::parse(&suggested) else {
            return suggested;
        };
        let dir = self.dialog_directory();
        let own = self.workspace.get(id).and_then(Document::path);
        name.resolve_collision(|candidate| {
            let path = dir.join(candidate);
            path.exists() && Some(path.as_path()) != own
        })
        .map_or(suggested, |free| free.to_filename())
    }

    /// Where Save as New Revision would write `id`: the same name with the
    /// next revision suffix, in the same folder, and free.
    ///
    /// A name already in the grammar keeps its title and date and gains or
    /// bumps its `_vN`. Any other name is brought into the grammar first --
    /// `notes.txt` becomes `notes_v2_<date>.txt` -- because a revision
    /// suffix has nowhere defined to go on a name without the grammar's
    /// date anchor. `None` for a document with no file: there is nothing to
    /// be a revision of.
    pub(crate) fn next_revision_path(&self, id: DocumentId) -> Option<PathBuf> {
        let doc = self.workspace.get(id)?;
        let path = doc.path()?;
        let dir = path.parent()?;
        let file_name = path.file_name()?.to_string_lossy();
        let current = SemanticName::parse(&file_name).unwrap_or_else(|| {
            let stem = path
                .file_stem()
                .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
            let extension = path
                .extension()
                .map_or_else(|| "txt".to_owned(), |e| e.to_string_lossy().into_owned());
            SemanticName::new(&stem, doc.created_at().date(), &extension)
        });
        let revision = current
            .revision()
            .map_or(bp_naming::FIRST_REVISION, |n| n.saturating_add(1));
        let next = current.with_revision(Some(revision)).with_collision(None);
        bp_files::resolve_in_dir(dir, &next)
    }

    /// Forget an encrypted document's passphrase.
    ///
    /// Called when a document closes. Holding it for the session is a
    /// deliberate trade; holding it past the document's life is just a leak.
    pub(crate) fn forget_passphrase(&mut self, id: DocumentId) {
        self.passphrases.remove(&id);
    }

    pub(crate) fn close(&mut self, id: DocumentId) {
        self.error = None;
        // Its checkpoints go with it. A clean document has none worth
        // keeping, and a dirty one reaches here only after the user chose
        // Don't Save -- leaving its journal behind offered that same work
        // for "recovery" at the next launch, and kept a plaintext copy of it
        // on disk in the meantime. Sealed checkpoints are keyed by path, as
        // in `save_document`.
        let _ = self.journal.discard(id.get());
        self.checkpointed.remove(&id);
        if let Some(path) = self.workspace.get(id).and_then(Document::path) {
            let _ = self.journal.discard_sealed_for(path);
        }
        if self.workspace.close(id).is_some() {
            self.editors.remove(&id);
            // Holding a passphrase for the session is a deliberate trade;
            // holding it past the document's life is just a leak.
            self.forget_passphrase(id);
        }
        // Never leave the user staring at an empty frame with no way back.
        if self.workspace.is_empty() {
            self.new_document();
        }
    }

    /// Replace the active document's text as a single undoable edit.
    ///
    /// What a data operation, Replace All or a line operation does, and what
    /// `TextInput` reports after every keystroke. Unchanged text is not an
    /// edit: a Format that found nothing to reformat must not mark a saved
    /// document unsaved.
    pub(crate) fn edit(&mut self, text: String) {
        let Some(id) = self.workspace.active_id() else {
            return;
        };
        let changed = self
            .editors
            .get_mut(&id)
            .is_some_and(|editor| editor.replace_all_text(&text));

        if changed && let Some(doc) = self.workspace.get_mut(id) {
            doc.mark_modified();
        }
    }

    /// `Ln 42, Col 18` when we own the caret; the line count when we do not.
    ///
    /// specs.md section 3 asks for line and column. Under `TextInput` the
    /// caret is reachable only through a property marked internal and
    /// undocumented, so the honest answer there is the thing we do know.
    pub(crate) fn cursor_label(&self) -> String {
        let position = self.position_label();
        if self.overwrite {
            // Shown only when on, as every editor with the mode does: the
            // exception is what needs pointing out, and `INS` on every
            // screen would be noise about the default.
            format!("{position} · OVR")
        } else {
            position
        }
    }

    fn position_label(&self) -> String {
        let lines = self.gutter_lines;
        // Both views: under `TextInput` the editor's caret follows the
        // widget's (`follow_widget_caret`), so it is as good an answer there.
        self.active_editor().map_or_else(
            || format!("{lines} lines"),
            |editor| {
                let position = editor.position();
                format!("Ln {}, Col {}", position.line, position.column)
            },
        )
    }

    /// Scroll so the caret is on screen, moving as little as possible.
    pub(crate) fn reveal_caret(&mut self) {
        let rows = self.visible_rows.max(1);
        let anchor = self.anchor;
        let Some(editor) = self.active_editor() else {
            return;
        };
        self.anchor = bp_editor::view::reveal_row(
            editor.buffer(),
            anchor,
            rows,
            editor.cursor(),
            editor.layout(),
        );
    }

    /// The layout the surface is drawing with.
    ///
    /// Taken from the active editor rather than assembled here, so the
    /// numbers that decide where the caret goes and the numbers that decide
    /// what is drawn are the same ones.
    pub(crate) fn layout(&self) -> bp_editor::view::Layout {
        self.active_editor().map_or_else(
            || bp_editor::view::Layout {
                wrap: bp_editor::wrap::Wrap::OFF,
                tab_width: self.indent.width,
            },
            bp_editor::Editor::layout,
        )
    }

    /// Tell the active editor how the surface is currently wrapping.
    ///
    /// Called before anything that depends on the layout, because the wrap
    /// width follows the window and the editor cannot see the window.
    pub(crate) fn sync_wrap(&mut self) {
        let wrap = if self.wrap_text && self.editor_view {
            bp_editor::wrap::Wrap::at(self.wrap_columns.max(1))
        } else {
            bp_editor::wrap::Wrap::OFF
        };
        let indent = self.indent;
        let overwrite = self.overwrite;
        if let Some(editor) = self.active_editor_mut() {
            editor.set_indent(indent);
            editor.set_wrap(wrap);
            editor.set_overwrite(overwrite);
        }
    }

    /// The Insert key.
    pub(crate) fn toggle_overwrite(&mut self) {
        self.overwrite = !self.overwrite;
        self.sync_wrap();
    }

    /// Note that the active document was edited in place, through the caret.
    pub(crate) fn mark_edited(&mut self) {
        if let Some(id) = self.workspace.active_id()
            && let Some(doc) = self.workspace.get_mut(id)
        {
            doc.mark_modified();
        }
    }
}

/// Status-bar save state, per specs.md section 3.
pub(crate) fn save_state_label(doc: &Document) -> String {
    if !doc.is_dirty() {
        return doc.last_disk_save().map_or_else(
            || "● Never saved".to_owned(),
            |t| format!("✓ Saved {}", clock(t.get())),
        );
    }

    // specs.md section 3's unsaved form: the recovery checkpoint and the last
    // real disk save, side by side and never confused for one another. The
    // whole reason `CheckpointTime` and `DiskSaveTime` are different types is
    // so this line cannot accidentally claim the work is safe.
    let mut parts = vec!["● Unsaved".to_owned()];
    if let Some(checkpoint) = doc.last_checkpoint() {
        parts.push(format!("Recovery {}", clock(checkpoint.get())));
    }
    if let Some(saved) = doc.last_disk_save() {
        parts.push(format!("Last disk save {}", clock(saved.get())));
    }
    parts.join(" │ ")
}

/// Local wall-clock time, falling back to UTC.
///
/// `now_local` can fail on Unix in a multithreaded process. A timestamp in
/// the wrong zone is a small wrong; refusing to record the save at all would
/// be a large one.
pub(crate) fn now() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

/// When the document at `path` was created, for naming (release plan M1-15).
///
/// The date an opened file was *opened* is not its creation date, and a
/// semantic rename used to stamp every file with the day somebody happened
/// to open it. `bp_naming::creation_date_for` weighs the evidence: a date
/// already in a semantic name, then the file's birth time, then its last
/// write. Only the date is ever read from `created_at`, so the time of day
/// stays now's; with no evidence at all, the date does too.
pub(crate) fn created_at(path: &Path) -> OffsetDateTime {
    let now = now();
    let local = |at: std::time::SystemTime| OffsetDateTime::from(at).to_offset(now.offset()).date();
    let meta = std::fs::metadata(path).ok();
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    // `created()` is an error on filesystems that keep no birth time, and
    // that is the evidence missing rather than a failure.
    bp_naming::creation_date_for(
        name,
        meta.as_ref().and_then(|m| m.created().ok()).map(local),
        meta.as_ref().and_then(|m| m.modified().ok()).map(local),
    )
    .map_or(now, |date| now.replace_date(date))
}

/// `8:05 PM`, as specs.md section 3 shows it.
///
/// Delegated to `bp-naming` rather than written here. The Insert menu puts
/// the same rendering into documents, and two copies of a twelve-hour clock
/// is two places for midnight to come out as `0:30 AM` -- in one of which
/// nobody would notice.
pub(crate) fn clock(t: OffsetDateTime) -> String {
    bp_naming::render(bp_naming::Stamp::Time, t)
}

/// How redaction replaces what it destroys.
///
/// [`bp_redaction::Replacement::Placeholder`], and the two it was chosen over
/// matter more than the one it is.
///
/// `Mask` with `MaskWidth::MatchOriginal` is out on its own terms: it draws
/// one character per character removed, which publishes the *length* of what
/// was there. For a name that is nearly nothing; for a PIN, a short token, or
/// an answer from a fixed set of options it is most of the secret, and a
/// redaction that discloses the secret is the failure ADR-0028 exists to
/// prevent.
///
/// `Remove` closes the gap, so the document reads as though the credential
/// had never been typed. That is right when the withholding itself should be
/// invisible. It is wrong here: the user is about to be told to save, to
/// rotate the key and to deal with every other copy, and a change they cannot
/// see on screen is one they will not act on.
///
/// `Placeholder` leaves `[REDACTED: AWS access key ID]`. The label is
/// `SecretKind::label` -- the name of the rule that matched, never what it
/// matched -- so the marker says what kind of thing was taken out without
/// putting it back. The cost is real, and is why this is a decision rather
/// than a default: the marker tells whoever reads the document afterwards
/// that an AWS key was there. That is a disclosure an author cleaning up
/// their own note can live with, and it is what makes a redaction findable
/// again later, which `bp_redaction::PLACEHOLDER` is public for.
const REDACTION_MODE: bp_redaction::Replacement = bp_redaction::Replacement::Placeholder;

/// A redaction the user has been shown and has not yet agreed to.
///
/// Holds the text the spans were measured against, so that a buffer edited
/// while the confirmation dialog was up is caught rather than redacted
/// against offsets that no longer describe it.
///
/// Carries no secret: `spans` are positions, `listing` is line numbers and
/// rule names, and `original` is the document the user is already looking at.
pub(crate) struct RedactionPlan {
    original: String,
    spans: Vec<bp_redaction::Span<'static>>,
    /// Line and kind per span, for the confirmation dialog. Never the text.
    listing: Vec<(usize, &'static str)>,
    /// Private-key blocks deliberately left alone. See [`RedactionPlan::from_scan`].
    key_blocks: usize,
    /// Findings whose position did not resolve to a range of this document.
    unlocatable: usize,
    /// Everything the scan reported, including what is not being redacted.
    total: usize,
}

impl RedactionPlan {
    /// Turn a scan of `original` into byte spans `bp_redaction` can act on.
    ///
    /// The conversion is the whole job, and it is not a formality.
    /// `bp_secrets::Finding` reports a 1-based line, a **character** column
    /// and a length in **characters**, because those are what a caret is
    /// moved to. `bp_redaction::Span` is in bytes, because those are what a
    /// `&str` can be sliced at. On any line holding a multi-byte character
    /// the two disagree, and ADR-0028 is explicit that a span in the wrong
    /// unit destroys the wrong text and reports success. A finding that does
    /// not resolve is counted and dropped rather than clamped, for the same
    /// reason `bp_redaction` refuses rather than clamping.
    ///
    /// A `PrivateKeyBlock` is deliberately left out. `bp-secrets` documents
    /// that its finding covers the `-----BEGIN ... PRIVATE KEY-----` marker
    /// only -- one line, column and length cannot describe a block that runs
    /// on to a matching `END` -- so redacting that span would take out the
    /// label and leave the key material in the document under a `[REDACTED]`
    /// marker. That is exactly the black-rectangle failure ADR-0028 is
    /// written against, so it is refused and named rather than half done.
    fn from_scan(original: String, findings: &[bp_secrets::Finding]) -> Self {
        let lines = line_spans(&original);
        let mut spans = Vec::new();
        let mut listing = Vec::new();
        let mut key_blocks = 0usize;
        let mut unlocatable = 0usize;

        for finding in findings {
            if finding.kind == bp_secrets::SecretKind::PrivateKeyBlock {
                key_blocks += 1;
                continue;
            }
            let Some((offset, line)) = finding
                .line
                .checked_sub(1)
                .and_then(|index| lines.get(index))
                .copied()
            else {
                unlocatable += 1;
                continue;
            };
            let first = finding.column.saturating_sub(1);
            let start = byte_of_char(line, first);
            let end = byte_of_char(line, first.saturating_add(finding.length));
            if start >= end {
                unlocatable += 1;
                continue;
            }
            spans.push(bp_redaction::Span::labelled(
                offset + start,
                offset + end,
                finding.kind.label(),
            ));
            listing.push((finding.line, finding.kind.label()));
        }

        Self {
            original,
            spans,
            listing,
            key_blocks,
            unlocatable,
            total: findings.len(),
        }
    }

    /// Why there is nothing to redact, when there is nothing to redact.
    ///
    /// Three different sentences, because they ask three different things of
    /// the user. "No credentials found" is an all-clear; the other two are
    /// not, and reporting them as one would be the quiet lie.
    fn refusal(&self) -> String {
        if self.key_blocks > 0 {
            return format!(
                concat!(
                    "nothing was redacted — the {} private key block{} found {} marked only by ",
                    "the BEGIN line, and replacing that would leave the key body in the ",
                    "document; remove {} by hand",
                ),
                self.key_blocks,
                if self.key_blocks == 1 { "" } else { "s" },
                if self.key_blocks == 1 { "is" } else { "are" },
                if self.key_blocks == 1 { "it" } else { "them" },
            );
        }
        if self.unlocatable > 0 {
            return format!(
                concat!(
                    "nothing was redacted — {} of the {} finding{} could not be located in ",
                    "this text; scan again",
                ),
                self.unlocatable,
                self.total,
                if self.total == 1 { "" } else { "s" },
            );
        }
        "nothing to redact — no credentials found".to_owned()
    }

    /// What the user is agreeing to, in full, before anything is destroyed.
    ///
    /// Line numbers and rule names only. The value is never printed here for
    /// the same reason the scan's own listing never prints it: a dialog is a
    /// thing people screenshot into bug reports.
    pub(crate) fn consent_body(&self) -> String {
        let mut body = format!(
            "{} credential{} will be replaced with {}: kind] markers:\n\n",
            self.listing.len(),
            if self.listing.len() == 1 { "" } else { "s" },
            // Built from the constant rather than written out, so the shape
            // the dialog promises and the shape a later search for previous
            // redactions looks for cannot drift apart.
            bp_redaction::PLACEHOLDER.trim_end_matches(']'),
        );
        for (line, kind) in &self.listing {
            let _ = writeln!(body, "    line {line} — {kind}");
        }
        body.push_str(concat!(
            "\nPositions and kinds only. The marker names the rule that matched, ",
            "never what it matched.\n",
        ));
        if self.key_blocks > 0 {
            let _ = write!(
                body,
                concat!(
                    "\n{} private key block{} will be left alone: the scan marks only the ",
                    "-----BEGIN ... PRIVATE KEY----- line, and replacing that would take out ",
                    "the label and leave the key body in the document. Remove {} by hand.\n",
                ),
                self.key_blocks,
                if self.key_blocks == 1 { "" } else { "s" },
                if self.key_blocks == 1 { "it" } else { "them" },
            );
        }
        if self.unlocatable > 0 {
            let _ = write!(
                body,
                concat!(
                    "\n{} finding{} could not be located in this text and will be left alone. ",
                    "That should not happen; scan again before trusting this.\n",
                ),
                self.unlocatable,
                if self.unlocatable == 1 { "" } else { "s" },
            );
        }
        body.push_str(concat!(
            "\nThis changes the document in front of you, not the file on disk, and undo ",
            "restores what was removed. The file you saved earlier, its recovery journal, ",
            "the undo history, and anything already copied to the clipboard or sent still ",
            "hold the originals. Redacting here deals with one of the places this text ",
            "lives.\n",
        ));
        body
    }

    /// One line for the status bar once the redaction has been carried out.
    ///
    /// `survivors` are the lines of redactions whose text still occurs
    /// somewhere in the result, or `None` when the check itself could not be
    /// run. ADR-0028 is careful that a survivor is a reason to look and not a
    /// verdict -- the same value appearing somewhere the scan did not mark is
    /// the usual cause -- so the wording says look, rather than failed.
    fn outcome(&self, written: usize, survivors: Option<&[usize]>) -> String {
        let mut line = match survivors {
            Some([]) => format!(
                concat!(
                    "✓ {} redaction{} written — undo brings the originals back, and the copy ",
                    "on disk still holds them until you save",
                ),
                written,
                if written == 1 { "" } else { "s" },
            ),
            Some(lines) => format!(
                concat!(
                    "⚠ {} redaction{} written, but {} still occurs elsewhere in this ",
                    "document (line{} {}) — the scan did not mark that copy; look at it and ",
                    "redact it by hand",
                ),
                written,
                if written == 1 { "" } else { "s" },
                lines.len(),
                if lines.len() == 1 { "" } else { "s" },
                lines
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            None => format!(
                concat!(
                    "⚠ {} redaction{} written, but the check that nothing survived could not ",
                    "be run",
                ),
                written,
                if written == 1 { "" } else { "s" },
            ),
        };
        if self.key_blocks > 0 {
            let _ = write!(
                line,
                "; {} private key block{} left alone, marked only by the BEGIN line",
                self.key_blocks,
                if self.key_blocks == 1 { "" } else { "s" },
            );
        }
        line
    }
}

/// Byte offset and text of every line, split exactly as `str::lines` splits.
///
/// `bp_secrets` numbers its findings by enumerating `text.lines()`, so this
/// has to agree with it line for line -- including that `lines` drops the
/// carriage return of a CRLF ending, which shifts every byte offset after it
/// if it is not put back.
fn line_spans(text: &str) -> Vec<(usize, &str)> {
    let mut spans = Vec::new();
    let mut offset = 0usize;
    for line in text.lines() {
        spans.push((offset, line));
        offset += line.len();
        if text[offset..].starts_with('\r') {
            offset += 1;
        }
        if text[offset..].starts_with('\n') {
            offset += 1;
        }
    }
    spans
}

/// The byte offset of character `index`, or the end of the line past it.
///
/// The end rather than `None`, so a column that runs off the line collapses
/// to an empty span, which the caller counts as unlocatable rather than
/// redacting something it guessed at.
fn byte_of_char(line: &str, index: usize) -> usize {
    line.char_indices()
        .nth(index)
        .map_or(line.len(), |(offset, _)| offset)
}

/// The 1-based line a byte offset falls on.
fn line_at(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

/// Which shape a document arrived in, from its extension.
///
/// By extension rather than through `bp_formats::detect`, because every
/// format `bp-formats` knows is one whose bytes *are* its content -- there is
/// no `Format::Docx` to ask about. The question here is the opposite one: is
/// this a container whose metadata reading the text cannot reach?
///
/// An unknown extension, and a document that has never been saved, are
/// [`bp_redaction::Container::PlainText`]. That claims nothing about hidden
/// metadata and still carries the sentence about the filesystem entry around
/// the file, which is the honest answer where there is no evidence either way.
fn container_of(path: Option<&Path>) -> bp_redaction::Container {
    use bp_redaction::Container;

    let extension = path
        .and_then(Path::extension)
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "docx" | "docm" | "xlsx" | "xlsm" | "pptx" | "pptm" => Container::OfficeOpenXml,
        "odt" | "ods" | "odp" | "odg" | "odf" => Container::OpenDocument,
        "pdf" => Container::Pdf,
        "rtf" => Container::RichText,
        "png" | "jpg" | "jpeg" | "gif" | "tif" | "tiff" | "webp" | "heic" | "heif" | "avif" => {
            Container::Image
        }
        _ => Container::PlainText,
    }
}

/// How many metadata findings the status bar names, for the same reason
/// [`SECRETS_IN_SUMMARY`] exists.
const METADATA_IN_SUMMARY: usize = 3;

/// One line for the status bar describing a metadata inspection.
///
/// Kinds and positions, never the value -- `MetadataFinding` deliberately
/// carries neither, and a status bar that reached back into the document to
/// quote an email address would undo that.
///
/// The container half is not garnish. For a format this build cannot see
/// inside, "nothing found" reads to a user as an all-clear and would be a
/// lie, so what was *not* looked at is said in the same sentence as what was.
pub(crate) fn metadata_summary(
    findings: &[bp_redaction::MetadataFinding],
    container: bp_redaction::Container,
) -> String {
    let hidden = container.hidden();
    let found = if findings.is_empty() {
        "nothing identifying in the text".to_owned()
    } else {
        let named: Vec<String> = findings
            .iter()
            .take(METADATA_IN_SUMMARY)
            .map(|f| format!("{} at line {}", f.kind.label(), f.line))
            .collect();
        let rest = findings.len() - named.len();
        format!(
            "{} identifying item{} — {}{}",
            findings.len(),
            if findings.len() == 1 { "" } else { "s" },
            named.join(", "),
            if rest == 0 {
                String::new()
            } else {
                format!(", and {rest} more")
            }
        )
    };

    if hidden.is_empty() {
        if findings.is_empty() {
            format!(
                concat!(
                    "✓ {} — the file's own timestamps, ownership and extended attributes are ",
                    "outside this check",
                ),
                found,
            )
        } else {
            format!("⚠ {found}")
        }
    } else {
        // "but" rather than "and" when the text came back clean, because
        // that clause is the one contradicting the clause before it: a clean
        // text scan is exactly what a user would otherwise read as an
        // all-clear for the whole file.
        format!(
            "⚠ {found}{} this {} file also carries {} where reading its text cannot look",
            if findings.is_empty() {
                ", but"
            } else {
                ", and"
            },
            container.label(),
            join_and(&hidden.iter().map(|kind| kind.label()).collect::<Vec<_>>()),
        )
    }
}

/// The full listing of a metadata inspection, one finding per line.
///
/// Ends with what was *not* looked at, in every case. For plain text that is
/// the filesystem entry around the file, which `bp-redaction` says explicitly
/// is the shell's and not its own; for anything else it is the container's
/// own metadata and what a build would need to read it.
pub(crate) fn metadata_report(
    findings: &[bp_redaction::MetadataFinding],
    container: bp_redaction::Container,
) -> String {
    let mut body = if findings.is_empty() {
        "Nothing identifying was found in the text.\n".to_owned()
    } else {
        let mut listing = format!(
            "{} item{} of identifying metadata in the text:\n\n",
            findings.len(),
            if findings.len() == 1 { "" } else { "s" },
        );
        for finding in findings {
            let _ = writeln!(
                listing,
                "    line {} — {} ({})",
                finding.line,
                finding.kind.label(),
                exposure_label(finding.exposure),
            );
        }
        listing.push_str("\nPositions and kinds only. The value itself is never printed here.\n");
        listing
    };

    match container.requires() {
        None => body.push_str(concat!(
            "\nThe bytes of a plain text file are the whole document, so this looked at all ",
            "of it. What it cannot see is the filesystem entry around the file: modification ",
            "and creation times, ownership, extended attributes and, on Windows, alternate ",
            "data streams. Those identify a document too, and they travel with a copy.\n",
        )),
        // No article in front of the label: a sentence that had to choose
        // between "a PDF" and "an Office Open XML" would need to know which,
        // and `Container` grows.
        Some(needs) => {
            let _ = write!(
                body,
                concat!(
                    "\nThis {} file also carries {} that reading its text cannot reach; ",
                    "seeing those would need {}. Nothing above is an all-clear for ",
                    "this file.\n",
                ),
                container.label(),
                join_and(
                    &container
                        .hidden()
                        .iter()
                        .map(|kind| kind.label())
                        .collect::<Vec<_>>()
                ),
                needs,
            );
        }
    }
    body
}

/// How far a finding goes towards identifying somebody, in the user's terms.
///
/// Presentation, so it lives here rather than in `bp-redaction`, on the same
/// argument as [`confidence_label`]: the crate is pure and has no opinion
/// about wording, and a bare "circumstantial" beside a real name reads as
/// "ignore me".
const fn exposure_label(exposure: bp_redaction::Exposure) -> &'static str {
    match exposure {
        bp_redaction::Exposure::Attributable => "names a person",
        bp_redaction::Exposure::Identifying => "names a machine, account or organisation",
        bp_redaction::Exposure::Circumstantial => "narrows the field",
    }
}

/// `"a, b and c"`. A sentence rather than a list, because these are read once
/// rather than scanned.
fn join_and(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [only] => (*only).to_owned(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// How many findings the status bar names before it stops counting them out.
///
/// The bar elides, so a list longer than this is a list whose tail nobody
/// reads. The full set goes in the report instead.
const SECRETS_IN_SUMMARY: usize = 3;

/// One line for the status bar describing a secret scan.
///
/// Names the kinds and where they are, and never the matched text. That is
/// not a nicety: `bp_secrets::Finding` deliberately holds no secret at all,
/// so that no `Debug` print or log line can leak one, and a status bar that
/// quoted the credential would put it back on screen -- and into the
/// screenshot of the bug it was attached to.
pub(crate) fn secret_scan_summary(findings: &[bp_secrets::Finding]) -> String {
    if findings.is_empty() {
        return "✓ no credentials found".to_owned();
    }
    let named: Vec<String> = findings
        .iter()
        .take(SECRETS_IN_SUMMARY)
        .map(|f| format!("{} at line {} col {}", f.kind.label(), f.line, f.column))
        .collect();
    let rest = findings.len() - named.len();
    format!(
        "⚠ {} possible credential{} — {}{}",
        findings.len(),
        if findings.len() == 1 { "" } else { "s" },
        named.join(", "),
        if rest == 0 {
            String::new()
        } else {
            format!(", and {rest} more")
        }
    )
}

/// The full listing of a scan, one finding per line.
///
/// Line and column first, because the reason to read this is to go and look
/// at each one, and `bp-secrets` reports 1-based positions for exactly that.
pub(crate) fn secret_scan_report(findings: &[bp_secrets::Finding]) -> String {
    findings
        .iter()
        .map(|f| {
            format!(
                "line {}, column {} — {} ({})",
                f.line,
                f.column,
                f.kind.label(),
                confidence_label(f.confidence)
            )
        })
        .collect::<Vec<_>>()
        .join(
            "
",
        )
}

/// How sure the scanner is, in the user's terms.
///
/// Presentation, so it lives here rather than in `bp-secrets`, which is pure
/// and has no opinion about wording. Low is spelled out rather than left as a
/// bare word: a row reading "low" beside a real password reads as "ignore
/// me", and the whole point of the tier is that the user decides.
const fn confidence_label(confidence: bp_secrets::Confidence) -> &'static str {
    match confidence {
        bp_secrets::Confidence::High => "high confidence",
        bp_secrets::Confidence::Medium => "medium confidence",
        bp_secrets::Confidence::Low => "low confidence, worth a look",
    }
}

/// The report shown for a document digest.
///
/// Both spellings, because they are read by different people in different
/// ways: the grouped one is for a human comparing two screens or reading it
/// down a telephone, the unbroken one is for pasting beside what `sha256sum`
/// or `certutil` printed. `bp_crypto` offers both and deliberately refuses to
/// choose between them.
fn hash_report(digest: &bp_crypto::DocumentHash, bytes: usize, unsaved: bool) -> String {
    let mut report = format!(
        "{}

{}

Taken over the {bytes} bytes this document would be written as, so it \
        matches the file once it is saved.",
        digest.to_display(),
        digest.to_hex(),
    );
    if unsaved {
        // The one way this digest can be honestly wrong about the file, and
        // it is invisible from the dialog otherwise.
        report.push_str(
            "

This document has unsaved changes, so it is not yet the digest of \
            anything on disk.",
        );
    }
    report
}

/// Read a verifying key from the file the user chose, in either spelling.
///
/// Thirty-two raw bytes is what `VerifyingKey::to_bytes` writes; hexadecimal
/// is what arrives when somebody pastes a key out of an email into a text
/// file, which is how a public key actually travels between people. The raw
/// form is tried first because a 32-byte file cannot also be 64 hex digits,
/// so the two can never be confused for one another.
fn read_verifying_key(path: &Path) -> Result<bp_crypto::VerifyingKey, String> {
    let raw = std::fs::read(path).map_err(|e| e.to_string())?;
    if let Ok(key) = bp_crypto::VerifyingKey::from_bytes(&raw) {
        return Ok(key);
    }
    let text = std::str::from_utf8(&raw)
        .map_err(|_| "this is neither 32 raw bytes nor a key in hexadecimal".to_owned())?;
    bp_crypto::VerifyingKey::from_hex(text).map_err(|e| e.to_string())
}

/// The user's Documents folder, if the platform names one.
pub(crate) fn documents_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        let profile = std::env::var_os("USERPROFILE")?;
        let docs = PathBuf::from(profile).join("Documents");
        docs.is_dir().then_some(docs)
    } else {
        // XDG_DOCUMENTS_DIR is set by user-dirs; fall back to the convention,
        // then to the home directory itself.
        if let Some(dir) = std::env::var_os("XDG_DOCUMENTS_DIR") {
            let dir = PathBuf::from(dir);
            if dir.is_dir() {
                return Some(dir);
            }
        }
        let home = PathBuf::from(std::env::var_os("HOME")?);
        let docs = home.join("Documents");
        Some(if docs.is_dir() { docs } else { home })
    }
}

/// Where this process keeps its recent list, recovery journal and security
/// history.
///
/// `bp-config` owns where each belongs and moving it there from where older
/// builds put it; this only chooses, once, between the machine's answer and
/// a test's.
#[cfg(not(test))]
fn locations() -> bp_config::Locations {
    bp_config::Locations::for_host()
}

/// The same, redirected and made unique under test.
///
/// The one `cfg(test)` in this crate, and every part of it was learned the
/// hard way. `AppState::new()` is how nearly every test starts, and each of
/// these three is written by ordinary operations -- opening a file pushes the
/// recent list, a dirty document checkpoints, and changing a security profile
/// appends to the history -- so **every** test writes to whatever this
/// returns:
///
/// * pointed at the real directories, `cargo test` wrote a live security
///   history into the user's `%APPDATA%`, created an empty recovery
///   directory there on every run, and replaced the developer's own recent
///   list with the paths of test fixtures;
/// * pointed at one shared temp location, parallel tests each read the
///   history, each computed the same next sequence number, and the log then
///   refused to open as out of order -- so every one of them failed, on a
///   file none of them was really testing.
///
/// A counter rather than a random name, so a failing run names a directory
/// that can be looked at, and the process id so two runs at once cannot
/// collide. Nothing is created here; the directory appears only if a test
/// writes into it. [`bp_config::Locations::for_host`] is never called under
/// test, because it may rename things.
#[cfg(test)]
fn locations() -> bp_config::Locations {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("bpad-test-{}-{n}", std::process::id()));
    bp_config::Locations {
        recent: Some(root.join("recent.toml")),
        recovery: Some(root.join(bp_config::locations::RECOVERY_DIR)),
        security_history: Some(root.join(bp_config::locations::SECURITY_HISTORY_FILE)),
        profiles: Some(root.join(bp_config::locations::PROFILES_FILE)),
        left_behind: Vec::new(),
    }
}

/// One document's checkpoint, snapshotted on the UI thread and ready to be
/// sealed and written anywhere (`write_checkpoints`).
pub(crate) struct CheckpointJob {
    id: DocumentId,
    edits: u64,
    ticket: bp_history::Ticket,
    checkpoint: bp_history::Checkpoint,
    recovery: bp_security::Recovery,
    passphrase: Option<zeroize::Zeroizing<String>>,
}

/// What became of one [`CheckpointJob`].
pub(crate) struct CheckpointWritten {
    id: DocumentId,
    edits: u64,
    result: std::io::Result<bp_history::Written>,
}

/// Seal and write `jobs` -- the slow half of a checkpoint pass, safe on any
/// thread: it touches nothing but the journal, which is shared and locked,
/// and each write is dropped if its ticket was cancelled meanwhile.
pub(crate) fn write_checkpoints(
    journal: &bp_history::Journal,
    jobs: Vec<CheckpointJob>,
) -> Vec<CheckpointWritten> {
    jobs.into_iter()
        .map(|job| {
            let prepared = journal.prepare(
                job.ticket,
                &job.checkpoint,
                job.recovery,
                job.passphrase.as_ref().map(|p| p.as_str()),
            );
            CheckpointWritten {
                id: job.id,
                edits: job.edits,
                result: journal.commit(prepared),
            }
        })
        .collect()
}

/// Where settings chosen in the window are saved, and what the file last
/// said about them.
#[derive(Debug, Clone)]
pub(crate) struct SavedSettings {
    pub(crate) path: PathBuf,
    /// The configuration as last read or written. Changes are measured from
    /// this, so a command-line override that was never changed in the
    /// window is never saved.
    pub(crate) baseline: bp_config::Config,
}

/// The security profile `bp-config` names. By name, because `bp-config`
/// does not depend on `bp-security`; a name one knows and the other does
/// not is Standard, the profile that changes nothing.
pub(crate) fn profile_named(profile: bp_config::SecurityProfile) -> bp_security::Profile {
    bp_security::Profile::all()
        .iter()
        .copied()
        .find(|p| p.name().eq_ignore_ascii_case(profile.name()))
        .unwrap_or_default()
}

/// What an answer to the startup "Unsaved work recovered" prompt means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecoveryChoice {
    /// Open the recovered documents now.
    Restore,
    /// Delete them. Only ever an explicit "No".
    Discard,
    /// Leave them in the journal and ask again at the next launch.
    Keep,
}

/// The recovery prompt's buttons, left to right. Their order is the mapping
/// [`recovery_choice`] reads back.
pub(crate) const RECOVERY_BUTTONS: [&str; 3] = ["Restore", "Discard", "Not Now"];

/// The one place the prompt's answer becomes an action.
///
/// **Only an explicit Discard discards.** This was a data-loss bug on Linux
/// while the prompt was an `rfd` dialog: rfd shows message dialogs by running
/// `zenity`, and when zenity is absent -- most KDE installations -- the
/// dialog could not appear and rfd reported `Cancel`. The prompt used to
/// treat every answer but Yes as a decision to discard, so recovered work was
/// deleted without the user ever seeing the question. The prompt is drawn in
/// the window now (release plan M1-01), but the rule stays: Escape, and any
/// index that is not a button, keep the work.
pub(crate) fn recovery_choice(chosen: usize) -> RecoveryChoice {
    match chosen {
        0 => RecoveryChoice::Restore,
        1 => RecoveryChoice::Discard,
        _ => RecoveryChoice::Keep,
    }
}

/// A status-bar line for anything `bp-config` could not move, or `None`.
///
/// Worth saying because both cases leave something where the product will
/// not look for it by itself -- and the recovery journal is the thing most
/// worth finding. The path is the useful half; the reason is shown for the
/// case where the user can act on it.
pub(crate) fn describe_left_behind(left: &[bp_config::Adoption]) -> Option<String> {
    let lines: Vec<String> = left
        .iter()
        .filter_map(|adoption| match adoption {
            bp_config::Adoption::KeptLegacy { legacy, error } => Some(format!(
                "⚠ could not move {} to its new place ({error}) — still using it there, nothing lost",
                legacy.display()
            )),
            bp_config::Adoption::BothOccupied { legacy } => Some(format!(
                "⚠ {} is from an older version and is no longer read — it was kept, not deleted",
                legacy.display()
            )),
            bp_config::Adoption::NothingToMove | bp_config::Adoption::Moved => None,
        })
        .collect();
    (!lines.is_empty()).then(|| lines.join("; "))
}

/// A document's path, if it has one.
///
/// A free function so `set_security` can read it without holding a borrow of
/// the workspace across the mutable one it already has.
fn doc_path(workspace: &Workspace, id: DocumentId) -> Option<PathBuf> {
    workspace
        .get(id)
        .and_then(Document::path)
        .map(Path::to_path_buf)
}

/// Map a tab id from the UI back to a `DocumentId`.
///
/// Returns `None` for ids that are no longer open, which is what makes a
/// stale click harmless rather than a panic.
pub(crate) fn find_id(workspace: &Workspace, raw: i32) -> Option<DocumentId> {
    workspace
        .iter()
        .map(Document::id)
        .find(|id| i32::try_from(id.get()).unwrap_or(i32::MAX) == raw)
}

/// Build the query the find bar currently describes.
///
/// One function rather than three `Query` literals, because every place that
/// searches has to agree about the options -- a find that honours the regex
/// toggle and a Replace All that quietly does not is worse than neither
/// honouring it.
///
/// What `bp-search` cannot catch is a toggle landing in the wrong field:
/// `case_sensitive` and `whole_word` are both `bool`, so swapping them
/// compiles, passes every test in that crate, and gives the user a whole-word
/// switch that changes the case rules. That is what the test below is for.
pub(crate) fn find_query(
    pattern: &str,
    case_sensitive: bool,
    whole_word: bool,
    regex: bool,
) -> bp_search::Query {
    bp_search::Query {
        pattern: pattern.to_owned(),
        case_sensitive,
        whole_word,
        regex,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn clock_uses_twelve_hour_time() {
        assert_eq!(clock(datetime!(2026-08-16 20:05 UTC)), "8:05 PM");
        assert_eq!(clock(datetime!(2026-08-16 08:05 UTC)), "8:05 AM");
        assert_eq!(clock(datetime!(2026-08-16 00:30 UTC)), "12:30 AM");
        assert_eq!(clock(datetime!(2026-08-16 12:00 UTC)), "12:00 PM");
    }

    #[test]
    fn saving_a_mixed_document_does_not_rewrite_the_minority_line_break() {
        // The defect this replaced. The shell used to normalise to LF and
        // re-expand to the document's declared ending, so every save quietly
        // converted whichever convention was in the minority -- and nothing
        // on screen said so. `bp-files` owns the encoder now and the shell
        // asks it for `Preserve`; this is the test that goes red if somebody
        // "tidies" that back to the declared ending.
        let dir = std::env::temp_dir().join(format!("bpad-ui-mixed-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mixed.txt");

        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        state.edit("crlf\r\nlf\nend".to_owned());

        assert_eq!(
            state.save_document(id, Some(path.clone())),
            SaveResult::Saved
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"crlf\r\nlf\nend",
            "both conventions survive a save exactly as the user left them"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_digest_is_taken_over_the_same_bytes_a_save_would_write() {
        // `active_bytes` and `save_document` have to agree on the policy, or
        // a digest taken to settle a question about a file describes bytes
        // that file does not contain -- and a mixed-ending document is
        // exactly the kind somebody takes a digest to settle.
        let dir = std::env::temp_dir().join(format!("bpad-ui-digest-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mixed.txt");

        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        state.edit("crlf\r\nlf\nend".to_owned());

        let hashed = state.active_bytes().expect("a document is open");
        assert_eq!(
            state.save_document(id, Some(path.clone())),
            SaveResult::Saved
        );
        assert_eq!(
            hashed,
            std::fs::read(&path).unwrap(),
            "the digest describes the file that was actually written"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn gutter_matches_line_count() {
        let mut state = AppState::new();
        assert!(state.sync_gutter());
        assert_eq!(state.gutter, "1", "an empty buffer still has line 1");

        state.edit("a\nb\nc".to_owned());
        assert!(state.sync_gutter());
        assert_eq!(state.gutter, "1\n2\n3");
    }

    #[test]
    fn the_gutter_numbers_the_line_after_a_trailing_newline() {
        // Regression: `str::lines()` reports "a\n" as one line, so the gutter
        // was a line short for essentially every file on disk.
        let mut state = AppState::new();
        state.edit("a\n".to_owned());
        state.sync_gutter();
        assert_eq!(state.gutter, "1\n2");
    }

    #[test]
    fn gutter_is_not_rebuilt_when_the_line_count_is_unchanged() {
        // The whole point of the cache: typing within a line must not
        // reallocate a string proportional to the document.
        let mut state = AppState::new();
        state.edit("hello".to_owned());
        assert!(state.sync_gutter());

        state.edit("hello world".to_owned());
        assert!(
            !state.sync_gutter(),
            "same line count must not rebuild the gutter"
        );

        state.edit("hello\nworld".to_owned());
        assert!(state.sync_gutter(), "a new line must rebuild it");
        assert_eq!(state.gutter, "1\n2");
    }

    #[test]
    fn a_new_document_reports_never_saved() {
        let state = AppState::new();
        let doc = state.workspace.active().unwrap();
        assert_eq!(save_state_label(doc), "● Never saved");
    }

    #[test]
    fn an_edited_document_reports_unsaved() {
        let mut state = AppState::new();
        state.edit("typing".to_owned());
        let doc = state.workspace.active().unwrap();
        assert_eq!(save_state_label(doc), "● Unsaved");
        assert_eq!(state.active_text(), "typing");
    }

    #[test]
    fn a_saved_document_reports_the_save_time() {
        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        state
            .workspace
            .get_mut(id)
            .unwrap()
            .record_disk_save(datetime!(2026-08-16 20:05 UTC));

        assert_eq!(
            save_state_label(state.workspace.active().unwrap()),
            "✓ Saved 8:05 PM"
        );
    }

    #[test]
    fn a_recovery_checkpoint_shows_beside_the_disk_save_not_instead_of_it() {
        // specs.md section 3. The failure this guards against is a status bar
        // that says "saved" because a checkpoint happened.
        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        let doc = state.workspace.get_mut(id).unwrap();
        doc.record_disk_save(datetime!(2026-08-16 20:01 UTC));
        doc.mark_modified();
        doc.record_checkpoint(datetime!(2026-08-16 20:05 UTC));

        let label = save_state_label(state.workspace.active().unwrap());
        assert_eq!(
            label,
            "● Unsaved │ Recovery 8:05 PM │ Last disk save 8:01 PM"
        );
        assert!(!label.contains("Saved 8:05"), "a checkpoint is not a save");
    }

    #[test]
    fn a_checkpoint_alone_never_reads_as_saved() {
        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        state.edit("work".to_owned());
        state
            .workspace
            .get_mut(id)
            .unwrap()
            .record_checkpoint(datetime!(2026-08-16 20:05 UTC));

        let label = save_state_label(state.workspace.active().unwrap());
        assert_eq!(label, "● Unsaved │ Recovery 8:05 PM");
        assert!(!label.contains('✓'));
    }

    #[test]
    fn editing_after_a_save_still_shows_the_last_disk_save() {
        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        state
            .workspace
            .get_mut(id)
            .unwrap()
            .record_disk_save(datetime!(2026-08-16 20:05 UTC));
        state.edit("more".to_owned());

        assert_eq!(
            save_state_label(state.workspace.active().unwrap()),
            "● Unsaved │ Last disk save 8:05 PM"
        );
    }

    #[test]
    fn save_without_a_path_reports_that_it_needs_one() {
        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        assert_eq!(
            state.save_document(id, None),
            SaveResult::NeedsPath,
            "Save on an unsaved document must escalate to Save As, not no-op"
        );
    }

    #[test]
    fn a_failed_save_is_not_reported_as_saved() {
        // The distinction close-on-quit depends on: if this returned Saved,
        // the buffer would be discarded after a save that never happened.
        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        state.edit("work".to_owned());

        let unwritable = std::path::PathBuf::from("no-such-dir-xyz").join("note.txt");
        assert_eq!(
            state.save_document(id, Some(unwritable)),
            SaveResult::Failed
        );
        assert!(state.error.is_some(), "a failure must be explained");
        assert!(state.is_dirty(id), "a failed save must not clear dirty");
    }

    #[test]
    fn a_successful_save_clears_dirty() {
        let dir = std::env::temp_dir().join(format!("bpad-ui-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("note.txt");

        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        state.edit("work".to_owned());

        assert_eq!(
            state.save_document(id, Some(path.clone())),
            SaveResult::Saved
        );
        assert!(!state.is_dirty(id));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "work");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dirty_tracking_drives_the_close_prompt() {
        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();

        assert!(!state.is_dirty(id), "a fresh buffer needs no prompt");
        state.edit("typed".to_owned());
        assert!(state.is_dirty(id), "an edited buffer must prompt");
        assert_eq!(state.display_name(id), UNTITLED);
    }

    #[test]
    fn closing_the_last_tab_opens_a_fresh_one() {
        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        state.close(id);

        assert_eq!(state.workspace.len(), 1);
        assert!(state.workspace.active().is_some());
        assert!(
            !state.editors.contains_key(&id),
            "closed text must be dropped"
        );
    }

    #[test]
    fn a_semantic_name_keeps_the_date_already_in_it_when_opened_later() {
        // Opening a file is not creating it. The rename used to stamp the
        // day it was opened over the date its name already carried.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Budget_02MAR2026.txt");
        std::fs::write(&path, "# Budget\n").unwrap();
        let mut state = AppState::new();
        state.open(path);
        let id = state.workspace.active_id().unwrap();

        let doc = state.workspace.get(id).unwrap();
        assert_eq!(doc.created_at().date(), time::macros::date!(2026 - 03 - 02));
        let name = state.semantic_filename(id).expect("a name");
        assert!(name.contains("02MAR2026"), "got {name}");
    }

    #[test]
    fn a_plain_name_takes_its_date_from_the_file_not_from_today() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        std::fs::write(&path, "notes\n").unwrap();
        // Noon, so no time zone can move it to another day.
        let written = time::macros::datetime!(2020 - 01 - 15 12:00 UTC);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(written.into())
            .unwrap();
        let mut state = AppState::new();
        state.open(path);
        let id = state.workspace.active_id().unwrap();

        assert_eq!(
            state.workspace.get(id).unwrap().created_at().date(),
            time::macros::date!(2020 - 01 - 15),
            "a file last written in 2020 cannot have been created today"
        );
    }

    #[test]
    fn every_profile_the_config_can_name_is_a_profile_the_shell_has() {
        // By name across two crates that do not depend on each other. A
        // name that fell through would quietly give Standard to somebody who
        // asked for Maximum.
        for &profile in bp_config::SecurityProfile::ALL {
            assert_eq!(
                profile_named(profile).name().to_ascii_lowercase(),
                profile.name().to_ascii_lowercase(),
                "{profile:?}"
            );
        }
    }

    #[test]
    fn configure_seeds_the_view_and_the_default_profile_reaches_every_new_document() {
        let config = bp_config::Config {
            word_wrap: true,
            line_numbers: false,
            tab_width: 2,
            indent_spaces: true,
            clipboard_record_other_programs: true,
            default_profile: bp_config::SecurityProfile::Confidential,
            editable_limit_mib: 1,
            ..bp_config::Config::default()
        };
        let mut state = AppState::new();
        state.configure(&config, None);

        assert!(state.wrap_text && !state.show_gutter);
        assert_eq!((state.indent.spaces, state.indent.width), (true, 2));
        assert!(state.record_other_programs);
        assert_eq!(state.editable_limit, 1024 * 1024);

        let confidential = bp_security::Security::Named(bp_security::Profile::Confidential);
        let untitled = state.workspace.active_id().unwrap();
        assert_eq!(
            state.workspace.get(untitled).unwrap().security(),
            confidential
        );
        state.new_document();
        let fresh = state.workspace.active_id().unwrap();
        assert_eq!(state.workspace.get(fresh).unwrap().security(), confidential);
    }

    #[test]
    fn a_file_past_the_configured_limit_opens_no_tab() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.log");
        std::fs::write(&path, vec![b'x'; 2 * 1024 * 1024]).unwrap();
        let config = bp_config::Config {
            editable_limit_mib: 1,
            ..bp_config::Config::default()
        };
        let mut state = AppState::new();
        state.configure(&config, None);
        let before = state.workspace.len();

        state.open(path);

        assert_eq!(state.workspace.len(), before);
        assert!(state.error.is_some(), "refused without saying why");
    }

    /// A session, and the state directory a restart of it would use.
    fn two_sessions() -> (AppState, bp_config::Locations) {
        let places = locations();
        (AppState::at(&places), places)
    }

    #[test]
    fn reopening_a_confidential_document_is_confidential() {
        // Release plan M1-19. The profile used to last only as long as the
        // tab: the next session opened the same file as Standard, with a
        // plaintext journal and clipboard history back on and nothing on
        // screen to say so.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("salaries.txt");
        std::fs::write(&path, "private\n").unwrap();
        let confidential = bp_security::Security::Named(bp_security::Profile::Confidential);
        let (mut first, places) = two_sessions();
        first.open(path.clone());
        first.set_security(confidential);
        drop(first);

        let mut second = AppState::at(&places);
        second.open(path);
        assert_eq!(second.security(), confidential);
    }

    #[test]
    fn save_as_takes_the_profile_to_the_new_file_and_leaves_the_old_one_its_own() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("draft.txt");
        let new = dir.path().join("final.txt");
        std::fs::write(&old, "text\n").unwrap();
        let private = bp_security::Security::Named(bp_security::Profile::Private);
        let (mut first, places) = two_sessions();
        first.open(old.clone());
        first.set_security(private);
        let id = first.workspace.active_id().unwrap();
        assert_eq!(
            first.save_document(id, Some(new.clone())),
            SaveResult::Saved
        );
        drop(first);

        let mut second = AppState::at(&places);
        second.open(new);
        assert_eq!(second.security(), private, "the new file lost it");
        second.open(old);
        assert_eq!(second.security(), private, "the old file lost its own");
    }

    #[test]
    fn an_untitled_documents_profile_is_kept_from_its_first_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        let maximum = bp_security::Security::Named(bp_security::Profile::Maximum);
        let (mut first, places) = two_sessions();
        first.set_security(maximum);
        first.edit("secret\n".to_owned());
        let id = first.workspace.active_id().unwrap();
        assert_eq!(
            first.save_document(id, Some(path.clone())),
            SaveResult::Saved
        );
        drop(first);

        let mut second = AppState::at(&places);
        second.open(path);
        assert_eq!(second.security(), maximum);
    }

    #[test]
    fn privacy_mode_is_not_mistaken_for_the_documents_own_profile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plain.txt");
        std::fs::write(&path, "text\n").unwrap();
        let (mut first, places) = two_sessions();
        first.open(path.clone());
        first.set_privacy(bp_security::Privacy::On);
        let id = first.workspace.active_id().unwrap();
        first.edit("changed\n".to_owned());
        assert_eq!(
            first.save_document(id, Some(path.clone())),
            SaveResult::Saved
        );
        drop(first);

        let mut second = AppState::at(&places);
        second.open(path);
        assert_eq!(second.security(), bp_security::Security::default());
    }

    #[test]
    fn a_default_profile_is_given_but_never_remembered_as_the_files_own() {
        // Only a choice the user makes is kept. Were the default written
        // down, changing the default later would not reach files opened
        // under the old one.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plain.txt");
        std::fs::write(&path, "text\n").unwrap();
        let config = bp_config::Config {
            default_profile: bp_config::SecurityProfile::Private,
            ..bp_config::Config::default()
        };
        let (mut first, places) = two_sessions();
        first.configure(&config, None);
        first.open(path.clone());
        assert_eq!(
            first.security(),
            bp_security::Security::Named(bp_security::Profile::Private)
        );
        drop(first);

        let mut second = AppState::at(&places);
        second.open(path);
        assert_eq!(second.security(), bp_security::Security::default());
    }

    #[test]
    fn opening_several_files_keeps_every_notice_and_asks_one_passphrase() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("plain.txt");
        std::fs::write(&plain, "text\n").unwrap();
        let missing = dir.path().join("missing.txt");
        let sealed = |name: &str| {
            let path = dir.path().join(name);
            let bytes =
                bp_crypto::seal(b"secret", "pass", bp_crypto::SealOptions::default()).unwrap();
            std::fs::write(&path, bytes).unwrap();
            path
        };
        let (one, two) = (sealed("one.bpadx"), sealed("two.bpadx"));
        let mut state = AppState::new();

        let asking = state.open_many(vec![plain.clone(), missing, one, two]);

        assert!(asking, "the first encrypted file should ask");
        assert!(
            state
                .workspace
                .iter()
                .any(|d| d.path() == Some(plain.as_path()))
        );
        let notice = state.error.clone().unwrap_or_default();
        assert!(
            notice.contains("missing.txt"),
            "the failed open was lost: {notice}"
        );
        assert!(
            notice.contains("two.bpadx"),
            "the second locked file vanished: {notice}"
        );
    }

    #[test]
    fn the_suggested_name_steps_round_a_file_that_already_has_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = AppState::new();
        let path = dir.path().join("here.txt");
        std::fs::write(&path, "# Budget\n").unwrap();
        state.open(path);
        let id = state.workspace.active_id().unwrap();
        let first = state.suggested_filename(id);
        std::fs::write(dir.path().join(&first), "somebody else's").unwrap();

        let second = state.suggested_filename(id);

        assert_ne!(first, second, "a taken name was offered again");
        let parsed = SemanticName::parse(&second).expect("still in the grammar");
        assert_eq!(parsed.collision(), Some(bp_naming::FIRST_COLLISION));
    }

    #[test]
    fn a_documents_own_file_is_not_a_collision_for_its_suggestion() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Budget_02MAR2026.txt");
        std::fs::write(&path, "# Budget\n").unwrap();
        let mut state = AppState::new();
        state.open(path);
        let id = state.workspace.active_id().unwrap();

        assert_eq!(state.suggested_filename(id), "Budget_02MAR2026.txt");
    }

    #[test]
    fn a_new_revision_bumps_the_suffix_and_skips_one_that_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Budget_v2_02MAR2026.txt");
        std::fs::write(&path, "text\n").unwrap();
        std::fs::write(dir.path().join("Budget_v3_02MAR2026.txt"), "taken").unwrap();
        let mut state = AppState::new();
        state.open(path);
        let id = state.workspace.active_id().unwrap();

        let next = state.next_revision_path(id).expect("a path");

        assert_eq!(
            next.file_name().unwrap(),
            "Budget_v3_02MAR2026_02.txt",
            "v3 is the next revision; the file already called that is not overwritten"
        );
    }

    #[test]
    fn a_plain_name_is_brought_into_the_grammar_for_its_first_revision() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.md");
        std::fs::write(&path, "text\n").unwrap();
        let mut state = AppState::new();
        state.open(path);
        let id = state.workspace.active_id().unwrap();

        let next = state.next_revision_path(id).expect("a path");
        let name = next.file_name().unwrap().to_string_lossy().into_owned();
        let parsed = SemanticName::parse(&name).expect("in the grammar");

        assert_eq!(parsed.title(), "notes");
        assert_eq!(parsed.revision(), Some(2));
        assert_eq!(parsed.extension(), "md");
    }

    #[test]
    fn an_untitled_document_has_no_revision_to_save() {
        let state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        assert_eq!(state.next_revision_path(id), None);
    }

    #[test]
    fn suggested_filename_follows_the_grammar() {
        let state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        let name = state.suggested_filename(id);
        assert!(name.starts_with("Untitled_"), "got {name}");
        assert!(name.ends_with(".txt"), "got {name}");
        assert!(SemanticName::parse(&name).is_some(), "got {name}");
    }

    #[test]
    fn no_menu_action_id_falls_inside_the_recent_files_window() {
        // The dispatch arm for recent files is a *range*, so an id landing
        // inside it opens a file instead of doing what its row says. That
        // was previously prevented only by the order the arms happened to be
        // written in.
        let end = action::RECENT_BASE + i32::try_from(bp_config::MAX_RECENT).unwrap();
        let window = action::RECENT_BASE..end;

        for id in [
            action::LINES_SORT_ASC,
            action::LINES_SORT_DESC,
            action::LINES_DEDUPE,
            action::LINES_REVERSE,
            action::LINES_TRIM,
            action::DUPLICATE_LINE,
            action::MOVE_LINE_UP,
            action::MOVE_LINE_DOWN,
            action::DATA_VALIDATE,
            action::DATA_REPORT,
            action::DATA_CSV_TO_JSON,
            action::DATA_CSV_TO_JSONL,
            action::DATA_COLUMN_TYPES,
            action::DATA_SORT_KEYS,
            action::NOTE_TITLE,
            action::NOTE_OUTLINE,
            action::SCAN_SECRETS,
            action::HASH_DOCUMENT,
            action::SIGN_DOCUMENT,
            action::VERIFY_SIGNATURE,
            // In the File menu, and therefore in the same *menu* as the
            // recent rows -- which is where an id landing in that window
            // would be least visible and most confusing.
            action::SET_DEFAULT_EDITOR,
        ] {
            assert!(!window.contains(&id), "id {id} collides with recent files");
        }
    }

    // --- Duplicate Line / Move Line Up / Move Line Down --------------------

    #[test]
    fn duplicating_a_line_through_the_menu_marks_the_document_modified() {
        let mut state = AppState::new();
        state.editor_view = true;
        state.edit("one\ntwo\nthree".to_owned());
        let id = state.workspace.active_id().unwrap();
        state.workspace.get_mut(id).unwrap().record_disk_save(now());
        state.active_editor_mut().unwrap().set_cursor(5); // on "two"

        state.run_line_edit(action::DUPLICATE_LINE);

        assert_eq!(state.active_text(), "one\ntwo\ntwo\nthree");
        assert!(state.is_dirty(id), "duplicating a line is an edit");
    }

    #[test]
    fn moving_a_line_down_through_the_menu_swaps_it_and_marks_modified() {
        let mut state = AppState::new();
        state.editor_view = true;
        state.edit("one\ntwo\nthree".to_owned());
        let id = state.workspace.active_id().unwrap();
        state.workspace.get_mut(id).unwrap().record_disk_save(now());
        state.active_editor_mut().unwrap().set_cursor(1); // on "one"

        state.run_line_edit(action::MOVE_LINE_DOWN);

        assert_eq!(state.active_text(), "two\none\nthree");
        assert!(state.is_dirty(id));
    }

    #[test]
    fn moving_the_top_line_up_is_a_no_op_and_must_not_mark_the_document_dirty() {
        // bp-editor's own no-op for Move Line Up at the first line; the point
        // of this test is that bp-ui does not turn that no-op into a false
        // "modified" flag.
        let mut state = AppState::new();
        state.editor_view = true;
        state.edit("one\ntwo".to_owned());
        let id = state.workspace.active_id().unwrap();
        state.workspace.get_mut(id).unwrap().record_disk_save(now());
        state.active_editor_mut().unwrap().set_cursor(1); // on "one", the top line

        state.run_line_edit(action::MOVE_LINE_UP);

        assert_eq!(state.active_text(), "one\ntwo");
        assert!(
            !state.is_dirty(id),
            "a no-op must not mark a clean document dirty"
        );
    }

    #[test]
    fn duplicate_line_acts_on_the_default_views_caret_once_it_is_copied_in() {
        // Release plan M1-04. The widget reports its caret in bytes; the
        // line under it is the one duplicated, accents before it or not.
        let mut state = AppState::new();
        state.edit("café\nsecond\nthird".to_owned());
        let id = state.workspace.active_id().unwrap();
        state.workspace.get_mut(id).unwrap().record_disk_save(now());

        // Byte 8 is in "second" ("café\n" is six bytes).
        state.follow_widget_caret(8, 8);
        state.run_line_edit(action::DUPLICATE_LINE);

        assert_eq!(state.active_text(), "café\nsecond\nsecond\nthird");
        assert!(state.is_dirty(id));
    }

    // --- CSV / TSV data operations ------------------------------------------

    fn csv_state(text: &str) -> AppState {
        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        // Format detection is extension-first (`bp_formats::detect`), so a
        // `.csv` path is what makes `run_data_action` see `Format::Csv`
        // without needing a real file on disk.
        state
            .workspace
            .get_mut(id)
            .unwrap()
            .set_path(PathBuf::from("table.csv"));
        state.edit(text.to_owned());
        state
    }

    #[test]
    fn converting_csv_to_json_replaces_the_document() {
        let mut state = csv_state("name,age\nAlice,30\nBob,25\n");
        let id = state.workspace.active_id().unwrap();
        // Cleared explicitly, rather than relying on the dirty flag the
        // initial `edit` already set, so the assertion below proves the
        // conversion itself is what marks the document unsaved.
        state.workspace.get_mut(id).unwrap().record_disk_save(now());

        state.run_data_action(action::DATA_CSV_TO_JSON);

        assert!(
            state.error.is_none(),
            "a successful conversion is not an error"
        );
        assert!(state.active_text().contains("Alice"));
        assert!(state.active_text().trim_start().starts_with('['));
        assert!(
            state.is_dirty(id),
            "nothing reaches disk on its own -- the conversion is an ordinary edit"
        );
    }

    #[test]
    fn converting_csv_to_json_lines_replaces_the_document() {
        let mut state = csv_state("name,age\nAlice,30\nBob,25\n");
        state.run_data_action(action::DATA_CSV_TO_JSONL);

        let text = state.active_text();
        assert_eq!(text.lines().count(), 2, "one compact object per data row");
        assert!(text.contains("Alice") && text.contains("Bob"));
    }

    #[test]
    fn column_types_reports_into_the_status_bar_without_changing_the_document() {
        let mut state = csv_state("name,age\nAlice,30\nBob,25\n");
        let before = state.active_text();

        state.run_data_action(action::DATA_COLUMN_TYPES);

        assert_eq!(
            state.active_text(),
            before,
            "a report must not edit the document"
        );
        let message = state.error.expect("column report in the status bar");
        assert!(message.contains("name"), "got {message}");
        assert!(
            message.contains("integer"),
            "age should read as integer; got {message}"
        );
    }

    #[test]
    fn an_action_that_is_not_a_csv_operation_on_a_csv_document_changes_nothing() {
        let mut state = csv_state("name,age\nAlice,30\n");
        let before = state.active_text();
        state.run_data_action(action::NOTE_TITLE);
        assert_eq!(state.active_text(), before);
    }

    // --- Save Copy ------------------------------------------------------

    /// A document with a path and unsaved edits -- the state in which every
    /// difference between Save Copy and Save As is visible.
    fn dirty_saved_state(dir: &std::path::Path) -> (AppState, DocumentId, PathBuf) {
        let original = dir.join("original.txt");
        std::fs::write(&original, "first\n").unwrap();

        let mut state = AppState::new();
        state.open(original.clone());
        let id = state.workspace.active_id().unwrap();
        state.edit("first\nsecond\n".to_owned());
        assert!(state.workspace.get(id).unwrap().is_dirty());
        (state, id, original)
    }

    /// A Windows-1252 file on disk, opened: "café" with a Windows line end.
    fn opened_1252(dir: &std::path::Path) -> (AppState, DocumentId, PathBuf) {
        let path = dir.join("ansi.txt");
        std::fs::write(&path, b"caf\xe9\r\n").unwrap();
        let mut state = AppState::new();
        state.open(path.clone());
        let id = state.workspace.active_id().unwrap();
        (state, id, path)
    }

    #[test]
    fn a_windows_1252_file_opens_as_one_and_saves_back_byte_for_byte() {
        let dir = tempfile::tempdir().unwrap();
        let (mut state, id, path) = opened_1252(dir.path());
        assert_eq!(
            state.workspace.get(id).unwrap().encoding(),
            Encoding::Windows1252
        );
        assert_eq!(state.active_text(), "café\r\n");

        state.edit("café au lait\r\n".to_owned());
        assert_eq!(state.save_document(id, None), SaveResult::Saved);
        assert_eq!(std::fs::read(&path).unwrap(), b"caf\xe9 au lait\r\n");
    }

    #[test]
    fn a_character_windows_1252_cannot_hold_stops_the_save_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (mut state, id, path) = opened_1252(dir.path());
        state.edit("café, Gdańsk\r\n".to_owned());

        let result = state.save_document(id, None);

        let SaveResult::Unrepresentable(what) = result else {
            panic!("saved anyway: {result:?}");
        };
        assert_eq!(what.character, 'ń');
        assert_eq!(&state.active_text()[what.byte_offset..], "ńsk\r\n");
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"caf\xe9\r\n",
            "the file changed"
        );
        assert!(state.is_dirty(id), "unsaved work was marked saved");
    }

    #[test]
    fn choosing_windows_1252_for_text_it_cannot_hold_is_refused_with_the_line() {
        let mut state = AppState::new();
        state.edit("fine\nstill fine\nGdańsk\n".to_owned());
        let id = state.workspace.active_id().unwrap();

        state.set_encoding(Encoding::Windows1252);

        assert_eq!(state.workspace.get(id).unwrap().encoding(), Encoding::Utf8);
        let notice = state.error.clone().expect("a reason");
        assert!(
            notice.contains('ń') && notice.contains("line 3"),
            "{notice}"
        );
    }

    #[test]
    fn choosing_windows_1252_for_text_it_can_hold_changes_the_encoding() {
        let mut state = AppState::new();
        state.edit("café — naïve\n".to_owned());
        let id = state.workspace.active_id().unwrap();

        state.set_encoding(Encoding::Windows1252);

        assert_eq!(
            state.workspace.get(id).unwrap().encoding(),
            Encoding::Windows1252
        );
    }

    #[test]
    fn a_copy_does_not_become_the_documents_path() {
        // The difference that turns Save Copy into Save As. If the path
        // moves, the next Ctrl+S writes the copy and leaves the original
        // behind, unsaved and unmentioned.
        let dir = tempfile::tempdir().unwrap();
        let (mut state, id, original) = dirty_saved_state(dir.path());
        let copy = dir.path().join("copy.txt");

        assert_eq!(state.save_copy(id, &copy), SaveResult::Saved);

        assert_eq!(
            state.workspace.get(id).unwrap().path(),
            Some(original.as_path()),
            "the document still belongs to the file it was opened from"
        );
    }

    #[test]
    fn a_copy_leaves_the_document_unsaved() {
        // The dangerous one. A cleared dirty flag means the close prompt
        // never appears and the real edits are discarded in silence.
        let dir = tempfile::tempdir().unwrap();
        let (mut state, id, _) = dirty_saved_state(dir.path());

        state.save_copy(id, &dir.path().join("copy.txt"));

        assert!(
            state.workspace.get(id).unwrap().is_dirty(),
            "the original still has edits that are not on disk"
        );
    }

    #[test]
    fn only_an_explicit_discard_discards_recovered_work() {
        assert_eq!(RECOVERY_BUTTONS[0], "Restore");
        assert_eq!(recovery_choice(0), RecoveryChoice::Restore);
        assert_eq!(RECOVERY_BUTTONS[1], "Discard");
        assert_eq!(recovery_choice(1), RecoveryChoice::Discard);
        assert_eq!(RECOVERY_BUTTONS[2], "Not Now");
        assert_eq!(recovery_choice(2), RecoveryChoice::Keep);
        // Nothing the sheet can send, but not a decision if it ever did.
        assert_eq!(recovery_choice(3), RecoveryChoice::Keep);
        assert_eq!(recovery_choice(usize::MAX), RecoveryChoice::Keep);
    }

    #[test]
    fn a_confidential_document_never_enters_the_recent_list() {
        let dir = tempfile::tempdir().unwrap();
        let (mut state, id, path) = dirty_saved_state(dir.path());
        state.recent = Recent::new();
        state.set_security(bp_security::Security::Named(
            bp_security::Profile::Confidential,
        ));

        state.save_document(id, None);

        assert!(
            !state.recent.paths().contains(&path),
            "Confidential forbids recording anything about the document"
        );
    }

    #[test]
    fn privacy_mode_keeps_an_opened_file_out_of_the_recent_list() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("diary.txt");
        std::fs::write(&path, b"dear diary").unwrap();
        let mut state = AppState::new();
        state.set_privacy(bp_security::Privacy::On);

        state.open(path.clone());

        assert!(!state.recent.paths().contains(&path));
    }

    #[test]
    fn a_standard_document_is_still_remembered() {
        // The control: the check must not switch the feature off.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        std::fs::write(&path, b"hello").unwrap();
        let mut state = AppState::new();

        state.open(path.clone());

        assert!(state.recent.paths().contains(&path));
    }

    #[test]
    fn a_file_too_large_to_edit_opens_no_tab_and_says_why() {
        // Sparse, so the test costs a `stat` and not 40 MiB of disk. The
        // point is that the shell never hands such a file to the editable
        // widget, which is what used to hang the whole application.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.log");
        std::fs::File::create(&path)
            .unwrap()
            .set_len(bp_files::EDITABLE_LIMIT_BYTES + 1)
            .unwrap();
        let mut state = AppState::new();
        let tabs = state.workspace.len();

        state.open(path);

        assert_eq!(state.workspace.len(), tabs, "a tab was opened for it");
        let error = state.error.expect("the refusal is reported");
        assert!(error.contains("too large to edit"), "{error}");
    }

    #[test]
    fn a_large_file_that_loads_says_tools_may_be_slow() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.txt");
        std::fs::File::create(&path)
            .unwrap()
            .set_len(bp_buffer::LARGE_FILE_BYTES + 1)
            .unwrap();
        let mut state = AppState::new();

        state.open(path);

        let notice = state.error.expect("a large file is flagged");
        assert!(notice.starts_with("Large file"), "{notice}");
    }

    #[test]
    fn a_test_state_keeps_every_file_it_writes_in_the_temp_directory() {
        // The protection `locations()` exists to give, pinned for all three
        // at once. Before it, `AppState::new()` here read and wrote the
        // developer's own recent list and made a recovery directory in their
        // real configuration directory on every run.
        let state = AppState::new();
        let temp = std::env::temp_dir();
        for (what, path) in [
            (
                "recent list",
                state.recent_file.clone().expect("a recent file"),
            ),
            ("recovery journal", state.journal.location().to_path_buf()),
            ("security history", state.audit_path.clone()),
        ] {
            assert!(
                path.starts_with(&temp),
                "the {what} is at {} under test",
                path.display()
            );
        }
    }

    #[test]
    fn two_test_states_never_share_a_recent_list() {
        // Parallel tests sharing one file is the failure the security history
        // hit; the recent list is written on every open and would hit it too.
        assert_ne!(AppState::new().recent_file, AppState::new().recent_file);
    }

    #[test]
    fn opening_a_file_records_it_in_the_list_this_state_was_given() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.txt");
        std::fs::write(&path, b"hello").unwrap();
        let mut state = AppState::new();
        let file = state.recent_file.clone().expect("a recent file");

        state.open(path.clone());

        assert!(
            bp_config::load_recent(&file).paths().contains(&path),
            "the list was not written where the state was told to keep it"
        );
    }

    #[test]
    fn a_left_behind_notice_survives_opening_a_file_at_startup() {
        // The launch a file association makes: one file named, which opens
        // and then closes the blank tab -- and both used to clear the notice
        // before anybody could see it.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.txt");
        std::fs::write(&path, b"hello").unwrap();
        let mut state = AppState::new();
        state.left_behind = Some("⚠ an old journal".to_owned());

        state.start_up(Some("config warning".to_owned()), &[path]);

        assert_eq!(state.workspace.len(), 1, "the blank tab was replaced");
        assert_eq!(
            state.error.as_deref(),
            Some("config warning; ⚠ an old journal")
        );
    }

    #[test]
    fn a_file_that_fails_to_open_at_startup_is_reported_beside_the_others() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = AppState::new();
        state.left_behind = Some("⚠ an old journal".to_owned());

        state.start_up(None, &[dir.path().join("absent.txt")]);

        let error = state.error.expect("something to say");
        assert!(error.starts_with("⚠ an old journal; "), "{error}");
        assert!(error.len() > "⚠ an old journal; ".len(), "{error}");
    }

    #[test]
    fn a_notice_is_added_beside_the_last_one_not_over_it() {
        let mut state = AppState::new();
        state.add_notice("first".to_owned());
        state.add_notice("second".to_owned());
        assert_eq!(state.error.as_deref(), Some("first; second"));
    }

    #[test]
    fn a_quiet_startup_says_nothing() {
        let mut state = AppState::new();
        state.start_up(None, &[]);
        assert_eq!(state.error, None);
    }

    #[test]
    fn a_move_that_worked_or_had_nothing_to_do_says_nothing() {
        assert_eq!(describe_left_behind(&[]), None);
        assert_eq!(
            describe_left_behind(&[
                bp_config::Adoption::Moved,
                bp_config::Adoption::NothingToMove
            ]),
            None
        );
    }

    #[test]
    fn a_journal_that_could_not_be_moved_is_named_and_the_user_told_it_is_safe() {
        let legacy = PathBuf::from("/old/bachelorpad/recovery");
        let line = describe_left_behind(&[bp_config::Adoption::KeptLegacy {
            legacy: legacy.clone(),
            error: std::io::Error::other("cross-device link"),
        }])
        .expect("a failed move must be reported");
        assert!(line.contains(&legacy.display().to_string()), "{line}");
        assert!(line.contains("cross-device link"), "{line}");
        assert!(line.contains("nothing lost"), "{line}");
    }

    #[test]
    fn an_old_copy_left_beside_a_new_one_is_named() {
        let legacy = PathBuf::from("/old/bachelorpad/recovery");
        let line = describe_left_behind(&[bp_config::Adoption::BothOccupied {
            legacy: legacy.clone(),
        }])
        .expect("an unread copy must be reported");
        assert!(line.contains(&legacy.display().to_string()), "{line}");
        assert!(line.contains("kept, not deleted"), "{line}");
    }

    #[test]
    fn a_copy_does_not_join_the_recent_files_list() {
        // Recent files is "documents you worked on". A copy is an export;
        // reopening it would give a stale fork of the document.
        let dir = tempfile::tempdir().unwrap();
        let (mut state, id, _) = dirty_saved_state(dir.path());
        let copy = dir.path().join("copy.txt");
        let before = state.recent.paths().to_vec();

        state.save_copy(id, &copy);

        assert_eq!(state.recent.paths(), before.as_slice());
        assert!(
            !state.recent.paths().contains(&copy),
            "the copy must not appear in Open Recent"
        );
    }

    #[test]
    fn a_copy_contains_the_edits_that_are_not_yet_on_disk() {
        // The point of the feature: it writes what is in the buffer, not
        // what is in the file it was opened from.
        let dir = tempfile::tempdir().unwrap();
        let (mut state, id, original) = dirty_saved_state(dir.path());
        let copy = dir.path().join("copy.txt");

        state.save_copy(id, &copy);

        assert_eq!(std::fs::read_to_string(&copy).unwrap(), "first\nsecond\n");
        assert_eq!(
            std::fs::read_to_string(&original).unwrap(),
            "first\n",
            "and the original on disk is untouched"
        );
    }

    #[test]
    fn a_copy_that_cannot_be_written_says_so_rather_than_reporting_success() {
        let dir = tempfile::tempdir().unwrap();
        let (mut state, id, _) = dirty_saved_state(dir.path());
        // A directory that does not exist: the write fails, and the user has
        // to be told, or they will believe a copy exists.
        let unwritable = dir.path().join("no-such-folder").join("copy.txt");

        assert_eq!(state.save_copy(id, &unwritable), SaveResult::Failed);
        assert!(
            state.error.is_some(),
            "a failed copy must reach the status bar"
        );
    }

    // --- statistics, stamps and go-to-line -------------------------------

    #[test]
    fn document_statistics_report_without_changing_the_document() {
        let mut state = AppState::new();
        state.edit("one two three\n\nfour\n".to_owned());
        let before = state.active_text();

        state.report_statistics();

        assert_eq!(state.active_text(), before, "a report must not edit");
        let message = state.error.expect("statistics in the status bar");
        assert!(message.contains("4 words"), "got {message}");
        assert!(message.contains("2 paragraphs"), "got {message}");
    }

    #[test]
    fn a_stamp_is_inserted_at_the_caret_and_marks_the_document_unsaved() {
        let mut state = AppState::new();
        state.editor_view = true;
        state.edit("ab".to_owned());
        state.active_editor_mut().unwrap().set_cursor(1);

        assert!(state.insert_stamp(0), "Date is the first stamp");

        let text = state.active_text();
        assert!(
            text.starts_with('a') && text.ends_with('b') && text.len() > 2,
            "the stamp landed at the caret; got {text:?}"
        );
        assert!(state.workspace.active().unwrap().is_dirty());
    }

    #[test]
    fn a_stamp_index_past_the_end_of_the_list_changes_nothing() {
        // The ids are a base plus an offset, so an id from a stale menu can
        // name a row that no longer exists.
        let mut state = AppState::new();
        state.editor_view = true;
        state.edit("text".to_owned());

        assert!(!state.insert_stamp(99));
        assert_eq!(state.active_text(), "text");
    }

    #[test]
    fn going_to_a_line_past_the_end_says_how_many_there_are() {
        // `Editor::go_to_line` moves somewhere sensible *and* reports that
        // the line did not exist, so the bar can do both.
        let mut state = AppState::new();
        state.editor_view = true;
        state.edit("one\ntwo\n".to_owned());

        assert!(state.go_to_line("99").is_some(), "the caret still moves");
        assert!(
            state.goto_status.contains("only"),
            "got {:?}",
            state.goto_status
        );
    }

    #[test]
    fn going_to_a_line_that_exists_reports_nothing() {
        let mut state = AppState::new();
        state.editor_view = true;
        state.edit("one\ntwo\nthree".to_owned());

        let moved = state.go_to_line("2").expect("moved");
        assert_eq!(moved.start, 4, "the start of line two");
        assert!(state.goto_status.is_empty(), "success is silent");
    }

    #[test]
    fn a_line_number_that_is_not_a_number_is_reported_rather_than_ignored() {
        // `input-type: number` is a hint to the keyboard, not a guarantee
        // from the backend, so the text still has to be parsed.
        let mut state = AppState::new();
        state.editor_view = true;
        state.edit("one\ntwo".to_owned());

        assert!(state.go_to_line("seven").is_none());
        assert_eq!(state.goto_status, "not a line number");
    }

    #[test]
    fn go_to_line_works_without_the_custom_view_and_returns_characters() {
        let mut state = AppState::new();
        state.edit("éé\ntwo".to_owned());

        let at = state.go_to_line("2").expect("a caret");

        assert_eq!(at, 3..3, "line 2 starts at character 3");
        assert_eq!(state.char_to_byte(3), 5, "which TextInput calls byte 5");
        assert!(state.goto_status.is_empty(), "{:?}", state.goto_status);
    }

    // --- tab context menu -------------------------------------------------

    #[test]
    fn the_tab_menu_acts_on_the_right_clicked_tab_not_the_active_one() {
        // The whole reason the target is stored: opening the menu and
        // choosing a row are two events, and between them the active tab is
        // still whatever it was.
        let mut state = AppState::new();
        let first = state.workspace.active_id().unwrap();
        state.new_document();
        let second = state.workspace.active_id().unwrap();
        assert_ne!(first, second);

        state.tab_context = Some(first);
        assert_eq!(state.tab_context_id(), Some(first));

        // With no menu open the rows fall back to the active tab.
        state.tab_context = None;
        assert_eq!(state.tab_context_id(), Some(second));
    }

    #[test]
    fn copy_full_path_is_unavailable_for_a_document_that_has_never_been_saved() {
        let mut state = AppState::new();
        state.new_document();

        let (others, has_path) = state.tab_context_shape();
        assert_eq!(others, 1, "one other tab is open");
        assert!(!has_path, "an untitled document has no path to copy");
    }

    // --- the security history ---------------------------------------------

    #[test]
    fn a_security_operation_reaches_the_history() {
        // The wiring, end to end and without a window: an operation happens,
        // and the report the menu row shows can see it. Everything about
        // *what* a history is belongs to `bp-audit`; what this asserts is
        // that the shell actually calls it. Standard is the default profile
        // and the one that keeps a plaintext history.
        let mut state = AppState::new();
        let before = state.security_history().unwrap_or_default();

        state.edit(
            "nothing secret here
"
            .to_owned(),
        );
        state.scan_for_secrets();

        let after = state
            .security_history()
            .expect("the history must be readable");
        // Not a length comparison: an empty history is a paragraph explaining
        // itself and a one-event history is a single line, so the report gets
        // *shorter* when the first thing is recorded.
        assert_ne!(after, before, "the scan recorded nothing");
        assert!(
            after.contains(&bp_audit::Event::SecretScanFinished { findings: 0 }.describe()),
            "the history does not mention the scan:
{after}"
        );
    }

    #[test]
    fn loosening_a_profile_is_recorded_and_names_both_ends() {
        // The direction is the interesting part, and this is the direction
        // that matters: a move down to Standard re-enables everything the
        // profile was switched on to stop, which is exactly the event a
        // history exists to hold. It is recorded because the *new* profile
        // permits a history -- see the sibling test for the other direction.
        let mut state = AppState::new();
        state.set_security(bp_security::Security::Named(
            bp_security::Profile::Confidential,
        ));
        state.set_security(bp_security::Security::Named(bp_security::Profile::Standard));

        let report = state.security_history().expect("readable");
        let expected = bp_audit::Event::SecurityProfileChanged {
            from: bp_audit::ProfileLabel::Confidential,
            to: bp_audit::ProfileLabel::Standard,
        };
        assert!(
            report.contains(&expected.describe()),
            "the history does not name both ends of the change:
{report}"
        );
    }

    #[test]
    fn tightening_into_a_profile_that_forbids_a_history_writes_nothing() {
        // The event is recorded against the profile being *moved into*, which
        // is the one that decides whether it may be kept. Recording a
        // tightening under the old, looser profile would write the very line
        // the user just asked to stop writing. `Destination::for_policy`
        // resolves Confidential to `SessionOnly`, so this is that decision
        // reaching the shell rather than a gap in the wiring.
        let mut state = AppState::new();
        let before = state.security_history().unwrap_or_default();
        state.set_security(bp_security::Security::Named(
            bp_security::Profile::Confidential,
        ));

        let after = state.security_history().expect("readable");
        assert_eq!(
            after, before,
            "a profile that forbids a history had one written for it"
        );
    }

    #[test]
    fn privacy_mode_records_nothing_because_it_clamps_the_policy_first() {
        // Switching Privacy Mode on is itself governed by the policy Privacy
        // Mode puts in force, which forbids writing anything down. That is
        // the mode doing what it was switched on for, and it is worth pinning
        // so nobody later "fixes" it into a line on disk.
        let mut state = AppState::new();
        let before = state.security_history().unwrap_or_default();
        state.set_privacy(bp_security::Privacy::On);

        let after = state.security_history().expect("readable");
        assert_eq!(
            after, before,
            "Privacy Mode wrote a line to disk about being switched on"
        );
    }

    #[test]
    fn a_history_notice_never_displaces_the_answer_the_user_asked_for() {
        // "3 possible credentials" is what somebody clicked Scan for. A line
        // about the *history* of that scan taking its place would be the log
        // making the product worse, which is what `note_quietly` exists for.
        let mut state = AppState::new();
        state.edit(
            "nothing secret here
"
            .to_owned(),
        );
        state.scan_for_secrets();
        let message = state.error.clone().expect("the scan says what it found");
        assert!(
            !message.contains("security history"),
            "the audit notice displaced the scan result: {message}"
        );
    }

    // --- security profiles ------------------------------------------------

    #[test]
    fn a_new_document_starts_on_the_default_profile() {
        let state = AppState::new();
        assert_eq!(state.security(), bp_security::Security::default());
        assert_eq!(state.policy().recovery, bp_security::Recovery::Plaintext);
    }

    #[test]
    fn the_profile_belongs_to_the_document_not_the_application() {
        // Two tabs open side by side can be governed differently, and the
        // stricter one must not be relaxed by the other being open.
        let mut state = AppState::new();
        state.set_security(bp_security::Security::Named(
            bp_security::Profile::Confidential,
        ));
        let strict = state.workspace.active_id().unwrap();

        state.new_document();
        assert_eq!(
            state.security(),
            bp_security::Security::default(),
            "a new document is not governed by the last one"
        );

        state.workspace.set_active(strict);
        assert_eq!(
            state.policy().clipboard,
            bp_security::Clipboard::Disabled,
            "switching back restores the stricter document's policy"
        );
    }

    #[test]
    fn tightening_a_profile_clears_the_clipboard_history() {
        // The history is one list for the whole application, so it is the
        // only dependant the profile change has to deal with directly -- the
        // journal is handled by the checkpoint the change triggers.
        let mut state = AppState::new();
        state
            .clips
            .push("copied earlier", bp_security::Clipboard::InMemory);
        assert!(!state.clips.is_empty());

        state.set_security(bp_security::Security::Named(
            bp_security::Profile::Confidential,
        ));

        assert!(
            state.clips.is_empty(),
            "a profile that forbids a clipboard history must not leave one \
             gathered under a looser profile sitting in the menu"
        );
        assert!(
            state.error.is_some(),
            "and the user is told why it vanished"
        );
    }

    #[test]
    fn setting_the_same_profile_again_changes_nothing() {
        // Refresh rebuilds menus constantly; a no-op that cleared the
        // clipboard would empty it whenever the menu was opened.
        let mut state = AppState::new();
        state.clips.push("keep", bp_security::Clipboard::InMemory);

        state.set_security(bp_security::Security::default());

        assert_eq!(state.clips.entries().len(), 1);
    }

    #[test]
    fn a_plaintext_document_under_a_strict_profile_is_told_how_to_fix_it() {
        // ADR-0020: fail loudly rather than degrade. The journal is sealed
        // with the document's own passphrase, so a plaintext document has no
        // key -- and the message says what turns recovery back on rather than
        // only that it is off.
        let mut state = AppState::new();
        state.set_security(bp_security::Security::Named(bp_security::Profile::Private));

        let message = state.error.expect("a capability gap reaches the user");
        assert!(
            message.contains("encrypt") || message.contains("clipboard"),
            "got {message}"
        );
    }

    #[test]
    fn an_encrypted_document_gets_an_encrypted_journal_and_recovers_at_unlock() {
        // The whole point of keying the sealed journal by path: a crashed
        // session's work comes back when the document is unlocked, and never
        // needs a prompt of its own.
        let recovery = tempfile::tempdir().unwrap();
        let docs = tempfile::tempdir().unwrap();
        let target = docs.path().join("notes.bpadx");

        let mut before = AppState::new();
        before.journal = bp_history::Journal::new(recovery.path().to_path_buf());
        before.edit("saved text".to_owned());
        let id = before.workspace.active_id().unwrap();
        before.ask = Some(crate::passphrase::Ask::Set {
            id,
            target: target.clone(),
        });
        before.answer_passphrase("hunter2");
        before.answer_passphrase("hunter2");

        // Work done after the save, then the session "crashes".
        before.set_security(bp_security::Security::Named(bp_security::Profile::Private));
        before.edit("saved text plus unsaved".to_owned());
        before.checkpoint_all();

        // Nothing readable without the passphrase, but something is there.
        assert!(before.journal.pending().is_empty());
        assert_eq!(before.journal.sealed_count(), 1);

        // The first session ends before the second starts, as a crash or a
        // quit would end it. While it is alive its journal is held, and a
        // second window rightly neither offers nor discards its work.
        drop(before);
        let mut after = AppState::new();
        after.journal = bp_history::Journal::new(recovery.path().to_path_buf());
        after.open_maybe_encrypted(target);
        after.answer_passphrase("hunter2");

        assert_eq!(
            after.active_text(),
            "saved text plus unsaved",
            "the unsaved work did not come back at unlock"
        );
        assert!(
            after.workspace.active().unwrap().is_dirty(),
            "recovered work is not on disk, so the tab must be dirty or the \
             user can close it and lose it without being asked"
        );
    }

    #[test]
    fn saving_an_encrypted_document_discards_its_sealed_journal() {
        // The work is on disk now. A journal left behind would offer to
        // "recover" a stale copy at the next unlock.
        let recovery = tempfile::tempdir().unwrap();
        let docs = tempfile::tempdir().unwrap();
        let target = docs.path().join("notes.bpadx");

        let mut state = AppState::new();
        state.journal = bp_history::Journal::new(recovery.path().to_path_buf());
        state.edit("first".to_owned());
        let id = state.workspace.active_id().unwrap();
        state.ask = Some(crate::passphrase::Ask::Set {
            id,
            target: target.clone(),
        });
        state.answer_passphrase("hunter2");
        state.answer_passphrase("hunter2");
        state.set_security(bp_security::Security::Named(bp_security::Profile::Private));

        state.edit("first and second".to_owned());
        state.checkpoint_all();
        assert_eq!(state.journal.sealed_count(), 1);

        assert_eq!(state.save_document(id, None), SaveResult::Saved);

        assert_eq!(
            state.journal.sealed_count(),
            0,
            "a stale sealed journal would be offered at the next unlock"
        );
    }

    #[test]
    fn a_confidential_document_is_not_checkpointed_in_plaintext() {
        // The end-to-end version of the journal's own test, through the
        // autosave path the application actually uses.
        let dir = tempfile::tempdir().unwrap();
        let mut state = AppState::new();
        state.journal = bp_history::Journal::new(dir.path().to_path_buf());
        state.edit("sensitive text".to_owned());

        state.checkpoint_all();
        assert_eq!(state.journal.pending().len(), 1, "Standard journals it");

        state.set_security(bp_security::Security::Named(
            bp_security::Profile::Confidential,
        ));
        state.checkpoint_all();

        assert!(
            state.journal.pending().is_empty(),
            "the earlier plaintext must be gone, not merely not added to"
        );
    }

    // --- privacy mode -----------------------------------------------------

    #[test]
    fn privacy_mode_clears_the_clipboard_and_removes_journals() {
        // Switching it on has to *act*, not merely be recorded. A mode that
        // only governed future writes would leave everything gathered a
        // moment ago exactly where it was, which is the opposite of what
        // somebody switching it on wants.
        let dir = tempfile::tempdir().unwrap();
        let mut state = AppState::new();
        state.journal = bp_history::Journal::new(dir.path().to_path_buf());
        state.edit("unsaved work".to_owned());
        state
            .clips
            .push("copied earlier", bp_security::Clipboard::InMemory);
        state.checkpoint_all();
        assert_eq!(state.journal.pending().len(), 1);
        assert!(!state.clips.is_empty());

        state.set_privacy(bp_security::Privacy::On);

        assert!(state.clips.is_empty(), "clipboard history survived");
        assert!(state.journal.pending().is_empty(), "the journal survived");
    }

    /// A state journalling into `dir`, with one dirty document.
    fn journalling(dir: &Path) -> (AppState, DocumentId) {
        let mut state = AppState::new();
        state.journal = bp_history::Journal::new(dir.to_path_buf());
        state.edit("first".to_owned());
        let id = state.workspace.active_id().unwrap();
        (state, id)
    }

    fn journalled_text(state: &AppState) -> Vec<String> {
        state
            .journal
            .pending()
            .into_iter()
            .map(|(_, c)| c.text)
            .collect()
    }

    #[test]
    fn an_unchanged_document_is_not_checkpointed_again() {
        // Release plan M1-08. Detected by removing the checkpoint behind
        // the journal's back: a second write would bring it back.
        let dir = tempfile::tempdir().unwrap();
        let (mut state, id) = journalling(dir.path());
        state.checkpoint_all();
        assert_eq!(journalled_text(&state), ["first"]);

        state.journal.discard(id.get()).unwrap();
        state.checkpoint_all();
        assert!(
            journalled_text(&state).is_empty(),
            "rewritten with nothing new"
        );

        state.edit("second".to_owned());
        state.checkpoint_all();
        assert_eq!(
            journalled_text(&state),
            ["second"],
            "a real edit was not journalled"
        );
    }

    #[test]
    fn a_checkpoint_snapshotted_before_a_save_never_lands_after_it() {
        // Release plan M1-07: the write now happens later, on another
        // thread. A save in between discards the journal, and the stale
        // snapshot must not bring it back as "recovered" work.
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("doc.txt");
        std::fs::write(&file, "old\n").unwrap();
        let mut state = AppState::new();
        state.journal = bp_history::Journal::new(dir.path().join("journal"));
        state.open(file.clone());
        state.edit("new\n".to_owned());
        let id = state.workspace.active_id().unwrap();

        let jobs = state.prepare_checkpoints();
        assert_eq!(jobs.len(), 1);
        assert_eq!(state.save_document(id, None), SaveResult::Saved);
        let journal = state.journal.clone();
        let written = std::thread::spawn(move || write_checkpoints(&journal, jobs))
            .join()
            .unwrap();
        state.apply_checkpoints(written);

        assert!(
            journalled_text(&state).is_empty(),
            "the saved work came back as recoverable"
        );
        assert!(!state.checkpointed.contains_key(&id));
    }

    #[test]
    fn a_checkpoint_written_on_another_thread_is_recorded_when_it_comes_back() {
        let dir = tempfile::tempdir().unwrap();
        let (mut state, id) = journalling(dir.path());
        let jobs = state.prepare_checkpoints();
        let journal = state.journal.clone();
        let written = std::thread::spawn(move || write_checkpoints(&journal, jobs))
            .join()
            .unwrap();
        state.apply_checkpoints(written);

        assert_eq!(journalled_text(&state), ["first"]);
        assert!(state.workspace.get(id).unwrap().last_checkpoint().is_some());
        // Recorded, so the next pass has nothing to do.
        assert!(state.prepare_checkpoints().is_empty());
    }

    #[test]
    fn leaving_privacy_mode_journals_unchanged_work_again() {
        let dir = tempfile::tempdir().unwrap();
        let (mut state, _) = journalling(dir.path());
        state.checkpoint_all();
        state.set_privacy(bp_security::Privacy::On);
        assert!(journalled_text(&state).is_empty());

        state.set_privacy(bp_security::Privacy::Off);
        state.checkpoint_all();

        assert_eq!(
            journalled_text(&state),
            ["first"],
            "recovery lapsed until the next keystroke"
        );
    }

    #[test]
    fn closing_a_tab_without_saving_leaves_nothing_to_recover() {
        // Don't Save is a decision. The checkpoint used to outlive the tab,
        // so the next launch offered to "recover" the work the user had just
        // thrown away -- and a plaintext copy of it sat on disk until then.
        let dir = tempfile::tempdir().unwrap();
        let mut state = AppState::new();
        state.journal = bp_history::Journal::new(dir.path().to_path_buf());
        state.edit("thrown away".to_owned());
        state.new_document();
        state.edit("kept".to_owned());
        state.checkpoint_all();
        assert_eq!(state.journal.pending().len(), 2);

        let first = state.workspace.iter().map(Document::id).next().unwrap();
        state.close(first);

        let left: Vec<String> = state
            .journal
            .pending()
            .into_iter()
            .map(|(_, c)| c.text)
            .collect();
        assert_eq!(left, ["kept"], "only the open tab's work is journalled");
    }

    #[test]
    fn privacy_mode_acts_on_every_tab_not_only_the_active_one() {
        // It is session-wide. A journal left behind for a background tab is
        // exactly what it was switched on to prevent.
        let dir = tempfile::tempdir().unwrap();
        let mut state = AppState::new();
        state.journal = bp_history::Journal::new(dir.path().to_path_buf());
        state.edit("first tab".to_owned());
        state.new_document();
        state.edit("second tab".to_owned());
        state.checkpoint_all();
        assert_eq!(state.journal.pending().len(), 2);

        state.set_privacy(bp_security::Privacy::On);

        assert!(state.journal.pending().is_empty());
    }

    #[test]
    fn privacy_mode_does_not_relax_a_document_that_was_already_stricter() {
        // The clamp takes the stricter of each axis rather than substituting
        // a policy. Substituting would drag a Maximum document *down* to
        // whatever Privacy Mode specified.
        let mut state = AppState::new();
        state.set_security(bp_security::Security::Named(bp_security::Profile::Maximum));
        let before = state.policy();

        state.set_privacy(bp_security::Privacy::On);

        assert_eq!(state.policy(), before);
    }

    #[test]
    fn turning_privacy_mode_off_restores_each_documents_own_profile() {
        let mut state = AppState::new();
        let standard = state.policy();

        state.set_privacy(bp_security::Privacy::On);
        assert_ne!(state.policy(), standard);

        state.set_privacy(bp_security::Privacy::Off);
        assert_eq!(state.policy(), standard);
    }

    // --- encryption -------------------------------------------------------

    /// Drive the passphrase bar the way the shell does: submit, and be told
    /// whether it stays open.
    fn answer(state: &mut AppState, entered: &str) -> bool {
        state.answer_passphrase(entered)
    }

    #[test]
    fn encrypting_a_document_asks_twice_and_writes_only_on_a_match() {
        // A typo when *setting* a passphrase costs the document permanently,
        // which is why this is the one place the bar asks twice.
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("notes.bpadx");
        let mut state = AppState::new();
        state.edit("secret notes".to_owned());
        let id = state.workspace.active_id().unwrap();

        state.ask = Some(crate::passphrase::Ask::Set {
            id,
            target: target.clone(),
        });

        assert!(answer(&mut state, "hunter2"), "asks again to confirm");
        assert!(!target.exists(), "nothing is written after only one entry");

        assert!(!answer(&mut state, "hunter2"), "the bar closes on a match");
        assert!(target.exists(), "and the document is written");
        assert!(state.is_encrypted(id));
    }

    #[test]
    fn a_mismatched_confirmation_starts_again_rather_than_writing() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("notes.bpadx");
        let mut state = AppState::new();
        state.edit("secret".to_owned());
        let id = state.workspace.active_id().unwrap();
        state.ask = Some(crate::passphrase::Ask::Set {
            id,
            target: target.clone(),
        });

        answer(&mut state, "hunter2");
        assert!(answer(&mut state, "hunter3"), "the bar stays up");

        assert!(!target.exists(), "a mismatch must not write anything");
        assert!(!state.is_encrypted(id));
        assert!(
            matches!(state.ask, Some(crate::passphrase::Ask::Set { .. })),
            "back to the first question, not the second -- confirming again \
             would compare against the entry the user already thinks is wrong"
        );
    }

    #[test]
    fn an_empty_passphrase_is_refused_before_it_is_typed_twice() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = AppState::new();
        state.edit("secret".to_owned());
        let id = state.workspace.active_id().unwrap();
        state.ask = Some(crate::passphrase::Ask::Set {
            id,
            target: dir.path().join("notes.bpadx"),
        });

        assert!(answer(&mut state, ""), "the bar stays up");
        assert!(
            matches!(state.ask, Some(crate::passphrase::Ask::Set { .. })),
            "still on the first question"
        );
        assert!(!state.passphrase_status.is_empty(), "and says why");
    }

    #[test]
    fn an_encrypted_document_round_trips_through_the_shell() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("notes.bpadx");

        let mut writer = AppState::new();
        writer.edit("the quick brown fox".to_owned());
        let id = writer.workspace.active_id().unwrap();
        writer.ask = Some(crate::passphrase::Ask::Set {
            id,
            target: target.clone(),
        });
        answer(&mut writer, "hunter2");
        answer(&mut writer, "hunter2");

        // The file on disk must not be readable as the text that went in.
        let raw = std::fs::read(&target).unwrap();
        assert!(bp_crypto::is_bpadx(&raw));
        assert!(
            !raw.windows(3).any(|w| w == b"fox"),
            "the plaintext is in the file"
        );

        let mut reader = AppState::new();
        assert!(
            reader.open_maybe_encrypted(target.clone()),
            "an encrypted file asks for a passphrase"
        );
        assert!(!answer(&mut reader, "hunter2"), "and then opens");

        assert_eq!(reader.active_text(), "the quick brown fox");
    }

    #[test]
    fn an_encrypted_file_does_not_become_a_tab_until_it_is_unlocked() {
        // A tab nobody can read looks like an empty document, and saving it
        // would write emptiness over the real one.
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("notes.bpadx");
        let sealed =
            bp_crypto::seal(b"secret", "hunter2", bp_crypto::SealOptions::default()).unwrap();
        std::fs::write(&target, sealed).unwrap();

        let mut state = AppState::new();
        let before = state.workspace.len();
        assert!(state.open_maybe_encrypted(target));

        assert_eq!(state.workspace.len(), before, "no tab was opened");
    }

    #[test]
    fn a_wrong_passphrase_keeps_the_bar_up_and_opens_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("notes.bpadx");
        let sealed =
            bp_crypto::seal(b"secret", "hunter2", bp_crypto::SealOptions::default()).unwrap();
        std::fs::write(&target, sealed).unwrap();

        let mut state = AppState::new();
        state.open_maybe_encrypted(target);
        let before = state.workspace.len();

        assert!(answer(&mut state, "wrong"), "the bar stays up for a retry");
        assert_eq!(state.workspace.len(), before);
        assert!(
            !state.passphrase_status.is_empty(),
            "a wrong passphrase has to say something, or it looks like the \
             file simply refused to open"
        );
    }

    #[test]
    fn saving_an_encrypted_document_keeps_it_encrypted() {
        // The failure this prevents is the worst one available: a later
        // Ctrl+S quietly writing the document in clear.
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("notes.bpadx");
        let mut state = AppState::new();
        state.edit("first".to_owned());
        let id = state.workspace.active_id().unwrap();
        state.ask = Some(crate::passphrase::Ask::Set {
            id,
            target: target.clone(),
        });
        answer(&mut state, "hunter2");
        answer(&mut state, "hunter2");

        state.edit("first and second".to_owned());
        assert_eq!(state.save_document(id, None), SaveResult::Saved);

        let raw = std::fs::read(&target).unwrap();
        assert!(bp_crypto::is_bpadx(&raw), "the save wrote plaintext");
        let opened = bp_crypto::open(&raw, "hunter2").unwrap();
        assert_eq!(
            String::from_utf8_lossy(&opened),
            "first and second",
            "the edit did not reach the encrypted file"
        );
    }

    #[test]
    fn a_plain_document_is_opened_without_asking_for_anything() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("notes.txt");
        std::fs::write(&plain, "ordinary text").unwrap();

        let mut state = AppState::new();
        assert!(
            !state.open_maybe_encrypted(plain),
            "a plain file must not prompt"
        );
        assert_eq!(state.active_text(), "ordinary text");
    }

    #[test]
    fn closing_an_encrypted_document_forgets_its_passphrase() {
        // Holding it for the session is a deliberate trade; holding it past
        // the document's life is just a leak.
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("notes.bpadx");
        let mut state = AppState::new();
        state.edit("secret".to_owned());
        let id = state.workspace.active_id().unwrap();
        state.ask = Some(crate::passphrase::Ask::Set {
            id,
            target: target.clone(),
        });
        answer(&mut state, "hunter2");
        answer(&mut state, "hunter2");
        assert!(state.is_encrypted(id));

        state.close(id);

        assert!(!state.is_encrypted(id), "the passphrase outlived the tab");
    }

    #[test]
    fn each_find_option_reaches_its_own_field_and_no_other() {
        // Set one at a time, so a pair swapped in the constructor fails here
        // rather than compiling into a switch that changes the wrong thing.
        let case = find_query("x", true, false, false);
        assert!(case.case_sensitive);
        assert!(!case.whole_word, "case sensitivity must not imply words");
        assert!(!case.regex, "case sensitivity must not imply a pattern");

        let word = find_query("x", false, true, false);
        assert!(word.whole_word);
        assert!(!word.case_sensitive);
        assert!(!word.regex);

        let re = find_query("x", false, false, true);
        assert!(re.regex);
        assert!(!re.case_sensitive);
        assert!(!re.whole_word);
    }

    #[test]
    fn all_options_off_is_the_literal_query_the_find_box_used_to_build() {
        // The default the bar opens with. Everything searched before the
        // toggles existed went through `Query::literal`, so this is the
        // guarantee that adding them changed nothing for anyone who does not
        // touch them.
        assert_eq!(
            find_query("a.b", false, false, false),
            bp_search::Query::literal("a.b")
        );
    }

    #[test]
    fn the_regex_toggle_is_what_stops_a_pattern_being_escaped() {
        // The behaviour is `bp-search`'s and tested there; what this pins is
        // that the toggle reaches it at all. `a.b` matching `axb` is only
        // possible when `regex` arrived as true, so this fails if the flag is
        // dropped on the way through.
        let literal = find_query("a.b", false, false, false);
        let pattern = find_query("a.b", false, false, true);

        assert!(bp_search::find_all("axb", &literal).unwrap().is_empty());
        assert_eq!(bp_search::find_all("axb", &pattern).unwrap().len(), 1);
    }

    // --- Scan for Secrets --------------------------------------------------

    /// A credential the scanner actually recognises, and a document with one
    /// in it, so a test can assert the shell never repeats it back.
    ///
    /// Not one of the vendors' published `EXAMPLE` keys: `bp-secrets`
    /// deliberately refuses those, on the grounds that documentation is not a
    /// leak, so a fixture built from one would test nothing.
    const LEAKED: &str = "AKIA3G7QVHBRN2WPKZ5F";
    const LEAKY: &str = "notes\naws_access_key_id = AKIA3G7QVHBRN2WPKZ5F\ndone\n";

    #[test]
    fn a_clean_document_is_told_it_is_clean_rather_than_told_nothing() {
        // Silence after a scan is indistinguishable from a scan that did not
        // run, which is the one thing a security check must never be.
        let mut state = AppState::new();
        state.edit("nothing to see here\n".to_owned());
        let findings = state.scan_for_secrets();

        assert!(findings.is_empty());
        assert_eq!(
            state.error.as_deref(),
            Some("\u{2713} no credentials found")
        );
    }

    #[test]
    fn a_scan_reports_the_count_the_kind_and_where_it_is() {
        let mut state = AppState::new();
        state.edit(LEAKY.to_owned());
        let findings = state.scan_for_secrets();

        assert_eq!(findings.len(), 1);
        let notice = state.error.clone().expect("a status message");
        assert!(
            notice.contains('1'),
            "the count belongs in it; got {notice}"
        );
        assert!(
            notice.contains("AWS access key ID"),
            "the kind belongs in it; got {notice}"
        );
        assert!(
            notice.contains("line 2"),
            "where it is is the actionable part; got {notice}"
        );
    }

    #[test]
    fn neither_the_summary_nor_the_report_ever_repeats_the_secret() {
        // The whole design of `bp-secrets`: a `Finding` carries a position and
        // a classification and deliberately not the matched text, so that no
        // print of one can leak a credential. The shell is the last place that
        // could undo that, by reaching back into the document to quote it.
        let findings = bp_secrets::scan(LEAKY);
        assert!(!findings.is_empty(), "the fixture must actually be found");

        for line in [
            secret_scan_summary(&findings),
            secret_scan_report(&findings),
        ] {
            assert!(
                !line.contains(LEAKED),
                "the credential is back on screen: {line}"
            );
        }
    }

    #[test]
    fn a_long_scan_names_the_first_few_and_says_how_many_it_did_not() {
        // The status bar elides, so a list longer than the bar is a list whose
        // tail is invisible. Saying "and 2 more" is what stops the invisible
        // part reading as "there were only three".
        let text: String = (0..5)
            .map(|i| format!("aws_access_key_id{i} = {LEAKED}\n"))
            .collect();
        let findings = bp_secrets::scan(&text);
        assert_eq!(findings.len(), 5, "the fixture should yield five");

        let summary = secret_scan_summary(&findings);
        assert!(summary.contains('5'), "got {summary}");
        assert!(summary.contains("and 2 more"), "got {summary}");
    }

    #[test]
    fn the_full_report_has_a_line_per_finding_with_its_position() {
        let findings = bp_secrets::scan(LEAKY);
        let report = secret_scan_report(&findings);
        assert_eq!(report.lines().count(), findings.len());
        assert!(report.contains("line 2, column"), "got {report}");
    }

    // --- Hash, sign, verify ------------------------------------------------

    #[test]
    fn the_digest_is_taken_over_the_bytes_the_document_would_be_written_as() {
        // Not over the buffer. The BOM is stripped on load and written back on
        // save, so a digest of the buffer is one `sha256sum` disagrees with --
        // and a user comparing the two would conclude their file had been
        // altered when nothing had touched it.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bom.txt");
        std::fs::write(&path, b"\xEF\xBB\xBFone\ntwo\n").unwrap();

        let mut state = AppState::new();
        state.open(path.clone());
        assert_eq!(
            state.active_text(),
            "one\ntwo\n",
            "the BOM belongs on disk, not in the buffer"
        );
        let report = state.hash_active_document().expect("a digest");

        let on_disk = std::fs::read(&path).unwrap();
        let expected = bp_crypto::hash_document(&on_disk, bp_crypto::HashAlgorithm::Sha256);
        assert!(
            report.contains(&expected.to_hex()),
            "the digest should match the file on disk; got {report}"
        );
        let of_the_buffer = bp_crypto::hash_document(
            state.active_text().as_bytes(),
            bp_crypto::HashAlgorithm::Sha256,
        );
        assert!(
            !report.contains(&of_the_buffer.to_hex()),
            "hashing the buffer would leave the BOM out and give the wrong answer"
        );
    }

    #[test]
    fn the_digest_is_offered_grouped_and_unbroken_and_names_its_algorithm() {
        // Two spellings for two readers: grouped for a person comparing two
        // screens or reading it aloud, unbroken for pasting beside what
        // `sha256sum` printed.
        let mut state = AppState::new();
        state.edit("hello".to_owned());
        let report = state.hash_active_document().expect("a digest");

        let digest = bp_crypto::hash_document(b"hello", bp_crypto::HashAlgorithm::Sha256);
        assert!(report.contains(&digest.to_display()), "got {report}");
        assert!(report.contains(&digest.to_hex()), "got {report}");
        assert!(
            state
                .error
                .as_deref()
                .is_some_and(|e| e.contains("SHA-256")),
            "the status bar should name the algorithm; got {:?}",
            state.error
        );
    }

    #[test]
    fn a_digest_of_an_unsaved_document_says_it_is_not_the_file_on_disk() {
        let mut state = AppState::new();
        state.edit("draft".to_owned());
        let report = state.hash_active_document().expect("a digest");
        assert!(
            report.contains("unsaved changes"),
            "an unsaved digest that claims to be the file's is a lie; got {report}"
        );
    }

    #[test]
    fn the_digest_report_wraps_in_the_source_without_wrapping_on_the_screen() {
        // Both sentences in `hash_report` are wrapped to fit this file, and a
        // string continuation is the only wrapping that leaves no trace. The
        // escaped newline these two started as put a line break and
        // eight spaces into the middle of a sentence in the dialog
        // instead. Both branches are checked, because the unsaved
        // sentence is its own literal and was separately wrong.
        let mut unsaved_state = AppState::new();
        unsaved_state.edit("draft".to_owned());
        let unsaved = unsaved_state.hash_active_document().expect("a digest");

        let dir = tempfile::tempdir().unwrap();
        let (mut saved_doc, _) = saved_state(dir.path(), "notes.txt", "saved content\n");
        let saved = saved_doc.hash_active_document().expect("a digest");

        for report in [&unsaved, &saved] {
            assert!(
                report.contains("would be written as, so it matches the file"),
                "the continuation has to close the sentence up; got {report}"
            );
            for line in report.lines() {
                assert!(
                    !line.starts_with(' '),
                    "a wrapped literal leaked its indentation: {line:?}"
                );
            }
        }
        assert!(
            unsaved.contains("not yet the digest of anything on disk"),
            "got {unsaved}"
        );
    }

    #[test]
    fn a_utf16_document_hashes_rather_than_refusing() {
        // This test used to assert the opposite, and it was right to: the
        // shell's own encoder refused UTF-16, and a digest over an empty
        // vector would have been a plausible-looking wrong answer. `bp-files`
        // encodes UTF-16 now, so the refusal is gone and what is worth
        // pinning instead is that the digest covers the two-byte form with
        // its byte-order mark -- the bytes the file would actually hold.
        let mut state = AppState::new();
        state.edit("hello".to_owned());
        state.set_encoding(Encoding::Utf16Le);

        let bytes = state.active_bytes().expect("a document is open");
        assert_eq!(
            bytes.len(),
            2 + "hello".len() * 2,
            "a byte-order mark plus two bytes per character"
        );
        assert!(
            state.hash_active_document().is_some(),
            "got {:?}",
            state.error
        );
    }

    /// A signing identity that is the same on every run.
    ///
    /// A fixed seed rather than `SigningKey::generate`, so a failing
    /// assertion names the same key twice and a test cannot fail once a
    /// fortnight because the OS random source was briefly unavailable.
    fn signer(seed: u8) -> bp_integrity::SigningKey {
        bp_integrity::SigningKey::from_bytes(&[seed; bp_integrity::SIGNING_KEY_LEN])
            .expect("32 bytes is a valid seed")
    }

    /// Write the public half where the Verify row's key picker would find it.
    fn publish(dir: &std::path::Path, key: &bp_integrity::SigningKey) -> PathBuf {
        let path = dir.join(format!("{}.pub", key.verifying_key().to_hex()));
        std::fs::write(&path, key.verifying_key().to_hex()).unwrap();
        path
    }

    /// A document open on a file that exists, with no unsaved edits.
    fn saved_state(dir: &std::path::Path, name: &str, text: &str) -> (AppState, PathBuf) {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        let mut state = AppState::new();
        state.open(path.clone());
        (state, path)
    }

    #[test]
    fn the_sidecar_is_found_beside_the_document_rather_than_asked_for() {
        // `document.ext.sig`, appended and not substituted. Two callers that
        // disagree about that name produce a document one half of the product
        // believes is unsigned, which is why the convention is a function in
        // `bp-integrity` and not a rule the shell writes down again.
        let dir = tempfile::tempdir().unwrap();
        let (mut state, path) = saved_state(dir.path(), "notes.txt", "signed content\n");
        let key = signer(7);
        bp_integrity::sign_file(&path, &key).expect("the sidecar is written");

        assert!(
            dir.path().join("notes.txt.sig").is_file(),
            "the sidecar is named after the whole file name"
        );
        assert!(state.verify_signature(&bp_integrity::Expectation::AnySigner));
        let notice = state.error.clone().unwrap();
        assert!(notice.starts_with('\u{2713}'), "got {notice}");
        assert!(
            notice.contains(&key.verifying_key().to_hex()),
            "the signer has to be named; got {notice}"
        );
    }

    #[test]
    fn a_missing_sidecar_fails_closed_and_says_where_it_looked() {
        // The variant the whole design rests on. A signature that can be
        // deleted to produce a pass is not a signature, so an unsigned
        // document is a failure and not an absence of opinion.
        let dir = tempfile::tempdir().unwrap();
        let (mut state, _) = saved_state(dir.path(), "notes.txt", "unsigned\n");

        assert!(
            !state.verify_signature(&bp_integrity::Expectation::AnySigner),
            "an unsigned document must not pass"
        );
        let notice = state.error.clone().unwrap();
        assert!(notice.starts_with('\u{2717}'), "got {notice}");
        assert!(notice.contains("Not signed"), "got {notice}");
        assert!(
            notice.contains("notes.txt.sig"),
            "the user has to be told which file to go and find; got {notice}"
        );
    }

    #[test]
    fn a_malformed_sidecar_is_not_reported_as_a_tampered_document() {
        // A truncated file is a copying accident and a bad signature is an
        // accusation. Telling the user the second when the first happened
        // sends them looking for an attacker who does not exist.
        let dir = tempfile::tempdir().unwrap();
        let (mut state, path) = saved_state(dir.path(), "notes.txt", "signed content\n");
        bp_integrity::sign_file(&path, &signer(7)).unwrap();
        std::fs::write(dir.path().join("notes.txt.sig"), "not a sidecar at all\n").unwrap();

        assert!(!state.verify_signature(&bp_integrity::Expectation::AnySigner));
        let notice = state.error.clone().unwrap();
        assert!(notice.contains("cannot be read"), "got {notice}");
        assert!(
            !notice.contains("altered"),
            "nothing here says anything about the document; got {notice}"
        );
    }

    #[test]
    fn a_document_altered_after_signing_does_not_match_and_the_signer_is_a_claim() {
        let dir = tempfile::tempdir().unwrap();
        let (mut state, path) = saved_state(dir.path(), "notes.txt", "signed content\n");
        let key = signer(7);
        bp_integrity::sign_file(&path, &key).unwrap();
        // Changed on disk, not in the buffer: this is about the file.
        std::fs::write(&path, "signed content, plus a line somebody added\n").unwrap();

        assert!(!state.verify_signature(&bp_integrity::Expectation::AnySigner));
        let notice = state.error.clone().unwrap();
        assert!(notice.contains("does NOT match"), "got {notice}");
        assert!(
            notice.contains("could not be confirmed"),
            "the named signer is a claim, not an attribution; got {notice}"
        );
        assert!(
            notice.contains(&key.verifying_key().to_hex()),
            "got {notice}"
        );
    }

    #[test]
    fn an_intact_document_signed_by_somebody_else_is_its_own_answer() {
        // The verdict a bare 64-byte `.sig` cannot produce, and the entire
        // reason ADR-0026's sidecar records a key. Nothing is damaged and
        // nothing was tampered with: this is a filing question, and reporting
        // it as "does not match" would be an alarm about nothing.
        let dir = tempfile::tempdir().unwrap();
        let (mut state, path) = saved_state(dir.path(), "notes.txt", "signed content\n");
        let colleague = signer(7);
        let expected = signer(9);
        bp_integrity::sign_file(&path, &colleague).unwrap();

        assert!(!state.verify_signature(&bp_integrity::Expectation::Key(expected.verifying_key())));
        let notice = state.error.clone().unwrap();
        assert!(notice.contains("by a different key"), "got {notice}");
        assert!(notice.contains("intact"), "got {notice}");
        assert!(
            notice.contains(&colleague.verifying_key().to_hex()),
            "got {notice}"
        );
        assert!(
            notice.contains(&expected.verifying_key().to_hex()),
            "got {notice}"
        );
    }

    #[test]
    fn naming_the_expected_key_is_what_turns_a_pass_into_a_claim_about_a_person() {
        let dir = tempfile::tempdir().unwrap();
        let (mut state, path) = saved_state(dir.path(), "notes.txt", "signed content\n");
        let key = signer(7);
        bp_integrity::sign_file(&path, &key).unwrap();

        assert!(state.verify_signature(&bp_integrity::Expectation::Key(key.verifying_key())));
        let notice = state.error.clone().unwrap();
        assert!(notice.starts_with('\u{2713}'), "got {notice}");
        assert!(
            !notice.contains("no key was named"),
            "the caveat belongs only to the permissive check; got {notice}"
        );
    }

    #[test]
    fn a_pass_with_no_key_named_carries_the_caveat_that_makes_it_honest() {
        // `Expectation::AnySigner` establishes only that the document has not
        // changed since the key *in the sidecar* signed it. Anyone who alters
        // a document can re-sign it with a key of their own and pass. A tick
        // with no caveat reads as "this is from who you think", which is a
        // claim nothing checked.
        let dir = tempfile::tempdir().unwrap();
        let (mut state, path) = saved_state(dir.path(), "notes.txt", "signed content\n");
        bp_integrity::sign_file(&path, &signer(7)).unwrap();

        assert!(state.verify_signature(&bp_integrity::Expectation::AnySigner));
        let notice = state.error.clone().unwrap();
        assert!(notice.contains("no key was named"), "got {notice}");
        assert!(
            notice.contains("re-sign it with a key of their own"),
            "got {notice}"
        );
    }

    #[test]
    fn every_verdict_gets_its_own_sentence() {
        // Four of the five are "no", and they are four rather than one
        // because their fixes are four different things: find the file, get
        // an undamaged copy, get an untampered document, get the right key. A
        // shell that collapsed them would undo the point of the format.
        let dir = tempfile::tempdir().unwrap();
        let mut seen: Vec<String> = Vec::new();

        // Verified, and signed by another key.
        let (mut state, path) = saved_state(dir.path(), "a.txt", "content\n");
        bp_integrity::sign_file(&path, &signer(7)).unwrap();
        state.verify_signature(&bp_integrity::Expectation::Key(signer(7).verifying_key()));
        seen.push(state.error.clone().unwrap());
        state.verify_signature(&bp_integrity::Expectation::Key(signer(9).verifying_key()));
        seen.push(state.error.clone().unwrap());

        // Missing.
        let (mut state, _) = saved_state(dir.path(), "b.txt", "content\n");
        state.verify_signature(&bp_integrity::Expectation::AnySigner);
        seen.push(state.error.clone().unwrap());

        // Malformed.
        let (mut state, path) = saved_state(dir.path(), "c.txt", "content\n");
        bp_integrity::sign_file(&path, &signer(7)).unwrap();
        std::fs::write(dir.path().join("c.txt.sig"), "rubbish\n").unwrap();
        state.verify_signature(&bp_integrity::Expectation::AnySigner);
        seen.push(state.error.clone().unwrap());

        // Does not match.
        let (mut state, path) = saved_state(dir.path(), "d.txt", "content\n");
        bp_integrity::sign_file(&path, &signer(7)).unwrap();
        std::fs::write(&path, "content, altered\n").unwrap();
        state.verify_signature(&bp_integrity::Expectation::AnySigner);
        seen.push(state.error.clone().unwrap());

        let mut distinct = seen.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(
            distinct.len(),
            seen.len(),
            "two verdicts share a message: {seen:#?}"
        );
    }

    #[test]
    fn unsaved_edits_are_reported_because_the_verdict_is_about_the_file() {
        // The shell's own contribution and the only sentence here it is in a
        // position to write: `verify_file` checks what is on the disk, and
        // the buffer on screen is not that.
        let dir = tempfile::tempdir().unwrap();
        let (mut state, path) = saved_state(dir.path(), "notes.txt", "signed content\n");
        bp_integrity::sign_file(&path, &signer(7)).unwrap();
        state.edit("signed content\nand a line typed since\n".to_owned());

        // The file still verifies -- it is untouched -- and saying so without
        // the caveat would be a tick beside text nobody signed.
        assert!(state.verify_signature(&bp_integrity::Expectation::AnySigner));
        let notice = state.error.clone().unwrap();
        assert!(notice.contains("unsaved changes"), "got {notice}");
        assert!(
            notice.contains("not the bytes anybody signed"),
            "got {notice}"
        );
    }

    #[test]
    fn a_document_that_was_never_saved_has_nothing_for_a_sidecar_to_sit_beside() {
        // The row is greyed for this, and the arm says it anyway: a silent
        // no-op would be a bug report nobody could describe.
        let mut state = AppState::new();
        state.edit("never saved\n".to_owned());

        assert!(!state.verify_signature(&bp_integrity::Expectation::AnySigner));
        let notice = state.error.clone().unwrap();
        assert!(notice.contains("never been saved"), "got {notice}");
        assert!(
            !notice.contains("  "),
            "a wrapped literal leaked its indentation into the status bar: {notice:?}"
        );
    }

    #[test]
    fn a_public_key_pasted_out_of_an_email_as_hexadecimal_is_accepted() {
        // The realistic way a public key travels. Refusing it would teach
        // users to retype keys by hand, which is how a wrong key gets used.
        let dir = tempfile::tempdir().unwrap();
        let mut state = AppState::new();
        let key = signer(7);

        let hex_key = dir.path().join("key.txt");
        let text = key.verifying_key().to_hex();
        // Wrapped across two lines, exactly as an email client would leave it.
        std::fs::write(&hex_key, format!("{}\n{}\n", &text[..32], &text[32..])).unwrap();

        assert_eq!(
            state.expected_signer(&hex_key),
            Some(bp_integrity::Expectation::Key(key.verifying_key()))
        );
    }

    #[test]
    fn the_raw_thirty_two_byte_spelling_of_a_public_key_is_accepted_too() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = AppState::new();
        let key = signer(7);
        let raw = publish(dir.path(), &key);
        std::fs::write(&raw, key.verifying_key().to_bytes()).unwrap();

        assert_eq!(
            state.expected_signer(&raw),
            Some(bp_integrity::Expectation::Key(key.verifying_key()))
        );
    }

    #[test]
    fn a_key_file_that_is_not_a_key_says_which_of_the_two_files_was_wrong() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = AppState::new();
        let not_a_key = dir.path().join("letter.txt");
        std::fs::write(&not_a_key, "dear bob, here is the file\n").unwrap();

        assert_eq!(state.expected_signer(&not_a_key), None);
        let notice = state.error.clone().unwrap();
        assert!(
            notice.contains("key"),
            "the user has to know which file to replace; got {notice}"
        );
    }

    // --- Data ▸ YAML -------------------------------------------------------

    /// A document `run_data_action` will see as `Format::Yaml`.
    ///
    /// Detection is extension-first, so the path is what decides it and no
    /// file has to exist -- the same trick `csv_state` uses.
    fn yaml_state(text: &str) -> AppState {
        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        state
            .workspace
            .get_mut(id)
            .unwrap()
            .set_path(PathBuf::from("config.yaml"));
        state.edit(text.to_owned());
        state
    }

    /// The "billion laughs" document from ADR-0023: ten anchors, each a
    /// sequence of ten references to the one before. Around 200 bytes, and it
    /// names more nodes than the machine has memory for.
    fn billion_laughs() -> String {
        let mut text = String::from("a: &a [x,x,x,x,x,x,x,x,x,x]\n");
        for (name, previous) in [
            ('b', 'a'),
            ('c', 'b'),
            ('d', 'c'),
            ('e', 'd'),
            ('f', 'e'),
            ('g', 'f'),
        ] {
            let refs = std::iter::repeat_n(format!("*{previous}"), 10)
                .collect::<Vec<_>>()
                .join(",");
            text.push_str(&format!("{name}: &{name} [{refs}]\n"));
        }
        text
    }

    #[test]
    fn validating_yaml_says_how_many_documents_the_file_holds() {
        // A YAML file is a stream. Format writes every document back and
        // Convert to JSON refuses more than one, so the refusal must not be
        // where the user first learns there are three.
        let mut state = yaml_state("one: 1\n---\ntwo: 2\n---\nthree: 3\n");
        state.run_data_action(action::DATA_VALIDATE);
        let notice = state.error.clone().expect("a status message");

        assert!(notice.starts_with('✓'), "got {notice}");
        assert!(
            notice.contains('3'),
            "the count belongs in it; got {notice}"
        );
    }

    fn json_state(text: &str) -> AppState {
        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        state
            .workspace
            .get_mut(id)
            .unwrap()
            .set_path(PathBuf::from("data.json"));
        state.edit(text.to_owned());
        state
    }

    #[test]
    fn json_format_keeps_the_key_order_and_sort_keys_sorts() {
        // Sort Keys shared Format's id and only sorted because Format did.
        // With the author's order kept, the two have to be different rows
        // doing different things.
        let mut state = json_state(r#"{"z":1,"a":2}"#);
        state.run_data_action(action::DATA_FORMAT);
        let formatted = state.active_text();
        assert!(
            formatted.find("\"z\"") < formatted.find("\"a\""),
            "{formatted}"
        );

        state.run_data_action(action::DATA_SORT_KEYS);
        let sorted = state.active_text();
        assert!(sorted.find("\"a\"") < sorted.find("\"z\""), "{sorted}");
    }

    #[test]
    fn formatting_toml_says_what_it_did_to_the_comments() {
        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        state
            .workspace
            .get_mut(id)
            .unwrap()
            .set_path(PathBuf::from("Cargo.toml"));
        state.edit(
            "# keep me
b = 1
a = 2
"
            .to_owned(),
        );

        state.run_data_action(action::DATA_FORMAT);

        assert!(!state.active_text().contains("keep me"));
        let notice = state.error.expect("a notice");
        assert!(
            notice.contains("comments") && notice.contains("Ctrl+Z"),
            "{notice}"
        );
    }

    #[test]
    fn a_single_yaml_document_is_not_counted_out_at_the_user() {
        let mut state = yaml_state("one: 1\n");
        state.run_data_action(action::DATA_VALIDATE);
        assert_eq!(state.error.as_deref(), Some("✓ valid YAML"));
    }

    #[test]
    fn formatting_yaml_says_that_the_comments_did_not_survive() {
        // ADR-0023's one place where "never silently change the user's data"
        // needs a warning rather than an error: `saphyr` parses YAML into
        // data, and a comment is not data.
        let mut state = yaml_state("# why this value matters\nkey:   value\n");
        state.run_data_action(action::DATA_FORMAT);
        let notice = state.error.clone().expect("a status message");

        assert!(
            !state.active_text().contains("why this value matters"),
            "the fixture must actually lose its comment, or this proves nothing"
        );
        assert!(
            notice.contains("comments"),
            "the user has to be told what went; got {notice}"
        );
        assert!(
            notice.contains("Ctrl+Z"),
            "and how to get it back; got {notice}"
        );
    }

    #[test]
    fn minifying_yaml_rewrites_it_in_flow_style_and_says_the_same_thing() {
        let mut state = yaml_state("key:\n  - one\n  - two\n");
        state.run_data_action(action::DATA_MINIFY);
        let notice = state.error.clone().expect("a status message");

        assert!(
            state.active_text().contains('['),
            "got {}",
            state.active_text()
        );
        assert!(notice.contains("comments"), "got {notice}");
    }

    #[test]
    fn yaml_nested_past_the_limit_is_refused_with_the_limit_named() {
        // Not a taste judgement. `saphyr`'s loader recurses one stack frame
        // per level and a stack overflow aborts the process, so this refusal
        // is the only form the answer can take -- and the number has to be in
        // it, because "too deep" is not something a person can act on.
        let deep = format!(
            "{}{}",
            "[".repeat(bp_data::MAX_YAML_NESTING + 1),
            "]".repeat(bp_data::MAX_YAML_NESTING + 1)
        );
        let mut state = yaml_state(&deep);
        state.run_data_action(action::DATA_FORMAT);
        let notice = state.error.clone().expect("a status message");

        assert!(
            notice.contains("left unchanged"),
            "the user has to know the document was not touched; got {notice}"
        );
        assert!(
            notice.contains(&bp_data::MAX_YAML_NESTING.to_string()),
            "the limit belongs in it; got {notice}"
        );
        assert!(
            notice.contains("line 1, column"),
            "and where it was hit; got {notice}"
        );
        assert_eq!(state.active_text(), deep, "nothing may have been rewritten");
    }

    #[test]
    fn yaml_that_expands_past_the_node_budget_is_refused_before_it_is_built() {
        // Around 200 bytes, so neither the file size nor the nesting cap sees
        // it coming -- the document is only ever seven levels deep.
        let laughs = billion_laughs();
        assert!(
            laughs.len() < 400,
            "the fixture must stay small to mean anything"
        );

        let mut state = yaml_state(&laughs);
        state.run_data_action(action::DATA_FORMAT);
        let notice = state.error.clone().expect("a status message");

        assert!(notice.contains("left unchanged"), "got {notice}");
        assert!(
            notice.contains(&bp_data::MAX_YAML_NODES.to_string()),
            "the budget belongs in it; got {notice}"
        );
        assert_eq!(state.active_text(), laughs);
    }

    #[test]
    fn a_duplicate_yaml_key_is_refused_in_words_that_say_why_it_matters() {
        // The whole reason this is an error rather than a merge: `saphyr`
        // drops the earlier value on the way into the map, so formatting the
        // file would delete a line and report success. A message that read
        // like an ordinary parse error would leave the user editing their
        // YAML looking for a missing colon.
        let mut state = yaml_state("name: first\nname: second\n");
        state.run_data_action(action::DATA_FORMAT);
        let notice = state.error.clone().expect("a status message");

        assert!(notice.contains("duplicate key"), "got {notice}");
        assert!(
            notice.contains("replace"),
            "it has to say what would have been lost, not just that it refused; got {notice}"
        );
        assert!(
            notice.contains("line 2"),
            "and which of the two keys to go and look at; got {notice}"
        );
        assert_eq!(state.active_text(), "name: first\nname: second\n");
    }

    #[test]
    fn a_duplicate_key_is_reported_by_validate_as_well_as_by_format() {
        // Validate is where somebody checks a file they are about to hand
        // over, and it is the row that must not answer "fine".
        let mut state = yaml_state("name: first\nname: second\n");
        state.run_data_action(action::DATA_VALIDATE);
        let notice = state.error.clone().expect("a status message");

        assert!(!notice.starts_with('✓'), "got {notice}");
        assert!(notice.contains("duplicate key"), "got {notice}");
    }

    #[test]
    fn converting_a_yaml_stream_to_json_refuses_and_names_the_count() {
        // JSON has one root value. Wrapping three documents in an array would
        // hand back a different shape from the one on screen.
        let mut state = yaml_state("one: 1\n---\ntwo: 2\n");
        state.run_data_action(action::DATA_YAML_TO_JSON);
        let notice = state.error.clone().expect("a status message");

        assert!(notice.contains("left unchanged"), "got {notice}");
        assert!(
            notice.contains('2'),
            "the count belongs in it; got {notice}"
        );
        assert_eq!(state.active_text(), "one: 1\n---\ntwo: 2\n");
    }

    #[test]
    fn converting_one_yaml_document_to_json_replaces_the_document() {
        let mut state = yaml_state("key: value\n");
        state.run_data_action(action::DATA_YAML_TO_JSON);

        assert!(
            state.active_text().contains("\"key\""),
            "got {}",
            state.active_text()
        );
        assert!(state.error.as_deref().is_some_and(|e| e.starts_with('✓')));
    }

    #[test]
    fn converting_json_to_yaml_promises_nothing_was_lost() {
        // The asymmetry ADR-0023 draws: every JSON value has a YAML spelling,
        // so this direction has no warning to carry -- and a warning attached
        // to it anyway would teach the user to ignore the ones that matter.
        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        state
            .workspace
            .get_mut(id)
            .unwrap()
            .set_path(PathBuf::from("data.json"));
        state.edit("{\"key\": \"value\"}".to_owned());
        state.run_data_action(action::DATA_JSON_TO_YAML);

        assert!(
            state.active_text().contains("key: value"),
            "got {}",
            state.active_text()
        );
        assert_eq!(state.error.as_deref(), Some("✓ converted to YAML"));
        assert!(
            !state.error.as_deref().unwrap().contains("comments"),
            "there are no comments in JSON to lose"
        );
    }

    // --- Security ▸ Redact -------------------------------------------------

    /// The same fake credential the scan tests use, on a line with multi-byte
    /// characters in front of it.
    ///
    /// The byte offset of the key and its character column differ by three
    /// here, which is the entire point: `bp_secrets::Finding` counts
    /// characters and `bp_redaction::Span` counts bytes, and a shell that
    /// treated them as the same unit would destroy the wrong text.
    const LEAKY_MULTIBYTE: &str = "«clé» aws_access_key_id = AKIA3G7QVHBRN2WPKZ5F\n";

    /// A PEM block, whose finding covers the BEGIN line and not the key.
    const PEM: &str =
        "-----BEGIN RSA PRIVATE KEY-----\nMIIBOgIBAAJBAKtQ\n-----END RSA PRIVATE KEY-----\n";

    #[test]
    fn redacting_takes_the_credential_out_and_leaves_a_marker_where_it_was() {
        let mut state = AppState::new();
        state.edit(LEAKY.to_owned());
        let plan = state.plan_redaction().expect("a plan");

        assert!(state.apply_redaction(&plan), "the document should change");
        let after = state.active_text();
        assert!(
            !after.contains(LEAKED),
            "the credential is still there: {after}"
        );
        assert!(
            after.contains(bp_redaction::PLACEHOLDER.trim_end_matches(']')),
            "a redaction nobody can see is one nobody will act on; got {after}"
        );
        assert!(
            after.contains("AWS access key ID"),
            "the marker names the rule that matched; got {after}"
        );
        assert!(
            after.starts_with("notes\n"),
            "only the span may go; got {after}"
        );
    }

    #[test]
    fn a_credential_after_multibyte_text_is_located_in_bytes_not_characters() {
        // The conversion this shell owns, and the one place an off-by-three
        // would destroy the wrong words and report success.
        let mut state = AppState::new();
        state.edit(LEAKY_MULTIBYTE.to_owned());
        let plan = state.plan_redaction().expect("a plan");
        assert!(state.apply_redaction(&plan));

        let after = state.active_text();
        assert!(!after.contains(LEAKED), "got {after}");
        assert!(
            after.starts_with("«clé» aws_access_key_id = "),
            "everything before the credential must survive intact; got {after}"
        );
        assert!(
            after.ends_with("]\n"),
            "and nothing after it may be eaten; got {after}"
        );
    }

    #[test]
    fn redaction_is_an_edit_to_the_document_and_not_to_the_file() {
        // Irreversible in the string it returns, and deliberately not
        // irreversible on disk. `bp-redaction` rewrites one string;
        // ADR-0028's first consequence is that a shell which saved over the
        // original would have dealt with exactly one of the places the text
        // lives and told the user it had dealt with all of them.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("leaky.txt");
        let mut state = AppState::new();
        state.edit(LEAKY.to_owned());
        let id = state.workspace.active_id().unwrap();
        assert_eq!(
            state.save_document(id, Some(path.clone())),
            SaveResult::Saved
        );

        let plan = state.plan_redaction().expect("a plan");
        assert!(state.apply_redaction(&plan));

        assert!(
            state.is_dirty(id),
            "the redaction is unsaved work like any edit"
        );
        assert!(
            std::fs::read_to_string(&path).unwrap().contains(LEAKED),
            "nothing may have been written to the file"
        );

        // And it is on the undo stack, which is the honest reading of "undo
        // brings the originals back" in the status bar.
        assert!(state.active_editor_mut().unwrap().undo());
        assert!(state.active_text().contains(LEAKED));
    }

    #[test]
    fn nothing_the_redaction_says_ever_repeats_the_credential() {
        // The guarantee the scan already makes, extended to the half of the
        // feature that has the document in its hand: the plan, the
        // confirmation dialog and every status line it produces.
        let mut state = AppState::new();
        state.edit(LEAKY.to_owned());
        let plan = state.plan_redaction().expect("a plan");
        let planned = state.error.clone().expect("a status message");
        let consent = plan.consent_body();
        assert!(state.apply_redaction(&plan));
        let outcome = state.error.clone().expect("a status message");

        for line in [planned, consent, outcome, plan.refusal()] {
            assert!(
                !line.contains(LEAKED),
                "the credential is back on screen: {line}"
            );
        }
    }

    #[test]
    fn a_private_key_block_is_left_alone_and_named_rather_than_half_redacted() {
        // `bp-secrets` marks the BEGIN line only -- one line, column and
        // length cannot describe a block that runs to a matching END -- so
        // redacting the span would take out the label and leave the key
        // material under a `[REDACTED]` marker. That is the black-rectangle
        // failure ADR-0028 exists to prevent.
        let mut state = AppState::new();
        state.edit(PEM.to_owned());

        assert!(
            state.plan_redaction().is_none(),
            "there is nothing here this can safely destroy"
        );
        let notice = state.error.clone().expect("a status message");
        assert!(notice.contains("nothing was redacted"), "got {notice}");
        assert!(notice.contains("private key"), "got {notice}");
        assert!(
            notice.contains("BEGIN"),
            "the user has to know why, or they will read it as a bug; got {notice}"
        );
        assert!(
            state.active_text().contains("MIIBOgIBAAJBAKtQ"),
            "the key body must be exactly where it was"
        );
    }

    #[test]
    fn a_clean_document_is_told_there_is_nothing_to_redact() {
        let mut state = AppState::new();
        state.edit("nothing to see here\n".to_owned());
        assert!(state.plan_redaction().is_none());
        assert_eq!(
            state.error.as_deref(),
            Some("nothing to redact — no credentials found")
        );
    }

    #[test]
    fn declining_a_redaction_says_so_rather_than_leaving_the_bar_alone() {
        // Cancelling a file dialog leaves the status bar alone because
        // nothing was claimed. Here the user was asked a question, and
        // "I pressed Cancel" and "the row does nothing" must not look alike.
        let mut state = AppState::new();
        state.edit(LEAKY.to_owned());
        let plan = state.plan_redaction().expect("a plan");
        state.decline_redaction(&plan);
        let notice = state.error.clone().expect("a status message");

        assert!(notice.contains("nothing was redacted"), "got {notice}");
        assert!(notice.contains("still in this document"), "got {notice}");
        assert_eq!(state.active_text(), LEAKY);
    }

    #[test]
    fn a_plan_measured_against_text_that_has_since_changed_is_refused() {
        // The consent sheet is answered later, from an event, so something
        // else can edit the buffer while it is open. `RedactionError::PastEnd` only catches
        // the half of that which runs off the end -- an edit that left the
        // length alone would redact the wrong bytes and report success.
        let mut state = AppState::new();
        state.edit(LEAKY.to_owned());
        let plan = state.plan_redaction().expect("a plan");
        state.edit(format!("prefix\n{LEAKY}"));

        assert!(
            !state.apply_redaction(&plan),
            "stale offsets must not be applied"
        );
        let notice = state.error.clone().expect("a status message");
        assert!(notice.contains("nothing was redacted"), "got {notice}");
        assert!(
            notice.contains("changed while the dialog was open"),
            "got {notice}"
        );
        assert!(
            state.active_text().contains(LEAKED),
            "and nothing was destroyed"
        );
    }

    #[test]
    fn a_copy_the_scan_did_not_mark_is_reported_as_surviving() {
        // ADR-0028 is careful that this is a reason to look and not a verdict:
        // every named span was destroyed, and the same text simply also
        // appears where nothing marked it. Reporting it as a failure would
        // teach the user to ignore it; saying nothing would be worse.
        let mut state = AppState::new();
        state.edit(format!(
            "aws_access_key_id = {LEAKED}\nnote: xx{LEAKED}xx\n"
        ));
        let plan = state.plan_redaction().expect("a plan");
        assert_eq!(
            plan.spans.len(),
            1,
            "the fixture must yield exactly one finding"
        );

        assert!(state.apply_redaction(&plan));
        let notice = state.error.clone().expect("a status message");
        assert!(notice.starts_with('⚠'), "got {notice}");
        assert!(notice.contains("still occurs elsewhere"), "got {notice}");
        assert!(
            notice.contains("line 1"),
            "the survivor is reported by position, never by text; got {notice}"
        );
        assert!(!notice.contains(LEAKED), "got {notice}");
    }

    #[test]
    fn the_outcome_says_the_original_is_still_everywhere_else() {
        // The sentence ADR-0028 makes the shell's obligation. Redacting the
        // buffer deals with one of the places the text lives, and a status
        // bar that reads "redacted" full stop tells the same lie the black
        // rectangle does.
        let plan = RedactionPlan::from_scan(LEAKY.to_owned(), &bp_secrets::scan(LEAKY));
        let clean = plan.outcome(1, Some(&[]));

        assert!(clean.starts_with('✓'), "got {clean}");
        assert!(clean.contains("undo"), "got {clean}");
        assert!(clean.contains("disk"), "got {clean}");
    }

    #[test]
    fn a_redaction_whose_check_could_not_run_does_not_report_it_as_clean() {
        let plan = RedactionPlan::from_scan(LEAKY.to_owned(), &bp_secrets::scan(LEAKY));
        let unchecked = plan.outcome(1, None);

        assert!(unchecked.starts_with('⚠'), "got {unchecked}");
        assert!(unchecked.contains("could not"), "got {unchecked}");
    }

    #[test]
    fn the_consent_dialog_lists_what_will_go_and_what_it_does_not_reach() {
        let plan = RedactionPlan::from_scan(LEAKY.to_owned(), &bp_secrets::scan(LEAKY));
        let body = plan.consent_body();

        assert!(body.contains("line 2 — AWS access key ID"), "got {body}");
        assert!(body.contains("not the file on disk"), "got {body}");
        assert!(body.contains("recovery journal"), "got {body}");
        assert!(body.contains("clipboard"), "got {body}");
    }

    // --- Security ▸ Inspect Metadata ---------------------------------------

    #[test]
    fn a_container_this_build_cannot_open_never_reports_an_all_clear() {
        // The failure `Container::hidden` exists to prevent: an inspector
        // that quietly says "no metadata found" about a `.docx` is worse than
        // no inspector, because the user believes it and sends the file.
        let summary = metadata_summary(&[], bp_redaction::Container::OfficeOpenXml);

        assert!(!summary.starts_with('✓'), "got {summary}");
        assert!(summary.contains("author name"), "got {summary}");
        assert!(summary.contains("cannot look"), "got {summary}");
    }

    #[test]
    fn plain_text_says_the_filesystem_entry_is_outside_the_check() {
        // `bp-redaction` is explicit that modification times, ownership and
        // extended attributes are the shell's and not its own, so the shell
        // is the only place that sentence can be said.
        let summary = metadata_summary(&[], bp_redaction::Container::PlainText);
        assert!(summary.starts_with('✓'), "got {summary}");
        assert!(summary.contains("ownership"), "got {summary}");

        let report = metadata_report(&[], bp_redaction::Container::PlainText);
        assert!(report.contains("alternate data streams"), "got {report}");
    }

    #[test]
    fn the_report_for_a_container_names_what_a_build_would_need_to_read_it() {
        let report = metadata_report(&[], bp_redaction::Container::Pdf);
        assert!(report.contains("PDF parser"), "got {report}");
        assert!(
            report.contains("all-clear"),
            "the reader has to be told not to take the empty list as one; got {report}"
        );
    }

    #[test]
    fn a_metadata_report_names_positions_and_kinds_and_never_the_value() {
        // The same rule as the secret scan, and for the same reason: a dialog
        // is a thing people screenshot into bug reports.
        let text = "author: Daniel Boles <daniel@example.com>\nnotes\n";
        let findings = bp_redaction::inspect(text);
        assert!(!findings.is_empty(), "the fixture must actually be found");

        for line in [
            metadata_summary(&findings, bp_redaction::Container::PlainText),
            metadata_report(&findings, bp_redaction::Container::PlainText),
        ] {
            assert!(!line.contains("Daniel Boles"), "got {line}");
            assert!(!line.contains("daniel@example.com"), "got {line}");
            assert!(line.contains("line 1"), "got {line}");
        }
    }

    #[test]
    fn inspecting_a_clean_plain_document_says_so_without_opening_a_dialog() {
        let mut state = AppState::new();
        state.edit("nothing identifying here\n".to_owned());

        assert!(
            state.inspect_metadata().is_none(),
            "a dialog with nothing in it is worse than the status bar line"
        );
        assert!(state.error.as_deref().is_some_and(|e| e.starts_with('✓')));
    }

    #[test]
    fn inspecting_a_document_in_a_container_opens_the_dialog_even_when_the_text_is_clean() {
        let mut state = AppState::new();
        let id = state.workspace.active_id().unwrap();
        state
            .workspace
            .get_mut(id)
            .unwrap()
            .set_path(PathBuf::from("report.docx"));
        state.edit("nothing identifying here\n".to_owned());

        let report = state
            .inspect_metadata()
            .expect("the part that was not looked at is the answer");
        assert!(report.contains("ZIP reader"), "got {report}");
    }

    #[test]
    fn container_of_recognises_the_shapes_a_text_only_inspection_cannot_open() {
        use bp_redaction::Container;

        for (name, expected) in [
            ("notes.txt", Container::PlainText),
            ("notes", Container::PlainText),
            ("report.DOCX", Container::OfficeOpenXml),
            ("sheet.ods", Container::OpenDocument),
            ("paper.pdf", Container::Pdf),
            ("letter.rtf", Container::RichText),
            ("photo.JPEG", Container::Image),
        ] {
            assert_eq!(
                container_of(Some(&PathBuf::from(name))),
                expected,
                "{name} was classified wrongly"
            );
        }
        assert_eq!(
            container_of(None),
            Container::PlainText,
            "a document that has never been saved claims nothing about hidden metadata"
        );
    }
}
