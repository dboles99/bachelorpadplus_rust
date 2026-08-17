# 01-foundation

| Task | Status |
| --- | --- |
| [workspace-scaffold](01-workspace-scaffold.md) | Done |
| [ui-prototype](02-ui-prototype.md) | Shell runs on both platforms; shortcuts, cursor, close-prompt outstanding |
| [ci-baseline](03-ci-baseline.md) | Done, both platforms |
| [config-logging](04-config-logging.md) | Logging wired; configuration not started |

## Phase exit criteria

The app opens a window, opens a file into a tab, saves it atomically, shows
honest save state in the status bar, and does all of that on Windows and
Linux — with the local gate green on both.

**Met**, with the caveats below.

## Outstanding before the phase is genuinely closed

- **Closing a dirty tab discards it without prompting.** The only outstanding
  item with data-loss consequences.
- **No keyboard shortcuts.** Ctrl+S is table stakes for a text editor; it
  currently exists only as a label in the File menu.
- **Configuration loading** (task 04) has not started. Logging is wired and
  reads `BACHELORPAD_LOG`.
- **Time to first interaction is unmeasurable** under the default renderer,
  so half of the ADR-0017 target is unverified.
- **Comparative Linux timing** needs a native machine; WSLg is not
  representative.
