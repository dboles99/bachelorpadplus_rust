//! The editor in a real window, driven by real key events.
//!
//! Everything else in this crate tests the *rules* -- which command a key
//! means, where a motion lands -- and never a window. That left the default
//! `TextInput` view with no tests at all, because its keys are handled by
//! Slint and never reach Rust, and it is how Page Up and Page Down came to do
//! nothing and the arrows to move the caret out of sight with nobody noticing.
//!
//! Each test builds the real `AppWindow` on a headless software-rendered
//! window and dispatches key events to it exactly as a backend would. No
//! display, no new dependency: `MinimalSoftwareWindow` and `dispatch_event`
//! are both public Slint API.
//!
//! Slint's platform is per thread, and the test harness runs each test on a
//! thread of its own, so each one installs it.

use std::rc::Rc;

use slint::ComponentHandle;
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Key, Platform, WindowAdapter, WindowEvent};

use crate::AppWindow;

struct Headless;

impl Platform for Headless {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer))
    }
}

/// Lines `line 001` .. `line 200`, and one very long line at 5.
fn document() -> String {
    let mut lines: Vec<String> = (1..=200).map(|i| format!("line {i:03}")).collect();
    lines[4] = format!("line 005 {}END", "long ".repeat(80));
    lines.join("\n") + "\n"
}

/// The window, 800 x 400, with the document in the `TextInput` view and the
/// keyboard focus in it -- what a user sees after opening a file.
fn window() -> AppWindow {
    // Already set on this thread is fine; anything else is not.
    if let Err(e) = slint::platform::set_platform(Box::new(Headless)) {
        assert!(
            matches!(e, slint::platform::SetPlatformError::AlreadySet),
            "{e:?}"
        );
    }
    let ui = AppWindow::new().expect("a headless window");
    ui.window().set_size(slint::PhysicalSize::new(800, 400));
    ui.set_doc_text(document().into());
    ui.on_overwrite_end(crate::editor_view::overwrite_end);
    ui.show().expect("show");
    ui.invoke_focus_editor();
    ui
}

fn press(ui: &AppWindow, key: impl Into<slint::SharedString> + Clone) {
    let text: slint::SharedString = key.into();
    ui.window()
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text });
}

fn chord(ui: &AppWindow, modifier: Key, key: impl Into<slint::SharedString> + Clone) {
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: modifier.into(),
    });
    press(ui, key);
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: modifier.into(),
    });
}

fn type_text(ui: &AppWindow, text: &str) {
    for ch in text.chars() {
        press(ui, ch.to_string());
    }
}

/// The 1-based line holding `marker`, which the tests type to find out
/// where a key left the caret.
fn line_of(ui: &AppWindow, marker: char) -> usize {
    ui.get_doc_text()
        .lines()
        .position(|line| line.contains(marker))
        .map(|index| index + 1)
        .unwrap_or_else(|| panic!("{marker:?} was not typed anywhere"))
}

#[test]
fn the_window_takes_typing_at_all() {
    // The control: if this fails, every other test here is testing nothing.
    let ui = window();
    chord(&ui, Key::Control, Key::Home);
    type_text(&ui, "Q");
    assert!(ui.get_doc_text().starts_with("Qline 001"));
}

#[test]
fn page_down_moves_the_caret_by_a_page_and_page_up_brings_it_back() {
    // Before `page-height` was set, `TextInput` refused a page of zero and
    // both keys did nothing: the marker landed on line 1.
    let ui = window();
    chord(&ui, Key::Control, Key::Home);
    press(&ui, Key::PageDown);
    type_text(&ui, "P");
    let paged = line_of(&ui, 'P');
    assert!(
        (10..=30).contains(&paged),
        "one page down from line 1 of a 400px window landed on line {paged}"
    );

    press(&ui, Key::PageDown);
    type_text(&ui, "R");
    assert!(line_of(&ui, 'R') > paged, "a second page went nowhere");

    press(&ui, Key::PageUp);
    type_text(&ui, "U");
    assert!(
        line_of(&ui, 'U') < line_of(&ui, 'R'),
        "Page Up did not move back up"
    );
}

#[test]
fn the_view_follows_the_caret_down_and_back_up() {
    // The arrows always moved the caret; nothing moved the view, so past the
    // bottom row the caret vanished and the keys looked dead.
    let ui = window();
    chord(&ui, Key::Control, Key::Home);
    assert_eq!(ui.get_editor_scroll_y(), 0.0);

    chord(&ui, Key::Control, Key::End);
    assert!(
        ui.get_editor_scroll_y() < -1000.0,
        "Ctrl+End left the view at {}",
        ui.get_editor_scroll_y()
    );

    chord(&ui, Key::Control, Key::Home);
    assert_eq!(ui.get_editor_scroll_y(), 0.0, "Ctrl+Home left it scrolled");
}

#[test]
fn arrowing_down_past_the_last_visible_row_scrolls_one_step_not_a_page() {
    let ui = window();
    chord(&ui, Key::Control, Key::Home);
    for _ in 0..40 {
        press(&ui, Key::DownArrow);
    }
    let scrolled = ui.get_editor_scroll_y();
    assert!(scrolled < 0.0, "forty lines down and the view never moved");
    type_text(&ui, "@");
    assert_eq!(line_of(&ui, '@'), 41);
}

#[test]
fn end_on_a_long_line_scrolls_across_to_it_and_home_scrolls_back() {
    let ui = window();
    chord(&ui, Key::Control, Key::Home);
    for _ in 0..4 {
        press(&ui, Key::DownArrow);
    }
    press(&ui, Key::End);
    assert!(
        ui.get_editor_scroll_x() < -100.0,
        "End on a 400-character line left the view at x = {}",
        ui.get_editor_scroll_x()
    );
    type_text(&ui, "E");
    assert!(
        ui.get_doc_text().contains("ENDE\n"),
        "End did not reach the end"
    );

    press(&ui, Key::Home);
    assert_eq!(ui.get_editor_scroll_x(), 0.0, "Home left the view across");
}

#[test]
fn insert_asks_the_shell_to_switch_modes_and_types_nothing() {
    let ui = window();
    let toggled = Rc::new(std::cell::Cell::new(0));
    let seen = Rc::clone(&toggled);
    ui.on_toggle_overwrite(move || seen.set(seen.get() + 1));
    let before = ui.get_doc_text();

    press(&ui, Key::Insert);

    assert_eq!(toggled.get(), 1);
    assert_eq!(ui.get_doc_text(), before, "Insert typed something");
}

#[test]
fn typing_in_overwrite_replaces_and_stops_at_the_end_of_the_line() {
    let ui = window();
    ui.set_overwrite(true);
    chord(&ui, Key::Control, Key::Home);
    type_text(&ui, "LINE");
    assert!(
        ui.get_doc_text().starts_with("LINE 001\nline 002"),
        "{:?}",
        &ui.get_doc_text()[..20]
    );

    // Past the end of the line it extends rather than eating the break.
    press(&ui, Key::End);
    type_text(&ui, "xyz");
    assert!(ui.get_doc_text().starts_with("LINE 001xyz\nline 002"));
}

#[test]
fn arrows_in_overwrite_move_exactly_as_they_do_in_insert() {
    // The overwrite hook runs before `TextInput` knows what a key is. An
    // arrow that selected a character first would move one too far.
    let ui = window();
    ui.set_overwrite(true);
    chord(&ui, Key::Control, Key::Home);
    press(&ui, Key::RightArrow);
    press(&ui, Key::RightArrow);
    type_text(&ui, "N");
    assert!(ui.get_doc_text().starts_with("liNe 001"));
}

