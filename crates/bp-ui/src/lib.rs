//! The BachelorPad+ application shell.
//!
//! This crate owns the window and nothing else. Document state lives in
//! `bp-core`, saving in `bp-files`, naming in `bp-naming`, colours in
//! `bp-theme`. The rule from `docs/architecture/ARCHITECTURE.md` runs one
//! way: the UI may call those, and none of them know this crate exists.
//!
//! Text is held in `bp-editor`'s rope -- `HashMap<DocumentId, Editor>` --
//! whichever view is drawing (ADR-0018). It was a `HashMap<DocumentId,
//! String>` once, and why that changed is worth keeping: Slint's `TextInput`
//! owns its own text and caret, hands the whole buffer back on every edit,
//! keeps its own undo stack, and exposes the caret only through a property
//! marked "internal, undocumented, only exposed for tests". The rope became
//! the storage independently of which view draws, which is what let the two
//! be separated at all.
//!
//! So there are two views over one model. Under `TextInput`, still the
//! default, text arriving from the widget lands in the rope as a single
//! undoable replacement rather than as keystrokes, undo is Slint's, and the
//! status bar can offer only a line count. Under `--editor-view` this crate
//! owns the caret, `bp-editor`'s transactions are the undo, and the status
//! bar shows Ln/Col; `editor_view.rs` owns that path.
//!
//! Both costs are measured. The `TextInput` path copies the document on
//! every keystroke, so it stays inside a frame budget only to roughly 40 MB;
//! the rope path does not care how large the document is. See
//! `--latency-probe`, ADR-0018 and `docs/architecture/ARCHITECTURE.md`.

// `deny` rather than `forbid`: Slint's build script generates the window's
// item tree into this crate, and that generated code needs `unsafe` with a
// local `allow`. `forbid` cannot be overridden even by generated code, so it
// fails the build. `deny` still stops hand-written `unsafe` here.
#![deny(unsafe_code)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;

use bp_core::{Document, DocumentId};
use bp_theme::{Palette as ThemePalette, ThemeId};

// Behind a module so the workspace lints do not judge it. Slint's generator
// emits some four hundred `unwrap()`s and `panic!`s against an item tree it
// has just built itself, none of which is a defect and none of which has a
// source file anybody could fix. Left at the crate root it would drown the
// lint that exists to keep *hand-written* code from panicking in somebody's
// editor with their document in it.
//
// The same accommodation as the `deny(unsafe_code)` above rather than
// `forbid`: narrow, aimed at the generator, and it leaves the rule standing
// everywhere else in this crate.
mod generated {
    #![allow(
        clippy::unwrap_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented
    )]
    slint::include_modules!();
}
pub use generated::*;

mod menus;

mod about;
mod audit;
mod default_editor;
mod dialog;
mod dispatch;
mod editor_view;
mod passphrase;
mod settings_sheet;
mod state;
#[cfg(test)]
mod window_tests;

/// Crate identity used by workspace smoke tests and diagnostics.
pub const CRATE_NAME: &str = "bp-ui";

#[derive(Debug, thiserror::Error)]
pub enum UiError {
    #[error("could not start the user interface: {0}")]
    Platform(#[from] slint::PlatformError),
}

fn apply_theme(ui: &AppWindow, theme: ThemeId) {
    let p: ThemePalette = theme.palette();
    let c = |v: bp_theme::Rgb| slint::Color::from_rgb_u8(v.r, v.g, v.b);
    let palette = ui.global::<Palette>();
    palette.set_shell(c(p.shell));
    palette.set_panel(c(p.panel));
    palette.set_edge(c(p.edge));
    palette.set_ink(c(p.ink));
    palette.set_ink_dim(c(p.ink_dim));
    palette.set_accent(c(p.accent));
}

fn refresh(ui: &AppWindow, state: &mut state::AppState, push_text: state::PushText) {
    apply_theme(ui, state.theme);
    ui.set_theme_name(state.theme.name().into());

    let active = state.workspace.active_id();
    let tabs: Vec<TabInfo> = state
        .workspace
        .iter()
        .map(|doc| TabInfo {
            id: i32::try_from(doc.id().get()).unwrap_or(i32::MAX),
            label: doc.display_name().into(),
            dirty: doc.is_dirty(),
            active: Some(doc.id()) == active,
        })
        .collect();
    ui.set_tabs(Rc::new(slint::VecModel::from(tabs)).into());

    if push_text == state::PushText::Yes {
        ui.set_doc_text(state.active_text().as_str().into());
    }
    if state.editor_view {
        editor_view::push_editor_view(ui, state);
    }
    if state.sync_gutter() {
        ui.set_gutter(state.gutter.as_str().into());
    }
    let cursor = state.cursor_label();
    let format = state.format();

    if let Some(doc) = state.workspace.active() {
        ui.set_save_state(state::save_state_label(doc).into());
        ui.set_is_dirty(doc.is_dirty());
        ui.set_location(doc.location_label().into());
        ui.set_encoding_label(doc.encoding().label().into());
        ui.set_line_ending_label(doc.line_ending().label().into());
        ui.set_format_label(format.label().into());
        ui.set_cursor_label(cursor.as_str().into());
    }
    ui.set_overwrite(state.overwrite);

    // Something that just failed outranks a standing warning about the file.
    let notice = state
        .error
        .as_deref()
        .or(state.disk_warning.as_deref())
        .unwrap_or_default();
    ui.set_error_message(notice.into());

    // Read on every refresh rather than once, because the desktop's setting
    // can change while the window is open -- which is the whole point of
    // following it.
    state.system_dark = ui.get_system_known().then(|| ui.get_system_dark());
    if state.follow_system_theme
        && let Some(theme) = ThemeId::for_system(state.system_dark)
    {
        state.theme = theme;
        apply_theme(ui, state.theme);
        ui.set_theme_name(state.theme.name().into());
    }

    // Empty under Standard: a permanent "Standard" would be noise on every
    // document, and the readout earns its place only when something unusual
    // is in force.
    let security = state.security();
    // Privacy Mode outranks the profile name here: it is the session-wide
    // fact, and it is the thing somebody switches on precisely because they
    // want to be able to see that it is on.
    ui.set_security_profile(
        if state.privacy.is_on() {
            "Privacy Mode".to_owned()
        } else if security == bp_security::Security::default() {
            String::new()
        } else {
            security.name().to_owned()
        }
        .into(),
    );

    // What the passphrase bar is asking, if anything.
    if let Some(ask) = &state.ask {
        ui.set_passphrase_prompt(ask.prompt().into());
        ui.set_passphrase_action(ask.action().into());
    }
    ui.set_passphrase_status(state.passphrase_status.as_str().into());

    ui.set_show_planned_menus(state.show_planned_menus);
    ui.set_show_gutter(state.show_gutter);
    ui.set_wrap_text(state.wrap_text);
    // Points to Slint's `length`. Both editor views read this one property,
    // so a zoom cannot apply to the widget and not to the surface.
    ui.set_font_size(f32::from(state.font_size));

    // Only the menus whose contents depend on state are rebuilt here; the
    // rest are set once at startup.
    let any_dirty = state.workspace.dirty().next().is_some();
    let has_path = state.workspace.active().and_then(Document::path).is_some();
    let model = |items: Vec<MenuItem>| slint::ModelRc::new(slint::VecModel::from(items));

    ui.set_file_items(model(menus::file(
        any_dirty,
        has_path,
        state.recent.paths(),
    )));
    ui.set_view_items(model(menus::view(
        state.theme,
        state.follow_system_theme,
        state.show_gutter,
        state.wrap_text,
        state.font_size,
    )));
    if let Some(doc) = state.workspace.active() {
        ui.set_format_items(model(menus::format(
            doc.encoding(),
            doc.line_ending(),
            state.indent,
        )));
    }
    // Rebuilt rather than set once, because the previews are rendered from
    // the clock: a menu built at startup would still be offering this
    // morning's time this afternoon.
    ui.set_insert_items(model(menus::insert(state::now())));
    ui.set_data_items(model(menus::data(format)));
    ui.set_note_items(model(menus::note(state.active_has_content())));
    ui.set_edit_items(model(menus::edit(state.clips.entries())));
    // Rebuilt rather than set once: it shows the *active* document's profile
    // and what that profile permits, both of which change with the tab.
    let encrypted = state
        .workspace
        .active_id()
        .is_some_and(|id| state.is_encrypted(id));
    ui.set_security_items(model(menus::security(
        state.security(),
        encrypted,
        state.privacy,
        // Redaction needs something to redact. A prefix check rather than a
        // scan: this runs on every refresh, and scanning the whole document
        // there is the trap R011 spends a paragraph on.
        state.active_has_content(),
        // Verification reads the *file*, and looks for the sidecar beside it
        // by name. Already computed above for Reload, which needs the same
        // fact for the same underlying reason.
        has_path,
    )));
}

/// Menus whose contents never change. Set once, not on every refresh.
fn set_static_menus(ui: &AppWindow) {
    let model = |items: Vec<MenuItem>| slint::ModelRc::new(slint::VecModel::from(items));
    ui.set_help_items(model(menus::help()));
    ui.set_notebook_items(model(menus::planned_menu("Notebook")));
    ui.set_organize_items(model(menus::planned_menu("Organize")));
    ui.set_research_items(model(menus::planned_menu("Research")));
    ui.set_run_items(model(menus::planned_menu("Run")));
    ui.set_tools_items(model(menus::tools()));
}

/// The query the find bar currently describes.
///
/// Every search path reads its options through here, so that a toggle cannot
/// apply to Find and quietly not to Replace All -- which would be a silent
/// wrong answer rather than a visible bug. The decision it wraps lives in
/// `state::find_query`, where it can be tested without a window.
fn ui_query(ui: &AppWindow) -> bp_search::Query {
    state::find_query(
        &ui.get_find_query(),
        ui.get_find_case_sensitive(),
        ui.get_find_whole_word(),
        ui.get_find_regex(),
    )
}

/// Whatever the OS clipboard holds, if it holds text.
/// Read the OS clipboard, if it holds text.
///
/// Failures are silent: a clipboard held open by another process is normal
/// and momentary, and there is nothing useful to say about it.
pub(crate) fn read_os_clipboard() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// Put text on the OS clipboard. Returns whether it worked.
///
/// `keep_private` asks the OS to keep this copy out of its own clipboard
/// history, and on Windows out of cloud clipboard sync and clipboard
/// monitors -- `bp_clipboard::keep_out_of_os_history` decides when. It is a
/// request, honoured by Windows' Win+V history and by KDE's Klipper; a
/// clipboard manager that ignores the hint still sees the text.
pub(crate) fn set_os_clipboard(text: &str, keep_private: bool) -> bool {
    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        return false;
    };
    let set = clipboard.set();
    #[cfg(windows)]
    let set = if keep_private {
        use arboard::SetExtWindows as _;
        set.exclude_from_history()
            .exclude_from_cloud()
            .exclude_from_monitoring()
    } else {
        set
    };
    #[cfg(target_os = "linux")]
    let set = if keep_private {
        use arboard::SetExtLinux as _;
        set.exclude_from_history()
    } else {
        set
    };
    #[cfg(not(any(windows, target_os = "linux")))]
    let _ = keep_private;
    set.text(text.to_owned()).is_ok()
}

