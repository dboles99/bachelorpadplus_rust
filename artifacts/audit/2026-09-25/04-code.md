# Audit 04: What the code says is unfinished or unreachable

Workspace: `/home/user/bachelorpadplus_rust`, branch `claude/compassionate-wright-wwclxq`. Read-only audit, 2026-09-25.
Method: I read the manifests and grepped for `bp_<crate>::` references. I also built a "production corpus" of every `src/*.rs` with `#[cfg(test)] mod … {}` blocks and `window_tests.rs` removed. Each crate's `pub fn/struct/enum/trait/type/const` names were then checked against that corpus, excluding the crate itself. The name matching is heuristic. Error types used only through `?`/`Display`, and methods whose names collide with other code, can be misclassified. Every item called out in the prose below was checked by hand.

---

## 1. Reachability map

"UI refs" means production (non-test) references to `bp_<crate>::` in `crates/bp-ui/src` and `apps/bachelorpad/src`. `bp-ui` itself depends on 20 of the 23 library crates. `apps/bachelorpad` depends only on `bp-config`, `bp-theme` and `bp-ui`.

| Crate | src LOC | Reverse deps (Cargo.toml) | Reached by shipped binary? | How reached | Pub items / unused outside crate (heuristic) |
|---|---|---|---|---|---|
| bp-audit | 2630 | bp-ui, integration | Yes | Security History, sealed audit log | 35 / 6 |
| bp-buffer | 2854 | bp-editor, bp-ui, integration | **Indirect only.** The rope is reached through `bp-editor`. bp-ui's only direct call is `Buffer::from_text` in `--latency-probe` (lib.rs:1108) | rope `Buffer` | 82 / **53** |
| bp-clipboard | 946 | bp-ui, integration | Yes | Edit ▸ clipboard history + transforms | 21 / 4 |
| bp-config | 2130 | bp-ui, bachelorpad, integration | Yes | config load, recent list, locations | 40 / ~16 (mostly false positives: `Loaded`/`Env` are used by value) |
| bp-core | 822 | bp-files, bp-ui, integration, fuzz | Yes | Workspace/Document | 54 / 3 |
| bp-crypto | 2027 | bp-audit, bp-history, bp-integrity, bp-ui, integration, fuzz | Yes | Encrypt/open `.bpadx`, hash | 34 / 4 |
| bp-data | 2002 | bp-ui, integration, fuzz | Yes | Data menu | 32 / 9 |
| bp-editor | 4608 | bp-ui, integration | Yes (rope is the storage in both views; caret/undo only under `--editor-view`) | Editor, Command, view:: | 90 / 35 |
| bp-files | 2278 | bp-ui, integration, fuzz | Yes | load/save/check | 22 / 10 |
| bp-formats | 743 | bp-platform, bp-ui, integration, fuzz | Yes | `detect`, `Format` | 11 / 4 |
| bp-history | 802 | bp-ui, integration | Yes | recovery journal | 17 / 2 |
| bp-integrity | 3477 | bp-ui, integration | **Verify half only** | `verify_file`, `sidecar_path`, `Expectation` | 49 / **22** |
| bp-naming | 1002 | bp-files, bp-ui, integration | Yes | Insert ▸ Date/Time, `SemanticName` | 28 / 13 |
| **bp-notebook** | 3583 | integration, fuzz only | **NO.** bp-ui does not depend on it | none (Notebook menu is a placeholder) | 72 / 47 (everything) |
| bp-platform | 5626 | bp-config, bp-files, bp-ui, integration | Yes | `editor::{plan,install}`, `dirs`, paths | 121 / **61** |
| bp-redaction | 2407 | bp-ui, integration | Yes | Redact / Inspect Metadata | 25 / 4 |
| **bp-research** | 5208 | **none**, not even `tests/integration` or `fuzz` | **NO**, orphan | none (Research menu is a placeholder) | 69 / 43 (everything) |
| bp-search | 984 | bp-ui, integration | Yes | find, replace-all plan, Find in Files | 23 / 8 |
| bp-secrets | 1677 | bp-ui, integration | Yes | Scan for Secrets | 8 / 1 |
| bp-security | 622 | bp-audit, bp-clipboard, bp-core, bp-history, bp-storage, bp-ui, integration | Yes | profiles, Privacy Mode | 19 / 5 |
| bp-semantic | 608 | bp-ui, integration | Yes | Note menu (title/summary/keywords/outline/stats) | 9 / 1 |
| **bp-storage** | 721 | integration only | **NO.** bp-ui does not depend on it, and its own doc says "Nothing in the application writes to this yet" (lib.rs:19) | none | 16 / 11 (everything) |
| bp-theme | 302 | bp-ui, bachelorpad, integration | Yes | palettes | 15 / 1 |
| bp-ui | 11651 | bachelorpad | Yes | shell | n/a |

