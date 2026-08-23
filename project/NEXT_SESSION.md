# Next session plan

Written 2026-08-23, at the end of the thirteenth build session.

State: **25 crates, 1,986 tests on Windows and 1,991 on Linux, 0 ignored**,
green across all eleven gate stages. 54 ADRs. The gate takes ~13 minutes with
both legs.

> **Phase 20 has started, and the queue is not empty any more.**
> [ADR-0054](../docs/decisions/ADR-0054.md). The command line is an interface
> now: `--help` lists thirteen flags, `--version` names the product and its
> licence, `--line=427 server.log` opens a file already at the line, and a
> mistyped flag says so instead of vanishing. The licences every manifest has
> claimed since the scaffold commit exist.
>
> **What that leaves is the first real artefact work in the project.** See §3.

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

`WORK_QUEUE.md` now has five ready items again, all of them phase 20's
artefact half plus one design pass.

---

## Read this first, then start

1. `CLAUDE.md` — the entry point, and the three standing authorities. **You do
   not need to ask to take work, to commit after a green gate, or to delete a
   planned row with an ADR.**
2. `project/DECISIONS_NEEDED.md` — ask its batch in your first message, then
   start work without waiting.
3. This file, for what matters most.

---

## 1. What the last session did, so nobody re-derives it

**The thirteenth session started phase 20**, inventory first as ADR-0048
recommends, and the inventory found four things in a product 1,972 passing
tests deep ([ADR-0054](../docs/decisions/ADR-0054.md)):

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

### The twelfth session, for context

**It took `NEXT_SESSION.md` §3a and finished it.**
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

**Then it drove the window at §3b**, which had three rows no script had ever
been pointed at. [ADR-0051](../docs/decisions/ADR-0051.md), four defects:

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

## 2. Two questions are open, and both are in the batch

`project/DECISIONS_NEEDED.md` was empty for exactly one day.

- **D15 — signing needs a certificate.** Buy OV or EV, ship unsigned, or
  defer? It gates how a release is *described*, not whether it can be built,
  and the honest default is **defer**: build the archive, the checksums and
  the Linux detached signature, and leave signing a single step for when a
  certificate exists. Nothing is wasted and nothing is claimed.
- **D16 — phase 10 needs embeddings.** Local model, opt-in network, or drop
  the phase? This is the question this file said on 2026-08-22 had not been
  asked yet. It is now asked, with the three answers written out, because a
  question described is not a question asked -- which is the same failure
  mode D13 spent six sessions in.

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

## 3. What to do, in priority order

### a. Phase 20's artefact half — the five ready items in `WORK_QUEUE.md`

**This is the top of the list and it is ordinary work.** P1 (a release script
that builds both legs, stages the binary with the two licences, archives and
checksums), P3 (upgrade testing against files an *earlier build* wrote --
trap 7, with `fuzz/tests/envelope.rs` as the pattern) and P4 (the `bpad`
short spelling) need no decision at all. P2 (icon and `.desktop`) carries a
**dependency decision** -- Windows takes an executable's icon from a
build-time resource -- so it wants an ADR, not just a commit.

Take them as a batch: they share one entry cost, which is "what does a
release actually consist of".

### ~~b. D4 — prepare the envelope review~~ **Done, ADR-0050**

What is left of it is Daniel's: send
[BPADX_ENVELOPE_REVIEW.md](../docs/architecture/BPADX_ENVELOPE_REVIEW.md) and
`crates/bp-crypto/` to somebody who did not write them. The brief names ten
questions; **three are live design questions rather than rhetorical ones**,
and a "no" to any of them is an ADR and a format version:

1. nonce uniqueness rests entirely on the 16-byte salt -- AES-256-GCM
   contributes only 4 random nonce bytes of its own;
2. no key commitment, so a partitioning oracle is available in principle;
3. no domain separation -- a document, a recovery checkpoint, an audit line
   and an Ed25519 signing seed share one construction with nothing telling
   them apart.

### c. Drive the window over things that are not dialogs