/// Which Slint renderer to ask for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Renderer {
    /// CPU rasterisation. The default, per ADR-0017: on this workload it is
    /// the only configuration that meets the specs.md section 22 budgets --
    /// 36 ms to window and 19 MB idle, against 129 ms and 63 MB for the GPU
    /// path, which also spends ~410 ms reaching its first frame.
    #[default]
    Software,
    /// Whatever Slint would pick on its own, normally GPU-accelerated.
    Platform,
}

/// How to run the shell.
#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    pub renderer: Renderer,
    /// Starting theme. `None` uses the built-in default.
    pub theme: Option<ThemeId>,
    /// Follow the desktop's light or dark preference rather than `theme`.
    pub follow_system_theme: bool,
    /// The resolved configuration, and where to save settings chosen in the
    /// window (release plan M1-03). `None` for both keeps the built-in
    /// defaults and saves nothing, which is what `run()` and the tests want.
    pub config: Option<bp_config::Config>,
    pub settings_path: Option<PathBuf>,
    /// Where the log and crash reports are written, for Tools ▸ Open Log
    /// Folder. Passed in rather than worked out again here, so the folder
    /// opened is the one the binary actually writes to, override and all.
    pub log_dir: Option<PathBuf>,
    /// Shown in the status bar at startup, where configuration problems go.
    /// A warning the user cannot see is a warning they will hit again.
    pub startup_notice: Option<String>,
    /// Files named on the command line, opened at startup (specs.md §19).
    pub files: Vec<PathBuf>,
    /// Print `BPSPIKE_READY_MS=<f64>` once the first frame has been rendered,
    /// then quit. Drives `scripts/Measure-UiSpike.ps1`.
    ///
    /// The instrumentation lives in the real application rather than in a
    /// separate benchmark binary on purpose: a fixture measures the fixture,
    /// and drifts from the product the moment either changes.
    pub measure_exit: bool,
    /// Draw with the custom editor surface instead of Slint's `TextInput`.
    ///
    /// Opt-in while it reaches parity: word wrap and input-method
    /// composition still belong to the widget it replaces. The rope is the
    /// storage either way -- only the drawing and the caret differ -- so this
    /// is a choice of view, not of document model.
    pub editor_view: bool,
    /// Editor font size in points, already through `bp-config`'s bounds.
    ///
    /// `None` keeps `bp_config::DEFAULT_FONT_SIZE`, so a caller that does not
    /// care -- `run()`, and every test -- gets the size the editor has always
    /// drawn at.
    pub font_size: Option<u8>,
}

/// Run the BachelorPad+ shell.
pub fn run() -> Result<(), UiError> {
    run_with(RunOptions::default())
}

