# Next session plan

Rewritten 2026-08-21, at the end of the eighth build session.

State: **24 crates, 1,758 tests, 0 ignored**, green on Windows and Linux
across all ten gate stages. 85 commits, all on
`feat/phase-01-foundation-ui`; all pushed.

---

## The bottleneck has moved, and this is the first session it has not been code

Four sessions ago the gap was that capabilities did not exist. Three sessions
ago they existed and were unreachable. Two sessions ago the record disagreed
with the product. Last session the queue was gated on questions only a human
could answer.

**None of those is the gap now.** Four decisions were answered on 2026-08-20
and three of them are built. Phase 4 is complete. Sign Document was the last
row in the product greyed for a missing *decision* rather than a missing
prerequisite, and it acts.

What is left divides cleanly into three kinds, and only one of them is
ordinary coding:

| | What | Who can do it |
| --- | --- | --- |
| **1** | Drive the window and write down what is wrong | **Only a person.** No agent, no test |
| **2** | Decide what `bp-storage`, notebooks and research *are* | **Only Daniel.** Product design, not implementation |
| **3** | Find over a huge document | An agent, once 1 and 2 are not blocking |

That ordering is the plan. It is deliberately not the order of difficulty.

---

## 1. Drive the window — the top of the list, and it has been for four sessions

The argument for this is no longer theoretical. **Three manual passes have
happened and each found a defect no test could**, all in the same seam:

- the first found a menu bar where twelve of fourteen menus swallowed clicks,
  and Save As defaulting to the process working directory — which wrote real
  documents into a git checkout;
- the second found a tab drawn as a single glyph while every column in
  `bp-editor` was computed as though it reached the next tab stop, so the
  caret on any tab-bearing line sat where the character was not, and the error
  grew with every tab further along;
- **the third, on 2026-08-21, found that typing a query into Find edits the
  document.** It was the worst of the three, and it is fixed — see "What is no
  longer on this list" below.

**None was findable from a test in this repository**, because all three lived
in what a toolkit does with what it is handed. 1,754 tests do not find the
third either: `AppState::find` is correct and every test of it passes.

```powershell
cargo run --release                    # the default surface
cargo run --release -- --editor-view   # the custom one
```

### The checklist, in priority order

The first two moved up when the huge-file viewer shipped, because scrolling is
the whole of what a viewer does. A wheel that goes the wrong way is not a
papercut there — it is the feature being broken.

| Check | Why it matters now |
| --- | --- |
| **Scroll the wheel** | Direction, and whether it snaps back to the caret. **In an editor *and* in the viewer** — they share `EditorSurface` but take different paths through `scroll_by_rows` |
| **Resize the window** | Whether the row count follows, or a blank strip appears. The viewer refetches on a size change, so this is also whether it refetches the right number |
| **Open something enormous** | Rendered and measured on 2026-08-20; nobody has *read* one. Check the gutter numbers the document rather than the screen, that the status bar reads `Ln 4812006-4812035`, and that Save / Save As / Save a Copy / Reload are greyed with their reason |
| **Sign a document** | Brand new and never clicked. Two passphrase entries the first time, one after; the message should carry your public key. Then Verify Signature on the same document, which should say ✓ |
| Drag to select | Whether the selection rectangles land where the drag went |
| Arrow down through a short line and back up | Whether the goal column survives |
| **File ▸ Set as Default Editor** | **The only row that changes state outside this application.** It reports before it offers and writes nothing on Windows without a Save dialog first — so it wants *reading* rather than clicking through |

**There is a written-out version of this checklist**, with every command, the
exact strings to expect and space to write findings, at
`~\Downloads\BachelorPad_Manual_Pass_21AUG2026.md`. It also carries Part 2's
three decisions with their options laid out.

### A 192 MiB fixture costs a few hundred bytes

Write real lines, then extend with `set_len`: both NTFS and ext4 record the
length without writing the bytes. `a_huge_file` in `crates/bp-ui/src/state.rs`
is the helper the tests use, and it is four lines.

