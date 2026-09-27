# 13-platform

Roadmap phase 18. **Built for RC1; the Windows half has never run on
Windows.** `bp-platform`, `bp-platform-windows` and File ▸ Set as Default
Editor ship, with the per-OS integration of release-plan milestone M2 (M2-02
to M2-08, M2-12, M2-16). Status checked against the code on 2026-09-27.

| Task | Status |
| --- | --- |
| windows-integration | Built, never run on Windows: icon, version resources and manifest (M2-02); `bp-platform-windows`, the one crate allowed to call Win32 (ADR-0033, M2-03). Clippy is clean for the Windows target; the owner's runbook is `docs/release/WINDOWS_BRINGUP.md` |
| linux-integration | Done. Config, state and data directories follow XDG; the packages ship the `.desktop` entry, MIME package and AppStream metadata (M2-05); the app id is `io.github.dboles99.BachelorPad` (M2-07); one clipboard for the process, Wayland data-control and timed clear (M2-08). The window's Wayland `app_id` / X11 `WM_CLASS` is still the binary name, so a dock may not match it to the entry; undecided (`project/WORK_QUEUE.md`, "Follow-ups from M2") |
| default-editor | Done. File ▸ Set as Default Editor shows what opens each file type today. On Linux it runs its commands with consent and reports each result, and Remove Registration takes back what it wrote (M2-06). On Windows the installer owns the registration (M3-01, D8); a registered install gets a Settings deep link, and a portable one gets Save Files…, which writes the `.reg` pair (M2-04). The Windows half has not run on Windows |

`docs/architecture/PLATFORM_MATRIX.md`'s capability table is generated from
`bp_platform::capabilities` and held by a drift test (M2-12).

The three task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).
