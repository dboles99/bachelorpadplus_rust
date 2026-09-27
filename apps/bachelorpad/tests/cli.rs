//! The binary's command line, run as a process.
//!
//! `bp_config::cli` tests what the arguments mean; these test only what the
//! parser cannot see -- that the executable acts on the answer, prints to
//! stdout and exits 0. None of them opens a window: each flag used here
//! returns before the shell starts.

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    let logs = tempfile::tempdir().expect("temp dir");
    run_with_logs(args, logs.path(), false)
}

/// The binary with its log directory redirected, as every process test must
/// run it: launching it creates a log file, and that must never land in the
/// developer's own state directory.
fn run_with_logs(args: &[&str], logs: &std::path::Path, crash: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_bachelorpad"));
    command
        .args(args)
        // A developer's own config must not change what these assert.
        .env(
            "BACHELORPAD_CONFIG",
            "/nonexistent/bachelorpad-test/config.toml",
        )
        .env("BACHELORPAD_LOG_DIR", logs);
    if crash {
        command.env("BACHELORPAD_CRASH_TEST", "1");
    }
    command.output().expect("the binary launches")
}

#[test]
fn a_run_leaves_a_log_file_in_the_log_directory() {
    let logs = tempfile::tempdir().expect("temp dir");
    let out = run_with_logs(&["--self-check"], logs.path(), false);
    assert!(out.status.success(), "{out:?}");
    assert!(
        logs.path().join("bachelorpad.log").is_file(),
        "a release build on Windows has no console; the log file is the only record"
    );
}

#[test]
fn a_crash_writes_a_report_naming_the_version_and_where_it_happened() {
    let logs = tempfile::tempdir().expect("temp dir");
    let out = run_with_logs(&["--self-check"], logs.path(), true);
    assert!(!out.status.success(), "the deliberate crash did not happen");

    let reports: Vec<_> = std::fs::read_dir(logs.path())
        .expect("log dir")
        .map(|e| e.expect("entry").path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("crash-"))
        })
        .collect();
    assert_eq!(reports.len(), 1, "{reports:?}");
    let report = std::fs::read_to_string(&reports[0]).expect("report");
    assert!(report.contains(env!("CARGO_PKG_VERSION")), "{report}");
    assert!(report.contains("deliberate crash test"), "{report}");
    assert!(
        report.contains("main.rs"),
        "the location is missing: {report}"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("A report was written to"),
        "the user is not told where the report is"
    );
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
