# 02-core-editor

Roadmap phase 2. **Crates built and tested; wiring blocked.**

| Task | Status |
| --- | --- |
| [rope-buffer](01-rope-buffer.md) | Crate done; not yet the editor's storage |
| [tabs](02-tabs.md) | Done in phase 1 (`bp-core::Workspace` + tab strip) |
| [file-io](03-file-io.md) | Done in phase 1 (`bp-files`) |
| [status-bar](04-status-bar.md) | Done except Ln/Col |
| [undo-redo](05-undo-redo.md) | Crate done; not yet the authority |

## The one thing blocking this phase

**Slint's `TextInput` owns the text and the caret.** It hands the whole buffer
back on every edit, keeps its own undo stack, and exposes the caret only
through a property marked *"internal, undocumented, only exposed for tests"*.

That single fact blocks three separate items:

- the rope cannot become the storage — a rope behind that widget means
  converting to `String` on every push, which is worse than the `String` it
  would replace;
- `bp-editor`'s undo cannot become the authority — two undo stacks over one
  document is worse than one;
- the status bar cannot show Ln/Col.

All three unblock together, with a custom virtualised editor view that owns
its own text and routes keys itself.

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
