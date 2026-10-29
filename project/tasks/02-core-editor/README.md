# 02-core-editor

Roadmap phase 2. **Done.**

| Task | Status |
| --- | --- |
| [rope-buffer](01-rope-buffer.md) | Crate done; editor's storage via custom surface |
| [tabs](02-tabs.md) | Done in phase 1 (`bp-core::Workspace` + tab strip) |
| [file-io](03-file-io.md) | Done in phase 1 (`bp-files`) |
| [status-bar](04-status-bar.md) | Done, including Ln/Col (ADR-0084) |
| [undo-redo](05-undo-redo.md) | Crate done; authority via custom surface |

## Blocking issue resolved

The custom editor surface ([ADR-0080](../docs/decisions/ADR-0080.md)) owns
its own text and caret, unblocking the three items previously blocked by
Slint's `TextInput`:

- the rope **is** the editor's storage — `draw_editor_view` passes
  `editor.buffer()` to `bp_editor::view::visible_rows`;
- `bp-editor`'s undo **is** the authority — menu UNDO/REDO dispatch to
  `bp_editor::Command::Undo`/`bp_editor::Command::Redo`;
- the status bar **does** show Ln/Col — `caret_byte_offset` in AppState
  captured during refresh and converted to position (ADR-0084).

Building the libraries first was the right order: their semantics are provable
without a window, and the widget is the risky part. `bp-editor` has 19 tests
pinning undo behaviour — typing coalesces into one step, newlines and caret
moves end the run, replacing a selection undoes as one — none of which needed
a window to establish.

## Evidence that the switch is worth making

`bachelorpad --latency-probe`, release, Windows:

| Document | insert via `String` | insert via rope |
| ---: | ---: | ---: |
| 100 KB | 55.8 µs | 0.3 µs |
| 1 MB | 563.7 µs | 0.3 µs |

Linear versus flat — roughly 1900× at 1 MB. The current path stays inside a
16 ms frame budget to about **40 MB**, so this is not urgent for notes. It is
the prerequisite for the large-file work in phase 4.

## Also outstanding

Multi-cursor, column selection, folding, indentation guides, line operations
(duplicate/move/sort/remove-duplicates), go-to-line and zoom — specs.md
section 4. All of them want the custom view too, since they need the caret.
