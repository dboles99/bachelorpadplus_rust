# Next session plan

Written 2026-08-22, at the end of the eleventh build session.

State: **25 crates, 1,972 tests, 0 ignored**, green on Windows and Linux
across all eleven gate stages. 49 ADRs. The gate takes ~7 minutes.

---

## Where this leaves things

**Every menu row in the product either does something or is a readout**, and a
test says so ([ADR-0048](../docs/decisions/ADR-0048.md)). `WORK_QUEUE.md`'s
Tier 0 is empty. Nothing anywhere says "not implemented yet".

That is not "finished". It is a specific state, and it is the third time this
project has reached it: **the next piece of work needs a decision before it
can be sized.** The difference this time is that the queue emptied for a
reason worth remembering — of 38 rows that looked like a backlog, most had
already shipped under other names, and eleven were better deleted than built.

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

## 2. Two decisions, and one of them was the wrong question

`project/DECISIONS_NEEDED.md` is the register. **D14 was answered and is
gone** (no Markdown preview; Slint has no rich text).

**D13 was corrected rather than answered, and that is the more useful
result.** For five sessions the record said "111 commits live on the branch;
`main` has none of them". `main` is at `31f6858` — *"Merge phases 1-19 into
main (PR #1)"* — so most of the branch's history is already there and the
branch is **48 commits past that merge**. One `git merge-base` settles it and
nobody had run it.

So the real question is small: **open PR #2 now, or keep accumulating?** A
fast-forward is not available; the merge commit is on `main` alone.

**D4 is not a question and is the largest unstarted work in the project.** An
outside review of `bp-crypto`'s `.bpadx` envelope, answered *yes* five
sessions ago and never done. Every primitive is vetted RustCrypto; the
*composition* is this project's own design, which is exactly what an author
cannot catch by re-reading their own work. **With the queue empty it has
nothing in front of it.**

---

## 3. What to do, in priority order

### a. D4 — prepare the envelope review

The only substantial work that needs no new decision. An agent cannot
*discharge* D4 — a reviewer in this session is not an outside one — but it can
make the review cheap for whoever does it:

- a written description of the `.bpadx` composition: Argon2id parameters, the
  two AEADs, the chunking, what goes in the AAD and why the chunk index is
  bound into it;
- the questions a reviewer should be asked to answer, listed explicitly;
- a self-audit against the standard composition pitfalls — nonce reuse across
  chunks, truncation and reordering, the header's authentication, what a
  wrong passphrase reveals — with each answer pointing at the test that holds
  it.

Ship that as a document, and D4 becomes an hour of somebody's time rather than
a project.

### b. Drive the window over things that are not dialogs

`scripts/Drive-Window.ps1` exists now and found a defect within an hour of
being written. What it has **not** been pointed at:

| Check | Why it matters |
| --- | --- |
| **A document taller than the window** | Find, Find Next/Previous, Go to Line, a cross-file result, a cell-outline row. All fixed once and never re-checked; a fixture that fits on one screen cannot test anything about scrolling |
| **Run ▸ Run Document on a real `.py`** | New, and the only new row that *executes* something rather than reporting. `-Kill` after, and read the run panel rather than a dialog |
| **Notebook ▸ Export as .ipynb** | Opens a save dialog, which `rfd` will not let a script past. Reading the dialog is possible; completing it is not |
| **Typing, resizing, the wheel** | Checked clean 2026-08-21, before four sessions of change |

Still a person's job, and the reasons have not changed: **`rfd` modals swallow
synthetic input**, and File ▸ Set as Default Editor changes state outside this
application. Completing a signature writes a real Ed25519 key into
`%LOCALAPPDATA%`.

### c. The one unexplained failure

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
  doing so reaches [ADR-0033](../docs/decisions/ADR-0033.md).
- **Phase 20, packaging and release.** Not started, and the first thing that
  would make "does `main` get this" matter.
- **Print.** Platform work with no cross-platform story yet.
- **`benches/` holds a README and no benchmark**, which is why Tools ▸
  Benchmarks was deleted rather than built. ADR-0017's scroll-smoothness
  revert condition still has no capture rig, and that is the benchmark most
  worth having.

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
