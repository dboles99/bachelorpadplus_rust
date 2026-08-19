# Work Queue

Bounded, independent pieces of work, each sized so that a competent coding
agent can finish one without holding the whole product in its head.

`project/NEXT_SESSION.md` says what matters most. This says what is *ready*,
which is not the same list.

## How to take an item

1. Read `prompts/rosettas/R011_WIRE_CAPABILITY_INTO_SHELL.md` first if the
   item touches `crates/bp-ui/`. It encodes the traps that have already cost
   somebody a session — action-id ranges, the two editor views, Slint `if`
   scoping.
2. Take the whole item. Each one below is already the smallest thing worth
   doing; splitting one further produces a half-feature.
3. Run the gate before saying it is done:
   `./scripts/Invoke-LocalCI.ps1 -Linux`

## Collision rules

`bp-ui/src/lib.rs` has been split, so `state.rs`, `editor_view.rs` and
`dispatch.rs` can take different agents. `ui/app.slint` has now been split too
— seven files, listed in `docs/architecture/ARCHITECTURE.md`. So the find
bar, the editor surface, the status bar, the results panel and the tab are
each somebody's own file.

**That is a smaller win than it looks, and the reason is worth knowing.**
`AppWindow`'s properties and callbacks all still live in `app.slint`, because
that is the API Rust binds to. Any item that adds a property or a callback
still edits that file — it just edits twenty lines of declaration rather than
twenty lines buried in twelve hundred. So:

> **Two agents may work in different `ui/*.slint` component files at once.
> Only one may add to `AppWindow` in `app.slint` at a time.**

Still wholly inside `app.slint`, and so still fully serialised: the menu bar,
the menu popups, both editor views and the tab strip.

Batching by menu so one agent does several related rows in a pass is usually
faster than queueing anyway.

Library items are one crate each and parallel-safe with everything.

Nobody edits the workspace `Cargo.toml`, `README.md`, `ROADMAP.md`,
`DECISIONS.md` or `docs/decisions/` concurrently — those are shared and small,
so whoever finishes an item updates them as part of it.

---

## Done

| # | Item | Landed as |
| --- | --- | --- |
| T0 | Split `bp-ui/src/lib.rs` into `state` / `editor_view` / `dispatch` | `b6e2544` |
| T1 | Split `bp-ui/ui/app.slint` into seven component files | `0215d29` |
| W1 | Duplicate Line, Move Line Up / Down | Edit menu, Ctrl+D / Alt+Up / Alt+Down |
| W2 | Paste transformations | Edit menu, per clipboard entry |
| W3 | CSV to JSON / JSON Lines / Column Types | Data menu, Csv and Tsv only |
| W5 (library half) | `Editor::go_to_line`, `go_to_position` | `bp-editor` |
| W8 (library half) | `bp_naming::{Stamp, render}` | `bp-naming` |
| W10 (library half) | `bp_semantic::statistics` | `bp-semantic` |
| W14 (library half) | `tab_width`, `indent_spaces` | `bp-config` |
| W4 | Find-box case / whole-word / regex toggles | Find bar |
| W6 | Zoom | View menu, Ctrl+= / Ctrl+- / Ctrl+0 |
| W7 | Double- and triple-click selection | `editor_surface.slint` |
| W5 | Go to Line | Edit menu, Ctrl+G, `ui/goto_bar.slint` |
| W8 | Insert Date / Time | Insert menu, five rows from `Stamp::all()` |
| W9 | Save a Copy | File menu |
| W10 | Document statistics | Note menu, into the status bar |
| W12 | Tab context menu | Right-click a tab |
| W13 | Follow System theme | View menu, `ThemeId::for_system` |
| W14 | Indentation settings | Format menu, `bp_editor::Indent` |
| — | YAML, bounded before a tree exists | ADR-0023, `bp-data` |
| — | Document hashes and detached Ed25519 signatures | `bp-crypto::sign` |
| — | Privacy Mode, and a secret scanner that never holds a secret | `bp-security`, `bp-secrets` |
| — | The large-file engine | ADR-0027, `bp-buffer` |
| — | Notebooks, research, redaction, audit, integrity | ADR-0024/0025/0026/0028 |
| — | Scan for Secrets, Hash, Verify Signature | Security menu, ids 200-203 |
| — | Cross-crate tests, and the seven defects they found | `tests/integration` |
| — | JSONL sniffing: a pretty-printed array is not JSON Lines | `bp-formats` |
| — | The encoder returns to a library; UTF-16 loads and saves | `bp-files` |
| — | The audit log's sealed destination gets an implementor | `bp-audit` |
| — | The platform seam, with ADR-0012 mechanised | `bp-platform` |
| — | Data ▸ YAML, Security ▸ Redact, Security ▸ Inspect Metadata | ids 204-205, 210-211 |
| — | File ▸ Set as Default Editor, which reports before it offers | ADR-0012, File menu, id 220 |
| — | Security ▸ Verify Signature, against `bp-integrity`'s sidecar | ADR-0026, id 203 |
| — | `bp-naming` tests the *stem* for a device name, as Win32 does | `bp-naming`, and a two-way list agreement test |