#[test]
fn typing_with_overwrite_off_inserts() {
    let ui = window();
    chord(&ui, Key::Control, Key::Home);
    type_text(&ui, "ab");
    assert!(ui.get_doc_text().starts_with("abline 001"));
}

#[test]
fn no_slint_source_names_a_font_the_platform_may_not_have() {
    // Slint resolves a family name literally, so a font that exists on one
    // OS and not another silently becomes a proportional fallback there --
    // which is how "Consolas" broke the custom view's caret on Linux. The
    // bundled DejaVu Sans Mono is the only monospace family allowed.
    for (file, source) in [
        ("app.slint", include_str!("../ui/app.slint")),
        (
            "editor_surface.slint",
            include_str!("../ui/editor_surface.slint"),
        ),
        (
            "results_panel.slint",
            include_str!("../ui/results_panel.slint"),
        ),
        ("status_bar.slint", include_str!("../ui/status_bar.slint")),
        ("find_bar.slint", include_str!("../ui/find_bar.slint")),
        ("goto_bar.slint", include_str!("../ui/goto_bar.slint")),
        ("menu.slint", include_str!("../ui/menu.slint")),
        ("tab.slint", include_str!("../ui/tab.slint")),
        (
            "passphrase_bar.slint",
            include_str!("../ui/passphrase_bar.slint"),
        ),
        ("types.slint", include_str!("../ui/types.slint")),
    ] {
        let code = source.lines().filter(|l| !l.trim_start().starts_with("//"));
        for line in code.filter(|l| l.contains("font-family")) {
            assert!(
                line.contains("\"DejaVu Sans Mono\""),
                "{file} asks for a font that is not bundled: {}",
                line.trim()
            );
        }
    }
}

/// Copies and cuts reported to Rust, as (selected text, was it a cut).
fn record_copies(ui: &AppWindow) -> Rc<std::cell::RefCell<Vec<(String, bool)>>> {
    let seen = Rc::new(std::cell::RefCell::new(Vec::new()));
    let log = Rc::clone(&seen);
    ui.on_copied_from_editor(move |text, anchor, cursor, cut| {
        let selected = crate::editor_view::selected_text(&text, anchor, cursor)
            .unwrap_or_default()
            .to_owned();
        log.borrow_mut().push((selected, cut));
        false
    });
    seen
}

fn select_first_four(ui: &AppWindow) {
    chord(ui, Key::Control, Key::Home);
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Shift.into(),
    });
    for _ in 0..4 {
        press(ui, Key::RightArrow);
    }
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Shift.into(),
    });
}

#[test]
fn ctrl_c_in_the_default_view_reports_the_selection_to_rust() {
    // Without this the history learned of a copy only by polling, which
    // cannot tell this window's copy from a password manager's.
    let ui = window();
    let seen = record_copies(&ui);
    select_first_four(&ui);

    chord(&ui, Key::Control, "c");

    assert_eq!(seen.borrow().as_slice(), [("line".to_owned(), false)]);
    assert!(
        ui.get_doc_text().starts_with("line 001"),
        "a copy changed the text"
    );
}

#[test]
fn ctrl_x_reports_a_cut_and_the_widget_still_cuts() {
    let ui = window();
    let seen = record_copies(&ui);
    select_first_four(&ui);

    chord(&ui, Key::Control, "x");

    assert_eq!(seen.borrow().as_slice(), [("line".to_owned(), true)]);
    assert!(
        ui.get_doc_text().starts_with(" 001"),
        "the cut did not happen"
    );
}

// --- the in-app dialog (release plan M1-01) ------------------------------

/// Route the sheet's answers to `dialog`, as `run_with` does.
fn answer_through_dialog(ui: &AppWindow) {
    let weak = ui.as_weak();
    ui.on_dialog_answered(move |chosen| {
        if let Some(ui) = weak.upgrade() {
            crate::dialog::answered(&ui, chosen);
        }
    });
}

/// Ask a three-button question and record the answer it gets.
fn ask_three(ui: &AppWindow, title: &str) -> Rc<std::cell::Cell<Option<usize>>> {
    let got = Rc::new(std::cell::Cell::new(None));
    let seen = Rc::clone(&got);
    crate::dialog::ask(
        ui,
        crate::dialog::Dialog {
            title: title.to_owned(),
            body: "body".to_owned(),
            buttons: vec!["One".to_owned(), "Two".to_owned(), "Three".to_owned()],
            default: 0,
            cancel: 2,
        },
        move |_, chosen| seen.set(Some(chosen)),
    );
    got
}

