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

The library halves have no menu rows yet: W5, W8, W10 and W14 each still
need their wiring item below.

## Tier 1 — pure wiring (the capability exists and is tested)

Nothing here needs a new library function. Each is `bp-ui` plus one or two of
the `ui/*.slint` files.

| # | Item | Capability already in | Notes |
| --- | --- | --- | --- |
| W4 | **Find-box options** | `bp_search::Query{case_sensitive, whole_word, regex}` | Three toggles in `find_bar.slint`, feeding fields that already exist and are already tested. Everything currently builds `Query::literal`, so regex search is implemented and unreachable. The find bar is a collapsed `Rectangle`, not an `if` — keep it that way. |
| W6 | **Zoom** | `bp_config::Config::font_size` (6–96, default 14) | Three hard-coded `14px`: the gutter and the `TextInput` in `app.slint`, and `EditorSurface`'s own default. Make it one property fed from config **first**, then add Ctrl+= / Ctrl+- / Ctrl+0 and the View menu row. `EditorSurface` already derives its character advance from `font-size` and `AppWindow` never sets it, so passing one down should just work — verify it does. |
| W7 | **Double- and triple-click selection** | `bp_editor::Editor::{select_word_at, select_line_at}` | `TouchArea` has `double-clicked`; triple needs counting clicks yourself. **`editor_surface.slint` only** — `TextInput` already does this natively, so do not touch that path. |

## Tier 2 — a small library addition, then wire

Each needs one function in a library crate (with tests) before the wiring.
The library half is parallel-safe; the wiring half needs the `app.slint` lock.

| # | Item | Library work | Wiring |
| --- | --- | --- | --- |
| W5 | **Go to Line** | **done** — `Editor::go_to_line` returns whether the line existed, so the caller can say "there are only 42 lines" and still move somewhere sensible | Reuse the find-bar pattern rather than inventing a dialog. Caret-dependent, so `--editor-view` only. |
| W8 | **Insert Date / Time** | **done** — `Stamp::all()` gives the menu order and its labels | The Insert menu is currently all planned rows. `Stamp::all()` turns five of them real; the shell supplies the clock, since `bp-naming` never reads one. |
| W9 | **Save Copy** | none — `bp_files::atomic_write` is enough | File menu. **The subtle part**: a copy must not change the document's path, must not clear its dirty flag, and must not touch the recent-files list. Getting that wrong turns Save Copy into Save As. Test those three. |
| W10 | **Document statistics** | **done** — `bp_semantic::statistics`, single pass | Status bar, plus Tools ▸ Document Inspector (specs §18). Must not be recomputed on every keystroke over a whole document — see the `active_text()` trap in R011. |
| W11 | **Drag and drop to open** | none | specs §4 requires it. Slint has `DropArea`. Wire to `AppState::open`; multiple files open multiple tabs, as the command line already does. |
| W12 | **Tab context menu** | none | Close Others, Close All, Copy Full Path. Reuse `close_with_prompt` so unsaved work still prompts — that is the whole risk in this item. |
| W13 | **Follow System theme** | `bp-theme`: resolve a system preference to a `ThemeId` | specs §16 lists it as a built-in. Slint exposes the colour scheme; ADR-0009 says themes are data, so this resolves to an existing palette rather than adding a new one. |
| W14 | **Indentation settings** | **done** — `Config::{tab_width, indent_spaces}` | `TAB_WIDTH` is still a `const` in `bp-ui/src/editor_view.rs`, and the Tab key still inserts a literal tab. Both should follow the setting now that it exists. |

## Blocked, and by what

| Item | Blocked on |
| --- | --- |
| Word wrap in the editor surface | Design, not effort. It makes a visual line differ from a document line, which every function in `bp_editor::view` assumes away, and it changes what Up and Down mean. Not a small-agent item. |
| Input-method composition | Slint delivers pre-edit text through `KeyEventType::UpdateComposition`, which a bare `FocusScope` does not see. Needs investigation before it needs code. |
| Anything writing to `bp-storage` | **A decision, not a task.** Recording a document's extracted title puts a plaintext index of what the user writes about on disk. ADR-0019 explains why it is deliberately unwired; ADR-0011 says the security profile governs it, and phase 14 does not exist. Do not "just wire it up". |
| `bp-organize` | The above, plus a design pass. |
| YAML support | `serde_yaml` is deprecated; the replacement is a long-lived dependency choice and wants a human. |
| Markdown preview | Slint has no HTML view. Needs a rendering decision first. |
| `.bpadx` encryption | ADR-0011 forbids custom cryptography; envelope format, crate and versioning want agreement before code. |
| Print | Platform work with no cross-platform story yet. |

## Not a queue item

The first thing in `NEXT_SESSION.md` is a person typing into `--editor-view`
and writing down what feels wrong. No agent can do that, and W1, W5, W6 and W7
are all worth less until somebody has — they are caret-dependent, and nobody
has yet confirmed the caret lands where the pointer does.
