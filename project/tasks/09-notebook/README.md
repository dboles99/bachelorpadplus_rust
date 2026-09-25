# 09-notebook

Roadmap phase 12. **Deferred to 1.1** by ADR-0030 (S1). Status checked
against the code on 2026-09-25.

| Task | Status |
| --- | --- |
| cell-model | Built in `bp-notebook` (cells, notebook operations); unwired |
| ipynb | Built in `bp-notebook` (Jupyter import and export); unwired. In RC1 an `.ipynb` file opens as JSON |
| runners | Not built. Execution belongs to a `bp-execution` crate that does not exist (ADR-0025) |
| outputs | Output model built in `bp-notebook`; no rendering, because nothing runs |
| execution-security | The never-auto-run rule is enforced in `bp-notebook`'s API (a run needs a `UserGesture` no parsed file can produce); the process-level half waits on `bp-execution` |

The five task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).

Nothing in the application depends on `bp-notebook`; the fuzz harnesses and
the cross-crate tests do. The Notebook and Run menus are hidden unless
`show_planned_menus = true` (M1-05).
