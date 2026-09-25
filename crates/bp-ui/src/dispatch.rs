//! Menu action dispatch and shared UI helpers.
//!
//! Owns the menu-action match and the small UI helpers (dialogs, selection)
//! that the action arms and find/replace share.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;

use bp_core::{Document, DocumentId, Encoding, LineEnding};
use bp_platform::editor::{Consent, InstallRefusal, RegistrationPlan};
use bp_theme::ThemeId;

use crate::menus::{self, action};
use crate::state::{AppState, NoteOutcome, PushText, secret_scan_report};
use crate::{AppWindow, dialog, set_os_clipboard};

const SHORTCUTS: &str = "\
Ctrl+N          New
Ctrl+O          Open
Ctrl+S          Save
Ctrl+Shift+S    Save As
Ctrl+W          Close tab
Ctrl+F          Find and replace
Ctrl+Z / Ctrl+Y Undo / Redo
Ctrl+X/C/V      Cut / Copy / Paste
Ctrl+A          Select all
Ctrl+= / Ctrl+- Zoom in / out
Ctrl+0          Reset zoom
Ctrl+D          Duplicate line (--editor-view)
Alt+Up / Down   Move line up / down (--editor-view)";

/// Open a file, starting in a sensible directory.
pub(crate) fn pick_file(state: &crate::state::AppState) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_directory(state.dialog_directory())
        .pick_file()
}

/// Choose a save location, starting in a sensible directory with the
/// ADR-0003 filename suggested.
pub(crate) fn pick_save_path(
    state: &crate::state::AppState,
    id: bp_core::DocumentId,
) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_directory(state.dialog_directory())
        .set_file_name(state.suggested_filename(id))
        .save_file()
}

/// Give a chosen path the `.bpadx` extension.
///
/// The file dialog suggests the document's current name, and a user who
/// accepts it would otherwise get an encrypted file called `notes.txt` --
/// which the shell would happily reopen, but which every other program on the
/// machine would treat as text and show as binary noise.
fn with_bpadx_extension(path: PathBuf) -> PathBuf {
    if path.extension().is_some_and(|e| e == "bpadx") {
        path
    } else {
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(".bpadx");
        path.with_file_name(name)
    }
}

/// Choose the public key a signature is claimed to have been made with.
fn pick_verifying_key(state: &AppState) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Choose the public key to verify against")
        .set_directory(state.dialog_directory())
        .add_filter("Public key", &["pub", "key", "txt"])
        .add_filter("Any file", &["*"])
        .pick_file()
}

/// Choose where to put the registry script Windows registration needs.
///
/// The file name comes from `bp-platform`'s own artefact, so what the user
/// saves is what the plan names. Where it goes is entirely their choice --
/// ADR-0012's whole position is that the user reads this file and applies it
/// themselves, and a location chosen for them is one step back towards an
/// installer writing the same keys invisibly.
fn pick_registry_script(state: &AppState, plan: &RegistrationPlan) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Save the registry script")
        .set_directory(state.dialog_directory())
        .set_file_name(crate::default_editor::registry_script_name(plan))
        .add_filter("Registry script", &["reg"])
        .save_file()
}

/// Show what registration would do, and ask before doing any of it.
///
/// The body is built in `default_editor`, which puts the current
/// associations above the plan: ADR-0012 is about the user keeping control,
/// and a user cannot keep control of a change they were not shown the
/// starting point of. The answer becomes the `Consent` value
/// `bp_platform::editor::install` refuses to act without.
fn confirm_registration(
    ui: &AppWindow,
    body: String,
    then: impl FnOnce(&AppWindow, Consent) + 'static,
) {
    dialog::confirm(ui, "Set as default editor", body, "Register", |ui, go| {
        then(
            ui,
            if go {
                Consent::Granted
            } else {
                Consent::Withheld
            },
        );
    });
}

