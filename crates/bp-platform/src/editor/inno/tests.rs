//! Tests for the installer generator.
//!
//! Everything here runs on Linux, which has no Inno Setup: the point of
//! generating the `[Registry]` section as text is that its shape, its
//! escaping and its uninstall flags can all be checked on a machine that
//! could never run the installer.

use super::*;
use crate::editor::AssociationPreset;

fn installed_values(preset: AssociationPreset) -> Vec<RegistryValue> {
    windows::registry_values(
        &AppInfo::bachelorpad(INSTALLED_EXECUTABLE),
        &AssociationSelection::preset(preset),
    )
}

fn value(key: &str, name: ValueName, data: &str) -> RegistryValue {
    RegistryValue {
        key: key.to_owned(),
        name,
        data: data.to_owned(),
    }
}

fn named(name: &str) -> ValueName {
    ValueName::Named(name.to_owned())
}

// --- flags per kind of key --------------------------------------------------

#[test]
fn every_value_under_a_prog_id_is_removed_with_its_whole_key() {
    let entries = registry_entries(&installed_values(PACKAGED_PRESET)).unwrap();
    let prog_ids: Vec<_> = entries
        .iter()
        .filter(|e| e.subkey.starts_with(r"Software\Classes\BachelorPadPlus."))
        .collect();
    assert!(!prog_ids.is_empty());
    for entry in prog_ids {
        assert_eq!(entry.uninstall, UninstallAction::DeleteKey, "{entry:?}");
    }
}

#[test]
fn values_added_to_shared_keys_remove_only_themselves() {
    let entries = registry_entries(&installed_values(PACKAGED_PRESET)).unwrap();
    let shared: Vec<_> = entries
        .iter()
        .filter(|e| {
            e.subkey.ends_with(r"\OpenWithProgids")
                || e.subkey == r"Software\RegisteredApplications"
        })
        .collect();
    // One OpenWithProgids value per type, and the RegisteredApplications one.
    assert_eq!(
        shared.len(),
        AssociationSelection::preset(PACKAGED_PRESET).len() + 1
    );
    for entry in shared {
        assert_eq!(entry.uninstall, UninstallAction::DeleteValue, "{entry:?}");
    }
}

#[test]
fn the_products_own_software_key_is_removed_whole_even_though_it_holds_no_value() {
    let entries = registry_entries(&installed_values(PACKAGED_PRESET)).unwrap();
    let root: Vec<_> = entries
        .iter()
        .filter(|e| e.subkey == r"Software\BachelorPad+")
        .collect();
    assert_eq!(root.len(), 1);
    assert_eq!(root[0].value, EntryValue::KeyOnly);
    assert_eq!(root[0].uninstall, UninstallAction::DeleteKey);
    for capability in entries
        .iter()
        .filter(|e| e.subkey.starts_with(r"Software\BachelorPad+\"))
    {
        assert_eq!(capability.uninstall, UninstallAction::DeleteKey);
    }
}

#[test]
fn no_key_only_entry_is_added_for_a_prog_id_that_already_has_values() {
    let entries = registry_entries(&installed_values(PACKAGED_PRESET)).unwrap();
    let key_only: Vec<_> = entries
        .iter()
        .filter(|e| e.value == EntryValue::KeyOnly)
        .map(|e| e.subkey.as_str())
        .collect();
    assert_eq!(key_only, [r"Software\BachelorPad+"]);
}

#[test]
fn a_value_in_a_key_neither_owned_nor_known_is_treated_as_shared() {
    // The same fallback the unregister script uses: the worst a surprising
    // input can do is remove the one value it named.
    let entries = registry_entries(&[value(
        r"HKCU\Software\Classes\.txt\OpenWithProgids",
        named("BachelorPadPlus.txt"),
        "",
    )])
    .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].uninstall, UninstallAction::DeleteValue);
}

#[test]
fn every_generated_value_becomes_exactly_one_entry_for_every_preset() {
    for &preset in AssociationPreset::ALL {
        let values = installed_values(preset);
        let entries = registry_entries(&values).unwrap();
        let written: Vec<_> = entries
            .iter()
            .filter(|e| e.value != EntryValue::KeyOnly)
            .collect();
        assert_eq!(written.len(), values.len(), "{preset:?}");
        for (entry, value) in written.iter().zip(&values) {
            assert_eq!(format!(r"HKCU\{}", entry.subkey), value.key);
            assert_eq!(
                entry.value,
                EntryValue::String {
                    name: value.name.clone(),
                    data: value.data.clone()
                }
            );
        }
    }
}

