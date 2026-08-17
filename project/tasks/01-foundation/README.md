# 01-foundation

| Task | Status |
| --- | --- |
| [workspace-scaffold](01-workspace-scaffold.md) | Done |
| [ui-prototype](02-ui-prototype.md) | Shell runs on both platforms; shortcuts, cursor, close-prompt outstanding |
| [ci-baseline](03-ci-baseline.md) | Done, both platforms |
| [config-logging](04-config-logging.md) | Done |

## Phase exit criteria

The app opens a window, opens a file into a tab, saves it atomically, shows
honest save state in the status bar, and does all of that on Windows and
Linux — with the local gate green on both.

**Met**, with the caveats below.

All four tasks are implemented. What remains is verification that this
environment cannot perform, plus work that belongs to later phases.

## Outstanding before the phase is genuinely closed

- **Keyboard shortcut delivery is unverified.** The bindings are wired and the
  Slint compiler validates them, but no key has actually been pressed — this
  environment has no interactive desktop. Worth 30 seconds of manual checking.
- **Time to first interaction is unmeasurable** under the default renderer, so
  half of the ADR-0017 target is unverified. Slint's software renderer exposes
  no rendering notifier.
- **Scroll smoothness** under CPU rasterisation is unmeasured; it needs a
  capture rig. This is the standing revert condition on ADR-0017.
- **Comparative Linux timing** needs a native machine; WSLg is not
  representative.
- **Real cursor position** in the status bar needs `bp-editor`.

## Closed since the last review

Unsaved-changes prompting (tab close, Ctrl+W, window close), keyboard
shortcuts, the per-keystroke gutter rebuild, typing-latency measurement, and
configuration loading.

The gate gained two stages. `launch` exists because the one bug that reached a
"green" state was a binary that could not start — something neither
`cargo build` nor `cargo test` ever checks. `log hygiene` exists because
ADR-0011's no-secrets-in-logs rule was a convention, and a convention is not
enforcement.