fn buttons(ui: &AppWindow) -> Vec<String> {
    use slint::Model as _;
    let model = ui.get_dialog_buttons();
    (0..model.row_count())
        .filter_map(|i| model.row_data(i))
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn a_question_holds_the_keyboard_so_nothing_reaches_the_document() {
    let ui = window();
    answer_through_dialog(&ui);
    let new_documents = Rc::new(std::cell::Cell::new(0));
    let count = Rc::clone(&new_documents);
    ui.on_new_document(move || count.set(count.get() + 1));
    let before = ui.get_doc_text();

    let answer = ask_three(&ui, "Question");
    assert!(ui.get_dialog_open());
    type_text(&ui, "typed");
    chord(&ui, Key::Control, "n");

    assert_eq!(ui.get_doc_text(), before, "typing got past the question");
    assert_eq!(
        new_documents.get(),
        0,
        "a shortcut fired behind the question"
    );
    assert_eq!(answer.get(), None, "typing is not an answer");
}

#[test]
fn escape_chooses_the_answer_that_changes_nothing() {
    let ui = window();
    answer_through_dialog(&ui);
    let answer = ask_three(&ui, "Question");

    press(&ui, Key::Escape);

    assert_eq!(answer.get(), Some(2));
    assert!(!ui.get_dialog_open());
}

#[test]
fn enter_chooses_the_default_answer() {
    let ui = window();
    answer_through_dialog(&ui);
    let answer = ask_three(&ui, "Question");

    press(&ui, Key::Return);

    assert_eq!(answer.get(), Some(0));
}

#[test]
fn the_arrows_and_tab_move_the_answer_enter_gives_and_wrap_around() {
    let ui = window();
    answer_through_dialog(&ui);
    let answer = ask_three(&ui, "Question");
    assert_eq!(ui.get_dialog_highlighted(), 0);

    press(&ui, Key::RightArrow);
    assert_eq!(ui.get_dialog_highlighted(), 1);
    press(&ui, Key::Tab);
    assert_eq!(ui.get_dialog_highlighted(), 2);
    press(&ui, Key::Tab);
    assert_eq!(ui.get_dialog_highlighted(), 0, "Tab past the last wraps");
    press(&ui, Key::LeftArrow);
    assert_eq!(ui.get_dialog_highlighted(), 2, "Left past the first wraps");
    press(&ui, Key::LeftArrow);

    press(&ui, Key::Return);
    assert_eq!(answer.get(), Some(1));
}

#[test]
fn a_question_asked_while_another_is_open_waits_its_turn() {
    let ui = window();
    answer_through_dialog(&ui);
    let first = ask_three(&ui, "First");
    let second = ask_three(&ui, "Second");
    assert_eq!(ui.get_dialog_title(), "First");

    press(&ui, Key::Return);
    assert_eq!(first.get(), Some(0));
    assert_eq!(second.get(), None);
    assert!(ui.get_dialog_open(), "the waiting question did not come up");
    assert_eq!(ui.get_dialog_title(), "Second");

    press(&ui, Key::Escape);
    assert_eq!(second.get(), Some(2));
    assert!(!ui.get_dialog_open());
}

#[test]
fn the_keyboard_goes_back_to_the_document_once_the_question_is_answered() {
    let ui = window();
    answer_through_dialog(&ui);
    chord(&ui, Key::Control, Key::Home);
    let _answer = ask_three(&ui, "Question");
    press(&ui, Key::Escape);

    type_text(&ui, "Q");
    assert!(ui.get_doc_text().starts_with("Qline 001"));
}

/// The window, with the shell's state behind it and one unsaved document.
fn window_with_unsaved_work() -> (
    AppWindow,
    Rc<std::cell::RefCell<crate::state::AppState>>,
    bp_core::DocumentId,
) {
    let ui = window();
    answer_through_dialog(&ui);
    let mut state = crate::state::AppState::new();
    state.edit("unsaved work".to_owned());
    let id = state.workspace.active_id().expect("a document");
    assert!(state.is_dirty(id));
    (ui, Rc::new(std::cell::RefCell::new(state)), id)
}

#[test]
fn closing_unsaved_work_offers_save_dont_save_and_cancel() {
    let (ui, cell, id) = window_with_unsaved_work();

    crate::close_with_prompt(&ui, &cell, id, |_, _| {});

    assert!(ui.get_dialog_open());
    assert_eq!(buttons(&ui), ["Save", "Don't Save", "Cancel"]);
    assert_eq!(ui.get_dialog_default(), 0, "Enter saves");
    assert_eq!(ui.get_dialog_cancel(), 2, "Escape cancels");
}

#[test]
fn escape_on_closing_unsaved_work_keeps_the_tab_and_its_changes() {
    let (ui, cell, id) = window_with_unsaved_work();
    let closed = Rc::new(std::cell::Cell::new(None));
    let seen = Rc::clone(&closed);
    crate::close_with_prompt(&ui, &cell, id, move |_, went| seen.set(Some(went)));

    press(&ui, Key::Escape);

    assert_eq!(closed.get(), Some(false));
    assert!(cell.borrow().workspace.get(id).is_some(), "the tab went");
    assert!(cell.borrow().is_dirty(id), "the changes went");
}

#[test]
fn dont_save_closes_the_tab() {
    let (ui, cell, id) = window_with_unsaved_work();
    let closed = Rc::new(std::cell::Cell::new(None));
    let seen = Rc::clone(&closed);
    crate::close_with_prompt(&ui, &cell, id, move |_, went| seen.set(Some(went)));

    press(&ui, Key::RightArrow);
    press(&ui, Key::Return);

    assert_eq!(closed.get(), Some(true));
    assert!(cell.borrow().workspace.get(id).is_none());
}

#[test]
fn cancel_stops_closing_the_rest_of_the_tabs() {
    // Close All with two unsaved tabs: Cancel on the first is a cancel of
    // the whole close, not a skip to the next question.
    let (ui, cell, first) = window_with_unsaved_work();
    let second = {
        let mut s = cell.borrow_mut();
        s.new_document();
        s.edit("more unsaved work".to_owned());
        s.workspace.active_id().expect("a second document")
    };

    crate::close_each(&ui, &cell, [first, second].into());
    press(&ui, Key::Escape);

    assert!(!ui.get_dialog_open(), "a second question followed a cancel");
    let s = cell.borrow();
    assert!(s.workspace.get(first).is_some());
    assert!(s.workspace.get(second).is_some());
}

#[test]
fn a_long_body_pages_from_the_keyboard_and_starts_at_the_top_each_time() {
    // The redaction consent lists every finding, unbounded. The sheet
    // scrolls rather than growing past the window, and since the focus is in
    // the sheet, only its own keys can move it.
    let ui = window();
    answer_through_dialog(&ui);
    let long: Vec<String> = (1..200).map(|i| format!("finding {i}")).collect();
    crate::dialog::inform(&ui, "Findings", &long.join("\n"));
    assert_eq!(ui.get_dialog_scroll_y(), 0.0);

    press(&ui, Key::PageDown);
    let paged = ui.get_dialog_scroll_y();
    assert!(paged < -100.0, "Page Down scrolled only {paged}px");
    press(&ui, Key::DownArrow);
    assert!(ui.get_dialog_scroll_y() < paged, "Down did not scroll");
    press(&ui, Key::PageUp);
    press(&ui, Key::PageUp);
    assert_eq!(ui.get_dialog_scroll_y(), 0.0, "Page Up stops at the top");

    press(&ui, Key::PageDown);
    press(&ui, Key::Escape);
    crate::dialog::inform(&ui, "Findings again", &long.join("\n"));
    assert_eq!(
        ui.get_dialog_scroll_y(),
        0.0,
        "the next question opened scrolled"
    );
}

/// A `.bpadx` whose header asks for the ceiling's cost, and the state that
/// was asked to open it (decision 15).
fn opening_an_expensive_file() -> (
    tempfile::TempDir,
    Rc<std::cell::RefCell<crate::state::AppState>>,
    crate::state::Opening,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("strong.bpadx");
    let cheap = bp_crypto::SealOptions {
        kdf: bp_crypto::KdfParams {
            memory_kib: 8,
            iterations: 1,
            lanes: 1,
        },
        ..bp_crypto::SealOptions::default()
    };
    let mut bytes = bp_crypto::seal(b"secret", "hunter2", cheap).unwrap();
    bytes[10..14].copy_from_slice(&bp_crypto::MAX_MEMORY_KIB.to_le_bytes());
    bytes[14..18].copy_from_slice(&bp_crypto::MAX_ITERATIONS.to_le_bytes());
    std::fs::write(&path, bytes).unwrap();
    let mut state = crate::state::AppState::new();
    let opening = state.open_maybe_encrypted(path);
    (dir, Rc::new(std::cell::RefCell::new(state)), opening)
}

#[test]
fn an_expensive_unlock_asks_in_the_sheet_and_a_yes_brings_up_the_passphrase_bar() {
    let ui = window();
    answer_through_dialog(&ui);
    let (_dir, cell, opening) = opening_an_expensive_file();

    crate::dispatch::ask_to_unlock(&ui, &cell, opening);

    assert!(
        ui.get_dialog_open(),
        "an expensive unlock opened without asking"
    );
    assert_eq!(buttons(&ui), ["Open Anyway", "Cancel"]);
    assert_eq!(
        ui.get_dialog_body().as_str(),
        "This file asks for an unusually expensive unlock (256 MiB, 8 passes, \
         about 4.0 s). Open it anyway?"
    );
    assert!(!ui.get_passphrase_open(), "the bar waits for the answer");

    press(&ui, Key::Return);

    assert!(!ui.get_dialog_open());
    assert!(ui.get_passphrase_open(), "a yes asks for the passphrase");
    assert!(matches!(
        cell.borrow().ask,
        Some(crate::passphrase::Ask::Unlock {
            agreed_cost: true,
            ..
        })
    ));
}

#[test]
fn cancelling_an_expensive_unlock_asks_for_no_passphrase() {
    let ui = window();
    answer_through_dialog(&ui);
    let (_dir, cell, opening) = opening_an_expensive_file();

    crate::dispatch::ask_to_unlock(&ui, &cell, opening);
    press(&ui, Key::Escape);

    assert!(!ui.get_dialog_open());
    assert!(!ui.get_passphrase_open());
    assert_eq!(cell.borrow().ask, None);
    assert_eq!(
        cell.borrow().error.as_deref(),
        Some("strong.bpadx was not opened")
    );
}

#[test]
fn a_save_windows_1252_cannot_hold_offers_utf8_and_saves_it_there() {
    let ui = window();
    answer_through_dialog(&ui);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ansi.txt");
    std::fs::write(&path, b"caf\xe9\n").unwrap();
    let mut state = crate::state::AppState::new();
    state.open(path.clone());
    state.edit("café, Gdańsk\n".to_owned());
    let id = state.workspace.active_id().expect("a document");
    let cell = Rc::new(std::cell::RefCell::new(state));
    let result = Rc::new(std::cell::Cell::new(None));
    let seen = Rc::clone(&result);

    crate::save_with_prompt(&ui, &cell, id, move |_, r| seen.set(Some(r)));
    assert!(ui.get_dialog_open(), "the save failed without asking");
    assert_eq!(buttons(&ui), ["Save as UTF-8", "Cancel"]);
    assert_eq!(result.get(), None);

    press(&ui, Key::Return);

    assert_eq!(result.get(), Some(crate::state::SaveResult::Saved));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "café, Gdańsk\n");
    assert_eq!(
        cell.borrow().workspace.get(id).unwrap().encoding(),
        bp_core::Encoding::Utf8
    );
}

