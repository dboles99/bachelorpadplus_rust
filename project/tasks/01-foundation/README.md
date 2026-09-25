# 01-foundation

Roadmap phase 1. **Done** (status checked 2026-09-25). The three measurement
gaps below are still open.

| Task | Status |
| --- | --- |
| [workspace-scaffold](01-workspace-scaffold.md) | Done |
| [ui-prototype](02-ui-prototype.md) | Done; native Linux timing open (M4-01) |
| [ci-baseline](03-ci-baseline.md) | Done, both platforms (native Linux gate since M0-13) |
| [config-logging](04-config-logging.md) | Done |

## Phase exit criteria

The app opens a window, opens a file into a tab, saves it atomically, shows
honest save state in the status bar, and does all of that on Windows and
Linux — with the local gate green on both.

**Met**, with the caveats below.

All four tasks are implemented. What remains is verification that this
environment cannot perform, plus work that belongs to later phases.

## Outstanding before the phase is genuinely closed

- **Time to first interaction is unmeasurable** under the default renderer, so
  half of the ADR-0017 target is unverified. Slint's software renderer exposes
  no rendering notifier.
- **Scroll smoothness** under CPU rasterisation is unmeasured; it needs a
  capture rig. This is the standing revert condition on ADR-0017.
- **Comparative Linux timing** needs a native machine; WSLg is not
  representative.

Time to first interaction and Linux timing are release-plan item M4-01.
Scroll smoothness has no plan item yet. Real cursor position, listed here
until 2026-09-25, is done: Ln/Col shows in both views (M1-04).

## Closed since the last review

Unsaved-changes prompting (tab close, Ctrl+W, window close), keyboard
shortcuts, the per-keystroke gutter rebuild, typing-latency measurement,
configuration loading, and a working menu bar.

**Verified by manual test on 2026-08-17:** keyboard shortcuts, the
unsaved-changes prompts, the gutter's trailing-newline fix, theme cycling,
Save As naming, and status-bar save state. That test also found the menu bar's
real defect — twelve labels that swallowed a click, which reads as broken.
All fourteen menus then opened. Since M1-05 a release bar shows ten: Notebook,
Organize, Research and Run are hidden behind `show_planned_menus`.

The gate gained two stages. `launch` exists because the one bug that reached a
"green" state was a binary that could not start — something neither
`cargo build` nor `cargo test` ever checks. `log hygiene` exists because
ADR-0011's no-secrets-in-logs rule was a convention, and a convention is not
enforcement.
