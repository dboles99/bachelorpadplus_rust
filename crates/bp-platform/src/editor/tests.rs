//! Tests for default-editor registration.
//!
//! Two rules shape this file. **No test installs anything anywhere but a
//! `tempfile` directory** -- there is no code path that could reach the real
//! `~/.local/share`, because [`install`] has no default root, but a test that
//! wrote one would still be wrong. And **every test runs on both legs of CI**,
//! including the Windows ones: [`plan`] takes the platform as an argument, so
//! the `.reg` shape and the ADR-0012 guard are checked on machines with no
//! registry, which is where they would otherwise never be checked at all.

use super::*;
use proptest::prelude::*;
use std::collections::BTreeMap;
use tempfile::tempdir;

fn app() -> AppInfo {
    AppInfo::bachelorpad("/usr/local/bin/bachelorpad")
}

fn windows_app() -> AppInfo {
    AppInfo::bachelorpad(r"C:\Program Files\BachelorPad+\bachelorpad.exe")
}

fn everything() -> AssociationSelection {
    AssociationSelection::preset(AssociationPreset::EverythingSupported)
}

/// The `key=value` pairs of a `.desktop` file, in order.
fn desktop_keys(entry: &str) -> Vec<(&str, &str)> {
    entry
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('['))
        .map(|line| {
            line.split_once('=')
                .unwrap_or_else(|| panic!("a .desktop line without an `=`: {line:?}"))
        })
        .collect()
}

// --- the file-type table ---------------------------------------------------

/// Whether `bp-formats` must be able to identify this extension.
///
/// Every type except the product's own. `.bpadx` is an encrypted envelope
/// (ADR-0021): `bp-crypto` opens it and *then* the plaintext inside is
/// detected, so `bp_formats::Format` has no variant for it and should not --
/// the question "what format is this" has no answer until it is decrypted.
/// The exemption is by group rather than by extension so that a second
/// product-owned type inherits it without anyone editing this test.
fn must_be_parseable(file_type: &FileType) -> bool {
    file_type.group != TypeGroup::Own
}

#[test]
fn every_registerable_extension_is_one_bp_formats_can_identify() {
    // The one agreement that must hold between the registration table and the
    // parser: claiming a file type the editor cannot open is the failure the
    // user experiences as a broken machine rather than a missing feature.
    for file_type in FILE_TYPES.iter().filter(|t| must_be_parseable(t)) {
        assert!(
            bp_formats::Format::from_extension(file_type.extension).is_some(),
            ".{} is offered for registration and bp-formats does not know it",
            file_type.extension
        );
    }
}

#[test]
fn extensions_are_unique_lowercase_and_undotted() {
    let mut seen: Vec<&str> = FILE_TYPES.iter().map(|t| t.extension).collect();
    let count = seen.len();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), count, "an extension is listed twice");
    for file_type in FILE_TYPES {
        assert!(!file_type.extension.starts_with('.'), "{file_type:?}");
        assert_eq!(
            file_type.extension.to_ascii_lowercase(),
            file_type.extension,
            "{file_type:?}"
        );
        assert!(!file_type.description.is_empty(), "{file_type:?}");
        assert!(file_type.mime.contains('/'), "{file_type:?}");
    }
}

#[test]
fn a_file_type_is_found_however_the_extension_is_spelled() {
    for spelling in ["txt", ".txt", ".TXT", "  Txt "] {
        assert_eq!(
            file_type(spelling).map(|t| t.extension),
            Some("txt"),
            "{spelling}"
        );
    }
    assert_eq!(file_type("docx"), None);
    assert_eq!(file_type(""), None);
    assert_eq!(file_type("."), None);
}

// --- presets ---------------------------------------------------------------

#[test]
fn the_escalating_presets_are_nested_in_the_order_they_are_offered() {
    // The same property the security profiles earn: each step adds and never
    // takes away, so a user moving down the list never loses an association
    // they had. `Developer` is deliberately excluded -- see the next test.
    let chain = [
        AssociationPreset::NotepadReplacement,
        AssociationPreset::TextAndNotes,
        AssociationPreset::TextAndStructuredData,
        AssociationPreset::EverythingSupported,
    ];
    for pair in chain.windows(2) {
        let narrower = AssociationSelection::preset(pair[0]);
        let wider = AssociationSelection::preset(pair[1]);
        for extension in narrower.extensions() {
            assert!(
                wider.contains(extension),
                "{} loses .{extension} relative to {}",
                pair[1].label(),
                pair[0].label()
            );
        }
        assert!(
            wider.len() > narrower.len(),
            "{} adds nothing to {}",
            pair[1].label(),
            pair[0].label()
        );
    }
}

#[test]
fn developer_is_not_on_the_escalating_chain_and_says_so_by_dropping_the_spreadsheets() {
    let developer = AssociationSelection::preset(AssociationPreset::Developer);
    let structured = AssociationSelection::preset(AssociationPreset::TextAndStructuredData);
    assert!(developer.contains("rs") && !structured.contains("rs"));
    assert!(structured.contains("csv") && !developer.contains("csv"));
}

#[test]
fn everything_supported_is_the_whole_table_and_the_others_are_inside_it() {
    let everything = everything();
    assert_eq!(everything.len(), FILE_TYPES.len());
    for &preset in AssociationPreset::ALL {
        let selection = AssociationSelection::preset(preset);
        assert!(!selection.is_empty(), "{} is empty", preset.label());
        // Every preset opens plain text and the product's own format --
        // whatever else the user chose, the product must open its own files.
        assert!(selection.contains("txt"), "{}", preset.label());
        assert!(selection.contains("bpadx"), "{}", preset.label());
        for extension in selection.extensions() {
            assert!(everything.contains(extension), "{extension}");
        }
    }
}

#[test]
fn a_preset_round_trips_through_its_token_and_recognises_itself() {
    for &preset in AssociationPreset::ALL {
        assert_eq!(AssociationPreset::parse(preset.token()), Some(preset));
        assert_eq!(
            AssociationSelection::preset(preset).matching_preset(),
            Some(preset),
            "{}",
            preset.label()
        );
    }
    assert_eq!(AssociationPreset::parse("custom"), None);
}

#[test]
fn a_custom_selection_drops_what_the_editor_cannot_open() {
    let selection = AssociationSelection::custom(["txt", ".MD", "docx", "xlsx"]);
    assert_eq!(
        selection.extensions().collect::<Vec<_>>(),
        vec!["md", "txt"]
    );
    assert_eq!(
        unknown_extensions(&["txt", ".MD", "docx", "xlsx"]),
        vec!["docx", "xlsx"]
    );
    // A custom selection that happens to equal a preset is reported as that
    // preset rather than as "Custom".
    assert_eq!(selection.matching_preset(), None);
}