#[test]
fn declining_utf8_leaves_the_file_and_the_encoding_as_they_were() {
    let ui = window();
    answer_through_dialog(&ui);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ansi.txt");
    std::fs::write(&path, b"caf\xe9\n").unwrap();
    let mut state = crate::state::AppState::new();
    state.open(path.clone());
    state.edit("café, Gdańsk\n".to_owned());
    let id = state.workspace.active_id().expect("a document");
    let cell = Rc::new(std::cell::RefCell::new(state));

    crate::save_with_prompt(&ui, &cell, id, |_, _| {});
    press(&ui, Key::Escape);

    assert_eq!(std::fs::read(&path).unwrap(), b"caf\xe9\n");
    assert_eq!(
        cell.borrow().workspace.get(id).unwrap().encoding(),
        bp_core::Encoding::Windows1252
    );
    assert!(cell.borrow().is_dirty(id));
}

// --- settings chosen in the window (release plan M1-03) -----------------

/// The window, with the shell's state configured from `config.toml` at
/// `path` -- as `main` does -- and `args` as the command line.
fn configured(
    path: &std::path::Path,
    args: &[&str],
) -> (AppWindow, Rc<std::cell::RefCell<crate::state::AppState>>) {
    let ui = window();
    let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
    let loaded = bp_config::load_from(Some(path), &bp_config::Env::default(), &args);
    let mut state = crate::state::AppState::new();
    state.configure(&loaded.config, loaded.path.clone());
    (ui, Rc::new(std::cell::RefCell::new(state)))
}

#[test]
fn a_view_choice_is_saved_into_the_users_config_with_their_comments_kept() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "# mine, do not lose\ntheme = \"Dark\"  # at night\n").unwrap();
    let (ui, cell) = configured(&path, &[]);

    crate::dispatch::handle_menu_action(&ui, &cell, crate::menus::action::TOGGLE_WRAP);
    crate::dispatch::handle_menu_action(&ui, &cell, crate::menus::action::TAB_WIDTH_8);

    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("# mine, do not lose"), "{written}");
    assert!(
        written.contains("theme = \"Dark\"  # at night"),
        "{written}"
    );
    assert!(written.contains("word_wrap = true"), "{written}");
    assert!(written.contains("tab_width = 8"), "{written}");

    // And it is what the next launch reads.
    let (_, next) = configured(&path, &[]);
    let next = next.borrow();
    assert!(next.wrap_text);
    assert_eq!(next.indent.width, 8);
}

#[test]
fn a_command_line_override_is_not_saved_when_something_else_changes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "").unwrap();
    let (ui, cell) = configured(&path, &["--font-size", "20"]);
    assert_eq!(cell.borrow().font_size, 20);

    crate::dispatch::handle_menu_action(&ui, &cell, crate::menus::action::TOGGLE_GUTTER);

    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("line_numbers = false"), "{written}");
    assert!(
        !written.contains("font_size"),
        "a one-off --font-size was made permanent: {written}"
    );
}

#[test]
fn picking_the_system_theme_is_saved_as_following_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let (ui, cell) = configured(&path, &[]);

    crate::dispatch::handle_menu_action(&ui, &cell, crate::menus::action::THEME_SYSTEM);

    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("theme = \"System\""), "{written}");
}

// --- the Settings sheet and the menu bar (release plan M1-03, M1-05) -----

/// The window wired as `run_with` wires it for menus and the Settings
/// sheet, over `state`.
fn wired(
    state: crate::state::AppState,
) -> (AppWindow, Rc<std::cell::RefCell<crate::state::AppState>>) {
    let ui = window();
    answer_through_dialog(&ui);
    let cell = Rc::new(std::cell::RefCell::new(state));
    let menu_cell = Rc::clone(&cell);
    let weak = ui.as_weak();
    ui.on_menu_action(move |id| {
        if let Some(ui) = weak.upgrade() {
            let _ = crate::dispatch::handle_menu_action(&ui, &menu_cell, id);
        }
    });
    crate::bind_files(&ui, &cell);
    crate::bind_editing(&ui, &cell);
    crate::bind_settings(&ui, &cell);
    crate::bind_viewer(&ui, &cell);
    crate::bind_find(&ui, &cell);
    crate::refresh(&ui, &mut cell.borrow_mut(), crate::state::PushText::Yes);
    ui.invoke_focus_editor();
    (ui, cell)
}

fn setting(ui: &AppWindow, label: &str) -> String {
    use slint::Model as _;
    let rows = ui.get_setting_rows();
    (0..rows.row_count())
        .filter_map(|i| rows.row_data(i))
        .find(|r| r.label.as_str() == label)
        .map(|r| r.value.to_string())
        .unwrap_or_else(|| panic!("no setting {label:?}"))
}

#[test]
fn ctrl_comma_opens_settings_and_it_holds_the_keyboard() {
    let (ui, _cell) = wired(crate::state::AppState::new());
    let before = ui.get_doc_text();

    chord(&ui, Key::Control, ",");
    assert!(ui.get_settings_open(), "Ctrl+, did not open Settings");
    type_text(&ui, "typed");

    assert_eq!(ui.get_doc_text(), before, "typing got past the sheet");
}

#[test]
fn the_arrows_choose_and_change_a_setting_and_escape_changes_nothing() {
    let (ui, cell) = wired(crate::state::AppState::new());
    chord(&ui, Key::Control, ",");
    assert_eq!(ui.get_settings_selected(), 0);

    // Down twice reaches Word wrap; Right turns it on -- in the sheet only.
    press(&ui, Key::DownArrow);
    press(&ui, Key::DownArrow);
    assert_eq!(ui.get_settings_selected(), 2);
    assert_eq!(setting(&ui, "Word wrap"), "Off");
    press(&ui, Key::RightArrow);
    assert_eq!(setting(&ui, "Word wrap"), "On");
    assert!(!cell.borrow().wrap_text, "a step reached the live state");

    press(&ui, Key::Escape);
    assert!(!ui.get_settings_open());
    assert!(!cell.borrow().wrap_text, "Escape kept the change");
    assert!(cell.borrow().settings_sheet.is_none());
}

