# Task: undo-redo

**Phase:** 02-core-editor
**Status:** Crate done; not yet wired (see Notes)

## Objective

Undo and redo that step the way a user expects, not the way the keystrokes
happened to arrive.

## Acceptance criteria

- [x] Transaction-based history, not keystroke-based.
- [x] **A run of ordinary typing is one undo step.** Typing "hello" and
      pressing undo removes "hello", not "o".
- [x] A newline ends the run, so undo steps back a line at a time rather than
      swallowing a paragraph.
- [x] Moving the caret ends the run.
- [x] Typing at a non-contiguous position does not merge, even when nothing
      else intervened — tested independently of the caret-move path.
- [x] Deleting starts its own entry rather than joining the typing before it.
- [x] Replacing a selection is one step: the delete and the insert undo
      together.
- [x] Undo restores the caret to where the edit began; redo restores it to
      where the edit ended.
- [x] A new edit discards the redo branch.
- [x] Undo and redo report whether they did anything, instead of silently
      doing nothing.
- [x] A no-op (backspace at position 0, inserting "") creates no undo entry.
- [x] Multi-byte text undoes by character, not byte — a byte-length undo
      leaves fragments of a character behind.
- [x] Loading a file clears history: undoing past a file load into the
      previous document's content is nonsense.
- [x] Full round trip — undo everything, redo everything — returns the exact
      text at both ends.
- [ ] Wired to Ctrl+Z / Ctrl+Y in the shell. Blocked; see Notes.

## Notes

Not wired for the same reason the rope is not yet the editor's storage:
Slint's `TextInput` has its own undo stack and its own caret. Two undo stacks
over one document is worse than one, and ours cannot become the authority
until the editor view owns the text.

`bp-editor` has no UI dependency, so all of the above is tested directly
rather than by driving a window — which is the only reason these semantics
could be pinned down at all in an environment with no interactive desktop.
