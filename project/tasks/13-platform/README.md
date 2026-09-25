# 13-platform

Roadmap phase 18. **Partly done.** `bp-platform` and File ▸ Set as Default
Editor ship; the per-OS integration that makes the app look and register
like a native application is release-plan milestone M2 (M2-02 to M2-08,
M2-12, M2-16). Status checked against the code on 2026-09-25.

| Task | Status |
| --- | --- |
| windows-integration | Not built. Icon, version resources and manifest: M2-02. `bp-platform-windows`, the one crate allowed to call Win32 (ADR-0033): M2-03. Nothing here has run on Windows 11, and the latest work has not run on Windows 10 (M0-12) |
| linux-integration | Partly. Config, state and data directories follow XDG; no `.desktop` or AppStream files are shipped (M2-05), the app id is not yet `io.github.dboles99.BachelorPad` (M2-07, D9 answered), and clipboard work (one clipboard for the process, Wayland data-control, timed clear) is M2-08 |
| default-editor | Partly. File ▸ Set as Default Editor shows what opens each file type today, writes a `.desktop` entry and MIME package on Linux or saves a `.reg` file on Windows with consent, and runs no command itself (ADR-0012). The Windows installer-owned registration is M2-04 and M3-01 (D8 answered); running `xdg-mime` with consent and unregistering are M2-06 |

`docs/architecture/PLATFORM_MATRIX.md` is hand-written; generating it from
`bp_platform::capabilities` with a drift test is M2-12.

The three task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).
