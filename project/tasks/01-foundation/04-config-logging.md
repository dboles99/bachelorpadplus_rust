# Task: config-logging

**Phase:** 01-foundation
**Status:** Not started

## Objective

Configuration loading and structured logging, with the constraint that
neither may become a way for document contents or secrets to reach disk.

## Acceptance criteria

- [ ] Configuration resolves from, in precedence order: command-line
      arguments, environment (`BACHELORPAD_*`), a user config file, then
      built-in defaults.
- [ ] Config file location follows platform convention on Windows and Linux,
      resolved through `bp-platform` rather than hardcoded.
- [ ] A malformed config file produces a clear message naming the file and
      the problem, and the app still starts on defaults. Bad config must not
      make the editor unopenable.
- [ ] Logging via `tracing`, level controlled by `BACHELORPAD_LOG`,
      defaulting to something quiet.
- [ ] **No document content, file contents, clipboard data, passphrases or
      key material is ever logged at any level.** Log paths and sizes, never
      bodies (ADR-0011, and the Definition of Done's "no secret data in
      logs").
- [ ] Log destination respects the security profile: a Private or higher
      document must not leave a trail of its own filename in a world-readable
      log.
- [ ] Tests covering precedence order, malformed-config recovery, and that
      the logging surface has no path for document content.

## Notes

`.env.example` currently documents `BACHELORPAD_LOG`. Keep it in step with
whatever the implementation actually reads.

The redaction requirement is the substance of this task, not a footnote —
logging is one of the easier ways to defeat the entire security model in
ADR-0011.
