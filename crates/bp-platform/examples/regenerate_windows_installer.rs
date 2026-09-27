//! Rewrite the generated blocks of `packaging/windows/bachelorpad.iss`.
//!
//! Run from anywhere in the workspace:
//!
//! ```text
//! cargo run -p bp-platform --example regenerate_windows_installer
//! ```
//!
//! The installer's `[Registry]` section and its list of the user's
//! directories come from `bp_platform::editor::inno`; the
//! `windows_installer` test fails when the shipped script differs from it by
//! a byte, and this is how to make them agree again. Only the text between
//! each pair of `BEGIN GENERATED` / `END GENERATED` lines is replaced; the
//! rest of the script is hand-written and left exactly as it is.

use std::path::Path;

use bp_platform::editor::inno;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(inno::INSTALLER_SCRIPT);
    let mut script = std::fs::read_to_string(&path)?;
    for block in inno::packaged_blocks()? {
        script = inno::splice(&script, &block)?;
        println!("regenerated block {}", block.name);
    }
    std::fs::write(&path, script)?;
    println!("wrote {}", path.display());
    Ok(())
}
