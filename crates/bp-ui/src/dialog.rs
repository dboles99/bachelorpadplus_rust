//! Questions asked inside the window (release plan M1-01).
//!
//! `ui/dialog.slint` draws the sheet; this decides what it says and what
//! happens once it is answered. Every message dialog in the shell goes
//! through here. `rfd` is left with the file pickers only, which are the one
//! thing a desktop's own dialog does better.
//!
//! The change that matters is that a question no longer blocks. An
//! `rfd::MessageDialog` returned its answer, so the code after it could carry
//! on as if nothing had happened. A sheet is answered later, from an event,
//! so whatever depends on the answer is passed in as a continuation, `then`,
//! and runs when the button is chosen. The upside: no `RefCell` borrow can be
//! held across a question any more, because the question returns before it
//! has an answer. The old code needed a comment at every call site about
//! `rfd` pumping events and a re-entrant callback panicking on a live borrow.
//!
//! One question at a time. A question asked while another is open waits its
//! turn. A question asked by a continuation goes straight up, ahead of any
//! that were waiting, because it is the next step of what the user is
//! answering (closing three tabs asks about each in turn), and anything that
//! was waiting would interrupt it.

use std::cell::RefCell;
use std::collections::VecDeque;

use slint::{ModelRc, SharedString, VecModel};

use crate::AppWindow;

/// What happens once the sheet is answered, given the button's index.
type Then = Box<dyn FnOnce(&AppWindow, usize)>;

/// One question.
pub(crate) struct Dialog {
    pub title: String,
    pub body: String,
    /// Labels, left to right. Each says what it does: "Save", "Don't Save",
    /// never "Yes" and "No".
    pub buttons: Vec<String>,
    /// What Enter chooses.
    pub default: usize,
    /// What Escape chooses, and what closing the window counts as. Always an
    /// answer that changes nothing.
    pub cancel: usize,
}

struct Sheets {
    /// The open question's continuation. `Some` while a sheet is showing.
    open: Option<Then>,
    waiting: VecDeque<(Dialog, Then)>,
}

// Per thread rather than beside `AppState` because a continuation holds the
// state, and the state holding the continuation would be a cycle. Slint runs
// one event loop on one thread, and the window tests run one window per test
// thread, so a thread-local is one queue per window in every case that
// exists.
thread_local! {
    static SHEETS: RefCell<Sheets> = const {
        RefCell::new(Sheets {
            open: None,
            waiting: VecDeque::new(),
        })
    };
}

/// Ask `dialog`, and run `then` with the chosen button's index once it is
/// answered. If another question is already open, this one waits its turn.
pub(crate) fn ask(ui: &AppWindow, dialog: Dialog, then: impl FnOnce(&AppWindow, usize) + 'static) {
    let then: Then = Box::new(then);
    let shown = SHEETS.with_borrow_mut(|sheets| {
        if sheets.open.is_some() {
            sheets.waiting.push_back((dialog, then));
            None
        } else {
            sheets.open = Some(then);
            Some(dialog)
        }
    });
    if let Some(dialog) = shown {
        present(ui, &dialog);
    }
}

/// Show information with a single Close button.
pub(crate) fn inform(ui: &AppWindow, title: &str, body: &str) {
    ask(
        ui,
        Dialog {
            title: title.to_owned(),
            body: body.to_owned(),
            buttons: vec!["Close".to_owned()],
            default: 0,
            cancel: 0,
        },
        |_, _| {},
    );
}

/// Ask for consent to `go` ahead, with Cancel beside it. `then` gets `true`
/// only when the user chose `go`.
///
/// Enter confirms. Every caller has just been asked for this by a menu
/// choice, and the body is the preview the user needs in order to decide.
/// Escape and Cancel refuse.
pub(crate) fn confirm(
    ui: &AppWindow,
    title: &str,
    body: String,
    go: &str,
    then: impl FnOnce(&AppWindow, bool) + 'static,
) {
    ask(
        ui,
        Dialog {
            title: title.to_owned(),
            body,
            buttons: vec![go.to_owned(), "Cancel".to_owned()],
            default: 0,
            cancel: 1,
        },
        move |ui, chosen| then(ui, chosen == 0),
    );
}

