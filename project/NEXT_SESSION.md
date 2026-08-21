# Next session plan

Rewritten 2026-08-21, at the end of the eighth build session. **Updated
same day, later in the session**, once `bp-storage`'s two menu rows,
`bp-execution`, and Research Report all landed — the numbers and §2 below
now describe the state after `d9bdf30`, not before it. **Updated again at
the start of the ninth session**: §2's defect is fixed and confirmed by
driving the window, and §2 now records what it was and what it cost.

State: **25 crates, 1,846 tests, 0 ignored**, green on Windows and Linux
across all ten gate stages. 102 commits, all on
`feat/phase-01-foundation-ui`; all pushed.

---

## The bottleneck has moved, and this is the first session it has not been code

Four sessions ago the gap was that capabilities did not exist. Three sessions
ago they existed and were unreachable. Two sessions ago the record disagreed
with the product. Last session the queue was gated on questions only a human
could answer.

**None of those is the gap now.** All nine decisions from 2026-08-20/21 are
answered, and every one of them that had scope to build is built: `bp-storage`'s
Organize menu, `bp-execution`, and Research Report all shipped the same day
they were decided. Sign Document was the last row in the product greyed for a
missing *decision* rather than a missing prerequisite, and it acts.

What is left divides cleanly into four kinds, and only one of them is
ordinary coding. **Three of the four are now closed**, all in the ninth
session, and what is left of the list is the one nobody can delegate:

| | What | Who can do it |
| --- | --- | --- |
| **1** | Drive the window and write down what is wrong | **Only a person.** No agent, no test |
| ~~**2**~~ | ~~Fix Sign Document's passphrase-focus leak~~ | **Done**, and watched working rather than read — see §2 |
| ~~**3**~~ | ~~Wire `bp-execution` into a notebook view~~; grow Research past its first insight | **Notebook is done** ([ADR-0043](../docs/decisions/ADR-0043.md)) — and it wanted a *purpose*, not a view. Research is still open — see §3 |
| ~~**4**~~ | ~~Find over a huge document~~ | **Done** ([ADR-0042](../docs/decisions/ADR-0042.md)), and watched working against a 213.5 MiB log |

That ordering is the plan. It is deliberately not the order of difficulty.

---

## 1. Drive the window — the top of the list, and it has been for four sessions

The argument for this is no longer theoretical. **Four manual passes have
happened and each found a defect no test could**, all in the same seam:

- the first found a menu bar where twelve of fourteen menus swallowed clicks,
  and Save As defaulting to the process working directory — which wrote real
  documents into a git checkout;
- the second found a tab drawn as a single glyph while every column in
  `bp-editor` was computed as though it reached the next tab stop, so the
  caret on any tab-bearing line sat where the character was not, and the error
  grew with every tab further along;
- the third found that typing a query into Find edits the document. It was
  the worst of the three at the time, and it is fixed — see "What is no
  longer on this list" below.
- **the fourth, on 2026-08-21, later in the session: Security ▸ Sign
  Document's passphrase bar opens but never takes keyboard focus, so typing
  a passphrase immediately after clicking the row types it in plain text
  into the document instead.** Worse than Find's version of this bug, because
  the leaked text is a passphrase, and it lands in a document that can then
  be saved with the passphrase sitting in it in the clear. **Fixed at the
  start of the ninth session, and the fix was watched working** — see §2
  below, which now records the mechanism rather than a hypothesis.