/// Run the shell with explicit options.
pub fn run_with(options: RunOptions) -> Result<(), UiError> {
    let started = std::time::Instant::now();

    // An explicit SLINT_BACKEND wins: someone debugging a rendering problem
    // has set it deliberately, and silently overriding them would waste an
    // afternoon. Selection must happen before the first window is created.
    // Said in Help ▸ About as it was decided here, rather than guessed at
    // later: Slint has no call that reports which renderer it picked.
    let renderer = if let Ok(asked) = std::env::var("SLINT_BACKEND") {
        format!("as SLINT_BACKEND={asked} asks")
    } else if options.renderer == Renderer::Software {
        // A failure here is not fatal -- the platform default still draws a
        // usable window, just a heavier one.
        match slint::BackendSelector::new()
            .renderer_name("software".to_owned())
            .select()
        {
            Ok(()) => "software renderer".to_owned(),
            Err(e) => {
                tracing::warn!("software renderer unavailable, using platform default: {e}");
                "the platform's default (the software renderer was unavailable)".to_owned()
            }
        }
    } else {
        "the platform's default".to_owned()
    };

    let ui = AppWindow::new()?;
    let mut initial = state::AppState::new();
    if let Some(config) = &options.config {
        initial.configure(config, options.settings_path.clone());
    }
    if let Some(theme) = options.theme {
        initial.theme = theme;
    }
    initial.follow_system_theme = options.follow_system_theme;
    initial.log_dir = options.log_dir.clone();
    initial.renderer = renderer;
    if let Some(size) = options.font_size {
        // Clamped again rather than trusted. `bp-config` bounds what it
        // parses, but `RunOptions` is a public API and a caller reaching it
        // by another route must not be able to hand the editor a 0 pt font.
        initial.font_size = bp_config::zoom(size, 0);
    }
    initial.start_up(options.startup_notice.clone(), &options.files);

    // What a previous session left unsaved is claimed now, before anything
    // else can journal, and offered once the window is up -- see
    // `offer_recovery`. Claimed here, so of two windows opened at once only
    // one asks.
    initial.journal.clean_temporaries();
    let pending = initial.journal.pending();
    // Work another window is still journalling is neither offered nor
    // touched here -- it is that window's. Said, so nobody wonders where it
    // went.
    let elsewhere = initial.journal.held_elsewhere();
    if elsewhere > 0 {
        initial.add_notice(format!(
            "{elsewhere} unsaved document(s) are open in another BachelorPad+ window"
        ));
    }

    initial.editor_view = options.editor_view;
    let state = Rc::new(RefCell::new(initial));
    ui.set_use_editor_view(options.editor_view);

    let mut reported = false;
    let measure_exit = options.measure_exit;
    let hooked = ui
        .window()
        .set_rendering_notifier(move |render_state, _api| {
            if reported || !matches!(render_state, slint::RenderingState::AfterRendering) {
                return;
            }
            reported = true;
            println!(
                "BPSPIKE_READY_MS={:.3}",
                started.elapsed().as_secs_f64() * 1000.0
            );
            if measure_exit {
                // Quitting inside a render callback would re-enter the
                // renderer; defer it to the next event-loop turn.
                let _ = slint::invoke_from_event_loop(|| {
                    let _ = slint::quit_event_loop();
                });
            }
        })
        .is_ok();

    // Slint's software renderer has no rendering notifier. Say so rather than
    // reporting a fast-looking number that is really a time-to-exit.
    if !hooked && measure_exit {
        println!("BPSPIKE_READY_UNAVAILABLE=this renderer has no rendering notifier");
        return Ok(());
    }

    macro_rules! wire {
        ($setter:ident, |$s:ident| $body:block) => {{
            let cell = Rc::clone(&state);
            let weak = ui.as_weak();
            ui.$setter(move || {
                {
                    let mut $s = cell.borrow_mut();
                    $body
                }
                if let Some(ui) = weak.upgrade() {
                    refresh(&ui, &mut cell.borrow_mut(), state::PushText::Yes);
                }
            });
        }};
    }

    wire!(on_new_document, |s| {
        s.new_document();
    });

    // Ctrl+O, the same as the menu row -- which, unlike the handler that
    // used to be here, knows an encrypted file from a plain one.
    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_open_document(move || {
            if let Some(ui) = weak.upgrade() {
                let _ = dispatch::handle_menu_action(&ui, &cell, menus::action::OPEN);
            }
        });
    }

    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_save_document(move || {
            let Some(ui) = weak.upgrade() else { return };
            let id = cell.borrow().workspace.active_id();
            match id {
                Some(id) => save_with_prompt(&ui, &cell, id, refresh_after(&cell)),
                None => refresh(&ui, &mut cell.borrow_mut(), state::PushText::Yes),
            }
        });
    }

    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_save_as_document(move || {
            let Some(ui) = weak.upgrade() else { return };
            let id = cell.borrow().workspace.active_id();
            match id {
                Some(id) => save_as(&ui, &cell, id),
                None => refresh(&ui, &mut cell.borrow_mut(), state::PushText::Yes),
            }
        });
    }

    wire!(on_cycle_theme, |s| {
        s.theme = s.theme.next();
        s.follow_system_theme = false;
        let name = s.theme.name();
        s.remember(|c| c.theme = Some(name.to_owned()));
    });

    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        let target = Rc::clone(&state);
        let target_weak = ui.as_weak();
        ui.on_tab_context_target(move |raw| {
            let Some(ui) = target_weak.upgrade() else {
                return;
            };
            let mut s = target.borrow_mut();
            s.tab_context = state::find_id(&s.workspace, raw);
            // Built now rather than on every refresh: it is the only menu
            // whose contents depend on which tab the pointer is over, and
            // that is knowable only at this moment.
            let (others, has_path) = s.tab_context_shape();
            ui.set_tab_items(slint::ModelRc::new(slint::VecModel::from(
                menus::tab_context(others, has_path),
            )));
        });

        ui.on_select_tab(move |raw| {
            {
                let mut s = cell.borrow_mut();
                if let Some(id) = state::find_id(&s.workspace, raw) {
                    s.workspace.set_active(id);
                }
            }
            if let Some(ui) = weak.upgrade() {
                refresh(&ui, &mut cell.borrow_mut(), state::PushText::Yes);
            }
        });
    }

    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_close_tab(move |raw| {
            let id = cell
                .borrow()
                .workspace
                .iter()
                .map(Document::id)
                .find(|id| i32::try_from(id.get()).unwrap_or(i32::MAX) == raw);
            let Some(ui) = weak.upgrade() else { return };
            match id {
                Some(id) => close_with_prompt(&ui, &cell, id, refresh_after(&cell)),
                None => refresh(&ui, &mut cell.borrow_mut(), state::PushText::Yes),
            }
        });
    }

    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_close_active_tab(move || {
            let Some(ui) = weak.upgrade() else { return };
            let id = cell.borrow().workspace.active_id();
            match id {
                Some(id) => close_with_prompt(&ui, &cell, id, refresh_after(&cell)),
                None => refresh(&ui, &mut cell.borrow_mut(), state::PushText::Yes),
            }
        });
    }

    // Closing the window must not be a quieter way to lose the same work
    // that closing a tab prompts about.
    //
    // The answer to a close request has to be given now, and the questions
    // are answered later, so a window with unsaved work refuses the request
    // and asks; `settle_then_quit` closes it once every document is settled.
    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.window().on_close_requested(move || {
            // A question is already open. It is answered first, whatever it
            // is about -- quitting past the recovery prompt would leave the
            // work it is asking about neither restored nor kept on purpose.
            if dialog::is_open() {
                return slint::CloseRequestResponse::KeepWindowShown;
            }
            let dirty: VecDeque<DocumentId> =
                cell.borrow().workspace.dirty().map(Document::id).collect();
            if dirty.is_empty() {
                return slint::CloseRequestResponse::HideWindow;
            }
            if let Some(ui) = weak.upgrade() {
                settle_then_quit(&ui, &cell, dirty, Vec::new());
            }
            slint::CloseRequestResponse::KeepWindowShown
        });
    }

    bind_editing(&ui, &state);
    bind_settings(&ui, &state);
    about::bind(&ui, &state);

    // Every in-app question reports here; `dialog` runs what was waiting on
    // the answer.
    {
        let weak = ui.as_weak();
        ui.on_dialog_answered(move |chosen| {
            if let Some(ui) = weak.upgrade() {
                dialog::answered(&ui, chosen);
            }
        });
    }

    // Menu dispatch. Ids of 100 and above never arrive here -- Slint handles
    // those on the TextInput itself.
    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_menu_action(move |id| {
            let Some(ui) = weak.upgrade() else { return };
            let _ = dispatch::handle_menu_action(&ui, &cell, id);
        });
    }

    // --- overwrite mode, for the `TextInput` path -----------------------
    // The custom surface reaches `toggle_overwrite` through its own key
    // handler; `TextInput` has no Rust key handler, so Insert arrives here.
    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_toggle_overwrite(move || {
            let Some(ui) = weak.upgrade() else { return };
            cell.borrow_mut().toggle_overwrite();
            refresh(&ui, &mut cell.borrow_mut(), state::PushText::No);
        });
    }
    ui.on_overwrite_end(editor_view::overwrite_end);

    // --- copies made in the `TextInput` view ----------------------------
    // Slint copies and cuts by itself, so without this the history only ever
    // learned of them by polling -- and a poll cannot tell a copy from this
    // window from one made in a password manager. Returns whether Rust put
    // the text on the clipboard itself, which it does for a private copy so
    // the OS can be asked to keep it out of its own history.
    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_copied_from_editor(move |text, anchor, cursor, cut| {
            let Some(ui) = weak.upgrade() else {
                return false;
            };
            let Some(selected) = editor_view::selected_text(&text, anchor, cursor) else {
                return false;
            };
            let copied_privately = {
                let mut s = cell.borrow_mut();
                s.record_copy(selected);
                // A cut is left to the widget: `TextInput` has no way to
                // delete a selection from outside that keeps its own undo
                // history intact. So a private *cut* in this view reaches
                // the OS clipboard unmarked -- a known gap, recorded in the
                // release plan, closed when the default view's editing moves
                // to Rust.
                !cut && s.keep_clipboard_private() && set_os_clipboard(selected, true)
            };
            refresh(&ui, &mut cell.borrow_mut(), state::PushText::No);
            copied_privately
        });
    }

    // --- the custom editor surface -------------------------------------
    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_editor_key(move |text, control, shift, alt| {
            let Some(ui) = weak.upgrade() else {
                return false;
            };
            let modifiers = bp_editor::Modifiers {
                control,
                shift,
                alt,
            };

            let handled = {
                let mut s = cell.borrow_mut();
                let rows = s.visible_rows;

                // A backend can deliver more than one character at once --
                // an input method committing, most often. Insert the lot
                // rather than all but the first.
                let command = if text.chars().count() > 1 && !modifiers.control {
                    bp_editor::Command::Insert(text.to_string())
                } else {
                    match editor_view::translate_key(&text) {
                        Some(key) => bp_editor::keys::command_for(key, modifiers, rows),
                        None => bp_editor::Command::Ignore,
                    }
                };
                editor_view::apply_editor_command(&mut s, &command)
            };

            if handled {
                refresh(&ui, &mut cell.borrow_mut(), state::PushText::No);
            }
            handled
        });
    }

    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_editor_pressed(move |row, column, extend| {
            let Some(ui) = weak.upgrade() else { return };
            editor_view::place_caret(&mut cell.borrow_mut(), row, column, extend);
            refresh(&ui, &mut cell.borrow_mut(), state::PushText::No);
        });
    }

    // Double- and triple-click. `EditorSurface` only; `TextInput` does both
    // natively and never reports a press to us, so wiring this for that path
    // would be wiring it twice and getting two behaviours.
    for what in [
        editor_view::ClickSelection::Word,
        editor_view::ClickSelection::Line,
    ] {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        let select = move |row, column| {
            let Some(ui) = weak.upgrade() else { return };
            editor_view::select_at_cell(&mut cell.borrow_mut(), row, column, what);
            refresh(&ui, &mut cell.borrow_mut(), state::PushText::No);
        };
        match what {
            editor_view::ClickSelection::Word => ui.on_editor_selected_word(select),
            editor_view::ClickSelection::Line => ui.on_editor_selected_line(select),
        }
    }

    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_editor_dragged(move |row, column| {
            let Some(ui) = weak.upgrade() else { return };
            // A drag always extends -- that is what dragging means.
            editor_view::place_caret(&mut cell.borrow_mut(), row, column, true);
            refresh(&ui, &mut cell.borrow_mut(), state::PushText::No);
        });
    }

    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_editor_scrolled(move |lines| {
            let Some(ui) = weak.upgrade() else { return };
            let mut s = cell.borrow_mut();

            // By visual rows, not document lines. With wrapping on the two
            // differ, and a wheel that moved whole lines would skip past
            // everything the reader can see inside a long one.
            //
            // `step_row` is bounded at both ends of the document, so the view
            // cannot scroll into empty space.
            s.sync_wrap();
            let anchor = s.anchor;
            let layout = s.layout();
            if let Some(editor) = s.active_editor() {
                s.anchor = bp_editor::view::step_row(
                    editor.buffer(),
                    anchor,
                    isize::try_from(lines).unwrap_or(0),
                    layout,
                );
            }

            // Drawn where it now is rather than through `refresh`: scrolling
            // away from the caret is exactly what the user asked for, and
            // revealing it again would snap the wheel straight back.
            editor_view::draw_editor_view(&ui, &s);
        });
    }

    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_editor_resized(move |rows, columns| {
            let Some(ui) = weak.upgrade() else { return };
            {
                let mut s = cell.borrow_mut();
                s.visible_rows = usize::try_from(rows).unwrap_or(1).max(1);
                // The column count comes from Slint because Slint owns the
                // font. A width derived from anything but the advance it
                // actually draws with would wrap in the wrong place, and
                // visibly so on every line.
                s.wrap_columns = usize::try_from(columns).unwrap_or(80).max(1);
            }
            refresh(&ui, &mut cell.borrow_mut(), state::PushText::No);
        });
    }

    // --- find and replace ---------------------------------------------
    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_find_changed(move || {
            let Some(ui) = weak.upgrade() else { return };
            let query = ui_query(&ui);
            let selection = cell.borrow_mut().find(&query);
            ui.set_find_status(cell.borrow().find_status.as_str().into());
            if let Some(range) = selection {
                dispatch::select(&ui, &mut cell.borrow_mut(), &range);
            }
        });
    }

    for forward in [true, false] {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        let step = move || {
            let Some(ui) = weak.upgrade() else { return };
            let selection = cell.borrow_mut().step_match(forward);
            ui.set_find_status(cell.borrow().find_status.as_str().into());
            if let Some(range) = selection {
                dispatch::select(&ui, &mut cell.borrow_mut(), &range);
            }
        };
        if forward {
            ui.on_find_next(step);
        } else {
            ui.on_find_previous(step);
        }
    }

    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_replace_all(move || {
            let Some(ui) = weak.upgrade() else { return };
            let query = ui_query(&ui);
            let replacement = ui.get_replace_query().to_string();

            // Planned first, and the plan shown, before anything changes:
            // specs.md section 6 wants the changes visible before they are
            // applied. The borrow ends before the dialog blocks -- a native
            // dialog can pump events, and holding a `RefCell` across one is
            // how a re-entrant callback panics.
            let planned = {
                let s = cell.borrow();
                bp_search::plan_replace_all(&s.active_text(), &query, &replacement)
            };

            match planned {
                Err(e) => cell.borrow_mut().find_status = e.to_string(),
                Ok(plan) if plan.is_empty() => {
                    cell.borrow_mut().find_status = "no matches".to_owned();
                }
                Ok(plan) => {
                    // The rest happens once the preview is answered.
                    let cell = Rc::clone(&cell);
                    dispatch::confirm_replace(&ui, plan, move |ui, plan| {
                        {
                            let mut s = cell.borrow_mut();
                            match plan {
                                None => s.find_status = "cancelled".to_owned(),
                                Some(plan) => {
                                    let n = plan.count();
                                    // An ordinary edit, so it is undoable and
                                    // nothing reaches disk until the user
                                    // saves.
                                    s.edit(plan.apply());
                                    s.matches.clear();
                                    s.find_status = format!("replaced {n}");
                                }
                            }
                        }
                        ui.set_find_status(cell.borrow().find_status.as_str().into());
                        refresh(ui, &mut cell.borrow_mut(), state::PushText::Yes);
                    });
                    return;
                }
            }
            ui.set_find_status(cell.borrow().find_status.as_str().into());
            refresh(&ui, &mut cell.borrow_mut(), state::PushText::Yes);
        });
    }

    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_passphrase_submitted(move |entered| {
            let Some(ui) = weak.upgrade() else { return };
            let again = cell.borrow_mut().answer_passphrase(&entered);
            // Wiped whatever happened. A passphrase left in the widget
            // outlives the question it answered, and the next prompt would
            // start pre-filled with the last one.
            ui.invoke_clear_passphrase();
            ui.set_passphrase_open(again);
            if again {
                ui.invoke_focus_passphrase();
            } else {
                ui.invoke_focus_editor();
            }
            refresh(&ui, &mut cell.borrow_mut(), state::PushText::Yes);
        });
    }

    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_passphrase_cancelled(move || {
            let Some(ui) = weak.upgrade() else { return };
            {
                let mut s = cell.borrow_mut();
                // Dropping the ask drops any half-entered passphrase with it
                // -- the first of two entries travels inside `Ask::Confirm`
                // precisely so it cannot outlive the question.
                s.ask = None;
                s.passphrase_status.clear();
            }
            ui.invoke_clear_passphrase();
            ui.set_passphrase_open(false);
            ui.invoke_focus_editor();
            refresh(&ui, &mut cell.borrow_mut(), state::PushText::No);
        });
    }

    // --- cross-file search ---------------------------------------------
    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_search_folder(move || {
            let Some(ui) = weak.upgrade() else { return };
            let query = ui_query(&ui);
            if query.is_empty() {
                ui.set_results_summary("type something to search for".into());
                ui.set_results_open(true);
                return;
            }

            let start = cell.borrow().dialog_directory();
            let Some(folder) = rfd::FileDialog::new().set_directory(start).pick_folder() else {
                return;
            };

            match bp_search::search_dir(&folder, &query) {
                Ok(report) => {
                    let rows: Vec<SearchHit> = report
                        .hits
                        .iter()
                        .enumerate()
                        .map(|(index, hit)| SearchHit {
                            // Relative to the searched folder: the shared
                            // prefix is the same on every row and identifies
                            // nothing.
                            label: format!(
                                "{}:{}",
                                hit.path
                                    .strip_prefix(&folder)
                                    .unwrap_or(&hit.path)
                                    .display(),
                                hit.line
                            )
                            .into(),
                            detail: hit.preview.as_str().into(),
                            index: i32::try_from(index).unwrap_or(i32::MAX),
                        })
                        .collect();

                    ui.set_search_hits(Rc::new(slint::VecModel::from(rows)).into());
                    ui.set_results_summary(report.summary().as_str().into());
                    cell.borrow_mut().file_hits = report.hits;
                }
                Err(e) => {
                    ui.set_search_hits(Rc::new(slint::VecModel::from(Vec::new())).into());
                    ui.set_results_summary(e.to_string().as_str().into());
                    cell.borrow_mut().file_hits.clear();
                }
            }
            ui.set_results_open(true);
        });
    }

    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        ui.on_open_hit(move |index| {
            let Some(ui) = weak.upgrade() else { return };
            let hit = usize::try_from(index)
                .ok()
                .and_then(|i| cell.borrow().file_hits.get(i).cloned());
            let Some(hit) = hit else { return };

            cell.borrow_mut().open(hit.path);
            refresh(&ui, &mut cell.borrow_mut(), state::PushText::Yes);
            // Select the match so the editor scrolls to it, rather than
            // opening the file at the top and leaving the user to hunt.
            dispatch::select(&ui, &mut cell.borrow_mut(), &(hit.offset..hit.offset));
        });
    }

    // Autosave to the recovery journal. Separate from the disk watcher
    // because they answer different questions on different clocks: "did
    // someone else change this file" and "is my work safe if the power goes".
    let journal_timer = slint::Timer::default();
    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        journal_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_secs(5),
            move || {
                cell.borrow_mut().checkpoint_all();
                // The status bar shows the checkpoint time, so it has to
                // repaint -- but a checkpoint is not a save and the save
                // state must not move.
                if let Some(ui) = weak.upgrade() {
                    refresh(&ui, &mut cell.borrow_mut(), state::PushText::No);
                }
            },
        );
    }

    // Capture clipboard changes. Polling, because neither platform offers a
    // portable change notification and a missed clip is a minor loss.
    //
    // History is in memory only: the clipboard carries passwords and tokens
    // constantly, and specs.md section 14 makes persistence opt-in.
    let clipboard_timer = slint::Timer::default();
    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        let mut last_seen: Option<String> = None;
        clipboard_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(1200),
            move || {
                let Some(text) = read_os_clipboard() else {
                    return;
                };
                // The active document's policy, because you copy out of the
                // document you are looking at and an OS clipboard read says
                // nothing about where the text came from.
                let changed = {
                    let mut s = cell.borrow_mut();
                    let policy = s.policy().clipboard;
                    // Enforce first: a document whose profile forbids a
                    // history must not keep one gathered a moment ago under a
                    // looser profile, and the poll is the soonest reliable
                    // point at which that is noticed.
                    let cleared = s.clips.enforce(policy);
                    // Copies made in the editor are recorded when they happen
                    // (`copied-from-editor`, and the custom view's commands).
                    // What the poll adds is everything else, which is another
                    // program's -- recorded only if the user opted in. The
                    // document check is the fallback for an in-app copy that
                    // took a path nobody wired; it runs only when the
                    // clipboard has changed, which is what `last_seen` is for.
                    let added = if last_seen.as_deref() == Some(text.as_str()) {
                        false
                    } else {
                        let origin = if s.active_text().contains(text.as_str()) {
                            bp_clipboard::Origin::OpenDocument
                        } else {
                            bp_clipboard::Origin::Elsewhere
                        };
                        let record_other_programs = s.record_other_programs;
                        s.clips
                            .observe(&text, policy, origin, record_other_programs)
                    };
                    last_seen = Some(text);
                    cleared || added
                };
                // Only rebuild the menus when the history actually changed.
                if changed && let Some(ui) = weak.upgrade() {
                    refresh(&ui, &mut cell.borrow_mut(), state::PushText::No);
                }
            },
        );
    }

    // Watch the file on disk. Polling two numbers every couple of seconds is
    // cheap, needs no dependency and no background thread pushing events into
    // this loop, and behaves identically on both platforms. Must outlive the
    // event loop, hence the binding.
    let disk_timer = slint::Timer::default();
    {
        let cell = Rc::clone(&state);
        let weak = ui.as_weak();
        disk_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_secs(2),
            move || {
                let changed = {
                    let mut s = cell.borrow_mut();
                    let before = s.disk_warning.clone();
                    s.poll_disk();
                    before != s.disk_warning
                };
                // Only touch the UI when the answer actually changed; a
                // two-second repaint of an unchanged status bar is waste.
                if changed && let Some(ui) = weak.upgrade() {
                    refresh(&ui, &mut cell.borrow_mut(), state::PushText::No);
                }
            },
        );
    }

    set_static_menus(&ui);
    refresh(&ui, &mut state.borrow_mut(), state::PushText::Yes);
    // `run` spelled out, because the recovery question needs the window
    // shown before it can take the keyboard focus.
    ui.show()?;
    if !pending.is_empty() {
        offer_recovery(&ui, &state, pending);
    }
    slint::run_event_loop()?;
    ui.hide()?;
    drop(disk_timer);
    Ok(())
}

