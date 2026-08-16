# 01-foundation

| Task | Status |
| --- | --- |
| [workspace-scaffold](01-workspace-scaffold.md) | Core crates done; 24 crates still placeholders |
| [ui-prototype](02-ui-prototype.md) | Spike done; shell blocked on BP-ADR-0015 sign-off |
| [ci-baseline](03-ci-baseline.md) | Done on Windows; Linux leg needs a WSL toolchain |
| [config-logging](04-config-logging.md) | Not started |

## Phase exit criteria

The phase is done when the app opens a window, opens a file into a tab, saves
it atomically, shows honest save state in the status bar, and does all of that
on both Windows and Linux — with the local gate green on both.

## Open items blocking the phase

- **BP-ADR-0015 (UI toolkit) is Proposed, not Accepted.** Part B of the
  ui-prototype task should not start until it is settled.
- **specs.md section 22 startup targets are unmet by any candidate toolkit**,
  by roughly 6-8x. Needs its own decision before the shell is built against
  an assumption that cannot hold.
- **No Linux verification exists yet.** BP-ADR-0001 makes Linux first-class,
  and nothing currently checks it.
