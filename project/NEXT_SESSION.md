# Next session plan

> **Superseded for scope, 2026-09-10, by [V1_1_1_PLAN.md](V1_1_1_PLAN.md)** --
> a thirty-day plan to ship **v1.1.1 on 2026-10-10**. What is below is still
> the truthful account of where the code stands and is not rewritten; what
> changed is which work comes next and why.
>
> **Three things in this file are now known to be wrong, and each is worth
> more than the correction:**
>
> - **`TextInput` will say where the caret is.** An undocumented internal
>   `out property` on 1.17.1, checked by compiling. Thirteen menu rows are
>   greyed in the build every user runs for a reason that was never true, and
>   [ADR-0018](../docs/decisions/ADR-0018.md)'s measured "tax" may be zero.
> - **The derived repository was not irrelevant to this one.** Section 1a says
>   the fourteenth session "produced no change here" and that nothing here
>   depends on `bachelorpluslite`. True of the product; **false of the
>   decisions.** It is nineteen ADRs ahead, and six of them answer questions
>   this project was still carrying -- including **the icon (P2) and the
>   installer (P5)**, both of which section 3 lists as work.
> - **`specs.md` §22's warm-start target and the memory drift are still
>   unmeasured**, and a release is the deadline that makes both real.
>
> The plan's section 0 is the evidence for all three.


Written 2026-08-24, at the end of the thirteenth build session -- which ran
past midnight, so its three commits are dated 2026-08-23 and this plan is not.

**Updated 2026-08-29, and the plan below is carried forward rather than
rewritten.** The fourteenth session spent itself outside this repository and
took nothing from it, so every item still stands exactly as it was sized
(section 1a). **This repository is unchanged by that session**: same branch,
same HEAD `3589205`, the same five modified files.

State: **25 crates, 1,990 tests on Windows and 1,995 on Linux, 0 ignored**,
green across all eleven gate stages. 56 ADRs. The gate takes ~13 minutes with
both legs, and **pre-push now recognises a full gate that already covers the
content and skips re-running it** -- so a commit made minutes after a green
gate pushes immediately.

> **Phase 20 started and this product now produces a release.**
> [ADR-0054](../docs/decisions/ADR-0054.md) made the command line an
> interface -- `--help` lists thirteen flags, `--version` names the product
> and its licence, `--line=427 server.log` opens a file already at the line,
> and a mistyped flag says so instead of vanishing.
> [ADR-0055](../docs/decisions/ADR-0055.md) deferred signing and refused
> self-signing, and `scripts/New-Release.ps1` builds, stages, archives and
> checksums both targets. Both archives were extracted and run.
>
> [ADR-0056](../docs/decisions/ADR-0056.md) closed the last large unscoped
> question, D16, by finding its answer already in `bp_security`.

State also worth one line: **three ADRs in one day, and two of them closed
questions this file had described for two sessions as large.**

---

## Where this leaves things

**Every menu row in the product either does something or is a readout**, and a
test says so ([ADR-0048](../docs/decisions/ADR-0048.md)). Nothing anywhere
says "not implemented yet".

**The queue emptied on 2026-08-22 and refilled on 2026-08-23**, and the
mechanism is the useful part. For three sessions this file has said the
project keeps reaching a state where "the next piece of work needs a decision
before it can be sized". That was true and it was not the whole truth. The
answer to an empty queue turns out to be: **take the work whose shape is
already known, and it will produce the next question.** ADR-0053 had already
named one such piece — phase 20 was the only *Not started* phase not blocked
on a decision — and starting it produced D15 inside an afternoon, plus four
defects nobody had a reason to look for.

**`WORK_QUEUE.md` has seven ready items and `DECISIONS_NEEDED.md` has none.**
That combination is the healthy one, and it took three sessions of the reverse
to recognise it. The seven: P2 and P5 (what is left of packaging), M1 (a
measurement mis-filed as a decision for several sessions), R1 (a design pass),
and E1–E3 (phase 10, sizeable at last).