/// Wire the editor's text, caret, Go to Line and tab-cycling callbacks. A
/// function of its own so the window tests drive exactly what `run_with`
/// wires.
fn bind_editing(ui: &AppWindow, state: &Rc<RefCell<state::AppState>>) {
    // Ctrl+Tab and Ctrl+Shift+Tab.
    {
        let cell = Rc::clone(state);
        let weak = ui.as_weak();
        ui.on_cycle_tab(move |forward| {
            let Some(ui) = weak.upgrade() else { return };
            cell.borrow_mut().cycle_tab(forward);
            refresh(&ui, &mut cell.borrow_mut(), state::PushText::Yes);
        });
    }

    {
        let cell = Rc::clone(state);
        let weak = ui.as_weak();
        ui.on_text_edited(move |text| {
            {
                let mut s = cell.borrow_mut();
                s.edit(text.to_string());
                // The caret moved with the typing, and the order in which
                // Slint reports the two is not promised, so it is read again
                // after the text rather than trusted from before it.
                if let Some(ui) = weak.upgrade() {
                    s.follow_widget_caret(ui.get_editor_anchor(), ui.get_editor_cursor());
                }
            }
            if let Some(ui) = weak.upgrade() {
                // PushText::No -- the widget already holds this text. Pushing
                // it back would clone the document on every keystroke.
                refresh(&ui, &mut cell.borrow_mut(), state::PushText::No);
            }
        });
    }

    // The `TextInput` caret moved: copy it into the editor, and say where it
    // is. Only the one label, not a whole refresh -- this runs on every
    // arrow key.
    {
        let cell = Rc::clone(state);
        let weak = ui.as_weak();
        ui.on_caret_moved(move || {
            let Some(ui) = weak.upgrade() else { return };
            let label = {
                // Pushing new text into the widget moves its caret, and Slint
                // reports that at once -- inside `refresh`, which has the
                // state borrowed. Skipped then rather than panicking: the
                // refresh under way writes the label itself, and the next
                // move catches the caret up.
                let Ok(mut s) = cell.try_borrow_mut() else {
                    return;
                };
                s.follow_widget_caret(ui.get_editor_anchor(), ui.get_editor_cursor());
                s.cursor_label()
            };
            ui.set_cursor_label(label.into());
        });
    }

    {
        let cell = Rc::clone(state);
        let weak = ui.as_weak();
        ui.on_goto_submitted(move || {
            let Some(ui) = weak.upgrade() else { return };
            let moved = cell.borrow_mut().go_to_line(&ui.get_goto_line());
            ui.set_goto_status(cell.borrow().goto_status.as_str().into());
            if let Some(range) = moved {
                // The bar stays open: going to a line is often the first of
                // several, and closing it would make the second one two
                // keystrokes further away.
                dispatch::select(&ui, &mut cell.borrow_mut(), &range);
            }
        });
    }
}