```powershell
$p = "$env:TEMP\enormous.log"
$sb = [System.Text.StringBuilder]::new()
1..5000 | ForEach-Object { [void]$sb.AppendLine("line $_`tcolumn two") }
[System.IO.File]::WriteAllText($p, $sb.ToString())
$fs = [System.IO.File]::Open($p, 'Open', 'Write'); $fs.SetLength(201326593); $fs.Close()
```

### Three things learned about doing this

- **Do not drive the window while somebody else is.** Two of three findings
  from the last pass evaporated on inspection: a fixture whose line labels were
  hand-numbered and had drifted from the gutter by one (the gutter was right),
  and an apparent stray deletion that was the other person typing. Generate a
  fixture rather than numbering it by hand.
- **`AKIAIOSFODNN7EXAMPLE` is not a finding**, and a scan that ignores it is
  working. `bp-secrets` treats anything containing `example` as a placeholder,
  deliberately, with a test pinning that exact string. A fixture meant to
  exercise the scan needs a key that looks random — `AKIA5MZ7QK2XPLRW4VTD`
  does.
- **The software renderer has no rendering notifier**, so `--measure-exit`
  returns before the first frame. `--renderer=platform` is what actually
  paints one, and its timings are not comparable to ADR-0017's.

---

## 2. Three decisions, answered 2026-08-21 — now design work rather than open questions

All three were the shape "what does the product *do* here", and none of them
were implementation questions. They are answered now; what is left for each is
building toward a description that exists, not guessing at one.

### `bp-storage` — 721 lines, 21 tests, called by nothing (phase 9)

**Answered in full.** ADR-0037 -- a related-notes panel and duplicate
detection, both reading the schema ADR-0019 already shipped (documents, tags,
the join between them). Neither needs a new schema to start; a similarity
measure beyond exact tag overlap is later work, not a blocker on shipping the
panel with what already exists.

**Presentation is settled too.** Related Notes opens a collapsible panel --
the same pattern `results_panel.slint` already established for cross-file
search results. Duplicate Detection is both automatic (a non-blocking
save-time notice) and on-demand (the `Organize` menu row runs the same check
any time). Both rows already exist, unwired, in `docs/product/MENU_MAP.md`.
This is now `project/WORK_QUEUE.md`'s ready tier, not its blocked one -- a new
panel component and two callbacks, not a blank page.

### `bp-notebook` — 3,627 lines, 59 tests, reached only by the fuzz harness (phase 12)

**Answered: a lightweight, one-cell-at-a-time utility.** ADR-0038 -- run a
cell on request, no persistent kernel session, no "run all". `bp-notebook`
refuses to run a cell without a `UserGesture` that no parsed file can produce,
which is the security half done and the reason the narrower shape fits: a run
that starts and stops reads naturally against a gesture-per-cell rule; a
persistent session would not.

**What is missing is not a view decision any more -- it is an engine.**
`bp-notebook` is only the format half; the crate that would actually run a
cell, named `bp-execution` in its own module docs, does not exist in this
workspace. The view can be built and tested against `bp-notebook` alone
before that engine exists, so the two are parallel-safe once both are
scoped -- but the engine is real, likely substantial, separate work, and
nothing here has scoped it yet.

### `bp-research` — 5,208 lines, 161 tests, called by nothing (phase 13)

**Answered, fully.** ADR-0039 (`Accepted`, foundation settled the same
session as D12) -- research mode synthesises data already present in
documents or the store into insights, options and recommendations that
inform a design, a feature or a function, **on a data model of its own,
separate from `bp-research`.** `bp-research`'s existing bibliography types
(BibTeX, CSL, DOI/arXiv identifiers) stay exactly what they are; phase 13 is
not built on top of them.

Offline throughout: nothing here reverses ADR-0006's no-network guarantee,
and if synthesis ever implies *generating* prose rather than arranging
gathered material, that reaches for ADR-0033's OpenAI policy instead, not
this one.

What is left is real design work, not a decision waiting on Daniel: what
"gathered data" means concretely (documents already open? a folder pointed
at? `bp-storage`'s related-notes graph, once ADR-0037 is built?), and what an
"insight" or "recommendation" looks like on screen. Nothing is scaffolded
yet -- naming the crate or module set is an implementation choice at the
point this is actually taken.

---

## 3. What an agent can take now

One item, real, not urgent. The other one — splitting `state.rs` — is done.

### Find, over a document served from disk

The last piece of the large-file story. `bp_buffer::LargeFile::windows`
exists for exactly this — overlapping text windows, with a fresh file handle
so a background search does not disturb the scrolling viewport — and
`bp-search` decides what a match is.

It is a feature rather than wiring, and the two hard parts are named in
ADR-0030: a match found in a window has to become a scroll position, and a
search over 2 GB has to be interruptible. Neither is solved.

---

## What is no longer on this list

Recorded so nobody re-derives them.

- **The viewer for a document the rope does not hold** — shipped (ADR-0030). A
  192 MiB log costs 0.8 MiB more than a small note, measured.
- **Sign Document** — shipped (ADR-0031). The key is sealed rather than
  permission-protected, which closes the asymmetry ADR-0026 measured.
- **The application id** — reverse-DNS and no longer the same constant as
  `APP_DIR` (ADR-0032).
- **`state.rs` at 6,282 lines** — split four ways, into `state/security.rs`,
  `state/data.rs`, `state/encryption.rs` and `state/find.rs`. What made it
  possible is worth carrying forward: the new modules are *children* of
  `state`, and a child can see its parent's private items, so `editors`,
  `stamps`, `gutter_lines` and `match_index` stayed private instead of being
  widened to `pub(crate)` for the whole crate to reach.
- **Find edited the document while you typed the query.** `on_find_changed`
  (`lib.rs`) fired `dispatch::select` on every keystroke, which ended in
  `editor.focus()` — right for Find Next, wrong for a live preview, so the
  character after the one that found a match landed in the document instead of
  the box. Fixed by splitting the focus-stealing "jump to it" path
  (`dispatch::select` / `select-range`) from a new focus-preserving "preview
  it" path (`dispatch::preview_match` / `preview-range`), covering both the
  `TextInput` and `--editor-view` branches. Confirmed by driving the window:
  four clean full-speed and slow-paced repro runs against the exact sequence
  that broke it, both before and after the fix. One further run corrupted a
  line anyway, immediately after the app had been killed via Task Manager and
  relaunched without clearing the stale recovery journal first; it did not
  reproduce again across three more attempts under the same or harsher timing,
  so it reads as a one-off tied to that unclean relaunch rather than a
  residual defect -- recorded rather than quietly dropped, since it was not
  run to ground with certainty.
- **Crash recovery restored the text and forgot the encoding and line
  ending.** `AppState::restore` never called `set_line_ending`/`set_encoding`,
  so a recovered document fell back to the platform default — an LF file came
  back reporting CRLF on Windows. Fixed by adding `encoding`/`line_ending` to
  `bp_history::Checkpoint` (`#[serde(default)]`, so journals already on disk
  still deserialize); line ending is re-detected from the text for a
  pre-upgrade checkpoint, encoding honestly falls back to a guess since it
  cannot be recovered from already-decoded text.