---

## Read this first, then start

1. `CLAUDE.md` — the entry point, and the three standing authorities. **You do
   not need to ask to take work, to commit after a green gate, or to delete a
   planned row with an ADR.**
2. `project/DECISIONS_NEEDED.md` — ask its batch in your first message, then
   start work without waiting.
3. This file, for what matters most.

---

## 1a. The fourteenth session produced no change here

**2026-08-29. It made a derived repository and did nothing to this one.**
BachelorPlusLite was cloned out of this repository at `3589205` into
`G:\dev2\bachelorpluslite`, with its own private remote and this
repository kept as a read-only `upstream`. It is a separate product with a
separate plan and **it is deliberately absent from section 3**.

Two things follow for whoever reads this next, and neither is about that
repository:

- **Nothing in section 3 was taken.** E1, M1, P2, P5, R1 and the manual pass
  are all still first in the queue, still decision-free, and still sized as
  they were on 2026-08-24. Start there without re-deriving the order.
- **The rename turned up a lesson this repository will need**, and it is in
  `DECISIONS.md` rather than here: an identity has four layers, and the two
  that break silently are the on-disk ones. It is the reason
  `what_an_earlier_build_wrote.rs` exists, arriving from a new direction.

---

## 1. What the thirteenth session did, so nobody re-derives it

**Three commits, three ADRs, and phase 20 went from *Not started* to mostly
done.** `c922630`, `1d56355`, `3589205` — all on PR #2.

| | |
| --- | --- |
| [ADR-0054](../docs/decisions/ADR-0054.md) | The command line becomes an interface: `--help`, `--version`, `--line=N`, a reported typo, and the licences every manifest already claimed |
| [ADR-0055](../docs/decisions/ADR-0055.md) | Signing deferred, self-signing **refused**; the archive, the checksums and a one-command signing step built anyway. D15 closed |
| [ADR-0056](../docs/decisions/ADR-0056.md) | All three embedding sources as choices — and the axis for them had existed since ADR-0020. D16 closed |

Plus P3, which needed no ADR: `what_an_earlier_build_wrote.rs` holds the four
files this product leaves on a disk still readable by a later build.

### The inventory, which is where it started

**Phase 20 began with an inventory rather than an estimate**, as ADR-0048
recommends, and it found four things in a product 1,972 passing tests deep:

- **The binary could not say its own version.** `--version` printed nothing
  and opened a window; so did `--help`. Twelve flags were accepted and the
  only inventory of them was `main.rs` and the middle of a `match`.
- **A mistyped flag was discarded in silence** -- by the crate that already
  reports the identical mistake in `config.toml`. One end of one precedence
  chain told the user and the other swallowed it, and nobody had chosen that.
- **Every manifest claimed `MIT OR Apache-2.0` and neither licence file
  existed.** Trap 3 in its purest form: nothing in a build reads the string,
  so nothing would have failed until a release shipped a claim with nothing
  behind it.
- **`specs.md` §19 advertised three flags that did not exist**, one of which
  made `bachelorpad --line 427 server.log` try to open a file called `427`.

All four are fixed. `--line=N` is built out of exactly what Ctrl+G does --
the same `go_to_line`, the same `reveal` -- and **confirmed by driving the
release binary under both editor surfaces**: line 555 of a 900-line document
at the top of the viewport, highlighted, with the custom surface's status bar
reading `Ln 555, Col 49`. `--large-file` was deleted (a decision had already
obsoleted it) and `--readonly` declined as a *flag* and queued as a design
pass, because read-only is a property of a document and this product already
has two of those.

**Two things worth not re-deriving:**