#[test]
fn enter_saves_what_changed_into_config_and_nothing_else() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "# mine\n").unwrap();
    let loaded = bp_config::load_from(
        Some(&path),
        &bp_config::Env::default(),
        &["--font-size".to_owned(), "20".to_owned()],
    );
    let mut state = crate::state::AppState::new();
    state.configure(&loaded.config, loaded.path.clone());
    let (ui, cell) = wired(state);

    chord(&ui, Key::Control, ",");
    press(&ui, Key::DownArrow);
    press(&ui, Key::DownArrow);
    press(&ui, Key::Space);
    press(&ui, Key::Return);

    assert!(!ui.get_settings_open());
    assert!(cell.borrow().wrap_text);
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.starts_with("# mine"), "{written}");
    assert!(written.contains("word_wrap = true"), "{written}");
    for untouched in ["font_size", "theme", "tab_width", "default_profile"] {
        assert!(
            !written.contains(untouched),
            "{untouched} was written without being changed: {written}"
        );
    }
}

#[test]
fn the_planned_menus_are_off_the_bar_unless_asked_for() {
    let (ui, _) = wired(crate::state::AppState::new());
    assert!(!ui.get_show_planned_menus());

    let config = bp_config::Config {
        show_planned_menus: true,
        ..bp_config::Config::default()
    };
    let mut state = crate::state::AppState::new();
    state.configure(&config, None);
    let (ui, _) = wired(state);
    assert!(ui.get_show_planned_menus());
}

// --- Help ▸ About (release plan M1-02) ----------------------------------

#[test]
fn about_opens_with_the_licences_and_holds_the_keyboard_until_escape() {
    let (ui, cell) = wired(crate::state::AppState::new());
    crate::about::bind(&ui, &cell);
    let before = ui.get_doc_text();

    crate::dispatch::handle_menu_action(&ui, &cell, crate::menus::action::ABOUT);
    assert!(ui.get_about_open());
    assert!(
        ui.get_about_body()
            .contains("Slint Royalty-free Licence 2.0")
    );
    type_text(&ui, "typed");
    assert_eq!(ui.get_doc_text(), before, "typing got past About");

    press(&ui, Key::Escape);
    assert!(!ui.get_about_open());
    type_text(&ui, "Q");
    assert!(
        ui.get_doc_text().starts_with('Q'),
        "the editor did not get the keys back"
    );
}

#[test]
fn notices_closes_about_and_shows_the_terms_as_a_question_would() {
    let (ui, cell) = wired(crate::state::AppState::new());
    crate::about::bind(&ui, &cell);
    crate::dispatch::handle_menu_action(&ui, &cell, crate::menus::action::ABOUT);
    assert_eq!(
        ui.get_about_highlighted(),
        3,
        "Enter on arrival should close, not act"
    );

    // Left three times from Close is Notices.
    press(&ui, Key::LeftArrow);
    press(&ui, Key::LeftArrow);
    press(&ui, Key::LeftArrow);
    press(&ui, Key::Return);

    assert!(!ui.get_about_open());
    assert!(ui.get_dialog_open());
    assert_eq!(ui.get_dialog_title(), "Notices");
    press(&ui, Key::Escape);
    assert!(!ui.get_dialog_open());
}

// --- the default view's caret (release plan M1-04) -----------------------

/// The wired window over `text`, caret at the top.
fn wired_with(text: &str) -> (AppWindow, Rc<std::cell::RefCell<crate::state::AppState>>) {
    let mut state = crate::state::AppState::new();
    state.edit(text.to_owned());
    let (ui, cell) = wired(state);
    chord(&ui, Key::Control, Key::Home);
    (ui, cell)
}

fn numbered(lines: usize) -> String {
    (1..=lines).map(|i| format!("line {i:03}\n")).collect()
}

#[test]
fn the_status_bar_follows_the_caret_in_the_default_view() {
    let (ui, _) = wired_with("é one\ntwo\n");
    press(&ui, Key::DownArrow);
    press(&ui, Key::RightArrow);
    press(&ui, Key::RightArrow);
    assert_eq!(ui.get_cursor_label(), "Ln 2, Col 3");
    // Through an accent, which is two bytes and one column.
    press(&ui, Key::UpArrow);
    assert_eq!(ui.get_cursor_label(), "Ln 1, Col 3");
}

#[test]
fn ctrl_g_42_lands_on_line_42_in_the_default_view() {
    let (ui, _) = wired_with(&numbered(100));
    chord(&ui, Key::Control, "g");
    assert!(ui.get_goto_open());
    type_text(&ui, "42");
    press(&ui, Key::Return);

    // The caret is in the editor, at the start of line 42: typing lands there.
    press(&ui, Key::Escape);
    type_text(&ui, "#");
    assert_eq!(line_of(&ui, '#'), 42);
    assert!(ui.get_doc_text().contains("#line 042"));
}

#[test]
fn ctrl_d_duplicates_the_line_under_the_caret_and_keeps_the_caret_on_it() {
    let (ui, _) = wired_with("café\nsecond\nthird\n");
    press(&ui, Key::DownArrow);
    chord(&ui, Key::Control, "d");

    assert_eq!(ui.get_doc_text(), "café\nsecond\nsecond\nthird\n");
    type_text(&ui, "X");
    assert!(
        ui.get_doc_text().contains("second\nXsecond")
            || ui.get_doc_text().contains("Xsecond\nsecond"),
        "the caret left the line it duplicated: {:?}",
        ui.get_doc_text()
    );
}

#[test]
fn alt_up_moves_the_line_and_the_caret_goes_with_it() {
    let (ui, _) = wired_with("one\ntwo\nthree\n");
    press(&ui, Key::DownArrow);
    press(&ui, Key::DownArrow);
    chord(&ui, Key::Alt, Key::UpArrow);

    assert_eq!(ui.get_doc_text(), "one\nthree\ntwo\n");
    assert_eq!(ui.get_cursor_label(), "Ln 2, Col 1");
}

#[test]
fn find_selects_the_match_itself_after_accented_text() {
    // `select` used to pass character offsets to a widget that counts
    // bytes, so the selection drifted one place per accent before it.
    let (ui, cell) = wired_with("ééé target\n");
    ui.set_find_query("target".into());
    let range = cell
        .borrow_mut()
        .find(&crate::state::find_query("target", false, false, false))
        .expect("a match");
    crate::dispatch::select(&ui, &mut cell.borrow_mut(), &range);

    // Typing replaces the selection, so what survives shows what was selected.
    type_text(&ui, "HIT");
    assert_eq!(ui.get_doc_text(), "ééé HIT\n");
}

#[test]
fn ctrl_tab_and_ctrl_shift_tab_walk_the_tabs_and_wrap() {
    let mut state = crate::state::AppState::new();
    state.edit("first".to_owned());
    state.new_document();
    state.edit("second".to_owned());
    state.new_document();
    state.edit("third".to_owned());
    let (ui, _) = wired(state);
    assert_eq!(ui.get_doc_text(), "third");

    chord(&ui, Key::Control, Key::Tab);
    assert_eq!(
        ui.get_doc_text(),
        "first",
        "past the last wraps to the first"
    );
    chord(&ui, Key::Control, Key::Tab);
    assert_eq!(ui.get_doc_text(), "second");

    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Shift.into(),
    });
    chord(&ui, Key::Control, Key::Tab);
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Shift.into(),
    });
    assert_eq!(ui.get_doc_text(), "first", "Ctrl+Shift+Tab went forward");
    assert!(!ui.get_doc_text().contains('\t'), "a tab was typed");
}

#[test]
fn a_notice_never_hides_the_path() {
    // Locked decision #5, release plan M1-16: the second status line showed
    // a notice *instead of* the path, so every message hid where the
    // document lives.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.txt");
    std::fs::write(&path, "text\n").unwrap();
    let mut state = crate::state::AppState::new();
    state.open(path);
    state.error = Some("something worth saying".to_owned());
    let (ui, _) = wired(state);

    assert!(
        ui.get_status_path().contains("notes.txt"),
        "{}",
        ui.get_status_path()
    );
    assert_eq!(ui.get_status_notice(), "something worth saying");
}

