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
