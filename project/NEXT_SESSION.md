# Next session plan

Written 2026-08-22, at the end of the tenth build session.

State: **25 crates, 1,941 tests, 0 ignored**, green on Windows and Linux
across all ten gate stages. 46 ADRs.

---

## The queue is empty, and that is the whole of the news

Five sessions ago capabilities did not exist. Four sessions ago they existed
and were unreachable. Three sessions ago the record disagreed with the
product. Two sessions ago the queue was gated on questions only a human could
answer. Last session the last three unreachable crates became reachable.

**This session took the one item that was ready, and there is no next one.**
`project/WORK_QUEUE.md`'s Tier 0 is empty. `docs/product/MENU_MAP.md`'s
Research section has no planned row left in it. Every item whose *shape* was
known has been taken.

That is not "the product is finished". It is a specific, checkable state:
**the next piece of work needs a decision before it can be sized**, and the
three decisions are in `project/DECISIONS_NEEDED.md`.

---

## 1. What this session did, so nobody re-derives it

Research mode's synthesis half, which `MENU_MAP.md` had listed as five
planned rows — research question, evidence, findings, methods, datasets —
since the scaffold commit.

**None of the five became a row named for itself**, and the scoping is
[ADR-0046](../docs/decisions/ADR-0046.md):

| Name | What it turned out to be |
| --- | --- |
| Evidence | Not a row. The documents each insight is drawn from, named *on* the insight |
| Methods | Not a row. Research Report's closing section, stating every threshold it applied |
| Datasets | Not a row in the report; the same closing section carries the numbers, and it *is* a row as **What the Store Holds** |
| Findings | Dropped. A second name for what Research Report already is |
| Research question | **Open Questions** — a document reading, because a question has a grammar and can be found, while which one you are asking is not something the document says |

Also shipped: `bp_semantic::questions` (fenced code skipped, questions
crossing lines within a paragraph, reported at the line they *begin* on),
`bp_storage::Store::summary`, and a fix to `Store::forget_document`, which had
promised to clear tags nothing carried and never had.

### The two lessons, both in `DECISIONS.md` at length

> **A name that has sat in a plan long enough starts to read like a
> specification.** This is the mirror of "undecided reads like large", and the
> more dangerous of the two: an unscoped *mode* at least looks unscoped, so
> somebody eventually asks. A named row in a table looks decided. The tell is
> to ask where the name came from — if the answer is the commit that
> scaffolded the repository rather than a decision that chose it, it is a
> sketch that has been read as a plan ever since.

And the one the scoping itself turned up:

> **"Inspectable" is a claim about the product, not about the source.**
> ADR-0041 promised "a fixed, inspectable rule". Every rule was written down —
> in `state/research.rs`. Meanwhile the report scanned the 500 most recently
> seen documents and told the reader nothing about it, so a store of nine
> hundred produced a report that read as complete. Ask of any honesty
> commitment: *honest to whom?*

---

## 2. Three decisions, and one of them has waited five sessions

`project/DECISIONS_NEEDED.md` is the register, unchanged this session at three
rows. **Nothing in the queue is blocked on them today** — which is exactly why
they are easy to keep not answering.

| # | Question | Why it is here rather than defaulted |
| --- | --- | --- |
| **D13** | **Does `main` get this product, and how?** | ~120 commits on a branch named for phase one; `main` has none of them. Four sessions of housekeeping have called this "the item most likely to hurt and least likely to be noticed", which is a description rather than a decision. Fast-forward, squash, or leave it — they say different things about the history, and leaving it should be *stated* rather than habitual |
| **D14** | **What should Markdown preview be?** | Slint has no HTML view. Render to styled Slint elements, shell out to a browser, or decline permanently and say so. It is the last row in `MENU_MAP.md` that is planned rather than decided |
| **D2** | Is `--editor-view`'s fork worth keeping? | Answered once (keep opt-in). Four features work in only one view, down from five last session |

**D4 is not on that list and is not done.** Get an outside review of
`bp-crypto`'s `.bpadx` envelope before trusting real documents to it. Answered
yes five sessions ago. Every primitive is vetted RustCrypto; the *composition*
is this project's own design, which is exactly what an author cannot catch by
re-reading their own work. **With the queue empty this is now the largest
unstarted piece of work in the project**, and it no longer has a more urgent
item in front of it.

---

## 3. Drive the window

Standing argument, unchanged: four manual passes each found a defect no test
could, and last session found five more, four of them by driving.

This session added two rows to a menu and a section to a report. **Nothing
here has been watched working** — the reports are asserted by tests over the
strings they produce, and the dialogs they open have not been opened by a
person.

### The checklist, in priority order

| Check | Why it matters now |
| --- | --- |
| **Research ▸ Open Questions, and ▸ What the Store Holds** | New this session and never clicked. Both open an `rfd` info dialog through `show_info`, the same path Research Report uses, so the risk is low and the *contents* are the point: does the store summary say something true about a machine that has actually been saving documents? |
| **Research ▸ Research Report on a real store** | Its closing section states a truncation that only appears above 500 documents. Nobody has a store that large, so **that branch has been tested and never observed** |
| **Anything that claims to take you somewhere, on a document taller than the window** | Regression check now rather than an investigation. Find, Find Next/Previous, Go to Line, a cross-file result, a cell-outline row. Open Questions reports a line and does *not* jump to it — deliberately, it is a dialog like Find Identifiers — so check that reading a line number and typing it into Ctrl+G works from a dialog that is still open |
| **File ▸ Set as Default Editor** | Still the only row that changes state outside this application, and deliberately never attempted by a script |
| **Complete a signature** | Reachable, and deliberately never finished by a script: it writes a real Ed25519 key into `%LOCALAPPDATA%` and a `.sig` beside the document |
| ~~Resize, wheel, huge file, drag-to-select, goal column~~ | Checked and clean, 2026-08-21 |

