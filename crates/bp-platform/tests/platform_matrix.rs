//! `docs/architecture/PLATFORM_MATRIX.md` carries the capability register
//! as `bp-platform` generates it (release plan M2-12).
//!
//! The matrix was edited by hand before, and it drifted: it said what
//! somebody believed on the day they last touched it. Now the part that
//! states what each platform can do is generated, and this compares it byte
//! for byte. The fix is always to regenerate, never to edit it by hand.

use std::path::Path;

use bp_platform::capabilities::{MATRIX_BEGIN, MATRIX_END, matrix_markdown};

const REGENERATE: &str = "run `cargo run -p bp-platform --example regenerate_platform_matrix` \
                          and commit the result";

#[test]
fn the_platform_matrix_says_exactly_what_the_capability_register_says() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/architecture/PLATFORM_MATRIX.md");
    let document = std::fs::read_to_string(&path).expect("PLATFORM_MATRIX.md is readable");
    let begin = document
        .find(MATRIX_BEGIN)
        .unwrap_or_else(|| panic!("the begin marker is missing; {REGENERATE}"));
    let end = document
        .find(MATRIX_END)
        .unwrap_or_else(|| panic!("the end marker is missing; {REGENERATE}"));
    let shipped = &document[begin..end + MATRIX_END.len()];
    let generated = matrix_markdown();
    assert!(
        generated.trim_end() == shipped,
        "PLATFORM_MATRIX.md differs from the register; {REGENERATE}.\n--- shipped\n{shipped}\n\
         --- generated\n{generated}"
    );
}

#[test]
fn every_capability_appears_in_the_generated_table() {
    let generated = matrix_markdown();
    for capability in bp_platform::capabilities::Capability::ALL {
        assert!(
            generated.contains(&format!("| `{}` |", capability.token())),
            "{capability:?}"
        );
    }
}
