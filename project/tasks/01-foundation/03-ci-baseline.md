# Task: ci-baseline

**Phase:** 01-foundation
**Status:** Done on both platforms (checked 2026-09-25)

## Objective

Establish an automatic quality gate. GitHub Actions is unavailable to this
project, so the gate is local and it is the only gate (ADR-0016).

## Acceptance criteria

- [x] One script runs format, locked dependency check, clippy with warnings
      denied, and the test suite.
- [x] Every stage runs even after one fails, so a run reports everything at
      once; exit code is non-zero if any stage failed.
- [x] Wired into `pre-commit` and `pre-push` so it runs without anyone
      remembering to.
- [x] Hook installation verifies the executable bit, because the global
      `core.hooksPath` stubs skip non-executable hooks silently.
- [x] Installation does not set a repo-local `core.hooksPath`, which would
      disable the machine's global commit-msg hook.
- [x] A documented bypass exists for deliberate work-in-progress commits.
- [x] The dormant GitHub workflow is set to manual trigger only, so pushes do
      not queue runs that cannot execute, and its steps match the local gate.
      No longer true of the second half on 2026-09-25: the workflow has no
      launch or log-hygiene step and no Linux system packages. Bringing it
      back in step is release-plan item M3-07.
- [x] Documented in `docs/governance/LOCAL_CI.md`.
- [x] Linux leg enabled — needed a Rust toolchain inside WSL. Working since
      2026-08-17 (`docs/governance/LOCAL_CI.md`, "The Linux leg"); on a Linux
      host the same script runs every stage natively (release-plan M0-13).

## Artifacts

- `scripts/Invoke-LocalCI.ps1`
- `scripts/Install-GitHooks.ps1`
- `scripts/hooks/pre-commit`, `scripts/hooks/pre-push`
- `docs/governance/LOCAL_CI.md`
- `docs/decisions/ADR-0016.md`

## Notes

BP-ADR-0001 makes Linux first-class, and nothing in the current setup would
catch a Windows-only regression. The Linux leg is the only thing that would,
so enabling it matters more than it looks.