Every library half now has its row.

## Tier 0 — four crates with no way in

Still the largest block of ready work, and every item is the same shape: a
tested library and a missing menu row. Read
`prompts/rosettas/R011_WIRE_CAPABILITY_INTO_SHELL.md` before taking any of
them. **Only one agent may add to `AppWindow` in `app.slint` at a time**, so
batch by menu.

| Item | What exists |
| --- | --- |
| The open path uses the large-file engine | `bp_buffer::{LargeFile, SizeClass, Access}` (ADR-0027). Opening a 2 GB file currently loads 2 GB. Needs a read-only-because-huge state in the shell, which is a real UI decision, so this is the biggest of the wiring items rather than the cheapest |
| Security ▸ Audit history | `bp-audit` (ADR-0024) now has a working sealed destination, so the crate is complete. What is missing is the view and the calls that record events -- every capability that should write one already exists |
| Notebook mode | `bp-notebook` (ADR-0025). A new view, not a menu row |
| Research mode | `bp-research`. Same: a mode, not a row |

## Tier 0.5 — what the cross-crate tests found

Eleven files of cross-crate tests have now found seven defects. Five are
fixed -- the last of them was the one that destroyed documents, and it is in
the Done table above. These are the rest, each pinned by an `#[ignore]`d test
naming it. **Take the first one before anything else in this file.**

