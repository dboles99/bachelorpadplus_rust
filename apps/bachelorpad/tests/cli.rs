//! The binary's command line, run as a process.
//!
//! `bp_config::cli` tests what the arguments mean; these test only what the
//! parser cannot see -- that the executable acts on the answer, prints to
//! stdout and exits 0. None of them opens a window: each flag used here
//! returns before the shell starts.

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bachelorpad"))
        .args(args)
        // A developer's own config must not change what these assert.
        .env(
            "BACHELORPAD_CONFIG",
            "/nonexistent/bachelorpad-test/config.toml",
        )
        .output()
        .expect("the binary launches")
}

#[test]
fn version_prints_the_package_version_and_exits_cleanly() {
    let out = run(&["--version"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!("bachelorpad {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn help_lists_the_flags_and_exits_cleanly() {
    let out = run(&["--help"]);
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8_lossy(&out.stdout);
    for flag in ["--line", "--readonly", "--large-file", "--version"] {
        assert!(text.contains(flag), "--help does not mention {flag}");
    }
}

#[test]
fn an_unknown_flag_does_not_stop_the_binary() {
    let out = run(&["--no-such-flag", "--self-check"]);
    assert!(
        out.status.success(),
        "an unknown flag must warn, never refuse to start: {out:?}"
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("BACHELORPAD_SELF_CHECK_OK"));
}
