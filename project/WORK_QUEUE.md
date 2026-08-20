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

`bp-ui/src/lib.rs` has been split, so `state.rs`, `editor_view.rs`,
`dispatch.rs`, `viewer.rs`, `audit.rs`, `passphrase.rs` and
`default_editor.rs` can take different agents — thirteen modules now, listed
with their sizes in `docs/architecture/ARCHITECTURE.md`. **`state.rs` has been
split too**, into `state/security.rs`, `state/data.rs`, `state/encryption.rs`
and `state/find.rs`, so the security operations, the Data menu and the
passphrase flow are each somebody's own file rather than three banners in one
6,282-line one. `ui/app.slint` has now been split too
— ten files, listed in `docs/architecture/ARCHITECTURE.md`. So the find
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
| — | `atomic_write` refuses a device name a user typed | `bp-files`, which reaches `bp-platform` now |
| — | The gate sees `fuzz/`, on both legs | `scripts/Invoke-LocalCI.ps1` |
| — | `paths::file_name`, and the panic lints | `bp-platform`, `[workspace.lints]` |
| — | Security ▸ Security History, and the five calls that fill it | ADR-0024, id 206 |
| — | A tab is drawn as wide as the caret believes it is | `bp-editor`, found by looking |
| — | `bp-files` applies the extended-length prefix | `bp-files` |
| — | A cell that named a language we cannot map is never given the kernel's | ADR-0025, `bp-notebook` |
| — | One definition of a line, and no ignored tests left | ADR-0029, `bp-buffer` |
| — | Size decides how a document is opened; opening stops reading the file twice | ADR-0027, `bp-ui` |
| — | The recent list, the recovery journal and the security history stop roaming | `DirKind::State`, `bp-config` and `bp-ui` |
| — | The Data menu's question is asked of the format, not of its class | `Format::has_data_operations`, `bp-formats` |
| — | The application id is reverse-DNS, and is not the directory name | ADR-0032, `bp-platform` |
| — | A view for a document the rope does not hold | ADR-0030, `bp-ui/viewer.rs` |
| — | Security ▸ Sign Document, and a key store that seals rather than restricts | ADR-0031, `bp-integrity`, ids 202 |
| — | `state.rs` splits four ways; the private fields stay private | `bp-ui/src/state/{security,data,encryption,find}.rs` |

Every library half now has its row.

## Tier 0 — three modes, and one thing an agent can take today

**The shape of this tier changed with the last item to leave it.** What is
here is not tested libraries missing a menu row; `bp-notebook` and
`bp-research` are each a *mode* — a view, a set of interactions, and a
decision about what the product is. The viewer for a huge document was the
last item that could be described as wiring, and even it turned out to be a
feature: a scroll model, four keys, four refusals and a surface decision.

Read `prompts/rosettas/R011_WIRE_CAPABILITY_INTO_SHELL.md` before touching
`bp-ui`. **Only one agent may add to `AppWindow` in `app.slint` at a time.**

**Splitting `state.rs` has left this tier**, which is why there is one row
below rather than two. It is worth knowing what it cost, because the same
shape will come round again: the split was possible only because the new
modules are *children* of `state`, so `AppState`'s private fields stayed
private. A sibling module would have widened every field it touched to
`pub(crate)`.

### Ready for an agent, now

| Item | Shape |
| --- | --- |
| **Find, over a document served from disk** | The last piece of the large-file story. `bp_buffer::LargeFile::windows` gives overlapping text windows with a fresh file handle, so a background search does not disturb the scrolling viewport, and `bp-search` decides what a match is. A feature rather than wiring, and ADR-0030 names the two hard parts: a match found in a window has to become a scroll position, and a search over 2 GB has to be interruptible |

### Blocked on a design pass, not on effort

| Item | What exists | What is missing |
| --- | --- | --- |
| Notebook mode | `bp-notebook`, 3,627 lines, 59 tests (ADR-0025). Format questions settled; a run needs a `UserGesture` no parsed file can produce, which is the security half done and a real constraint on the UI | A **view** where a document is a sequence of cells rather than a rope |
| Research mode | `bp-research`, 5,208 lines, 161 tests. The largest unreachable crate and the least specified | Same: a mode, and a decision about what it is for |
| `bp-storage` | 721 lines, 21 tests. ADR-0020 settled the security half — `record_document` honours the policy | **What the user gets for it.** D3 permits wiring *after* a design pass; recording something before deciding what it is for is how a metadata store becomes a liability |