`bp-storage` pulls `rusqlite` with `bundled` (C build of SQLite) into `cargo build --workspace` and the test gate, even though nothing ships it.

### Notable pub API that is built and tested but not wired, in crates that ARE reached

| Crate | Unwired public surface (checked by hand) | Consequence |
|---|---|---|
| bp-buffer | `LargeFile`, `SizeClass`, `Access`, `size_class`, `is_large`, `must_stream`, `LineIndex`, `LineLocation`, `IndexProgress`, `WindowReader`, `StreamWindow`, `DisplayLine`, all `*_BYTES` thresholds (`large.rs`, `line_index.rs`, `stream.rs` ≈ 2.6k LOC) | The large-file path does not exist in the app. `bp_files::load` (load.rs:86) calls `std::fs::read` on the whole file with no size check, and the default `TextInput` view copies the document on every keystroke. lib.rs:24-26 says it stays in frame budget only up to about 40 MB. |
| bp-integrity | `sign_file`, `write_signing_key`, `read_signing_key`, `write_verifying_key`, `key_file_protection`, `KeyFileProtection`, `Manifest`/`ManifestEntry`/`ManifestReport`/`all_match`, `hash_file`, `Sidecar` | Only verification ships. Security ▸ "Sign Document — no signing key yet" is hard-disabled (menus.rs:752). `sign_file` appears only in bp-ui tests (state.rs:4468+). Manifests (folder integrity) have no UI. |
| bp-editor | `undo`/`redo`/`can_undo`/`can_redo`, `move_caret`, `delete_*`, `select_all`, `go_to_position`, row/visual-column helpers | The UI drives the editor through `keys::command_for` + `apply`, so the direct methods are an alternative API. The caret-owning features exist only under the `--editor-view` CLI flag (main.rs:90). No config key or menu toggle turns it on. |
| bp-clipboard | `toggle_pin`, `clear_all` | The menu renders a 📌 for pinned entries (menus.rs:493), but no UI can pin or clear history. |
| bp-platform | `capabilities()`/`Availability`/`Capability::is_available_on`, `paths::{is_portable_file_name,to_portable_file_name,max_path_len,has_extended_length_prefix}`, `editor::FollowUp` details, `report_from_prog_ids` | Capability matrix is reported to nobody. Windows association query `report_from_prog_ids` has no registry reader behind it. |
| bp-naming | `format_date`, `parse_date`, `with_revision`, `with_collision`, `sanitize_extension`, `truncate_bytes` | Revision/collision naming (Untitled_…_r2 / (2)) is not used by the shell. Collision handling on save is absent at this level. |
| bp-data | `json_sort_keys`, `detect_delimiter`, `delimited_rows` | JSON "Sort Keys" row is wired to `DATA_FORMAT` (menus.rs:880). That is equivalent today only because `serde_json` `preserve_order` is off in the lockfile. `json_sort_keys` is literally `json_format` (bp-data lib.rs:109). |
| bp-search | `replace_all`, `next_match`, `previous_match` | UI uses `find_all` + `plan_replace_all`. |
| bp-files | `resolve_in_dir`, `LineEndingSurvey`/`is_mixed`/`converges_to` | Mixed-line-ending survey not surfaced. |
| bp-security | `Policy.temporary_files`, `Policy.zeroise`, `Policy.embeddings` have **no production reader** anywhere | Confidential/Maximum profiles promise `TemporaryFiles::Denied` + `Zeroise::On` (bp-security lib.rs:256/268). Atomic save still writes a temp file for every profile. This contradicts the crate's own rule at lib.rs:124-126: "an axis nothing consults reads as a guarantee while being decoration". |
| bp-config | no `Config` writer | View-menu choices (theme, zoom, wrap, gutter, indent) are not persisted. Only the recent list is saved (`recent.rs:108`, a plain `fs::write` rather than an atomic write). |