/// Act on the answer to [`confirm_registration`], and say what happened.
fn register(
    ui: &AppWindow,
    state: &Rc<RefCell<AppState>>,
    offer: &crate::default_editor::Offer,
    consent: Consent,
) -> String {
    match (consent, offer.root.as_deref()) {
        (Consent::Withheld, _) => InstallRefusal::ConsentWithheld.describe(),
        // The only call in this application that writes into a directory
        // the desktop environment reads. It takes the root `bp-platform`
        // named and the consent the user just gave, and neither has a
        // default.
        (Consent::Granted, Some(root)) => {
            let (status, body) = crate::default_editor::install_outcome(
                bp_platform::editor::install(&offer.plan, root, consent),
                &offer.plan.handoff,
                root,
            );
            if let Some(body) = body {
                dialog::inform(ui, "Set as default editor", &body);
            }
            status
        }
        // No install root means this platform has no supported install --
        // Windows. The `.reg` script is what ADR-0012 leaves in its place: a
        // file the user chose the location of and can read before running.
        (Consent::Granted, None) => {
            let chosen = pick_registry_script(&state.borrow(), &offer.plan);
            let Some(path) = chosen else {
                return "nothing was written — no location was chosen for the registry script"
                    .to_owned();
            };
            match crate::default_editor::save_registry_script(&offer.plan, &path) {
                Ok(status) => {
                    let note = crate::default_editor::handoff_note(&offer.plan.handoff);
                    dialog::inform(ui, "Set as default editor", &format!("{status}\n\n{note}"));
                    status
                }
                Err(refusal) => refusal,
            }
        }
    }
}

/// Show what a redaction is about to destroy, and ask before destroying it.
///
/// The listing is not bounded the way `confirm_replace`'s is. A scan that
/// found four hundred credentials is a document nobody should redact without
/// reading the list, and a truncated list is one nobody read -- the sheet
/// scrolls instead.
fn confirm_redaction(
    ui: &AppWindow,
    plan: crate::state::RedactionPlan,
    then: impl FnOnce(&AppWindow, crate::state::RedactionPlan, bool) + 'static,
) {
    let body = plan.consent_body();
    dialog::confirm(ui, "Redact", body, "Redact", move |ui, go| {
        then(ui, plan, go)
    });
}

/// Show what Replace All would do, and ask before doing it. `then` gets the
/// plan back only if the user chose to go ahead.
///
/// specs.md section 6 wants the changes visible before they are applied.
/// Replace All is the one search operation that rewrites the document in
/// places the user cannot see, so it is also the one worth confirming --
/// undo covers a mistake, but only if you notice you made one.
///
/// The listing is bounded: a preview of forty thousand replacements is not a
/// preview.
pub(crate) fn confirm_replace(
    ui: &AppWindow,
    plan: bp_search::ReplacePlan,
    then: impl FnOnce(&AppWindow, Option<bp_search::ReplacePlan>) + 'static,
) {
    const SHOWN: usize = 12;

    let body = format!("{}\n\n{}", plan.summary(), plan.preview(SHOWN));
    dialog::confirm(ui, "Replace All", body, "Replace All", move |ui, go| {
        then(ui, go.then_some(plan));
    });
}

/// Select a character range in the editor.
///
/// Under the custom surface we own the selection, so this sets it and asks
/// only for the focus back. Under `TextInput` the widget owns it, and the
/// offsets are clamped into `i32` because that is what its API takes -- a
/// document long enough to overflow one would have other problems first.
pub(crate) fn select(ui: &AppWindow, state: &mut AppState, range: &std::ops::Range<usize>) {
    use crate::editor_view::draw_editor_view;

    if state.editor_view {
        if let Some(editor) = state.active_editor_mut() {
            editor.select(range.start, range.end);
        }
        state.reveal_caret();
        draw_editor_view(ui, state);
        ui.invoke_focus_editor();
        return;
    }
    let start = i32::try_from(range.start).unwrap_or(i32::MAX);
    let end = i32::try_from(range.end).unwrap_or(i32::MAX);
    ui.invoke_select_range(start, end);
}

