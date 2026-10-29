# Task: replace-preview

**Done.** `dispatch::confirm_replace` shows `plan.summary()` and `plan.preview(12)` in a native dialog before any replacements are applied, satisfying specs.md's requirement that changes be shown before they are applied.

## Objective

Implement and validate `replace-preview` as part of phase `06-search`.

## Steps

1. Inspect relevant specifications and ADRs.
2. Define interfaces and acceptance criteria.
3. Implement smallest production-quality vertical slice.
4. Add tests before broadening scope.
5. Validate cross-platform/security/performance implications as applicable.
6. Record artifacts and metadata.
7. Update decision log only if architecture changes.

## Required Rosetta

Use the closest prompt in `prompts/rosettas/`, adapting only the task-specific variables.

## Artifacts

Store task outputs under an appropriate `artifacts/` category or source crate.

## Metadata

Record generated artifact metadata under `metadata/` where useful.

## Database

Add/modify schema only through explicit migrations.

## Notes

- **Implementation:** `confirm_replace` in `crates/bp-ui/src/dispatch.rs` (line ~209) displays a preview of up to 12 replacement contexts before applying.
- **Wiring:** Called from `ui.on_replace_all` callback in `crates/bp-ui/src/lib.rs` (line ~959).
- **Verification:** Preview shown before any document modification; user must confirm with OK to proceed.

Keep observations that are not architectural decisions here or in `notes/`.