// --- the guard ----------------------------------------------------------------

#[test]
fn a_value_the_guard_refuses_fails_the_whole_section() {
    let mut values = installed_values(PACKAGED_PRESET);
    values.push(value(
        r"HKCU\Software\Classes\txtfile",
        ValueName::Default,
        "x",
    ));
    let error = registry_entries(&values).unwrap_err();
    assert_eq!(
        error,
        GeneratorError::Objection {
            key: r"HKCU\Software\Classes\txtfile".to_owned(),
            why: KeyObjection::AnotherApplication,
        }
    );
}

#[test]
fn notepad_user_choice_and_the_extension_default_are_all_refused() {
    let refused = [
        (
            r"HKCU\Software\Classes\Applications\notepad.exe",
            KeyObjection::TouchesNotepad,
        ),
        (
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.txt\UserChoice",
            KeyObjection::SeizesTheAssociation,
        ),
        (
            r"HKCU\Software\Classes\.txt",
            KeyObjection::SeizesTheAssociation,
        ),
        (
            r"HKLM\Software\Classes\BachelorPadPlus.txt",
            KeyObjection::OutsideTheUserHive,
        ),
    ];
    for (key, why) in refused {
        let error = registry_entries(&[value(key, ValueName::Default, "x")]).unwrap_err();
        assert_eq!(
            error,
            GeneratorError::Objection {
                key: key.to_owned(),
                why
            }
        );
    }
}

#[test]
fn the_packaged_block_never_names_notepad_or_user_choice() {
    let block = packaged_registry_block().unwrap().to_ascii_lowercase();
    assert!(!block.contains("notepad"));
    assert!(!block.contains("userchoice"));
    assert!(!block.contains("hklm"));
    assert!(!block.contains("hkey_local_machine"));
}

// --- rendering and escaping -----------------------------------------------

#[test]
fn the_open_command_quotes_the_installed_executable_and_keeps_app_as_a_constant() {
    let block = packaged_registry_block().unwrap();
    assert!(
        block.contains(
            r#"Root: HKCU; Subkey: "Software\Classes\BachelorPadPlus.txt\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\bachelorpad.exe"" ""%1"""; Flags: uninsdeletekey"#
        ),
        "{block}"
    );
    assert!(!block.contains("{{app}"));
}

#[test]
fn the_default_icon_names_the_installed_executable() {
    let block = packaged_registry_block().unwrap();
    assert!(block.contains(
        r#"Subkey: "Software\Classes\BachelorPadPlus.txt\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\bachelorpad.exe"",0"; Flags: uninsdeletekey"#
    ));
}

#[test]
fn the_open_with_value_is_named_for_the_prog_id_and_left_empty() {
    let block = packaged_registry_block().unwrap();
    assert!(block.contains(
        r#"Root: HKCU; Subkey: "Software\Classes\.txt\OpenWithProgids"; ValueType: string; ValueName: "BachelorPadPlus.txt"; ValueData: ""; Flags: uninsdeletevalue"#
    ));
}

#[test]
fn the_registered_application_points_at_the_capabilities_path() {
    let block = packaged_registry_block().unwrap();
    assert!(block.contains(&format!(
        r#"Root: HKCU; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "BachelorPad+"; ValueData: "{}"; Flags: uninsdeletevalue"#,
        windows::CAPABILITIES_PATH
    )));
}

#[test]
fn the_products_software_key_is_written_as_a_key_only_entry() {
    let block = packaged_registry_block().unwrap();
    assert!(block.contains(
        "Root: HKCU; Subkey: \"Software\\BachelorPad+\"; ValueType: none; Flags: uninsdeletekey\n"
    ));
}

