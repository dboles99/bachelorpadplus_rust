//! Rewrite `packaging/linux/` from `bp_platform::editor::desktop`.
//!
//! Run from anywhere in the workspace:
//!
//! ```text
//! cargo run -p bp-platform --example regenerate_linux_assets
//! ```
//!
//! The shipped `.desktop` entry, MIME package and AppStream metadata are
//! generated, not edited: the `linux_assets` test fails when they differ from
//! `packaged_assets()` by a single byte, and this is how to make them agree
//! again. An example rather than a build script, because a build that writes
//! into the source tree is a build whose output depends on whether somebody
//! ran it; this runs when a developer asks and the diff shows what changed.

use std::path::Path;

fn main() -> std::io::Result<()> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging/linux");
    std::fs::create_dir_all(&directory)?;
    for asset in bp_platform::editor::desktop::packaged_assets() {
        let path = directory.join(&asset.file_name);
        std::fs::write(&path, &asset.contents)?;
        println!("wrote {}", path.display());
    }
    Ok(())
}