- **A Windows release build is GUI-subsystem, so stdout goes nowhere when no
  console is attached** -- and Rust tolerates the invalid handle rather than
  panicking, so the text is simply dropped and exit stays 0. Measured three
  ways, in ADR-0054's table. `AttachConsole` would fix it and costs `unsafe`
  plus a Windows dependency to serve somebody double-clicking a text editor
  in order to read its `--help`. Conceded deliberately.
- **`Drive-Window.ps1` gained `-Flag`.** A flag does its work at startup, so
  driving one means launching with it and photographing the first frame.
  There was no way to do that before.

**The gate failed once and it was worth the run**: `clippy::double_must_use`,
because `Result` already carries `#[must_use]`. Both legs, same line, and
nothing else in eleven stages.

### Then the release, and two lessons from building it

`scripts/New-Release.ps1` and `scripts/release-linux.sh` build, stage, archive
and checksum both targets — binary, both licence texts, a `bpad` short
spelling, and a `BUILD.txt` naming the commit. **Both archives were extracted
and run before it was called done.** Two things went wrong and both are
lessons rather than fixes, in `DECISIONS.md`:

- The Linux leg was a PowerShell string inside `bash -c` around a bash
  `$(...)` around an `awk '{print $2}'`. It did not error — it compared an
  empty version string and reported "the Linux leg failed" with the cause
  invisible. **When quoting nests three deep, write a file.**
- Staging on the `/mnt` mount produced world-writable licence files, because
  **DrvFs ignores `chmod` silently**: exit 0, no warning, no change. **A call
  that succeeds and changes nothing is worse than one that fails.**

### And P3, which found the hole trap 7 only half-closed

ADR-0050 fixed golden vectors for `.bpadx` last session. The same hole was
open for **every other file this product leaves behind** — `config.toml`,
`recent.toml`, the security history and the recovery journal — each covered by
a write-then-read-back test that proves one build agrees with itself.

The worst is `recent.toml`: `Recent::parse` ends in `unwrap_or_default()`, so
a format change there does not fail, it **silently empties the user's recent
list**. Every guard was watched failing before the file was called done, by
mutating all four fixtures at once.

### The twelfth session, for context

**It took what was then the top of the plan -- prepare D4's envelope review -- and finished it.**
[ADR-0050](../docs/decisions/ADR-0050.md):
[BPADX_ENVELOPE_REVIEW.md](../docs/architecture/BPADX_ENVELOPE_REVIEW.md) is
the brief D4 needed -- the byte layout, the four call sites, a self-audit
against fourteen standard pitfalls each naming the test behind it, and ten
questions stated explicitly. **Four of the fourteen are conceded rather than
mitigated** -- no key commitment, no domain separation, no padding, no
streaming API -- which is the point: a brief claiming fourteen for fourteen is
one a reviewer stops trusting at the first gap they find themselves.

Writing it found two things, both in the seam ADR-0049 was already about:

- **Nothing tested that a document written by an earlier build still opens.**
  Every round trip in the workspace seals and opens with the *same* build,
  which proves the two halves agree with each other and nothing about what is
  on somebody's disk. Three golden vectors had been checked into
  `fuzz/corpus/envelope/` since the corpus was seeded and the survival harness
  fed all three to `open` and discarded the result. Two tests now assert them.
- **`fuzz/` was still measuring a ceiling ADR-0035 lowered a session earlier.**
  Three rows of its cost table were past the bound, so they timed an instant
  refusal rather than the work -- real measurements of the wrong thing. The
  control is now one KiB past the ceiling instead of sixteen times past it.

**Then it drove the window**, over three rows no script had ever been
pointed at. [ADR-0051](../docs/decisions/ADR-0051.md), four defects:

- **Run's greyed readout denied there was anything to run** while an enabled
  Run Document three rows below ran the file correctly. `document_runnable`
  was already a parameter and unused;
- that readout was **52 characters against a `LABEL_BUDGET` of 42**, and two
  more were around 70 -- all three *reasons*, the worst text to lose;
