# Task: ui-prototype

**Phase:** 01-foundation
**Status:** Shell running on both platforms; keyboard shortcuts and buffer outstanding

## Objective

Choose the UI toolkit on evidence, then build the BachelorPad+ shell on it:
window, menu bar, tab strip, editor pane, status bar and four themes.

## Acceptance criteria

### Part A — toolkit decision (done)

- [x] The same shell implemented on two candidate toolkits, identical release
      profiles.
- [x] Both instrumented to report readiness at the same point, verified rather
      than assumed.
- [x] Startup, idle memory and binary size measured by a repeatable harness,
      raw data under `artifacts/benchmarks/`.
- [x] Results compared against specs.md section 22, including where missed.
- [x] Written up as ADR-0015, accepted.
- [x] Startup targets resolved separately as ADR-0017, with the software
      renderer adopted as the default.
- [x] Linux viability confirmed — builds, links and runs.
- [ ] Comparative Linux *timing*. WSLg is not representative; needs a native
      machine.

### Part B — the real shell

- [x] Window, 14-menu bar, tab strip, editor pane and two-line status bar,
      built in `bp-ui`.
- [x] Four themes from ADR-0009 supplied as data from `bp-theme`.
      `app.slint` contains no colour literals.
- [x] Status bar renders from `bp-core`'s `Document`; a recovery checkpoint is
      never displayed as a disk save (ADR-0005).
- [x] New / Open / Save / Save As wired to `bp-files`, with errors shown in
      the status bar rather than logged and swallowed.
- [x] Save on a never-saved document escalates to Save As instead of silently
      doing nothing.
- [x] Save As suggests a filename in the ADR-0003 grammar.
- [x] Multiple documents open as tabs, backed by `bp-core::Workspace`;
      closing the last tab opens a fresh one.
- [x] Encoding and line endings round-trip: a CRLF file with a BOM is written
      back that way.
- [x] Builds and runs on Windows and Linux.
- [x] Both spikes deleted; instrumentation moved into the real application.
- [ ] **Keyboard shortcuts.** Ctrl+N/O/S/Shift+S exist as menu labels only.
      A `TextInput` with focus consumes key events before an ancestor
      `FocusScope` sees them, so this needs a deliberate solution rather than
      a wrapper.
- [ ] **Real cursor position.** The status bar reports a line count, not
      Ln/Col. Needs editor state from `bp-editor`.
- [ ] **Unsaved-changes prompt on close.** Closing a dirty tab currently
      discards it. `Workspace::close` returns the document so the caller can
      prompt; the caller does not yet.
- [ ] **Rope buffer.** Text lives in a `HashMap<DocumentId, String>` and the
      gutter is rebuilt on every keystroke. Correct for notes, wrong for the
      large files specs.md section 5 promises. `bp-buffer`, phase 2.

## Artifacts

- `crates/bp-ui`, `crates/bp-theme`
- `scripts/Measure-Startup.ps1`
- `artifacts/ui/ui-toolkit-spike-17AUG2026.md`
- `artifacts/benchmarks/*.json`
- `docs/decisions/ADR-0015.md`, `docs/decisions/ADR-0017.md`

## Notes

The unsaved-changes prompt is the one gap here with data-loss consequences.
Everything else on the outstanding list is an inconvenience.
