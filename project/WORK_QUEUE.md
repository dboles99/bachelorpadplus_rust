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

> **One agent in `crates/bp-ui/` at a time.** `lib.rs` is ~2,400 lines and
> every wiring item touches it. Two agents in there produce a merge, not a
> feature.

**T0 below exists to remove that bottleneck.** Until it is done, wiring items
must be taken one at a time, or batched by menu so one agent does several
related rows in a single pass — which is usually faster than queueing anyway.

Library items are one crate each and parallel-safe with everything.

Nobody edits the workspace `Cargo.toml`, `README.md`, `ROADMAP.md`,
`DECISIONS.md` or `docs/decisions/` concurrently — those are shared and small,
so whoever finishes an item updates them as part of it.

---

## T0 — do this first if more than one agent is working

| # | Item | Where |
| --- | --- | --- |
| T0 | **Split `bp-ui/src/lib.rs` into modules.** Suggested seams, which the file already has as comment banners: `state.rs` (`AppState` and its methods), `editor_view.rs` (the `EditorSurface` handlers, `push_editor_view`, `draw_editor_view`, `place_caret`, `apply_editor_command`, `translate_key`), `dispatch.rs` (the menu-action match), leaving `lib.rs` as `run_with` and the callback wiring. Tests move with the code they cover. **Pure refactor — no behaviour change, no test deleted, no test added.** The gate passing is the whole acceptance criterion. | `bp-ui` |

Afterwards, most wiring items below touch different modules and can run in
parallel.

---

## Tier 1 — pure wiring (the capability exists and is tested)

Nothing here needs a new library function. Each is `bp-ui` + `app.slint` only.

| # | Item | Capability already in | Notes |
| --- | --- | --- | --- |
| W1 | **Duplicate Line, Move Line Up / Down** | `bp_editor::Editor::{duplicate_line, move_line_up, move_line_down}` | The rows exist greyed in `menus.rs`. Shortcuts: Ctrl+D, Alt+Up, Alt+Down. **Caret-dependent** — enable only under `--editor-view`, grey otherwise with the reason, exactly as the menu already does for planned rows. Free id block: 95–99. |
| W2 | **Paste transformations** | `bp_clipboard::{Transform, apply, transforms_for}` | specs §14. Offer only `transforms_for(entry.kind)`, and only when `apply` returns `Some` — a row that does nothing is worse than no row. **`PrettyJson`/`MinifyJson` return `None` on purpose**: route those two to `bp_data::json_format` / `json_minify`, which is the one JSON implementation. The doc comment on `apply` explains why. |
| W3 | **CSV → JSON, CSV → JSON Lines, Column Types** | `bp_data::{delimited_to_json, delimited_to_jsonl, column_types}` | Add to the `Format::Csv \| Format::Tsv` arm of `menus::data()`. The first two are `apply_to_active`; `column_types` reports into the status bar like `DATA_REPORT` does. The `DATA_*` id block has room. |
| W4 | **Find-box options** | `bp_search::Query{case_sensitive, whole_word, regex}` | Three toggles in the find bar, feeding fields that already exist and are already tested. Everything currently builds `Query::literal`, so regex search is implemented and unreachable. The find bar is a collapsed `Rectangle`, not an `if` — keep it that way. |
| W6 | **Zoom** | `bp_config::Config::font_size` (6–96, default 14) | The size is hard-coded as `14px` in several places in `app.slint`; make it one property fed from config **first**, then add Ctrl+= / Ctrl+- / Ctrl+0 and the View menu row. The editor surface already derives its character advance from the font size, so zoom should just work there — verify it does. |
| W7 | **Double- and triple-click selection** | `bp_editor::Editor::{select_word_at, select_line_at}` | `TouchArea` has `double-clicked`; triple needs counting clicks yourself. **`EditorSurface` only** — `TextInput` already does this natively, so do not touch that path. |

## Tier 2 — a small library addition, then wire

Each needs one function in a library crate (with tests) before the wiring.
The library half is parallel-safe; the wiring half obeys the T0 rule.

| # | Item | Library work | Wiring |
| --- | --- | --- | --- |
| W5 | **Go to Line** | `bp-editor`: line number → caret offset, clamped. `Buffer::line_start` already does the arithmetic. | Reuse the find-bar pattern rather than inventing a dialog. Caret-dependent, so `--editor-view` only. |
| W8 | **Insert Date / Time** | `bp-naming` already formats `DDMMMYYYY`; add the display formats specs §4 wants for insertion. Keep it clock-free — take the time as a parameter, as that crate does throughout. | The Insert menu is currently all planned rows. Three of them become real. |
| W9 | **Save Copy** | none — `bp_files::atomic_write` is enough | File menu. **The subtle part**: a copy must not change the document's path, must not clear its dirty flag, and must not touch the recent-files list. Getting that wrong turns Save Copy into Save As. Test those three. |
| W10 | **Document statistics** | a small `bp-semantic` or `bp-core` function: characters, words, lines, and the same for the selection | Status bar, plus Tools ▸ Document Inspector (specs §18). Must not be recomputed on every keystroke over a whole document — see the `active_text()` trap in R011. |
| W11 | **Drag and drop to open** | none | specs §4 requires it. Slint has `DropArea`. Wire to `AppState::open`; multiple files open multiple tabs, as the command line already does. |
| W12 | **Tab context menu** | none | Close Others, Close All, Copy Full Path. Reuse `close_with_prompt` so unsaved work still prompts — that is the whole risk in this item. |
| W13 | **Follow System theme** | `bp-theme`: resolve a system preference to a `ThemeId` | specs §16 lists it as a built-in. Slint exposes the colour scheme; ADR-0009 says themes are data, so this resolves to an existing palette rather than adding a new one. |
| W14 | **Indentation settings** | `bp-config`: tab width, and spaces-vs-tabs | `TAB_WIDTH` is currently a `const` in `bp-ui`, and the Tab key inserts a literal `\t`. Both should follow the setting. `bp_editor::view` already takes `tab_width` as a parameter throughout, so the library side is ready. |

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
