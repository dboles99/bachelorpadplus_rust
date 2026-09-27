# Menu Map

The intended contents of each menu, and how much of each is real.

**A release shows only real rows** (release plan M1-05, 2026-09-25). The
application used to show planned rows greyed, so the menu bar doubled as a
roadmap. On a release candidate a menu of promises reads as a broken product,
so the planned rows and every "not implemented yet" trailer are gone.
Notebook, Organize, Research and Run come off the bar unless
`show_planned_menus = true` is set in `config.toml`. A greyed row that
remains is a **readout** (what a profile does, that the clipboard history is
empty) or a real row that cannot act right now, with the reason on it. This
file keeps the planned rows, as the roadmap the menus no longer are.

**Live** means reachable and doing something, in both editor views: the
caret rows that once needed `--editor-view` read `TextInput`'s caret since
M1-04. **Planned** means not built, and not shown. **Built** means the
library work exists and is tested but has no row yet — those are the
cheapest items in `project/WORK_QUEUE.md`.

## File

| Row | State |
| --- | --- |
| New, Open, Open Recent, Save, Save As, Save All, Reload, Close Tab | **live** |
| Save a Copy | **live** — writes the buffer elsewhere without moving the document's path, clearing its dirty flag, or touching Open Recent; all three are pinned by tests |
| Set as Default Editor... | **live** — `bp_platform::editor`, under ADR-0012, ADR-0032 and release plan M2-04/M2-06 (`bp-ui/src/default_editor.rs`). The row's hint names the preset it registers for (Notepad Replacement, `desktop::PACKAGED_PRESET` — the same set the packaged `.desktop` claims). A click **reports before it offers**: the dialog opens with what opens each of those types today — on Linux from `association_report_in`, which reads the whole XDG chain (desktop-specific `mimeapps.list`, `/etc/xdg`, the data dirs, then `mimeinfo.cache`); on Windows from the registry (`UserChoice`, then the class default) through `bp-platform-windows`, and "cannot tell" where there is no registry to ask, never "nothing is set to". Then the whole plan, and the button says what confirming does. **Linux** ("Set as Default"): `linux_plan` resolves against the machine — no per-user `.desktop` or MIME package when a package already installed them, the `.AppImage` file rather than its temporary mount as `Exec` — and the dialog lists the files, the root, the files left alone and every command (`update-desktop-database`, `update-mime-database`, `xdg-mime default`) line for line. Confirming writes the files and runs those commands, every one even if an earlier one fails, and reports each result and what the desktop now says. In a Flatpak the row greys ("managed by the Flatpak sandbox"). **Windows**: once registered (the installer's job, ADR-0032), "Open Settings" opens Default apps at the product's own page on build 22000 and later; before that, "Save Files..." saves the UTF-16 `.reg` pair — register and remove — where the user chooses. The application never writes the registry itself. **No key belonging to another application is ever written, and Notepad is never named**: `bp-platform` refuses to build such a plan and the shell re-checks with `registry_objections` before a `.reg` reaches a Save dialog |
| Remove Registration... | **live** (M2-06, Linux only) — `unregistration_plan` and `uninstall`, the inverse of Set as Default: consent first, then it removes only the per-user `.desktop` entry and MIME package that Set as Default wrote (each carries a marker line; a menu editor's customised copy and every system file are left alone), and runs the database rebuilds, reporting each. It leaves `mimeapps.list` alone — those defaults are the user's choices, and a desktop skips one naming an application that is no longer installed. Greyed ("nothing to remove") when Set as Default has written nothing — a package install — and in a Flatpak; read at startup and again after either row is used |
| New Window, Open Folder, Revert, Print | planned |

## Edit

| Row | State |
| --- | --- |
| Undo, Redo, Cut, Copy, Paste, Select All | **live** (both editor views) |
| Double-click a word, triple-click a line | **live** — native under `TextInput`, `bp_editor::{select_word_at, select_line_at}` under `--editor-view` |
| Clipboard History | **live** — the history **as it was when the menu opened** (U5, release plan M1-18): opening Edit or the editor context menu freezes a snapshot of the history as one fingerprint per entry (`bp_clipboard::Snapshot`), a row's id is a slot in it, and a click acts on the entry the slot names wherever the 1.2 s poll has moved it. An entry that has gone since is reported in the status bar and nothing is done. A copy made while the menu is open appears the next time it opens |
| Paste transformations | **live** — offered per entry kind, and only where the result would differ |
| Paste Special... | **live** — a question over what is on the clipboard *now*: each transformation that would change it, then Paste Unchanged and Cancel. The body gives its kind, lines and characters and never a preview, because the clipboard may hold another program's copy |
| Clear Clipboard History | **live** — forgets the unpinned entries the menu showed and nothing copied since; greyed when everything shown is pinned |
| Sort / Deduplicate / Reverse / Trim lines | **live** |
| Duplicate Line, Move Line Up / Down | **live** (Ctrl+D, Alt+Up / Alt+Down) |
| Go to Line | **live** (Ctrl+G) |
| Multi-cursor, column selection | planned — phase 2, needs the caret |

## View

| Row | State |
| --- | --- |
| Light / Dark / Organic / Green | **live** |
| Line Numbers, Word Wrap | **live** |
| Zoom In / Zoom Out / Reset Zoom | **live** — Ctrl+= / Ctrl+- / Ctrl+0, bounded by `bp_config::{MIN,MAX}_FONT_SIZE`; the rows grey at the bound and the reset row names the size in force |
| Follow System | **live** — `ThemeId::for_system` resolves the desktop's preference to Light or Dark; a desktop that will not say leaves the theme alone and says so |
| Split / Preview, panels, inspectors | planned |

## Insert

| Row | State |
| --- | --- |
| Date, Time, Date and Time, ISO 8601, Filename date | **live** — each row's hint is the stamp rendered from the clock, so the row shows what it will insert |
| Markdown constructs, citation, code block, table, notebook cell | planned |

## Format

| Row | State |
| --- | --- |
| LF / CRLF, UTF-8 / UTF-8 with BOM | **live** |
| Indent with Tabs / Spaces, Tab Width 2 / 4 / 8 | **live** — one width serves both the Tab key and how wide a tab is drawn; a soft tab goes to the next stop, not a fixed count |

## Data

Context-sensitive: the menu is built from the detected format, so a note gets
no data rows rather than a column of greyed ones.

| Row | State |
| --- | --- |
| JSON: validate, format, minify, sort keys, to JSON Lines | **live** |
| JSON Lines: validate, to JSON | **live** |
| TOML: validate, format | **live** |
| CSV/TSV: shape report, to JSON, to JSON Lines, column types | **live** |
| YAML: validate, format, minify, to JSON | **live** — `saphyr`, per ADR-0023. Validate names how many documents the file holds, because Format writes all of them back and Convert to JSON refuses more than one. The Format and Minify rows carry "comments not kept" as their hint and say it again in the status bar: a tree round trip has nothing to put a comment back from and resolves aliases into copies, and ADR-0023 names a row that offers this silently as a trap. Three refusals are the feature and not a failure mode — nesting past 128 levels (a 200-byte paste would otherwise overflow the stack and abort the process), alias expansion past 1,000,000 nodes, and a duplicate mapping key, which is refused rather than merged because formatting such a file would delete a line and report success. Each names its limit and where it was hit, and says the document was left unchanged. Sort Keys is not offered: there is no ordering over YAML nodes yet |
| JSON: to YAML | **live** — the direction that cannot lose anything, so it is the one with no warning on the row |
| Filter, query, schema, export | planned |

## Note

| Row | State |
| --- | --- |
| Suggest Title, Semantic Rename, Summary, Keywords, Outline | **live** |
| Document Statistics | **live** — `bp_semantic::statistics` into the status bar, one pass on a menu click and never on the typing path |
| Tags, classification, related notes, properties, history | planned — phase 9 |

## Security

| Row | State |
| --- | --- |
| Standard / Private / Confidential / Maximum | **live** — the active document's profile; exactly one ticks, and a Custom policy ticks none. Private and Confidential seal the recovery journal with the document's passphrase, so they need the document encrypted |
| What the profile permits (recovery, clipboard, network) | **live** — a readout, greyed because it is not clickable. A profile is a promise about invisible behaviour, and a promise nobody can see is not one |
| Encrypt Document... | **live** — asks for a passphrase twice, writes a `.bpadx` beside the original, and the tab adopts it so later saves stay encrypted. Reads "Encrypted (.bpadx)" and greys once the document is |
| Privacy Mode | **live** — a session-wide override that can only tighten |
| Scan for Secrets | **live** — `bp_secrets::scan` over the active document. The status bar gives the count, the kinds and the first three positions; the full listing is a dialog, one line per finding. **Neither ever prints the matched text**: a `Finding` deliberately carries a position and a classification and nothing else, and the shell must not undo that by reaching back into the document to quote it |
| Hash Document (SHA-256) | **live** — `bp_crypto::hash_document` over the bytes the document *would be written as*, not over the buffer, so the digest matches `sha256sum` on a document with a BOM or CRLF endings. Shown grouped in fours (to read down a telephone) and unbroken (to paste), and says so when unsaved edits mean it is not yet the digest of anything on disk |
| Sign Document... | **live** (release plan M2-10, decision 16 = D5 option c) — for a saved document with no unsaved changes, because a signature covers the file on disk; greyed "save it first" otherwise. The first use makes a key: the passphrase bar asks for a new passphrase twice (a mismatch starts again), then `bp_integrity::write_sealed_signing_key` keeps the seed sealed in a `.bpadx` envelope at `signing-key.bpadx` in the data directory, and the public half goes beside it as `signing-key.pub`, the hex a recipient pastes. Later uses ask for that passphrase; a wrong one keeps the bar open. The `.sig` sidecar is written beside the document, the security history records "document signed", and the status bar names the public key to hand out. Forgetting the passphrase means a new key |
| Verify Signature... | **live** — `bp_integrity::verify_file` over the `.sig` sidecar, per ADR-0026. The sidecar is **found, not asked for**: `document.ext` is signed by `document.ext.sig`, appended and never substituted, and that name is `bp-integrity`'s function rather than a rule the shell writes down a second time. Greyed for a document that has never been saved, with the reason on the row — there is no file for a sidecar to sit beside. The first pass asks the user nothing, because a key cannot change its answer; **a missing sidecar fails there, closed**, since a check that can be passed by deleting a file is not a check. Only a document that already holds together is worth asking for a key, and that key is what turns "intact" into "signed by who you expected". Each of the five verdicts gets its own sentence, worded by `Verification::explain` so that every surface says the same thing about the same answer: verified / no signature file / the file cannot be read / does not match, the named signer being a claim nothing confirmed / signed by a different key, the document intact. That last one is the verdict a bare 64-byte `.sig` could not produce at all, and the whole reason the format records a key. A pass with no key named carries the caveat `Expectation::AnySigner` earns — anyone who alters a document can re-sign it with a key of their own. The one sentence the shell adds is its own: unsaved edits mean these are not the bytes anybody signed, and the check was made against the file on disk. Public keys are still read in either spelling, 32 raw bytes or hexadecimal as pasted out of an email |
| Security History... | **live** — `bp-audit` under ADR-0024. The reading end of every row above it: a profile change, Privacy Mode, a secret scan, a redaction and a signature check each append one. What is recorded is governed by the document's own profile, so under Confidential and Maximum, and under Privacy Mode, **nothing is written at all** — `Destination::for_policy` resolves those to `SessionOnly`. A tightening is therefore silent and a loosening is recorded, which is the direction worth having: a move down to Standard re-enables everything the profile was switched on to stop. Under Private the history is sealed with the document's own passphrase, so an unencrypted document under that profile gets the notice telling it so rather than a plaintext line |
| Redact Found Secrets... | **live** — `bp_secrets::scan` produces the spans and `bp_redaction::redact` destroys them. `Placeholder`, not `Mask`: `MatchOriginal` publishes the length of what was removed, which for a PIN or a short token is most of the secret. The marker is `[REDACTED: kind]`, labelled with the name of the rule that matched and never with what it matched. A confirmation dialog lists line and kind first and states plainly that this changes the document and not the file — the file on disk, the recovery journal, the undo history and the clipboard all still hold the originals, and saying "redacted" without saying that is the same lie the black rectangle tells. Applied as an ordinary undoable edit for the same reason. **A private key block is deliberately not redacted**: `bp-secrets` marks only its `-----BEGIN` line, so redacting the span would take out the label and leave the key body — the row says so rather than half-doing it. `bp_redaction::verify` runs afterwards and a survivor is reported by line number, never by text. Greyed on an empty document, with the reason on the row |
| Inspect Metadata | **live** — `bp_redaction::metadata::inspect` over the text, reporting kind, exposure and line and never the value. For a container this build cannot open — `.docx`, `.pdf`, `.rtf`, an image — `Container::hidden()` and `requires()` are reported instead of silence: "no metadata found" about a `.docx` reads as an all-clear and would be a lie. For plain text it says the opposite thing it is easy to leave out — that the filesystem entry around the file, its timestamps, ownership and alternate data streams, is not part of the check |
| Lock Document, audit history | planned — phase 16 |

## Notebook

Enable mode, new cell, run, run selection, run all, convert selection,
split/merge cells, export. All planned — phase 12. Notebook content never
auto-runs (ADR-0011).

## Organize

Project, suggested folder, tags, topics, entities, related notes, smart
collections, duplicate detection, semantic search. All planned — phase 9.
`bp-storage` is the foundation and is deliberately unwired; see ADR-0019
before adding anything here.

## Research

Citation metadata, DOI and scholarly metadata, research question, evidence,
findings, methods, datasets. All planned — phase 13.

## Run

Run selection/cell/document, choose interpreter, stop, history, Rust
scratchpad. All planned — phase 12.

## Security (the rest)

Lock, decrypt in place, secure clipboard, audit, settings. Planned — phases 15
and 16. Redaction and the metadata inspector are no longer among them; both are
rows above.

Hash, sign and verify are no longer among them: `bp-crypto`'s hashing half and
`bp-integrity`'s sidecar are wired into the rows above. Signing was the last
of the three, waiting on where an Ed25519 key could live; decision 16 put it
in a passphrase envelope in the data directory, and the row went live with
release plan M2-10 (see its row above).

The three things that were waiting on profiles now read them: the recovery
journal refuses rather than writing plaintext under a profile that forbids it,
clipboard history is cleared and stops recording, and `bp-storage`'s
`record_document` honours `Metadata`. `bp-storage` is still not called by the
application — that is ADR-0019's product decision, not a security one any
more.

## Tools

| Row | State |
| --- | --- |
| Document inspector | planned — `bp_semantic::statistics` is most of it |
| Settings... (Ctrl+,) | **live** — a sheet over the window editing a draft; Save writes only what changed into the user's own `config.toml`, comments kept |
| Open Log Folder, Open Config Folder | **live** |
| Security inspector, file analysis, conversions, benchmarks | planned |

## Help

| Row | State |
| --- | --- |
| Keyboard Shortcuts | **live** |
| About | **live** — the version, the build's commit, what draws the window, both licences, Notices, and Slint's `AboutSlint` attribution (ADR-0031) |
| Help, diagnostics | planned |

## Over a viewer tab

A file past the editable limit, or any file opened with `--readonly` or
`--large-file`, opens in the read-only large-file viewer (release plan
M1-06; `bp-ui/src/viewer.rs`). The menus stay the same shape; what changes is
which rows can act. One list decides it -- `menus::viewer_refusal` -- and
both greys the rows and refuses the same ids in `dispatch` when they arrive
by a shortcut, so a row cannot be grey and still reachable.

| Rows | State over a viewer tab |
| --- | --- |
| New, Open, Open Recent, Save All, Reload, Close Tab, the tab context menu | **live** — Reload reads the file again and keeps the line |
| Save, Save As, Save as New Revision, Save a Copy | greyed, hint **read-only** |
| Undo, Redo, Cut, Paste, Paste Special, Select All, clipboard history, the line operations, Insert, Format's line endings and encodings, the Data conversions, Semantic Rename, Encrypt, Redact, Sign | greyed, hint **read-only** |
| Copy, Word Wrap, Hash, Scan for Secrets, Inspect Metadata, Verify Signature, Document Statistics, the Note extractions, Data Validate / Report / Column Types | greyed, hint **not in the viewer** — each would need the whole file in memory, which the viewer exists not to do |
| Go to Line, Find bar (Find, Next, Previous), themes, Line Numbers, zoom, indentation settings, profiles, Privacy Mode, Security History, Clear Clipboard History, Tools, Help | **live** |
| The editor context menu | **not shown** — the viewer has no selection to cut or copy and nowhere to paste |
| Replace All | refused in the find bar ("read-only") |

## Find bar

Not a menu. The bar itself is **live**, and so are its three options.

| Control | State |
| --- | --- |
| Find, Replace, Replace All, In Folder | **live** — over a viewer tab Find runs on a background thread from the caret and closing the bar stops it; Replace All is refused |
| Aa / Word / .* toggles | **live** — `bp_search::Query::{case_sensitive, whole_word, regex}`; flipping one re-runs the search |

## Go to Line

Not a menu of its own. Edit ▸ Go to Line and Ctrl+G open a bar beside the find
bar, built the same way and for the same reason: a modal dialog is a heavy
answer to "which line?". `Editor::go_to_line` reports whether the line existed,
so an out-of-range number moves the caret to the end *and* says "there are only
42 lines". **live** in both views.

## Tab strip context menu

Right-click a tab. The rows act on the tab that was clicked, not the active
one — the target is recorded when the menu opens, because clicking a row is a
separate event, by which time the pointer has moved.

| Row | State |
| --- | --- |
| Close Tab, Close Other Tabs, Close All Tabs | **live** — each goes through the same unsaved-changes prompt as closing one |
| Copy Full Path | **live** — greyed for a document that has never been saved |

## Editor context menu

Right-click in the text, or press the Menu key or Shift+F10 (release plan
M1-18, locked requirement 11). **live** in both editor views, and not over
the viewer. A right-click does not move the caret, so Cut and Copy act on
the selection there already is. The keyboard opens it just below the caret.

| Row | State |
| --- | --- |
| Cut, Copy, Paste, Select All | **live** — the same ids as Edit's, so they go to the widget under `TextInput` and to `bp-editor` under `--editor-view` |
| Paste Special... | **live** — as in Edit |
| Clipboard History ▸ | **live** — Edit's clipboard rows and transformations, built by the same function from the same snapshot, with the entry's kind in the label because a context menu has no hint column |
| Pin ▸ | **live** — one row per entry shown, ticked when pinned; choosing one pins or unpins that entry, found by its fingerprint (U5) |
| Clear Clipboard History | **live** — as in Edit |

Slint's `ContextMenuArea` opens itself on a right-click that reaches it and
on the Menu key, and says nothing when it does. Neither view lets it: the
custom surface's `TouchArea` and the `TextInput` view's `Flickable` both take
every button, so each view catches the right-click (and both catch the Menu
key and Shift+F10) and opens the menu through one function that takes the
clipboard snapshot first.

## Drag and drop to open

**Blocked on Slint, not on effort.** specs §4 wants files dropped onto the
window to open. `DropArea` exists and compiles, but on Slint 1.17.1 the winit
backend has no file-drop plumbing and `DataTransfer` carries only plain text
or an image — there is no channel a file path could arrive through. It works
for drags that start inside a Slint window and nowhere else.