/// Wire the Settings sheet's callbacks. Each step changes the draft and
/// redraws the rows; nothing reaches the live state until Save. A function
/// of its own so the window tests wire the sheet exactly as `run_with` does.
fn bind_settings(ui: &AppWindow, state: &Rc<RefCell<state::AppState>>) {
    {
        let cell = Rc::clone(state);
        let weak = ui.as_weak();
        ui.on_setting_stepped(move |row, delta| {
            let Some(ui) = weak.upgrade() else { return };
            let rows = {
                let mut s = cell.borrow_mut();
                let Some(sheet) = s.settings_sheet.as_mut() else {
                    return;
                };
                sheet.step(row, delta);
                sheet.rows()
            };
            ui.set_setting_rows(slint::ModelRc::new(slint::VecModel::from(rows)));
        });
    }
    {
        let cell = Rc::clone(state);
        let weak = ui.as_weak();
        ui.on_settings_saved(move || {
            let Some(ui) = weak.upgrade() else { return };
            {
                let mut s = cell.borrow_mut();
                if let Some(sheet) = s.settings_sheet.take() {
                    sheet.save(&mut s);
                }
            }
            refresh(&ui, &mut cell.borrow_mut(), state::PushText::No);
        });
    }
    {
        let cell = Rc::clone(state);
        ui.on_settings_cancelled(move || {
            cell.borrow_mut().settings_sheet = None;
        });
    }
}

