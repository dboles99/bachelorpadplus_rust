# Audit 03 — the project's own record of open work, checked against the code

Repo: `/home/user/bachelorpadplus_rust`, branch `claude/compassionate-wright-wwclxq` (HEAD `c9c89d7`, 2026-09-25).
Branch is **8 commits ahead of `main`**, 0 behind, and fully pushed to `origin`. Read-only audit; nothing in the repo was modified.

Sources read: ROADMAP.md, README.md (Current state, Known gaps), DECISIONS.md, project/DECISIONS_NEEDED.md,
project/WORK_QUEUE.md, project/NEXT_SESSION.md, notes/*.md, prompts/, artifacts/, metadata/, database/,
all 72 files under project/tasks/, docs/decisions/ADR-0001..0029. Claims were checked with grep and git.

## Summary counts

| Class | Open items |
| --- | ---: |
| Decision needed | 15 (10 registered D1–D10, plus 5 unregistered: U1–U5) |
| Feature gap | 25 (F1–F25) |
| Defect | 8 (X1–X8; X2, X7 and X8 are also decisions D6, D10, D7) |
| Tooling / process | 13 (T1–T13) |
| Documentation | 34 (the stale records in section 4, plus the 49 template task files counted as one item) |
| **Stale records** | **34** (S1–S34) |

Open task-file checkboxes: **7** unchecked boxes in 5 files. After checking the code, **4 are done** (stale),
**2 are partly done**, and **1 is really open**. **62 of 72 task files have no checkboxes at all.**
49 of those are the same scaffold template with only the name changed. The other 13 are phase READMEs.
All 29 ADRs are **Accepted**. None is still Proposed.

---

## 1) Decisions awaiting the owner (Daniel)

### Registered in `project/DECISIONS_NEEDED.md`. All 10 checked in code and all still open.

| ID | Question | Options | What it blocks | Recommended default for the RC |
| --- | --- | --- | --- | --- |
| D1 | What may leave the machine (document content or embeddings to providers)? | (a) Nothing in v1: local-only; (b) opt-in allow-list of named providers, gated per security profile; (c) local embeddings only, generative providers later | Phase 8 layer 3, phase 10 hybrid search, Research ▸ DOI Lookup; `Network::Allowed` means only "not forbidden" | (a) Nothing leaves the machine in v1. Record it in an ADR so `Network::Allowed` has a defined meaning. Neither phase is in RC scope |
| D2 | Keep the `--editor-view` two-view fork for good? | (a) Keep it opt-in (current answer); (b) retire the custom view; (c) make it the default for non-IME locales | 5 features that work only in that view; a small "tax" on every wiring item | (a) Keep it opt-in and label it experimental in the RC. It has a Linux caret defect (X1) and no IME (F6). Revisit on a Slint upgrade |
| D3 | May `bp-storage` be wired into the app (ADR-0019 product judgement)? | (a) Keep it unwired for v1; (b) wire `Metadata::Summary` under Standard; (c) path-only records | Phase 9, `bp-organize`, Tags/Related Notes/Revision History rows | (a) Leave it unwired for the RC. Verified: no crate in the app depends on `bp-storage`, only `tests/integration` |
| D4 | Should the `.bpadx` envelope get an outside crypto review before real use? | (a) Commission a review before 1.0; (b) ship with an "unreviewed format" warning; (c) hide Encrypt until reviewed | Trusting `.bpadx` with real documents | (b) for the RC and (a) before GA. The owner must decide on budget and on who reviews |
| D5 | Where does a signing key live? | (a) Key file: `0600` on Unix, no DACL narrowing on Windows (ADR-0026); (b) platform keyring (DPAPI / Secret Service, which needs deps or `unsafe`); (c) seed sealed in a passphrase `.bpadx` envelope in the Data dir | Security ▸ Sign Document. Verified greyed at `menus.rs:753` "Sign Document — no signing key yet". `bp_integrity::sign_file` exists and is tested | (c) reuses `bp-crypto` and avoids the Windows DACL problem. Otherwise keep the row greyed for the RC, which is acceptable |
| D6 | What is a line? `bp-search` counts `\n`; ropey's `unicode_lines` also breaks on bare `\r`, VT, FF, NEL, LS, PS | (a) `bp-search` adopts ropey's break set; (b) `bp-buffer` drops `unicode_lines`; (c) middle ground: LF, CRLF and bare CR everywhere (ropey `cr_lines`) | Find-in-files jumps to the wrong line; the two views report different line counts. Pinned by the `#[ignore]`d test at `tests/integration/tests/search_over_buffer.rs:299` | (c) matches Notepad and most editors. Ask for a one-line answer; the patch is small either way |
| D7 | Should `bp-crypto` expose a derive-once API? | (a) Opaque, zeroizing `SealingKey` from `derive`, plus `seal_with_key` (amends ADR-0021); (b) cheaper KDF in `bp-audit` (forbidden by ADR-0020); (c) keep it as is | Security History usability: about 45 ms per sealed line, so seconds for a long-lived document. Verified `fn derive` is private (`bp-crypto/src/lib.rs:196`) | (a) Non-exportable key type, amendment to ADR-0021 |
| D8 | How is Windows default-editor registration applied? | (a) Hand the user a `.reg` file plus `ms-settings:defaultapps` (current); (b) `winreg` dependency; (c) `unsafe` Win32 | The Windows half of specs §19 | (a) Adopt it formally. It matches ADR-0012, and Windows 10/11's UserChoice hash stops programmatic defaults anyway. Verified: no `winreg` in any Cargo.toml |
| D9 | Application id: `bachelorpad` or reverse-DNS? | (a) Keep `bachelorpad`; (b) `io.github.<owner>.BachelorPad` or `com.<domain>.BachelorPad` | Every association after the first release. Changing it later orphans them. Verified `app_id: crate::APP_DIR` (`bp-platform/src/editor.rs:104`) | (b), **decide before the RC**. It is cheap now and expensive after any install |
| D10 | A `.bpadx` header chooses its own Argon2id cost before authentication (about 75 s and 1 GiB per attempt worst case) | (a) Keep the current ceiling; (b) lower the hard ceiling (e.g. 256 MiB, t≤8); (c) keep the ceiling but prompt when the header asks for more than N× the default | Opening hostile or corrupt `.bpadx` files | (c) Prompt above 4× the default, with a lower hard cap. Verified `header.rs:74 validate` |

### Decisions the record names but never put in the register

| ID | Question | Where it lives | Options | Blocks | Recommended default |
| --- | --- | --- | --- | --- | --- |
| U1 | Memory-map large files? ADR-0027 declines and says "Left as a stated decision for a human, not taken here" | ADR-0027 §Alternatives | (a) Accept the decline (chunked reads); (b) `memmap2` with copy-on-open or a SIGBUS handler, which revisits ADR-0002 | README's "memory-mapped modes" promise | (a) Accept it, register it, and change the README tagline |
| U2 | How to get a monospace font on Linux? | WORK_QUEUE Tier 0.75 ("a product choice") | (a) Bundle an OFL monospace font via Slint `import`; (b) choose the family per platform in Rust | X1 (caret drift) and the proportional editor font on Linux | (a) Bundle one font. That is deterministic on both OSes and also fixes the default view |
| U3 | What does "read-only because huge" look like in the shell? | WORK_QUEUE Tier 0, large-file row ("a real UI decision") | Viewer tab with a banner; separate viewer window; refuse and offer an external tool | F1 (a 2 GB open loads 2 GB) | A viewer tab with a read-only banner (ADR-0027 already says viewer) |
| U4 | How to render Markdown preview (Slint has no HTML view)? | WORK_QUEUE Blocked; 05-formats README | Styled `Text` runs drawn in-app; external browser preview; defer | F8, View ▸ Split / Preview | Defer past the RC |
| U5 | Should clipboard menu rows be frozen while a menu is open? | DECISIONS.md open items ("a design question") | Snapshot the history on menu open; encode entry identity (hash) in the row id; pause the poll while the popup is open | X3 | Snapshot on open |

---

## 2) Open task-file checkboxes by phase

Every `- [ ]` in `project/tasks/**`, with its status verified against the code.

| Phase dir | File : line | Unchecked item | Verified status | Class |
| --- | --- | --- | --- | --- |
| 01-foundation | 01-workspace-scaffold.md:28 | The remaining placeholder crates either gain content or are removed | **Done (stale).** The workspace `members` holds only 24 real crates, and the Cargo.toml comment says empty crates are not created. The file's Status line and Notes ("24 of the 27 crates are still four-line placeholders") are also stale | documentation |
| 01-foundation | 02-ui-prototype.md:26 | Comparative Linux timing (needs a native machine) | **Open.** No native-Linux startup artefact; `artifacts/benchmarks/` holds Windows runs only | tooling/process |
| 01-foundation | 02-ui-prototype.md:75 | Real cursor position (Ln/Col) | **Partly done.** `state.rs:1895-1911` shows `Ln x, Col y` only under `--editor-view`; the default view still shows "N lines". The default view now reads `cursor-position_byte-offset` for overwrite (`app.slint:679-711`), so the stated reason ("the caret is not readable") is weaker than the record says | feature gap |
| 01-foundation | 02-ui-prototype.md:77 | Rope buffer (text lives in `HashMap<DocumentId, String>`) | **Done (stale).** `state.rs:73 editors: HashMap<DocumentId, bp_editor::Editor>`, and `bp_editor::Editor { buffer: Buffer }` is a ropey rope | documentation |
| 01-foundation | 03-ci-baseline.md:27 | Linux leg enabled (needs a Rust toolchain in WSL) | **Done (stale).** LOCAL_CI.md:117 says "Working as of 2026-08-17". Caveat: the leg runs only fmt, clippy and test; it skips `--locked`, launch and log hygiene (T2) | documentation (+T2) |
| 02-core-editor | 01-rope-buffer.md:28 | Used as the editor's text storage | **Done (stale).** Same evidence as above. The Status line "not yet the editor's storage" is stale too | documentation |
| 02-core-editor | 05-undo-redo.md:36 | Wired to Ctrl+Z / Ctrl+Y | **Partly done (mostly stale).** `bp-editor/src/keys.rs:144-146` maps Ctrl+Z, Ctrl+Y and Ctrl+Shift+Z, and `bp-editor` is the authority under `--editor-view`. The default `TextInput` view uses Slint's stack by design (ADR-0018) | documentation |

### Task files with no checkboxes (62)

**49 files are the unmodified scaffold template** ("Implement and validate `X` as part of phase `Y`" and seven generic steps). They have no status and no acceptance criteria. ADR-0014 says "A task file without acceptance criteria is incomplete". Real status by file, taken from the phase READMEs and the code:

| Phase dir | Template-only task files, with real status in brackets |
| --- | --- |
| 02-core-editor | tabs [done], file-io [done], status-bar [done except Ln/Col in the default view] |
| 03-file-safety | atomic-save [done], file-watcher [done, polling], recovery-journal [done, plus encrypted journal per ADR-0022], revision-history [**not started**; Note ▸ Revision History is `planned`, phase 9] |
| 04-large-files | mmap-reader [**declined** in ADR-0027, U1], streaming-search [built in `bp-buffer/large.rs`, **unwired**], large-file-ui [**not started**, Tier 0, U3] |
| 05-formats | format-registry [done], markdown [**partial**: outline only, no preview/TOC], json-yaml-toml [done; YAML sort keys missing], jsonl-csv [done; RFC 4180 quoting is implemented at `bp-data/src/lib.rs:966`], source-formats [**detection only**, no highlighting] |
| 06-search | exact-regex [done], recursive-search [done], replace-preview [done, `dispatch.rs:158`], fuzzy-search [**not started**; no match anywhere in `crates/`] |
| 07-semantic | deterministic-analysis [done], semantic-naming [done], metadata [built, **unwired**, D3], related-notes [**not started**], hybrid-search [**not started**, D1] |
| 08-clipboard | windows-history [**not started**], local-history [done, in-memory], paste-transformations [done, W2], secure-clipboard [**not started**; no timed clear or secure mode in `bp-clipboard`] |
| 09-notebook | cell-model, ipynb, outputs, execution-security [built, **unwired**], runners [**not built**: no `std::process` in `bp-notebook`; `RunRequest` is inert and the ADR-0025 `bp-execution` crate does not exist] |
| 10-research | research-profile, citation-model, paper-metadata [built in `bp-research` (bibtex, csl, paper, profile), **unwired**: nothing in the app depends on it] |
| 11-security | profiles, bpadx, secret-scanning, privacy-mode [done], hash-sign-redact [hash, verify and redact done; **sign blocked on D5**], key-protection [**not started**, D5] |
| 12-themes-brand | themes [done, 4 plus Follow System], personality [**not started**: no Standard/Minimal personality mode, which ADR-0013 requires], accessibility [**minimal**: only `text_contrasts_with_its_surface` test] |
| 13-platform | windows-integration [**partial**: `.reg` artefact only, D8], linux-integration [done for `.desktop`/MIME], default-editor [done: Set as Default Editor, id 220] |
| 14-hardening | fuzzing [5 harnesses, gated; no harness for `bp-research`'s BibTeX/CSL parsers], benchmarks [`bp-buffer` bench and startup script; `benches/` has only a README], threat-tests [**no evidence**; `artifacts/security/` is empty], packaging [**not started**, phase 20] |

**13 README files** have no checkboxes. Seven of them (`04`, `09`, `10`, `11`, `12`, `13`, `14`) are bare name lists with no status. The status tables in 01, 02, 03, 05, 06, 07 and 08 are partly stale (section 4).

---

## 3) Open items from WORK_QUEUE, NEXT_SESSION and DECISIONS, with verified status

### Feature gaps

| # | Item (source) | Verified status |
| --- | --- | --- |
| F1 | The open path uses the large-file engine (WQ Tier 0) | **Open.** `LargeFile`/`SizeClass` are named nowhere outside `crates/bp-buffer`. A 2 GB open loads 2 GB. Needs U3 |
| F2 | Notebook mode (WQ Tier 0) | **Open.** `bp-notebook` is depended on only by `fuzz/` and `tests/integration`. The Notebook and Run menus are `planned_menu` (`lib.rs:243,246`). No runner exists |
| F3 | Research mode (WQ Tier 0) | **Open.** `bp-research` has zero reverse dependencies. Research is a `planned_menu` |
| F4 | Anything writing to `bp-storage` / `bp-organize` (WQ Blocked) | **Open, gated on D3.** Organize is a `planned_menu`; Tags, Related Notes and Revision History are `planned` (`menus.rs:969-972`) |
| F5 | Security ▸ Sign Document (WQ Blocked) | **Open, gated on D5.** Greyed with a reason (`menus.rs:753`) |
| F6 | IME composition under `--editor-view` (WQ Blocked) | **Open, blocked on Slint 1.17.1** (upstream) |
| F7 | W11 drag-and-drop to open (WQ Blocked) | **Open, blocked on Slint 1.17.1** |
| F8 | Markdown preview, TOC, Split/Preview (WQ Blocked) | **Open.** View ▸ Split / Preview is `planned` (`menus.rs:604`). Needs U4 |
| F9 | Print (WQ Blocked) | **Open.** No cross-platform story; no code |
| F10 | Syntax highlighting (05 README) | **Open.** Detection only |
| F11 | Fuzzy search, search within a selection, replace in files (06 README) | **Open.** No code for any of the three |
| F12 | Multi-cursor, column selection, folding, indentation guides (02 README; `menus.rs:547` "Multi-cursor — phase 2") | **Open** |
| F13 | Five features only under `--editor-view` (duplicate, move up/down, date/time, go to line) plus Ln/Col (README Known gaps) | **Open.** `menus.rs:530-544,851` gate on `editor_view`. The default view now reads the caret offset internally (see section 2), so this could be revisited |
| F14 | Clipboard: opt-in persistence, secure clipboard / timed clear, pin UI, Windows native history (08 README; ADR-0010) | **Open.** `toggle_pin` is called only in tests; `bp-clipboard` has no persistence or timed clear |
| F15 | Revision history, tags, related notes, duplicate detection (phase 9) | **Open**, gated on D3 |
| F16 | Semantic layers 2/3, hybrid search (phase 10) | **Open**, gated on D1 |
| F17 | YAML sort keys, outline/folding (ADR-0023) | **Open.** No `yaml_sort` in `bp-data` |
| F18 | Personality modes (Standard/Minimal, ADR-0013); accessibility (phase 17) | **Open.** No personality code anywhere; accessibility is one contrast test |
| F19 | Lock Document (`menus.rs:785`, phase 16) | **Open** |
| F20 | Association presets: ADR-0012 wants several, the UI hard-codes `NotepadReplacement` (`default_editor.rs:51`) | **Open** (library supports them; the UI offers one) |
| F21 | "Something is waiting" recovery hint: ADR-0022 says "`Journal::sealed_count` … Nothing uses it yet" | **Still true.** Only test asserts call it (`state.rs:3813+`) |
| F22 | Packaging, signing, installer, upgrade testing (phase 20 "Not started") | **Open, and the largest RC gap.** Version is `0.1.0`, there are no git tags, `artifacts/releases/` holds only `.gitkeep`, and there is no installer script |
| F23 | Tools menu and Help ▸ Diagnostics (phase 19) | **Open**, `planned` |
| F24 | A plaintext document under Private/Confidential gets no crash recovery (README Known gaps) | **Open, a documented limitation by design** (ADR-0022). The status bar says what to do |
| F25 | Insert ▸ Markdown constructs, citation, code block, table (`menus.rs:863`, phase 6) | **Open** |

### Defects

| # | Item (source) | Verified status |
| --- | --- | --- |
| X1 | `--editor-view` caret drawn right of the text on Linux (WQ Tier 0.75) | **Open, and broader than recorded.** `font-family: "Consolas"` is hard-coded in `editor_surface.slint:64,155,168`, and also in the **default view's** `TextInput` and gutter (`app.slint:612,628`) and in `results_panel.slint:71`. So on Linux the default editor also renders in a proportional fallback font: no caret drift, but a "Notepad" with a non-monospace font and misaligned columns. Needs U2 |
| X2 | `bp-search` and `bp-buffer` disagree about what a line is (WQ Tier 0.5) | **Open, needs D6.** `bp-buffer` uses `ropey = "1"` with default features (`unicode_lines`). The test is `#[ignore]`d at `search_over_buffer.rs:299` |
| X3 | Clipboard menu row decoded against the history at click time (NS Housekeeping; DECISIONS) | **Open.** No snapshot logic in `bp-ui`. Needs U5 |
| X4 | **New.** Security menu tells encrypted-document users their recovery is "off until encryption ships (phase 15)" | **Open.** `menus.rs:799` `Recovery::Encrypted => "off until encryption ships (phase 15)"`, with a comment claiming `bp-crypto` does not exist. Phase 15 shipped and ADR-0022 seals the journal, so this user-visible text is wrong under Private, Confidential and Maximum |
| X5 | **New, minor.** Data menu for a non-data document lists Validate, Format/Minify, Sort Keys, Convert and others as "— not implemented yet (phase 6)" (`menus.rs:925-931` reuses `planned_menu("Data")`) | **Open.** Most of these are implemented for JSON, YAML and TOML. The text misstates the product |
| X6 | Idle memory drifted from about 20 MB to about 32 MB after dialogs (NS Housekeeping) | **Open, never investigated** |
| X7 | A `.bpadx` header chooses its own KDF cost (D10) | **Open, needs D10** |
| X8 | Sealed audit history costs about 45 ms per line to read (D7) | **Open, needs D7** |

### Tooling and process

| # | Item (source) | Verified status |
| --- | --- | --- |
| T1 | The Windows leg has not run on any sixth-session change (NS: "Run `Invoke-LocalCI.ps1` on Windows first") | **Open.** 8 commits on this branch (`e88c2e6..c9c89d7`), including the Windows-only `\\?\` rename fix, have never been gated on Windows |
| T2 | No gate on native Linux | **Open (new finding).** `-Linux` shells out to `wsl`, so it runs only from Windows. The `launch` stage hard-codes `target/debug/bachelorpad.exe` (script line ~141). The WSL leg runs only fmt, clippy and test: no `--locked` check, no launch, no log hygiene. WORK_QUEUE tells agents to run `./scripts/Invoke-LocalCI.ps1 -Linux`, which cannot work in a Linux container (the sixth session ran the stages by hand) |
| T3 | The dormant `.github/workflows/ci.yml` must stay in step with the gate (ADR-0016) | **Out of step.** It has no launch or log-hygiene steps and no `apt-get` for `libfontconfig1-dev`/`libxkbcommon-dev` on `ubuntu-latest`, which LOCAL_CI.md:126 says Slint needs |
| T4 | Time to first interaction unmeasurable under the software renderer; specs §22 warm-start 75 ms unverified (ADR-0017; DECISIONS) | **Open** |
| T5 | Scroll smoothness under CPU rasterisation needs a capture rig (ADR-0017 revert condition) | **Open** |
| T6 | Native Linux startup timing (NS; 01/02 checkbox) | **Open** |
| T7 | Manual pass. Security rows and Set as Default Editor never clicked by a person. `--editor-view` checklist rows never run: arrow round trip, drag select, wheel, Ctrl+D/Alt+Up/Down, tab-indented file, resize; on Windows, advance measurement and wheel direction | **Open.** This is item 1 of NEXT_SESSION |
| T8 | `expect_used`, `unreachable` and `indexing_slicing` lints not on (NS Housekeeping) | **Open.** `[workspace.lints.clippy]` has only unwrap_used, panic, todo and unimplemented |
| T9 | `fuzz/corpus/yaml/billion-laughs.yaml` crash-loops the Cloud Code YAML server (NS Housekeeping) | **Open** (corpus present). A workspace-tooling note, not a product issue |
| T10 | ADR-0008 says "parsers operate on untrusted input and are therefore fuzz targets" | **Open (new finding).** `bp-research`'s BibTeX and CSL parsers have no harness (`fuzz/tests` = data, envelope, files, notebook, yaml) |
| T11 | ADR-0002: crates that do not need `unsafe` declare `#![forbid(unsafe_code)]` | **Open (new finding).** Missing in `crates/bp-security/src/lib.rs` and `apps/bachelorpad/src/main.rs`. Neither uses `unsafe`, so the fix is trivial |
| T12 | This branch is 8 commits ahead of `main` | **Open.** Merge after T1 |
| T13 | ADR-0023: two `saphyr` 0.0.12 emitter defects are "worth reporting upstream" | **Unverifiable from the repo.** Nothing records an upstream issue |

### Items the record lists as open that are actually closed or superseded

Covered in section 4. The main ones: all four "rope / placeholder / Linux leg / undo" checkboxes; "main has none of the commits" (PR #1 merged); YAML "no way in"; replace preview and CSV quoting "not started"; journal plaintext and clipboard "blocked on phase 14".

---

## 4) Stale records found (34)

| # | Where | Claim | Reality |
| --- | --- | --- | --- |
| S1 | tasks/01-foundation/01-workspace-scaffold.md (box, Status line, Notes) | Placeholder crates remain; "24 of the 27 crates are still four-line placeholders" | Only real crates are members; the smallest is `bp-theme` at 302 lines |
| S2 | tasks/01-foundation/02-ui-prototype.md (Status, box :77) | "keyboard shortcuts and buffer outstanding"; text in `HashMap<DocumentId,String>` | Shortcuts confirmed 2026-08-17; the rope is the storage |
| S3 | tasks/01-foundation/02-ui-prototype.md:75 and 01-foundation/README "Real cursor position needs bp-editor" | No Ln/Col | Ln/Col shipped under `--editor-view` |
| S4 | tasks/01-foundation/03-ci-baseline.md (Status, box :27) | Linux leg built but not enabled | Working since 2026-08-17 (LOCAL_CI.md:117) |
| S5 | tasks/02-core-editor/01-rope-buffer.md (Status, box, Notes) | Rope not the editor's storage | `bp_editor::Editor { buffer: Buffer }` holds every document |
| S6 | tasks/02-core-editor/05-undo-redo.md (Status, box) | Not wired | Wired under `--editor-view` (keys.rs:144-146) |
| S7 | tasks/02-core-editor/README.md | "Crates built and tested; wiring blocked"; line ops, go-to-line and zoom outstanding | Custom view exists (ADR-0018); W1, W5 and W6 are done |
| S8 | tasks/03-file-safety/README.md | Journal plaintext "Blocked on phase 14" | Phase 14 governs it and ADR-0022 seals it |
| S9 | tasks/05-formats/README.md | "YAML not started"; "CSV … does not understand quoting" | YAML shipped (ADR-0023, Data menu); RFC 4180 quoting implemented (`bp-data/src/lib.rs:966`) |
| S10 | tasks/06-search/README.md | Replace preview not started | Implemented (`dispatch.rs:158` "Show what Replace All would do, and ask") |
| S11 | tasks/07-semantic/README.md | metadata "Not started — needs bp-storage" | `bp-storage` exists (built, unwired, D3) |
| S12 | tasks/08-clipboard/README.md | Paste transformations "nothing acts on it"; persistence and secure clipboard "blocked on phase 14"; Windows history "needs bp-platform-windows" | W2 shipped; phase 14 is done, so these are unbuilt rather than blocked; ARCHITECTURE.md says `bp-platform-windows` will not be created |
| S13 | tasks/04, 09, 10, 11, 12, 13, 14 READMEs | Bare task-name lists | No status recorded, including for phases that are largely done (11-security) |
| S14 | 49 template task files | Template text | No acceptance criteria, which ADR-0014 calls incomplete. Status lives only in READMEs or nowhere |
| S15 | ROADMAP.md header | "Status as of 2026-08-20" | Body reflects the 2026-09-25 sixth session |
| S16 | ROADMAP.md last section | Encrypted-journal profiles "cannot be honoured until bp-crypto (phase 15) … menu says 'off until encryption ships'" | Phase 15 shipped. The same stale text is live in the UI (X4) |
| S17 | DECISIONS.md "Decisions needed" | `bp-storage` titles "before phase 14 exists" | Phase 14 exists; D3 is now purely a product question |
| S18 | DECISIONS.md "Decisions needed" | Lists 4 decisions | DECISIONS_NEEDED has 10 (D2, D4 and D6–D10 missing); the warm-start item there is a measurement, not a decision |
| S19 | DECISIONS.md open items | "custom editor view has never been typed into … Go to Line will be" | Driven end-to-end on Xvfb in the sixth session; Go to Line shipped (W5) |
| S20 | DECISIONS.md open items | "542 tests cover the pieces" | About 1,729 (README); 1,735 `#[test]` attributes counted |
| S21 | WORK_QUEUE Blocked, YAML row | "Done and unreachable … No `bp-ui` code calls any of it" | `state.rs:593` calls `bp_data::yaml_validate`. The row is struck through but its text is wrong |
| S22 | WORK_QUEUE Tier 0 vs NEXT_SESSION | WQ: "three crates with no way in"; NS: "Tier 0 … four crates with no way in"; README: "three crates and one module" | Inconsistent. The code says 3 crates (`bp-research`, `bp-notebook`, `bp-storage`) plus the `large.rs` module |
| S23 | WORK_QUEUE "Not a queue item" | Nobody has confirmed whether the caret lands under the pointer | Answered for Linux: it does not (X1) |
| S24 | NEXT_SESSION Housekeeping | "All 59 commits live on `feat/phase-01-foundation-ui`. `main` has none … 44 unpushed" | `31f6858 Merge phases 1-19 into main (PR #1)`. This branch is 8 ahead of `main`, 0 unpushed |
| S25 | NEXT_SESSION "Decisions still waiting on a human" | Lists 4 | DECISIONS_NEEDED has 10 |
| S26 | NEXT_SESSION commit table | Five sixth-session commits | Two later fixes (`eeb875c`, `c9c89d7`, ADR-0029 migration races and errors) landed after "docs: the record matches the sixth session" and are not recorded |
| S27 | ADR-0018 Consequences | "Word wrap and IME composition do not work under `--editor-view`"; also lists double-click selection as unused | Word wrap works (`bp_editor::wrap`); W7 is done; the ADR is unamended |
| S28 | ADR-0027 | Memory mapping "left as a stated decision for a human" | Not in DECISIONS_NEEDED (U1). README still advertises "memory-mapped modes" |
| S29 | README "Install the repository scaffold" | Run `Install-BachelorPadPlusRepo.ps1` from a release ZIP | No such script or ZIP in the repo |
| S30 | metadata/repository_manifest.json | Manifest of 181 files with SHA-256s | Scaffold snapshot from 2026-08-16. Every hash of an edited file is wrong |
| S31 | database/README.md and database/migrations/0001_initial.sql | "plans to use SQLite"; governance schema (projects, tasks, artifacts, notes) | `bp-storage` uses its own `src/migrations.rs` (documents, tags). `database/migrations` is read by nothing |
| S32 | `#[ignore]` reason at search_over_buffer.rs:299 | "known defect" | Every record calls it a decision (D6) |
| S33 | README Known gaps table | `bp-notebook` 3,514 lines | 3,583 lines now (trivial drift) |
| S34 | Code comment `menus.rs:795-797` | "The honest answer while bp-crypto does not exist (ADR-0020)" | `bp-crypto` exists. The comment and the string are both stale (X4) |

The notes (`notes/Brainstorm Session_17AUG2026.md`) are a pasted chat transcript from day one: rename to `main` and create `feat/phase-01…`. This is historical, not a live claim. No action needed.
`prompts/` and the rosettas README are consistent with the code; no stale claims found.

---

## 5) ADR follow-ups owed

All 29 ADRs have Status **Accepted**. None is Proposed or Superseded. Follow-ups that are promised or implied and have **not** happened (checked in code):

| ADR | Promise | Status |
| --- | --- | --- |
| ADR-0002 | Crates that do not need `unsafe` declare `#![forbid(unsafe_code)]` | **Owed.** Missing in `bp-security` and `apps/bachelorpad` (T11) |
| ADR-0008 | Parsers over untrusted input are fuzz targets | **Owed.** No harness for `bp-research` BibTeX/CSL (T10) |
| ADR-0010 | Persistence governed by profile, **timed clearing and a secure-clipboard mode** | **Owed.** Neither exists in `bp-clipboard` (F14) |
| ADR-0012 | Association presets (Notepad Replacement, Text + Notes, Developer…) offered as UI convenience; installer must be removable cleanly | **Partly owed.** Presets exist in `bp-platform`; the UI hard-codes one (F20). There is no installer (F22) |
| ADR-0013 | Standard and Minimal personality modes **must exist** | **Owed.** No code (F18) |
| ADR-0014 | Task files carry acceptance criteria; generated artefacts record metadata | **Owed.** 49 template task files (S14). Metadata manifest stale (S30); `scripts/New-ArtifactMetadata.ps1` exists but no per-artefact metadata beside `artifacts/benchmarks/*.json` |
| ADR-0016 | Local gate and dormant workflow kept in step; Linux leg covers cross-platform | **Owed.** Workflow lacks launch, log hygiene and Linux apt deps (T3). The Linux leg lacks `--locked`, launch and log hygiene, and there is no native-Linux gate path (T2) |
| ADR-0017 | TTFI measurement; scroll-smoothness revert condition; native Linux timing | **Owed** (T4, T5, T6) |
| ADR-0018 | "The two views must be kept in step"; a person must verify keys, caret and wheel | **Owed.** Manual checklist incomplete (T7). ADR text stale on word wrap (S27). Linux caret defect found (X1) |
| ADR-0019 | Schema grows with the phases that consume it; nothing writes yet | **Deliberately pending** (D3) |
| ADR-0021 | "Whoever wires [hash] must refuse or warn while dirty" | **Done.** `hash_report(.., unsaved)` at `state.rs:2589` plus a test. Recovery-journal encryption: **done** (ADR-0022) |
| ADR-0022 | `Journal::sealed_count` "so the shell can say something is waiting … Nothing uses it yet" | **Owed.** Still test-only (F21) |
| ADR-0023 | Report two `saphyr` emitter defects upstream; YAML sort keys and outline not done | **Owed** (T13, F17) |
| ADR-0025 | `bp-execution` inherits the run rule | **Owed.** The crate does not exist; runners not built (F2) |
| ADR-0026 | Key protection (DPAPI / Secret Service) | **Owed**, blocked on D5 |
| ADR-0027 | Memory mapping left to a human | **Owed as a decision** (U1). Engine unwired (F1) |
| ADR-0028 | Shell must tell the user what redaction cannot destroy | **Done** (`state.rs:2215-2235` consent text) |
| ADR-0029 | `legacy_path_in` stays until nobody upgrades from an old build | Ongoing by design. No action |

ADRs 0001, 0003–0007, 0009, 0011, 0015 and 0020 have no outstanding follow-up, or their follow-ups were verified done: PLATFORM_MATRIX.md exists; `created_at` has no setter per its test; profile policy is wired into the journal, clipboard and storage.