- **`every_menu()` never held the Run menu**, which is why a test asserting 42
  passed over a 52-character label every run. Its doc comment names that
  failure mode in as many words;
- **`Drive-Window.ps1` photographed the wrong window again** -- the 1x1
  capture ADR-0049 was written to prevent, because it fixed the *selection*
  rule and left the *wait* condition asking a different question.

**And then the worst defect since the passphrase one.** Driving Find Next on
that fixture found that **Enter twice in the find box replaces the match with
a line break** ([ADR-0052](../docs/decisions/ADR-0052.md)). The cause was a
doc comment asserting that Find Next, Go to Line and a cross-file result are
"each a single deliberate jump the user makes once" -- three claims, one true,
with Go to Line's own call site contradicting it three lines away. The fix is
`reveal` (formerly `preview_match`), which already did the right thing for the
typing case and had never been asked about the others. **Confirmed at the
keyboard**, after this session lost the foreground and could not check its own
work.

### The eleventh session, for context

Four commits, four ADRs. `16af4d6`, `f097cd5`, `c94806d`, `b6603e3`.

| | |
| --- | --- |
| [ADR-0046](../docs/decisions/ADR-0046.md) | Research mode's last five planned rows were a paper's structure (IMRaD), not this product's. Two folded into Research Report, one became Open Questions, one became What the Store Holds, one dropped |
| [ADR-0047](../docs/decisions/ADR-0047.md) | `CLAUDE.md`, three standing authorities, one home per kind of fact; pre-push checks for a full gate instead of repeating it; `cargo-nextest`. **15.0 min to 7.2** |
| [ADR-0048](../docs/decisions/ADR-0048.md) | Every menu row works or is a readout. Ten built, eleven deleted |
| [ADR-0049](../docs/decisions/ADR-0049.md) | The window is driven by a script; a clipped label is elided; a temp path a recycled pid can reuse is not unique |

**Four defects, and the shape of each is in `DECISIONS.md`:**

- `Store::forget_document` never cleared orphaned tags, while its own test
  asserted the opposite of its doc comment;
- Research Report scanned 500 documents and said so nowhere the reader could
  see — "inspectable" was true of the source and not of the product;
- a menu label was clipped mid-word with no ellipsis, and every label it
  happened to was a **greyed row's reason**;
- the test suite got **flakier the more it was run** — 5,862 leftover temp
  files across 495 pids, and a recycled pid inherited an earlier run's
  security history.

---

## 2. Both questions were answered the same day

`project/DECISIONS_NEEDED.md` emptied on 2026-08-22, refilled on
2026-08-23, and emptied again the same day.

- **D15 — signing:** **defer**, and refuse self-signing outright
  ([ADR-0055](../docs/decisions/ADR-0055.md)). It gated one sentence in a
  README, not the work.
- **D16 — embeddings:** **all three sources, as choices**
  ([ADR-0056](../docs/decisions/ADR-0056.md)) — and looking found the choice
  already existed. `bp_security::Embeddings` has carried `Cloud`/`Local`/`None`
  since ADR-0020, with a per-profile mapping and a doc comment reading
  *"Phase 10 reads this"*. Two sessions called it the largest unasked question
  in the project while its answer sat in `src/`. **Before asking a question,
  grep for its answer.**

**PR #2 is open** and has been since 2026-08-22; clicking merge stays yours
(ADR-0053). This session added four commits' worth of work to it.

**D4 remains prepared and undischarged.** The brief is
[BPADX_ENVELOPE_REVIEW.md](../docs/architecture/BPADX_ENVELOPE_REVIEW.md);
what is left is finding a reviewer, which no agent can be.

### How D13 read before it was corrected, because the shape recurs

`project/DECISIONS_NEEDED.md` is the register. **D14 was answered and is
gone** (no Markdown preview; Slint has no rich text).