/// Offer to recover what a previous session left unsaved.
///
/// Asked as the window opens, over it, so nobody starts typing over work
/// they have not been told about: the sheet holds the keyboard until it is
/// answered. Each button says what it does, which `rfd`'s Yes/No/Cancel
/// could not on Windows without comctl32 v6, and Escape keeps the work --
/// only an explicit Discard deletes anything.
fn offer_recovery(
    ui: &AppWindow,
    cell: &Rc<RefCell<state::AppState>>,
    pending: Vec<(u64, bp_history::Checkpoint)>,
) {
    let count = pending.len();
    let mut names: Vec<&str> = pending
        .iter()
        .map(|(_, c)| c.name.as_str())
        .take(5)
        .collect();
    let more = format!("and {} more", count.saturating_sub(5));
    if count > 5 {
        names.push(&more);
    }
    let body = format!(
        "BachelorPad+ closed with {count} unsaved document(s):\n\n{}\n\n\
         Restore opens them now. Discard deletes them for good. \
         Not Now keeps them and asks again next time.",
        names.join("\n")
    );

    let cell = Rc::clone(cell);
    dialog::ask(
        ui,
        dialog::Dialog {
            title: "Unsaved work recovered".to_owned(),
            body,
            buttons: state::RECOVERY_BUTTONS
                .iter()
                .map(|&b| b.to_owned())
                .collect(),
            default: 0,
            cancel: 2,
        },
        move |ui, chosen| {
            {
                let mut s = cell.borrow_mut();
                match state::recovery_choice(chosen) {
                    // Restored documents are journalled again under this
                    // session's ids first, and only then are the ended
                    // sessions' copies removed, so there is no moment with no
                    // copy on disk.
                    state::RecoveryChoice::Restore => {
                        s.restore(pending);
                        s.checkpoint_all();
                        let _ = s.journal.discard_recovered();
                    }
                    // An explicit Discard is a decision; honour it rather
                    // than asking again every launch. `discard_recovered`
                    // rather than `discard_all`: the question was about the
                    // ended sessions' work, and this one's own journal is
                    // none of its business.
                    state::RecoveryChoice::Discard => {
                        let _ = s.journal.discard_recovered();
                    }
                    state::RecoveryChoice::Keep => {
                        s.journal.keep_for_later();
                        s.add_notice(format!(
                            "{count} unsaved document(s) from last time were kept, not \
                             restored; you will be asked again at the next launch"
                        ));
                    }
                }
            }
            refresh(ui, &mut cell.borrow_mut(), state::PushText::Yes);
        },
    );
}

