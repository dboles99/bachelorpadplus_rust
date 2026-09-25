# BachelorPad+ release plan: the road to RC1 on Windows 10, Windows 11 and Linux

Written 2026-09-25 from five code-verified audits, kept beside it as evidence in [`artifacts/audit/2026-09-25/`](../artifacts/audit/2026-09-25/) (01 spec §1–12, 02 spec §13–25, 03 the project's records, 04 the code, 05 per-OS release engineering), against the repository at `c9c89d7`. Every claim was either taken from an audit table or re-checked in the code; where the evidence is thin it says so. The audits were run by five parallel agents and the plan was drafted by a sixth; the load-bearing claims (the zenity recovery path, `--line`, JSON key order, cp1252) were spot-checked again before commit.

Sizes use the audits' scale: **S** ≤ ½ day, **M** 1–3 days, **L** about a week, **XL** more than a week, for one agent session or one person. Calendar estimates assume three to four coding agents in parallel plus the owner, and they are honest guesses, not commitments.

---

## 1. Verdict

**Linux is closest in code and furthest in the things around the code.** The editor, file safety, formats, security and clipboard all run and are tested on Linux, but the build is not shippable: every rfd message dialog shells out to `zenity`, and without it the startup recovery prompt silently deletes recovered work (`crates/bp-ui/src/lib.rs:383-400`); the app dlopens eleven libraries nobody declares and panics at start without `libxkbcommon-x11`; the release binary built here needs glibc 2.39; "Consolas" is hard-coded so the editor is proportional; and there is no `.desktop`, icon, package or app id (D9). **Biggest Linux risk: the zenity dialog path, because it destroys data.** With the dialogs moved in-app and packaging done on a Debian 12 baseline, Linux can reach RC1 first and without any purchase.

**Windows 11 is where the product will be judged, and it has the least evidence.** Nothing in the repository has ever run on Windows 11; the Windows gate has not run on the last eight commits (`project/NEXT_SESSION.md`), and the shipping surface has no installer, icon, manifest, version resource, signing, panic hook, log file or working `--version` under `windows_subsystem`. **Biggest Windows 11 risk: code signing.** Smart App Control blocks unsigned unknown binaries outright, and a certificate or Azure Trusted Signing identity has procurement lead time measured in weeks, so it must be started in week 1 or it becomes the critical path.

**Windows 10 is the same binary with two extra problems.** Consumer ESU ends 2026-10-13, eighteen days from now, so the RC will land in a world where Win10 is ESU/LTSC only and the plan needs a stated support policy. And the "Notepad replacement" preset refuses the files Notepad users have most: legacy ANSI/cp1252 `.txt` fails at open (`crates/bp-files/src/load.rs:109`). **Biggest Windows 10 risk: shipping a Notepad replacement that cannot open Notepad's files.**

Across all three: licensing is unresolved (no `LICENSE-*`, no third-party notices, no Slint licence choice, an About box that states MIT/Apache for a binary that embeds Slint), the large-file engine is unwired so a 2 GB open loads 2 GB, everything runs on the UI thread, and five of fourteen menus are placeholders. None of these is hard; all of them are RC-blocking.

---

## 2. What "RC1" means, per OS

An RC1 is a build a stranger could install, use as their default editor for a week, and uninstall cleanly, and for which we hold evidence on that OS. It is not feature-complete against `specs.md`; section 3 says what is in it. A tag is `v1.0.0-rc.N` (one tag, artifacts per OS; an OS whose criteria are not met is simply absent from that tag's assets and listed as "not yet" in the release notes).

### Exit criteria common to every OS

| # | Criterion | How it is checked |
|---|---|---|
| C1 | The release surface has no placeholder: no `planned()` row, no `planned_menu`, no "not implemented yet (phase N)" text. Every visible row either works on that OS or is greyed with a reason the user can act on | `menus.rs` test `no_release_row_is_a_placeholder` (M1-05); manual click-through |
| C2 | Open → type → Ctrl+S → safe, end to end, in the default view, proven by a headless window test and by a person on the real OS | `window_tests.rs` (M1-12); RC checklist |
| C3 | Recovery: kill the process with unsaved work; relaunch; the work is offered and restored; declining does not delete it unless the user explicitly said so; a second running instance never touches the first's journal | `bp-history` tests (M0-01, M0-02); RC checklist |
| C4 | Opening a file of any size never hangs or OOMs the app: ≤ 32 MiB editable, larger read-only in the viewer, and the status bar says which | `bp-files`/`bp-ui` tests with lowered thresholds (M0-04, M1-06); manual with a 1 GB log |
| C5 | Nothing blocks the event loop for more than ~100 ms: Find in Files, checkpoints, hashing and save run on the worker with progress and cancel | state tests with a slow fake worker (M1-07); manual on a 10 k-file tree |
| C6 | Data fidelity: JSON Format keeps key order, TOML Format warns before dropping comments, YAML warns as today | `bp-data` tests (M0-05) |
| C7 | Privacy: the clipboard history records only in-app copies unless the user opts in; copies from Private+ documents are excluded from Win+V/cloud history; the recent list honours the profile and Privacy Mode | `bp-clipboard` and state tests (M0-07, M0-09) |
| C8 | Settings persist: theme, wrap, line numbers, zoom, indent, clipboard opt-in, default profile, across restarts, via `Tools ▸ Settings…` | `bp-config` round-trip tests, window test (M1-03) |
| C9 | CLI: `bachelorpad file`, `--line N`, `--readonly`, `--version`, `--help`, `--`, unknown-flag warning | `bp-config::cli` tests (M0-06); manual |
| C10 | Failures are visible: a panic writes `crash-<ts>.log` under the state dir and shows a message; the log file exists and contains no document content | spawn test of the binary (M0-10); log-hygiene gate stage |
| C11 | Licensing: `LICENSE-MIT`, `LICENSE-APACHE`, `THIRD-PARTY-NOTICES` (cargo-about, including the bundled font and the 13 Slint crates), an About window that states the Slint licence and hosts `AboutSlint` (royalty-free option) or GPL terms (GPL option), `cargo deny check` green | files present in every package; `cargo deny` in the gate (M1-14, M1-02) |
| C12 | Supply chain: `cargo build --release --locked` on a pinned toolchain, `SHA256SUMS` plus a minisign signature, an SBOM per target, split debuginfo kept | release script output in `artifacts/releases/<tag>/` (M3-03) |
| C13 | The gate is green on that OS for the tagged commit: fmt, `check --locked`, clippy `-D warnings`, test, release build, `--self-check` launch of the release binary, log hygiene, fuzz | `artifacts/test-evidence/<tag>-<os>.json` (M0-12, M0-13) |
| C14 | Performance measured on that OS on the RC build and recorded: time to window, time to first interaction, warm start, idle RAM, 1/10 MB open and typing latency, 100 MB viewer open | `artifacts/benchmarks/<tag>-<os>.json` (M4-01..03) |
| C15 | Docs: user README (install per OS, requirements, known gaps), user guide, CHANGELOG, SECURITY.md (with the D4 "unreviewed envelope" statement), CONTRIBUTING; `PLATFORM_MATRIX.md` regenerated from `capabilities::report` | files present; matrix drift test (M2-12, M3-05) |
| C16 | The RC checklist (`docs/release/RC_CHECKLIST.md`) run by a person on that OS with the evidence JSON committed | `artifacts/test-evidence/` (M4-04) |

### Linux-specific

| # | Criterion |
|---|---|
| L1 | No `zenity` dependency: every message dialog is in-app Slint; file pickers still use the portal and say so when no portal is present |
| L2 | Packages: `.deb` (Debian 12/Ubuntu 22.04+), `.rpm` (Fedora), `.tar.gz`, `PKGBUILD` (AUR `-bin`); all runtime libraries declared; built on a glibc 2.36 baseline (`objdump -T` checked by the release script); a Recommends on an xdg-desktop-portal backend |
| L3 | Desktop integration: static `<app-id>.desktop`, AppStream metainfo, hicolor icons and shared-mime-info package installed by the package; `Set as Default Editor` runs `xdg-mime default` with consent and offers "Remove Registration"; app id is reverse-DNS (D9) |
| L4 | Bundled monospace font on every surface; `--editor-view` caret lands under the pointer on Xvfb |
| L5 | Clipboard works natively on Wayland (`wayland-data-control`) and survives the process holding one `Clipboard` |
| L6 | Smoke-tested on GNOME Wayland, KDE Plasma Wayland and one X11 desktop; IME (IBus or Fcitx5) types CJK in the default view; `SLINT_BACKEND=winit-x11` documented as the fallback |

### Windows 11 and Windows 10 (one build, two test passes)

| # | Criterion |
|---|---|
| W1 | Per-user installer (Inno Setup, HKCU) that writes exactly `bp_platform::editor::windows::registry_values`, sends `SHChangeNotify`, and removes every value on uninstall, asking before deleting state/data; plus a portable zip that keeps the `.reg` route (UTF-16LE BOM, with an unregister `.reg`) |
| W2 | Authenticode on the exe and the installer with an RFC 3161 timestamp; Smart App Control set to "On" on the Win11 test machine does not block the installer |
| W3 | Resources: multi-size icon, VERSIONINFO from `CARGO_PKG_VERSION`, manifest (comctl32 v6, PerMonitorV2, `longPathAware`, supportedOS Win10/11, UTF-8 code page); static CRT or the redistributable chained |
| W4 | `--version`/`--help` print in a console under the GUI subsystem (`AttachConsole`); a panic shows a `MessageBoxW` and writes the crash log |
| W5 | Legacy ANSI/cp1252 `.txt` opens, is labelled, and round-trips; characters outside cp1252 prompt to switch to UTF-8 |
| W6 | `Set as Default Editor` opens Windows Settings on the app's own page on Win11 (`ms-settings:defaultapps?registeredAppUser=...`) and the generic page on Win10; Explorer shows the icon for associated types after install |
| W7 | Tested on Win11 23H2 or 24H2 **and** Win10 22H2 clean VMs with no VC++ redistributable: install, associate, double-click a file, HiDPI 125/150/200 %, Microsoft IME (Pinyin or Japanese), uninstall |
| W8 | Release notes carry the Windows 10 support statement (ESU/LTSC, best effort, same binary) |

---

## 3. Scope: what RC1 contains

The spec is a full product; RC1 is the text appliance that the tagline promises, with every locked decision that can be honoured on the current toolkit honoured, and the rest deferred **openly**. Three locked decisions cannot be met in RC1 without XL work and are presented as decisions for the owner in section 4 (S1–S4), not silently dropped.

### IN RC1

| Area | What ships | Spec § |
|---|---|---|
| Core editor (default `TextInput` view) | Tabs, Ctrl+Tab, open many, undo, wrap, line numbers, zoom, Ln/Col, Go to Line, Duplicate/Move Line, Insert Date/Time, editor context menu, Ctrl+S proven, multi-select Open | 3, 4 |
| `--editor-view` | Ships as **experimental** behind the flag and a Settings toggle, labelled; not in the RC checklist except caret-under-pointer on Linux | 4 (D2) |
| Large files | Size guard; editable ≤ 32 MiB; read-only viewer above (`bp-buffer::LargeFile`) with a banner; `--readonly`, `--large-file`; streaming search inside the viewer | 5 |
| Responsiveness | Worker thread for Find in Files, checkpoints, hashing, save; progress and cancel | 5, 22 |
| File safety | Atomic save, watcher, journal with instance lock, restore-on-launch, collision suffix, Save as New Revision, creation date preserved (locked #4), path always visible (locked #5) | 2, 3, 6 |
| Formats | Detection for all listed types; Data tools for JSON (order-preserving), YAML, TOML (with warning), JSONL, CSV/TSV; cp1252 on Windows; UTF-16 | 7, 9 |
| Markdown | Source view + Outline (fence-aware fix) | 8 |
| Semantic | Layer 1 only: title, summary, keywords, outline, statistics, semantic naming | 10 |
| Clipboard | In-app history in the editor context menu and Edit menu (locked #11); pin, clear; format-aware paste; opt-in recording of other apps; exclusion from Win+V/cloud under Private+; timed clear | 14 |
| Security | `.bpadx`, passphrases, Argon2id (D10 ceiling), profiles persisted, Privacy Mode, secret scan, hash, verify, redact, metadata inspector, audit history, sealed journal, `zeroise` on close; `SECURITY.md` states the envelope is unreviewed (D4) | 15 |
| Themes | Four themes + Follow System, persisted | 16 |
| Brand | Original icon set (owner-approved); the tagline; no personality mode yet | 17 |
| Menus | File, Edit, View, Insert, Format, Data, Note, Security, Tools (Settings, Open Log Folder, Open Config Folder), Help (Shortcuts, About, Roadmap link) | 18 |
| Default editor | Windows: installer-written HKCU registration, Settings deep link, portable `.reg`; Linux: package-installed `.desktop`/MIME/icons, `xdg-mime` with consent, unregister (locked #13, #14) | 19 |
| CLI | `bachelorpad file`, `--line`, `--readonly`, `--large-file`, `--version`, `--help`, `--`; `bpad` as a symlink/shim in the packages | 19 |
| Platform | `bp-platform` + a new `bp-platform-windows` (Win32 shims only) | 20 |
| Perf evidence | Measured on all three OSes | 22 |
| Testing | Existing suites + window tests + release-build launch + `cargo deny`; coverage-guided fuzz run once on Linux | 23 |

### DEFERRED to 1.x (no locked decision affected, or affected only in a way the owner has already accepted)

Multi-cursor, column selection, folding, indentation guides, syntax highlighting, Markdown preview/split/export/TOC/heading ops (U4), JSON tree and CSV table views, filter/query/statistics, YAML sort keys, JSONPath copy, INI/XML tools, revision history, Tags/Related Notes (D3 = keep unwired), Compare-with-disk diff, session restore of clean tabs, single-instance hand-off (see M2 note), fuzzy search, replace in files, Lock Document/auto-lock, Security Inspector panel, Custom profile editor, native Windows clipboard history (WinRT), Diagnostics, user theme files, per-surface theme roles, accessibility beyond menu roles and F10, i18n, print, drag-and-drop (Slint), `bp-platform-linux`.

### OWNER'S CALL: locked decisions RC1 cannot honour in full

| ID | Locked decision | What RC1 would ship | What is deferred | Trade-off |
|---|---|---|---|---|
| S1 | #7 Notebook execution, mixed-language cells, run-selection | Nothing user-visible; `.ipynb` opens as JSON; Notebook and Run menus hidden | Cell view, `bp-execution`, per-OS process spawn/kill, output rendering: **XL, two crates, a new Slint view** | Including it pushes RC1 out by 6–10 weeks and adds the largest attack surface in the product to the first release. Recommendation: defer to 1.1 and say so in the roadmap and the release notes |
| S2 | #8 Local semantic organisation: related notes, duplicate detection, hybrid search | Semantic naming and layer-1 analysis only; Organize menu hidden | `bp-storage` wiring (D3), similarity, simhash, index, search UI: **XL** | Same reasoning. Also blocked on D1/D3 product answers. Recommendation: 1.2 |
| S3 | #9 memory mapping | Chunked reads (ADR-0027); the README stops advertising mmap | mmap needs `unsafe` (U1) | Recommendation: accept the decline permanently; register it |
| S4 | #12 DPAPI / Windows Hello / Linux keyring; Sign Document | Everything else in #12; Sign greyed with reason, or hidden | Key protection needs D5 plus platform adapters: M–L | Recommendation: D5 option (c) (seed sealed in a passphrase envelope) lands Sign in RC1 for S–M; DPAPI/Hello/keyring go to 1.x |
| S5 | #15 personality modes (ADR-0013 says Standard and Minimal "must exist") | Icon and brand assets only | String table and mode switch: M | Recommendation: defer; it is cosmetic and the assets are the blocking part |
| S6 | #6 "common source formats" | Detection and label only; no highlighting | Highlighter + styled runs, only possible in the custom view: L | Recommendation: defer; say "source files open as plain text" in the notes |

Research mode (`bp-research`, orphan crate, 5.2 kLOC) is not a locked decision; it stays unwired and its menu is hidden.

### Placeholder menus: the position

Hide them. A release does not advertise its backlog in the menu bar. Notebook, Organize, Research and Run come off the bar in M1-05 (the builders stay in `menus.rs` behind a `show_planned_menus` config key that defaults to false, so agents and the owner can still see them). Tools becomes live because Settings, Open Log Folder and Open Config Folder are real. Every `planned()` row and every "— not implemented yet (phase N)" trailer inside live menus is removed; Help gains a "Roadmap" row that opens the public roadmap URL. Greyed-with-reason rows (`row_enabled(.., false)`) stay, because they describe the current document, not the backlog.

---

## 4. Decisions the owner must make, in the order they block work

Batch these: every one is a one-line answer. "Deadline" is the milestone that cannot finish without it.

**Answered 2026-09-25:** 1 (scope, ADR-0030), 2 (Slint royalty-free, ADR-0031 -- the copyright holder is still open), 4 (`io.github.dboles99.BachelorPad`), 5 (DejaVu Sans Mono) and 6 (installer-owned registration), all ADR-0032; 7 (ADR-0033); 16 (D5 option c, via 1). Decision 3, the signing identity, is being applied for by the owner, who is also running the Windows gate (M0-12).

| # | Decision | Recommendation | Unblocks | Deadline |
|---|---|---|---|---|
| 1 | **Scope sign-off** (S1–S6 above) | Defer S1, S2, S5, S6; accept S3; take S4 = D5(c) | The whole plan | Before M0 ends |
| 2 | **Slint licence**: royalty-free 2.0 (About window with `AboutSlint` and/or the badge on the download page), GPLv3 combined work, or commercial | **Royalty-free**, recorded in an ADR; About states "BachelorPad+ is MIT OR Apache-2.0; this binary includes Slint under the Slint Royalty-free Licence 2.0". Also name the **copyright holder** for `LICENSE-*` ("BachelorPad+ contributors" is not a legal entity) | M1-02, M1-14, every package | M1 |
| 3 | **Signing identity** (human, costs money, lead time): Azure Trusted Signing or an OV certificate on cloud KMS | Start the application in week 1, whichever is cheaper to obtain as an individual/small entity; timestamped Authenticode either way | W2, M3-04, Windows RC1 | Start M0; needed by M3 |
| 4 | **D9 app id** | Reverse-DNS, e.g. `io.github.<owner>.BachelorPad` (no `+`); directory name stays `bachelorpad` | M2-05, M2-07, all Linux packaging, AppStream | M2 |
| 5 | **U2 font** | Bundle DejaVu Sans Mono (Bitstream Vera licence, permissive, no reserved-name clause) on every OS; JetBrains Mono (OFL) if the owner prefers the look | M0-03 | M0 |
| 6 | **D8 Windows registration** | The **installer** writes the HKCU keys and removes them; the app never writes the registry, only reports and deep-links; portable builds keep the `.reg` route | M2-04, M3-01 | M2 |
| 7 | **Win32 linkage**: may a new `bp-platform-windows` crate use `windows-sys` and confined `unsafe`? | Yes, with an ADR: the only crate allowed to; `bp-platform` and `apps/bachelorpad` stay `forbid(unsafe_code)` | M0-06, M0-10, M2-03 | M0 |
| 8 | **U3 huge-file UI** | Viewer tab with a read-only banner, editable limit 32 MiB (configurable) | M1-06 | M1 |
| 9 | **D6 what is a line** | (c) LF, CRLF and bare CR everywhere | M1-09 | M1 |
| 10 | **U5 clipboard rows while a menu is open** | Snapshot on open, row id carries the entry hash | M1-18 | M1 |
| 11 | **D2 two-view fork** | Keep opt-in, label experimental in Settings and docs, out of the RC checklist | M1-04, docs | M1 |
| 12 | **U1 mmap** | Accept ADR-0027's decline; register it; drop "memory-mapped" from README | docs | M1 |
| 13 | **U4 Markdown preview** | Defer past RC1 | scope | M1 |
| 14 | **D3 bp-storage** | Keep unwired for RC1; drop it from the default build features so `rusqlite` stops costing every gate run | scope, gate time | M1 |
| 15 | **D10 Argon2 ceiling** | (c) prompt above 4× default, hard cap 256 MiB / t ≤ 8 | M2-09 | M2 |
| 16 | **D5 signing key** | (c) seed sealed in a passphrase `.bpadx` envelope in the Data dir; if declined, hide Sign for RC1 | M2-10 | M2 |
| 17 | **D1 what leaves the machine** | (a) nothing in v1; ADR defines `Network::Allowed` as "not forbidden, and no consumer exists" | M2-11, SECURITY.md | M2 |
| 18 | **Packaging formats** | Windows: Inno Setup per-user + portable zip. Linux: deb + rpm + tar.gz + PKGBUILD; AppImage/Flatpak later | M3-01, M3-02 | M3 |
| 19 | **Release builds: local script or tag-only Actions workflow** (ADR-0016 amendment) | Local, scripted, run in a pinned container for Linux; recorded in `artifacts/releases/` | M3-03 | M3 |
| 20 | **D4 crypto review** | (b) ship RC1 with the unreviewed statement in SECURITY.md and in the Encrypt consent; (a) before GA — budget and reviewer are the owner's | M3-05 | M3 |
| 21 | **Windows 10 support statement** | "Supported on 22H2 with ESU or LTSC, same binary, tested at each RC; no Win10-only fixes after 1.0" | W8 | M5 |
| 22 | **D7 derive-once key API** | (a), but it blocks nothing in RC1; schedule for 1.1 unless Security History is slow on a real document | none | post-RC1 |

---

## 5. Milestones

Sequence: **M0 stop the bleeding → M1 make the shipping view fit and the gate honest → M2 platform integration → M3 packaging and pipeline → M4 evidence → M5 RC1 per OS.** M0 and M1 serve all OSes; M2 splits by OS; Linux can reach M5 first.

Columns: **OS** = which target the item serves; **Par** = parallel-safe under the collision rules (`AppWindow` in `app.slint` is single-writer; `state.rs`/`dispatch.rs`/`editor_view.rs`/`menus.rs` and the `ui/*.slint` component files can each take a different agent; library crates are one agent each; `README`/`ROADMAP`/`WORK_QUEUE`/`DECISIONS`/workspace `Cargo.toml` are updated by whoever finishes an item, never concurrently). **Verify** names the test, gate stage or measurement that proves it.

### M0 — Stop the bleeding (data safety, privacy, honesty). Target: 1 week. Serves all OSes.

Goal: nothing in the current build can destroy or leak a user's work, and the code stops telling untruths.

| ID | Title | Crate(s) / files | OS | Size | Depends | Par | Verify |
|---|---|---|---|---|---|---|---|
| M0-01 | Recovery prompt: only an explicit **No** discards; `Cancel`, a failed dialog or anything else keeps the journal and shows a status notice. Extract the decision into a pure fn | `bp-ui/src/lib.rs:383-400` (new `recovery_decision()` in `state.rs`) | all (Linux data loss) | S | — | Lane A | unit test `a_dialog_that_could_not_be_shown_keeps_the_journal`; manual on KDE without zenity |
| M0-02 | Journal instance lock: `bp-history` writes `owner.lock` (pid, start time); `pending()` excludes checkpoints held by a live process; `discard_all()` refuses them. **Planner finding, not in the audits: confirm with a failing test first** | `bp-history/src/lib.rs` (`pending`, `discard_all`, `checkpoint`) | all | M | — | Lane C | `bp-history` test with two `Journal`s on one dir; integration test spawning the binary twice with `--self-check=journal` |
| M0-03 | Bundle a monospace font (U2): `import "fonts/DejaVuSansMono.ttf"` in `app.slint`; `font-family` at the six Consolas sites; add its licence to notices | `bp-ui/ui/app.slint:612,628`, `editor_surface.slint:64,155,168`, `results_panel.slint:71`, `bp-ui/fonts/` | Linux (Windows unaffected) | S | decision 5 | Lane A (touches `app.slint`) | Xvfb caret check from session 6 (`--editor-view`, click at column 40); grep test that no `.slint` names Consolas |
| M0-04 | Size guard on open: `bp_files::load` refuses > `HUGE_FILE_BYTES` with `LoadError::TooLarge` and reports `SizeClass`; the shell shows "too large to edit; the viewer arrives in M1" rather than loading it | `bp-files/src/load.rs:86`, `bp-buffer::SizeClass`, `bp-ui/src/state.rs` (`open`) | all | S | — | Lane C then A | `bp-files` test with a `set_len` sparse file; state test for the notice |
| M0-05 | JSON Format keeps key order (`serde_json` `preserve_order`), Sort Keys gets its own action id and really sorts; TOML Format row warns like YAML | `bp-data/src/lib.rs:105-109`, workspace `Cargo.toml:73`, `bp-ui/src/menus.rs:876-919`, `dispatch.rs` | all | S | — | Lane C then B | `bp-data` tests `format_keeps_the_authors_key_order`, `sort_keys_sorts`; menus test for the new id |
| M0-06 | CLI parser in a library: `bp_config::cli::parse` handles `--line N`/`--line=N`, `--readonly`, `--large-file`, `--version`, `--help`, `--`, files starting with `-` after `--`, warns on unknown flags; `main.rs` calls it. `--version`/`--help` on Windows print via `AttachConsole` (M2-03) or a message box until then | `bp-config/src/lib.rs:391-403` (new `cli.rs`), `apps/bachelorpad/src/main.rs:59-66` | all (bug: `--line 427 f` opens `427`) | M | — | Lane C + D | `bp-config` tests; state test that `--line` reaches `go_to_line`/the caret |
| M0-07 | Clipboard privacy default: history records **only in-app copies** (editor-view copy path; default view via a Slint `copied(text)` callback on Ctrl+C/X); the 1.2 s poll runs only when `clipboard.record_external = true` (off by default, explained in Settings); our own copies under Private+ use arboard `exclude_from_history/cloud/monitoring` (Windows) and the KDE hint (Linux) | `bp-clipboard/src/lib.rs` (source policy), `bp-ui/src/lib.rs:975-1003`, `dispatch.rs`, `editor_view.rs:237-252`, `app.slint` (one callback) | all (Windows Win+V/cloud leak) | M | M0-01 (same file) | Lane C, then A | `bp-clipboard` tests; window test `ctrl_c_in_the_default_view_reaches_the_history`; manual Win+V check |
| M0-08 | Stale user-visible text: `menus.rs:799` recovery readout for `Recovery::Encrypted`; comments `menus.rs:795-797`, `bp-security/src/lib.rs:155-157,235`; Data fallback says only "nothing for {format} documents"; Help ▸ Shortcuts stops listing Ctrl+D/Alt+Up/Down until M1-04 lands | `bp-ui/src/menus.rs`, `bp-security/src/lib.rs`, `dispatch.rs:18-31` | all | S | — | Lane B | menus tests updated; grep test for "phase 15" in strings |
| M0-09 | Recent-files list honours `policy.metadata` and Privacy Mode; `recent.rs` writes atomically | `bp-ui/src/state.rs:423-426`, `bp-config/src/recent.rs:108` | all | S | — | Lane A/C | state test `a_confidential_document_never_enters_the_recent_list` |
| M0-10 | Panic hook + log file + crash log: `tracing-appender` rolling file under `DirKind::State/logs` (size/count cap); hook writes `crash-<ts>.log` (version, OS, backend, backtrace), flushes what it can, then Win32 `MessageBoxW` (via M2-03) or stderr. Dependency noted in an ADR | `apps/bachelorpad/src/main.rs`, `bp-platform/src/dirs.rs:58-61` | all (Windows: silent panics) | M | decision 7 | Lane D | integration test spawning the binary with `--self-check=panic` and reading the crash log; log-hygiene stage |
| M0-11 | Housekeeping: `.gitignore` whitelists `CHANGELOG.md`, `SECURITY.md`, `CONTRIBUTING.md`, `LICENSE-*`; `#![forbid(unsafe_code)]` on `bp-security` and `apps/bachelorpad` (T11); `expect_used`/`indexing_slicing` lints as warn (T8) | `.gitignore:34-39`, crate roots, workspace `Cargo.toml` | all | S | — | any | gate green; `git add CHANGELOG.md` no longer ignored |
| M0-12 | **Human + Windows machine:** run `Invoke-LocalCI.ps1` (full) on the branch, fix what it finds, merge to `main` (T12). Repeat at the end of every milestone | `scripts/Invoke-LocalCI.ps1` | Win10/Win11 | S (calendar: owner) | — | human | `artifacts/test-evidence/*.json` from the Windows host |
| M0-13 | Native Linux gate: `Invoke-LocalCI.ps1 -Native` runs under `pwsh` on Linux with every stage (`--locked`, launch under `xvfb-run` of `target/debug/bachelorpad`, log hygiene, fuzz); WORK_QUEUE stops telling Linux agents to run `-Linux` | `scripts/Invoke-LocalCI.ps1:130-176,276-337`, `docs/governance/LOCAL_CI.md` | Linux | M | — | Lane E | the script's results JSON on this container; ADR-0016 note |

**Exit:** C3, C6, C7, C9 (parsing), C10 met on Linux; the Windows gate has run on `main`; the Linux gate runs natively.

### M1 — Make the shipping view fit and the gate honest. Target: 3–4 weeks. Serves all OSes.

Goal: the default view is a complete Notepad; nothing blocks; the release surface is honest; licensing is in place.

| ID | Title | Crate(s) / files | OS | Size | Depends | Par | Verify |
|---|---|---|---|---|---|---|---|
| M1-01 | In-app dialogs: `ui/dialog.slint` (title, body, up to three buttons, default/cancel, keyboard) driven by `AppWindow` properties and one result callback; replace every `rfd::MessageDialog`/`show_info`/consent prompt; `rfd` stays for file pickers only. Enables Save / Don't Save / Cancel | `bp-ui/ui/dialog.slint` (new), `app.slint` (AppWindow), `dispatch.rs:35-42`, `lib.rs:383-400,568-584,1142-1160` | all (removes the zenity dependency on Linux) | L | M0-01 | Lane A (serialised) | window tests: Esc cancels, Enter defaults, close-with-dirty offers three choices; `grep MessageDialog` finds nothing |
| M1-02 | About window: version + git hash + dirty flag (`build.rs`), real backend and renderer, both licences, `AboutSlint`, buttons for Notices, Open Log Folder, Open Config Folder | `bp-ui/ui/about.slint` (new), `dispatch.rs:648-656`, `apps/bachelorpad/build.rs` | all | S | M1-01, decision 2 | Lane B | window test opens About; text contains the Slint licence line |
| M1-03 | Settings: `bp_config::save()` (atomic), `Config` grows (word_wrap, line_numbers, zoom, theme, clipboard.record_external, default_profile, editor_view, show_planned_menus); `Tools ▸ Settings…` dialog; View-menu changes persist | `bp-config/src/lib.rs:124-144`, `bp-ui/ui/settings.slint` (new), `app.slint`, `menus.rs`, `dispatch.rs` | all | M | M1-01 | Lane C then A | `bp-config` round-trip tests; window test toggles wrap, reopens, reads config |
| M1-04 | Default view gains Ln/Col, Go to Line, Insert Date/Time, Duplicate/Move Line using `cursor-position_byte-offset`/`anchor-position-byte-offset` and `set-selection-offsets`; the five `enabled: editor_view` gates go away | `app.slint:679-711`, `state.rs:1895-1911`, `menus.rs:530-544,851`, `dispatch.rs` | all | M | — | Lane A | window tests `ctrl_g_42_lands_on_line_42_in_the_default_view`, `alt_up_moves_the_line`; status bar shows `Ln`/`Col` |
| M1-05 | Hide placeholders: Notebook/Organize/Research/Run off the bar (behind `show_planned_menus`); Tools live (Settings, Open Log Folder, Open Config Folder); remove every `planned()` row and trailer; Help ▸ Roadmap (URL) | `app.slint:424-502`, `lib.rs:240-247`, `menus.rs:989-1082` | all | S | M1-03 | Lane A + B | menus test `no_release_row_is_a_placeholder`; window test counts top-level menus |
| M1-06 | Large-file viewer (U3): open path classifies with `SizeClass`; > editable limit opens a read-only viewer tab (`ui/viewer.slint`, virtualised rows from `LargeFile::display_lines`, banner, Find via `WindowReader`); `--readonly`, `--large-file`; editable limit in Settings | `bp-ui/src/state.rs` (`open`), `ui/viewer.slint` (new), `app.slint`, `bp-buffer/src/{large,stream}.rs` | all | L | M0-04, M1-01 | Lane A (+ Lane C for a `SizeClass::with_thresholds` test hook) | `bp-buffer` tests exist; state test with lowered thresholds; manual 1 GB log on each OS |
| M1-07 | Worker thread: `bp-ui/src/worker.rs` (std thread, mpsc, `slint::invoke_from_event_loop`); Find in Files, checkpoint write, hash and save move off the loop with a generation counter (a save result clears dirty only if the generation is unchanged), progress in the status bar, cancel | `bp-ui/src/worker.rs` (new), `state.rs:305,909-950,1222`, `lib.rs:887-892,951-966` | all | L | M1-01 | Lane A (state.rs) | state tests with a fake worker; test `a_save_that_finishes_after_more_typing_keeps_the_document_dirty`; timing test on a 5 k-file tree |
| M1-08 | Checkpoint only documents whose generation changed since the last checkpoint | `bp-ui/src/state.rs:305`, `bp-history` | all | S | M1-07 | Lane A | state test counting writes |
| M1-09 | D6 patch: LF, CRLF, bare CR everywhere (`ropey` `cr_lines`; `bp-search` adopts the same set); un-ignore `search_over_buffer.rs:299` | `bp-buffer/Cargo.toml`, `bp-search/src/lib.rs` | all | S | decision 9 | Lane C | the un-ignored integration test |
| M1-10 | Legacy encodings: std-only cp1252 table in `bp-core::encoding` (decode + encode), `bp_files::load` falls back to it when not UTF-8/UTF-16 (Windows-1252 label; on Linux label Latin-1/1252 the same way), Format menu row, save round-trips, out-of-range char prompts to switch to UTF-8 | `bp-core/src/encoding.rs`, `bp-files/src/load.rs:109`, `bp-files/src/encode.rs`, `menus.rs` (Format) | Win10/Win11 primarily | M | M1-01 (prompt) | Lane C then B | `bp-files` fixture tests (é, €, smart quotes); both legs |
| M1-11 | Open dialog multi-select (`pick_files`); Ctrl+Tab / Ctrl+Shift+Tab | `dispatch.rs:61-65,18` | all | S | — | Lane B | window test for Ctrl+Tab; dispatch test |
| M1-12 | Verification tooling: Ctrl+S end-to-end window test; `scripts/smoke-linux.sh` (Xvfb + python-xlib from session 6) kept in the repo; window test helper for right-click and dialogs | `bp-ui/src/window_tests.rs`, `scripts/` | all | S | — | Lane B/E | the tests themselves |
| M1-13 | Build hygiene: `[profile.release]` (thin LTO, `codegen-units=1`, `strip="symbols"`, split debuginfo kept, `panic="unwind"`), pin `rust-toolchain.toml` to an exact version, `rust-version`, `.cargo/config.toml` (msvc `+crt-static`, `--remap-path-prefix`), Slint `default-features=false` (drop `system-tray`), arboard without `image-data` | workspace `Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`, `bp-ui/Cargo.toml:47-48` | all | S | — | Lane E (serialised on `Cargo.toml`) | gate builds release; size recorded in `artifacts/benchmarks/` |
| M1-14 | Licensing files: `LICENSE-MIT`, `LICENSE-APACHE`, `THIRD-PARTY-NOTICES` via `cargo about` (font + Slint royalty-free text), `deny.toml` (licence allow-list, advisories, crates.io only), `cargo deny check` and `cargo audit` in the gate | repo root, `scripts/Invoke-LocalCI.ps1` | all | S | decision 2 | Lane E | `cargo deny check` green in the gate |
| M1-15 | Creation date fidelity (locked #4): opened file's `created_at` from `SemanticName::parse` of the existing name, else `Metadata::created()`, else mtime | `state.rs:891`, `bp-naming` | all (source differs per OS) | S | — | Lane A | state test `semantic_rename_keeps_the_date_already_in_the_name` |
| M1-16 | Path always visible (locked #5): notices get their own status segment; errors never replace the path | `status_bar.slint:88-89`, `state.rs` (`error`) | all | S | — | Lane B | state test `a_notice_never_hides_the_path` |
| M1-17 | Save As suggests a collision-free name (`resolve_in_dir`); File ▸ Save as New Revision (`with_revision`) | `state.rs:1830`, `bp-files/src/save.rs:442`, `menus.rs`, `dispatch.rs` | all | S | — | Lane B | state tests |
| M1-18 | Editor context menu (locked #11): `ContextMenuArea` in both views with Cut/Copy/Paste/Select All/Paste Special…/Clipboard History (snapshot on open, U5)/Pin/Clear | `app.slint`, `editor_surface.slint:214`, `menus.rs:488-521`, `dispatch.rs:678-707`, `bp-clipboard::toggle_pin` | all | M | M1-04, decision 10 | Lane A | window test: right-button `PointerPressed` opens the menu; rows present; pin toggles |
| M1-19 | Security profile persists for plaintext documents (map under `DirKind::State` keyed by path digest, as ADR-0022 keys the journal); `.bpadx` carries it in the header | `bp-ui/src/state.rs`, `bp-config` or `bp-security`, `bp-crypto` header | all | M | — | Lane C then A | state test `reopening_a_confidential_document_is_confidential` |
| M1-20 | Axes with a reader: `Zeroise::On` wipes the rope and strings on close; `TemporaryFiles::Denied` documented as "no temporary outside the document's directory" with the atomic-save temp recorded as an accepted deviation in `SECURITY_MODEL.md`; `embeddings` stays decoration and the doc says so | `bp-ui/src/state.rs` (close), `bp-security/src/lib.rs:124-126`, `docs/architecture/SECURITY_MODEL.md` | all | S | — | Lane A + docs | test that close drops through `Zeroizing`; doc updated. **Evidence thin: wiping Slint's own `TextInput` copy is not possible; say so** |
| M1-21 | Markdown outline ignores fenced code; Follow System on X11 reports "unknown" in the row instead of silently doing nothing | `bp-semantic/src/lib.rs:103`, `menus.rs` (View) | all | S | — | Lane C/B | `bp-semantic` test; menus test |
| M1-22 | Records sweep 1: ROADMAP status/date, WORK_QUEUE (done rows, `-Native`), NEXT_SESSION, DECISIONS.md answered rows, README known gaps, ADR-0018/0027 amendments (S15–S28) | docs | all | S | end of M1 | serialised, one agent | the audit's S-list re-checked |

**Exit:** C1, C2, C4, C5, C8, C11 met on Linux and the Windows gate green; a 1 GB file opens in the viewer; no `MessageDialog` in the code.

### M2 — Platform integration. Target: 2–3 weeks. Splits by OS.

Goal: the app looks and registers like a native application on each OS, and every capability the docs claim is true.

| ID | Title | Crate(s) / files | OS | Size | Depends | Par | Verify |
|---|---|---|---|---|---|---|---|
| M2-01 | Icon set from `docs/brand/BRAND_GUIDE.md`: original SVG, hicolor PNGs 16–512, multi-size `.ico`. **Owner approves the artwork** | `assets/icon/` (new) | all | M | — | any + human | files exist; owner sign-off recorded in `artifacts/ui/` |
| M2-02 | Windows resources in `apps/bachelorpad/build.rs` (`winresource`): icon, VERSIONINFO from `CARGO_PKG_VERSION`, manifest (comctl32 v6, PerMonitorV2, `longPathAware`, supportedOS, UTF-8 code page); correct the wrong rfd comment in `bp-ui/Cargo.toml` and R011 trap 8 | `apps/bachelorpad/build.rs`, `apps/bachelorpad/app.manifest`, `bp-ui/Cargo.toml`, `prompts/rosettas/R011…md` | Win10/Win11 | S | M2-01 | Lane D | Windows gate `--self-check` of the release exe; `(Get-Item exe).VersionInfo` asserted in the gate |
| M2-03 | `bp-platform-windows` (ADR): `windows-sys` only, confined `unsafe`, `AttachConsole`, `MessageBoxW`, `SHChangeNotify(SHCNE_ASSOCCHANGED)`, `ShellExecuteW` for `ms-settings:`, `UserChoice\ProgId` read for `AssociationQuery`; `bp-platform` stays dependency-free | `crates/bp-platform-windows/` (new), `apps/bachelorpad`, `bp-ui/src/default_editor.rs` | Win10/Win11 | M | decision 7 | Lane C (new crate) | `cfg(windows)` unit tests; Windows gate; capability matrix updated |
| M2-04 | D8 delivered: `.reg` written UTF-16LE with BOM; unregister `.reg` (`-` keys); `Capabilities\ApplicationIcon`; the in-app row reports, then "Open Windows Settings" (Win11 `registeredAppUser` deep link by build number, Win10 generic) and, for portable builds, "Save registration file…" | `bp-platform/src/editor/windows.rs:207-309`, `bp-ui/src/default_editor.rs:230,307`, `dispatch.rs:495-551` | Win10/Win11 | M | M2-03, decision 6 | Lane C then B | `bp-platform` byte-level tests (BOM, `-` keys); manual on both VMs |
| M2-05 | Static Linux assets generated from `bp-platform` (a test asserts the shipped `.desktop`/MIME XML equal `desktop_entry`/`mime_package` output): `packaging/linux/<id>.desktop`, `<id>.metainfo.xml` (AppStream, `content_rating`, `provides/mediatype`, releases), `<id>.xml`, hicolor icons | `packaging/linux/` (new), `bp-platform/src/editor/desktop.rs:70-148` | Linux | S | decision 4, M2-01, M2-07 | Lane E | drift test; `appstreamcli validate` in the release script |
| M2-06 | Linux Set as Default: run `update-desktop-database`, `update-mime-database`, `xdg-mime default` with consent (`std::process::Command` from the shell, command lines from `bp-platform`); skip the per-user `.desktop` when a system one exists; `$APPIMAGE` Exec; Flatpak → row greyed; "Remove Registration" row; association lookup follows the XDG chain (`$XDG_CURRENT_DESKTOP-mimeapps.list`, `/etc/xdg`, `/usr/share/applications`, `mimeinfo.cache`) | `bp-platform/src/editor.rs:460`, `state.rs:164-187` (bp-platform), `bp-ui/src/default_editor.rs:130-150` | Linux | M | M2-05 | Lane C then B | `bp-platform` tests with temp XDG roots; integration test with a fake `xdg-mime` on `PATH`; manual on GNOME/KDE |
| M2-07 | D9: app id becomes reverse-DNS; `APP_DIR` stays `bachelorpad` | `bp-platform/src/lib.rs:90`, `editor.rs:104` | Linux (Windows unaffected) | S | decision 4 | Lane C | tests updated; grep for the old id in `packaging/` |
| M2-08 | Clipboard on Linux: one `arboard::Clipboard` for the process, `wayland-data-control`, timed clear under Private+ (policy in `bp-clipboard`, timer in the shell) | `bp-ui/src/lib.rs:281-288`, `bp-ui/Cargo.toml:47`, `bp-clipboard` | Linux (timed clear: all) | S–M | M0-07 | Lane C then A | `bp-clipboard` timer-policy test; manual on GNOME Wayland without XWayland sync |
| M2-09 | D10: Argon2 ceiling per decision 15; prompt above 4× default | `bp-crypto/src/header.rs:74`, `bp-ui` (dialog) | all | S | decision 15, M1-01 | Lane C then B | `bp-crypto` tests for the cap and the prompt threshold |
| M2-10 | Sign Document per D5(c): seed sealed in a passphrase `.bpadx` envelope under `DirKind::Data`; row enabled; or hide the row if declined | `bp-integrity`, `bp-ui/src/menus.rs:752`, `dispatch.rs` | all | M | decision 16 | Lane C then B | `signing_over_files.rs` extended; state tests |
| M2-11 | D1 ADR: nothing leaves the machine in v1; `Network::Allowed` defined; Security readout "Leaves this machine: nothing (no network code)" | `docs/decisions/ADR-0030.md`, `menus.rs:690` | all | S | decision 17 | docs + Lane B | ADR merged; menus test |
| M2-12 | `PLATFORM_MATRIX.md` generated from `capabilities::report` with a drift test; `SECURITY_MODEL.md` lists only enforced propagation | `bp-platform/src/capabilities.rs`, `docs/architecture/*.md` | all | S | M2-03, M2-06 | Lane C | the drift test |
| M2-13 | Records sweep 2 (S1–S14, S29–S34): delete the 49 template task files or give them acceptance criteria (ADR-0014); phase READMEs get real status; README install section replaced; `metadata/repository_manifest.json` regenerated or removed; `database/` README says what it is | `project/tasks/**`, `README.md`, `metadata/`, `database/` | all | M | — | serialised, one agent | audit S-list re-checked |
| M2-14 | Accessibility minimum: `accessible-role/label/action-default` on MenuLabel, MenuRow, tabs, bars, status; F10/Alt opens the bar; arrows, Enter, Esc, mnemonics | `bp-ui/ui/menu.slint:9-60`, `app.slint` (bar) | all | M | M1-05 | Lane B then A | window test `f10_opens_the_first_menu`; NVDA/Orca smoke in the RC checklist (human) |
| M2-15 | `Journal::sealed_count` hint: status notice "N encrypted document(s) have recovered work; open them to restore" (F21) | `bp-ui/src/state.rs` | all | S | — | Lane A | state test |
| M2-16 | `bpad` alias: symlink in Linux packages, `bpad.cmd` shim in the Windows installer/zip | `packaging/` | all | S | M3 | Lane E | package install test |

**Single-instance hand-off:** deferred to 1.1. After M0-02 a second instance can no longer damage the first's journal, and one window per file is what classic Notepad does. A std-only design exists (lock file plus loopback listener) if the owner wants it in RC1 (M, Lane C in `bp-platform`).

**Exit:** L3–L5 and W3–W6 code-complete; capability matrix true; the Windows gate green with the new crate.

### M3 — Packaging and the release pipeline. Target: 2 weeks (plus signing lead time).

| ID | Title | Crate(s) / files | OS | Size | Depends | Par | Verify |
|---|---|---|---|---|---|---|---|
| M3-01 | Windows installer: Inno Setup per-user `packaging/windows/bachelorpad.iss` whose `[Registry]` section is generated from `registry_values` (a test compares them); uninstall removes every value, `SHChangeNotify`, asks before deleting state/data; portable zip with `bpad.cmd` and the `.reg` pair | `packaging/windows/`, `bp-platform` (generator) | Win10/Win11 | M | M2-02, M2-04, decision 18 | Lane E | drift test; `packaging/windows/Test-Install.ps1` (`reg query` before/after, uninstall leaves nothing) on both VMs (human) |
| M3-02 | Linux packages: `cargo-deb` and `cargo-generate-rpm` (or `nfpm`) with declared Depends (fontconfig, freetype, xkbcommon, xkbcommon-x11, x11, x11-xcb, xcursor, xi, wayland-client, dbus) and Recommends (`xdg-desktop-portal-gtk | -kde`), postinst triggers, static assets from M2-05; `tar.gz`; `PKGBUILD` `-bin`; built in `packaging/linux/Dockerfile.release` (Debian 12, glibc 2.36) | `packaging/linux/`, `Cargo.toml` `[package.metadata.deb]` | Linux | M | M2-05, M1-13, decision 18 | Lane E | release script asserts max `GLIBC_` ≤ 2.36; install + `--self-check` under `xvfb-run` in Ubuntu 22.04/24.04, Fedora, Arch containers |
| M3-03 | Release script `scripts/Invoke-Release.ps1` (pwsh, both OS): version check, `cargo deny`, `cargo build --release --locked` with remap, SBOM (`cargo auditable` + CycloneDX), split debuginfo archived, `SHA256SUMS` + minisign, `artifacts/releases/<tag>/manifest.json` (rustc, lock hash, host), `gh release upload`; ADR-0016 amendment | `scripts/`, `docs/decisions/ADR-0031.md` | all | M | M1-13, M1-14, decision 19 | Lane E | a dry run producing the artifact set for `v1.0.0-rc.0` |
| M3-04 | Signing step: `signtool`/`AzureSignTool` on exe and installer with RFC 3161 timestamp; identity from the owner's procurement (decision 3) | `scripts/Invoke-Release.ps1` | Win10/Win11 | S + human | decision 3 | Lane E + human | `signtool verify /pa`; Smart App Control test (W2) |
| M3-05 | User docs: README rewritten for users (dev material to `CONTRIBUTING.md`), `docs/user/GUIDE.md`, `CHANGELOG.md` (Keep a Changelog), `SECURITY.md` (disclosure address, supported versions, D4 statement, D1 statement), `docs/release/RC_CHECKLIST.md` | repo root, `docs/` | all | M | M0-11 | docs lane | files present; links checked |
| M3-06 | Version `1.0.0-rc.1`; `build.rs` git hash/dirty; CHANGELOG entry | workspace `Cargo.toml:64`, `apps/bachelorpad/build.rs` | all | S | M1-02 | Lane D | About shows the hash |
| M3-07 | Hosted workflow kept in step but dormant (ADR-0016): apt deps, launch, log hygiene, deny | `.github/workflows/ci.yml` | all | S | M0-13 | Lane E | `act`-style dry run or review |
| M3-08 | Post-RC1: winget manifest, AUR submission | `packaging/` | all | S | first signed release | Lane E | — |

**Exit:** every package installs and uninstalls cleanly on a clean VM/container; `artifacts/releases/v1.0.0-rc.0/` holds a complete dry-run set; SECURITY.md and CHANGELOG exist.

### M4 — Evidence on each OS. Target: 2 weeks, dominated by human time.

| ID | Title | Crate(s) / files | OS | Size | Depends | Par | Verify |
|---|---|---|---|---|---|---|---|
| M4-01 | Startup measurement on all OSes: in-app clock (`--measure-exit` extended: time to window, first frame, first idle after first frame = TTFI, warm run), `scripts/measure-startup.sh` for Linux, `Measure-Startup.ps1` reports warm for software; ADR-0017's "known hole" closed | `bp-ui/src/lib.rs:405`, `scripts/` | all | M | M1-13 | Lane A + E | `artifacts/benchmarks/<tag>-{win10,win11,linux}.json` |
| M4-02 | Open-time and typing latency at 1/10/32 MB (`--latency-probe`, new `--open-probe`), viewer open at 100 MB and 1 GB | `bp-ui/src/lib.rs:1043-1108` | all | S | M1-06 | Lane A | same JSON |
| M4-03 | Idle RAM (working set / PSS) per OS; investigate the 20→32 MB drift after dialogs (X6), which M1-01 may have removed | scripts | all | S | M1-01 | Lane E | same JSON |
| M4-04 | **Human:** run `docs/release/RC_CHECKLIST.md` on Win11 (Smart App Control on), Win10 22H2, GNOME Wayland, KDE Wayland, one X11 desktop: install, first run, open/type/save, kill and recover, `.bpadx` round trip, profiles, Set as Default and double-click from Explorer/Files, HiDPI 125/150/200 %, IME, screen reader smoke, uninstall; evidence via `New-ArtifactMetadata.ps1` | `artifacts/test-evidence/` | all | 1–2 days per OS | M3 | human | committed evidence JSON per OS |
| M4-05 | Coverage-guided fuzz run (cargo-fuzz, nightly, Linux) over `bp-crypto::open`, `bp-data`, `bp-formats`, `bp-files::load`, cp1252 decode; findings fixed | `fuzz/` | all | M | M1-10 | Lane C | run log in `artifacts/security/`; T10 note that `bp-research` is unshipped |
| M4-06 | `docs/security/THREAT_MODEL.md` from spec §24 with the honest per-category status | docs | all | S | M2-11 | docs | file present; linked from SECURITY.md |
| M4-07 | Windows gate run on the RC commit; Linux native gate run; both JSONs committed | scripts | all | S | all | human + Lane E | C13 |

**Exit:** C13, C14, C16 per OS.

### M5 — RC1 per OS

| ID | Title | OS | Depends | Verify |
|---|---|---|---|---|
| M5-L | Tag `v1.0.0-rc.1`; upload deb, rpm, tar.gz, PKGBUILD, `SHA256SUMS(.minisig)`, SBOM; release notes with known gaps and the S1–S6 deferrals | Linux | C1–C16, L1–L6 | assets present; a fresh Ubuntu 22.04 VM installs from the release page and passes the checklist |
| M5-W11 | Same tag, signed installer + portable zip | Windows 11 | C1–C16, W1–W7 | clean Win11 VM with Smart App Control on installs without a block |
| M5-W10 | Same binaries, Win10 checklist evidence, support statement | Windows 10 | M5-W11, W7, W8 | clean Win10 22H2 VM passes the checklist |

**Calendar, honestly:** M0 ≈ 1 week, M1 ≈ 3–4 weeks (three L items, two of them serialised on `AppWindow`), M2 ≈ 2–3 weeks, M3 ≈ 2 weeks, M4 ≈ 2 weeks of mostly human time. Linux RC1 at roughly week 10–11; Windows 11 RC1 the same week if the signing identity arrived by week 8, otherwise when it does; Windows 10 within days of Windows 11. The M2/M4 Windows estimates rest on the thinnest evidence in the audits (nothing has run on Win11; the last eight commits have not run on Win10), so expect M0-12's first Windows gate run to add a week of fixes somewhere.

---

## 6. Parallelisation

Lanes, by the collision rules in `project/WORK_QUEUE.md`:

| Lane | Owns | Rule |
|---|---|---|
| **A — shell** | `crates/bp-ui/src/{lib,state}.rs`, `ui/app.slint` (`AppWindow`) | **One agent at a time.** M0-01 → M0-07 → M0-03 → M1-01 → M1-04 → M1-05 → M1-07 → M1-06 → M1-18 is the critical path; run it as one long-lived lane |
| **B — shell components** | `dispatch.rs`, `menus.rs`, `editor_view.rs`, `ui/{dialog,about,settings,viewer,menu,status_bar,…}.slint` | One agent per file; a component that needs a new `AppWindow` property hands a 20-line request to Lane A rather than editing `app.slint` |
| **C — libraries** | one crate per agent: `bp-files`, `bp-config`, `bp-data`, `bp-clipboard`, `bp-history`, `bp-buffer`, `bp-search`, `bp-core`, `bp-crypto`, `bp-integrity`, `bp-platform`, `bp-platform-windows` | Fully parallel with everything; each item's library half lands first, its wiring half queues on A or B |
| **D — binary** | `apps/bachelorpad/{main.rs,build.rs,app.manifest}` | One agent |
| **E — release engineering** | `scripts/`, `packaging/`, `.cargo/`, `deny.toml`, `.github/`, workspace `Cargo.toml` profile/lints | One agent; workspace `Cargo.toml` edits are announced because Lane C items also touch `[workspace.dependencies]` |
| **F — records** | `README`, `ROADMAP`, `WORK_QUEUE`, `NEXT_SESSION`, `DECISIONS`, `project/tasks`, ADRs, `docs/` | Serialised. Two sweeps (M1-22, M2-13) by one agent each; between sweeps whoever finishes an item updates only the rows it changed |
| **Human** | decisions, purchases, artwork approval, Windows gate runs, VM testing, RC checklist | Never on the critical path if decisions 1–7 are answered in week 1 and the signing application is filed |

Serialised by nature: every `AppWindow` change; the two records sweeps; version bump and tagging; any workspace `Cargo.toml` edit. Everything else can run four-wide: typically A + two of C + one of E/D.

The gate is the merge rule: an item is done when `Invoke-LocalCI.ps1 -Native` (Linux) is green and, at milestone ends, the Windows gate too. Between Windows runs, Windows-only changes (M2-02, M2-03, M2-04, M1-10's cp1252 on Windows) carry an explicit "not yet run on Windows" line in their commit message, as the sixth session did.

---

## 7. Risk register

| # | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R1 | **Slint 1.17.1 limits**: no IME in the custom view, no file drop, no HTML view, no fontconfig aliasing | Certain | Medium | Ship the `TextInput` view; `--editor-view` experimental; drop and preview deferred; font bundled. Re-evaluate on the next Slint minor |
| R2 | **Licensing**: Slint royalty-free obligations unmet, no `LICENSE-*`, no notices, no legal copyright holder | Certain today | High (cannot distribute) | Decision 2 in week 1; M1-02, M1-14; `cargo deny` in the gate |
| R3 | **Signing**: cost, identity validation lead time, Smart App Control on Win11 | High | High for Windows RC1 | File the application in week 1; Linux RC1 does not wait; keep the identity stable across releases for SmartScreen reputation |
| R4 | **Windows has not run the code**: eight commits plus all of M0/M1 land before the next Windows gate | High | Medium | M0-12 first; Windows gate at every milestone end; a Windows VM the owner can run the gate on in under an hour |
| R5 | **Worker-thread refactor breaks save semantics** (dirty flag, generation, journal ordering) | Medium | High (data) | Generation counter, `a_save_that_finishes_after_more_typing_keeps_the_document_dirty`, journal only discarded on a verified save; keep `atomic_write` itself untouched |
| R6 | **`AppWindow` serialisation** makes M1 a queue behind one agent | High | Schedule | Order Lane A by value (dialogs, then default-view caret, then hiding, then worker, then viewer); push every library half to Lane C early |
| R7 | **Scope creep from locked decisions** (#7, #8) if S1/S2 are not signed off | Medium | High | Section 3's owner's-call table; the release notes carry the deferrals verbatim |
| R8 | **glibc / Wayland / portal variance** on Linux | High | Medium | Debian 12 build container; declared deps; in-app dialogs remove zenity; `SLINT_BACKEND=winit-x11` documented; three-desktop smoke |
| R9 | **Windows 10 support window** (consumer ESU ends 2026-10-13) | Certain | Low–medium | Statement in the notes (decision 21); test on 22H2 anyway because of ESU and LTSC |
| R10 | **Journal shared across instances** (planner finding; not audited) | Medium | High (data) | M0-02 with a failing test first; if the test passes today, the item shrinks to a doc note |
| R11 | **Thin evidence**: IME, HiDPI, Explorer icon behaviour, Win11 perf, Wayland clipboard are unmeasured; M2/M4 estimates are the least reliable | High | Schedule | RC checklist early (M3-05) so the owner can start clicking before M4 |
| R12 | **No hosted CI** (ADR-0016) makes releases depend on one machine | Certain | Medium | Container-pinned Linux build; `manifest.json` records rustc and lock hash; ADR-0031 |
| R13 | **cp1252 heuristics** misdetect UTF-8-looking binary or Latin-1 text | Medium | Low | Only after UTF-8/UTF-16 fail; label shown; fuzz the decoder (M4-05) |
| R14 | **The clipboard callback in the default view** (M0-07) fights Slint's own copy; double-set race on Private+ exclusion | Medium | Low–medium | Window test; document the race; fallback is "history off in the default view" |

---

## 8. The first week: ten items, in order

Each is one agent session. Instructions are written to be pasted as the task line of R011 (or R001 for library-only work) with the repo's usual preamble. Owner tasks for the same week, in parallel: answer decisions 1–7, file the signing application, run the Windows gate (M0-12), start the icon brief.

1. **M0-01 — `bp-ui` (Lane A).** "In `crates/bp-ui/src/lib.rs:383-400` the startup recovery prompt calls `journal.discard_all()` for any answer but `Yes`; on Linux rfd shells out to zenity and returns `Cancel` when it is absent, so recovered work is deleted without a prompt. Extract the decision into a pure `state::recovery_decision(answer: rfd::MessageDialogResult) -> Recovery { Restore, Keep, Discard }` where only an explicit `No` discards; `Keep` shows the status notice 'N unsaved document(s) kept for next launch'. Add unit tests for all three arms including `Cancel`. Do not add dependencies. Gate: `-Native` on Linux."
2. **M0-13 — `scripts/` (Lane E).** "Make `scripts/Invoke-LocalCI.ps1` runnable under `pwsh` on Linux as `-Native`: every stage the Windows host runs (fmt, `check --locked`, clippy `-D warnings`, test, debug build, `--self-check` launch of `target/debug/bachelorpad` under `xvfb-run` when `DISPLAY` is unset, log hygiene, fuzz). Keep `-Linux` (WSL) working. Update `docs/governance/LOCAL_CI.md` and the gate line in `project/WORK_QUEUE.md` and R011. Prove it with the results JSON from this container."
3. **M0-04 — `bp-files` (Lane C).** "`bp_files::load` (`load.rs:86`) reads any file whole. Before reading, `metadata().len()` is classified with `bp_buffer::SizeClass` (add `bp-buffer` as a dependency of `bp-files` and say so); above `HUGE_FILE_BYTES` return a new `LoadError::TooLarge { bytes, limit }`; between `LARGE_FILE_BYTES` and huge, load and return the class in `Loaded` so the shell can warn. Tests use `File::set_len` sparse files. Then the smallest shell change in `state.rs` `open`: show 'Too large to open here (N MiB); the read-only viewer is coming' and 'Large file: some tools may be slow'. No `app.slint` edit."
4. **M0-05 — `bp-data` then `menus.rs` (Lane C → B).** "`serde_json` is built without `preserve_order` (workspace `Cargo.toml:73`), so Data ▸ Format silently reorders keys, and Sort Keys is wired to the same action (`menus.rs:880`, `bp-data/src/lib.rs:109` is literally `json_format`). Enable `preserve_order`, make `json_sort_keys` really sort recursively, give Sort Keys its own action id in a free block, and add a 'comments are not kept' hint to TOML Format like YAML has. Tests: `format_keeps_the_authors_key_order`, `sort_keys_sorts_every_level`, `jsonl_conversion_keeps_order`, and a menus test for the new id."
5. **M0-06 — `bp-config` then `apps/bachelorpad` (Lane C → D).** "`main.rs:61-64` treats every non-dash argument as a file, so `bachelorpad --line 427 server.log` opens a file named `427`. Add `bp_config::cli::{parse, Invocation}` handling `--line N`, `--line=N`, `--readonly`, `--large-file`, `--version`, `--help`, `--` and unknown-flag warnings, with tests for each and for `--` followed by `-weird.txt`. `main.rs` uses it; `RunOptions` gains `line`, `readonly`, `large_file`. `--version`/`--help` print and exit on Linux; on Windows print and exit too, noting in a comment that `AttachConsole` arrives with `bp-platform-windows` (M2-03). Wire `--line` to `go_to_line` under `--editor-view` and leave a `todo`-free note that the default view lands in M1-04."
6. **M0-08 — `menus.rs`, `bp-security` (Lane B + C).** "Fix stale user-visible text: `menus.rs:799` says the recovery journal is 'off until encryption ships (phase 15)' for `Recovery::Encrypted`, but `bp-history` seals it (ADR-0022); reword to 'sealed with the document passphrase'. Remove the stale comments at `menus.rs:795-797`, `bp-security/src/lib.rs:155-157,235`. The Data fallback (`menus.rs:925-931`) must not list Validate/Format as 'not implemented' for a `.txt`; show one row 'nothing for {format} documents'. Help ▸ Keyboard Shortcuts (`dispatch.rs:18-31`) stops advertising Ctrl+D/Alt+Up/Down while they are greyed in the default view. Tests updated; a grep test that no string contains 'phase 15'."
7. **M0-09 + M0-11 — `state.rs`, `bp-config`, repo root (Lane A after item 1, plus housekeeping).** "The recent-files list (`state.rs:423-426`) is written regardless of profile or Privacy Mode; gate it on `policy.metadata != Disabled` and Privacy Mode, and make `recent.rs:108` write atomically (use `bp_files::atomic_write` or a temp+rename in `bp-config`; no new dependency). Test `a_confidential_document_never_enters_the_recent_list`. Same session, no logic: `.gitignore` whitelists `CHANGELOG.md`, `SECURITY.md`, `CONTRIBUTING.md`, `LICENSE-*`; `#![forbid(unsafe_code)]` on `bp-security` and `apps/bachelorpad`; `expect_used` and `indexing_slicing` added as `warn` and the count in the `Cargo.toml` comment corrected."
8. **M0-02 — `bp-history` (Lane C).** "Hypothesis to confirm first with a failing test: two `Journal`s on one directory (as two running instances would be after default-editor registration opens a second file) both see each other's checkpoints as `pending()`, and `discard_all()` in one deletes the other's. If confirmed, add an `owner.lock` (pid + start time, stale if the pid is gone) written on first checkpoint; `pending()` skips held checkpoints and reports them separately; `discard_all()` never removes held ones. Tests for both instances, a stale lock, and a crash mid-lock. Then one shell line in `lib.rs` so the startup prompt ignores held checkpoints. If the hypothesis is false, write the test that proves it and a doc note; no code."
9. **M0-07 — `bp-clipboard` then `bp-ui` (Lane C → A).** "The 1.2 s poll (`lib.rs:975-1003`) records every clipboard change on the machine, including password managers, into the history and the Edit menu preview. Library half: `bp_clipboard::History::push` takes a `Source { InApp, External }` and a `RecordExternal` policy from config (default off). Shell half: the poll runs only when recording external is on; in-app copies reach the history from `editor_view.rs:237-252` and, in the default view, from a new `AppWindow` callback `copied(text)` fired on Ctrl+C/Ctrl+X/Ctrl+Insert beside Slint's own copy; under Private+ our own `set_os_clipboard` uses arboard `SetExtWindows::exclude_from_history().exclude_from_cloud()` and `SetExtLinux::exclude_from_history()`. Tests in `bp-clipboard`; window test `ctrl_c_in_the_default_view_reaches_the_history`. One `app.slint` callback only."
10. **M0-03 — `app.slint` and three component files (Lane A, after item 9).** "Slint resolves `font-family: \"Consolas\"` literally, so Linux renders both editor views in a proportional font and `--editor-view` drifts its caret. Add `bp-ui/fonts/DejaVuSansMono.ttf` (with `LICENSE` beside it), `import` it at the top of `app.slint`, and use `\"DejaVu Sans Mono\"` at `app.slint:612,628`, `editor_surface.slint:64,155,168`, `results_panel.slint:71`. A test greps the `.slint` files for `Consolas`. Verify with `scripts/smoke-linux.sh` (or the session-6 python-xlib driver) that a click at column 40 under `--editor-view` puts the caret at column 40 on Xvfb. Record the licence text for `THIRD-PARTY-NOTICES` (M1-14)."

After these ten, M0-10 (panic hook and log file) and M0-12 (the Windows gate) close M0, and Lane A moves to M1-01, the in-app dialogs, which is the single largest step towards a Linux RC1.