// --- Find in Files off the UI thread (release plan M1-07) ----------------

/// Drive Slint's timers until `done` says so, or a generous deadline.
fn run_timers_until(done: impl Fn() -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !done() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
        slint::platform::update_timers_and_animations();
    }
}

fn needles() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "a needle\n").unwrap();
    std::fs::write(dir.path().join("b.txt"), "no\nanother needle\n").unwrap();
    dir
}

#[test]
fn find_in_files_runs_in_the_background_and_fills_the_results() {
    let dir = needles();
    let (ui, cell) = wired(crate::state::AppState::new());
    let searches = Rc::new(std::cell::RefCell::new(crate::worker::Background::new()));
    let timer = Rc::new(slint::Timer::default());

    crate::search_folder(
        &ui,
        &cell,
        &searches,
        &timer,
        dir.path().to_path_buf(),
        bp_search::Query::literal("needle"),
    );
    // Back at once, saying so, rather than frozen until the walk is done.
    assert!(ui.get_results_summary().starts_with("searching"));

    run_timers_until(|| !ui.get_results_summary().starts_with("searching"));
    assert_eq!(ui.get_results_summary(), "2 matches in 2 files searched");
    assert_eq!(cell.borrow().file_hits.len(), 2);
    assert!(!timer.running(), "the timer outlived its search");
}

#[test]
fn closing_the_results_stops_the_search_and_drops_what_it_would_have_said() {
    let dir = needles();
    let (ui, cell) = wired(crate::state::AppState::new());
    let searches = Rc::new(std::cell::RefCell::new(crate::worker::Background::new()));
    let timer = Rc::new(slint::Timer::default());

    crate::search_folder(
        &ui,
        &cell,
        &searches,
        &timer,
        dir.path().to_path_buf(),
        bp_search::Query::literal("needle"),
    );
    ui.set_results_open(false);
    run_timers_until(|| !timer.running());

    assert!(!searches.borrow().busy());
    assert!(
        cell.borrow().file_hits.is_empty(),
        "a stopped search filled the list"
    );
    assert!(
        !ui.get_results_open(),
        "a stopped search reopened the panel"
    );
}

// --- Ctrl+S end to end (release plan M1-12) -------------------------------

#[test]
fn ctrl_s_writes_what_was_typed_and_the_document_is_clean_after() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.txt");
    std::fs::write(&path, "first line\n").unwrap();
    let mut state = crate::state::AppState::new();
    state.open(path.clone());
    let (ui, cell) = wired(state);
    chord(&ui, Key::Control, Key::End);
    type_text(&ui, "typed here");
    assert!(
        ui.get_is_dirty(),
        "typing did not mark the document unsaved"
    );

    chord(&ui, Key::Control, "s");

    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "first line\ntyped here"
    );
    assert!(!ui.get_is_dirty(), "the window still says unsaved");
    let id = cell.borrow().workspace.active_id().unwrap();
    assert!(!cell.borrow().is_dirty(id));
}

#[test]
fn ctrl_s_over_a_file_changed_on_disk_asks_and_escape_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shared.txt");
    std::fs::write(&path, "ours\n").unwrap();
    let mut state = crate::state::AppState::new();
    state.open(path.clone());
    let (ui, _cell) = wired(state);
    chord(&ui, Key::Control, Key::End);
    type_text(&ui, "more");
    // Somebody else writes the file, a little later by the clock.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(&path, "theirs, longer than ours\n").unwrap();

    chord(&ui, Key::Control, "s");
    assert!(
        ui.get_dialog_open(),
        "saved over their change without asking"
    );
    assert_eq!(ui.get_dialog_title(), "Changed on disk");
    press(&ui, Key::Return);

    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "theirs, longer than ours\n",
        "Enter is Cancel here, and their change must survive it"
    );
    assert!(ui.get_is_dirty());
}

#[test]
fn line_on_the_command_line_puts_the_caret_there_in_the_default_view() {
    let (ui, _cell) = wired_with(&numbered(100));
    crate::start_at_line(&ui, &_cell, std::num::NonZeroUsize::new(42).unwrap());
    type_text(&ui, "#");
    assert_eq!(line_of(&ui, '#'), 42);
}

#[test]
fn a_line_past_the_end_lands_on_the_last_and_says_so() {
    let (ui, cell) = wired_with(&numbered(10));
    crate::start_at_line(&ui, &cell, std::num::NonZeroUsize::new(500).unwrap());
    let notice = cell.borrow().error.clone().unwrap_or_default();
    assert!(notice.contains("--line 500"), "{notice}");
    assert!(ui.get_status_notice().contains("--line 500"));
}

// --- the menu bar from the keyboard (release plan M2-14) -----------------

#[test]
fn f10_opens_the_first_menu_and_enter_chooses_its_first_row() {
    let (ui, cell) = wired(crate::state::AppState::new());
    crate::set_static_menus(&ui);
    let before = cell.borrow().workspace.len();

    press(&ui, Key::F10);
    press(&ui, Key::Return);

    // File's first row is New.
    assert_eq!(
        cell.borrow().workspace.len(),
        before + 1,
        "Enter did not choose New"
    );
}

#[test]
fn the_arrows_move_through_a_menu_and_right_moves_to_the_next_one() {
    let (ui, cell) = wired(crate::state::AppState::new());
    crate::set_static_menus(&ui);
    let before = cell.borrow().workspace.len();

    // File, then Right to Edit, then Left back to File: New again.
    press(&ui, Key::F10);
    press(&ui, Key::RightArrow);
    press(&ui, Key::LeftArrow);
    press(&ui, Key::Return);
    assert_eq!(cell.borrow().workspace.len(), before + 1);
}

#[test]
fn escape_closes_a_menu_and_typing_reaches_the_document_again() {
    let (ui, _) = wired(crate::state::AppState::new());
    crate::set_static_menus(&ui);
    press(&ui, Key::F10);
    type_text(&ui, "x");
    assert_eq!(
        ui.get_doc_text(),
        "",
        "a key typed into an open menu reached the document"
    );

    press(&ui, Key::Escape);
    type_text(&ui, "Q");
    assert_eq!(ui.get_doc_text(), "Q");
}

#[test]
fn alt_and_a_letter_open_that_menu() {
    // Alt+H opens Help; its first row is Keyboard Shortcuts, a question.
    let (ui, _) = wired(crate::state::AppState::new());
    crate::set_static_menus(&ui);
    chord(&ui, Key::Alt, "h");
    press(&ui, Key::Return);
    assert!(ui.get_dialog_open());
    assert_eq!(ui.get_dialog_title(), "Keyboard shortcuts");
}

#[test]
fn alt_and_a_letter_open_that_menu_in_the_custom_view_too() {
    let mut state = crate::state::AppState::new();
    state.editor_view = true;
    let (ui, cell) = wired(state);
    ui.set_use_editor_view(true);
    crate::set_static_menus(&ui);
    ui.invoke_focus_editor();
    let before = cell.borrow().workspace.len();
    chord(&ui, Key::Alt, "f");
    press(&ui, Key::Return);
    assert_eq!(
        cell.borrow().workspace.len(),
        before + 1,
        "Alt+F then Enter did not choose New"
    );
}

