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
    crate::bind_settings(&ui, &cell);
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