**Three of the four rows here were done on 2026-08-22 and found four defects**
([ADR-0051](../docs/decisions/ADR-0051.md)). What is left:

| Check | Why it matters |
| --- | --- |
| **Notebook ▸ Export as .ipynb** | Opens a save dialog, which `rfd` will not let a script past. Reading the dialog is possible; completing it is not. **A person's job** |
| **Typing, resizing, the wheel** | Checked clean 2026-08-21, before five sessions of change |
| **Cross-file results** | Reached through the find bar's "In Folder…", which opens an `rfd` folder picker. **A person's job** |

**Confirmed working on a document taller than the window**, so nobody
re-checks them: Go to Line scrolls and highlights; Find puts the query in the
box rather than the document, scrolls to the match and counts it; Find Next
steps between matches without editing the document (ADR-0052); Run ▸ Run
Document runs a real `.py` in a fresh subprocess and reports the right answer
in a panel rather than a dialog; and **Notebook ▸ Cell Outline jumps to a cell
below the fold**, scrolling the document and highlighting the fence.

That last one is worth more than it looks. It is the `dispatch::select` path
ADR-0052 deliberately **left alone** while routing three others to `reveal`,
and it is the check that the split was drawn in the right place: a panel click
still takes the caret, because there is no box to take it from.

**It is also drivable with no foreground**, which is how it got checked at all
— clicks are *posted* to the window and need nothing in front, so a menu, a
panel and a row in it are all reachable when `SendKeys` is not.

The reasons the rest is a person's job have not changed: **`rfd` modals
swallow synthetic input**, and File ▸ Set as Default Editor changes state
outside this application. Completing a signature writes a real Ed25519 key
into `%LOCALAPPDATA%`.

**`Drive-Window.ps1` got two fixes in the doing**, and both are worth knowing
before using it: it now waits for the window it will actually photograph
rather than for `MainWindowHandle` (which transiently names a 0x0 window, and
produced a 1x1 capture on the first run of the session), and `-Keys` takes a
**list** -- `-Keys "^g","555{ENTER}"` -- because one send cannot drive a bar
that takes the caret a tick later.

### d. The one unexplained failure

A Linux gate run failed once on 2026-08-22 and never again. `nextest` exits
100 for a test failure, so something genuinely failed; five whole-suite Linux
runs and two gate runs since were clean. **The name is unknown because the
gate's output had been truncated** — `CLAUDE.md` now forbids that.

If a Linux-only failure appears again, that is the second data point. Keep the
whole output.

---

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
- **Phase 10, semantic search.** Needs embeddings; nothing computes one, and
  doing so reaches [ADR-0033](../docs/decisions/ADR-0033.md). **Now D16**, so
  it is a question in the queue rather than a paragraph in this file.
- ~~**Phase 20, packaging and release.** Not started~~ **Started 2026-08-23**,
  ADR-0054. The half a person meets at the command line is done; the half they
  download is `WORK_QUEUE.md`'s P1-P4 and is the top of §3 above.
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

- **`crates/bp-ui/src/menus.rs` at 3,478 lines is the largest file in the
  crate**, and grew ~300 lines last session. Roughly half is tests. Its seam,
  if it needs one, is the `action` module against the builders.
- **No phase is "Built, unwired".** That count was three for four sessions.
  `ROADMAP.md` keeps the legend because the state is real and will recur.
- **`cargo-nextest` is installed on this machine and in `Ubuntu-24.04`.** A
  fresh clone works without it, more slowly.
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

---

## The traps, and where they live

`CLAUDE.md` names all six and `DECISIONS.md` explains each. The two added last
session are both about verification itself:

> **"Unique" must mean no *earlier* run either.** A scheme that prevents
> collisions and never deletes anything is a scheme that fails eventually, and
> gets likelier the more it is used.

> **A claim in the record is not a property of the repository.** D13 sat for
> five sessions on a premise one command disproved. A question nobody can act
> on gets re-read rather than re-checked, and re-reading is what preserved it.