## Tier 0.5 — what the cross-crate tests found

Eleven files of cross-crate tests found seven defects. **All seven are fixed,
the decision behind the last of them is made (ADR-0029), and there is no
`#[ignore]`d test left anywhere in the tree.** The two latent items that
remained -- neither wrong today, both wrong the moment anything depended on
them -- **are fixed too, and this tier is empty.**

| Item | Landed as |
| --- | --- |
| The recent-files list, the recovery journal and the security history all roamed on Windows | All three resolve through `DirKind::State`; `bp-config` and `bp-ui` in one change, because splitting it would have scattered the product's files |
| `Profile::is_data` was true for `Ini` and `Xml` | `Profile::is_data_class` for the class, `Format::has_data_operations` for the menu, `Format::ALL` and a test holding the second against `menus::data` |

What that pair is worth remembering for:

- **A path built by subtracting another path's last component moves when that
  one does.** The security history was `recovery_dir()` with `with_file_name`.
  Renaming the recovery folder would have moved it silently, and a history
  that moves starts again at sequence one.
- **Separate the rule from the edge that reads the environment.**
  `recovery_dir_under` and `audit_path_under` take the state directory as an
  argument, so both are assertable without a real profile -- the same split
  `bp_config::config_path` and `config_path_in` already made.
- **A predicate on the wrong type is a trap with no name.** `is_data` was
  asked of the *profile*, and `Profile::StructuredData` holds six formats of
  which `bp-data` parses four. The fix was not a better `match`; it was
  asking the question of the format.

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
| ~~**Security ▸ Sign Document**~~ | **Done** — [ADR-0031](../docs/decisions/ADR-0031.md). The key is sealed in a `.bpadx` envelope under a passphrase rather than protected by file permissions Windows cannot narrow. One ceremony, not one per signature: the first signature creates the key, the second unlocks it, and the row says which before the click |
| Anything writing to `bp-storage` | **A design pass, and only that.** D3 is answered: it may be wired, *after* somebody decides what it records and what the user sees for it. ADR-0019 stands as written, and "just wire it up" is still the wrong move -- now by confirmation rather than by default. |
| `bp-organize` | The above, plus a design pass. |
| ~~YAML support~~ | **Done and unreachable.** ADR-0023 picked `saphyr`; `bp-data` validates, formats, minifies and converts, with nesting, alias expansion and duplicate keys all bounded. No `bp-ui` code calls any of it. This is now a wiring item, not a blocked one. |
| Markdown preview | Slint has no HTML view. Needs a rendering decision first. |
| ~~`.bpadx` encryption~~ | **Done.** ADR-0021 settled the envelope and it ships; ADR-0022 seals the recovery journal with it. |
| Print | Platform work with no cross-platform story yet. |
| **W11, drag and drop to open** | **Slint 1.17.1**, and this was checked rather than assumed. `DropArea` exists and compiles, and the item is implemented in `i-slint-core/items/drag_n_drop.rs` -- but the winit backend has no file-drop plumbing, and `DataTransfer` exposes only `plain_text` and `image`. There is no channel a dropped file's path could arrive through. Revisit on a Slint upgrade; do not spend an afternoon on it before checking that changed. |

## Not a queue item, and it is the top of `NEXT_SESSION.md`

**A person driving the window and writing down what is wrong.** No agent can
do it, and the argument is no longer theoretical: two manual passes have
happened and each found a defect no test could — a menu bar where twelve of
fourteen menus swallowed clicks, and a tab drawn as one glyph while every
column was computed as though it reached the next stop.

Both lived in the same seam, and it is the one a test in this repository
cannot see: **what a toolkit does with what it is handed.** 1,754 tests will
not find the third.

The caret question that used to head this section is answered — a click at the
end of `line 30` reported `Ln 30, Col 8`, exactly its length plus one — so W1,
W5, W6 and W7 are no longer waiting on it. What is unchecked now, in priority
order, is the wheel and the resize, because the huge-file viewer inherits both
and does nothing else. `NEXT_SESSION.md` has the full checklist and a
four-line recipe for a 192 MiB fixture.

## Also not a queue item: three design passes

`bp-storage`, notebook mode and research mode are each blocked on a decision
about **what the product is**, not on effort. They are listed in Tier 0 above
so the size of them is visible, but taking one without an answer produces a
feature nobody asked for. `project/DECISIONS_NEEDED.md` carries the questions.