- **The two latent defects** — the three files that roamed on Windows, and
  `Profile::is_data` answering a question it could not.
- **Parity, or retire `--editor-view`** — settled and not in the view's
  favour. Word wrap is done; input-method composition cannot be done at all on
  Slint 1.17.1. The flag stays opt-in and now means precisely "which surface
  *edits* a document the rope holds".
- **Make the record match the product** — done twice now, and the lesson kept:
  every stale claim went stale the same way, a fix landing without the record
  moving.
- **Put `fuzz/` inside the gate** — done, on both legs, verified to fail before
  it was trusted.
- **The seven cross-crate defects** — all fixed. **D6, what a line is** —
  ADR-0029.

---

## Decisions still waiting on a human

`project/DECISIONS_NEEDED.md` is the register. **Down to one row.** D1, D3,
D4, D7, D8, D10 and D12 were all answered on 2026-08-21 -- this table used to
list every one of them and none is still open. What is left:

| # | Question | What it holds up |
| --- | --- | --- |
| D2 | Is `--editor-view`'s two-view fork worth keeping indefinitely | Nothing right now. Its own row says not to re-ask unless the tax becomes visible or Slint ships composition support |

D4's answer was "yes, get an outside review of the `.bpadx` envelope" --
that is not a question waiting on Daniel any more, but it is also not done.
See Housekeeping, below.

