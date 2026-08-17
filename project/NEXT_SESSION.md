# Next session plan

Written 2026-08-17, at the end of the first build session.

State: 14 crates, 280 tests, green on Windows and Linux, 13 commits all
through the full local gate. `ROADMAP.md` has per-phase status.

---

## Start here: half an hour of using it

**Before writing any new code.** The single most productive half hour
available is using the application as an editor for real work.

Justification, not a platitude: one five-minute manual pass during the last
session found two defects that 264 tests had not — a menu bar where twelve of
fourteen menus silently swallowed clicks, and Save As defaulting to the
process working directory, which wrote real documents into a git checkout and
then committed them. Neither was the kind of thing another unit test would
have caught. Almost everything built since then has had *less* exercise than
that.

Specifically worth exercising, because it has never been used:

- Clipboard history — copy several things, paste from the menu.
- Recovery — type, don't save, kill the process from Task Manager, relaunch.
- Cross-file search on a real folder, then click through to a hit.
- Data operations on a genuine JSON or CSV file.
- Note ▸ Suggest Title / Summary / Keywords / Outline on a real note.
- Reload from Disk after editing a file in another program.

Write down what feels wrong, not only what breaks. Several of these are
first-draft interactions and the judgement about whether they are *good* is
not one the tests can make.

---

## Then: the custom editor view

This is the highest-value engineering work available, and it has been the
gating item for three sessions' worth of "outstanding" notes.

### Why it matters more than anything else

Slint's `TextInput` owns its own text, caret and undo stack, and exposes the
caret only through a property marked *"internal, undocumented, only exposed
for tests"*. That one fact currently blocks:

- the rope buffer becoming the editor's storage (phase 2, and the prerequisite
  for the whole large-file engine in phase 4);
- `bp-editor`'s undo becoming the authority (phase 2);
- Ln/Col in the status bar (phase 2);
- multi-cursor, column selection, folding, go-to-line — all of specs.md
  section 4, all of which need the caret.

All of it unblocks together. Nothing else on the roadmap has that leverage.

### Why it needs a person present

It is a keyboard-driven widget almost end to end. Building one in an
environment with no interactive desktop means writing a text editor that
cannot be verified — the worst possible ratio of risk to feedback. Plan it for
a session where the keyboard is available and expect to iterate.

### Rough shape

1. Render only the visible lines, from a `bp-buffer::Buffer`.
2. Own the caret and selection in `bp-editor::Editor`.
3. Route keys through a `FocusScope` rather than a `TextInput`.
4. Draw the caret and selection.
5. Map clicks and drags to character offsets.
6. Wire `bp-editor`'s undo, delete the widget's.
7. Ln/Col from `Editor::position()`, which already exists and is tested.

Steps 1–2 are already written and tested; the risk is entirely in 3–5.

### Evidence it is worth it

`bachelorpad --latency-probe`:

| Document | insert via `String` | insert via rope |
| ---: | ---: | ---: |
| 100 KB | 55.8 µs | 0.3 µs |
| 1 MB | 563.7 µs | 0.3 µs |

Linear versus flat. The current path holds inside a frame budget to ~40 MB, so
this is not urgent for notes — it is the prerequisite for large files.

---

## Alternative if the editor view is too large for the session

Any of these are self-contained and need no interactive verification:

| Work | Phase | Notes |
| --- | --- | --- |
| Metadata store (`bp-storage`) | 9 | SQLite + migrations. Unblocks related notes, tags, duplicate detection, revision history. Largest downstream payoff after the editor view. |
| Replace preview | 7 | specs.md wants changes shown before applying. Currently Replace All just does it. |
| Markdown preview | 5 | Needs a rendering decision first — Slint has no HTML view. |
| CSV with quoted fields | 6 | Current shape report miscounts a field containing a comma. Needs a real CSV reader. |
| Paste transformations | 11 | Kind detection already identifies entries; nothing acts on it. |
| Line operations | 2 | Duplicate/move/sort/remove-duplicates — some are possible without caret access. |

---

## Decisions to make before the work they block

These want a human, not a default:

- **`.bpadx` envelope format** (phase 15). ADR-0011 forbids custom
  cryptography, which constrains this more than it looks. Crate choice,
  envelope layout and version scheme should be agreed before code exists.
- **What may leave the machine** (phases 8 and 10). Generative providers and
  embeddings both imply sending document content somewhere. ADR-0006 keeps
  them optional; nothing says which providers are acceptable.
- **YAML parser** (phase 6). `serde_yaml` is deprecated; the replacement is a
  long-lived dependency choice.
- **Security profiles** (phase 14) are quietly urgent. Two shipped features
  already hold sensitive data — the recovery journal writes unsaved text to
  disk in plaintext, clipboard history keeps copied secrets in memory — and
  both are waiting on a subsystem that does not exist. Neither is wrong today,
  but the longer they run unqualified the more surprising they become.

---

## Housekeeping worth doing at some point

- **Native Linux timing.** WSLg's compositor makes startup numbers
  unrepresentative, so ADR-0015's evidence remains Windows-only.
- **Scroll smoothness under CPU rasterisation** is the standing revert
  condition on ADR-0017 and needs a capture rig.
- **`bp-ui` is approaching 1,500 lines.** It has earned a split — menus are
  already out; state, dispatch and the timers could follow.
- **Idle memory drifted from ~20 MB to ~32 MB** in one session after dialogs
  had been opened. Below the 50 MB target and possibly nothing, but it has
  been observed twice and never investigated.

---

## How to work in this repository

- `./scripts/Invoke-LocalCI.ps1 -Linux` is the gate. There is no hosted CI
  (ADR-0016) and Actions is disabled at the repository level.
- `./scripts/Install-GitHooks.ps1` once per clone; the hooks run the gate.
- Every commit in this repository has passed the gate. Keep it that way.
- `docs/decisions/` is the register; add an ADR when a decision has
  consequences beyond one file.
