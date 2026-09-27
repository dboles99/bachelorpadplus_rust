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
3. Run the gate before saying it is done: `./scripts/Invoke-LocalCI.ps1 -Linux`
   on Windows (both legs, the second in WSL), or
   `pwsh -NoProfile -File scripts/Invoke-LocalCI.ps1` on a Linux host, where
   every stage runs natively (launch under `xvfb-run -a` when there is no
   display). Never share one `CARGO_TARGET_DIR` between two worktrees of this
   repository: cargo names workspace crates by their path relative to the
   workspace root, so one tree's build is silently reused by the other.

## Collision rules

`bp-ui/src/lib.rs` has been split, so `state.rs`, `editor_view.rs` and
`dispatch.rs` can take different agents. `ui/app.slint` has now been split too
— into seven files at first (T1), fourteen as of 2026-09-27 (`ls
crates/bp-ui/ui`). So the find bar, the editor surface, the status bar, the
results panel, the tab, the dialog sheet, About, Settings, the viewer and the
Go to Line and passphrase bars are each somebody's own file.

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
| — | `bp-notebook` keeps a cell that names a foreign language `Raw` | ADR-0025, `ipynb.rs` `infer_kind` |
| — | `bp-files` spells a long path `\\?\` for the rename `tempfile` does raw | `bp-files` `save.rs`, `SaveError::PathTooLong` |
| — | Recent list, recovery journal and security history leave the roaming directory | ADR-0029, `bp_config::Locations` |
| — | `Profile::is_data` says what it answers | `bp-formats` |
| — | Page Up / Page Down, and a view that follows the caret, in the default view | `app.slint`, `bp-ui` window tests |
| — | Insert toggles overwrite; Shift+Insert, Ctrl+Insert, Shift+Delete | `bp_editor::{overwritten, Editor::type_text}`, both views |
| M0-01/02 | Recovered work is never discarded unasked; a second window cannot touch the first's journal | `bp-history` instance folders, `state::recovery_choice` |
| M0-03 | Bundled DejaVu Sans Mono (ADR-0032) | `bp-ui/fonts`, `app.slint` |
| M0-04 | Files past 32 MiB are refused before being read (superseded by M1-06: they open in the read-only viewer) | `bp_files::load_with_limit`, `LoadError::TooLarge` |
| M0-05 | JSON key order kept; Sort Keys sorts; TOML Format warns | `bp-data`, `serde_json/preserve_order` |
| M0-06 | One command-line parser | `bp_config::cli` |
| M0-08/09 | Stale readouts fixed; recent list honours the profile | `menus.rs`, `state.rs` |
| M0-13 | The gate runs natively on a Linux host | `scripts/Invoke-LocalCI.ps1` |
| M0-07 | Clipboard history records the editor's copies only; private copies kept out of OS history | `bp_clipboard::{Origin, keep_out_of_os_history}`, `copied-from-editor` |
| M0-10 | Log file and crash report | `apps/bachelorpad/src/logging.rs`, `bp_config::locations::log_dir` |
| M0-09/11 | Atomic recent-list write; `forbid(unsafe_code)` everywhere but `bp-ui`'s generated code | `bp-config`, `bp-security`, the binary |
| M1-01 | Every question is a sheet in the window; `rfd` keeps the file pickers only | `ui/dialog.slint`, `src/dialog.rs` |
| M1-10 | Windows-1252 opens, saves back byte for byte, and a character it cannot hold offers UTF-8 | `bp_core::encode_windows_1252`, `bp_files::try_encode`, `lib.rs::offer_utf8` |
| M1-15 | An opened file keeps its real creation date | `bp_naming::creation_date_for`, `state::created_at` |
| M1-21 (outline) | The outline ignores fenced code | `bp-semantic` |
| M1-03 | View and Format choices saved into config.toml, comments kept; Tools ▸ Settings… sheet | `bp_config::save`, `AppState::{configure, remember}`, `settings_sheet.rs` |
| M1-02 | About sheet with `AboutSlint`, build commit, renderer, licences | `ui/about.slint`, `about.rs`, `build.rs` |
| M1-04 | Default view: Ln/Col, Go to Line, Insert Date/Time, line moves | `follow_widget_caret`, `at_the_caret`, `Buffer::byte_to_char` |
| M1-05 | No placeholders on a release bar; Tools live | `menus::{readout, tools}`, `show_planned_menus` |
| M1-07 (1 of 2) | Find in Files off the UI thread, stoppable | `worker::Background`, `bp_search::search_dir_until` |
| M1-11 | Multi-select Open; Ctrl+Tab | `AppState::{open_many, cycle_tab}` |
| M1-13 | Release profile, pinned toolchain, crt-static, trimmed features | `Cargo.toml`, `.cargo/config.toml` |
| M1-16 | A notice never hides the path | `status_bar.slint` |
| M1-17 | Collision-free Save As name; Save as New Revision | `suggested_filename`, `next_revision_path` |
| M1-19 (store) | A document's profile survives a restart | `bp_history::ProfileStore` |
| M1-21 | Outline ignores fences; Follow System says when the desktop is silent | `bp-semantic`, `menus::view` |
| M1-06 | Files over the editable limit open read-only in a viewer tab with a banner, as `--readonly` and `--large-file` do; lines are indexed and searched on background jobs. The 1 GB measurement is M4-02 | `ui/viewer.slint`, `src/viewer.rs`, `bp_buffer::LargeFile` |
| M1-09 | D6: LF, CRLF and a bare CR end a line everywhere; the pinned integration test is un-ignored | ropey `cr_lines`, `bp-search`, `tests/integration` |
| M1-14 | Licence files (holder Daniel Boles), generated third-party notices, `cargo deny` in the gate | `LICENSE-*`, `THIRD-PARTY-NOTICES.txt`, `deny.toml`, gate stage `deny` |
| M1-18 | Editor context menu in both views (right-click, Menu key, Shift+F10) with Pin and Clear; clipboard rows snapshot on open (U5) | `menus.rs`, `app.slint`, `bp_clipboard::toggle_pin` |
| M2-01 | Icon set, approved by the owner | `assets/icon/` |
| M2-02/03/04 | Windows resources, `bp-platform-windows`, a `.reg` pair from Set as Default ▸ Save Files…, Settings deep link. **Built, never run on Windows** | `apps/bachelorpad/build.rs`, `crates/bp-platform-windows`, `bp-platform` |
| M2-05/07 | Desktop entry, MIME package and AppStream metadata, generated and held by a drift test; app id `io.github.dboles99.BachelorPad` | `packaging/linux/`, `bp-platform` |
| M2-06 | Linux Set as Default runs its commands with consent; Remove Registration | `default_editor.rs`, `bp-platform` |
| M2-08 | One clipboard per process, Wayland data-control, a private copy cleared after a minute | `bp_clipboard::PendingClear` |
| M2-09 | Argon2 ceiling: refuse more than 256 MiB, 8 passes or 16 lanes; ask above 4x the default | `bp-crypto` `header.rs`, ADR-0021 amendment |
| M2-10 | Sign Document, with a passphrase-sealed key made on first use | `bp-integrity`, `signing-key.bpadx` |
| M2-11 | Nothing leaves the machine | ADR-0034, the Security readout |
| M2-12 | `PLATFORM_MATRIX.md` generated, with a drift test | `capabilities::matrix_markdown` |
| M2-13 | Records sweep 2: template task files deleted, phase READMEs carry status | `project/tasks/` |
| M2-14 | Accessibility minimum: F10 and Alt+letter open the menus; roles and labels | `menu.slint`, `app.slint` |
| M2-15 | A startup notice counts encrypted documents with recovered work | `Journal::sealed_count` |
| M2-16 | `bpad` alias | a symlink in the Linux packages, `bpad.cmd` in the installer and zip |
| M3-01 | Per-user Inno Setup installer and portable zip (no `.reg` files; Save Files… writes them). **Built, never run on Windows** | `packaging/windows/` |
| M3-02 | `.deb`, `.rpm` (rpmbuild, no maintainer scripts), `tar.gz`, `PKGBUILD`. Tested on Ubuntu 22.04 and 24.04 only; the pinned Debian 12 image has never been built | `packaging/linux/` |
| M3-03 | Release script; a native Linux dry run is recorded (glibc 2.39, over the ceiling) | `scripts/Invoke-Release.ps1`, `artifacts/releases/v1.0.0-rc.0/linux/` |
| M3-04 | Windows signing wired, through Artifact Signing or an OV certificate. **No identity; never run** | `packaging/windows/Signing.ps1`, `-Sign` / `-AllowUnsigned` |
| M3-05 | User docs: README, guide, CHANGELOG, SECURITY.md, RC checklist | repo root, `docs/user/`, `docs/release/` |
| M3-06 (in part) | Version 1.0.0-rc.0 for the dry runs; rc.1 is cut after M4 | workspace `Cargo.toml` |
| M3-07 | The dormant hosted workflow runs the gate script itself | `.github/workflows/ci.yml` |

Every library half now has its row.

## Tier 0 — three crates with no way in

`bp-notebook`, `bp-research` and `bp-storage` are named by no `Cargo.toml` in
the application (only `tests/integration`, and for `bp-notebook` the fuzz
harness). ADR-0030 keeps all three unwired for RC1, so nothing in this tier
is release work. (The large-file viewer that used to head it is done:
M1-06.) Every item is the same shape: a tested library and a missing way
in. Read `prompts/rosettas/R011_WIRE_CAPABILITY_INTO_SHELL.md` before taking
any of them. **Only one agent may add to `AppWindow` in `app.slint` at a
time**, so batch by menu.

| Item | What exists |
| --- | --- |
| Notebook mode | `bp-notebook` (ADR-0025). A new view, not a menu row. Deferred to 1.1 by ADR-0030 |
| Research mode | `bp-research`. Same: a mode, not a row. Not in RC1 (ADR-0030) |

## Tier 0.5 — what the cross-crate tests found

Eleven files of cross-crate tests have found seven defects, and **all seven
are fixed** -- the sixth session took the notebook fallback, the
extended-length path and the roaming state, and M1-09 took the last:

| Item | Shape |
| --- | --- |
| ~~`bp-search` and `bp-buffer` disagree what a line is~~ | **Fixed by M1-09.** D6 was answered: a line ends at LF, CRLF or a bare CR, everywhere (ropey `cr_lines`), and the pinned test in `search_over_buffer.rs` is un-ignored. What is left is line-ending *conversion*, which still treats a bare CR as data: D11, under "Follow-ups from M3" |

## Tier 0.75 — what the window tests found

`bp-ui` has tests that drive the real window now (`src/window_tests.rs`),
on a headless software renderer with real key events. They found two
defects in their first hour, both fixed. Found alongside them, or since:

| Item | Shape |
| --- | --- |
| ~~**`--editor-view` draws its caret right of the text on Linux**~~ | **Fixed by M0-03**: DejaVu Sans Mono is bundled with `import` and named by every text element that used to ask for Consolas, on every OS (ADR-0032). Not yet re-checked by a person on Linux; the window tests send keys and do not place the pointer. What was recorded: `EditorSurface` measures one advance and multiplies it by the column, which holds only for a monospace font. It asks for `"Consolas"`; Slint resolves a family name literally (no fontconfig aliasing, no generic `monospace`), so on Linux it falls back to a proportional sans and the caret drifts further right with every character. Found by driving the app on Xvfb. Fix is a product choice: bundle a monospace font with `import`, or pick the family per platform in Rust. Windows is unaffected. (Answered as U2: bundle one.) |
| ~~**`--editor-view` loses every key typed before the first click**~~ | **Fixed**: `run_with` gives the editor the keyboard once the window is shown; the smoke script reported XPASS and its `KNOWN_DEFECTS` entry is gone. What was recorded: Launch `bachelorpad --editor-view file.txt`, type, press Ctrl+S: nothing is typed and nothing is saved. Click in the text and the same keys work; the default view takes keys at once. `app.slint` has `forward-focus: editor` -- the `TextInput`, hidden in this view -- and nothing calls `focus-editor` on launch. The window tests call it themselves, so they cannot see this. The keys are dropped, not stored invisibly (checked). Found by `scripts/smoke-linux.sh`, which reports it as `XFAIL`; delete its `KNOWN_DEFECTS` entry with the fix. The cause is in `.slint`, so every platform is probably affected; seen on X11 |

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

- **`Command::Indent` is not `Command::Insert("\t")`.** What a soft tab
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
| ~~Security ▸ Sign Document~~ | **Done** (release plan M2-10, decision 16 = D5 option c). The key is made on first use and kept at `signing-key.bpadx` in the data directory, sealed with a passphrase the bar asks for twice. The public half is `signing-key.pub` beside it. The row is live for a saved document. |
| Anything writing to `bp-storage` | **Still a decision, but a narrower one.** Phase 14 exists now and `record_document` honours the policy, so the security half is answered. What remains is ADR-0019's product judgement plus a design pass. Do not "just wire it up". |
| `bp-organize` (planned in `ARCHITECTURE.md`, not created) | The above, plus a design pass. Deferred to 1.2 by ADR-0030 (S2). |
| ~~YAML support~~ | **Done and wired.** ADR-0023 picked `saphyr`; `bp-data` validates, formats, minifies and converts, with nesting, alias expansion and duplicate keys all bounded, and the Data menu reaches all of it: `bp-ui`'s `state.rs` calls `bp_data::yaml_validate`, `yaml_format`, `yaml_minify` and `yaml_to_json`. |
| Markdown preview | Slint has no HTML view. Needs a rendering decision first. |
| ~~`.bpadx` encryption~~ | **Done.** ADR-0021 settled the envelope and it ships; ADR-0022 seals the recovery journal with it. |
| Print | Platform work with no cross-platform story yet. |
| **W11, drag and drop to open** | **Slint 1.17.1**, and this was checked rather than assumed. `DropArea` exists and compiles, and the item is implemented in `i-slint-core/items/drag_n_drop.rs` -- but the winit backend has no file-drop plumbing, and `DataTransfer` exposes only `plain_text` and `image`. There is no channel a dropped file's path could arrive through. Revisit on a Slint upgrade; do not spend an afternoon on it before checking that changed. |

## Follow-ups from M2

- **The window's Wayland `app_id` / X11 `WM_CLASS` is the binary name
  (`bachelorpad`), not `io.github.dboles99.BachelorPad`.** A desktop matches
  a window to its `.desktop` entry, and so to its icon in the dock, by that
  id. Setting it needs Slint's `unstable-winit-030` feature
  (`BackendSelector::with_winit_window_attributes_hook`), an API that pins
  the shell to Slint's winit version. The alternative is `StartupWMClass=bachelorpad`
  in the generated entry, which GNOME and KDE both honour. This was to be
  decided before M3-02 packaged the entry; M3-02 shipped with neither, so
  it is still undecided and the dock may show a generic icon.
- **Set as Default runs its commands on the UI thread.** They are short
  (`update-mime-database` on a per-user tree is the longest), but a hung
  `xdg-mime` would freeze the window. `worker::Background` is the pattern if
  that is ever seen.
- **Windows: none of M2-02/03/04 has run on Windows.** The owner's gate run
  checks VERSIONINFO now. The things to watch are listed in the plan's M2
  progress note.

## Follow-ups from M3

- **The editor context menu ignores the theme (M1-18).** It is Slint's
  own fluent popup, light under Dark, Organic and Green, and near the
  screen's right edge its submenus can be cut off (seen under Xvfb with no
  window manager). Styling it means a custom popup or a Slint style
  override; check the cut-off under a real window manager first.

- **A `.bpadx` opened from Recent Files, the command line or a Find in
  Files hit never gets the passphrase flow.** Those paths call
  `AppState::open`, not `open_maybe_encrypted`. It derives nothing, so it
  is not a cost risk (M2-09), but the file cannot be unlocked that way.
  Pre-existing; found during M2-09.
- **Line-ending conversion and the bare CR (D6 follow-up).** "Convert to
  LF" leaves a CR-only file's CRs in place, and `LineEnding` has no CR
  variant, so a CR-only file is labelled as the platform default. Waits on
  D11 in `DECISIONS_NEEDED.md` (recommended: yes to both).
- **Slint's default TextInput view draws line breaks by its own rules**
  (probably also U+2028/U+2029, maybe FF). The custom view draws exactly
  the counted lines; check the default one in a render.

- **Nothing packaged for Windows has run on Windows.** On the owner's
  machine: `packaging/windows/Build-Installer.ps1`, then
  `Test-Install.ps1` on the Windows 10 and 11 VMs. ISCC has never
  compiled `bachelorpad.iss`, so its `[Code]` Pascal is unproven.
- **The Debian 12 release image has not been built.** Every Debian, Fedora
  and Arch mirror is unreachable from the build container used so far.
  The Dockerfile was proven with `--base-image ubuntu:22.04` (glibc 2.35).
  The first real `Invoke-Release.ps1 -Container` run is also the first
  build of the pinned Debian image and of its cargo-auditable install.
- **`test-packages.sh` has run on Ubuntu 22.04 and 24.04 only.** Fedora and
  Arch need a host that can reach their mirrors.
- **The installer's "Add bpad to PATH" task is the installer agent's
  addition** (unchecked by default, removed on uninstall). Keep it or drop
  it; the plan did not ask for it.
- **M3-04, Windows signing, is wired but has never signed anything.**
  `packaging/windows/Signing.ps1` takes Artifact Signing or an OV
  certificate from environment variables; it needs an identity (decision
  3, eligibility question 3a in `DECISIONS_NEEDED.md`) and a first run on
  Windows. Until then a Windows release needs `-AllowUnsigned` and records
  `windows_signed: false`.
- **`artifacts/test-evidence/*.json` is ignored** by `.gitignore`, which
  clashes with committing RC evidence there. The RC checklist uses per-tag
  subfolders to get around it; decide which is meant before M4.
- **`SLINT_BACKEND=winit-x11`**, named in the plan's risk R8, is probably
  wrong for Slint 1.17: its parser reads that as backend `winit` with a
  renderer called `x11`. Check before documenting it anywhere.

## Not a queue item

The first thing in `NEXT_SESSION.md` is a person typing into `--editor-view`
and writing down what feels wrong. No agent can do that. Whether the caret
lands where the pointer does has been answered once, for Linux on Xvfb: it
did not, because "Consolas" fell back to a proportional font (X1). The
bundled DejaVu Sans Mono (M0-03, ADR-0032) removes that cause, but nobody
has re-checked it since -- the headless window tests do not place the
pointer -- so it is **not yet verified** on either OS. It matters less than
it did: since M1-04, W1, W5 and W8 also work in the default `TextInput`
view, so only W7 and the custom view's own caret depend on the answer.