#[test]
fn the_mime_list_is_deduplicated() {
    // Six extensions map to text/plain; a MimeType line that repeats it six
    // times fails desktop-file-validate.
    let mimes = everything().mime_types();
    let mut sorted = mimes.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), mimes.len());
    assert!(mimes.contains(&"text/plain"));
}

// --- the .desktop entry ----------------------------------------------------

#[test]
fn the_desktop_entry_has_the_keys_a_desktop_environment_looks_for() {
    let entry = desktop::desktop_entry(&app(), &everything());
    assert!(entry.starts_with("[Desktop Entry]\n"));
    let keys: Vec<&str> = desktop_keys(&entry).into_iter().map(|(k, _)| k).collect();
    for required in ["Type", "Name", "Exec", "MimeType", "Icon", "Categories"] {
        assert!(keys.contains(&required), "no {required} key in\n{entry}");
    }
    assert!(entry.contains("Exec=/usr/local/bin/bachelorpad %F"));
    assert!(entry.contains("MimeType=application/x-bachelorpad-encrypted;"));
    assert!(entry.contains(";text/plain;"), "{entry}");
    assert!(entry.contains("Categories=Utility;TextEditor;\n"));
}

#[test]
fn an_empty_selection_still_produces_an_entry_but_claims_nothing() {
    let entry = desktop::desktop_entry(&app(), &AssociationSelection::default());
    assert!(!entry.contains("MimeType"));
    assert!(entry.contains("Exec="));
}

#[test]
fn a_path_with_a_space_is_quoted_in_exec_and_not_in_tryexec() {
    let mut app = app();
    app.executable = "/opt/BachelorPad Plus/bachelorpad".to_owned();
    let entry = desktop::desktop_entry(&app, &everything());
    assert!(
        entry.contains("Exec=\"/opt/BachelorPad Plus/bachelorpad\" %F"),
        "{entry}"
    );
    // TryExec is a plain path, not an Exec value, so it is not quoted.
    assert!(entry.contains("TryExec=/opt/BachelorPad Plus/bachelorpad"));
}

#[test]
fn a_windows_style_path_survives_both_layers_of_escaping() {
    // The classic defect: the Exec value is quoted by the Exec rules and then
    // escaped again as a desktop string, so one backslash becomes four. A
    // single-escaped path launches the wrong program.
    let quoted = desktop::quote_exec_argument(r"C:\Program Files\bp.exe");
    assert_eq!(quoted, "\"C:\\\\Program Files\\\\bp.exe\"");
    assert_eq!(
        desktop::escape_value(&quoted),
        r#""C:\\\\Program Files\\\\bp.exe""#
    );
}

#[test]
fn the_mime_package_defines_only_the_products_own_types() {
    let package = desktop::mime_package(&everything()).expect("bpadx is in every preset");
    assert!(package.contains("application/x-bachelorpad-encrypted"));
    assert!(package.contains("<glob pattern=\"*.bpadx\"/>"));
    // Not a redefinition of the distribution's own types.
    assert!(!package.contains("application/json"));
    assert!(!package.contains("text/plain"));
    // The three PowerShell extensions are one type with three globs, not
    // three definitions of the same type.
    assert_eq!(package.matches("text/x-powershell").count(), 1);
    assert_eq!(package.matches("<glob pattern=\"*.ps").count(), 3);
}

// --- the application id and the packaged assets ----------------------------

#[test]
fn the_application_id_is_reverse_dns_and_the_directory_name_did_not_move() {
    // ADR-0032: the id is an identity, the directory is a path, and moving
    // the second would move every user's settings for nothing.
    assert_eq!(crate::APP_ID, "io.github.dboles99.BachelorPad");
    assert_eq!(crate::APP_DIR, "bachelorpad");
    assert!(
        !crate::APP_ID.contains('+'),
        "a desktop-file id may not contain a plus"
    );
    let app = app();
    assert_eq!(app.app_id, crate::APP_ID);
    assert_eq!(
        app.icon,
        crate::APP_ID,
        "hicolor icons are named for the id"
    );
    assert_eq!(
        desktop::desktop_file_name(&app),
        "io.github.dboles99.BachelorPad.desktop"
    );
    assert_eq!(
        desktop::mime_package_file_name(&app),
        "io.github.dboles99.BachelorPad.xml"
    );
    assert_eq!(
        desktop::metainfo_file_name(&app),
        "io.github.dboles99.BachelorPad.metainfo.xml"
    );
}

#[test]
fn the_windows_registry_script_keeps_the_name_it_always_had() {
    // The id is a Linux identity; nothing a Windows user sees derives from it.
    let plan = plan(Platform::Windows, &windows_app(), &everything());
    assert_eq!(
        plan.artefact(ArtefactKind::RegistryScript)
            .map(|a| a.relative_path.as_str()),
        Some("bachelorpad-file-types.reg")
    );
    assert!(
        plan.registry_values
            .iter()
            .all(|v| !v.key.contains(crate::APP_ID) && !v.data.contains(crate::APP_ID)),
        "no registry key or value names the Linux application id"
    );
}

#[test]
fn the_packaged_entry_launches_the_program_on_path() {
    let entry = desktop::desktop_entry(
        &AppInfo::packaged(),
        &AssociationSelection::preset(desktop::PACKAGED_PRESET),
    );
    assert!(entry.contains("\nExec=bachelorpad %F\n"), "{entry}");
    assert!(entry.contains("\nTryExec=bachelorpad\n"), "{entry}");
    assert!(
        entry.contains("\nIcon=io.github.dboles99.BachelorPad\n"),
        "{entry}"
    );
}

#[test]
fn the_metainfo_provides_exactly_the_types_the_desktop_entry_claims() {
    // Two lists of the same MIME types kept by hand drift the first time a
    // file type is added; both come from the selection, and this holds them
    // to it.
    let app = AppInfo::packaged();
    let selection = AssociationSelection::preset(desktop::PACKAGED_PRESET);
    let entry = desktop::desktop_entry(&app, &selection);
    let claimed: Vec<&str> = entry
        .lines()
        .find_map(|line| line.strip_prefix("MimeType="))
        .expect("the packaged entry claims types")
        .split(';')
        .filter(|m| !m.is_empty())
        .collect();
    let metainfo = desktop::metainfo(&app, &selection);
    let provided: Vec<&str> = metainfo
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("<mediatype>")
                .and_then(|l| l.strip_suffix("</mediatype>"))
        })
        .collect();
    assert_eq!(claimed, provided);
    assert!(metainfo.contains("<id>io.github.dboles99.BachelorPad</id>"));
    assert!(metainfo.contains(
        "<launchable type=\"desktop-id\">io.github.dboles99.BachelorPad.desktop</launchable>"
    ));
    assert!(metainfo.contains("<content_rating type=\"oars-1.1\"/>"));
}