---

## Working with agents in this repository

Four sessions of evidence, worth reading before dispatching any:

- **`prompts/rosettas/R011_WIRE_CAPABILITY_INTO_SHELL.md` earns its keep.** An
  agent that read it found and fixed an unbounded dispatch range that would
  have swallowed its own new ids.
- **Give one crate per agent.** Four ran concurrently without collision when
  each owned one crate. The moment two want `bp-ui` or `app.slint`, serialise.
- **Ask for a property, not more examples.** Every defect found by review has
  been found by a property — "moving a line down and back returns the exact
  original bytes" caught two that example-based tests written alongside the
  feature missed entirely.
- **Tell them to report problems outside their scope rather than fix them.**
  Both times an agent fixed something adjacent, it was wrong to: a JSON
  reformatter that duplicated `bp-data` and disagreed with it on key order,
  and the removal of the launch stage's timeout inside a "pure refactor"
  commit.
- **`scripts/Invoke-LocalCI.ps1` changes are their own commit, never a
  passenger.** It is the thing that validates everything else.
- **Review the substance, not the report.** Agents self-reported all green in
  every case where a real defect was present.

### The two traps this repository springs repeatedly

**A function reading the real profile directory means `cargo test` writes
there.** Sprung **three times**: the security history wrote a live log into
`%APPDATA%`, `AppState::new()` built its recovery journal from the real path,
and — during the change that added it — a test run wrote **a real Ed25519
private key** into `%LOCALAPPDATA%`. The fix is a field on `AppState`, not a
function, and for the key a `cfg(test)` redirect on the *function* would not
have been enough: `has_signing_key` decides which question the passphrase bar
asks, and the signing that follows must open the file that answer was about.

> **Ask of anything that resolves a path at startup: what happens when a test
> constructs one of these?**

**Code taking a `Platform` parameter must not let `std::path` answer for it.**
Also sprung three times, by three different functions, and every time the
Windows leg was green while the Linux leg found it. `4390593` recorded a
surviving mutation where `PathBuf::join` stood in for
`bp_platform::paths::join`. The device-name refusal did it again: the
*judgement* was parameterised and the *split* was `Path::file_name()`, which
splits by the platform the binary was compiled for — so on Linux
`C:\Users\me\con.txt` is one file name containing no device, and the Windows
rule passed by never being evaluated. The third was `writable_path`, which
took a platform and then called `std::path::absolute`, so on the Linux leg
`C:\docs\...` is a *relative* path.

`bp_platform::paths::file_name(platform, path)` exists now, with
`Path::file_name`'s exact semantics and the platform made explicit;
`paths::join` is the same fix for the writing direction.

Neither is a gate check, and that was measured rather than assumed: a grep for
host-dependent `std::path` calls in files mentioning `Platform` returns five
hits and all five are legitimate. A stage that cries wolf gets disabled
wholesale, which `docs/governance/LOCAL_CI.md` already says about gates that
are wrong. The enforcement is the primitive being obvious, plus a test named
for the trap in each function that has fallen into it.

> **Ask of any parameterised function: which platform is each line of this
> answering about?** `is_absolute`, `components`, `file_name` and `join` all
> take one. `std::path` never does.

---

## Housekeeping