**None was findable from a test in this repository**, because all four live
in what a toolkit does with what it is handed. 1,812 tests do not find the
fourth either: `PassphraseBar`'s own Slint code, and the Rust call site that
opens it, both look correct in isolation.

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
| ~~**Resize the window**~~ | **Checked 2026-08-21, later in the session.** Whether the row count follows, or a blank strip appears — no gap, in the editor's default surface, in `EditorSurface` (`--editor-view`), and in the huge-file viewer, up to near-full-screen height. **One false alarm on the way, worth knowing**: driving it via `SetWindowPos` past the physical monitor's bounds makes the render surface stop growing at the screen edge and looks exactly like a resize-refetch defect. It is not one — re-tested fully on-screen and it is clean. See `resize-testing-must-stay-onscreen` in the agent's memory if this gets re-driven by script again |
| ~~**Open something enormous**~~ | **Checked the same pass.** The 192 MiB fixture opens correctly: gutter numbers the document (not the screen), the status bar reads `Huge file (192.0 MiB)` and the correct `Ln` range, content is exactly what was written. Save-state greying not separately re-checked this pass |
| ~~**Scroll the wheel**~~ | **Checked 2026-08-21, later in the session, once the session unlocked.** Direction correct both ways, in the default surface's editor and in the huge-file viewer, which takes a different path through `scroll_by_rows`. One false-alarm status-bar staleness on the way, resolved by waiting slightly longer before the screenshot — not a real defect |
| **Sign a document** | **Attempted 2026-08-21 — found a real defect before reaching the signature itself, and that defect is now fixed (§2).** The passphrase bar takes the caret: typing straight after the menu row lands six dots in the field with the document untouched, and the confirm step keeps it too. **The signature itself is still unclicked**, deliberately: completing the ceremony writes a real Ed25519 key into `%LOCALAPPDATA%` and a `.sig` beside the document, which is a person's call rather than a script's. Everything before that point is now known to work |
| ~~Drag to select~~ | **Checked the same pass.** Correct in `EditorSurface` (`--editor-view`) — the selection rectangle landed exactly where the drag went, status bar read the right `Ln`/`Col` at the end. Not re-checked in the default `TextInput` surface, which uses Slint's own built-in selection rather than this codebase's drag logic |
| ~~Arrow down through a short line and back up~~ | **Checked the same pass.** Goal column survives correctly: end of a 59-character line (Col 60) → clamped to a 5-character line (Col 6) → clamped again to a 52-character line, shorter than the goal (Col 53) → back up two lines restores exactly Col 60 |
| **Drive Find on a document that does not fit on one screen** | **Added 2026-08-21, because this is how the biggest defect of the session hid.** Find reported `1 of 1` for a match a hundred lines below the fold and left the reader at line 1, and four manual passes missed it because every fixture anyone had used was short enough to see whole. Fixed and watched working — but the lesson outlives the fix: **a fixture that fits on one screen cannot test anything about scrolling**, and most of this product's readouts are about where you are |
| **File ▸ Set as Default Editor** | **The only row that changes state outside this application.** It reports before it offers and writes nothing on Windows without a Save dialog first — so it wants *reading* rather than clicking through. **Deliberately not attempted this pass**: it changes OS-level file associations, which is a different category of risk from everything else on this list and wants a person's explicit go-ahead before a script clicks it, not just an unlocked session |

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
- **`SetWindowPos` works while the session is locked; synthetic keyboard and
  mouse input does not.** `GetForegroundWindow()` returning `0` is the
  diagnostic signal for a locked session. Resize and window-management
  checks can proceed regardless; scroll, drag, typed passphrases and menu
  clicks cannot, and fail silently rather than with a clear error —
  `SendKeys` returns the same "the operation completed successfully" message
  whether it worked or the session was locked the whole time. Confirm
  `GetForegroundWindow()` is non-zero before trusting a synthetic-input
  result, and re-check it after, since the session can lock again mid-pass.

---

## 2. Fixed: Sign Document's passphrase leaked into the document

**Found 2026-08-21, the first time anyone drove Sign Document from the menu
rather than calling the underlying function directly. Fixed at the start of
the ninth session, and the fix was watched working rather than read.**

`Security ▸ Sign Document` opened the passphrase bar with the right prompt
and then let the editor keep the caret, so typing the passphrase -- the
obvious thing to do the moment a prompt appears -- put every character into
the open document in plain text, and marked it dirty. A user who did not
notice and saved had written their own signing passphrase into the file they
meant to sign.

**Reproduced against the unfixed binary while confirming the fix**, so the
before-and-after is a measured pair rather than a memory: `secret`, typed at
1.6 s per keystroke with the passphrase bar reading "New signing key
passphrase", turned `alpha beta gamma` into `siecalpha beta gamma`, left the
field empty and the tab dirty. After the fix, the same sequence puts six dots
in the field and leaves the document untouched.

### It was two focus-stealers, and neither is in the code that looked wrong

The previous session ruled out `PassphraseBar`, its `focus-input()`, the
`SIGN_DOCUMENT` arm and `refresh()` one at a time and was right about every
one of them. What it had not read is that **both things that take the caret
back run after the callback that asks for it**:

1. **`dispatch` in `ui/app.slint` ends every menu action with
   `root.focus-editor()`.** It is the last line of the function that
   `security-menu`'s `chosen` forwards to, it runs immediately after
   `menu-action` returns, and it is *right* for every row that is not opening
   a bar.
