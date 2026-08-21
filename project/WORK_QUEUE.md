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
| — | `AuditLog` reads lazily instead of eagerly | ADR-0036, `bp-audit`, `24610f2` |
| — | `bp-storage`: a related-notes panel and duplicate detection | ADR-0037, `Organize` menu, `b0c152a` (fixed: the store never existed on a fresh machine until `4b549a5`) |
| — | `bp-execution`: a fresh subprocess per cell, three languages first | ADR-0040, `8c2dfab`. Reachable since ADR-0043 |
| — | Research Report, a synthesis layer over `bp-storage` | ADR-0041, `Research` menu, `d9bdf30` |
| — | File ▸ New Window, Insert ▸ Markdown, Tools ▸ Inspector, Help ▸ Diagnostics | `354934d` |
| — | A bar takes the caret a tick after the row that opened it | `34f02e4`, and `prompts/rosettas/R011` rule 9 |
| — | `bp_search::StreamSearch`: find in a file too large to hold | ADR-0042 |
| — | Find, over a document served from disk — the shell half | ADR-0042. Driven against a 213.5 MiB log |
| — | A huge document had no keyboard shortcuts at all | Same change. `scroll_for_command` claimed to handle `Command::Ignore` |
| — | Nothing held the keyboard until you clicked, in the custom surface | `focus-editor-soon()`, because `forward-focus` is resolved at compile time and cannot follow a property |
| — | The status bar went stale after every jump | `sync_cursor_label`, in `draw_editor_view`, where drawing and saying where it is are one act |
| — | The Research menu reads the document: Citation Metadata, Find Identifiers, Check Bibliography | ADR-0044. `bp-research` reachable; no crate is unreachable now |
| — | A `.md` with fenced blocks runs like a notebook | ADR-0045. Read as a `Notebook`, so consent carries over untouched |
| — | Notebook ▸ Cell Outline, on one shared `ListPanel` | ADR-0045. Two near-identical panels became one, with three uses |
| — | Find did not scroll to its match in the default surface | Found by the outline jump. Slint's own `cursor-position-changed` pattern |
| — | Go to Line works in both surfaces | It never needed the caret *read*, only *moved*. The refusal outlived the reason for it |
| — | `bp-execution` gains a resumable run: `Run::start`, `poll`, `stop` | ADR-0043. `run` is now written over it, so there is one implementation |
| — | Run ▸ a notebook cell, over an `.ipynb` | ADR-0043. `bp-notebook` and `bp-execution` are reachable |
| — | Research Report names the documents behind each insight, and states the rules it applied | ADR-0046. The truncation it used to leave silent is now said out loud |
| — | Research ▸ Open Questions, and `bp_semantic::questions` | ADR-0046. Not "Research Question": a question has a grammar, and which one you are asking does not |
| — | Research ▸ What the Store Holds | ADR-0046, and `bp_storage::Store::summary` |
| — | `forget_document` does what its doc comment had promised for two sessions | ADR-0046. Orphaned `tags` rows were never cleared, and its own test said so while the doc comment said otherwise |

Every library half now has its row.

## Tier 0 — empty

**This tier held three modes for four sessions, then one scoping job, and now
holds nothing.** `bp-storage` left under ADR-0037, `bp-notebook`/`bp-execution`
under ADR-0043, `bp-research` under ADR-0044, and research mode's synthesis
half under ADR-0046. No crate in this workspace is unreachable, and
`docs/product/MENU_MAP.md`'s Research section has no planned row left in it.

None of the four needed the work it had been sized for, and that is recorded
in `DECISIONS.md` because the estimate was the error each time. The first
three were waiting on a question about what the mode was *for*: **undecided
reads like large.** The fourth was the mirror image, and the more dangerous of
the two — five row names sitting in `MENU_MAP.md` since the scaffold commit,
looking decided because they were written in a table. **A name that has sat in
a plan long enough starts to read like a specification.** Asking where each
came from resolved all five in an afternoon, two of them by deletion.

Read `prompts/rosettas/R011_WIRE_CAPABILITY_INTO_SHELL.md` before touching
`bp-ui` — **rule 9** cost a session's worth of confusion. **Only one agent
may add to `AppWindow` in `app.slint` at a time.**

### Ready for an agent, now

Nothing. What remains is in `project/DECISIONS_NEEDED.md` (three questions
only a person can answer), a manual pass at the window, and the outside review
of `bp-crypto`'s envelope that D4 asked for four sessions ago.