---

## 2. Menu surface (`crates/bp-ui/src/menus.rs`, `crates/bp-ui/ui/app.slint`)

### Top-level menus (app.slint:424-502, items bound at 828-961)

| Menu | Builder (lib.rs:192-247) | Status |
|---|---|---|
| File | `menus::file` | Live (Set as Default Editor is live on both OSes, with different work on each) |
| Edit | `menus::edit` | Live. 4 caret rows disabled unless `--editor-view`. Multi-cursor planned. |
| View | `menus::view` | Live; Split/Preview planned |
| Insert | `menus::insert` | **Inert in the default view.** Every Date/Time row has `enabled: editor_view` (menus.rs:851), and the rest are planned. |
| Format | `menus::format` | Live (LF/CRLF, UTF-8/BOM, indent, tab width). No UTF-16 toggle, although UTF-16 files load. |
| Data | `menus::data(format)` | Live for JSON/YAML/JSONL/TOML/CSV/TSV. **Placeholder for every other format** (`planned_menu("Data")`). |
| Note | `menus::note` | Live; Tags/Related/Revision History planned |
| **Notebook** | `planned_menu("Notebook")` | **Entirely placeholder** |
| **Organize** | `planned_menu("Organize")` | **Entirely placeholder** |
| **Research** | `planned_menu("Research")` | **Entirely placeholder** |
| **Run** | `planned_menu("Run")` | **Entirely placeholder** |
| Security | `menus::security` | Live. Sign is hard-disabled, Lock Document is planned, and there are 3 read-only readout rows. |
| **Tools** | `planned_menu("Tools")` | **Entirely placeholder** (including "Settings", so there is no settings UI) |
| Help | `menus::help` | Live (Shortcuts, About); Diagnostics planned |

**5 of 14 top-level menus are entirely placeholder.** Two more are effectively inert in common cases: Insert in the default view, and Data for non-data formats.

### `planned(...)` rows (row does not exist yet; always disabled, action NONE)

| menus.rs line | Menu | Label | Note |
|---|---|---|---|
| 489 | Edit | "Clipboard History — nothing copied yet" | Informational, only when history is empty |
| 547 | Edit | Multi-cursor | feature |
| 548 | Edit | — not implemented yet (phase 2) | trailer |
| 604 | View | Split / Preview | feature |
| 605 | View | — not implemented yet (phase 8) | trailer |
| 640 | Format | — not implemented yet (phase 5) | trailer only |
| 680 | Security | "Recovery journal: …" | Read-only readout. **Stale text**: `Recovery::Encrypted` renders "off until encryption ships (phase 15)" (menus.rs:799), but bp-history already seals the journal with the document passphrase (bp-history lib.rs:189-199). |
| 684 | Security | "Clipboard history: …" | readout |
| 690 | Security | "Leaves this machine: …" | readout |
| 785 | Security | Lock Document | feature |
| 786 | Security | — not implemented yet (phase 16) | trailer |
| 863 | Insert | Markdown constructs, citation, code block, table | feature |
| 864 | Insert | — not implemented yet (phase 6) | trailer |
| 929 | Data (fallback) | "— nothing for {format} documents" | then the whole `planned_menu("Data")` list |
| 969 | Note | Tags | feature (bp-storage has `tag_document`, unwired) |
| 970 | Note | Related Notes | feature |
| 971 | Note | Revision History | feature |
| 972 | Note | — not implemented yet (phase 9) | trailer |
| 980 | Help | Diagnostics | feature |
| 981 | Help | — not implemented yet (phase 19) | trailer |

