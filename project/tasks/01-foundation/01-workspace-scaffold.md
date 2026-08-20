# Task: workspace-scaffold

**Phase:** 01-foundation
**Status:** Done for the crates in use; most crates remain placeholders

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
- [ ] The remaining placeholder crates either gain content or are removed.

## Artifacts

- `crates/bp-core`, `crates/bp-naming`, `crates/bp-files`

## Notes

24 of the 27 crates are still four-line placeholders. That is a deliberate
scaffold of the intended shape, but each one is a real compile unit and a
real merge surface for a structure nothing has exercised yet. Prefer creating
a crate when there is something to put in it.
