//! BachelorPad+ — Notepad when you want it. More when you need it.

// No console window on Windows for a release build. Kept for debug builds so
// `tracing` output stays visible while developing.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
// No `unsafe` here, ever: the Win32 calls a release needs (`AttachConsole`,
// `MessageBoxW`) belong to `bp-platform-windows`, the one crate ADR-0033
// allows it in.
#![forbid(unsafe_code)]

mod logging;

use bp_config::RendererPref;
use bp_config::cli::{self, Action};
use bp_theme::ThemeId;

fn main() -> anyhow::Result<()> {
    // `argv[0]` is the executable, not a file the user asked for.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let invocation = cli::parse(&args);

    // Answered before configuration is even read: asking for the version must
    // work with a broken config.toml, and must print nothing else.
    //
    // A release build on Windows is a GUI-subsystem binary with no console,
    // so launched from a shell these lines would go nowhere. Attaching to the
    // parent's console first gives them somewhere to go (ADR-0033, release
    // plan M2-03). Only for these two: a normal launch must not grab the
    // console of the shell it was started from. The call is a no-op that
    // returns false on Linux and in a debug build (which already has a
    // console), so there is no `cfg` here; whether it attached changes
    // nothing about what is printed.
    match invocation.action {
        Action::Version => {
            let _ = bp_platform_windows::attach_parent_console();
            println!("bachelorpad {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Action::Help => {
            let _ = bp_platform_windows::attach_parent_console();
            print!("{}", cli::usage());
            return Ok(());
        }
        Action::Run => {}
    }

    let loaded = bp_config::load(&args);

    // Logging level comes from configuration, so BACHELORPAD_LOG and the
    // config file agree on one answer. An unparseable filter falls back
    // rather than refusing to start.
    let filter = tracing_subscriber::EnvFilter::try_new(&loaded.config.log)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    // To a file as well as the console, and a crash report on panic: a
    // release build on Windows has no console, so without these a failure
    // left no trace at all (release plan M0-10).
    let log_dir = logging::directory();
    logging::init(filter, log_dir.as_deref().and_then(logging::open_log));
    logging::install_crash_hook(log_dir.clone());
    if std::env::var_os(logging::CRASH_TEST_ENV).is_some() {
        crash_on_purpose();
    }

    // Report configuration problems before the self-check and probe exits, so
    // a broken config is visible however the binary was run -- not only on the
    // path that opens a window. (`--version` and `--help` are the exception
    // above, by design.)
    if let Some(source) = &loaded.source {
        tracing::info!("configuration loaded from {}", source.display());
    }
    // Command-line mistakes first: they are what the user typed a moment
    // ago, and the status bar has room for only the first notice.
    let mut notices: Vec<String> = invocation
        .warnings
        .iter()
        .map(ToString::to_string)
        .chain(loaded.notices.iter().map(ToString::to_string))
        .collect();

    // An unrecognised theme name is a recoverable mistake: warn and fall back
    // rather than refusing to start over a spelling. Resolved here, alongside
    // the other notices, so every invocation reports the same set.
    let follow_system_theme = loaded
        .config
        .theme
        .as_deref()
        .is_some_and(|name| name.eq_ignore_ascii_case(bp_theme::SYSTEM_THEME_NAME));
    let theme = loaded
        .config
        .theme
        .as_ref()
        .filter(|_| !follow_system_theme)
        .and_then(|name| {
            let id = ThemeId::from_name(name);
            if id.is_none() {
                notices.push(format!("'{name}' is not a known theme; using the default"));
            }
            id
        });

    for notice in &notices {
        tracing::warn!("startup: {notice}");
    }

    // Load-and-exit check for the local CI gate. Neither `cargo build` nor
    // `cargo test` ever loads the linked executable, so a bad link-time
    // feature or manifest passes both and still fails before `main` --
    // exactly what enabling rfd's `common-controls-v6` did. Reaching this
    // line at all is the assertion.
    if invocation.self_check {
        println!("BACHELORPAD_SELF_CHECK_OK");
        return Ok(());
    }

    if invocation.latency_probe {
        bp_ui::latency_probe();
        return Ok(());
    }

    tracing::info!("starting BachelorPad+");
    bp_ui::run_with(bp_ui::RunOptions {
        files: invocation.files,
        line: invocation.line,
        // Both open the files in the read-only viewer (release plan M1-06);
        // `--readonly` says so in the banner.
        readonly: invocation.readonly,
        large_file: invocation.large_file,
        renderer: match loaded.config.renderer {
            RendererPref::Software => bp_ui::Renderer::Software,
            RendererPref::Platform => bp_ui::Renderer::Platform,
        },
        theme,
        follow_system_theme,
        startup_notice: notices.first().map(|first| {
            if notices.len() > 1 {
                format!("{first} (+{} more; see the log)", notices.len() - 1)
            } else {
                first.clone()
            }
        }),
        font_size: Some(loaded.config.font_size),
        measure_exit: invocation.measure_exit,
        // Opt-in while the custom editor surface reaches parity with the
        // widget it replaces (word wrap, input-method composition): the flag
        // is for trying it once, `editor_view` in config.toml for keeping it.
        editor_view: invocation.editor_view || loaded.config.editor_view,
        settings_path: loaded.path.clone(),
        log_dir,
        config: Some(loaded.config),
    })?;
    Ok(())
}

/// The deliberate crash `logging::CRASH_TEST_ENV` asks for, so a test can
/// prove the crash hook writes its report. The one `panic!` in production
/// code, and the reason it is allowed is its whole job.
#[allow(clippy::panic)]
fn crash_on_purpose() -> ! {
    panic!("deliberate crash test ({})", logging::CRASH_TEST_ENV);
}