- **Nine decisions were answered across 2026-08-21** (D1, D3, D4, D7, D8,
  D10, D12, plus notebook mode and research mode's purpose), recorded as
  ADR-0033 through ADR-0039 -- all `Accepted` now; ADR-0039 moved off
  `Proposed` once D12 settled its foundation. **ADR-0035 is built**:
  `KdfParams::validate`'s two constants are lowered, a test pins the
  tightening itself, and the new worst case is measured in release at
  ~5.56 s (was ~75 s). **ADR-0036's shape is settled** -- an iterator
  primitive in `bp-audit`, driven paged or as a background decrypt depending
  on the interaction -- and is now `project/WORK_QUEUE.md`'s ready tier, not
  its blocked one; the code itself is not written. ADR-0033's OpenAI
  integration remains purely a policy: nothing calls it, phase 8 has not
  started.
- **D4: get an outside review of `bp-crypto`'s `.bpadx` envelope before
  trusting real documents to it.** Answered yes; not done. Every primitive is
  vetted RustCrypto, but the envelope composition is this project's own
  design, and that is exactly the kind of format-level mistake an author
  cannot fully catch by re-reading their own work. Blocks nothing today —
  `.bpadx` is already in use for the recovery journal and Sign Document — but
  should happen before phase 15 asks anyone to trust it with something that
  would hurt to lose.
- **All 85 commits live on `feat/phase-01-foundation-ui`, and all are
  pushed.** `main` still has none of them, so a twenty-phase product exists
  in exactly one place under a branch named for phase one. This is the item
  most likely to hurt and least likely to be noticed.
- **Three orphaned files sit in `%APPDATA%\bachelorpad\`**: `recent.toml`,
  `security-history.log`, and an empty `recovery` folder. Left behind
  deliberately when all three moved to `DirKind::State` — a ten-path recent
  list rebuilds itself and a recovery journal is transient — so they are safe
  to delete and nothing will recreate them.
- **`crates/bp-ui/src/state/security.rs` at 2,820 lines** is the largest file
  in `bp-ui` now, and it is listed here for the same reason `state.rs` used to
  be: this is the kind of thing that stops being noticed. It is not the same
  problem — it is one subject, half of it is tests under their own banners,
  and it is roughly the size the whole shell was when the *first* split
  happened rather than twice it. Its seams, if it ever needs them, are
  `bp-secrets`/`bp-redaction` on one side and `bp-crypto`/`bp-integrity` on
  the other: "what is in this document" against "is this document what it
  was".
- **Native Linux timing.** WSLg's compositor makes startup numbers
  unrepresentative, so ADR-0015's evidence remains Windows-only.
- **Scroll smoothness under CPU rasterisation** is the standing revert
  condition on ADR-0017 and needs a capture rig. **The huge-file viewer makes
  this more pressing than the custom editor did**: it redraws a set of `Text`
  elements per scroll *and* reads the disk to fill them.
- **Idle memory drifted from ~20 MB to ~32 MB** in one session after dialogs
  had been opened. Below the 50 MB target, observed three times now, never
  investigated.
- **A clipboard menu row decodes its id against the history at click time**,
  not when the menu was built, and a 1.2 s poll can shift it in between.
  Pre-existing and applies to plain paste too; fixing it means freezing a
  snapshot while a menu is open.
- **`fuzz/corpus/yaml/` breaks editor tooling that scans the workspace.**
  Google's Cloud Code extension crash-loops its YAML language server on
  `billion-laughs.yaml` — a 570-byte file, 21 aborts in one window. **No
  checked-in setting fixes it**; narrowing `cloudcode.yaml.yamlFileMatcher`
  was tried and does not, because the server dies during initialisation.
  Disable the extension for this workspace; nothing here is GCP. Any other
  tool that walks the tree for `*.yaml` is a candidate. Reproduction and
  numbers in `fuzz/README.md`.
- **The panic lints are on** — `unwrap_used`, `panic`, `todo`,
  `unimplemented`, at `warn` in `[workspace.lints.clippy]`, made errors by the
  gate's `-D warnings`. `expect_used` (nine in library code, each with a
  message saying what was assumed), `unreachable` (one) and `indexing_slicing`
  (thirty-six) are **not** on; each is a change to the code rather than to the
  policy and wants its own pass.

---

## How to work in this repository

- `./scripts/Invoke-LocalCI.ps1 -Linux` is the gate. There is no hosted CI
  (ADR-0016) and Actions is disabled at the repository level.
- `./scripts/Install-GitHooks.ps1` once per clone; the hooks run the gate.
- Every commit in this repository has passed the gate. Keep it that way.
- `docs/decisions/` is the register; add an ADR when a decision has
  consequences beyond one file. **Amend it in place when the build changes the
  answer** — ADR-0031 said the signing key goes in `DirKind::State` and it
  goes in `Data`; the amendment says so and why, rather than the sentence
  quietly changing.
- **Do not edit files while the gate is running.** The Linux leg re-runs
  `cargo fmt --check` from scratch, and a mid-run edit fails it for a reason
  that has nothing to do with the change. The fuzz stage's `cargo fmt` reaches
  the main crates through path dependencies, so this applies to it too.
