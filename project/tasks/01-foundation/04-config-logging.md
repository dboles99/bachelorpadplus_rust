# Task: config-logging

**Phase:** 01-foundation
**Status:** Done

## Objective

Configuration loading and structured logging, with the constraint that
neither may become a way for document contents or secrets to reach disk.

## Acceptance criteria

- [x] Configuration resolves from, in precedence order: command-line
      arguments, environment (`BACHELORPAD_*`), a user config file, then
      built-in defaults.
- [x] Config file location follows platform convention — `%APPDATA%` on
      Windows, `$XDG_CONFIG_HOME` (else `$HOME/.config`) on Linux — and can be
      overridden with `BACHELORPAD_CONFIG`.
- [x] A malformed config file produces a clear message naming the file and
      the problem, and the app still starts on defaults. Bad config must not
      make the editor unopenable.
- [x] A malformed file does not discard higher-precedence sources: a broken
      file plus `BACHELORPAD_THEME` still gives you that theme.
- [x] One unknown key does not discard the rest of the file; it is reported
      and ignored.
- [x] An unrecognised theme or renderer name warns and falls back rather than
      refusing to start.
- [x] Problems are reported before any early exit, so they surface however
      the binary is invoked — not only on the path that opens a window.
- [x] Notices reach the user, not just the log: the first appears in the
      status bar at startup.
- [x] Logging via `tracing`, level from `BACHELORPAD_LOG` or the config file,
      defaulting to `warn`.
- [x] **No document content, file contents, clipboard data, passphrases or
      key material is logged at any level.** Enforced by the `log hygiene`
      stage in the local CI gate, which fails the build on any `tracing::`
      macro mentioning a content-bearing identifier.
- [x] Tests covering precedence order, malformed-config recovery, unknown
      keys and values, and that `Config` carries no content-shaped fields.

## Artifacts

- `crates/bp-config`
- `log hygiene` stage in `scripts/Invoke-LocalCI.ps1`

## Notes

`resolve()` is pure — it takes file contents, environment and arguments as
values — so precedence and malformed-input recovery are tested without a
filesystem or a process environment. Only `load()` touches the outside world.

`Config::theme` is a `String` rather than a `bp-theme` enum deliberately. An
unrecognised name in a user's file has to be recoverable, and keeping it
textual means this crate does not need to know which themes exist.

The redaction rule is the substance of this task, not a footnote: logging is
one of the easier ways to defeat the entire security model in ADR-0011. The
grep-based gate is crude and will need widening as more crates land, but it
fails closed and it runs on every commit.
