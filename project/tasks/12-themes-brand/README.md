# 12-themes-brand

Roadmap phase 17. **Done for RC1.** Themes ship, the icon set is approved
(M2-01) and the accessibility minimum is in (M2-14); personality modes are
deferred to 1.x (ADR-0030, S5). Status checked against the code on
2026-09-27.

| Task | Status |
| --- | --- |
| themes | Done — `bp-theme`: Light, Dark, Organic and Green (ADR-0009) plus Follow System, which says when the desktop does not report light or dark (M1-21); the choice is saved to `config.toml` (M1-03) |
| personality | Not built; 1.x (ADR-0030, S5). ADR-0013's Standard and Minimal modes do not exist yet |
| accessibility | Done for RC1 (M2-14): F10 and Alt+letter open the menus, arrows, Enter and Esc work in them, and roles and labels are on the menu bar, rows, tabs, bars and status (the notice is a polite live region). Slint 1.17 has no menu roles, so a row is a list item. A screen-reader check is a human step in the RC checklist (M4-04) |

Brand: `docs/brand/BRAND_GUIDE.md` and the icon set it describes
(`assets/icon/`: SVG, hicolor PNGs 16–512, a multi-size `.ico`), approved by
the owner (M2-01).

The three task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).
