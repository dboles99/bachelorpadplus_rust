# Architecture

## Dependency rule

```text
UI
 ↓
Core Editor
 ↓
Files / Formats / Search / Storage
 ↓
Optional Semantic / Research / Notebook Services
 ↓
Security Policy Constraints
```

The UI may call services through typed interfaces. The semantic layer may not
become a prerequisite for basic text editing (ADR-0006). `bp-core` depends on
one other `bp-*` crate, `bp-security`, because a security profile is a
property of a document (ADR-0011, ADR-0020) and travels with it;
`bp-security` depends on none. Everything else may depend on `bp-core`.

## Crate map

This is the intended shape of the workspace. **Crates marked *live* or
*built* exist on disk and are workspace members** — *built, unwired* means the
crate is written and tested but nothing in the shell reaches it yet. Crates
marked *planned* are the plan, not the build.
An empty crate is a compile unit and a merge surface for a structure nothing
has exercised, so a crate gets created when there is something to put in it,
and this table carries the intent until then.

| Crate | Status | Purpose | Phase |
| --- | --- | --- | --- |
| `bp-core` | **live** | Document, Workspace, SaveState, encoding and line endings. Holds no text. | 1 |
| `bp-naming` | **live** | Semantic filename grammar (ADR-0003), and the date and time formats the Insert menu writes into documents. Pure: no filesystem, no clock. | 1 |
| `bp-files` | **live** | Atomic save, load, encoding detection, external-change stamps (ADR-0007). | 1, 3 |
| `bp-theme` | **live** | Palettes as data (ADR-0009). Green is the default. | 1, 17 |
| `bp-config` | **live** | Settings precedence, config file, recent-files list, recovery from bad input. | 1 |
| `bp-ui` | **live** | The Slint application shell (ADR-0015). Split into modules — see below. | 1 |
| `bp-buffer` | **live** | Rope buffer, character indices, line/column maths. Plus the large-file engine (ADR-0027): detection, chunked reading, a sparse line index and line-aligned streaming windows. The engine backs the read-only viewer tab a file over the 32 MiB editable limit opens in (release plan M1-06). | 2, 4 |
| `bp-editor` | **live** | Caret, selection, motion, transaction-based undo/redo, line operations, key-to-command mapping, document-to-screen geometry. The editor's storage — see below. | 2 |
| `bp-history` | **live** | Crash-safe recovery journal and autosave checkpoints. | 3 |
| `bp-formats` | **live** | Format detection and profiles (ADR-0008). | 5 |
| `bp-data` | **live** | Structured-data operations: validate, format, minify, convert, report. | 6 |
| `bp-search` | **live** | Literal and regex find/replace, plus recursive cross-file search. | 7 |
| `bp-semantic` | **live** | Deterministic extraction: titles, keywords, summaries, outlines, document statistics. Layer one only. | 8 |
| `bp-clipboard` | **live** | Clipboard history, kind detection and format-aware paste transformations (ADR-0010). In memory only. | 11 |
| `bp-organize` | planned | Projects, topics, tags, related notes, duplicate detection. | 9 |
| `bp-storage` | **live** | SQLite metadata store and migrations (ADR-0019). Documents, tags. Nothing writes to it yet. | 9 |
| `bp-notebook` | **built, unwired** | Cell model and `.ipynb` interchange (ADR-0025). Eight cell kinds, split/merge/move/duplicate/collapse, outputs as data. A run needs a `UserGesture` no parsed file can produce. Pure. | 12 |
| `bp-execution` | planned | Runners and execution security. Never auto-runs (ADR-0011). | 12 |
| `bp-research` | **built, unwired** | BibTeX and CSL JSON in and out, DOI and arXiv identifiers, four reference styles, the research profile. Offline and deterministic; a test forbids a setting that could reach the network (ADR-0006). Pure. | 13 |
| `bp-security` | **live** | Security profiles resolving to a policy over seven axes, plus Privacy Mode (ADR-0020). Decides policy; performs none of it. Read by the document model, the journal, the clipboard, the metadata store and the audit history. | 14, 16 |
| `bp-crypto` | **live** | The `.bpadx` envelope (ADR-0021): Argon2id, XChaCha20-Poly1305 and AES-256-GCM, chunked with the header and chunk position authenticated. Plus document hashing and detached Ed25519 signatures (specs §15). Composes primitives, implements none. Reached from Security ▸ Encrypt Document and from opening a `.bpadx`; the signing half from Security ▸ Sign Document and Verify Signature, through `bp-integrity` (release plan M2-10). | 15, 16 |
| `bp-secrets` | **live** | Secret scanning (specs §15): AWS/GitHub/GitLab/Slack tokens, PEM blocks, JWTs, connection strings, high-entropy assignments. Reports where a secret is, never what it is. No dependencies. Reached from Security ▸ Scan for Secrets. Platform key protection is deferred to 1.x (ADR-0030 S4). | 16 |
| `bp-integrity` | **live** | `.sig` sidecars, signing-key files and hash manifests over `bp-crypto` (ADR-0026). Says what it cannot enforce per platform rather than implying it did. Reached from Security ▸ Sign Document and Verify Signature. | 16 |
| `bp-redaction` | **live** | Irreversible redaction with merging spans, and metadata inspection (ADR-0028). Documents exactly what verification cannot prove. Pure. Reached from Security ▸ Redact Found Secrets and Inspect Metadata. | 16 |
| `bp-audit` | **live** | Security audit history (ADR-0024). An event is `Copy`, so it cannot own a secret; documents are named by an opaque id, never a path. A sealed log is written through `PassphraseSealer`, one `.bpadx` envelope per line; a profile that wants a sealed log and has no key gets no log and a notice, never a plaintext one. Read back by Security ▸ Security History. | 16 |
| `bp-platform` | **built, wired** (Set as Default, Remove Registration, the capability matrix) | The seam ADR-0001 requires, and it carries rules rather than only traits: path legality per platform (reserved device names, forbidden characters, the length limits each platform counts in its own unit), config/data/cache directories from an injected environment, a capability register, and default-editor registration -- state, plan, artefacts. ADR-0012 is mechanised: a plan that would seize an association or touch Notepad is refused. Two `cfg` attributes in the whole crate, so its tests run on both legs. Windows registration is two UTF-16LE `.reg` files (register, and an unregister that deletes only the product's own keys and its own values from shared ones), the Settings deep link by build number, and a `ProgIdQuery` trait for "what opens `.txt` now" that the Windows crate implements. | 18 |
| `bp-platform-windows` | **built, wired** | The one crate allowed `unsafe` (ADR-0033): safe functions over `AttachConsole`, `GetStdHandle`, `MessageBoxW`, `SHChangeNotify`, `ShellExecuteW` (`ms-settings:` only) and `RegGetValueW` (reads only; ADR-0032). `windows-sys` only under `cfg(windows)`; elsewhere every function answers "no", and the pure helpers are tested on Linux. The binary uses the console and crash-box calls; Set as Default reads what opens each type now and opens the Settings page (release plan M2-04). Not yet run on Windows. DPAPI and Hello are deferred to 1.x: the signing key is kept in a passphrase envelope instead (decision 16). | 18 |
| `bp-platform-linux` | **not created, deliberately** | Same reasoning. `.desktop` and MIME registration are in `bp-platform` and run on both legs; the shell runs the three commands that finish it. Secret Service is deferred to 1.x with DPAPI. | 18 |

## Current dependency edges

```text
bachelorpad ──┬─> bp-config
              ├─> bp-theme
              ├─> bp-ui
              └─> bp-platform-windows ──> bp-platform   (windows-sys on Windows only)

bp-ui ──┬─> bp-core, bp-buffer, bp-editor, bp-files, bp-formats, bp-naming,
        │   bp-history, bp-config, bp-theme
        ├─> bp-search, bp-semantic, bp-data, bp-clipboard
        ├─> bp-security, bp-crypto, bp-secrets, bp-integrity, bp-redaction,
        │   bp-audit
        ├─> bp-platform, bp-platform-windows
        └─> slint, rfd, arboard

Between the library crates:

bp-core ──────> bp-security
bp-files ─────> bp-core, bp-buffer, bp-naming, bp-platform
bp-editor ────> bp-buffer
bp-config ────> bp-platform
bp-history ───> bp-crypto, bp-security
bp-audit ─────> bp-crypto, bp-security
bp-integrity ─> bp-crypto
bp-clipboard ─> bp-security
bp-storage ───> bp-security
```

`bp-ui` reaches twenty-one workspace crates. The three library crates it does
not reach are `bp-storage`, `bp-notebook` and `bp-research`: nothing in the
shell uses them yet. `bp-integration-tests` dev-depends on most of the workspace and
is depended on by nothing. Regenerate this from
`cargo metadata --no-deps --format-version 1` rather than by hand.

**Fourteen crates depend on nothing else in the workspace**: `bp-security`,
`bp-naming`, `bp-theme`, `bp-buffer`, `bp-formats`, `bp-data`, `bp-search`,
`bp-semantic`, `bp-crypto`, `bp-secrets`, `bp-redaction`, `bp-notebook`,
`bp-research` and `bp-platform` (whose tests borrow `bp-formats`; the crate
itself does not). The rest of the library crates depend only on those and on
each other, never on `bp-ui`. That is what keeps them cheap to test and
impossible to entangle with the UI toolkit — and it is why every test outside
`bp-ui`'s `window_tests.rs` runs without a window.

Two deliberate non-dependencies:

- `bp-config` does not depend on `bp-theme`. A theme name from a user's file
  has to be recoverable when it is wrong, so it stays a string until the
  application resolves it.
- `bp-semantic` does not depend on `bp-formats`. Extraction reads text; asking
  it what kind of file it is would couple two layers that have no reason to
  know about each other.

## Inside `bp-ui`

The shell was one 2,675-line file and the single-writer bottleneck for every
piece of wiring work. It was split into five modules along seams the file
already had as comment banners, and each feature since has taken a module of
its own rather than growing one of those five:

| Module | Owns |
| --- | --- |
| `state.rs` | `AppState`: documents, workspace, saving, reloading, format detection, status labels, find matches, and the Security and Data menus' work |
| `editor_view.rs` | The custom surface: key translation, caret placement, what to draw, `TAB_WIDTH` |
| `dispatch.rs` | The menu-action match, and the dialogs its arms share |
| `menus.rs` | Menu contents and the action-id map, including the editor context menu |
| `lib.rs` | `run_with`, `refresh`, and the Slint callback wiring |
| `viewer.rs` | The large-file viewer tab (M1-06): what is on screen, and the reads and finds kept off the UI thread |
| `worker.rs` | `Background`: one kind of slow job on a thread of its own (M1-07) |
| `dialog.rs` | Every in-window question: what it says and what happens once it is answered (M1-01) |
| `passphrase.rs` | Which of the passphrase bar's questions is being asked, and what to do with the answer |
| `settings_sheet.rs` | The Settings sheet's model: the settings, their values, and what saving does (M1-03) |
| `about.rs` | Help ▸ About: version, commit, renderer, platform, licences (M1-02) |
| `audit.rs` | Reading the security history back for Security ▸ Security History |
| `default_editor.rs` | Set as Default Editor and Remove Registration; every decision is `bp-platform`'s (M2-04, M2-06) |
| `window_tests.rs` | The editor in a real window — a software-rendered one, no display needed — driven by real key events |

`ui/app.slint` took the same treatment for the same reason. It was one
1,189-line file; it is now fourteen:

| File | Owns |
| --- | --- |
| `types.slint` | The six structs and the `Palette` global — the boundary `slint-build` generates into Rust |
| `menu.slint` | `MenuLabel`, `MenuRow`, `MenuPopup`, and the `MenuNav` global |
| `editor_surface.slint` | `EditorSurface`: the custom view's drawing, measurement and input |
| `viewer.slint` | `ViewerPane`: the large-file viewer's banner, lines (drawn by `EditorSurface`) and scrollbar |
| `tab.slint` | One tab |
| `find_bar.slint` | Find and replace, including `focus-query` |
| `goto_bar.slint` | Go to line |
| `passphrase_bar.slint` | The one-field passphrase bar |
| `results_panel.slint` | Cross-file search results |
| `status_bar.slint` | Both status rows |
| `dialog.slint` | The in-window question sheet |
| `settings.slint` | The Settings sheet |
| `about.slint` | Help ▸ About, hosting Slint's `AboutSlint` attribution (ADR-0031) |
| `app.slint` | The window's properties and callbacks, the shortcut bindings, the menu bar and its popups, the tab and editor context menus, both editor views, the tab strip, and the layout |

Three things stayed in `app.slint` deliberately, and they are what a wiring
item may still have to serialise on:

- **The menu bar and its popups.** A `PopupWindow`'s position is relative to
  its parent element, and the popups are placed from their labels' `x`. Moving
  both into a component changes the coordinate parent — a visual change
  nothing in the gate can catch.
- **Both editor views**, because `dispatch`, `select-range` and
  `forward-focus` name `editor` and `surface` directly, and a parent cannot
  name an element inside a child component.
- **The tab strip**, a `for` over `root.tabs` and three lines of layout.

Two rules the modules run on:

- **Action ids are blocks, and the dispatch matches ranges.** An id in the
  wrong block silently does something else. Every range arm is bounded at both
  ends; the unbounded one that used to exist would have swallowed any id
  allocated above 300.
- **`bp-ui` connects; it does not decide.** An `if` in here about what a
  feature should *do* belongs in a library crate, where it can be tested
  without a window.

## The editor-view boundary

**The rope is the storage.** `bp-ui` holds
`HashMap<DocumentId, bp_editor::Editor>`, whichever view is drawing. That was
the change ADR-0018 made, and it is what unblocked phase 2.

| Document | insert via `String` | insert via rope |
| ---: | ---: | ---: |
| 100 KB | 55.8 µs | 0.3 µs |
| 1 MB | 563.7 µs | 0.3 µs |

There are two views over it:

| | `TextInput` (default) | `EditorSurface` (`--editor-view`) |
| --- | --- | --- |
| Caret and selection | Slint's, read into `bp-editor` (M1-04) | `bp-editor`'s |
| Undo | Slint's | `bp-editor`'s transactions |
| Status bar | Ln/Col | Ln/Col |
| Word wrap | yes | yes (`bp_editor::wrap`) |
| Input-method composition | yes | not possible on Slint 1.17.1 |
| Lines drawn | all of them | only the visible ones |

`TextInput` owns its own text, caret and undo stack and exposes the caret only
through a property marked *"internal, undocumented, only exposed for tests"*,
which is why text arriving from it lands in the rope as a single undoable
replacement rather than as keystrokes. Since release plan M1-04 the shell
reads that caret anyway and copies it into `bp-editor`, so the line
commands and the Ln/Col readout work in both views.

The custom surface exists to own none of those things either. Every decision
is in `bp-editor` — motion in `Editor`, chords in `keys`, document-to-screen
arithmetic in `view` — and the widget draws what it is handed and reports rows,
columns and key text back. That split is what lets an editor's behaviour be
checked by `cargo test`; what is left needing a person is genuinely visual.

The rule the boundary runs on: **`bp-editor` deals in rows and visual columns,
never pixels.** The toolkit knows its own font, so it converts. One place for
the two to disagree instead of two.

Word wrap was the parity item with real depth, because it makes a visual
row differ from a document line. It is done: `bp_editor::wrap` owns the
breaking rule, `view`'s wrapped half maps rows to characters, and `Editor`
moves Up and Down by visual row. What is left is input-method composition,
and Slint 1.17.1 gives a `FocusScope` no route to pre-edit text, so it waits
on a Slint upgrade (see `project/WORK_QUEUE.md`).