/// Measure the cost of one keystroke through the application's state path,
/// at a range of document sizes.
///
/// This is the half of typing latency we control: taking the edited text from
/// the widget, storing it, marking the document modified, and refreshing the
/// gutter. It deliberately does **not** include rasterisation or presentation,
/// which need a capture rig to measure and are the renderer's contribution.
///
/// The two tables are the two views. The first is the `TextInput` path, and
/// it is faithful in one uncomfortable way: Slint's `edited` callback hands
/// us the entire buffer as a fresh string, so a keystroke copies the
/// document. Those numbers should grow linearly with document size, and if
/// they do, that is the finding, not a flaw in the probe. The second is the
/// `--editor-view` path, where the rope is written at the caret and size
/// stops mattering -- which is the case ADR-0018 was decided on.
pub fn latency_probe() {
    const SAMPLES: usize = 300;
    const SIZES: [usize; 4] = [1_000, 10_000, 100_000, 1_000_000];

    println!("typing-path latency (state update only, excludes rendering)");
    println!(
        "{:>12}  {:>10}  {:>10}  {:>10}",
        "doc chars", "p50", "p95", "max"
    );

    for size in SIZES {
        // A realistic-ish document: 80-column lines.
        let mut text: String = (0..size)
            .map(|i| if i % 80 == 79 { '\n' } else { 'x' })
            .collect();

        let mut state = state::AppState::new();
        state.edit(text.clone());
        state.sync_gutter();

        let mut timings = Vec::with_capacity(SAMPLES);
        for _ in 0..SAMPLES {
            text.push('y');
            let next = text.clone();
            let start = std::time::Instant::now();
            state.edit(next);
            state.sync_gutter();
            timings.push(start.elapsed());
        }

        timings.sort_unstable();
        let at = |q: f64| timings[((timings.len() as f64 * q) as usize).min(timings.len() - 1)];
        println!(
            "{size:>12}  {:>8.1}µs  {:>8.1}µs  {:>8.1}µs",
            at(0.50).as_secs_f64() * 1e6,
            at(0.95).as_secs_f64() * 1e6,
            timings[timings.len() - 1].as_secs_f64() * 1e6,
        );
    }

    println!();
    println!("rope insert at the caret (what --editor-view costs)");
    println!(
        "{:>12}  {:>10}  {:>10}  {:>10}",
        "doc chars", "p50", "p95", "max"
    );

    for size in SIZES {
        let text: String = (0..size)
            .map(|i| if i % 80 == 79 { '\n' } else { 'x' })
            .collect();
        let mut buffer = bp_buffer::Buffer::from_text(&text);
        let mut timings = Vec::with_capacity(SAMPLES);

        for i in 0..SAMPLES {
            // Insert mid-document, the worst realistic case for a flat string
            // and the case a rope exists to make cheap.
            let at = buffer.len_chars() / 2 + i;
            let start = std::time::Instant::now();
            buffer.insert(at, "y");
            timings.push(start.elapsed());
        }

        timings.sort_unstable();
        let at = |q: f64| timings[((timings.len() as f64 * q) as usize).min(timings.len() - 1)];
        println!(
            "{size:>12}  {:>8.1}µs  {:>8.1}µs  {:>8.1}µs",
            at(0.50).as_secs_f64() * 1e6,
            at(0.95).as_secs_f64() * 1e6,
            timings[timings.len() - 1].as_secs_f64() * 1e6,
        );
    }

    println!();
    println!("Perceptible-latency threshold is roughly 16ms (16000µs) per frame.");
    println!("The first table grows with document size because Slint hands back the");
    println!("whole buffer on every edit -- that is the default view. The second is");
    println!("the same keystroke under --editor-view, which writes the rope directly.");
}

/// A continuation that refreshes the window, whatever the outcome it is
/// handed. What most callers of the flows below want once they are done.
fn refresh_after<T>(cell: &Rc<RefCell<state::AppState>>) -> impl FnOnce(&AppWindow, T) + 'static {
    let cell = Rc::clone(cell);
    move |ui, _| refresh(ui, &mut cell.borrow_mut(), state::PushText::Yes)
}

/// Save, escalating to Save As when the document has no path yet, and asking
/// first if the file changed underneath us. `then` gets the outcome once
/// there is one -- straight away, unless there was something to ask.
///
/// Save on a never-saved document must not silently do nothing, and Save on a
/// file somebody else edited must not silently discard their work.
fn save_with_prompt(
    ui: &AppWindow,
    cell: &Rc<RefCell<state::AppState>>,
    id: DocumentId,
    then: impl FnOnce(&AppWindow, state::SaveResult) + 'static,
) {
    let changed = cell.borrow().would_overwrite_external_change(id);
    if !changed {
        save_or_pick(ui, cell, id, then);
        return;
    }

    let name = cell.borrow().display_name(id);
    let cell = Rc::clone(cell);
    dialog::ask(
        ui,
        dialog::Dialog {
            title: "Changed on disk".to_owned(),
            body: format!(
                "{name} has changed on disk since you opened it.\n\n\
                 Saving will overwrite those changes."
            ),
            buttons: vec!["Save Anyway".to_owned(), "Cancel".to_owned()],
            // Enter cancels here, unlike the other questions: the work
            // Save Anyway destroys is somebody else's, and they are not the
            // one pressing Enter.
            default: 1,
            cancel: 1,
        },
        move |ui, chosen| {
            if chosen == 0 {
                save_or_pick(ui, &cell, id, then);
            } else {
                // Not a failure -- a refusal. Nothing was written either
                // way, so the caller must not treat it as saved.
                then(ui, state::SaveResult::NeedsPath);
            }
        },
    );
}

/// Save to the document's own path, or ask for one if it has none.
fn save_or_pick(
    ui: &AppWindow,
    cell: &Rc<RefCell<state::AppState>>,
    id: DocumentId,
    then: impl FnOnce(&AppWindow, state::SaveResult) + 'static,
) {
    let (result, path) = {
        let mut s = cell.borrow_mut();
        match s.save_document(id, None) {
            state::SaveResult::NeedsPath => match dispatch::pick_save_path(&s, id) {
                Some(path) => (s.save_document(id, Some(path.clone())), Some(path)),
                // The user dismissed the dialog. Nothing was written, and
                // the caller must not treat that as saved.
                None => (state::SaveResult::NeedsPath, None),
            },
            other => (other, None),
        }
    };
    offer_utf8(ui, cell, id, result, path, then);
}