#[test]
fn a_literal_brace_anywhere_else_is_doubled_so_inno_does_not_read_a_constant() {
    let entries = registry_entries(&[value(
        r"HKCU\Software\BachelorPad+\Capabilities",
        named("{name}"),
        r"{app} is not the constant here, but {app}\bachelorpad.exe is",
    )])
    .unwrap();
    let text = render_registry_entries(&entries, INSTALLED_EXECUTABLE).unwrap();
    assert!(text.contains(r#"ValueName: "{{name}""#), "{text}");
    assert!(
        text.contains(
            r#"ValueData: "{{app} is not the constant here, but {app}\bachelorpad.exe is""#
        ),
        "{text}"
    );
}

#[test]
fn a_brace_in_a_subkey_is_doubled_too() {
    let entries =
        registry_entries(&[value(r"HKCU\Software\BachelorPad+\{odd}", named("x"), "")]).unwrap();
    let text = render_registry_entries(&entries, INSTALLED_EXECUTABLE).unwrap();
    assert!(
        text.contains(r#"Subkey: "Software\BachelorPad+\{{odd}""#),
        "{text}"
    );
}

#[test]
fn a_double_quote_is_doubled_and_cannot_end_the_parameter_early() {
    let entries = registry_entries(&[value(
        r"HKCU\Software\BachelorPad+\Capabilities",
        named(r#"say "hi""#),
        r#"a "quoted"; Flags: uninsneveruninstall"#,
    )])
    .unwrap();
    let text = render_registry_entries(&entries, INSTALLED_EXECUTABLE).unwrap();
    assert!(text.contains(r#"ValueName: "say ""hi""""#), "{text}");
    assert!(
        text.contains(
            r#"ValueData: "a ""quoted""; Flags: uninsneveruninstall"; Flags: uninsdeletekey"#
        ),
        "{text}"
    );
}

#[test]
fn a_line_break_in_a_value_becomes_a_space_rather_than_a_new_entry() {
    let entries = registry_entries(&[value(
        r"HKCU\Software\BachelorPad+\Capabilities",
        named("x"),
        "one\r\nRoot: HKLM; Subkey: \"Software\"",
    )])
    .unwrap();
    let text = render_registry_entries(&entries, INSTALLED_EXECUTABLE).unwrap();
    assert_eq!(text.lines().count(), 2, "{text}");
    assert!(!text.lines().any(|line| line.starts_with("Root: HKLM")));
}

#[test]
fn an_empty_executable_escapes_everything() {
    let entries = registry_entries(&[value(
        r"HKCU\Software\BachelorPad+\Capabilities",
        named("x"),
        "{app}",
    )])
    .unwrap();
    let text = render_registry_entries(&entries, "").unwrap();
    assert!(text.contains(r#"ValueData: "{{app}""#), "{text}");
}

#[test]
fn text_outside_ascii_is_refused_rather_than_left_to_the_code_page() {
    let entries = registry_entries(&[value(
        r"HKCU\Software\BachelorPad+\Capabilities",
        named("ApplicationName"),
        "Zoë",
    )])
    .unwrap();
    assert!(matches!(
        render_registry_entries(&entries, INSTALLED_EXECUTABLE),
        Err(GeneratorError::NotAscii(_))
    ));
}

#[test]
fn every_line_of_the_packaged_block_has_the_same_field_order() {
    // Test-Install.ps1 reads the block back with one pattern.
    let block = packaged_registry_block().unwrap();
    assert!(block.ends_with('\n'));
    for line in block.lines() {
        assert!(line.starts_with("Root: HKCU; Subkey: \""), "{line}");
        assert!(
            line.contains("; ValueType: string; ValueName: \"")
                || line.contains("; ValueType: none;"),
            "{line}"
        );
        assert!(
            line.ends_with("; Flags: uninsdeletekey")
                || line.ends_with("; Flags: uninsdeletevalue"),
            "{line}"
        );
    }
}

// --- the user's directories -------------------------------------------------

#[test]
fn the_user_directories_are_the_ones_bp_platform_resolves_on_windows() {
    let directories = user_directories().unwrap();
    let path = |kind| {
        directories
            .iter()
            .find(|d| d.kind == kind)
            .map(|d| d.path.clone())
            .unwrap()
    };
    assert_eq!(path(DirKind::Config), r"{userappdata}\bachelorpad");
    assert_eq!(path(DirKind::Data), r"{localappdata}\bachelorpad\data");
    assert_eq!(path(DirKind::Cache), r"{localappdata}\bachelorpad\cache");
    assert_eq!(path(DirKind::State), r"{localappdata}\bachelorpad\state");
}

#[test]
fn only_the_roaming_directory_is_rooted_at_userappdata() {
    for directory in user_directories().unwrap() {
        assert_eq!(
            directory.path.starts_with("{userappdata}"),
            directory.kind.roams(Platform::Windows),
            "{directory:?}"
        );
    }
}

#[test]
fn the_user_directories_block_is_pascal_constants() {
    assert_eq!(
        user_directories_block().unwrap(),
        "const\n\
         \x20 ConfigDirectory = '{userappdata}\\bachelorpad';\n\
         \x20 DataDirectory = '{localappdata}\\bachelorpad\\data';\n\
         \x20 CacheDirectory = '{localappdata}\\bachelorpad\\cache';\n\
         \x20 StateDirectory = '{localappdata}\\bachelorpad\\state';\n\
         \x20 ProductLocalDirectory = '{localappdata}\\bachelorpad';\n"
    );
}

#[test]
fn a_single_quote_in_a_pascal_string_is_doubled() {
    assert_eq!(pascal_string("it's"), "'it''s'");
    assert_eq!(pascal_string("a\nb"), "'a b'");
}

// --- markers and splicing ---------------------------------------------------

fn sample_block() -> GeneratedBlock {
    block("sample", ";", "new line\n".to_owned())
}

#[test]
fn splicing_replaces_only_the_text_between_the_markers() {
    let b = sample_block();
    let script = format!("before\n{}\nold one\nold two\n{}\nafter\n", b.begin, b.end);
    let spliced = splice(&script, &b).unwrap();
    assert_eq!(
        spliced,
        format!("before\n{}\nnew line\n{}\nafter\n", b.begin, b.end)
    );
    assert_eq!(generated_text(&spliced, &b).unwrap(), "new line\n");
}

#[test]
fn splicing_twice_changes_nothing_the_second_time() {
    let b = sample_block();
    let script = format!("{}\n{}\n", b.begin, b.end);
    let once = splice(&script, &b).unwrap();
    assert_eq!(splice(&once, &b).unwrap(), once);
}

#[test]
fn crlf_line_endings_around_the_markers_are_kept() {
    let b = sample_block();
    let script = format!("x\r\n{}\r\nold\r\n{}\r\ny\r\n", b.begin, b.end);
    assert_eq!(
        splice(&script, &b).unwrap(),
        format!("x\r\n{}\r\nnew line\n{}\r\ny\r\n", b.begin, b.end)
    );
}

#[test]
fn a_missing_marker_is_an_error_not_an_append() {
    let b = sample_block();
    assert_eq!(
        splice(&format!("{}\n", b.begin), &b),
        Err(GeneratorError::MissingMarker(b.end.clone()))
    );
    assert_eq!(
        splice(&format!("{}\n", b.end), &b),
        Err(GeneratorError::MissingMarker(b.begin.clone()))
    );
}

#[test]
fn a_duplicated_marker_is_an_error() {
    let b = sample_block();
    let script = format!("{0}\n{0}\n{1}\n", b.begin, b.end);
    assert_eq!(
        splice(&script, &b),
        Err(GeneratorError::DuplicateMarker(b.begin.clone()))
    );
}

#[test]
fn markers_in_the_wrong_order_are_an_error() {
    let b = sample_block();
    let script = format!("{}\n{}\n", b.end, b.begin);
    assert_eq!(
        splice(&script, &b),
        Err(GeneratorError::MarkersOutOfOrder("sample".to_owned()))
    );
}

#[test]
fn a_marker_quoted_inside_a_longer_line_is_not_the_marker() {
    let b = sample_block();
    let script = format!("; see {} below\n{}\nold\n{}\n", b.begin, b.begin, b.end);
    assert_eq!(generated_text(&script, &b).unwrap(), "old\n");
}

#[test]
fn the_markers_name_the_command_that_regenerates_them() {
    for b in packaged_blocks().unwrap() {
        assert!(b.begin.ends_with(REGENERATE), "{}", b.begin);
        assert!(b.end.contains(b.name));
    }
}

#[test]
fn registry_markers_are_inno_comments_and_code_markers_are_pascal_comments() {
    let blocks = packaged_blocks().unwrap();
    assert!(blocks[0].begin.starts_with("; ") && blocks[0].end.starts_with("; "));
    assert!(blocks[1].begin.starts_with("// ") && blocks[1].end.starts_with("// "));
}