**D13 was corrected rather than answered, and that is the more useful
result.** For five sessions the record said "111 commits live on the branch;
`main` has none of them". `main` is at `31f6858` — *"Merge phases 1-19 into
main (PR #1)"* — so most of the branch's history is already there and the
branch is **49 commits past that merge**. One `git merge-base` settles it and
nobody had run it -- and it was re-checked again on 2026-08-22 before the
question was asked a sixth time, which is the habit trap 6 exists to install.

So the real question is small: **open PR #2 now, or keep accumulating?** A
fast-forward is not available; the merge commit is on `main` alone.

**D4 was the largest unstarted work in the project and is now prepared.** The
outside review of `bp-crypto`'s `.bpadx` envelope is written up as a brief
(ADR-0050) so that what remains is **finding a reviewer** -- an hour of
somebody's time rather than a project, and the half no agent can do.

Why it waited five sessions is the more useful part. An agent cannot
*discharge* D4, so every session correctly declined to do the thing and then
did nothing else about it either. **"I cannot do this" is not the same as
"there is nothing to do."**

---

## 3. The plan for the next session — day 3 of thirty

**Rewritten 2026-09-10.** The old section 3 planned the fifteenth session
around E1, M1, P2, P5 and R1. **Three of those five are gone** — P2, P5 and R1
were answered in the repository downstream — and the rest is now sequenced by
[V1_1_1_PLAN.md](V1_1_1_PLAN.md) against a release dated **2026-10-10**.

### What the sixteenth session did

The plan, and **days 1–2 of it**, in one sitting: [ADR-0080](../docs/decisions/ADR-0080.md).

- **Thirteen greyed rows went live in the build a user runs.** `TextInput` does
  report its caret; the record denied it in five places.
- **Find was highlighting the wrong span** on any document with a multi-byte
  character before the match, silently, because character ranges were handed to
  a toolkit that selects by bytes.
- **`Buffer::{set_access, with_access}` have zero callers** outside their own
  crate, so the read-only guards in `insert`/`remove` have **never fired**. That
  answers R1 with evidence.

### Take these, in this order

#### a. Drive the window over the thirteen rows. **Start here.**

**Nothing built on 2026-09-10 has ever been clicked**, and it lands in the one
seam this repository says no test can reach — five defects have lived there,
including a passphrase typed into the open document.

**An agent can do all of this**: none of these rows opens an `rfd` dialog, which
is what usually makes a manual pass a person's job.

| Check | Why this one |
| --- | --- |
| **Insert ▸ Date with the caret mid-document** | `adopt_widget_caret` reads `caret-byte-offset` *at click time*, and a `MenuPopup` takes focus when it opens. The property is stored state rather than focus-derived, so it *should* hold — **"should" is the word that cost eight sessions**. If it reads 0, the stamp lands at the top of the file |
| **The same with a selection active** | The anchor makes the trip too, so the insert must *replace* the selection rather than insert beside it |
| **Insert ▸ Date in a document containing `é` before the caret** | The byte-to-character conversion, in the one case where getting it wrong is visible |
| **Ctrl+D, Alt+Up, Alt+Down** | These reach `run_line_edit` by *shortcut*, not through the menu, so they never pass through `dispatch`'s menu arm. **Check they adopt the caret too** — if they do not, the fix is incomplete in the way that is hardest to see |
| **Find `cat` in `héllo cat`** | The Find fix, directly. Screenshot the highlight |
| **Then the same six under `--editor-view`** | Trap 3: a wiring change usually needs both surfaces |

`Drive-Window.ps1 -Keys "..."` takes a **list**, and a bar takes the caret a
tick after the row that opens it.

#### b. Finish the inventory — passes A, C and D

[V1_1_1_PLAN.md](V1_1_1_PLAN.md) §2 has the methods. Pass A is every row
clicked, and (a) above is its first slice. **Pass D already has two entries**:
`Policy::embeddings`, an axis two inspectors read out and nothing computes, and
`Buffer::access` above.

#### c. Wire `Buffer::access`, or delete it — but decide with the evidence

Downstream deleted it, correctly, because Lite has neither a huge-document
viewer nor Lock Document. **This product has both**, so here the same finding
argues for *wiring*: `LargeFile::open` already computes an `Access`, and
Security ▸ Lock Document is a second producer nobody connected.

**Adapt, do not adopt** — that distinction is what pass E is for.

#### d. The status bar could now say `Ln 42, Col 18` in both surfaces

`cursor_label` returns a line count under `TextInput` because reading the caret
meant depending on a toolkit internal. That is now depended on anyway, so the
reason has changed and specs.md §3 asks for line and column.

**Deliberately not done on 2026-09-10, and the reason is the useful part.**
ADR-0080 took the caret for rows that *edit* at it, where a wrong answer shows
up immediately. A status bar refreshed on **every keystroke** is a different
risk profile and wants its own decision — and it needs a caret feed into
`AppState`, which `cursor_label` has no access to today.

#### e. Then week 2 of the plan: the Experimental menu and `bp-feedback`

Both are decision-free. Ids **1000–1049**, and 950–999 left as the gap that
proves somebody checked `RUN_CELL_BASE`'s bound.

### What is waiting on Daniel, and what it blocks

| # | Question | Blocks |
| --- | --- | --- |
| **D26** | `MIT OR Apache-2.0` or GPL-3.0-only? | **Day 22.** It ships inside the artefact. Escalate on day 8, not day 22 |
| **D25** | Hosted CI, against ADR-0016? | Nothing yet; it edits `CLAUDE.md` |
| **D21** | Does `--editor-view` survive? | Answer on (a)'s evidence. **Not before it** |
| **D27** | What does the countdown count to? | The website, on day 29 |

**Everything else in the plan has a default and is not waiting.**


## 4. Blocked on Slint 1.17.1, checked in its source rather than assumed

- **Input-method composition.** `FocusScope` rejects `UpdateComposition` and
  `CommitComposition` in both handlers and exposes no callback for either.
- **Drag and drop to open.** The winit backend has no file-drop plumbing and
  `DataTransfer` carries only plain text or an image.
- **Markdown preview.** No rich-text item — no styled runs, no spans — so
  inline formatting cannot be rendered at all (ADR-0048, answering D14).

All three want a re-check on a Slint upgrade and none wants an afternoon
before that.

---

## 5. What is left that is not blocked and not decided

Honest inventory, for whoever wants to size something:

- **Multi-cursor** (ADR-0048 dropped the row, not the idea). Needs
  `bp-editor` to carry a set of carets through every command, selection and
  undo entry, and it can only work under `--editor-view`. A real feature with
  a real design pass in front of it.
- ~~**Phase 10, semantic search.** Needs embeddings~~ **Decided 2026-08-23,
  [ADR-0056](../docs/decisions/ADR-0056.md), and now three queue items rather
  than a phase.** E1 is the one to take: it needs neither provider to exist.
- ~~**Phase 20, packaging and release.** Not started~~ **Started and mostly
  done, 2026-08-23.** P1, P3 and P4 landed; the product builds archives for
  both targets with checksums. P2 (an icon, half of which is Daniel's) and P5
  (an installer, or a decision there is not one) are what remain.
- **Print.** Platform work with no cross-platform story yet.
- ~~**`benches/` holds a README and no benchmark**~~ **Done, 2026-08-22.**
  `cargo bench -p bp-editor --bench scroll` measures the half of ADR-0017's
  revert condition that can be measured without a capture rig -- the
  application's own per-frame work -- and pins the claim in `Anchor`'s doc
  comment that scrolling "costs only what is on screen". A thousand-fold
  document is **1.19x** the cost. The compositor half still needs the rig and
  the bench says so in its own output.

---

## Housekeeping

- **`crates/bp-ui/src/menus.rs` at 3,574 lines is the largest file in the
  crate.** Roughly half is tests. Its seam, if it needs one, is the `action`
  module against the builders. It did not grow this session -- the command
  line is not a menu.
- **No phase is "Built, unwired".** That count was three for four sessions.
  `ROADMAP.md` keeps the legend because the state is real and will recur.
- **`cargo-nextest` is installed on this machine and in `Ubuntu-24.04`.** A
  fresh clone works without it, more slowly.
- **A derived repository exists downstream**, `bachelorpluslite`, cloned at
  `3589205` on 2026-08-29. It has its own remote and its own plan; nothing in
  this repository depends on it and no work here is blocked by it. Recorded
  only because **it inherits every on-disk format decision this project has
  made**, so a format change here is now a change with a reader elsewhere.
- **Three orphaned files sit in `%APPDATA%\bachelorpad\`**: `recent.toml`,
  `security-history.log`, and an empty `recovery` folder. Safe to delete.
- **A single stray recovery journal blocks every launch, and therefore all
  window-driving.** `rfd`'s "Unsaved work recovered" is its own top-level
  window (class `#32770`); the application's own window never appears behind
  it, so `Drive-Window.ps1` fails with *"no usable window appeared within
  30s"* -- which reads like a driver fault and is not one. It cost two runs
  on 2026-08-22 before it was diagnosed by enumerating the process's windows.

  **The obvious filter does not work.** Clearing journals whose `path`
  matches a scratch directory misses the ones that matter most: an *Untitled*
  buffer has `"path": null`, so it matches nothing and survives every sweep.
  The one found on 2026-08-22 held the three characters `D13`.

  Look in `%LOCALAPPDATA%\bachelorpad\state\recovery\*.json`, read each one --
  they are small and name their own document -- and **preserve before
  removing**: it is somebody's unsaved work, however little of it.
- **Idle memory drifted from ~20 MB to ~32 MB** after dialogs are opened.
  Below the 50 MB target, observed three times, never investigated.
- **A clipboard menu row decodes its id against the history at click time.**
  Pre-existing; the Run menu's `run-menu-opening` has the pattern that fixes
  it.
- **`fuzz/corpus/yaml/` breaks editor tooling that scans the workspace.**
  Disable Google's Cloud Code extension for this workspace.
- **The panic lints are on** — `unwrap_used`, `panic`, `todo`,
  `unimplemented`. `expect_used`, `unreachable` and `indexing_slicing` are
  not; each is a change to the code rather than the policy.
- **`artifacts/releases/` holds ~23 MB of built archives and is gitignored.**
  Safe to delete; `./scripts/New-Release.ps1 -Linux` rebuilds them in about a
  minute once the release profile is warm.
- **The release script refuses a dirty working tree** and says which files.
  That is deliberate -- an archive built from a tree nobody can check out
  again is a copy, not a release -- and `-AllowDirty` is there for trying it
  out.
- **`clippy::double_must_use` is denied by the gate**, and `Result` already
  carries `#[must_use]`. Adding one to a function returning `Result` fails the
  build; it cost one gate run on 2026-08-23.

---

## The traps, and where they live

`CLAUDE.md` names all **nine** and `DECISIONS.md` explains each. The one
added this session is the mirror of the one added last:

> **Before asking a question, grep for its answer.** Trap 6 says a claim in
> the record is not a property of the repository. This is the other
> direction: a decision recorded in a *type* is still a decision, and it is
> the kind least likely to be re-read, because nothing about a queue of
> questions suggests looking in `src/`. D16 cost two sessions to that.

The two added the session before are both about verification itself:

> **"Unique" must mean no *earlier* run either.** A scheme that prevents
> collisions and never deletes anything is a scheme that fails eventually, and
> gets likelier the more it is used.

> **A claim in the record is not a property of the repository.** D13 sat for
> five sessions on a premise one command disproved. A question nobody can act
> on gets re-read rather than re-checked, and re-reading is what preserved it.
