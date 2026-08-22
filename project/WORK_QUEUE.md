# Work Queue

Bounded, independent pieces of work, each sized so that a competent coding
agent can finish one without holding the whole product in its head.

`project/NEXT_SESSION.md` says what matters most. This says what is *ready*,
which is not the same list.

## How to take an item

1. Read `CLAUDE.md` first -- it grants the standing authorities, so you do not
   have to ask whether to start, whether to commit, or whether a planned row
   may be deleted.
2. Read `prompts/rosettas/R011_WIRE_CAPABILITY_INTO_SHELL.md` if the item
   touches `crates/bp-ui/`. It encodes the traps that have already cost
   somebody a session -- action-id ranges, the two editor views, Slint `if`
   scoping.
3. **Take more than one.** See below.
4. Run the gate before saying it is done:
   `./scripts/Invoke-LocalCI.ps1 -Linux`

### How big a bite

This list used to say: *take the whole item; each one is already the smallest
thing worth doing, and splitting one further produces a half-feature.* Both
halves of that are still true, and the advice built on them stopped being
right.

**What changed is the fixed cost of *entering* this repository.** Learning
enough to work here safely -- the record, the traps, the two editor views, the
action-id blocks -- now costs more than most individual items do. An agent
that takes one small item pays that cost in full and amortises it over one
change.

So: **take a batch that shares an entry cost, and finish all of it.** The
natural seams, in order of how well they work:

- **one menu, several rows** -- the builder, the ids and the dispatch arms are
  read once and edited once;
- **one crate's whole surface** -- a library item and every row that reaches
  it, so nothing lands unreachable;
- **one subject across crates** -- the shape ADR-0046 took: a library
  capability, a store query, two rows, and the scoping that decided which rows
  should exist at all.

**The floor has not moved.** A batch is still made of whole items, and
splitting one to make a batch smaller produces the same half-feature it always
did. What is discouraged is stopping after one because the queue happened to
list it separately.

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
| — | `CLAUDE.md`, three standing authorities, and one home per kind of fact | ADR-0047 |
| — | pre-push checks for a full gate instead of repeating it; `cargo-nextest` with a doctest stage | ADR-0047. 15.0 min to 7.2 |
| — | Tools ▸ Security Inspector, File Analysis, Configuration | ADR-0048, ids 721-723 |
| — | Note ▸ Tags, Recovery Checkpoints | ADR-0048, ids 87-88 |
| — | Notebook ▸ Export as .ipynb | ADR-0048, id 747 |
| — | Organize ▸ Suggested Folder | ADR-0048, id 702 |
| — | Run ▸ Run Document, Interpreters | ADR-0048, ids 751-752, and `Language::{all,interpreter}` |
| — | Security ▸ Lock Document | ADR-0048, id 212 |
| — | `scripts/Drive-Window.ps1`: the window is driven by a script, not re-derived | ADR-0049 |
| — | A clipped menu label is elided, and the reasons are short enough not to need it | ADR-0049. Found by driving |
| — | `crate::testpaths`: a temp path a recycled pid cannot reuse | ADR-0049. A flake that got worse the more the suite was run |
| — | Every menu row works or is a readout | ADR-0048. Ten built, eleven deleted; `planned_menu` and `arrives` gone |
| — | D4's envelope review, prepared as a brief a stranger can act on | ADR-0050, `docs/architecture/BPADX_ENVELOPE_REVIEW.md` |
| — | Golden vectors: a document sealed by an earlier build must still open | ADR-0050, `fuzz/tests/envelope.rs`. Nothing had ever asserted it |
| — | Run's readout stops denying there is anything to run while Run Document runs it | ADR-0051, `menus.rs`. Found by driving the window |
| — | `every_menu()` finally holds the Run menu, six times over | ADR-0051. The sweep's own comment named the failure mode it had |
| — | The window driver waits for the window it will photograph | ADR-0051, `scripts/Drive-Window.ps1`. `-Keys` takes a list now |
| — | Enter twice in the find box no longer replaces the match with a line break | ADR-0052, `dispatch::reveal`. Confirmed at the keyboard, both bars |
| — | Tools ▸ Security Inspector, File Analysis, Configuration | ADR-0048, ids 721-723 |
| — | Note ▸ Tags, Recovery Checkpoints | ADR-0048, ids 87-88 |
| — | Notebook ▸ Export as .ipynb | ADR-0048, id 747 |
| — | Organize ▸ Suggested Folder | ADR-0048, id 702 |
| — | Run ▸ Run Document, Interpreters | ADR-0048, ids 751-752, and `Language::{all,interpreter}` |
| — | Security ▸ Lock Document | ADR-0048, id 212 |

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

Nothing. What remains is in `project/DECISIONS_NEEDED.md` and a manual pass at
the window.

**D4 has left this list.** The outside review of `bp-crypto`'s envelope was
the largest unstarted work in the project for five sessions; its *preparation*
was taken on 2026-08-22 and is
[BPADX_ENVELOPE_REVIEW.md](../docs/architecture/BPADX_ENVELOPE_REVIEW.md).
What is left of D4 is finding a reviewer, which is not a queue item because no
agent can be one. **The lesson is worth more than the item**: a task that
cannot be finished can usually be prepared, and five sessions read "an agent
cannot do this" as "there is nothing here to do" ([ADR-0050](../docs/decisions/ADR-0050.md)).

**One item came back and was taken the same session**: every menu row working
(ADR-0048). It is worth remembering how it looked before it was measured --
38 planned rows, sized as a phase of work -- and what it actually was: mostly
finished features nobody had pruned from a list, plus eleven rows that were
better deleted than built. **Take the inventory before the estimate.**

**That is not the same as "the product is finished."** It means every item
whose shape is known has been taken, and the next one needs a decision before
it can be sized — which is the state this queue has now reached three
times, each time briefly.

## Tiers 0.5, 1 and 2 — all empty

Nothing is left in any of them. What each held, and what it cost to learn, is
in [DECISIONS.md](../DECISIONS.md) under "What the tiers taught" — those were
631 words of pure *lesson* sitting in the file an agent has to read to find
work, which is the wrong place for them twice over.

The headlines, so somebody knows whether to go and look:

- **Tier 0.5** — eleven files of cross-crate tests found seven defects. A path
  built by subtracting another path's last component moves when that one does;
  separate the rule from the edge that reads the environment; a predicate on
  the wrong type is a trap with no name.
- **Tier 1** — pure wiring. Two constants had to start following `font-size`,
  not one; a menu builder whose rows depend on a value must be handed it;
  `row_enabled(.., false)` and `planned()` are different statements.
- **Tier 2** — `Command::Indent` is not `Command::Insert("\t")`; Save a Copy
  shares nothing with Save but the encoder; a `PopupWindow`'s `x` cannot be
  assigned from outside it.

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
finish it: `rfd` modals swallow synthetic input, and the rows that most need
watching are the ones behind a native dialog.

Five manual passes have happened and every one found a defect no test could --
a menu bar where twelve of fourteen menus swallowed clicks, a tab drawn as one
glyph while every column was computed as though it reached the next stop, Find
typing the query into the document, and Sign Document typing a passphrase into
the document in plain text.

**The checklist lives in `project/NEXT_SESSION.md` §3** and the lesson from
each defect in `DECISIONS.md`. Neither is repeated here; this section exists
only so that somebody reading the queue for something to do knows that the
most valuable thing available is not in it.

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