Feature rows planned inside live menus: **8** (Multi-cursor, Split/Preview, Lock Document, Markdown constructs…, Tags, Related Notes, Revision History, Diagnostics). There are also 3 security readouts, 1 empty-history note, 7 phase trailers, and the Data fallback.

### `planned_menu(...)` whole menus (menus.rs:989-1082)

| Name | Entries | Phase | Used in production? |
|---|---|---|---|
| Notebook | Enable Notebook Mode, New Cell, Run Cell, Run All, Export .ipynb | 12 | Yes (lib.rs:243) |
| Organize | Project, Suggested Folder, Topics, Duplicate Detection, Semantic Search | 9 | Yes (lib.rs:244) |
| Research | Citation Metadata, DOI Lookup, Evidence, Datasets | 13 | Yes (lib.rs:245) |
| Run | Run Selection, Run Document, Choose Interpreter, Stop, Rust Scratchpad | 12 | Yes (lib.rs:246) |
| Tools | Document Inspector, Security Inspector, File Analysis, Benchmarks, Settings | 19 | Yes (lib.rs:247) |
| Data | Validate, Format / Minify, Sort Keys, Filter / Query, Statistics, Convert | 6 | Fallback for non-data formats (menus.rs:927). Misleading: it tells a `.txt` user that Validate/Format are "not implemented". |
| Insert | Date and Time, Code Block, Table, Citation, Cell | 5 | **Dead arm** (Insert is live). Phase disagrees with `insert()` (phase 6). |
| Note | Semantic Rename, Title and Filename, Tags, Summary, Related Notes, History | 8 | **Dead arm**. Phase disagrees with `note()` (phase 9). |
| Security | Lock Document, Encrypt as .bpadx, Privacy Mode, Scan for Secrets, Hash and Sign, Redact, Audit History | 14 | **Dead arm**. Phase disagrees with `security()` (phase 16). Most entries now ship. |

Placeholder rows in the 5 live placeholder menus: 5+5+4+5+5 = **24**, plus 5 trailers.

### Rows built disabled, or conditionally disabled

| menus.rs line | Menu | Row | Condition | Reason given in code |
|---|---|---|---|---|
| 425 | File | Save All | `any_dirty` | nothing to save |
| 431 | File | Reload from Disk | `has_path` | needs a file to re-read |
| 530-544 | Edit | Duplicate Line, Move Line Up, Move Line Down, Go to Line… | `editor_view` | `TextInput`'s caret is unreadable (ADR-0018). **Always off in the default build.** Help ▸ Keyboard Shortcuts still advertises Ctrl+D and Alt+Up/Down (dispatch.rs:25-31). |
| 583/589/597 | View | Zoom In / Zoom Out / Reset Zoom | font-size bounds / not default | at the bound |
| 698 | Security | Encrypt Document… | `!encrypted` | already `.bpadx` |
| 725 | Security | Redact Found Secrets… | `has_content` | label says "this document is empty" |
| **752** | Security | **Sign Document — no signing key yet** | **hard `false`** | no key store; blocked on the credential-store decision (bp-platform lib.rs:50-62, `Capability::CredentialStore` = AwaitingDecision) |
| 769 | Security | Verify Signature… | `has_path` | sidecar is named after a file |
| 821 | Tab ctx | Close Other Tabs | `others > 0` | — |
| 832 | Tab ctx | Copy Full Path | `has_path` | no path for Untitled |
| 851 | Insert | every Date/Time stamp | `editor_view` | needs the caret. **Always off in the default build.** |
| 943-962 | Note | Suggest Title, Semantic Rename…, Summary, Keywords, Outline | `has_content` | nothing to analyse |