| Item | Shape |
| --- | --- |
| **`bp-notebook` runs unnameable code through the kernel's interpreter** | `infer_kind` does `stated.or(notebook_language)`, conflating "said nothing" with "said something we cannot map". ADR-0025 promises the second becomes `Raw` with a warning, and that only holds for a notebook with no kernelspec. Splitting those two cases is the fix; the ADR already says what the behaviour should be |
| **`bp-files` never applies `\\?\`** | A name at `SemanticName::to_filename`'s own 255-byte maximum is unwritable in a deep enough directory, and `SaveError` blames a read-only file for `os error 3`. `bp_platform::paths::to_extended_length` is the fix; `bp-files` does not depend on `bp-platform` yet |
| `bp-search` and `bp-buffer` disagree what a line is | **An ADR, not a patch — D6.** Either `bp-search` adopts ropey's break set or `bp-buffer` drops `unicode_lines`. One definition has to win, and `bp-ui` uses both today, in different views |
| The recent-files list and the recovery directory both roam on Windows | `recent.toml` and `bp-ui`'s `recovery_dir` both hang off the config directory, which is `%APPDATA%` and roams. Both hold machine-specific absolute paths; `bp-platform` calls this `DirKind::State`. Needs one change owning `bp-config` **and** `bp-ui`, so it cannot be split between agents |
| `Profile::is_data` is true for `Ini` and `Xml` | Latent. Harmless until somebody gates a menu on it, at which point every row fails |

## Tier 1 — pure wiring (the capability exists and is tested)

**All three original Tier 1 items are done** (W4, W6, W7 — see the table
above). What was learned doing them, for whoever wires the next row:

- `EditorSurface` had **two** constants that had to start following
  `font-size`, not one: `line-height` and `gutter-width`. Both feed
  `row-at` / `column-at`, so a constant left behind does not merely look
  wrong — it moves the caret away from the pointer, and further the further
  down or across the click is. Anything else that scales has the same trap.
- `menus::view` now takes the font size, so **every menu builder whose rows
  depend on a value must be handed that value**; there is no ambient state
  in `menus.rs` and there should not be.
- A row that exists but cannot act right now uses `row_enabled(.., false)`,
  which is deliberately *not* `planned()`. `planned` means "does not exist
  yet"; greying for a reason the user can act on is a different statement.

## Tier 2 — done

All of it, in one pass. What is worth carrying forward:

- **`Command::Indent` is not `Command::Insert("	")`.** What a soft tab
  inserts depends on the caret's visual column -- at column 3 with width 4 it
  is one space, not four -- so `command_for` cannot decide it and
  `Editor::apply` expands it instead. Any future key whose text depends on
  where the caret is has the same shape.
- **Save Copy shares nothing with `save_document` except the encoder.** The
  three things it must not do -- move the path, clear the dirty flag, touch
  Open Recent -- are the three that one does. Calling it and subtracting three
  behaviours is how Save Copy becomes Save As; there is a test for each.
- **A `PopupWindow`'s `x` cannot be assigned from outside it.** The tab
  context menu positions itself through a property the popup binds to. The
  compiler catches this, but only once you try.
- **`menus::insert` is rebuilt on every refresh**, because its row hints are
  rendered from the clock. A menu built once at startup would offer this
  morning's time all afternoon.
- **The tab menu's target is stored, not passed.** Opening the menu and
  clicking a row are separate events, and by the second one the pointer has
  moved.

## Blocked, and by what

| Item | Blocked on |
| --- | --- |
| ~~Word wrap in the editor surface~~ | **Done.** `bp_editor::wrap` owns the breaking rule; `view`'s wrapped half maps rows to characters; `Editor` moves Up and Down by visual row. The shell anchors scrolling to a line *and* a row within it, and Slint reports the column count because it owns the font. |
| **Input-method composition** | **Slint 1.17.1, and this was checked rather than assumed.** `FocusScope` returns `EventResult::Reject` for `UpdateComposition` and `CommitComposition` in *both* `capture_key_event` and `key_event` (`i-slint-core/items/input_items.rs`), and exposes no callback for either. `TextInput` is the only item in the whole toolkit that consumes them, and `input_method_request` is a backend-internal trait, not an application hook. There is no route to pre-edit text from a `FocusScope`. Revisit on a Slint upgrade. |
| **Security ▸ Sign Document** | **D5 — where a signing key lives.** `bp_integrity::sign_file` exists, is tested, and writes the very sidecar the Verify row now reads, so this is a key store away from working. The row is greyed with the reason on it rather than `planned`. Verifying shipped without it, because verifying needs nothing stored. |
| Anything writing to `bp-storage` | **Still a decision, but a narrower one.** Phase 14 exists now and `record_document` honours the policy, so the security half is answered. What remains is ADR-0019's product judgement plus a design pass. Do not "just wire it up". |
| `bp-organize` | The above, plus a design pass. |
| ~~YAML support~~ | **Done and unreachable.** ADR-0023 picked `saphyr`; `bp-data` validates, formats, minifies and converts, with nesting, alias expansion and duplicate keys all bounded. No `bp-ui` code calls any of it. This is now a wiring item, not a blocked one. |
| Markdown preview | Slint has no HTML view. Needs a rendering decision first. |
| ~~`.bpadx` encryption~~ | **Done.** ADR-0021 settled the envelope and it ships; ADR-0022 seals the recovery journal with it. |
| Print | Platform work with no cross-platform story yet. |
| **W11, drag and drop to open** | **Slint 1.17.1**, and this was checked rather than assumed. `DropArea` exists and compiles, and the item is implemented in `i-slint-core/items/drag_n_drop.rs` -- but the winit backend has no file-drop plumbing, and `DataTransfer` exposes only `plain_text` and `image`. There is no channel a dropped file's path could arrive through. Revisit on a Slint upgrade; do not spend an afternoon on it before checking that changed. |

## Not a queue item

The first thing in `NEXT_SESSION.md` is a person typing into `--editor-view`
and writing down what feels wrong. No agent can do that, and W1, W5, W6 and W7
are all worth less until somebody has — they are caret-dependent, and nobody
has yet confirmed the caret lands where the pointer does.
