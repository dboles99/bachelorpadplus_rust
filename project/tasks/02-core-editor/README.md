# 02-core-editor

Roadmap phase 2. **Done for RC1 except the editor context menu** (release-plan
M1-18, waiting on U5). Status checked against the code on 2026-09-25.

| Task | Status |
| --- | --- |
| [rope-buffer](01-rope-buffer.md) | Done: a `bp-buffer` rope holds every document |
| tabs | Done in phase 1 (`bp-core::Workspace` + tab strip); Ctrl+Tab since M1-11 |
| file-io | Done in phase 1 (`bp-files`) |
| status-bar | Done, with Ln/Col in both views (M1-04) |
| [undo-redo](05-undo-redo.md) | Done under `--editor-view`; the default view keeps `TextInput`'s own undo |

The tabs, file-io and status-bar task files were empty templates and were
deleted on 2026-09-25 (see [../README.md](../README.md)).

## What changed the picture

This folder used to say the phase was blocked on one fact: Slint's
`TextInput` owns the text and the caret. Two things answered it.

- **A custom editor view** (ADR-0018) owns its text and routes keys itself,
  so under `--editor-view` the rope is the text and `bp-editor`'s undo is the
  authority. It ships as experimental, behind the flag (D2); it cannot do
  input-method composition on Slint 1.17.1.
- **The default view reads `TextInput`'s caret** (M1-04), converting bytes to
  characters through the rope, so Ln/Col, Go to Line, Insert Date/Time,
  Duplicate Line and Move Line Up/Down work in the view that ships. Its text
  is copied into the rope after every edit.

Line operations (sort, remove duplicates, reverse, trim, duplicate, move),
Go to Line and zoom are all reachable from the menus.

## Evidence for the rope

`bachelorpad --latency-probe`, release, Windows, measured in phase 2:

| Document | insert via `String` | insert via rope |
| ---: | ---: | ---: |
| 100 KB | 55.8 µs | 0.3 µs |
| 1 MB | 563.7 µs | 0.3 µs |

## Not in RC1

Multi-cursor, column selection, folding and indentation guides (specs.md
section 4) are deferred to 1.x by the release plan (§3). Drag and drop to
open is blocked on Slint (README, "Known gaps").