2. **Closing a `MenuPopup` restores the focus the popup took when it opened**,
   and this one was the actual surprise. In `i-slint-core-1.17.1`,
   `WindowInner::show_popup` calls `take_focus_item` and stores the result in
   `focus_item_in_parent`; `process_mouse_input` decides *before* dispatching
   the click which popup to close, dispatches the click (running the row's
   callback, and everything this repository does inside it), and only then
   calls `close_popup` -- whose `close_popup_impl` hands the caret back to
   `focus_item_in_parent`. **Read in the toolkit's source, not inferred from
   the symptom**, and it belongs in the same "checked rather than assumed"
   category as the IME-composition and drag-and-drop findings. There is no
   API to opt a popup out of it, which rules out direction (2) of the two the
   previous session proposed.

### The fix is direction (1): one tick of patience

`focus-find`, `focus-goto` and `focus-passphrase` no longer call into their
bar. Each records *which* bar wants the caret in a private `pending-focus`
property, and a 1 ms `Timer` -- whose `running` is bound to that property, so
clearing the request is what stops it -- hands the caret over on the next
turn of the event loop, once both stealers have had theirs. All three go
through the same mechanism deliberately: which route opened a bar is not
knowable from inside it, and `Ctrl+G` and `Edit ▸ Go to Line` must not
behave differently.

**`Edit ▸ Go to Line` had the identical defect**, and that was confirmed
rather than assumed: against the same unfixed binary, `9` typed straight
after the menu row landed in the document as `third line of text9`. The two
encrypted-document paths (`Security ▸ Encrypt Document`, and opening an
encrypted file) reach `focus-passphrase` the same way and were exposed to the
same race; neither was driven, because both put a native `rfd` dialog in
front of the bar and a native dialog swallows synthetic input.

### What was checked, and with what

Everything below was driven from PowerShell against a release build --
`PrintWindow` for screenshots, `SendKeys` at 1.6 s per keystroke.

| Check | Result |
| --- | --- |
| Sign Document, default surface, unfixed binary | Passphrase into the document. **The defect** |
| Sign Document, default surface, fixed | Six dots in the field, document untouched, tab clean |
| Its confirm step ("Confirm key passphrase") | Keeps the caret too -- the re-ask path defers the same way |
| Go to Line, `--editor-view`, unfixed | `9` into the document. **The same defect** |
| Go to Line, `--editor-view`, fixed | `3` into the box; Go moved the view to line 3 |
| Ctrl+F Find, default surface, fixed | `second` fully in the box, document untouched. No regression |
| Ctrl+F Find, `--editor-view`, fixed | Two clean runs, `second` and `third` |

### Two things found on the way, neither fixed, both real

- **After Go to Line, the status bar kept reporting the old `Ln`/`Col`.**
  Fixed. It was never only Go to Line: Find Next and Previous, opening a
  cross-file result, and stepping a scan all jump and then draw the surface
  directly, and `cursor_label` was only ever set in `refresh`. Nothing
  corrected it either -- both background pollers refresh only when *they*
  found a change, so a stale readout stayed stale. `sync_cursor_label` now
  runs at the end of `draw_editor_view`, which is the one function all of
  them go through: drawing the surface and saying where it is are one act.
- **`--editor-view`'s find box dropping keystrokes was the harness, and this
  is the third false result the same cause has produced.** `SendKeys`
  delivers to whatever window has the foreground *at that moment*, and
  `SetForegroundWindow` fails silently. Sending each key with the foreground
  checked on both sides of it landed 34 of 34 across five runs -- and the
  artefact reproduces on demand: minimise the window mid-sequence and the
  characters sent during the gap vanish, with the document untouched, which
  is exactly the signature that had been written down as a defect. **Settled
  by reproducing the cause rather than by failing to reproduce the symptom**,
  which is the only way round that distinguishes a fixed defect from a quiet
  one.

### And one thing about driving the window that cost two false results

**`SetForegroundWindow` fails silently, and `GetForegroundWindow() != 0` does
not mean your window has the focus.** Two runs in this session produced
"the passphrase went nowhere" and "the menu did not open", both of which were
the input going to a different window entirely. The existing note says a
locked session reads as `GetForegroundWindow() == 0`; the missing half is
that an *unlocked* session can still refuse the foreground, and every
keystroke is then lost without an error. Compare the handle against the
target window before trusting any synthetic-input result, tap ALT to release
Windows' foreground lock before asking for it, and expect the first click
after a window gains the foreground to be swallowed by activation.

---

## 3. Three decisions, answered 2026-08-21 — two now built in full, one has an engine and no view

All three were the shape "what does the product *do* here", and none of them
were implementation questions.

