# Task: ui-prototype

**Phase:** 01-foundation
**Status:** Spike complete; integration blocked on BP-ADR-0015 sign-off

## Objective

Choose the UI toolkit on evidence, then build the BachelorPad+ shell on it:
window, menu bar, tab strip, editor pane, status bar and four themes.

`PROJECT_MEMORY.md` requires the toolkit be benchmarked and prototyped before
an irreversible commitment, so this task splits in two: decide, then build.

## Acceptance criteria

### Part A — toolkit decision (done)

- [x] The same shell implemented on at least two candidate toolkits, with
      identical release profiles so the numbers are comparable.
- [x] Both instrumented to report readiness at the same point (first frame
      rendered), verified rather than assumed.
- [x] Startup, idle memory and binary size measured by a repeatable harness,
      with raw data recorded under `artifacts/benchmarks/`.
- [x] Results compared against the specs.md section 22 targets, including
      where they are missed.
- [x] Written up as an ADR with the caveats stated.
- [ ] ADR-0015 accepted by the owner.
- [ ] Linux measurements taken (needs a Rust toolchain in WSL; see
      `docs/governance/LOCAL_CI.md`).

### Part B — the real shell (not started)

- [ ] Window, menu bar (all 14 menus from `docs/product/MENU_MAP.md`), tab
      strip, editor pane and two-line status bar, built in `bp-ui`.
- [ ] Four themes from ADR-0009, defined as data, switchable at runtime,
      restyling every surface including text input.
- [ ] Status bar renders from `bp-core`'s `Document`, and a recovery
      checkpoint is never displayed as a disk save (ADR-0005).
- [ ] New / Open / Save / Save As wired to `bp-files`, with error states
      surfaced to the user rather than logged.
- [ ] Multiple documents open as tabs, backed by `bp-core::Workspace`.
- [ ] Builds and runs on Windows and Linux.
- [ ] The losing spike deleted and the winner folded into `bp-ui`.

## Artifacts

- `spikes/ui-toolkit/` — the two candidate shells
- `scripts/Measure-UiSpike.ps1` — measurement harness
- `artifacts/ui/ui-toolkit-spike-17AUG2026.md` — findings
- `artifacts/benchmarks/ui-toolkit-*.json` — raw measurements
- `docs/decisions/ADR-0015.md` — the decision

## Notes

Neither candidate met the specs.md section 22 startup targets, missing cold
start by roughly 6-8x. That is tracked as an open item in `DECISIONS.md` and
needs its own decision — a software-renderer spike, revised targets, or a
split between time-to-window and time-to-interactive. Do not start Part B
assuming the targets are reachable as written.
