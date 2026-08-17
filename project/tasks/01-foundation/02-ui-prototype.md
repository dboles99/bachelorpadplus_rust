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
- [x] **Keyboard shortcuts.** Ctrl+N/O/S/Shift+S/W via Slint `KeyBinding`
      elements in a `FocusScope` that wraps the content. Bindings match during
      the capture phase, from the Window down toward the focused element, so
      the editor no longer swallows Ctrl+S. **Confirmed working by manual
      test, 2026-08-17.**
- [x] **All 14 menus open.** File, Edit, View, Format and Help do real work;
      the other nine list their intended contents as disabled rows naming the
      phase they arrive in. A label that swallows a click reads as broken —
      which is how the previous version was reported.
- [x] Edit menu is genuinely functional: undo, redo, cut, copy, paste and
      select-all are public functions on Slint's `TextInput`, so they are not
      stubs even though our own `bp-editor` is not yet the authority.
- [x] View: theme selection with the active one ticked, line-number and
      word-wrap toggles.
- [x] Format: line endings and encoding, ticked to the document's real
      settings. Changing either marks the document unsaved, because the bytes
      that would be written no longer match disk.
- [x] File: Save All (disabled when nothing is unsaved) and Reload from Disk
      (disabled without a file, and prompts before discarding edits).
- [x] **Unsaved-changes prompt on close.** Closing a dirty tab, closing the
      active tab with Ctrl+W, and closing the window all prompt Save / Discard
      / Cancel. A save that fails or is abandoned keeps the tab open rather
      than discarding the work.
- [x] **Typing latency measured.** `--latency-probe`; see ADR-0017. 376 µs p50
      at 1 MB against a 16 ms frame budget.
- [x] **Gutter no longer rebuilt on every keystroke** — only when the line
      count changes.
- [ ] **Real cursor position.** The status bar reports a line count, not
      Ln/Col. Needs editor state from `bp-editor`.
- [ ] **Rope buffer.** Text lives in a `HashMap<DocumentId, String>`, and
      Slint hands the whole buffer back on every edit, so a keystroke copies
      the document. Measured: linear, reaching the frame budget near 40 MB.
      That makes `bp-buffer` a phase-4 large-file concern rather than a
      phase-1 correctness one.

## Artifacts

- `crates/bp-ui`, `crates/bp-theme`
- `scripts/Measure-Startup.ps1`
- `artifacts/ui/ui-toolkit-spike-17AUG2026.md`
- `artifacts/benchmarks/*.json`
- `docs/decisions/ADR-0015.md`, `docs/decisions/ADR-0017.md`

## Notes

Enabling rfd's `common-controls-v6` feature made the binary fail to start with
`STATUS_ENTRYPOINT_NOT_FOUND` before `main`. Neither `cargo build` nor
`cargo test` catches that, because neither loads the linked executable. Any
change to link-time features or manifests needs a launch check, not just a
green gate.
