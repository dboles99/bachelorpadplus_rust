//! BachelorPad+ — Notepad when you want it. More when you need it.

// No console window on Windows for a release build. Kept for debug builds so
// `tracing` output stays visible while developing.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use bp_config::RendererPref;
use bp_config::cli::{self, Action, Invocation};
use bp_theme::ThemeId;

fn main() -> anyhow::Result<()> {
    // `argv[0]` is the executable, not a file the user asked for.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let invocation = cli::parse(&args);

    // Answered before configuration is even read: asking for the version must
    // work with a broken config.toml, and must print nothing else.
    //
    // A release build on Windows is a GUI-subsystem binary with no console,
    // so these lines currently go nowhere there when launched from a shell.
    // Attaching to the parent's console needs `AttachConsole`, which is
    // `unsafe` Win32 and arrives with `bp-platform-windows` (ADR-0033,
    // release plan M2-03) -- the one crate allowed to hold it. Debug builds
    // keep their console, and Linux is unaffected.
    match invocation.action {
        Action::Version => {
            println!("bachelorpad {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Action::Help => {
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
    tracing_subscriber::fmt().with_env_filter(filter).init();

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
        .chain(not_yet_applied(&invocation))
        .collect();

    // An unrecognised theme name is a recoverable mistake: warn and fall back
    // rather than refusing to start over a spelling. Resolved here, alongside
    // the other notices, so every invocation reports the same set.
    let theme = loaded.config.theme.as_ref().and_then(|name| {
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
        renderer: match loaded.config.renderer {
            RendererPref::Software => bp_ui::Renderer::Software,
            RendererPref::Platform => bp_ui::Renderer::Platform,
        },
        theme,
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
        // widget it replaces (word wrap, input-method composition). Parsed
        // here beside the other switches rather than through the config
        // precedence, because it is a thing to try rather than a preference
        // to keep.
        editor_view: invocation.editor_view,
    })?;
    Ok(())
}

/// Notices for flags that parse but that the shell cannot act on yet.
///
/// `--line`, `--readonly` and `--large-file` are understood by `cli::parse`
/// (specs.md section 19), but `bp_ui::RunOptions` has no field to carry them
/// to the window. Silently opening a file editable after `--readonly` was
/// asked for is the one outcome worse than refusing, so each says in the
/// status bar that it was not applied. The notice goes when `RunOptions`
/// grows the field and this function stops listing it.
fn not_yet_applied(invocation: &Invocation) -> Vec<String> {
    let mut notices = Vec::new();
    if let Some(line) = invocation.line {
        notices.push(format!(
            "--line {line} is not applied yet; the caret starts at the top"
        ));
    }
    if invocation.readonly {
        notices.push("--readonly is not applied yet; the files open editable".to_owned());
    }
    if invocation.large_file {
        notices.push("--large-file is not applied yet; the files open in the editor".to_owned());
    }
    notices
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_strs(list: &[&str]) -> Invocation {
        let args: Vec<String> = list.iter().map(|s| (*s).to_owned()).collect();
        cli::parse(&args)
    }

    #[test]
    fn a_plain_invocation_reports_nothing_unapplied() {
        assert!(not_yet_applied(&parse_strs(&["a.txt"])).is_empty());
    }

    #[test]
    fn readonly_that_cannot_be_honoured_says_so_rather_than_opening_editable() {
        let notices = not_yet_applied(&parse_strs(&["--readonly", "a.txt"]));
        assert_eq!(notices.len(), 1);
        assert!(
            notices[0].contains("editable"),
            "the user must learn the file is not protected: {notices:?}"
        );
    }

    #[test]
    fn each_unapplied_flag_gets_its_own_notice() {
        let notices = not_yet_applied(&parse_strs(&[
            "--line",
            "427",
            "--readonly",
            "--large-file",
            "server.log",
        ]));
        assert_eq!(notices.len(), 3, "{notices:?}");
        assert!(notices[0].contains("427"));
    }
}
