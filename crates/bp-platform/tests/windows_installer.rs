//! The generated blocks of `packaging/windows/bachelorpad.iss` are exactly
//! what `bp-platform` generates.
//!
//! The installer writes the registration and removes it again (ADR-0032).
//! If its `[Registry]` section and `registry_values` disagree, the
//! association report, Remove Registration and the ADR-0012 guard all reason
//! about values the installer does not write. So the comparison is byte for
//! byte, and the fix is always to regenerate, never to edit the block by hand.

use std::path::{Path, PathBuf};

use bp_platform::editor::inno::{self, INSTALLER_SCRIPT, REGENERATE};

fn script_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(INSTALLER_SCRIPT)
}

fn shipped_script() -> String {
    std::fs::read_to_string(script_path()).expect("packaging/windows/bachelorpad.iss is readable")
}

#[test]
fn every_generated_block_of_the_shipped_installer_is_byte_for_byte_what_bp_platform_generates() {
    let script = shipped_script();
    for block in inno::packaged_blocks().expect("the packaged blocks generate") {
        let shipped = inno::generated_text(&script, &block).unwrap_or_else(|error| {
            panic!("{}: {error}; run `{REGENERATE}`", script_path().display())
        });
        assert!(
            shipped == block.contents,
            "block {} of {} differs from what bp-platform generates; run `{REGENERATE}` and \
             commit the result.\n--- shipped\n{shipped}\n--- generated\n{}",
            block.name,
            script_path().display(),
            block.contents
        );
    }
}

#[test]
fn regenerating_the_shipped_installer_would_change_nothing() {
    // The same promise from the other side: splicing is what the example
    // does, so a script it would rewrite is a script that is out of date,
    // even if some future block escaped the comparison above.
    let script = shipped_script();
    let mut regenerated = script.clone();
    for block in inno::packaged_blocks().expect("the packaged blocks generate") {
        regenerated = inno::splice(&regenerated, &block).expect("the markers are present");
    }
    assert!(
        regenerated == script,
        "run `{REGENERATE}` and commit the result"
    );
}

#[test]
fn the_shipped_installer_is_ascii_so_iscc_reads_it_the_same_on_every_code_page() {
    let script = shipped_script();
    if let Some((line, text)) = script.lines().enumerate().find(|(_, l)| !l.is_ascii()) {
        panic!("line {} is not ASCII: {text}", line + 1);
    }
}

#[test]
fn the_shipped_installer_is_per_user_and_cannot_be_elevated() {
    let script = shipped_script();
    let setup: Vec<&str> = script.lines().map(str::trim).collect();
    assert!(setup.contains(&"PrivilegesRequired=lowest"));
    assert!(
        !setup
            .iter()
            .any(|l| l.starts_with("PrivilegesRequiredOverridesAllowed")),
        "an override would let the installer run elevated and write HKLM"
    );
    assert!(setup.contains(&"ChangesAssociations=yes"));
    assert!(
        !setup
            .iter()
            .any(|l| l.starts_with("Root: HKLM") || l.starts_with("Root: HKA")),
        "every registry entry is HKCU"
    );
}

#[test]
fn the_shipped_installer_never_names_notepad_or_user_choice() {
    let lower = shipped_script().to_ascii_lowercase();
    // Comments may explain the rule; no entry and no code may act on it.
    for line in lower.lines().map(str::trim) {
        if line.starts_with(';') || line.starts_with("//") {
            continue;
        }
        assert!(!line.contains("notepad"), "{line}");
        assert!(!line.contains("userchoice"), "{line}");
    }
}