### `bp-storage` — done (phase 9)

**Built in full.** ADR-0037 -- a related-notes panel and duplicate
detection, both reading the schema ADR-0019 shipped, landed as `b0c152a`.
Related Notes opens a collapsible panel, the same pattern `results_panel.slint`
established for cross-file search. Duplicate Detection is both automatic
(a non-blocking save-time notice) and on-demand (the `Organize` menu row).
**One latent defect found and fixed the same day** (`4b549a5`): the store's
directory was never created before `Store::open`, so on a fresh machine
Organize silently never persisted anything -- no test caught it, because the
test path deliberately uses `Store::in_memory()`. Confirmed fixed by direct
filesystem inspection after rebuild. A similarity measure beyond exact tag
overlap is real future work, not a gap in what shipped.

### `bp-notebook` and `bp-execution` — reachable, and it was never a view that was missing (phase 12)

**Done 2026-08-21** ([ADR-0043](../docs/decisions/ADR-0043.md), `ce633eb`
and `b8acd58`). Open an `.ipynb` and the Run menu lists its runnable cells;
choosing one runs it and shows `stdout`, `stderr`, the exit code and the
duration in a panel, with a Stop that keeps what the cell had already
printed.

**What had actually been blocking it was the question "what is this mode
for", and the answer decided the size of the work.** ADR-0038's
no-persistent-session model is fatal to data exploration and free to a
*literate document* -- a runbook whose examples are verified rather than
asserted -- because every cell in one is self-contained by intent. Pick that
purpose and a cell view is not needed at all: the notebook stays its own
JSON in the ordinary editor, which `bp-notebook` had anticipated all along
(`raw_json_view` exists because the JSON "is the thing the user might want
to hand-edit"), and the whole surface is a menu.

Three sessions of calling this "the largest unscoped item" were three
sessions of sizing a cell-sequence view. The item was never that large; it
was undecided, and undecided reads like large.

`bp-execution` gained a resumable run for it -- `Run::start`/`poll`/`stop`,
with the blocking `run` now written over them -- because a window cannot
block for a cell's timeout. Same shape as ADR-0042's scan, and the same
reason.

### What the old note said, kept because the estimate is the lesson

**ADR-0038's shape, and ADR-0040's engine, are both built.** `bp-execution`
(`8c2dfab`) runs a cell as a fresh subprocess -- no persistent kernel
session, no "run all", matching ADR-0038 exactly -- for three languages
(Python, PowerShell, Shell; Rust and SQL explicitly deferred, named honestly
rather than attempted at a lower standard). A `UserGesture` is required to
start a process, output is capped, a wall-clock timeout kills a hung cell.
Tested standalone, the same way `bp-notebook` was before anything called it.

**What is missing is not an engine decision any more -- it is a whole UI.**
Nothing in `bp-ui` calls `bp-execution`. There is no cell-sequence view, no
Run Cell menu row, no rendering of a cell's output back into a notebook.
Wiring `bp-notebook`'s `RunRequest` through `bp-execution` into a real view
is real, unscoped work -- likely the largest single remaining item in the
product, and nothing here has scoped it yet.

### `bp-research` — a first insight is built; the rest of the mode is not (phase 13)

**ADR-0039's foundation, and ADR-0041's data model, are both settled and
built.** Research Report (`d9bdf30`) reads `bp-storage` directly -- no new
schema, no new store -- through a synthesis layer of four plain, inspectable
aggregate queries: dominant themes, stale clusters, under-connected
documents, consolidation candidates. `bp-research`'s existing bibliography
types (BibTeX, CSL, DOI/arXiv identifiers) stay exactly what they are;
this is not built on top of them.

Offline throughout: nothing here reverses ADR-0006's no-network guarantee,
and if synthesis ever implies *generating* prose rather than arranging
gathered material, that reaches for ADR-0033's OpenAI policy instead, not
this one.

**What is left is a fifth-and-beyond insight, not a first one.** Research
Report is a single report-dialog menu row; the rest of
`docs/product/MENU_MAP.md`'s Research section (citation metadata, evidence,
findings, methods, datasets) is still `planned()`. ADR-0041 itself says the
four shapes it named are a starting set, not an exhaustive design.

---

## 4. Find over a document served from disk — done, and it found two defects

**Built in the ninth session, engine and shell** ([ADR-0042](../docs/decisions/ADR-0042.md),
`011ca6d` and `7717a71`), and watched working rather than read: against a
213.5 MiB log of 4.8 million lines, `searching 86%` while the scan ran and
`1 of 1` at line 4,800,001 when it finished, with the match highlighted on
its own characters.