---

## 3. Markers (tracked `*.rs` and `*.slint`, target/ excluded; 132 files)

| Kind | Raw hits | Real (non-fixture) | Locations |
|---|---|---|---|
| TODO | 2 | **0** | bp-redaction/src/metadata.rs:1165 (test input string), bp-semantic/src/lib.rs:336 (test assertion) |
| FIXME | 0 | 0 | — |
| XXX | 2 | 0 | bp-files/src/save.rs:24 (`.tmpXXXXXX` template), bp-naming/src/date.rs:88 (test) |
| HACK | 0 | 0 | — |
| `todo!` / `unimplemented!` | 0 | 0 | Lints `todo`/`unimplemented`/`unwrap_used`/`panic` = warn workspace-wide (Cargo.toml). Allowed only on Slint-generated code (bp-ui/src/lib.rs:53-58). |
| `unreachable!` | 1 | 0 | bp-data/src/lib.rs:692 (comment only) |
| `#[ignore]` | 2 | **2** | **tests/integration/tests/search_over_buffer.rs:299**: "known defect: bp-search counts only `\n` as a line break, bp-buffer's rope counts every Unicode line break". Consequence: the status bar reports different line counts in the two editor views (comment at 290-297). fuzz/tests/envelope.rs:283: resource-heavy, run deliberately. |
| `#[allow(dead_code)]` | 1 | 1 | tests/integration/tests/common/mod.rs:18 (shared test helpers) |
| other `#[allow]` | 4 | — | bp-integrity/src/atomic.rs:113 (`unnecessary_wraps` on the Windows no-op `set_mode`), bp-buffer/src/large.rs:1106 (test), bp-buffer/benches/large_file.rs:6, bp-naming/tests/properties.rs:5 |
| `expect(` in library (non-test) code | 6 | 6 | bp-data/src/lib.rs:314, :830; bp-buffer/src/large.rs:701, stream.rs:304; bp-search/src/lib.rs:311; bp-notebook/src/ipynb.rs:364. The root Cargo.toml comment says "nine". The count has drifted, all six carry messages, and 3 of them sit in unwired code (bp-buffer ×2, bp-notebook). There are 515 `expect(` in all, almost all in tests. |
| "not yet" / "not implemented" (unfinished work, not incidental prose) | — | 7 | bp-ui/src/menus.rs:294 (`arrives()` trailer text), menus.rs:796-799 (**stale**: "while bp-crypto does not exist"), bp-security/src/lib.rs:155-157 (**stale**: "`bp-crypto` arrives in phase 15"), bp-security/src/lib.rs:235 (**stale**: "until bp-crypto exists"), bp-security/src/lib.rs:443 (test message), bp-platform/src/lib.rs:50-62 (credential store / DPAPI / Secret Service not implemented), bp-storage/src/lib.rs:19 ("Nothing in the application writes to this yet") |
| "for now" | 0 | 0 | — |
| "stub" | 1 | 0 | bp-research/src/lib.rs:16 ("absent, not stubbed": network features deliberately omitted under ADR-0006) |
| "placeholder" | ~50 | 0 | All domain vocabulary (`bp_redaction::Replacement::Placeholder`/`PLACEHOLDER`, `bp-secrets` placeholder-value filter). None marks unfinished work. |

By crate, real unfinished-work markers: bp-ui 2 (menus.rs:294, 796-799). bp-security 3 (all stale phase-15 text). bp-platform 1. bp-storage 1. integration 1 `#[ignore]` (bp-search × bp-buffer defect). fuzz 1 `#[ignore]` (resource). The code carries almost no TODO debt. Its "unfinished" signal lives in `planned()` rows, `NotImplemented`/`AwaitingDecision` capabilities, and module docs.