### What an agent can and cannot do here

An agent *can* drive this window — PowerShell, `PrintWindow`, `SendKeys`.
Two things it cannot, both learned the hard way:

- **`rfd` modals swallow synthetic input.** Every row added this session opens
  one, so *reading* the dialog is possible and dismissing it may not be.
- **`SetForegroundWindow` fails silently**, and a lost foreground is
  indistinguishable from the application dropping keystrokes. One "defect" was
  recorded and later withdrawn for exactly this. Check the foreground handle
  on both sides of every keystroke.

---

## 4. Blocked on Slint 1.17.1, checked in its source rather than assumed

Unchanged for two sessions, and listed so nobody re-derives them:

- **Input-method composition.** `FocusScope` rejects `UpdateComposition` and
  `CommitComposition` in both handlers and exposes no callback for either.
- **Drag and drop to open.** The winit backend has no file-drop plumbing and
  `DataTransfer` carries only plain text or an image.

Both want a re-check on a Slint upgrade, and neither wants an afternoon
before that.

---

## Housekeeping

- **`crates/bp-ui/src/menus.rs` at 3,160 lines is the largest file in the
  crate.** Listed for the reason `state.rs` used to be: this is the kind of
  thing that stops being noticed. It is one subject and roughly half of it is
  tests. Its seam, if it needs one, is the `action` module against the
  builders.
- **`state/research.rs` more than doubled**, 320 to 758 lines, and about half
  of that is tests. It now owns two menu rows rather than one. If it grows
  again the seam is the four insight functions against the two report
  renderers.
- **The Win32 device-name rule has been missed four times.**
  `bp_platform::paths::reserved_device_name` is the one home for the answer;
  the recurring mistake is not asking it.
- **Three orphaned files sit in `%APPDATA%\bachelorpad\`**: `recent.toml`,
  `security-history.log`, and an empty `recovery` folder. Safe to delete;
  nothing recreates them.
- **Idle memory drifted from ~20 MB to ~32 MB** after dialogs are opened.
  Below the 50 MB target, observed three times, never investigated.
- **Scroll smoothness under CPU rasterisation** is the standing revert
  condition on ADR-0017 and still needs a capture rig.
- **A clipboard menu row decodes its id against the history at click time.**
  Pre-existing; the Run menu's `run-menu-opening` has the pattern that would
  fix it.
- **`fuzz/corpus/yaml/` breaks editor tooling that scans the workspace.**
  Disable Google's Cloud Code extension for this workspace.
- **The panic lints are on** — `unwrap_used`, `panic`, `todo`,
  `unimplemented`. `expect_used`, `unreachable` and `indexing_slicing` are
  not; each is a change to the code rather than the policy.

---

## How to work in this repository

- `./scripts/Invoke-LocalCI.ps1 -Linux` is the gate. No hosted CI (ADR-0016).
  **It takes ~13 minutes, which is longer than a single tool call may run** —
  background it, and *do not edit files while it runs*: the Linux leg re-runs
  `cargo fmt --check` from scratch and a mid-run edit fails it for a reason
  that has nothing to do with the change.
- `./scripts/Install-GitHooks.ps1` once per clone. Pre-commit runs the quick
  gate; pre-push runs the full Windows one. **Neither runs the Linux leg.**
- Every commit in this repository has passed the gate. Keep it that way.
- `docs/decisions/` is the register; add an ADR when a decision has
  consequences beyond one file, and **amend it in place when the build changes
  the answer**.
- **Read `prompts/rosettas/R011_WIRE_CAPABILITY_INTO_SHELL.md` before touching
  `bp-ui`.** Rule 9 cost a session's worth of confusion: a menu row's callback
  cannot take the keyboard focus for itself.

### The traps this repository springs repeatedly

Four now, and the fourth is this session's:

1. **A function reading the real profile directory means `cargo test` writes
   there.** Sprung three times, once writing a real Ed25519 private key into
   `%LOCALAPPDATA%`. The fix is a field on `AppState`, not a function.
2. **Code taking a `Platform` parameter must not let `std::path` answer for
   it.** Sprung three times, every time green on Windows and caught by Linux.
   `bp_platform::paths::{file_name, join}` exist for this.
3. **A claim in a comment is not a property of the code.** Sprung three times.
   Ask of any comment asserting a property: what would fail if it stopped
   being true? If the answer is "nothing", the comment is a wish.
4. **Two claims about the same function can coexist for as long as nothing
   asks.** New, and it is trap 3 seen from the other side.
   `Store::forget_document`'s doc comment promised to clear orphaned tags;
   forty lines below, its own test asserted `"the tag itself survives"`. Both
   readers were correctly describing what they were looking at, and no query
   in the crate could tell them apart, because every tag-shaped query joins
   `document_tags` and an orphan is invisible to a join. It surfaced only when
   a new function had to count tags and pick which claim was true.

   **The generalisation: when trap 3's answer is "nothing would fail", the
   test beside the comment is free to say the opposite — and probably does.**
