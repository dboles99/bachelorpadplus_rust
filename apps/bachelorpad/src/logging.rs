//! Where diagnostics go when there is nobody at a console to read them.
//!
//! A release build on Windows is a GUI-subsystem binary: it has no console,
//! so `tracing` output and a panic's message went nowhere, and a crash was a
//! window that vanished with no record of why (release plan M0-10). This
//! module gives both a file:
//!
//! * a log, `bachelorpad.log`, beside the other per-machine state
//!   (ADR-0029's state directory), written as well as the console rather than
//!   instead of it, so a developer's terminal is unchanged;
//! * a crash report, `crash-<unix time>.log`, written by a panic hook before
//!   the default hook runs.
//!
//! What goes in either is governed by the same rule as every log in this
//! product (ADR-0011, the log-hygiene gate stage): about a document, never
//! from it. A panic message is written by our own code, and the one place it
//! could carry text the user typed is a `panic!` that formats it -- which the
//! workspace lints forbid in library code.
//!
//! Deliberately not here: a message box on Windows. That is `MessageBoxW`,
//! Win32 and `unsafe`, and belongs to `bp-platform-windows` (ADR-0033,
//! release plan M2-03).

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

/// The log file's name inside the log directory.
pub const LOG_FILE: &str = "bachelorpad.log";

/// Past this size the log is rotated at startup, keeping one previous file.
///
/// Rotated at startup and not mid-session: a session's log in one piece is
/// worth more than a precise cap, and the product writes little at the
/// default `warn` level. Two files of at most about 1 MiB each is the budget.
pub const ROTATE_AT_BYTES: u64 = 1024 * 1024;

/// Overrides the log directory. For tests, which must not write into the
/// developer's own state directory, and for a user asked to capture a log
/// somewhere they can find it.
pub const LOG_DIR_ENV: &str = "BACHELORPAD_LOG_DIR";

/// Makes the binary panic on purpose, right after the crash hook is in
/// place, so a test can prove the hook writes a report. A developer switch
/// rather than a command-line flag: nobody should be able to type it by
/// accident, and `--help` has no business advertising it.
pub const CRASH_TEST_ENV: &str = "BACHELORPAD_CRASH_TEST";

/// The directory logs and crash reports go to, if there is one.
pub fn directory() -> Option<PathBuf> {
    match std::env::var_os(LOG_DIR_ENV) {
        Some(dir) if !dir.is_empty() => Some(PathBuf::from(dir)),
        _ => bp_config::locations::log_dir(),
    }
}

/// Open the log for appending, rotating it first if it has grown past
/// [`ROTATE_AT_BYTES`]. `None` if the directory cannot be made or the file
/// cannot be opened: a log that cannot be written is not a reason to refuse
/// to start an editor.
pub fn open_log(dir: &Path) -> Option<File> {
    std::fs::create_dir_all(dir).ok()?;
    let path = dir.join(LOG_FILE);
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > ROTATE_AT_BYTES) {
        let mut previous = path.as_os_str().to_owned();
        previous.push(".1");
        // Losing the older of two logs is the rotation working.
        let _ = std::fs::rename(&path, PathBuf::from(previous));
    }
    OpenOptions::new().create(true).append(true).open(path).ok()
}

/// Send `tracing` output to the console and, when there is one, the log file.
pub fn init(filter: EnvFilter, file: Option<File>) {
    // The console layer keeps the old behaviour exactly: stdout, as
    // `tracing_subscriber::fmt()` wrote before. The file gets no colour codes.
    let console = tracing_subscriber::fmt::layer();
    let to_file = file.map(|file| {
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(std::sync::Mutex::new(file))
    });
    tracing_subscriber::registry()
        .with(filter)
        .with(console)
        .with(to_file)
        .init();
}

/// Install a panic hook that writes a crash report into `dir` and then runs
/// the hook that was there before, so the console output is unchanged.
pub fn install_crash_hook(dir: Option<PathBuf>) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(dir) = &dir
            && let Some(report) = write_crash_report(dir, info)
        {
            eprintln!(
                "BachelorPad+ stopped unexpectedly. A report was written to {}",
                report.display()
            );
        }
        previous(info);
    }));
}

/// Write one crash report and return where it went.
fn write_crash_report(dir: &Path, info: &std::panic::PanicHookInfo<'_>) -> Option<PathBuf> {
    std::fs::create_dir_all(dir).ok()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // Milliseconds in the name, so two crashes in one second -- a crash loop
    // under a file association -- do not overwrite each other.
    let path = dir.join(format!(
        "crash-{}{:03}.log",
        now.as_secs(),
        now.subsec_millis()
    ));

    let message = info
        .payload()
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "(no message)".to_owned());
    let location = info
        .location()
        .map_or_else(|| "unknown".to_owned(), ToString::to_string);
    let thread = std::thread::current()
        .name()
        .unwrap_or("unnamed")
        .to_owned();

    let mut file = File::create(&path).ok()?;
    let _ = writeln!(
        file,
        "BachelorPad+ {version} crashed\n\
         unix time: {secs}\n\
         os: {os} ({arch})\n\
         thread: {thread}\n\
         at: {location}\n\
         message: {message}\n\n\
         backtrace:\n{backtrace}",
        version = env!("CARGO_PKG_VERSION"),
        secs = now.as_secs(),
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        backtrace = std::backtrace::Backtrace::force_capture(),
    );
    Some(path)
}