---

## 4. Platform-specific code

### Compile-time `cfg` sites (production)

| Location | Windows | Linux | Equivalent on other OS? |
|---|---|---|---|
| apps/bachelorpad/src/main.rs:5 `windows_subsystem = "windows"` (release) | no console | n/a | N/A |
| bp-platform/src/lib.rs:130/135 `Platform::HOST` | Windows | **everything non-Windows collapses to Linux** (macOS is treated as Linux) | by design (ADR-0001) |
| bp-core/src/encoding.rs:65 `LineEnding::platform_default` | CRLF | LF | yes |
| bp-files/src/save.rs:387/394 `preserve_permissions` | no-op (ACL inherited from directory; per-file ACL/readonly lost on atomic replace) | copies mode | partial |
| bp-files/src/save.rs:403/410 `sync_directory` | no-op | fsync dir | documented as not needed |
| bp-files/src/save.rs:312 `os_path` (`Platform::HOST != Windows`) | adds `\\?\` for >259 chars | passthrough | N/A on Linux |
| bp-files/src/save.rs:160 `refuse_device_name(Platform::HOST, …)` | refuses CON/NUL/… | Linux rules | yes |
| bp-integrity/src/atomic.rs:93/112 `set_mode` | **no-op**: key files not narrowed | 0600/0644 | **No.** Windows signing keys are readable by whatever the directory ACL allows (reported as `KeyFileProtection::InheritedFromDirectory`). |
| bp-integrity/src/atomic.rs:122/131 `sync_directory` | no-op | fsync | documented |
| bp-integrity/src/keys.rs:228-257 `freshly_written_protection`/`protection_of` | always `InheritedFromDirectory` | reads mode bits | honest, but no protection |
| bp-storage/src/lib.rs:559 `#[cfg(unix)]` | — | test only | unreached crate |
| bp-ui/src/state.rs:2632 `documents_dir` (`cfg!(windows)`) | `%USERPROFILE%\Documents` | `$XDG_DOCUMENTS_DIR` env, then `~/Documents`, then `~` | **Both are approximations.** Windows ignores Known-Folder/OneDrive redirection. Linux reads `XDG_DOCUMENTS_DIR` from the environment, which `user-dirs.dirs` normally does not export, so localized folders (e.g. `~/Dokumente`) are missed. |
| bp-platform/src/dirs.rs:204-280 | `%APPDATA%` / `%LOCALAPPDATA%` | XDG_* / `~/.config` etc. | yes |

### Capability matrix (bp-platform/src/capabilities.rs:193-284): the crate's own statement

| Capability | Windows | Linux |
|---|---|---|
| DesktopEntry / MimeAssociation | Unsupported | Available |
| WindowsFileTypeRegistration | **NotImplemented** (generates `.reg`; nothing applies it) | Unsupported |
| DefaultAppHandoff | Available (text points at `ms-settings:defaultapps`; nothing is launched) | Unsupported |
| AssociationQuery | **NotImplemented** (no registry reader; the report says "cannot tell") | Available (`mimeapps.list`) |
| AssociationInstall | **NotImplemented** | Available (writes `.desktop` + MIME XML; does **not** run `update-desktop-database`/`update-mime-database`, `FollowUp` only) |
| CredentialStore | **AwaitingDecision** (DPAPI) | **AwaitingDecision** (Secret Service) |
| BiometricUnlock | AwaitingDecision (Hello) | Unsupported |
| FilePermissionNarrowing | **Unsupported** (no `unsafe`, no Win32) | Available |
| ExtendedLengthPaths | Conditional | Available |
| CaseSensitivePaths | Unsupported | Available |
| NativeClipboardHistory | NotImplemented | Unsupported |

### Runtime one-platform paths in the shell

