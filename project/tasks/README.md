# project/tasks

**The task list is [`project/RELEASE_PLAN.md`](../RELEASE_PLAN.md), not this
folder.** Each plan item (M0-01, M1-04, ...) names its files, size,
dependencies and a **Verify** column: the test, gate stage or measurement that
proves it done. That column is the acceptance criteria ADR-0014 asks every
task to carry (see its 2026-09-25 amendment). New work is added there.

What stays here:

- one README per folder, giving the folder's status on 2026-09-25 and the
  plan items or ADRs where its remaining work went;
- the six task files from phases 1 and 2 that were written with real
  acceptance criteria, kept as the record of what those phases accepted:
  [workspace-scaffold](01-foundation/01-workspace-scaffold.md),
  [ui-prototype](01-foundation/02-ui-prototype.md),
  [ci-baseline](01-foundation/03-ci-baseline.md),
  [config-logging](01-foundation/04-config-logging.md),
  [rope-buffer](02-core-editor/01-rope-buffer.md) and
  [undo-redo](02-core-editor/05-undo-redo.md).

**Deleted on 2026-09-25 (release plan M2-13):** 52 task files that were the
repository scaffold's template with only the task name changed -- the same
seven generic steps, no acceptance criteria, no status and nothing specific
to the task. ADR-0014 calls a task file without acceptance criteria
incomplete. Filling them in would have copied the release plan's Verify
column into a second place to drift, and for the deferred ones it would have
invented criteria for work nobody has planned. `git log --diff-filter=D --
project/tasks` recovers them.

## Folders, roadmap phases and where the work is

Folder numbers are a coarser grouping of the roadmap's 20 phases (see
ROADMAP.md, "Two numbering schemes, reconciled").

| Folder | Roadmap phases | Status (2026-09-25) | Where the rest is |
| --- | --- | --- | --- |
| [01-foundation](01-foundation/) | 1 | Done | Native Linux timing: M4-01 |
| [02-core-editor](02-core-editor/) | 2 | Done for RC1 except the editor context menu | Context menu: M1-18 (waits on U5); multi-cursor, column selection, folding: 1.x (plan §3) |
| [03-file-safety](03-file-safety/) | 3 | Done | Revision history: deferred to 1.x (plan §3) |
| [04-large-files](04-large-files/) | 4 | Partly: engine built, not reachable | Viewer: M1-06 (waits on U3); memory mapping declined for good (ADR-0027, ADR-0030) |
| [05-formats](05-formats/) | 5, 6 | Done for RC1 | Highlighting: 1.x (ADR-0030 S6); Markdown preview: U4 |
| [06-search](06-search/) | 7 | Done for RC1 except two items | Line definition: M1-09 (D6); search inside the large-file viewer: M1-06; fuzzy search, replace in files: 1.x |
| [07-semantic](07-semantic/) | 8, 9, 10 | Phase 8 done; 9 built and unwired; 10 not started | Phases 9 and 10: 1.2 (ADR-0030 S2, D3) |
| [08-clipboard](08-clipboard/) | 11 | Partly | Context menu and pin: M1-18 (U5); timed clear: M2-08; Windows native history: 1.x |
| [09-notebook](09-notebook/) | 12 | Deferred to 1.1 | ADR-0030 S1 |
| [10-research](10-research/) | 13 | Built and unwired; no target release | ADR-0030 |
| [11-security](11-security/) | 14, 15, 16 | Mostly done | M1-19 header half, M2-09, M2-10, M2-11; OS keystores 1.x (ADR-0030 S4) |
| [12-themes-brand](12-themes-brand/) | 17 | Partly | Icon: M2-01; accessibility: M2-14; personality: 1.x (ADR-0030 S5) |
| [13-platform](13-platform/) | 18 | Partly | M2-02 to M2-08, M2-12, M2-16 |
| [14-hardening](14-hardening/) | 19, 20 | Partly | M1-14, M3, M4 |