#[test]
fn every_interactive_shell_element_says_what_it_is_to_a_screen_reader() {
    // Release plan M2-14. A role deleted in a restyle is silent: nothing on
    // screen changes, and only a screen-reader user notices.
    let ui = concat!(env!("CARGO_MANIFEST_DIR"), "/ui/");
    for (file, needles) in [
        (
            "menu.slint",
            &["accessible-role: button", "accessible-role: list-item"][..],
        ),
        ("tab.slint", &["accessible-role: tab"][..]),
        (
            "status_bar.slint",
            &["accessible-role: content-info", "accessible-live-region"][..],
        ),
        ("dialog.slint", &["accessible-role: button"][..]),
        ("find_bar.slint", &["accessible-role: search"][..]),
        (
            "app.slint",
            &["accessible-role: navigation", "accessible-role: tab-list"][..],
        ),
        ("viewer.slint", &["accessible-role: slider"][..]),
    ] {
        let source = std::fs::read_to_string(format!("{ui}{file}")).expect(file);
        for needle in needles {
            assert!(source.contains(needle), "{file} lost `{needle}`");
        }
    }
}

// --- the large-file viewer (release plan M1-06) --------------------------

/// A 5,000-line log past a lowered editable limit, open in the wired window
/// with its indexing finished -- by the window's own poll timer, which is
/// the thing being relied on.
fn viewing() -> (
    AppWindow,
    Rc<std::cell::RefCell<crate::state::AppState>>,
    tempfile::TempDir,
    std::path::PathBuf,
    String,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("server.log");
    let text: String = (1..=5_000).map(|i| format!("line {i:06}\n")).collect();
    std::fs::write(&path, &text).unwrap();
    let mut state = crate::state::AppState::new();
    state.editable_limit = 1024;
    state.open(path.clone());
    let (ui, cell) = wired(state);
    run_timers_until(|| !ui.get_viewer_busy());
    assert!(!ui.get_viewer_busy(), "the viewer's poll never finished");
    (ui, cell, dir, path, text)
}

fn viewer_row(ui: &AppWindow, row: usize) -> String {
    use slint::Model as _;
    ui.get_viewer_rows()
        .row_data(row)
        .map(|r| r.text.to_string())
        .unwrap_or_default()
}

fn viewer_rows(ui: &AppWindow) -> Vec<String> {
    use slint::Model as _;
    let rows = ui.get_viewer_rows();
    (0..rows.row_count())
        .filter_map(|i| rows.row_data(i))
        .map(|r| r.text.to_string())
        .collect()
}

#[test]
fn a_file_over_the_limit_shows_the_viewer_with_its_banner_and_scrolls_by_keyboard() {
    let (ui, _cell, _dir, _path, _) = viewing();
    assert!(ui.get_viewer_active());
    assert!(
        ui.get_viewer_banner()
            .starts_with("Read-only: this file is too large to edit"),
        "{}",
        ui.get_viewer_banner()
    );
    assert_eq!(ui.get_save_state(), "Read-only");
    assert_eq!(viewer_row(&ui, 0), "line 000001");
    assert!(ui.get_cursor_label().starts_with("Ln 1 of 5,001"));

    press(&ui, Key::PageDown);
    press(&ui, Key::PageDown);
    assert_ne!(
        viewer_row(&ui, 0),
        "line 000001",
        "two pages down and the view never moved"
    );
    assert!(
        !ui.get_cursor_label().starts_with("Ln 1 of"),
        "the caret stayed"
    );

    chord(&ui, Key::Control, Key::End);
    let rows = viewer_rows(&ui);
    assert_eq!(rows.last().map(String::as_str), Some(""));
    assert!(rows.contains(&"line 005000".to_owned()), "{rows:?}");
    assert!(ui.get_viewer_thumb_position() > 0.99);

    chord(&ui, Key::Control, Key::Home);
    assert_eq!(viewer_row(&ui, 0), "line 000001");
    assert_eq!(ui.get_viewer_thumb_position(), 0.0);
}

#[test]
fn typing_saving_and_line_edits_in_the_viewer_leave_the_file_alone() {
    let (ui, cell, _dir, path, text) = viewing();

    type_text(&ui, "typed");
    assert!(
        ui.get_status_notice().contains("Read-only"),
        "{}",
        ui.get_status_notice()
    );
    chord(&ui, Key::Control, "s");
    chord(&ui, Key::Control, "d");
    chord(&ui, Key::Alt, Key::DownArrow);

    let id = cell.borrow().workspace.active_id().unwrap();
    assert!(!cell.borrow().is_dirty(id));
    assert!(
        ui.get_status_notice().contains("Read-only"),
        "{}",
        ui.get_status_notice()
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    assert_eq!(viewer_row(&ui, 0), "line 000001");
}

#[test]
fn ctrl_g_in_the_viewer_lands_on_the_line() {
    let (ui, _cell, _dir, _path, _) = viewing();
    chord(&ui, Key::Control, "g");
    assert!(ui.get_goto_open());
    type_text(&ui, "4242");
    press(&ui, Key::Return);
    assert!(
        ui.get_cursor_label().starts_with("Ln 4,242 of"),
        "{}",
        ui.get_cursor_label()
    );
    assert!(viewer_rows(&ui).contains(&"line 004242".to_owned()));
}

#[test]
fn find_in_the_viewer_searches_on_the_poll_and_highlights_the_match() {
    use slint::Model as _;
    let (ui, _cell, _dir, _path, _) = viewing();
    chord(&ui, Key::Control, "f");
    type_text(&ui, "003210");
    run_timers_until(|| !ui.get_viewer_busy());

    assert_eq!(ui.get_find_status(), "line 3,210");
    assert!(viewer_rows(&ui).contains(&"line 003210".to_owned()));
    let boxes = ui.get_viewer_highlight();
    assert_eq!(boxes.row_count(), 1, "the match is highlighted");
    let found = boxes.row_data(0).unwrap();
    assert_eq!((found.start, found.end), (5, 11));
}

#[test]
fn switching_tabs_swaps_the_viewer_and_the_editor_and_the_keyboard_follows() {
    let (ui, cell, _dir, _path, _) = viewing();
    cell.borrow_mut().new_document();
    crate::refresh(&ui, &mut cell.borrow_mut(), crate::state::PushText::Yes);
    assert!(!ui.get_viewer_active());
    type_text(&ui, "hello");
    assert_eq!(ui.get_doc_text(), "hello", "the editor has the keyboard");

    // Back one tab: the viewer, which sits between the untitled document
    // the window started with and this new one.
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Shift.into(),
    });
    chord(&ui, Key::Control, Key::Tab);
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Shift.into(),
    });
    assert!(ui.get_viewer_active());
    type_text(&ui, "more");
    assert!(
        ui.get_status_notice().contains("Read-only"),
        "the keys went somewhere other than the viewer: {}",
        ui.get_status_notice()
    );
    assert_eq!(viewer_row(&ui, 0), "line 000001");
    chord(&ui, Key::Control, Key::Tab);
    assert_eq!(
        ui.get_doc_text(),
        "hello",
        "nothing typed at the viewer reached it"
    );
}

// --- clipboard rows and the editor context menu (release plan M1-18) -----

type Shared = Rc<std::cell::RefCell<crate::state::AppState>>;

/// The wired window with `copied` in the clipboard history, newest last,
/// over the custom editor view when `editor_view` and `TextInput` if not.
fn wired_with_clips(copied: &[&str], editor_view: bool) -> (AppWindow, Shared) {
    let mut state = crate::state::AppState::new();
    state.editor_view = editor_view;
    for text in copied {
        state.clips.push(text, bp_security::Clipboard::InMemory);
    }
    let (ui, cell) = wired(state);
    ui.set_use_editor_view(editor_view);
    crate::set_static_menus(&ui);
    crate::refresh(&ui, &mut cell.borrow_mut(), crate::state::PushText::Yes);
    ui.invoke_focus_editor();
    (ui, cell)
}

