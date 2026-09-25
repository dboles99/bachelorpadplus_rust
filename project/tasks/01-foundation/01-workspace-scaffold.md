# Task: workspace-scaffold

**Phase:** 01-foundation
**Status:** Done (checked 2026-09-25)

## Objective

Establish the Cargo workspace and the core model that everything above it
plugs into, without pulling in a UI toolkit or the semantic layer.

## Acceptance criteria

- [x] Cargo workspace with `bp-*` crates and an `apps/bachelorpad` binary.
- [x] `Cargo.lock` tracked, and the gate runs `--locked`.
- [x] `bp-core` models `Document`, `DocumentId`, `SaveState` and `Workspace`,
      depends on no other `bp-*` crate (ADR-0006), and holds no text.
- [x] A recovery checkpoint cannot be represented as a disk save. Enforced by
      distinct types, not by convention (ADR-0005).
- [x] Creation date is fixed at creation and has no setter (ADR-0004).
- [x] `bp-naming` implements the filename grammar as a pure module: no
      filesystem access, no clock reads, collision resolution takes an
      injected `exists` predicate (ADR-0003).
- [x] `bp-files` saves atomically — temp file in the target directory,
      flushed to the device, read back, then renamed over the target — and a
      failed save provably leaves the original intact (ADR-0007).
- [x] Every error path names the file and says what the user can do.
- [x] Unit tests for the above, plus property tests over the filename grammar.
- [x] The remaining placeholder crates either gain content or are removed.
      Removed: only crates with content are workspace members, and the
      intended map lives in `docs/architecture/ARCHITECTURE.md`.

## Artifacts

- `crates/bp-core`, `crates/bp-naming`, `crates/bp-files`

## Notes

When this task was written, 24 of the 27 crates were four-line placeholders.
That was a deliberate scaffold of the intended shape, but each one was a real
compile unit and a real merge surface for a structure nothing had exercised.
They were removed, and the rule became the one in the workspace
`Cargo.toml`: create a crate when there is something to put in it. On
2026-09-25 the workspace has 24 crates under `crates/`, the smallest
(`bp-theme`) about 300 lines.