/// What to do with a document's unsaved changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unsaved {
    Save,
    Discard,
    Cancel,
}

/// The buttons, left to right, for [`ask_about_unsaved`]. Their order is the
/// mapping [`unsaved_choice`] reads back.
const UNSAVED_BUTTONS: [&str; 3] = ["Save", "Don't Save", "Cancel"];

/// Map a chosen button back to what it meant. Anything out of range is
/// Cancel: the one answer that loses nothing.
pub(crate) fn unsaved_choice(chosen: usize) -> Unsaved {
    match chosen {
        0 => Unsaved::Save,
        1 => Unsaved::Discard,
        _ => Unsaved::Cancel,
    }
}

/// Ask about `name`'s unsaved changes before something discards them.
///
/// Three answers, because Cancel has to be distinguishable from Don't Save,
/// or the safe choice becomes the destructive one. Enter saves; Escape
/// cancels.
pub(crate) fn ask_about_unsaved(
    ui: &AppWindow,
    name: &str,
    then: impl FnOnce(&AppWindow, Unsaved) + 'static,
) {
    ask(
        ui,
        Dialog {
            title: "Unsaved changes".to_owned(),
            body: format!("{name} has unsaved changes.\n\nSave them before closing?"),
            buttons: UNSAVED_BUTTONS.iter().map(|&b| b.to_owned()).collect(),
            default: 0,
            cancel: 2,
        },
        move |ui, chosen| then(ui, unsaved_choice(chosen)),
    );
}

/// Whether a question is showing. The window's close button checks this: a
/// close request while a question is open is answered by the question, not
/// by closing.
pub(crate) fn is_open() -> bool {
    SHEETS.with_borrow(|sheets| sheets.open.is_some())
}

/// The sheet reported a button. Bound to `dialog-answered`.
pub(crate) fn answered(ui: &AppWindow, chosen: i32) {
    let Some(then) = SHEETS.with_borrow_mut(|sheets| sheets.open.take()) else {
        // A second answer for one question -- a double click landing twice
        // before the sheet hid. The first one counted.
        return;
    };
    ui.set_dialog_open(false);

    // Run the continuation with nothing borrowed here, so it can ask its own
    // next question. That question takes the empty slot directly, ahead of
    // anything waiting.
    then(ui, usize::try_from(chosen).unwrap_or(usize::MAX));

    let next = SHEETS.with_borrow_mut(|sheets| {
        if sheets.open.is_some() {
            return None;
        }
        let (dialog, then) = sheets.waiting.pop_front()?;
        sheets.open = Some(then);
        Some(dialog)
    });
    if let Some(dialog) = next {
        present(ui, &dialog);
    }
}

fn present(ui: &AppWindow, dialog: &Dialog) {
    let buttons: Vec<SharedString> = dialog.buttons.iter().map(SharedString::from).collect();
    let index = |i: usize| i32::try_from(i).unwrap_or(0);
    ui.set_dialog_title(dialog.title.as_str().into());
    ui.set_dialog_body(dialog.body.as_str().into());
    ui.set_dialog_buttons(ModelRc::new(VecModel::from(buttons)));
    ui.set_dialog_default(index(dialog.default));
    ui.set_dialog_cancel(index(dialog.cancel));
    ui.set_dialog_open(true);
    ui.invoke_focus_dialog();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_unsaved_buttons_mean_what_they_say() {
        assert_eq!(UNSAVED_BUTTONS[0], "Save");
        assert_eq!(unsaved_choice(0), Unsaved::Save);
        assert_eq!(UNSAVED_BUTTONS[1], "Don't Save");
        assert_eq!(unsaved_choice(1), Unsaved::Discard);
        assert_eq!(UNSAVED_BUTTONS[2], "Cancel");
        assert_eq!(unsaved_choice(2), Unsaved::Cancel);
    }

    #[test]
    fn an_answer_out_of_range_loses_nothing() {
        assert_eq!(unsaved_choice(3), Unsaved::Cancel);
        assert_eq!(unsaved_choice(usize::MAX), Unsaved::Cancel);
    }
}