/// Handle a menu action id from the UI.
///
/// Returns `Some(push)` when the action was handled and the caller should
/// refresh with the given `PushText` value. Returns `None` when the action
/// already performed its own refresh, or returned early without changing
/// state, so no further refresh is needed.
pub fn handle_menu_action(
    ui: &AppWindow,
    state: &Rc<RefCell<AppState>>,
    id: i32,
) -> Option<PushText> {
    use crate::editor_view::apply_editor_command;
    use crate::{
        close_each, close_with_prompt, refresh, refresh_after, save_each, save_with_prompt,
    };

    let mut push = PushText::Yes;
    match id {
        action::NEW => state.borrow_mut().new_document(),
        action::OPEN => {
            let chosen = pick_file(&state.borrow());
            if let Some(path) = chosen {
                // An encrypted file does not become a tab until it is
                // unlocked: a tab nobody can read looks like an empty
                // document, and saving it would write emptiness over the
                // real one.
                if state.borrow_mut().open_maybe_encrypted(path) {
                    ui.invoke_focus_passphrase();
                    return None;
                }
            }
        }
        action::SAVE => {
            let id = state.borrow().workspace.active_id();
            if let Some(id) = id {
                save_with_prompt(ui, state, id, refresh_after(state));
                return None;
            }
        }
        action::SAVE_AS => {
            let target = state.borrow().workspace.active_id();
            if let Some(id) = target {
                crate::save_as(ui, state, id);
                return None;
            }
        }
        action::SAVE_ALL => {
            let dirty: VecDeque<DocumentId> =
                state.borrow().workspace.dirty().map(Document::id).collect();
            save_each(ui, state, dirty);
            return None;
        }
        action::RELOAD => {
            let (dirty, name) = {
                let s = state.borrow();
                let id = s.workspace.active_id();
                (
                    id.is_some_and(|i| s.is_dirty(i)),
                    id.map(|i| s.display_name(i)).unwrap_or_default(),
                )
            };
            // Reloading discards edits, so it asks like closing does -- and
            // goes ahead only on Don't Save, since Save followed by a reload
            // is a reload of what was just written.
            if dirty {
                let cell = Rc::clone(state);
                dialog::ask_about_unsaved(ui, &name, move |ui, answer| {
                    if answer == dialog::Unsaved::Discard {
                        cell.borrow_mut().reload();
                        refresh(ui, &mut cell.borrow_mut(), PushText::Yes);
                    }
                });
                return None;
            }
            state.borrow_mut().reload();
        }
        action::CLOSE_TAB => {
            let id = state.borrow().workspace.active_id();
            if let Some(id) = id {
                close_with_prompt(ui, state, id, refresh_after(state));
                return None;
            }
        }

        // Picking a theme by name stops following the system. Leaving the
        // flag set would let the next preference change silently undo the
        // choice the user just made.
        action::THEME_LIGHT | action::THEME_DARK | action::THEME_ORGANIC | action::THEME_GREEN => {
            let mut s = state.borrow_mut();
            s.theme = match id {
                action::THEME_LIGHT => ThemeId::Light,
                action::THEME_DARK => ThemeId::Dark,
                action::THEME_ORGANIC => ThemeId::Organic,
                _ => ThemeId::Green,
            };
            s.follow_system_theme = false;
            let name = s.theme.name();
            s.remember(|c| c.theme = Some(name.to_owned()));
        }
        action::THEME_SYSTEM => {
            let mut s = state.borrow_mut();
            s.follow_system_theme = true;
            s.remember(|c| c.theme = Some(bp_theme::SYSTEM_THEME_NAME.to_owned()));
            match ThemeId::for_system(s.system_dark) {
                Some(theme) => s.theme = theme,
                // Not an error, and not a guess: a desktop that will not say
                // leaves the current theme alone. The row still ticks, so the
                // preference is remembered for when an answer arrives.
                None => {
                    s.error = Some(
                        "this desktop does not report a light or dark preference; \
                                    keeping the current theme"
                            .to_owned(),
                    );
                }
            }
        }

        action::TOGGLE_GUTTER => {
            let mut s = state.borrow_mut();
            s.show_gutter = !s.show_gutter;
            let on = s.show_gutter;
            s.remember(|c| c.line_numbers = on);
            push = PushText::No;
        }
        action::TOGGLE_WRAP => {
            let mut s = state.borrow_mut();
            s.wrap_text = !s.wrap_text;
            let on = s.wrap_text;
            s.remember(|c| c.word_wrap = on);
            push = PushText::No;
        }

        // The document does not change, so the text is not re-pushed: under
        // `TextInput` that would discard the widget's caret and selection for
        // a change of size, which is the one thing a zoom must not do.
        action::ZOOM_IN | action::ZOOM_OUT => {
            let mut s = state.borrow_mut();
            let steps = if id == action::ZOOM_IN { 1 } else { -1 };
            s.font_size = bp_config::zoom(s.font_size, steps);
            let size = s.font_size;
            s.remember(|c| c.font_size = size);
            push = PushText::No;
        }
        action::ZOOM_RESET => {
            let mut s = state.borrow_mut();
            s.font_size = bp_config::DEFAULT_FONT_SIZE;
            s.remember(|c| c.font_size = bp_config::DEFAULT_FONT_SIZE);
            push = PushText::No;
        }

        // A copy, deliberately not a save: `save_copy` leaves the path, the
        // dirty flag and the recent-files list exactly where they were.
        action::SAVE_COPY => {
            // Same shape as Save As -- the borrow ends with the statement,
            // before anything takes a mutable one.
            let target = state.borrow().workspace.active_id();
            if let Some(id) = target {
                let chosen = pick_save_path(&state.borrow(), id);
                if let Some(path) = chosen {
                    state.borrow_mut().save_copy(id, &path);
                }
            }
            // The document did not change, so its text must not be re-pushed:
            // under `TextInput` that would throw away the caret for an
            // operation that did not touch the document at all.
            push = PushText::No;
        }

        action::DOCUMENT_STATS => {
            state.borrow_mut().report_statistics();
            push = PushText::No;
        }

        // Tab context menu. All three act on `tab_context` -- the tab that was
        // right-clicked -- falling back to the active one, so a row can never
        // act on a tab the user was not pointing at.
        action::CLOSE_OTHER_TABS | action::CLOSE_ALL_TABS => {
            let keep = (id == action::CLOSE_OTHER_TABS)
                .then(|| state.borrow().tab_context_id())
                .flatten();
            let doomed: VecDeque<DocumentId> = state
                .borrow()
                .workspace
                .iter()
                .map(Document::id)
                .filter(|id| Some(*id) != keep)
                .collect();
            // Asking about each, so unsaved work still asks -- closing
            // several tabs is exactly when losing one would hurt.
            close_each(ui, state, doomed);
            return None;
        }
        action::COPY_TAB_PATH => {
            let path = {
                let s = state.borrow();
                s.tab_context_id()
                    .and_then(|id| s.workspace.get(id))
                    .and_then(Document::path)
                    .map(|p| p.display().to_string())
            };
            let mut s = state.borrow_mut();
            s.error = match path {
                // A path, not document text, so the OS may keep it.
                Some(path) if crate::set_os_clipboard(&path, false) => {
                    Some(format!("copied {path}"))
                }
                Some(_) => Some("could not reach the clipboard".to_owned()),
                None => Some("this document has never been saved".to_owned()),
            };
            push = PushText::No;
        }

        id if (action::PROFILE_BASE..menus::profile_end()).contains(&id) => {
            let index = usize::try_from(id - action::PROFILE_BASE).unwrap_or(0);
            if let Some(profile) = bp_security::Profile::all().get(index).copied() {
                state
                    .borrow_mut()
                    .set_security(bp_security::Security::Named(profile));
            }
            push = PushText::No;
        }

        action::PRIVACY_MODE => {
            let next = if state.borrow().privacy.is_on() {
                bp_security::Privacy::Off
            } else {
                bp_security::Privacy::On
            };
            state.borrow_mut().set_privacy(next);
            push = PushText::No;
        }

        action::ENCRYPT_DOCUMENT => {
            // Save As, not encrypt-in-place: the plaintext original is left
            // where it was rather than silently destroyed by an operation
            // that is irreversible without the passphrase. The tab then
            // adopts the encrypted file, so later saves stay encrypted.
            let target = state.borrow().workspace.active_id();
            if let Some(id) = target {
                let suggested = pick_save_path(&state.borrow(), id);
                if let Some(path) = suggested {
                    let path = with_bpadx_extension(path);
                    state.borrow_mut().ask = Some(crate::passphrase::Ask::Set { id, target: path });
                    ui.invoke_focus_passphrase();
                }
            }
            return None;
        }

        action::SCAN_SECRETS => {
            let findings = state.borrow_mut().scan_for_secrets();
            // The listing is a dialog rather than more status bar: the bar
            // elides, and the positions are the part worth reading. Nothing
            // here is the matched text -- a `Finding` does not carry it, and
            // reaching back into the document to quote it would undo the one
            // decision the crate is built around.
            if !findings.is_empty() {
                dialog::inform(ui, "Possible credentials", &secret_scan_report(&findings));
            }
            push = PushText::No;
        }

        action::HASH_DOCUMENT => {
            let report = state.borrow_mut().hash_active_document();
            if let Some(report) = report {
                dialog::inform(ui, "Document hash", &report);
            }
            push = PushText::No;
        }

        action::SIGN_DOCUMENT => {
            // The row is greyed, so this is only reachable by a keyboard
            // route that does not exist yet -- but a silent no-op would be a
            // bug report nobody could describe, the same reasoning as the
            // stamp arm above.
            state.borrow_mut().error = Some(
                "signing needs a signing key, and there is nowhere to keep one yet".to_owned(),
            );
            push = PushText::No;
        }

        action::VERIFY_SIGNATURE => {
            push = PushText::No;
            // The first pass asks the user nothing, because a key cannot
            // change its answer: whether a sidecar is there at all, whether
            // it reads as one, and whether it holds against the key it itself
            // names are all settled before anybody is asked for a file. A
            // missing one fails here, closed, and stops.
            let holds = state
                .borrow_mut()
                .verify_signature(&bp_integrity::Expectation::AnySigner);

            // Only now is a key worth asking for. It is the one thing that
            // tells "signed by who you expected" from "signed by somebody
            // else" -- the verdict a bare 64-byte `.sig` cannot produce, and
            // the reason ADR-0026's sidecar records a key at all. Cancelling
            // leaves the first pass's answer standing, caveat and all, which
            // is why that caveat is written.
            if holds && let Some(path) = pick_verifying_key(&state.borrow()) {
                let expect = state.borrow_mut().expected_signer(&path);
                if let Some(expect) = expect {
                    state.borrow_mut().verify_signature(&expect);
                }
            }
        }

        action::SET_DEFAULT_EDITOR => {
            match crate::default_editor::offer() {
                Err(reason) => state.borrow_mut().error = Some(reason),
                Ok(offer) => {
                    let cell = Rc::clone(state);
                    let body = offer.body.clone();
                    confirm_registration(ui, body, move |ui, consent| {
                        let message = register(ui, &cell, &offer, consent);
                        cell.borrow_mut().error = Some(message);
                        refresh(ui, &mut cell.borrow_mut(), PushText::No);
                    });
                    return None;
                }
            }
            push = PushText::No;
        }

        action::REDACT_SECRETS => {
            // The consent is not ceremony. Redaction destroys text the user
            // wrote, in places on the screen they cannot all see at once, and
            // it is offered because a scanner *guessed* the text was a
            // credential. Replace All is confirmed for the weaker version of
            // the same reason.
            let plan = state.borrow_mut().plan_redaction();
            push = PushText::No;
            if let Some(plan) = plan {
                let cell = Rc::clone(state);
                confirm_redaction(ui, plan, move |ui, plan, go| {
                    // The document changed only if it was redacted, so only
                    // then is its text re-pushed -- re-pushing throws away
                    // `TextInput`'s caret.
                    let push = if go && cell.borrow_mut().apply_redaction(&plan) {
                        PushText::Yes
                    } else {
                        if !go {
                            cell.borrow_mut().decline_redaction(&plan);
                        }
                        PushText::No
                    };
                    refresh(ui, &mut cell.borrow_mut(), push);
                });
                return None;
            }
        }

        action::SECURITY_HISTORY => {
            let report = state.borrow_mut().security_history();
            if let Some(report) = report {
                dialog::inform(ui, "Security History", &report);
            }
            push = PushText::No;
        }

        action::INSPECT_METADATA => {
            let report = state.borrow_mut().inspect_metadata();
            if let Some(report) = report {
                dialog::inform(ui, "Metadata", &report);
            }
            push = PushText::No;
        }

        action::GO_TO_LINE => {
            // The bar owns the interaction from here; opening it is all the
            // menu row does, which is why one action id covers the feature.
            ui.invoke_focus_goto();
            return None;
        }

        // Indentation. The width applies to both modes -- it is how far a
        // literal tab reaches as well as how far a soft one goes.
        action::INDENT_TABS | action::INDENT_SPACES => {
            let mut s = state.borrow_mut();
            s.indent.spaces = id == action::INDENT_SPACES;
            let spaces = s.indent.spaces;
            s.remember(|c| c.indent_spaces = spaces);
            push = PushText::No;
        }
        action::TAB_WIDTH_2 | action::TAB_WIDTH_4 | action::TAB_WIDTH_8 => {
            let mut s = state.borrow_mut();
            let width: u8 = match id {
                action::TAB_WIDTH_2 => 2,
                action::TAB_WIDTH_8 => 8,
                _ => 4,
            };
            s.indent.width = usize::from(width);
            s.remember(|c| c.tab_width = width);
            push = PushText::No;
        }

        id if (action::STAMP_BASE..menus::stamp_end()).contains(&id) => {
            let index = usize::try_from(id - action::STAMP_BASE).unwrap_or(0);
            let mut s = state.borrow_mut();
            if !s.insert_stamp(index) {
                // The rows are disabled without the custom editor view, so
                // this is only reachable by a keyboard route that does not
                // exist yet -- but a silent no-op would be a bug report
                // nobody could describe.
                s.error = Some("date and time insertion needs --editor-view".to_owned());
            }
            // `PushText::Yes`, the default: the document changed, same as
            // Duplicate Line beside it.
        }

        action::LINE_ENDING_LF => state.borrow_mut().set_line_ending(LineEnding::Lf),
        action::LINE_ENDING_CRLF => state.borrow_mut().set_line_ending(LineEnding::CrLf),
        action::ENCODING_UTF8 => state.borrow_mut().set_encoding(Encoding::Utf8),
        action::ENCODING_UTF8_BOM => state.borrow_mut().set_encoding(Encoding::Utf8Bom),
        action::ENCODING_WINDOWS_1252 => state.borrow_mut().set_encoding(Encoding::Windows1252),

        action::SHORTCUTS => dialog::inform(ui, "Keyboard shortcuts", SHORTCUTS),
        action::ABOUT => dialog::inform(
            ui,
            "About BachelorPad+",
            &format!(
                "BachelorPad+ {}\n\nNotepad when you want it. More when you need it.\n\n\
                 Renderer: {}\nLicence: MIT OR Apache-2.0",
                env!("CARGO_PKG_VERSION"),
                std::env::var("SLINT_BACKEND").unwrap_or_else(|_| "software".to_owned()),
            ),
        ),

        // Only reachable with the custom surface. Under `TextInput`
        // Slint handles these on the widget itself and they never get
        // this far.
        id if (action::UNDO..=action::SELECT_ALL).contains(&id) => {
            let command = match id {
                action::UNDO => bp_editor::Command::Undo,
                action::REDO => bp_editor::Command::Redo,
                action::CUT => bp_editor::Command::Cut,
                action::COPY => bp_editor::Command::Copy,
                action::PASTE => bp_editor::Command::Paste,
                _ => bp_editor::Command::SelectAll,
            };
            apply_editor_command(&mut state.borrow_mut(), &command);
            push = PushText::No;
        }

        // Bounded above by `clip_end()` -- an unbounded `>=` here is exactly
        // what let the recent-files arm swallow everything up to 100 before
        // it had a real range, and the paste-transformation ids just below
        // would have fallen into this arm the same way.
        id if (action::CLIP_BASE..menus::clip_end()).contains(&id) => {
            let index = usize::try_from(id - action::CLIP_BASE).unwrap_or(0);
            let text = state.borrow().clips.get(index).map(|e| e.text.clone())?;
            set_os_clipboard(&text, state.borrow().keep_clipboard_private());

            let owns_caret = state.borrow().editor_view;
            if owns_caret {
                {
                    let mut s = state.borrow_mut();
                    if let Some(editor) = s.active_editor_mut() {
                        editor.insert(&text);
                    }
                    s.mark_edited();
                }
                refresh(ui, &mut state.borrow_mut(), PushText::No);
                ui.invoke_focus_editor();
            } else {
                // The OS clipboard now holds the entry, so the
                // widget's own paste puts it at the caret -- the one
                // way to insert there without caret access.
                ui.invoke_paste_from_clipboard();
            }
            return None;
        }

        // A paste transformation. Same two-path shape as the plain
        // clipboard rows just above -- the caret is reached differently
        // under each editor view, and missing one path is how a feature
        // works for whoever tested it and does nothing for whoever did not.
        id if (action::CLIP_TRANSFORM_BASE..menus::clip_transform_end()).contains(&id) => {
            let (entry_index, transform) = {
                let s = state.borrow();
                menus::decode_transform(id, s.clips.entries())?
            };
            let source = state
                .borrow()
                .clips
                .get(entry_index)
                .map(|e| e.text.clone())?;
            let text = menus::apply_transform(transform, &source)?;
            set_os_clipboard(&text, state.borrow().keep_clipboard_private());

            let owns_caret = state.borrow().editor_view;
            if owns_caret {
                {
                    let mut s = state.borrow_mut();
                    if let Some(editor) = s.active_editor_mut() {
                        editor.insert(&text);
                    }
                    s.mark_edited();
                }
                refresh(ui, &mut state.borrow_mut(), PushText::No);
                ui.invoke_focus_editor();
            } else {
                ui.invoke_paste_from_clipboard();
            }
            return None;
        }

        id if (action::NOTE_TITLE..=action::NOTE_OUTLINE).contains(&id) => {
            let outcome = state.borrow().note_action(id);
            match outcome {
                NoteOutcome::Show { title, body } => dialog::inform(ui, &title, &body),
                NoteOutcome::SaveAs => {
                    let target = state.borrow().workspace.active_id();
                    if let Some(doc_id) = target {
                        crate::save_as(ui, state, doc_id);
                        return None;
                    }
                }
                NoteOutcome::Nothing => {}
            }
        }

        // Two ranges rather than one, because the Data block at 70-79 had a
        // single id left when YAML needed two. Both reach the same place; the
        // arm that decides which library function a click meant is
        // `run_data_action`, where the format is in scope.
        id if (action::DATA_VALIDATE..=action::DATA_SORT_KEYS).contains(&id)
            || (action::DATA_YAML_TO_JSON..=action::DATA_JSON_TO_YAML).contains(&id) =>
        {
            state.borrow_mut().run_data_action(id);
        }

        id if (action::LINES_SORT_ASC..=action::LINES_TRIM).contains(&id) => {
            state.borrow_mut().run_line_action(id);
        }

        // Duplicate Line, Move Line Up/Down. A separate range from the line
        // operations above: those replace the whole document, these need
        // the caret, and `run_line_edit` is where that distinction lives.
        id if (action::DUPLICATE_LINE..=action::MOVE_LINE_DOWN).contains(&id) => {
            state.borrow_mut().run_line_edit(id);
        }

        // Recently opened files. Bounded by the length of the list
        // rather than by the next block of ids: this arm used to
        // claim everything up to 100, which was safe only because
        // every other arm in that window happened to come first.
        id if (action::RECENT_BASE
            ..action::RECENT_BASE + i32::try_from(bp_config::MAX_RECENT).unwrap_or(0))
            .contains(&id) =>
        {
            let index = usize::try_from(id - action::RECENT_BASE).unwrap_or(0);
            let path = state.borrow().recent_path(index);
            if let Some(path) = path {
                state.borrow_mut().open(path);
            }
        }

        // action::NONE and anything unrecognised: a row that exists
        // to describe what is coming.
        _ => return None,
    }

    refresh(ui, &mut state.borrow_mut(), push);
    None
}
