//! Rewrite the generated part of `docs/architecture/PLATFORM_MATRIX.md` from
//! `bp_platform::capabilities` (release plan M2-12).
//!
//! ```text
//! cargo run -p bp-platform --example regenerate_platform_matrix
//! ```
//!
//! Everything between the two markers is replaced; everything outside them
//! is the document's own and left alone. The `platform_matrix` test fails
//! when the two differ, and this is how to make them agree again.

use std::path::Path;

use bp_platform::capabilities::{MATRIX_BEGIN, MATRIX_END, matrix_markdown};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/architecture/PLATFORM_MATRIX.md");
    let document = std::fs::read_to_string(&path)?;
    let (Some(begin), Some(end)) = (document.find(MATRIX_BEGIN), document.find(MATRIX_END)) else {
        return Err(format!("{} has lost its generated-section markers", path.display()).into());
    };
    // The end marker's own line goes too; `matrix_markdown` supplies it.
    let after = document[end..]
        .find('\n')
        .map_or(document.len(), |newline| end + newline + 1);
    let rewritten = format!(
        "{}{}{}",
        &document[..begin],
        matrix_markdown(),
        &document[after..]
    );
    std::fs::write(&path, rewritten)?;
    println!("wrote {}", path.display());
    Ok(())
}