| Area | Windows | Linux | Gap |
|---|---|---|---|
| File ▸ Set as Default Editor (bp-ui/src/default_editor.rs, dispatch.rs:511) | Saves a `.reg` through a Save dialog for the user to apply; association state is "unknown" | Writes into `$XDG_DATA_HOME` with consent; reads `mimeapps.list` | Asymmetric by design. Windows does less work and cannot query. |
| **rfd dialogs** (`xdg-portal` feature, bp-ui/Cargo.toml:61) | Native Win32 file/message dialogs | File dialogs need a running **xdg-desktop-portal** (falls back to `zenity`). **Every `MessageDialog` on Linux shells out to `zenity`** (rfd-0.17.2 `xdg_desktop_portal.rs:299-340`). If zenity is missing, `show()` returns `Cancel` without showing anything. | **High.** (a) Startup recovery prompt, lib.rs:383-400: anything but `Yes` runs `journal.discard_all()`. Without zenity, recovered unsaved work is **silently deleted** at launch. (b) Every `show_info` (Shortcuts, About, hash, secret-scan report, metadata, Security History, Note results) shows nothing. (c) Close-window with dirty docs: `Cancel` means KeepWindowShown, so the window cannot be closed without saving. (d) All consent prompts (Replace All, Redact, Set Default Editor, overwrite-on-disk) read as refused. |
| **Clipboard** (`arboard` 3.6.1, default features; lib.rs:281-288) | Clipboard persists after the handle drops | A new `Clipboard` is created and dropped **per call**. On X11 the selection is handed to a clipboard manager with a 100 ms timeout, or lost. `wayland-data-control` is **not enabled**, so pure Wayland needs XWayland. | **Medium-high.** In `--editor-view`, Cut (editor_view.rs:237-246) deletes the text after `set_os_clipboard`. Without a clipboard manager the cut text can be lost. Clipboard-history paste and Copy Full Path are affected the same way. The default `TextInput` path uses Slint's own clipboard and is unaffected. |
| **Font `"Consolas"`**: app.slint:612, :628; editor_surface.slint:64, :155, :168; results_panel.slint:71 | Present | **Not installed on Linux.** No bundled font, no fallback list, no `register_font` | **Medium-high.** Linux falls back to the default, likely proportional, face. The `--editor-view` surface measures "M" and assumes a fixed advance (editor_surface.slint:61-69, 82, 99, 142, 179), so the caret, selection and click-to-column mapping drift on any non-monospace fallback. The default view also renders in a proportional font. |
| Follow System theme (app.slint:68-69) | Works via winit | `SystemPalette.color-scheme` is often `unknown` on X11. Handled (`system-known`), but the menu toggle then has no effect. | minor |
| Emoji/glyphs in menu labels (📌 ✓ ↳ … ⚠) | Segoe UI Emoji | Depends on installed emoji font | cosmetic |
| Text encodings (bp-core/src/encoding.rs:9-17) | UTF-8/BOM/UTF-16 only. **Legacy ANSI (cp1252) Notepad files are refused** as "not valid UTF-8" (bp-files/src/load.rs:109) | same | Affects Windows users more ("Notepad replacement" preset) |
| Scripts: all `scripts/*.ps1` (local CI, hooks installer, startup measurement) | Native | Need `pwsh` | Linux developers have no gate without PowerShell |

---

## 5. Test posture

Counts are `#[test]` attributes (proptest cases included) and `proptest!` blocks across each crate's `src/`, `tests/` and `benches/`.

