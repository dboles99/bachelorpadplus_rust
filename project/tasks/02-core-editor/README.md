# 02-core-editor

| Task | Status |
| --- | --- |
| [rope-buffer](01-rope-buffer.md) | Crate done and tested; not yet the editor's storage |
| [tabs](02-tabs.md) | Delivered in phase 1 (`bp-core::Workspace` + the tab strip) |
| [file-io](03-file-io.md) | Delivered in phase 1 (`bp-files`: atomic save, load, encodings) |
| [status-bar](04-status-bar.md) | Delivered in phase 1 except Ln/Col |
| [undo-redo](05-undo-redo.md) | Crate done and tested; not yet wired |

## The one thing blocking this phase

**Slint's `TextInput` owns the text and the caret.** It hands back the whole
buffer on every edit, keeps its own undo stack, and exposes the caret only
through a property marked "internal, undocumented, only exposed for tests".

That single fact blocks three separate items: the rope cannot become the
storage, our undo cannot become the authority, and the status bar cannot show
Ln/Col. All three unblock together, and only together, when the editor view
owns its own text.

So `bp-buffer` and `bp-editor` are built, tested and idle — which is the right
order. The semantics are pinned down and provable now; the widget that
consumes them is the risky part.

## Next: the editor view

A custom, virtualised text view — we render the visible lines, own the caret,
and route keys ourselves. It is the largest single piece of work in the
project so far and it is almost entirely keyboard-driven, so it needs an
environment where a key can actually be pressed to verify it. That is the
gating constraint, not the code.

Evidence that it is worth doing, from `--latency-probe`:

| Document | via `String` (today) | via rope |
| ---: | ---: | ---: |
| 100 KB | 55.8 µs | 0.3 µs |
| 1 MB | 563.7 µs | 0.3 µs |

Linear versus flat. The current path stays under a frame budget to roughly
40 MB, so this is not urgent for notes — it is the prerequisite for the
large-file work in phase 4.
