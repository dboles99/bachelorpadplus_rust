# Task: rope-buffer

**Phase:** 02-core-editor
**Status:** Crate done; not yet the editor's storage (see Notes)

## Objective

A text buffer where an edit in the middle of a large document costs
`O(log n)` rather than copying the whole thing (specs.md section 5), with the
line and column maths every other feature depends on.

## Acceptance criteria

- [x] Rope-backed buffer (`ropey` 1.x — the stable line, not 2.0.0-beta).
- [x] Indices are **character** indices, not bytes. A byte index can land
      inside a multi-byte character; this type is reached from UI code.
- [x] 1-based line/column positions, matching what the status bar displays.
- [x] Position round-trip: `char_of_position(position_of(i)) == i` for every
      index in a document, asserted exhaustively.
- [x] A trailing newline creates a line. `"a\n"` is two lines, not one —
      `str::lines()` disagrees and is wrong for display.
- [x] Out-of-range indices, lines and columns clamp rather than panic.
- [x] A column cannot land past the end of its line.
- [x] Line length excludes the line break, including CRLF.
- [x] `line_count(&str)` free function for callers that need only the count,
      so the gutter does not build a rope on every keystroke — with a test
      asserting it never disagrees with the rope.
- [ ] Used as the editor's text storage. Blocked on the editor view; see
      Notes.

## Measurements

`bachelorpad --latency-probe`, release, Windows. Insert at the caret:

| Document | via `String` (today) | via rope |
| ---: | ---: | ---: |
| 1 KB | 0.8 µs | 0.2 µs |
| 10 KB | 5.8 µs | 0.2 µs |
| 100 KB | 55.8 µs | 0.3 µs |
| 1 MB | 563.7 µs | 0.3 µs |

The string path is linear in document size; the rope is flat. At 1 MB the
difference is roughly 1900×.

## Notes

**The rope is not yet the editor's storage, deliberately.** Slint's
`TextInput` owns its own text and caret: it hands back the entire buffer on
every edit and exposes the caret only through a property marked "internal,
undocumented, only exposed for tests". Putting a rope behind that widget means
converting rope→`String` on every push and back on every edit — strictly worse
than the `String` it replaces, in exchange for architectural tidiness.

The rope pays off when the editor view owns its own text and rendering. That
is a custom, virtualised, keyboard-driven widget, and it cannot be built
responsibly in an environment with no interactive desktop to press a key in.
So the buffer is built, tested and ready; the switch happens with the view.

The measurements above are the argument for making that switch, and the
reason to make it before the large-file work in phase 4 rather than after.