ADR-0030's two hard parts are both answered, and neither needed what it
looked like it needed. **A hit does not have to *become* a scroll position**
-- it is one, if a hit is reported as a document line and a column in it,
which is the only thing a view with no caret can act on anyway.
**Interruptible did not need a thread**: `advance(windows)` does a few
megabytes and returns, so cancelling is the absence of the next call, and a
scan cannot outlive its document because it lives on the `HugeView`.

### Two defects it turned up, both older than it

**One is fixed, in the same commit, because without it the feature could not
be reached at all: a huge document had no keyboard shortcuts.** Not Ctrl+F --
the only route to Find here, since there is no Find menu row -- and not
Ctrl+S, Ctrl+O or any other. `scroll_for_command` returned `true`,
"handled", for every key it was given, including the `Command::Ignore` that
`bp_editor::keys::command_for` produces for exactly the keys its own comment
calls out as the window's. `EditorSurface`'s `FocusScope` accepts whatever
that reports as handled, and an accepted key never bubbles to the
`KeyBinding` waiting for it. It had been that way since the viewer shipped.

**The other was fixed straight after, and the reason it looked harder than
it was is worth keeping: a document drawn in the custom surface had no
keyboard focus until you clicked in it.** `AppWindow`'s
`forward-focus: editor` names the `TextInput`, which is invisible whenever
`use-editor-view` is true -- so a huge document, *and every document under
`--editor-view`*, opened with nothing holding the keyboard.

It was written up here as wanting "a decision rather than a patch", on the
grounds that the candidate fixes differed in whether they survived the window
being re-activated. **That premise was wrong, and reading the toolkit is what
showed it**: `forward-focus` is resolved at *compile* time by
`i-slint-compiler`'s `focus_handling` pass into the component's init code, so
it fires once, is one fixed element, and no binding will ever move it. There
was no re-activation behaviour to weigh -- only a hand-over to write.
`focus-editor-soon()` is it, guarded so that a surface change cannot take the
caret out of an open find box, which was the only real risk in the change and
is confirmed by driving the window.

---

## 5. What an agent can take next

`project/WORK_QUEUE.md` has the full list. In the order they are worth doing:

| Item | Why it is first |
| --- | --- |
| ~~Focus in the custom surface~~ | **Done, same session.** It turned out to want evidence rather than a decision: `forward-focus` is resolved at *compile* time by `i-slint-compiler`'s `focus_handling` pass into the component's init code, so it is one fixed element and no binding can move it, and there is no re-activation subtlety to weigh. `focus-editor-soon()` hands the caret over instead, guarded so a tab switch cannot take it out of an open find box |
| ~~The status bar goes stale after a jump~~ | **Done, same session**, and it was all five jump paths rather than the one it was noticed in. `sync_cursor_label` in `draw_editor_view` |
| ~~`--editor-view`'s find box drops keystrokes~~ | **Withdrawn: it was the harness.** `SendKeys` goes to whatever holds the foreground, and losing it is silent. Reproduced on demand by minimising the window mid-sequence |
| **Research mode's synthesis half** | Its *citation* half is built (ADR-0044). What is left is more insight shapes over `bp-storage`: research question, evidence, findings, methods, datasets, all still `planned()`. ADR-0041 says its four shapes were a starting set |
| ~~Research mode past its first insight~~ | Partly done. Research Report is one report-dialog row; the rest of `docs/product/MENU_MAP.md`'s Research section (citation metadata, evidence, findings, methods, datasets) is still `planned()`, and ADR-0041 says its four shapes were a starting set |
| ~~Markdown fences that run~~ | **Done** (ADR-0045). Was: the same purpose wants ` ```python ` blocks in an ordinary `.md` to be runnable. Needs a fence parser and a decision about what names a block |
| ~~A notebook view~~ | **Not needed for the purpose chosen** — see §3. A cell-sequence view remains a real option and is now a thing to design against evidence rather than guess at |

---

## What is no longer on this list

Recorded so nobody re-derives them.

- **Find, over a document served from disk** — shipped (ADR-0042), and §4
  above says what it cost.
- **A signing passphrase typing itself into the document** — shipped
  (`34f02e4`). Two focus-stealers ran after the callback that asked for the
  caret, and one of them was a `MenuPopup` restoring focus on close, which is
  Slint's own behaviour with no way to opt out. A bar now asks for the caret
  a tick late, through `focus-timer`.

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