/// What the clipboard poll does when it sees a new copy: record it and
/// rebuild the menus.
fn poll_sees(ui: &AppWindow, cell: &Shared, text: &str) {
    cell.borrow_mut()
        .clips
        .push(text, bp_security::Clipboard::InMemory);
    crate::refresh(ui, &mut cell.borrow_mut(), crate::state::PushText::No);
}

/// A right-click in the middle of the editor pane, well clear of the menu
/// bar and the tab strip.
fn right_click_in_the_editor(ui: &AppWindow) {
    use slint::platform::PointerEventButton;
    let position = slint::LogicalPosition::new(300.0, 200.0);
    ui.window()
        .dispatch_event(WindowEvent::PointerMoved { position });
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position,
        button: PointerEventButton::Right,
    });
    ui.window().dispatch_event(WindowEvent::PointerReleased {
        position,
        button: PointerEventButton::Right,
    });
}

fn labels(items: &slint::ModelRc<crate::MenuItem>) -> Vec<String> {
    use slint::Model as _;
    items.iter().map(|item| item.label.to_string()).collect()
}

/// The rows that paste an entry as it is, leaving out its transformations.
fn entry_rows(items: &slint::ModelRc<crate::MenuItem>) -> Vec<String> {
    let mut rows = labels(items);
    rows.retain(|label| !label.trim_start().starts_with('↳'));
    rows
}

fn is_pinned(cell: &Shared, text: &str) -> bool {
    cell.borrow()
        .clips
        .entries()
        .iter()
        .find(|e| e.text == text)
        .unwrap_or_else(|| panic!("{text:?} is not in the history"))
        .pinned
}

/// Down from nothing to the context menu's Pin row, into its submenu, and
/// Enter on the submenu's first row. Cut, Copy, Paste, Select All, then
/// the separator the arrows step over, Paste Special, Clipboard History,
/// Pin: seven presses.
fn pin_the_first_row_from_the_keyboard(ui: &AppWindow) {
    for _ in 0..7 {
        press(ui, Key::DownArrow);
    }
    press(ui, Key::RightArrow);
    press(ui, Key::DownArrow);
    press(ui, Key::Return);
}

#[test]
fn a_clipboard_row_pastes_the_entry_it_named_when_a_copy_lands_while_the_edit_menu_is_open() {
    let (ui, cell) = wired_with_clips(&["older", "newest"], true);

    // Edit opens on Undo. Found by label rather than counted, so a row
    // added above the clipboard block does not quietly retarget the test.
    chord(&ui, Key::Alt, "e");
    let rows = labels(&ui.get_edit_items());
    let newest = rows
        .iter()
        .position(|label| label == "newest")
        .expect("Edit shows the newest entry");
    for _ in 0..newest {
        press(&ui, Key::DownArrow);
    }

    // The U5 race: the poll records a copy while the highlight is on the
    // row. Rebuilt from the live history, that row became "arrived later".
    poll_sees(&ui, &cell, "arrived later");
    assert_eq!(labels(&ui.get_edit_items())[newest], "newest");
    press(&ui, Key::Return);

    let text = cell.borrow().active_text();
    assert_eq!(text, "newest", "the row pasted something else: {text:?}");
}

#[test]
fn a_right_click_opens_the_context_menu_over_the_custom_view_and_pin_pins_the_entry_it_names() {
    let (ui, cell) = wired_with_clips(&["older", "newest"], true);
    assert_eq!(
        labels(&ui.get_context_history_items()),
        ["Nothing copied yet"],
        "nothing has been shown yet, so there is no snapshot"
    );

    right_click_in_the_editor(&ui);
    assert_eq!(
        entry_rows(&ui.get_context_history_items()),
        ["newest  (text)", "older  (text)"],
        "opening the menu takes the snapshot its rows are built from"
    );
    let pins = labels(&ui.get_context_pin_items());
    assert!(pins[0].ends_with("newest"), "{pins:?}");
    assert!(ui.get_context_can_clear());

    pin_the_first_row_from_the_keyboard(&ui);
    assert!(is_pinned(&cell, "newest"), "Pin did not reach the entry");
    assert!(!is_pinned(&cell, "older"));
    assert!(
        labels(&ui.get_context_pin_items())[0].starts_with('✓'),
        "the Pin row should tick once its entry is pinned"
    );
}

#[test]
fn a_right_click_opens_the_context_menu_over_the_default_view_too() {
    let (ui, cell) = wired_with_clips(&["older", "newest"], false);
    right_click_in_the_editor(&ui);
    assert_eq!(
        entry_rows(&ui.get_context_history_items()),
        ["newest  (text)", "older  (text)"]
    );
    pin_the_first_row_from_the_keyboard(&ui);
    assert!(is_pinned(&cell, "newest"));
}

#[test]
fn pin_pins_the_entry_it_named_when_a_copy_lands_while_the_context_menu_is_open() {
    let (ui, cell) = wired_with_clips(&["older", "newest"], true);
    right_click_in_the_editor(&ui);
    poll_sees(&ui, &cell, "arrived later");
    pin_the_first_row_from_the_keyboard(&ui);

    assert!(is_pinned(&cell, "newest"), "the row named newest");
    assert!(
        !is_pinned(&cell, "arrived later"),
        "the copy the poll recorded was never on screen"
    );
}

#[test]
fn a_row_whose_entry_has_gone_says_so_and_pastes_nothing() {
    let (ui, cell) = wired_with_clips(&["only"], true);
    chord(&ui, Key::Alt, "e");
    press(&ui, Key::Escape);
    cell.borrow_mut().clips.clear_all();
    crate::dispatch::handle_menu_action(&ui, &cell, crate::menus::action::CLIP_BASE);

    assert_eq!(cell.borrow().active_text(), "");
    let status = ui.get_error_message().to_string();
    assert!(status.contains("no longer in the history"), "{status:?}");
}

#[test]
fn the_menu_key_opens_the_context_menu_from_the_keyboard_in_both_views() {
    for editor_view in [true, false] {
        let (ui, cell) = wired_with_clips(&["older", "newest"], editor_view);
        press(&ui, Key::Menu);
        pin_the_first_row_from_the_keyboard(&ui);
        assert!(is_pinned(&cell, "newest"), "editor view {editor_view}");
    }
}

#[test]
fn shift_f10_opens_the_context_menu_too() {
    let (ui, cell) = wired_with_clips(&["newest"], true);
    chord(&ui, Key::Shift, Key::F10);
    pin_the_first_row_from_the_keyboard(&ui);
    assert!(is_pinned(&cell, "newest"));
}

#[test]
fn a_right_click_in_the_custom_view_leaves_the_caret_and_selection_for_cut_and_copy() {
    use slint::platform::PointerEventButton;
    let (ui, _) = wired_with_clips(&[], true);
    let presses = Rc::new(std::cell::Cell::new(0));
    let counted = Rc::clone(&presses);
    ui.on_editor_pressed(move |_, _, _| counted.set(counted.get() + 1));

    right_click_in_the_editor(&ui);
    press(&ui, Key::Escape);
    assert_eq!(presses.get(), 0, "a right-click placed the caret");

    // The control: a left-click in the same place does.
    let position = slint::LogicalPosition::new(300.0, 200.0);
    for event in [
        WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        },
        WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        },
    ] {
        ui.window().dispatch_event(event);
    }
    assert_eq!(presses.get(), 1);
}
