//! Help ▸ About (release plan M1-02).
//!
//! What a bug report needs -- the version, the commit, what is drawing the
//! window, the platform -- and what the licences require: BachelorPad+'s own
//! terms, and Slint's attribution, which `ui/about.slint` hosts as the
//! `AboutSlint` element (ADR-0031).

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle as _, ModelRc, SharedString, VecModel};

use crate::AppWindow;
use crate::state::AppState;

/// The buttons, left to right. The last closes the sheet.
const BUTTONS: [&str; 4] = ["Notices", "Open Log Folder", "Open Config Folder", "Close"];
const NOTICES: usize = 0;
const LOG_FOLDER: usize = 1;
const CONFIG_FOLDER: usize = 2;

/// The commit and whether the tree had changes, from `build.rs`.
const COMMIT: &str = env!("BP_BUILD_COMMIT");
const BUILD_STATE: &str = env!("BP_BUILD_STATE");

/// The sheet's text for `state`.
pub(crate) fn body(state: &AppState) -> String {
    let build = match BUILD_STATE {
        "clean" => COMMIT.to_owned(),
        "modified" => format!("{COMMIT}, with local changes"),
        _ => format!("{COMMIT} (whether it had local changes is unknown)"),
    };
    format!(
        "Version   {version}\n\
         Commit    {build}\n\
         Drawn by  {renderer}\n\
         Platform  {os} ({arch})\n\n\
         BachelorPad+'s own code is licensed MIT OR Apache-2.0. This program \
         also contains Slint, used under the Slint Royalty-free Licence 2.0, \
         and the DejaVu Sans Mono font; Notices has their terms.",
        version = env!("CARGO_PKG_VERSION"),
        renderer = if state.renderer.is_empty() {
            "not recorded"
        } else {
            state.renderer.as_str()
        },
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
    )
}

/// The third-party terms Notices shows.
///
/// Written here, not generated, until `cargo about` produces the full list
/// for every shipped crate (release plan M1-14). What is here is the part a
/// user can be shown today without guessing: the two components with terms
/// of their own that are visible in the product.
pub(crate) fn notices() -> String {
    format!(
        "Slint\n\
         This program uses Slint (https://slint.dev) under the Slint \
         Royalty-free Desktop, Mobile, and Web Applications Licence 2.0.\n\n\
         DejaVu Sans Mono\n{}",
        include_str!("../fonts/LICENSE-DejaVu.txt").trim()
    )
}

/// Open the sheet over the window.
pub(crate) fn open(ui: &AppWindow, state: &AppState) {
    let buttons: Vec<SharedString> = BUTTONS.iter().map(|&b| b.into()).collect();
    ui.set_about_body(body(state).into());
    ui.set_about_buttons(ModelRc::new(VecModel::from(buttons)));
    ui.set_about_open(true);
    ui.invoke_focus_about();
}

/// Wire the sheet's one callback.
pub(crate) fn bind(ui: &AppWindow, state: &Rc<RefCell<AppState>>) {
    let cell = Rc::clone(state);
    let weak = ui.as_weak();
    ui.on_about_chosen(move |chosen| {
        let Some(ui) = weak.upgrade() else { return };
        match usize::try_from(chosen).unwrap_or(usize::MAX) {
            // Handing over to a question: the About sheet closes first, so
            // the question is the one thing on screen asking for the keys.
            NOTICES => {
                close(&ui);
                crate::dialog::inform(&ui, "Notices", &notices());
            }
            // The sheet stays up: somebody fetching a log for a bug report
            // is likely to want the version beside it.
            LOG_FOLDER => {
                crate::dispatch::open_log_folder(&mut cell.borrow_mut());
                crate::refresh(&ui, &mut cell.borrow_mut(), crate::state::PushText::No);
                ui.invoke_focus_about();
            }
            CONFIG_FOLDER => {
                crate::dispatch::open_config_folder(&mut cell.borrow_mut());
                crate::refresh(&ui, &mut cell.borrow_mut(), crate::state::PushText::No);
                ui.invoke_focus_about();
            }
            _ => close(&ui),
        }
    });
}

fn close(ui: &AppWindow) {
    ui.set_about_open(false);
    ui.invoke_focus_editor();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_names_the_version_the_commit_and_both_licences() {
        let mut state = AppState::new();
        state.renderer = "software renderer".to_owned();
        let text = body(&state);
        assert!(text.contains(env!("CARGO_PKG_VERSION")), "{text}");
        assert!(text.contains(COMMIT), "{text}");
        assert!(text.contains("software renderer"), "{text}");
        assert!(text.contains("MIT OR Apache-2.0"), "{text}");
        assert!(text.contains("Slint Royalty-free Licence 2.0"), "{text}");
    }

    #[test]
    fn the_notices_carry_the_fonts_licence_and_slints_attribution() {
        let text = notices();
        assert!(text.contains("Royalty-free"), "{text}");
        assert!(text.contains("Bitstream"), "the DejaVu licence is missing");
    }

    #[test]
    fn the_last_button_is_the_one_that_closes() {
        assert_eq!(BUTTONS[BUTTONS.len() - 1], "Close");
        assert_eq!(BUTTONS[NOTICES], "Notices");
        assert_eq!(BUTTONS[LOG_FOLDER], "Open Log Folder");
        assert_eq!(BUTTONS[CONFIG_FOLDER], "Open Config Folder");
    }
}