/// Save As: ask where, save there, and refresh.
pub(crate) fn save_as(ui: &AppWindow, cell: &Rc<RefCell<state::AppState>>, id: DocumentId) {
    let chosen = dispatch::pick_save_path(&cell.borrow(), id);
    let Some(path) = chosen else {
        refresh(ui, &mut cell.borrow_mut(), state::PushText::Yes);
        return;
    };
    let result = cell.borrow_mut().save_document(id, Some(path.clone()));
    offer_utf8(ui, cell, id, result, Some(path), refresh_after(cell));
}

/// If a save failed only because the document's encoding cannot hold one of
/// its characters, offer to save it as UTF-8 instead; otherwise pass the
/// outcome straight on.
///
/// `path` is where the failed save was going (`None` for the document's own
/// path), so the retry goes to the same place rather than asking again.
/// Declining puts the selection on the character in the way when the
/// document is the one on screen, so the user can see what to change.
fn offer_utf8(
    ui: &AppWindow,
    cell: &Rc<RefCell<state::AppState>>,
    id: DocumentId,
    result: state::SaveResult,
    path: Option<PathBuf>,
    then: impl FnOnce(&AppWindow, state::SaveResult) + 'static,
) {
    let state::SaveResult::Unrepresentable(what) = result else {
        then(ui, result);
        return;
    };
    let name = cell.borrow().display_name(id);
    let encoding = what.encoding.label();
    let cell = Rc::clone(cell);
    dialog::ask(
        ui,
        dialog::Dialog {
            title: format!("Can't save as {encoding}"),
            body: format!(
                "{name} contains \"{}\" (U+{:04X}), which {encoding} has no \
                 form for.\n\n\
                 Saving as UTF-8 keeps every character. A program that reads \
                 only {encoding} may show some of them wrongly.",
                what.character,
                u32::from(what.character),
            ),
            buttons: vec!["Save as UTF-8".to_owned(), "Cancel".to_owned()],
            default: 0,
            cancel: 1,
        },
        move |ui, chosen| {
            if chosen == 0 {
                let result = {
                    let mut s = cell.borrow_mut();
                    s.set_encoding_of(id, bp_core::Encoding::Utf8);
                    s.save_document(id, path)
                };
                then(ui, result);
                return;
            }
            let on_screen = cell.borrow().workspace.active_id() == Some(id);
            if on_screen {
                // `select` takes characters; the encoder reports bytes.
                let start = cell.borrow().byte_to_char(what.byte_offset);
                dispatch::select(ui, &mut cell.borrow_mut(), &(start..start + 1));
            }
            then(ui, result);
        },
    );
}

/// Close a tab, asking first if it holds unsaved work. `then` gets whether
/// the tab went: false after Cancel, or a save that did not happen, which
/// is what a caller closing several tabs stops at.
fn close_with_prompt(
    ui: &AppWindow,
    cell: &Rc<RefCell<state::AppState>>,
    id: DocumentId,
    then: impl FnOnce(&AppWindow, bool) + 'static,
) {
    let (dirty, name) = {
        let s = cell.borrow();
        (s.is_dirty(id), s.display_name(id))
    };
    if !dirty {
        cell.borrow_mut().close(id);
        then(ui, true);
        return;
    }

    let cell = Rc::clone(cell);
    dialog::ask_about_unsaved(ui, &name, move |ui, answer| match answer {
        dialog::Unsaved::Save => {
            let after = Rc::clone(&cell);
            save_with_prompt(ui, &cell, id, move |ui, result| {
                // Keep the tab open rather than discard unsaved work.
                let saved = result == state::SaveResult::Saved;
                if saved {
                    after.borrow_mut().close(id);
                }
                then(ui, saved);
            });
        }
        dialog::Unsaved::Discard => {
            cell.borrow_mut().close(id);
            then(ui, true);
        }
        dialog::Unsaved::Cancel => then(ui, false),
    });
}

/// Close each of `ids` in turn, asking about each one's unsaved work, and
/// refresh once done. Cancel stops the lot: the user cancelled closing, not
/// closing that one tab.
pub(crate) fn close_each(
    ui: &AppWindow,
    cell: &Rc<RefCell<state::AppState>>,
    mut ids: VecDeque<DocumentId>,
) {
    let Some(id) = ids.pop_front() else {
        refresh(ui, &mut cell.borrow_mut(), state::PushText::Yes);
        return;
    };
    let next = Rc::clone(cell);
    close_with_prompt(ui, cell, id, move |ui, closed| {
        if closed {
            close_each(ui, &next, ids);
        } else {
            refresh(ui, &mut next.borrow_mut(), state::PushText::Yes);
        }
    });
}

/// Save each of `ids` in turn and refresh once done, stopping at the first
/// that is not saved rather than asking again of someone who just
/// cancelled.
pub(crate) fn save_each(
    ui: &AppWindow,
    cell: &Rc<RefCell<state::AppState>>,
    mut ids: VecDeque<DocumentId>,
) {
    let Some(id) = ids.pop_front() else {
        refresh(ui, &mut cell.borrow_mut(), state::PushText::Yes);
        return;
    };
    let next = Rc::clone(cell);
    save_with_prompt(ui, cell, id, move |ui, result| {
        if result == state::SaveResult::Saved {
            save_each(ui, &next, ids);
        } else {
            refresh(ui, &mut next.borrow_mut(), state::PushText::Yes);
        }
    });
}

/// Settle every unsaved document in `ids`, then close the window.
///
/// Documents the user chose not to save are closed only once the window is
/// going: closing each as it was answered would, after a later Cancel, leave
/// the user in a window with tabs missing that they never asked to close.
/// Closing them is what discards their recovery checkpoints, so Don't Save
/// is not offered back as "recovered" work at the next launch.
fn settle_then_quit(
    ui: &AppWindow,
    cell: &Rc<RefCell<state::AppState>>,
    mut ids: VecDeque<DocumentId>,
    mut discarded: Vec<DocumentId>,
) {
    let Some(id) = ids.pop_front() else {
        {
            let mut s = cell.borrow_mut();
            for id in discarded {
                s.close(id);
            }
        }
        let _ = ui.hide();
        return;
    };
    let name = cell.borrow().display_name(id);
    let cell = Rc::clone(cell);
    dialog::ask_about_unsaved(ui, &name, move |ui, answer| match answer {
        dialog::Unsaved::Save => {
            let next = Rc::clone(&cell);
            save_with_prompt(ui, &cell, id, move |ui, result| {
                if result == state::SaveResult::Saved {
                    settle_then_quit(ui, &next, ids, discarded);
                } else {
                    // Quitting now would discard exactly what the user asked
                    // to keep.
                    refresh(ui, &mut next.borrow_mut(), state::PushText::Yes);
                }
            });
        }
        dialog::Unsaved::Discard => {
            discarded.push(id);
            settle_then_quit(ui, &cell, ids, discarded);
        }
        dialog::Unsaved::Cancel => refresh(ui, &mut cell.borrow_mut(), state::PushText::Yes),
    });
}
