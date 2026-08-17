# Next session plan

Written 2026-08-17, at the end of the third build session.

State: 15 crates, 542 tests, 26 commits, every one through the full gate,
green on Windows and Linux.

---

## Start here: ten minutes with `--editor-view`

```powershell
cargo run --release -- --editor-view
```

This has been the first item for two sessions running and it has not been
done. It is now the single largest thing standing between this project and
knowing whether it works.

The custom editor view (ADR-0018) has 147 tests behind its *rules* — motion,
word boundaries, chords, tab stops, the involution property that caught two
real defects. Its *behaviour in a window* has been verified to exactly one
standard: it renders a frame with a real file open and exits without
panicking.

Everything below is unverified, and each is seconds:

| Check | What would be wrong |
| --- | --- |
| Type a character | Keys not reaching the `FocusScope` at all |
| Click mid-line | Caret lands left or right of the pointer — the measured advance is off |
| Click at the end of a long line | Advance error accumulating across columns |
| Arrow down through a short line and back up | The goal column not surviving |
| Drag to select | Selection rectangles misplaced, or drag not extending |
| Scroll the wheel | Wrong direction, or snapping back to the caret |
| **Ctrl+S** | The surface swallowing chords it should reject — this would stop saving |
| Ctrl+D, Alt+Up, Alt+Down | Duplicate and move line, shipped this session, never run |
| Watch the status bar | Ln/Col is the payoff; it should track the caret |
| Open a tab-indented file | Tab stops wrong, caret off by columns |
| Resize the window | Row count not following, blank strip at the bottom |

**The advance measurement and the wheel direction are the two most likely to
be subtly wrong.** Both are single-line fixes once someone has looked.

Write down what feels wrong, not only what breaks. The default is still
`TextInput`, so nothing is lost if the answer is bad — but five features
shipped in the last two sessions are enabled only here, and they all inherit
whatever that caret does.

---

## Then, in order of value

### 1. Wire the five built-but-unreachable things

Each is a tested library function with no menu row. They are the cheapest
work available and `project/WORK_QUEUE.md` has the specifics.

| Row | Function that already exists |
| --- | --- |
| Zoom (Ctrl+= / Ctrl+- / Ctrl+0) | `Config::font_size` |
| Indentation (tab width, spaces or tabs) | `Config::{tab_width, indent_spaces}` |
| Insert ▸ Date / Time / ISO 8601 | `bp_naming::Stamp::all()` |
| Document statistics | `bp_semantic::statistics` |
| Go to Line | `Editor::go_to_line` |

Two more Tier 1 items need no library work at all: the find box's
case-sensitive / whole-word / **regex** toggles (regex search is fully
implemented and unreachable), and double- and triple-click selection.

**They no longer all queue behind one file.** `ui/app.slint` has been split
the way `lib.rs` was — seven files, listed in
`docs/architecture/ARCHITECTURE.md` — so the find bar, the editor surface, the
status bar, the results panel and the tab are each somebody's own file.

What still serialises is narrower but real: `AppWindow`'s properties and
callbacks stay in `app.slint`, because that is the API Rust binds to, and so
do the menu bar, the popups, both editor views and the tab strip. An item that
adds a property still edits that file; it just no longer edits twelve hundred
lines to do it. `project/WORK_QUEUE.md` has the rule.

### 2. Parity, or retire the flag

`--editor-view` is temporary and should not become furniture. Two things
stand between it and being the default:

1. **Word wrap.** The deep one: it makes a visual line differ from a document
   line, which every function in `bp_editor::view` assumes away, and it
   changes what Up and Down mean.
2. **Input-method composition.** Slint delivers pre-edit text through
   `KeyEventType::UpdateComposition`, which a bare `FocusScope` never sees.
   Without it CJK entry does not work at all. Investigate before writing code.

If those look worse than they are worth, the honest outcome is to delete the
surface and keep the rope. The storage change is the valuable half and it is
already unconditional.

### 3. Security profiles (phase 14)