**That is not the same as "the product is finished."** It means every item
whose shape is known has been taken, and the next one needs a decision before
it can be sized — which is the state this queue has now reached three
times, each time briefly.

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
| ~~Anything writing to `bp-storage`~~ | **Done** — [ADR-0037](../docs/decisions/ADR-0037.md), `b0c152a`. A related-notes panel and duplicate detection, both automatic at save and on-demand from the `Organize` menu. |
| ~~`bp-organize`~~ | **Same ADR, same commit.** Its two `Organize` menu rows (Related Notes, Duplicate Detection) are wired. |
| ~~YAML support~~ | **Done and unreachable.** ADR-0023 picked `saphyr`; `bp-data` validates, formats, minifies and converts, with nesting, alias expansion and duplicate keys all bounded. No `bp-ui` code calls any of it. This is now a wiring item, not a blocked one. |
| Markdown preview | Slint has no HTML view. Needs a rendering decision first. |
| ~~`.bpadx` encryption~~ | **Done.** ADR-0021 settled the envelope and it ships; ADR-0022 seals the recovery journal with it. |
| Print | Platform work with no cross-platform story yet. |
| **W11, drag and drop to open** | **Slint 1.17.1**, and this was checked rather than assumed. `DropArea` exists and compiles, and the item is implemented in `i-slint-core/items/drag_n_drop.rs` -- but the winit backend has no file-drop plumbing, and `DataTransfer` exposes only `plain_text` and `image`. There is no channel a dropped file's path could arrive through. Revisit on a Slint upgrade; do not spend an afternoon on it before checking that changed. |

## Not a queue item, and it is the top of `NEXT_SESSION.md`

**A person driving the window and writing down what is wrong.** No agent can
do it, and the argument is no longer theoretical: four manual passes have
happened and each found a defect no test could — a menu bar where twelve of
fourteen menus swallowed clicks, a tab drawn as one glyph while every column
was computed as though it reached the next stop, **Find typing the query
into the document**, and **Sign Document typing a passphrase into the
document in plain text**. All four were fixed the same way as they were
found: by somebody watching the window rather than reading a green test
suite.

All four lived in the same seam, and it is the one a test in this repository
cannot see: **what a toolkit does with what it is handed.** 1,754 tests did not
find the third — `AppState::find` was correct throughout and every test
of it passed. 1,812 did not find the fourth either, and the reason is sharper:
`PassphraseBar`, its `focus-input()` and the Rust that invokes it are each
correct alone, and the defect lived in *when* two other things took the caret
back — one of them `dispatch`'s own last line, the other a `MenuPopup`
restoring focus after the row's callback returns. Both are closed — see
`README.md`'s "Recently closed" section, and `project/NEXT_SESSION.md` §2 for
the mechanism.

**A trap for anything opening a bar from a menu row**, since this will come
round again: a menu row's callback cannot take the keyboard focus for itself.
`dispatch` calls `focus-editor()` right after it, and closing the popup
restores the focus the popup took when it opened. `ui/app.slint`'s
`focus-timer` is how a bar asks for the caret; anything new that wants it
should go through the same `pending-focus` property rather than calling
`.focus()` inside the click.

The caret question that used to head this section is answered — a click at the
end of `line 30` reported `Ln 30, Col 8`, exactly its length plus one — so W1,
W5, W6 and W7 are no longer waiting on it. What is unchecked now, in priority
order, is the wheel and the resize, because the huge-file viewer inherits both
and does nothing else. `NEXT_SESSION.md` has the full checklist and a
four-line recipe for a 192 MiB fixture.

## Also not a queue item, any more: no design passes, down from three

`bp-storage`, notebook mode and research mode were each blocked on a decision
about **what the product is**, not on effort -- and that decision is what made
an item premature to take, since building one without an answer produces a
feature nobody asked for. All three are settled: ADR-0037, ADR-0043 and
ADR-0044 in turn, and ADR-0046 for the last of research mode's planned rows.

**What this section is kept for** is the pattern rather than the three
instances, because it will come round again: an item whose *purpose* is
undecided cannot be estimated, and an estimate made anyway measures the
question. `project/DECISIONS_NEEDED.md` carries what is still genuinely open,
which as of 2026-08-22 is three rows — and none of them blocks a queue item
today, which is why this queue is empty rather than blocked.