#[test]
fn the_newest_release_in_the_metainfo_is_the_workspace_version() {
    // A software centre shows the newest release as "what's new"; bumping
    // the version without adding a release here would advertise the old one.
    assert_eq!(
        desktop::RELEASES.first().map(|r| r.version),
        Some(env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn the_per_user_marker_reads_the_same_in_both_comment_syntaxes() {
    let marker = desktop::PER_USER_MARKER;
    assert_eq!(desktop::escape_xml(marker), marker);
    assert!(!marker.contains("--"), "an XML comment may not contain --");
    assert!(!marker.contains('\n'));

    let selection = AssociationSelection::preset(desktop::PACKAGED_PRESET);
    let entry = desktop::per_user_desktop_entry(&app(), &selection);
    let package = desktop::per_user_mime_package(&selection).expect("bpadx is ours");
    assert!(desktop::is_per_user_registration(&entry));
    assert!(desktop::is_per_user_registration(&package));
    assert!(
        package.starts_with("<?xml "),
        "the declaration must stay first"
    );
    // The packaged files carry no marker: a package owns its files and
    // Remove Registration must never take them.
    for asset in desktop::packaged_assets() {
        assert!(
            !desktop::is_per_user_registration(&asset.contents),
            "{}",
            asset.file_name
        );
    }
}

// --- the Windows registry shape --------------------------------------------

#[test]
fn registration_offers_the_extension_and_never_takes_it() {
    let values = windows::registry_values(&windows_app(), &everything());
    // The additive value: our ProgID appears as a *value name* under
    // OpenWithProgids.
    assert!(values.iter().any(|v| {
        v.key == r"HKCU\Software\Classes\.txt\OpenWithProgids"
            && v.name == ValueName::Named("BachelorPadPlus.txt".to_owned())
    }));
    // ...and nothing writes the extension key's own default value, which is
    // what taking the association looks like.
    assert!(
        !values
            .iter()
            .any(|v| v.key == r"HKCU\Software\Classes\.txt" && v.name == ValueName::Default),
        "the extension's default value is the association itself"
    );
    // ...nor UserChoice, which is the user's own record and is hash-protected.
    assert!(!values.iter().any(|v| v.key.contains("UserChoice")));
}

#[test]
fn no_generated_key_is_one_this_crate_refuses_to_write() {
    for &preset in AssociationPreset::ALL {
        let selection = AssociationSelection::preset(preset);
        let plan = plan(Platform::Windows, &windows_app(), &selection);
        assert_eq!(
            plan.registry_objections(),
            vec![],
            "{} generated a key the guard refuses",
            preset.label()
        );
    }
}

#[test]
fn the_guard_refuses_what_adr_0012_forbids() {
    use windows::{KeyObjection, key_objection};
    assert_eq!(
        key_objection(r"HKLM\Software\Classes\BachelorPadPlus.txt"),
        Some(KeyObjection::OutsideTheUserHive)
    );
    assert_eq!(
        key_objection(r"HKCU\Software\Classes\txtfile\shell\open\command"),
        Some(KeyObjection::AnotherApplication)
    );
    assert_eq!(
        key_objection(r"HKCU\Software\Classes\Applications\notepad.exe"),
        Some(KeyObjection::TouchesNotepad)
    );
    assert_eq!(
        key_objection(r"HKCU\Software\Classes\.txt\UserChoice"),
        Some(KeyObjection::SeizesTheAssociation)
    );
    assert_eq!(
        key_objection(r"HKCU\Software\Classes\.txt"),
        Some(KeyObjection::SeizesTheAssociation)
    );
    assert_eq!(
        key_objection(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run"),
        Some(KeyObjection::AnotherApplication)
    );
    // ...and permits exactly what registration needs.
    assert_eq!(
        key_objection(r"HKCU\Software\Classes\.txt\OpenWithProgids"),
        None
    );
    assert_eq!(
        key_objection(r"HKCU\Software\Classes\BachelorPadPlus.md"),
        None
    );
    assert_eq!(
        key_objection(r"HKCU\Software\BachelorPad+\Capabilities"),
        None
    );
    assert_eq!(key_objection(r"HKCU\Software\RegisteredApplications"), None);
}

#[test]
fn the_guard_is_case_insensitive_like_the_registry() {
    use windows::{KeyObjection, key_objection};
    assert_eq!(
        key_objection(r"hkcu\software\classes\.TXT\openwithprogids"),
        None
    );
    assert_eq!(
        key_objection(r"HKEY_CURRENT_USER\Software\Classes\NOTEPAD.exe"),
        Some(KeyObjection::TouchesNotepad)
    );
}

#[test]
fn the_registry_script_is_something_regedit_would_accept() {
    let plan = plan(Platform::Windows, &windows_app(), &everything());
    let script = plan
        .artefact(ArtefactKind::RegistryScript)
        .expect("Windows plans carry a .reg script");
    assert!(
        script
            .contents
            .starts_with("Windows Registry Editor Version 5.00\r\n")
    );
    assert!(
        script
            .contents
            .contains("[HKEY_CURRENT_USER\\Software\\Classes\\")
    );
    assert!(
        !script.contents.contains("[HKCU\\"),
        "the short hive name is not valid in a .reg file"
    );
    // Backslashes in the command doubled, quotes escaped.
    assert!(
        script
            .contents
            .contains(r#"@="\"C:\\Program Files\\BachelorPad+\\bachelorpad.exe\" \"%1\"""#),
        "{}",
        script.contents
    );
    // Every line ends CRLF.
    assert!(!script.contents.replace("\r\n", "").contains('\n'));
}

/// `bytes` decoded as the UTF-16LE-with-BOM file regedit reads, asserting
/// the BOM on the way.
fn decode_reg_file(bytes: &[u8]) -> String {
    assert_eq!(
        bytes.get(..2),
        Some(&[0xFF, 0xFE][..]),
        "a .reg file starts with the UTF-16LE byte-order mark"
    );
    let body = &bytes[2..];
    assert_eq!(body.len() % 2, 0, "UTF-16 is two bytes a unit");
    let units: Vec<u16> = body
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16(&units).expect("the file is valid UTF-16")
}

/// The `[...]` key lines of a `.reg` text, and the lines under each.
fn reg_sections(text: &str) -> Vec<(String, Vec<String>)> {
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();
    for line in text.split("\r\n") {
        if let Some(key) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            sections.push((key.to_owned(), Vec::new()));
        } else if !line.is_empty()
            && let Some((_, lines)) = sections.last_mut()
        {
            lines.push(line.to_owned());
        }
    }
    sections
}

#[test]
fn the_registry_file_is_utf16le_with_a_byte_order_mark_and_decodes_to_the_script() {
    let values = windows::registry_values(&windows_app(), &everything());
    let bytes = windows::registry_file(&values);
    // FF FE, then "Wi" as little-endian code units.
    assert_eq!(&bytes[..6], &[0xFF, 0xFE, b'W', 0x00, b'i', 0x00]);
    assert_eq!(decode_reg_file(&bytes), windows::registry_script(&values));
}

#[test]
fn a_non_ascii_executable_path_reaches_the_file_as_the_characters_it_is() {
    // The failure the encoding exists to prevent: an ANSI .reg file turns
    // `Zoë` into whatever byte 0xEB is in the reader's code page.
    let app = AppInfo::bachelorpad(r"C:\Users\Zoë\東京\bachelorpad.exe");
    let values = windows::registry_values(&app, &AssociationSelection::custom(["txt"]));
    let bytes = windows::registry_file(&values);

    // U+00EB and U+6771 as UTF-16LE code units.
    assert!(bytes.windows(2).any(|w| w == [0xEB, 0x00]), "ë is missing");
    assert!(bytes.windows(2).any(|w| w == [0x71, 0x67]), "東 is missing");
    let text = decode_reg_file(&bytes);
    assert!(
        text.contains(r#""\"C:\\Users\\Zoë\\東京\\bachelorpad.exe\",0""#),
        "{text}"
    );
}

#[test]
fn a_saved_artefact_is_written_in_the_encoding_its_format_is_read_in() {
    let windows = plan(Platform::Windows, &windows_app(), &everything());
    for kind in [ArtefactKind::RegistryScript, ArtefactKind::UnregisterScript] {
        let artefact = windows.artefact(kind).expect("both .reg files are planned");
        let bytes = artefact.bytes();
        assert_eq!(&bytes[..2], &[0xFF, 0xFE], "{kind:?}");
        assert_eq!(decode_reg_file(&bytes), artefact.contents, "{kind:?}");
    }
    // The Linux formats are defined as UTF-8 and stay exactly that.
    let linux = plan(Platform::Linux, &app(), &everything());
    for artefact in &linux.artefacts {
        assert_eq!(artefact.bytes(), artefact.contents.as_bytes());
    }
}

#[test]
fn the_capabilities_key_names_the_executable_as_the_application_icon() {
    let values = windows::registry_values(&windows_app(), &everything());
    let icon = values
        .iter()
        .find(|v| {
            v.key == windows::CAPABILITIES_KEY
                && v.name == ValueName::Named("ApplicationIcon".to_owned())
        })
        .expect("Capabilities carries an ApplicationIcon");
    assert_eq!(
        icon.data,
        r#""C:\Program Files\BachelorPad+\bachelorpad.exe",0"#
    );
}

#[test]
fn the_registered_application_is_the_display_name_pointing_at_the_capabilities() {
    // Microsoft: ApplicationName "must always match the name that is
    // registered under RegisteredApplications", and the Windows 11 deep link
    // looks the application up by that same name.
    let app = windows_app();
    let values = windows::registry_values(&app, &everything());
    let registered = values
        .iter()
        .find(|v| v.key == windows::REGISTERED_APPLICATIONS_KEY)
        .expect("one value under RegisteredApplications");
    assert_eq!(
        registered.name,
        ValueName::Named(windows::registered_application_name(&app).to_owned())
    );
    assert_eq!(registered.data, windows::CAPABILITIES_PATH);
    assert!(values.iter().any(|v| v.key == windows::CAPABILITIES_KEY
        && v.name == ValueName::Named("ApplicationName".to_owned())
        && v.data == windows::registered_application_name(&app)));
}

#[test]
fn the_unregister_file_is_utf16le_with_a_byte_order_mark_and_decodes_to_its_script() {
    let values = windows::registry_values(&windows_app(), &everything());
    let bytes = windows::unregister_file(&values);
    assert_eq!(&bytes[..6], &[0xFF, 0xFE, b'W', 0x00, b'i', 0x00]);
    let text = decode_reg_file(&bytes);
    assert_eq!(text, windows::unregister_script(&values));
    assert!(text.starts_with("Windows Registry Editor Version 5.00\r\n"));
    assert!(
        !text.replace("\r\n", "").contains('\n'),
        "every line ends CRLF"
    );
}

#[test]
fn unregistering_deletes_our_own_keys_whole_and_only_our_value_from_a_shared_key() {
    let values = windows::registry_values(&windows_app(), &everything());
    let text = windows::unregister_script(&values);
    let sections = reg_sections(&text);

    let deleted: Vec<&str> = sections
        .iter()
        .filter_map(|(key, _)| key.strip_prefix('-'))
        .collect();
    // One whole-key deletion per ProgID, plus the product's own key.
    assert_eq!(deleted.len(), everything().file_types().count() + 1);
    for key in &deleted {
        let ours = key.starts_with(r"HKEY_CURRENT_USER\Software\Classes\BachelorPadPlus.")
            || *key == r"HKEY_CURRENT_USER\Software\BachelorPad+";
        assert!(ours, "deletes a key that is not ours outright: {key}");
        assert_eq!(windows::key_objection(key), None, "{key}");
    }
    assert!(deleted.contains(&r"HKEY_CURRENT_USER\Software\Classes\BachelorPadPlus.txt"));

    // Shared keys are only ever opened, never deleted, and inside each the
    // one line removes the one value the register script added.
    for (key, lines) in sections.iter().filter(|(key, _)| !key.starts_with('-')) {
        assert!(
            key.ends_with(r"\OpenWithProgids") || key.ends_with(r"\RegisteredApplications"),
            "only shared keys are opened: {key}"
        );
        assert_eq!(windows::key_objection(key), None, "{key}");
        assert_eq!(lines.len(), 1, "{key}: {lines:?}");
        assert!(lines[0].ends_with("=-"), "{key}: {lines:?}");
    }
    let open_with = sections
        .iter()
        .find(|(key, _)| key == r"HKEY_CURRENT_USER\Software\Classes\.txt\OpenWithProgids")
        .expect(".txt's Open With list is visited");
    assert_eq!(open_with.1, vec![r#""BachelorPadPlus.txt"=-"#.to_owned()]);
    let registered = sections
        .iter()
        .find(|(key, _)| key == r"HKEY_CURRENT_USER\Software\RegisteredApplications")
        .expect("RegisteredApplications is visited");
    assert_eq!(registered.1, vec![r#""BachelorPad+"=-"#.to_owned()]);

    // No extension key, no UserChoice, and never Notepad.
    assert!(!text.contains(r"[-HKEY_CURRENT_USER\Software\Classes\."));
    assert!(!text.contains("UserChoice"));
    assert!(!text.to_ascii_lowercase().contains("notepad"));
}

#[test]
fn unregistering_undoes_every_value_the_register_script_adds() {
    for &preset in AssociationPreset::ALL {
        let values =
            windows::registry_values(&windows_app(), &AssociationSelection::preset(preset));
        let sections = reg_sections(&windows::unregister_script(&values));
        for value in &values {
            let long = value.key.replace(r"HKCU\", r"HKEY_CURRENT_USER\");
            let under_deleted_key = sections.iter().any(|(key, _)| {
                key.strip_prefix('-')
                    .is_some_and(|root| long == root || long.starts_with(&format!(r"{root}\")))
            });
            let value_removed = sections.iter().any(|(key, lines)| {
                *key == long
                    && lines.iter().any(|line| match &value.name {
                        ValueName::Default => line == "@=-",
                        ValueName::Named(name) => *line == format!("\"{name}\"=-"),
                    })
            });
            assert!(
                under_deleted_key || value_removed,
                "{}: {:?} under {} survives unregistering",
                preset.label(),
                value.name,
                value.key
            );
        }
    }
}

#[test]
fn a_value_in_a_key_that_is_not_ours_is_removed_by_name_and_its_key_is_kept() {
    // Never produced by `registry_values`; this is the fallback's promise:
    // the worst a surprising input can do is remove the value it names.
    let values = vec![RegistryValue {
        key: r"HKCU\Software\Classes\.txt\OpenWithProgids".to_owned(),
        name: ValueName::Named("SomebodyElse.txt".to_owned()),
        data: String::new(),
    }];
    let text = windows::unregister_script(&values);
    assert!(!text.contains("[-"), "{text}");
    assert!(text.contains("\"SomebodyElse.txt\"=-"), "{text}");
}

#[test]
fn a_line_break_in_a_value_cannot_start_a_new_key_in_either_script() {
    let mut app = windows_app();
    app.display_name = "x\r\n[-HKEY_LOCAL_MACHINE\\Software]\r\n".to_owned();
    app.description = "y\n[HKEY_CURRENT_USER\\Software\\Classes\\txtfile]".to_owned();
    let values = windows::registry_values(&app, &AssociationSelection::custom(["txt"]));
    for text in [
        windows::registry_script(&values),
        windows::unregister_script(&values),
    ] {
        for (key, _) in reg_sections(&text) {
            let key = key.trim_start_matches('-');
            assert!(
                values
                    .iter()
                    .any(|v| v.key.replace(r"HKCU\", r"HKEY_CURRENT_USER\") == key)
                    || key == r"HKEY_CURRENT_USER\Software\BachelorPad+"
                    || key.starts_with(r"HKEY_CURRENT_USER\Software\Classes\BachelorPadPlus."),
                "a value's text became a key line: [{key}]\n{text}"
            );
        }
    }
}

#[test]
fn windows_11_gets_the_products_own_settings_page_and_windows_10_the_generic_one() {
    let app = windows_app();
    let own = "ms-settings:defaultapps?registeredAppUser=BachelorPad%2B";
    assert_eq!(
        windows::default_apps_uri(None, &app),
        "ms-settings:defaultapps"
    );
    // Windows 10 22H2 and the last build before Windows 11.
    assert_eq!(
        windows::default_apps_uri(Some(19045), &app),
        "ms-settings:defaultapps"
    );
    assert_eq!(
        windows::default_apps_uri(Some(21999), &app),
        "ms-settings:defaultapps"
    );
    // Windows 11 21H2, 23H2, 24H2.
    assert_eq!(windows::default_apps_uri(Some(22000), &app), own);
    assert_eq!(windows::default_apps_uri(Some(22631), &app), own);
    assert_eq!(windows::default_apps_uri(Some(26100), &app), own);
}

#[test]
fn the_deep_link_escapes_the_registered_name_as_a_uri_component() {
    let mut app = windows_app();
    app.display_name = "Bachelor Pad+ & Zoë".to_owned();
    assert_eq!(
        windows::default_apps_uri(Some(22631), &app),
        "ms-settings:defaultapps?registeredAppUser=Bachelor%20Pad%2B%20%26%20Zo%C3%AB"
    );
}

#[test]
fn settings_is_offered_once_registered_and_the_file_until_then() {
    use windows::{RegistrationRoute, registration_route};
    assert_eq!(
        registration_route(Some(r"Software\BachelorPad+\Capabilities")),
        RegistrationRoute::OpenSettings
    );
    assert_eq!(
        registration_route(Some(r"software\bachelorpad+\capabilities")),
        RegistrationRoute::OpenSettings,
        "the registry is case-insensitive"
    );
    assert_eq!(
        registration_route(None),
        RegistrationRoute::SaveRegistrationFile
    );
    assert_eq!(
        registration_route(Some(r"Software\Elsewhere\Capabilities")),
        RegistrationRoute::SaveRegistrationFile,
        "a value Default apps cannot follow to our capabilities is not a registration"
    );
}

#[test]
fn the_users_recorded_choice_is_refused_wherever_it_lives() {
    use windows::{KeyObjection, key_objection};
    assert_eq!(
        key_objection(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.txt\UserChoice"
        ),
        Some(KeyObjection::SeizesTheAssociation)
    );
}

// --- the plan --------------------------------------------------------------

#[test]
fn each_platform_gets_the_artefacts_it_has_a_use_for() {
    let linux = plan(Platform::Linux, &app(), &everything());
    assert!(linux.artefact(ArtefactKind::DesktopEntry).is_some());
    assert!(linux.artefact(ArtefactKind::MimePackage).is_some());
    assert!(linux.artefact(ArtefactKind::RegistryScript).is_none());
    assert!(linux.artefact(ArtefactKind::UnregisterScript).is_none());
    assert!(linux.registry_values.is_empty());
    assert!(matches!(linux.handoff, Handoff::LinuxSetDefault { .. }));

    let windows = plan(Platform::Windows, &windows_app(), &everything());
    assert!(windows.artefact(ArtefactKind::RegistryScript).is_some());
    assert!(windows.artefact(ArtefactKind::UnregisterScript).is_some());
    assert!(windows.artefact(ArtefactKind::DesktopEntry).is_none());
    assert!(!windows.registry_values.is_empty());
    assert_eq!(
        windows.handoff,
        Handoff::WindowsDefaultApps {
            uri: "ms-settings:defaultapps"
        }
    );
}

#[test]
fn the_follow_up_commands_are_named_and_explained_never_run() {
    let plan = plan(Platform::Linux, &app(), &everything());
    let programs: Vec<&str> = plan.follow_up.iter().map(|f| f.program).collect();
    assert_eq!(
        programs,
        vec!["update-desktop-database", "update-mime-database"]
    );
    for follow_up in &plan.follow_up {
        assert!(follow_up.why.len() > 20, "{follow_up:?}");
        assert!(!follow_up.command_line().is_empty());
    }
}

#[test]
fn the_handoff_names_the_command_that_would_set_the_default_and_stops_there() {
    let plan = plan(
        Platform::Linux,
        &app(),
        &AssociationSelection::custom(["txt"]),
    );
    let Handoff::LinuxSetDefault { program, args } = &plan.handoff else {
        panic!("Linux hands off through xdg-mime");
    };
    assert_eq!(*program, "xdg-mime");
    assert_eq!(args[0], "default");
    assert_eq!(args[1], "io.github.dboles99.BachelorPad.desktop");
    assert!(args.contains(&"text/plain".to_owned()));
    assert!(plan.handoff.describe().contains("xdg-mime"));
}

#[test]
fn an_app_info_says_what_is_wrong_with_it_before_a_plan_is_built() {
    let mut app = app();
    app.executable = "bachelorpad".to_owned();
    let problems = app.problems(Platform::Linux);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("relative"));
    assert!(
        AppInfo::bachelorpad("/usr/bin/bachelorpad")
            .problems(Platform::Linux)
            .is_empty()
    );
    // The same absolute-path question has a different answer per platform,
    // which is the point of taking the platform.
    assert!(
        !AppInfo::bachelorpad("/usr/bin/bachelorpad")
            .problems(Platform::Windows)
            .is_empty()
    );
}

// --- reading the current state ---------------------------------------------

#[test]
fn a_mimeapps_list_is_read_the_way_the_desktop_reads_it() {
    let text = "\
# a comment
[Added Associations]
text/plain=someone-else.desktop;

[Default Applications]
text/plain = io.github.dboles99.BachelorPad.desktop;gedit.desktop;
text/markdown=marker.desktop
application/json=
";
    let report = state::report_from_mimeapps(text, &app(), &everything());
    assert_eq!(report.state_of("txt"), Some(&AssociationState::Ours));
    assert_eq!(
        report.state_of("md"),
        Some(&AssociationState::Other {
            handler: "marker.desktop".to_owned()
        })
    );
    // An empty value claims nothing, and `[Added Associations]` is not the
    // default -- it only means "can open", which is a different question.
    assert_eq!(report.state_of("json"), Some(&AssociationState::Unclaimed));
    assert!(report.is_ours(".TXT"));
    assert!(!report.is_ours("md"));
}

#[test]
fn a_missing_or_empty_mimeapps_list_means_unclaimed_not_broken() {
    let report = state::report_from_mimeapps("", &app(), &everything());
    assert!(
        report
            .entries()
            .iter()
            .all(|e| e.state == AssociationState::Unclaimed)
    );
    assert_eq!(report.ours(), 0);
}

#[test]
fn a_repeated_key_takes_the_first_like_the_desktop_entry_specification() {
    let text = "[Default Applications]\ntext/plain=first.desktop\ntext/plain=second.desktop\n";
    let defaults = state::default_applications(text);
    assert_eq!(defaults["text/plain"], vec!["first.desktop".to_owned()]);
}

#[test]
fn the_windows_state_is_read_from_prog_ids_supplied_by_the_caller() {
    let mut prog_ids = BTreeMap::new();
    prog_ids.insert(".txt".to_owned(), "BachelorPadPlus.txt".to_owned());
    prog_ids.insert("MD".to_owned(), "Notepad++_file".to_owned());
    let report = state::report_from_prog_ids(&prog_ids, &everything());
    assert_eq!(report.state_of("txt"), Some(&AssociationState::Ours));
    assert_eq!(
        report.state_of("md"),
        Some(&AssociationState::Other {
            handler: "Notepad++_file".to_owned()
        })
    );
    assert_eq!(report.state_of("json"), Some(&AssociationState::Unclaimed));
}

#[test]
fn a_query_is_asked_once_per_type_and_its_answers_mean_what_the_map_s_do() {
    // The Windows path: `bp-platform-windows` reads the registry, and this
    // crate decides what the answers mean. A query that records what it was
    // asked stands in for the registry.
    struct Recording(std::cell::RefCell<Vec<String>>);
    impl ProgIdQuery for Recording {
        fn current_prog_id(&self, extension: &str) -> Option<String> {
            self.0.borrow_mut().push(extension.to_owned());
            match extension {
                "txt" => Some("BachelorPadPlus.txt".to_owned()),
                "md" => Some("txtfile".to_owned()),
                _ => None,
            }
        }
    }
    let query = Recording(std::cell::RefCell::new(Vec::new()));
    let selection = everything();
    let report = state::report_from_query(&query, &selection);

    assert_eq!(report.state_of("txt"), Some(&AssociationState::Ours));
    assert_eq!(
        report.state_of("md"),
        Some(&AssociationState::Other {
            handler: "txtfile".to_owned()
        })
    );
    assert_eq!(report.state_of("json"), Some(&AssociationState::Unclaimed));
    let asked: Vec<String> = selection
        .file_types()
        .map(|t| t.extension.to_owned())
        .collect();
    assert_eq!(*query.0.borrow(), asked, "each type asked once, in order");
}

#[test]
fn a_map_answers_a_query_the_same_way_it_answers_the_map_report() {
    let mut prog_ids = BTreeMap::new();
    prog_ids.insert(".TXT".to_owned(), "BachelorPadPlus.txt".to_owned());
    prog_ids.insert("md".to_owned(), "Other.md".to_owned());
    assert_eq!(
        state::report_from_query(&prog_ids, &everything()),
        state::report_from_prog_ids(&prog_ids, &everything())
    );
}

#[test]
fn windows_reports_cannot_tell_rather_than_not_registered() {
    // The distinction a user acts on: "unclaimed" invites them to fix
    // something, "cannot tell" tells them the truth.
    let report = association_report(Platform::Windows, None, &app(), &everything());
    for entry in report.entries() {
        let AssociationState::Unknown { reason } = &entry.state else {
            panic!("{entry:?} claimed to know something Windows cannot tell us");
        };
        assert!(reason.contains("registry"), "{reason}");
    }
    assert!(report.summary().starts_with("Cannot tell"));
}

#[test]
fn the_report_is_read_from_a_directory_the_caller_names() {
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("mimeapps.list"),
        "[Default Applications]\ntext/plain=io.github.dboles99.BachelorPad.desktop;\n",
    )
    .unwrap();
    let selection = AssociationSelection::custom(["txt", "md"]);
    let report = association_report(Platform::Linux, Some(dir.path()), &app(), &selection);
    assert!(report.is_ours("txt"));
    assert!(!report.is_ours("md"));
    assert_eq!(
        report.summary(),
        "BachelorPad+ opens 1 of the 2 selected file types."
    );
}

#[test]
fn a_directory_with_no_mimeapps_list_is_not_an_error() {
    let dir = tempdir().unwrap();
    let report = association_report(
        Platform::Linux,
        Some(dir.path()),
        &app(),
        &AssociationSelection::custom(["txt"]),
    );
    assert_eq!(report.state_of("txt"), Some(&AssociationState::Unclaimed));
}

// --- installing, the one thing that changes anything -----------------------

#[test]
fn nothing_is_written_without_consent() {
    let dir = tempdir().unwrap();
    let plan = plan(Platform::Linux, &app(), &everything());
    assert_eq!(
        install(&plan, dir.path(), Consent::Withheld),
        Err(InstallRefusal::ConsentWithheld)
    );
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        0,
        "a refused install wrote something"
    );
}

#[test]
fn consent_is_checked_before_anything_else_can_refuse_first() {
    // Otherwise a refused install reports "nothing selected" and the fact that
    // nobody agreed to it never reaches the log.
    let dir = tempdir().unwrap();
    let empty = plan(Platform::Linux, &app(), &AssociationSelection::default());
    assert_eq!(
        install(&empty, dir.path(), Consent::Withheld),
        Err(InstallRefusal::ConsentWithheld)
    );
    assert_eq!(
        install(&empty, dir.path(), Consent::Granted),
        Err(InstallRefusal::NothingSelected)
    );
}

#[test]
fn a_consented_install_writes_exactly_the_plans_artefacts_under_the_given_root() {
    let dir = tempdir().unwrap();
    let plan = plan(Platform::Linux, &app(), &everything());
    let installed = install(&plan, dir.path(), Consent::Granted).expect("Linux can install");

    assert_eq!(installed.written.len(), plan.artefacts.len());
    for path in &installed.written {
        assert!(path.starts_with(dir.path()), "{path:?} escaped the root");
        assert!(path.is_file());
    }
    let entry = std::fs::read_to_string(
        dir.path()
            .join("applications/io.github.dboles99.BachelorPad.desktop"),
    )
    .expect("the desktop entry landed where mimeapps.list will look for it");
    assert_eq!(entry, plan.artefacts[0].contents);
    assert!(
        dir.path()
            .join("mime/packages/io.github.dboles99.BachelorPad.xml")
            .is_file()
    );
    assert_eq!(installed.follow_up, plan.follow_up);
}

#[test]
fn installing_sets_no_default_and_the_report_still_says_so() {
    // The line ADR-0012 draws: after a successful install the product is in
    // Open With, and nothing about what opens `.txt` has changed.
    let data = tempdir().unwrap();
    let config = tempdir().unwrap();
    let plan = plan(Platform::Linux, &app(), &everything());
    install(&plan, data.path(), Consent::Granted).unwrap();

    let report = association_report(Platform::Linux, Some(config.path()), &app(), &everything());
    assert_eq!(report.ours(), 0, "installing took an association");
}

#[test]
fn windows_refuses_to_install_and_says_the_same_thing_the_capability_says() {
    let dir = tempdir().unwrap();
    let plan = plan(Platform::Windows, &windows_app(), &everything());
    let refusal = install(&plan, dir.path(), Consent::Granted).expect_err("Windows cannot install");
    let InstallRefusal::NotSupported { platform, reason } = refusal else {
        panic!("Windows refused for the wrong reason: {refusal:?}");
    };
    assert_eq!(platform, Platform::Windows);
    assert_eq!(
        Some(reason),
        Capability::AssociationInstall
            .availability(Platform::Windows)
            .reason(),
        "the refusal and the capability register must not drift apart"
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn an_artefact_path_that_could_escape_the_root_is_refused_before_anything_is_written() {
    let dir = tempdir().unwrap();
    for bad in [
        "../elsewhere/bachelorpad.desktop",
        "/etc/bachelorpad.desktop",
        r"C:\Windows\bachelorpad.desktop",
        r"..\up.desktop",
        "  ",
    ] {
        let mut plan = plan(Platform::Linux, &app(), &everything());
        plan.artefacts[0].relative_path = bad.to_owned();
        let refusal = install(&plan, dir.path(), Consent::Granted).expect_err(bad);
        assert!(
            matches!(refusal, InstallRefusal::UnsafeArtefactPath { .. }),
            "{bad} produced {refusal:?}"
        );
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            0,
            "{bad}: a bad path in one artefact wrote another"
        );
    }
}

#[test]
fn install_root_is_the_shared_data_hierarchy_on_linux_and_nothing_on_windows() {
    let env = EnvSnapshot {
        home: Some("/home/me".into()),
        ..EnvSnapshot::default()
    };
    assert_eq!(
        install_root(Platform::Linux, &env),
        Some(PathBuf::from("/home/me/.local/share"))
    );
    assert_eq!(
        install_root(Platform::Windows, &EnvSnapshot::default()),
        None
    );
}

#[test]
fn every_refusal_can_be_explained() {
    let refusals = [
        InstallRefusal::ConsentWithheld,
        InstallRefusal::NothingSelected,
        InstallRefusal::NotSupported {
            platform: Platform::Windows,
            reason: "There is no registry writer here.",
        },
        InstallRefusal::UnsafeArtefactPath {
            relative_path: "../x".to_owned(),
        },
        InstallRefusal::Failed {
            path: PathBuf::from("/x"),
            message: "permission denied".to_owned(),
        },
    ];
    for refusal in &refusals {
        assert!(refusal.describe().len() > 20, "{refusal:?}");
    }
}

// --- properties ------------------------------------------------------------

fn any_selection() -> impl Strategy<Value = AssociationSelection> {
    prop_oneof![
        prop::sample::select(AssociationPreset::ALL).prop_map(AssociationSelection::preset),
        prop::collection::vec(
            prop::sample::select(FILE_TYPES.iter().map(|t| t.extension).collect::<Vec<_>>()),
            0..8,
        )
        .prop_map(AssociationSelection::custom),
    ]
}

/// Strings a hostile or merely careless caller might put in an `AppInfo`.
fn awkward_text() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            prop::char::range('a', 'z'),
            Just(' '),
            Just('='),
            Just(';'),
            Just('\n'),
            Just('\r'),
            Just('\t'),
            Just('\\'),
            Just('"'),
            Just('<'),
            Just('&'),
            Just('$'),
        ],
        0..40,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// Whatever the shell puts in an `AppInfo`, the generated `.desktop` file
    /// is still a sequence of `key=value` lines under one group header. A
    /// display name containing a newline must not become an `Exec` line, which
    /// is the injection this format invites: there is no quoting at the line
    /// level, so a bad value produces a *valid* file that does something else.
    #[test]
    fn no_app_info_can_add_a_key_to_the_desktop_entry(
        display_name in awkward_text(),
        description in awkward_text(),
        executable in awkward_text(),
    ) {
        let mut app = app();
        app.display_name = display_name;
        app.description = description;
        app.executable = executable;
        let entry = desktop::desktop_entry(&app, &everything());

        let expected: Vec<&str> = vec![
            "Type", "Version", "Name", "GenericName", "Comment", "Exec", "TryExec", "Icon",
            "Terminal", "StartupNotify", "Categories", "Keywords", "MimeType",
        ];
        let keys: Vec<&str> = desktop_keys(&entry).into_iter().map(|(k, _)| k).collect();
        prop_assert_eq!(keys, expected, "the entry grew or lost a key:\n{}", entry);
        prop_assert_eq!(entry.matches("[Desktop Entry]").count(), 1);
    }

    /// Whatever the selection, every registry key a plan generates is one the
    /// ADR-0012 guard permits. The guard is only worth having if the generator
    /// cannot get around it, and this is the statement of that.
    #[test]
    fn no_selection_produces_a_key_the_guard_refuses(selection in any_selection()) {
        let plan = plan(Platform::Windows, &windows_app(), &selection);
        prop_assert_eq!(plan.registry_objections(), vec![]);
        // ...and none of them so much as mentions Notepad.
        for value in &plan.registry_values {
            prop_assert!(!value.key.to_ascii_lowercase().contains("notepad"));
            prop_assert!(!value.data.to_ascii_lowercase().contains("notepad"));
        }
    }

    /// Whatever the application is called and wherever it lives, both `.reg`
    /// files decode back to exactly their text, and the unregister script
    /// never deletes a key the guard would not let the register script write.
    #[test]
    fn both_reg_files_round_trip_and_unregistering_stays_inside_the_guard(
        display_name in awkward_text(),
        executable in awkward_text(),
        selection in any_selection(),
    ) {
        let mut app = windows_app();
        app.display_name = display_name;
        app.executable = executable;
        let values = windows::registry_values(&app, &selection);
        prop_assert_eq!(
            decode_reg_file(&windows::registry_file(&values)),
            windows::registry_script(&values)
        );
        let unregister = windows::unregister_script(&values);
        prop_assert_eq!(decode_reg_file(&windows::unregister_file(&values)), unregister.clone());
        for (key, _) in reg_sections(&unregister) {
            if let Some(root) = key.strip_prefix('-') {
                prop_assert_eq!(windows::key_objection(root), None, "{}", root);
            }
        }
    }

    /// Whatever the selection, an install writes only inside the root it was
    /// given, and writes one file per artefact.
    #[test]
    fn an_install_never_leaves_the_root_it_was_given(
        selection in any_selection(),
        prefix in prop::sample::select(vec![
            "", "../", "..\\", "/", r"C:\", "./", "a/../../",
        ]),
    ) {
        prop_assume!(!selection.is_empty());
        let dir = tempdir().unwrap();
        let mut plan = plan(Platform::Linux, &app(), &selection);
        // A plan is a value, and values arrive from elsewhere. Whatever is
        // stuck on the front of an artefact's path, an install either refuses
        // outright or writes inside the root it was given -- never both, and
        // never outside.
        for artefact in &mut plan.artefacts {
            artefact.relative_path = format!("{prefix}{}", artefact.relative_path);
        }
        match install(&plan, dir.path(), Consent::Granted) {
            Err(refusal) => prop_assert!(
                matches!(refusal, InstallRefusal::UnsafeArtefactPath { .. }),
                "{:?}",
                refusal,
            ),
            Ok(installed) => {
                prop_assert_eq!(installed.written.len(), plan.artefacts.len());
                for path in &installed.written {
                    prop_assert!(path.starts_with(dir.path()), "{:?}", path);
                    prop_assert!(!path.to_string_lossy().contains(".."), "{:?}", path);
                }
            }
        }
    }

    /// A plan is deterministic. Two runs over the same inputs give
    /// byte-identical artefacts, so a registration file does not appear in
    /// every backup diff for having reordered itself.
    #[test]
    fn a_plan_is_the_same_plan_twice(
        selection in any_selection(),
        platform in prop::sample::select(Platform::ALL),
    ) {
        let first = plan(platform, &windows_app(), &selection);
        let second = plan(platform, &windows_app(), &selection);
        prop_assert_eq!(first, second);
    }

    /// Whatever is selected, the product claims no file type it cannot open.
    /// Stated over selections rather than over the table, because `custom`
    /// takes arbitrary strings and dropping the unknown ones is what keeps
    /// this true.
    #[test]
    fn a_plan_never_claims_a_type_the_editor_cannot_open(
        proposed in prop::collection::vec("[a-z]{1,6}", 0..10),
    ) {
        let refs: Vec<&str> = proposed.iter().map(String::as_str).collect();
        let selection = AssociationSelection::custom(refs);
        for file_type in selection.file_types().filter(|t| must_be_parseable(t)) {
            prop_assert!(
                bp_formats::Format::from_extension(file_type.extension).is_some(),
                "{}",
                file_type.extension,
            );
        }
    }
}