| Crate | LOC | #[test] | proptest! blocks | tests/kLOC | Note |
|---|---|---|---|---|---|
| bp-audit | 2630 | 44 | 7 | 16.7 | thin for size; heavy proptest |
| bp-buffer | 3186 | 62 | 0 | 19.5 | large-file/stream half unwired |
| bp-clipboard | 946 | 42 | 0 | 44.4 | |
| bp-config | 2130 | 73 | 0 | 34.3 | |
| bp-core | 822 | 23 | 0 | 28.0 | |
| bp-crypto | 2027 | 56 | 1 | 27.6 | + fuzz |
| bp-data | 2002 | 51 | 1 | 25.5 | + fuzz |
| bp-editor | 4861 | 202 | 0 | 41.6 | no proptest over edit/undo |
| bp-files | 2278 | 75 | 2 | 32.9 | + fuzz |
| bp-formats | 743 | 23 | 0 | 31.0 | + fuzz |
| bp-history | 802 | 23 | 0 | 28.7 | recovery journal; no test covers the UI's discard-on-non-Yes path |
| bp-integrity | 3477 | 99 | 3 | 28.5 | |
| bp-naming | 1141 | 54 | 1 | 47.3 | |
| bp-notebook | 3583 | 57 | 3 | **15.9** | thin, and unwired |
| bp-platform | 5626 | 108 | 3 | 19.2 | thin for the largest non-UI crate; Windows/Linux both exercised via the `Platform` value |
| bp-redaction | 2915 | 66 | 1 | 22.6 | |
| bp-research | 5208 | 161 | 5 | 30.9 | orphan |
| bp-search | 984 | 34 | 0 | 34.6 | known line-break defect `#[ignore]`d |
| bp-secrets | 1988 | 37 | 1 | **18.6** | pattern-heavy scanner |
| bp-security | 622 | 14 | 0 | 22.5 | |
| bp-semantic | 608 | 29 | 0 | 47.7 | |
| bp-storage | 721 | 21 | 0 | 29.1 | unreached |
| bp-theme | 302 | 11 | 0 | 36.4 | |
| bp-ui | 11654 | 244 | 0 | 20.9 | includes 9 headless-window tests (`window_tests.rs`, `MinimalSoftwareWindow`). Dialogs (rfd), clipboard (arboard) and fonts are untested. |
| apps/bachelorpad | 93 | 0 | 0 | 0 | covered only by the `--self-check` launch in local CI |
| tests/integration | 4803 | 124 | 11 | 25.8 | 1 ignored; no bp-research, bp-config or bp-theme coverage |
| fuzz (separate workspace) | 2574 | 43 | 21 | 16.7 | 1 ignored; targets bp-core/crypto/data/files/formats/notebook |

Thin coverage relative to size: bp-notebook, bp-audit and bp-platform (≈16-19/kLOC); bp-secrets and bp-buffer (≈19); bp-ui (≈21, where the untested OS integration is the risk rather than the logic).

### CI posture

- `.github/workflows/ci.yml` is **dormant**. The trigger is `workflow_dispatch` only, and Actions is disabled at the repository level (ADR-0016, docs/decisions/ADR-0016.md). When run, it would use a matrix `windows-latest` + `ubuntu-latest` with fmt → `check --locked` → clippy `-D warnings` → `cargo test --workspace`, then fmt/clippy/test in `fuzz/`. It has **no** `--self-check` launch step and no `--ignored` run.
- The authoritative gate is `scripts/Invoke-LocalCI.ps1` (371 lines, PowerShell), run from pre-commit/pre-push hooks and bypassable with `BPAD_SKIP_CI=1`. On the Windows host it runs fmt, `check --locked` (skipped with `-Quick`), clippy, test, a build plus `--self-check` load of the binary (lines 130-176), and fuzz (skipped with `-Quick`).
- **The Linux leg is opt-in** (`-Linux`, through WSL, lines 276-337) and runs only fmt/clippy/test (no `--locked`, no `--self-check` launch). A missing WSL toolchain reports as "skipped". WSL has no real desktop, so nothing exercises xdg-portal, zenity, the X11/Wayland clipboard or font fallback.
- So neither OS has a hosted gate. Linux verification depends on someone passing `-Linux`, and no leg on either OS tests real dialogs, clipboard or fonts.
