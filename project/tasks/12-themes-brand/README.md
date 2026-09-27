# 12-themes-brand

Roadmap phase 17. **Partly done.** Themes ship; the icon set (M2-01) and the
accessibility minimum (M2-14) are open for RC1; personality modes are
deferred to 1.x (ADR-0030, S5). Status checked against the code on
2026-09-25.

| Task | Status |
| --- | --- |
| themes | Done — `bp-theme`: Light, Dark, Organic and Green (ADR-0009) plus Follow System, which says when the desktop does not report light or dark (M1-21); the choice is saved to `config.toml` (M1-03) |
| personality | Not built; 1.x (ADR-0030, S5). ADR-0013's Standard and Minimal modes do not exist yet |
| accessibility | Partly: Slint's AccessKit support is compiled in, and the dialog, About and Settings sheets set roles; menus, tabs and the status bar do not, and F10 does not open the menu bar. Release-plan item M2-14. A screen-reader check is a human step in the RC checklist (M4-04) |

Brand: `docs/brand/BRAND_GUIDE.md` exists; the icon set it describes does
not yet (M2-01, which needs the owner's approval of the artwork).

The three task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).