Now the largest structural gap, and it has three dependants rather than two:
the recovery journal writing unsaved text to disk in plaintext, clipboard
history holding copied secrets in memory, and `bp-storage` — built, tested,
and deliberately unreachable because recording a document's extracted title
puts a plaintext index of what the user writes about on disk.

The first two are *running*. The third is *waiting*, which is the better
failure mode and the reason it was left unwired.

---

## Decisions still waiting on a human

- **`.bpadx` envelope format** (phase 15). ADR-0011 forbids custom
  cryptography, which constrains this more than it looks: crate, envelope
  layout and version scheme want agreeing before code exists.
- **What may leave the machine** (phases 8 and 10). Generative providers and
  embeddings both imply sending document content somewhere. Nothing says
  which providers are acceptable.
- **YAML parser** (phase 6). `serde_yaml` is deprecated; the replacement is a
  long-lived dependency choice.
- **Whether `bp-storage` may record titles before phase 14.** Currently no,
  by ADR-0019. That is a decision, not an oversight, and reversing it is
  allowed — but deliberately.

---

## Working with agents in this repository

Three sessions of evidence, worth reading before dispatching any:

- **`prompts/rosettas/R011_WIRE_CAPABILITY_INTO_SHELL.md` earns its keep.** An
  agent that read it found and fixed an unbounded dispatch range that would
  have swallowed its own new ids.
- **Give one crate per agent.** Four ran concurrently without collision when
  each owned one crate. The moment two want `bp-ui` or `app.slint`, serialise.
- **Ask for a property, not more examples.** Every defect found by review this
  session was found by a property — "moving a line down and back returns the
  exact original bytes" caught two that example-based unit tests written
  alongside the feature missed entirely.
- **Tell them to report problems outside their scope rather than fix them.**
  Both times an agent fixed something adjacent, it was wrong to: a JSON
  reformatter that duplicated `bp-data` and disagreed with it on key order,
  and the removal of the launch stage's timeout inside a "pure refactor"
  commit.
- **`scripts/Invoke-LocalCI.ps1` changes are their own commit, never a
  passenger.** It is the thing that validates everything else.
- **Review the substance, not the report.** Agents self-reported all green in
  every case where a real defect was present.

---

## Housekeeping

- ~~**`ui/app.slint` wants splitting.**~~ **Done** — seven files. What is left
  in it, and why each thing stayed, is in `docs/architecture/ARCHITECTURE.md`.
  The popups were the one piece deliberately not moved: a `PopupWindow` is
  positioned relative to its parent element, so moving them would change the
  coordinate parent, and nothing in the gate can see a menu opening in the
  wrong place.
- **Native Linux timing.** WSLg's compositor makes startup numbers
  unrepresentative, so ADR-0015's evidence remains Windows-only.
- **Scroll smoothness under CPU rasterisation** is the standing revert
  condition on ADR-0017 and needs a capture rig. The custom view makes it more
  pressing: it redraws a set of `Text` elements per scroll.
- **Idle memory drifted from ~20 MB to ~32 MB** in one session after dialogs
  had been opened. Below the 50 MB target, observed twice, never investigated.
- **A clipboard menu row decodes its id against the history at click time**,
  not when the menu was built, and a 1.2 s poll can shift it in between.
  Pre-existing and applies to plain paste too; fixing it means freezing a
  snapshot while a menu is open.

---

## How to work in this repository

- `./scripts/Invoke-LocalCI.ps1 -Linux` is the gate. There is no hosted CI
  (ADR-0016) and Actions is disabled at the repository level.
- `./scripts/Install-GitHooks.ps1` once per clone; the hooks run the gate.
- Every commit in this repository has passed the gate. Keep it that way.
- `docs/decisions/` is the register; add an ADR when a decision has
  consequences beyond one file.
- **Do not edit files while the gate is running.** The Linux leg re-runs
  `cargo fmt --check` from scratch, and a mid-run edit fails it for a reason
  that has nothing to do with the change.
