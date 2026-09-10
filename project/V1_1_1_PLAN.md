# v1.1.1, in thirty days

Written 2026-09-10, for **2026-10-10**.

This plan answers four asks, and they are less separate than they look:

1. **every function works, or it is removed;**
2. **an Experimental Features menu**, so unproven work has somewhere to live
   that is not the product;
3. **an internal feedback system**, so a ticket costs a click rather than an
   afternoon;
4. **breadth** — standard features, the gaps, and speculative ones, *"even if
   they are wrong or end up being illogical"*.

**Asks 1 and 4 are one ask seen from two ends, and the Experimental menu is
what reconciles them.** "Remove anything that does not work" and "build
speculative things that may be wrong" contradict each other only while there
is one shelf to put a feature on. With two shelves the rule is short, and it
governs everything below:

> **The product makes no promise it cannot keep. The Experimental menu makes
> no promise at all, says so on every row, and is off by default.**

---

## 0. Three findings, taken before the plan was written

**This section exists because ADR-0048's lesson is the one this repository has
paid for most often: take the inventory before the estimate.** Three hours of
looking, before any scheduling, moved more of this plan than any amount of
sequencing would have.

### 0a. `TextInput` will tell you where the caret is

It has been able to since before this repository existed, and **the record says
the opposite in six places.**

`i-slint-compiler-1.17.1/builtins.slint`, lines 1681–1684:

```slint
// Internal, undocumented property, only exposed for tests.
out property <int> cursor-position_byte-offset;
// Internal, undocumented property, only exposed for tests.
out property <int> anchor-position-byte-offset;
```

A real `Property<i32>` on the item, set on every caret move
(`i-slint-core-1.17.1/items/text.rs:1617`), and **it compiles when read from
our own `.slint`** — checked on 2026-09-10 by adding
`editor.cursor-position-byte-offset` to a function in `app.slint` and running
`cargo check -p bp-ui`, which passed. The probe was then reverted.

| Claim, and where it is written | Status |
| --- | --- |
| "only *reading* the caret is impossible under `TextInput`" ([ADR-0018](../docs/decisions/ADR-0018.md)) | **False on 1.17.1** |
| "`TextInput`'s caret is unreadable from here" (`menus.rs`) | **False** |
| "the caret is not readable" (`app.slint`, `paste-from-clipboard`) | **False** |
| "**Caret** means live only under `--editor-view`" ([MENU_MAP.md](../docs/product/MENU_MAP.md)) | **False, for thirteen rows** |
| "the tax is four features" (ADR-0018) | **The tax may be zero** |

**Thirteen rows that are greyed in the build every user runs** become ordinary
rows: Edit's Duplicate Line, Move Line Up and Move Line Down; Insert's five
date/time stamps; Insert's Bold, Italic, Link, Code Block and Table. That is
the largest single block of *"this row is here and does nothing"* in the
product, and **ask 1 is mostly this.**

It also makes ADR-0018's second re-ask condition — *"when the tax becomes
visible"* — met from an unexpected direction. `--editor-view` exists to buy
four features `TextInput` could not provide. If the caret is readable it buys
**none**, and what it still costs is unchanged: **CJK input does not work in
it**, because `FocusScope` rejects composition events and there is no route on
1.17.1. That half of ADR-0018 was checked in Slint's source and stands. See
**D21**.

**One distinction must not be lost when D21 is answered.** `EditorSurface` and
`--editor-view` are different things. The surface draws every huge document in
*every* build ([ADR-0030](../docs/decisions/ADR-0030.md)) and is read-only
there; the *flag* is a statement about which surface **edits**. Retiring the
flag does not delete the surface.

> **Two things were learned after this section was written, on 2026-09-10, and
> both sharpen it rather than change it.**
>
> **The answer was also inside this repository.** `AppState::cursor_label`'s doc
> comment said *"under `TextInput` the caret is reachable only through a
> property marked internal and undocumented"* -- accurate, and the local
> decision built on it was right. Four other places said the caret was
> **unreadable**. ***"Reachable only through an internal property" and "not
> reachable" are not the same sentence***, and the distance between them was
> thirteen menu rows. **A local decision not to use a capability reads, three
> files away, as proof the capability does not exist.**
>
> **And enabling the rows was not enough.** `run_line_edit` began with an
> `editor_view` early return, so the three Edit rows became enabled rows that
> silently did nothing -- worse than greyed -- **while 1,997 tests passed**,
> because a state test was still asserting that was correct. Trap 4 from the
> inside, caught by reading test names during an unrelated investigation. The
> lesson for the rest of this plan: **when a row stops being greyed, check the
> `AppState` method behind it for the guard that made it greyable.**

### 0b. The downstream repository has already answered six of this plan's
questions — and shipped the code

`G:/dev2/bachelorpluslite` was cloned from this repository at `3589205` on
2026-08-29 and was recorded here as *"a separate product with a separate plan"*
that this repository does not depend on. **That was true of the product and
false of the decisions.** It is now nineteen ADRs ahead — 0057 to 0076 — and
several of them answer questions this plan was about to schedule as work:

| Downstream ADR | What it settles | What it does to this plan |
| --- | --- | --- |
| **ADR-0068** — the icon ships beside the executable, because there is no installer | **P2 / D23.** And `assets/bachelorpad.ico` and `assets/io.github.dboles99.BachelorPadPlus.png` **exist there** | Week 4's icon work is a **back-port, not a design**. The artwork blocker is gone |
| **ADR-0067** — there is no installer, and an unsigned one would be worse than none | **P5 / D24**, with the reasoning already written | Deletes a decision from this plan |
| **ADR-0066** — read-only is the save refusing, and the type that claimed otherwise never ran | **R1**, the open design pass on `--readonly` | Deletes a queue item |
| **ADR-0073** — hosted CI is restored, because **ADR-0016's premise was never re-checked** | `.github/workflows/{ci,release,docs}.yml` exist there | **D25.** `CLAUDE.md` and R011 both assert *"this IS CI — there is no hosted CI"* |
| **ADR-0071** — the licence is GPL-3.0-only, and the `-only` is the decision | Downstream is GPL-3.0-only; **every manifest here claims `MIT OR Apache-2.0`** | **D26.** `--version` prints it, so it is user-visible |
| **ADR-0072** — macOS is declined, and what it would cost is written down | A question nobody here has asked out loud | Back-port the refusal |
| **ADR-0075** — one documentation source, four destinations, nothing retyped | The mechanism this plan's day 28 needs | Back-port |
| **ADR-0058, 0065, 0069, 0070** | A hash manifest nobody verified; two things the reduction weakened; a registration table offering a format it cannot open; state written by a departed feature | Survey each for whether the defect exists here too |

And one commit is worth naming on its own: **`b474291` — "the window icon is a
PNG, because Slint cannot decode an ICO."** That is a trap this plan would
otherwise have sprung in week 4, found and fixed downstream already.

**The lesson, and it belongs in `DECISIONS.md`:** *a derived repository is a
second reader of every decision this one made, and it re-checks premises for
free.* [NEXT_SESSION.md](NEXT_SESSION.md) recorded the clone as inheriting
this project's on-disk formats — the real inheritance ran the other way, and
nineteen ADRs of it went unread for twelve days.

**Nothing is back-ported without being read.** Lite is a *reduction*; several
of its ADRs remove things this product keeps, and a decision that is right for
a Notepad replacement can be wrong here. The eight rows above are the ones
whose reasoning is about packaging, licensing or verification rather than about
scope.

### 0c. The website has already published what 1.1.1 promises

`bpad.prompt-forge.dev` ships Lite and collects early-access signups for **this
product** ([downstream ADR-0076](../../bachelorpluslite/docs/decisions/ADR-0076.md)).
Its waitlist feature list is not marketing copy — it is *"the list of what
ADR-0057 and ADR-0059 through ADR-0064 removed from this product, every item of
which demonstrably worked before it was taken out."*

**So 1.1.1's headline scope is already fixed, publicly, and it is the
difference between the two products:**

| Promised, because Lite removed it | Downstream ADR | State here |
| --- | --- | --- |
| Running code, and notebooks | 0057 | **Ships.** ADR-0040/0043/0045 |
| Citations and the research menu | 0060 | **Ships.** ADR-0044/0046 |
| Clipboard history | 0061 | **Ships** |
| The Data menu, every format | 0062 | **Ships.** ADR-0023 |
| Huge-file mode | 0063 | **Ships.** ADR-0027/0030/0042 |
| The security stack — profiles, `.bpadx`, signing, redaction, audit | 0064 | **Ships.** ADR-0020/0021/0024/0026/0028/0031 |

**Every one of those already works in this repository.** That is the single
most important fact about a thirty-day plan: **1.1.1 is not a build, it is a
finish.** The differentiating features exist and are tested; what is missing is
that the product does not ship, does not say its own version truthfully, has
thirteen dead rows in the build a user runs, and has no way for a user to tell
anybody about it.

**One discrepancy needs resolving before the page and this plan disagree in
public.** Downstream ADR-0076 says the countdown *"marks early-access signup
closing, thirty days out, at a fixed date"* and that the waitlist page
*"promises features and no dates and no prices."* The instruction that produced
this plan describes it as a countdown to the **release**. Those are different
claims and only one can be on the page. See **D27** — and note that ADR-0076
already commits to the honest form: *"if the date moves, the constant is edited
and the reason is said out loud."*

---

## 1. Decisions this needs from Daniel

**Paste-ready, as [DECISIONS_NEEDED.md](DECISIONS_NEEDED.md) requires.** Work
starts on everything not blocked by one of these, without waiting.

| # | Question | My default if you say nothing |
| --- | --- | --- |
| **D17** | **`1.1.1` from `0.1.0`?** Nothing has shipped, so `1.1.1` tells a first user there were two releases they missed | **Take `1.1.1`, as asked and as the website says.** One line in the workspace manifest; the release script reads the *binary*, so `--version` and the archive names follow untouched. An ADR records why it starts there, so nobody later reads it as two lost releases |
| **D18** | **Does an experimental flag survive a restart?** | **Persist**, in an `[experimental]` table in `config.toml`, **and show every active flag in the status bar and in Help ▸ Diagnostics.** A forgotten experiment produces a bug report about a different product |
| **D19** | **Does a ticket ever leave the machine by itself?** | **No, and never.** Build the bundle locally, show the user all of it, then hand off — clipboard, a saved file, or their browser at a pre-filled form. Auto-submit contradicts [ADR-0006](../docs/decisions/ADR-0006.md). **A real endpoint is a different design and about four days** |
| **D20** | **Where do tickets go?** | **A pre-filled GitHub issue URL.** The site already posts the waitlist to a hosted form service (downstream ADR-0076); **that same provider is the fallback** if the tracker must stay private |
| **D21** | **Does `--editor-view` survive 1.1.1?** §0a | **Retire the flag, keep the surface** — one editing path means one place a feature can be wrong, and it deletes R011's trap 3. **Do not answer before day 7**: the evidence arrives then, and answering now is answering on a claim |
| **D22** | **Unsigned at 1.1.1?** A *public* release means every Windows user meets "Windows protected your PC" first | **Ship unsigned, say so on the download page, and put the exact click-through in the README.** [ADR-0055](../docs/decisions/ADR-0055.md) refuses self-signing and downstream ADR-0067 agrees. A certificate is weeks of lead time and does not fit thirty days — **start the purchase now if you want it for 1.2** |
| **D23** | ~~The icon is yours~~ **The icon exists.** `assets/bachelorpad.ico` and the PNG are downstream | **Back-port both, plus ADR-0068's reasoning and `b474291`'s fix** (Slint cannot decode an ICO — the window icon must be the PNG). No longer blocked on you |
| **D24** | ~~Installer or zip?~~ | **Downstream ADR-0067 answers it: no installer, and an unsigned one would be worse than none.** Adopt it unless you disagree |
| **D25** | **Hosted CI?** Downstream restored it (ADR-0073) on the grounds that **ADR-0016's premise was never re-checked**. `CLAUDE.md` and R011 both still assert there is none | **Read ADR-0073 and adopt it.** A public release wants a build a stranger can verify. **This changes `CLAUDE.md`, so it is your call, not an agent's** |
| **D26** | **Which licence?** Every manifest here claims `MIT OR Apache-2.0`; downstream is **GPL-3.0-only** (ADR-0071) and is derived from this code | **Decide before day 22.** `--version` prints it and the archives carry the texts, so a wrong answer ships in the artefact. **A licensing decision is not an agent's to make** |
| **D27** | **What does the countdown count to** — signup closing (what the page's own ADR says) or the release (what you said)? | **Make it the release, and edit ADR-0076 to say so.** If 1.1.1 slips, ADR-0076 already commits to the honest fix: move the constant and say why |

**D21, D25 and D26 are the three that can move the date or the artefact.** The
rest have defaults good enough to build on.

---

## 2. The inventory, and why it comes first

**Days 1–3. Nothing else starts before it.**

Ask 1 cannot be scheduled until somebody has written down what does not work,
and **the record cannot be trusted for it** — §0a is what happens when it is.

### Five passes, each with a named method

| Pass | Method | Output |
| --- | --- | --- |
| **A. Every row, clicked** | `Drive-Window.ps1` over every row `every_menu()` returns, both surfaces, four fixtures — empty, plain text, a `.json`, a huge log. Not "does it dispatch"; **does the thing it names happen** | A table: row, what it promised, what it did |
| **B. Every row a person must click** | The `rfd` rows no script can reach — Save As, Export as .ipynb, Set as Default Editor, both passphrase ceremonies, cross-file search's folder picker. [NEXT_SESSION.md](NEXT_SESSION.md) §3 has the checklist; **six manual passes, six defects, never missed** | The same table, by hand |
| **C. Every claim in a doc comment** | Trap 3, mechanised: for each `///` asserting behaviour, ask *what test fails if this stops being true.* Where the answer is "none", write it or delete the claim | A list of wishes, converted or deleted |
| **D. Every axis nothing reads** | A policy axis, config field or capability nothing consults is a promise with no keeper. **`Policy::embeddings` is the known one** — two inspectors read it out and nothing computes an embedding | A list of inert promises |
| **E. The back-port survey** | Read downstream ADR-0057 to 0076 and each named commit. **Separate "decided for a reduced product" from "found a defect in shared code."** §0b's eight rows are the starting set, not the answer | One row per ADR: adopt, adapt, or reject with a reason |

### What the inventory is already known to find

Not a substitute for running it — this is what was visible on 2026-09-10
without clicking anything, so nobody spends day 1 rediscovering it.

| Thing | State | Disposition |
| --- | --- | --- |
| **Thirteen caret-gated rows** | Greyed in the default build | **Fix.** §0a. Week 1 |
| **CJK input under `--editor-view`** | Does not work; unfixable on 1.17.1 | **D21** — retire the flag, or ship a startup notice naming the limitation |
| **`Policy::embeddings`** | An axis nothing computes, read out by two inspectors | **Experimental**, behind E1 and a provider — or the readout says it governs nothing yet |
| **Clipboard rows decode their id against the history at click time** | Latent: the history can change between the menu opening and the click, and the row then pastes something else | **Fix.** The Run menu's `run-menu-opening` is the pattern — snapshot at open, dispatch against the snapshot |
| **Idle memory drifts ~20 MB to ~32 MB after dialogs** | Seen three times, never investigated, under the 50 MB target | **Measure, or accept with a number.** An unexplained leak in a release is a leak; a measured ceiling is a specification |
| **Drag and drop to open** | `specs.md` §4 wants it; Slint 1.17.1 cannot deliver it | **Delete the promise from `specs.md`**, with an ADR. A spec claiming what the toolkit refuses is trap 3 in a spec |
| **Warm start, 75 ms** | Never measured ([M1](WORK_QUEUE.md)) | **Measure, or change the target with an ADR.** A number nobody can check is not a target |
| **Print** | Not a row, no cross-platform story | **Stays absent.** Absent is not broken |
| **Markdown preview** | Declined, ADR-0048, and the reason holds | **Stays declined** — but see *Reading view*, §5a, which is most of what people wanted and needs no rich text |

**Nothing is removed before pass A has clicked it.** Removing a row because a
document says it is broken is how §0a's claim survived eight sessions.

---

## 3. The Experimental Features menu

**The vehicle for ask 4 and the safety valve for ask 1.** A fifteenth menu,
last before Help. Every row is a **toggle**, not an action: turning one on
changes what other menus contain, and turning it off puts the product back.

```
Experimental
  ------------------------------------------------
  These are unfinished. They may lose your work.        (readout)
  Nothing here is supported, and each row says
  what it might break.                                 (readout)
  ------------------------------------------------
  [ ] Multiple carets           may corrupt undo history
  [ ] Command palette           no known risk
  [ ] Reading view              read-only, no edits lost
  [ ] Semantic search           builds an index on disk
  [x] Session restore           reopens tabs from last run
  ------------------------------------------------
  Turn everything off                                  (action)
  Report a problem with an experiment...               (action, §4)
  ------------------------------------------------
  Three experiments are on.                            (readout)
```

### The seven rules it is built to

1. **Off by default, all of them, always.** A fresh install and a fresh
   profile are the supported product and nothing else.
2. **Every row names its risk in its own hint**, where the greyed rows already
   put their reasons. "Experimental" is not a risk statement; *"may corrupt
   undo history"* is.
3. **A flag that is on is visible without opening the menu** — status bar, and
   Help ▸ Diagnostics. **This is the rule that makes §4 work at all:** a ticket
   from a user running three experiments is a ticket about a different product,
   and the bundle must say so before anybody spends an hour on it.
4. **One row turns everything off**, reachable while the product misbehaves. An
   experiment that cannot be switched off is not an experiment.
5. **No experiment may write a format another build must read.** Nothing
   experimental touches `.bpadx`, the recovery journal, the security history,
   or `config.toml` outside its own table. This is what keeps ask 4's *"even if
   they are wrong"* from meaning *"and your documents go with them"*, and
   `what_an_earlier_build_wrote.rs` is what enforces it.
6. **No experiment relaxes a security profile.** A flag may add a capability;
   it may never widen `Policy`. `bp-security`'s monotonicity test is the
   backstop.
7. **An experiment graduates or dies at the next release.** A flag on the menu
   for two releases is a feature nobody finished; it goes, with an ADR, exactly
   as a planned row does under [ADR-0048](../docs/decisions/ADR-0048.md).

### Where it lives

- **`bp-config`** — an `[experimental]` table, one boolean per flag, unknown
  keys **reported and ignored** as an unknown setting already is. A flag
  removed in a later build must not stop the product loading.
- **A registry beside it** — id, label, risk sentence, default. **The menu is
  built from the registry**, so a flag cannot exist in code and be absent from
  the menu, or the reverse. Precedent: `Stamp::all()`, `Profile::all()` — a
  base plus an offset, never a hand-written list.
- **`menus.rs`** — `pub fn experimental(...)`, ids in a **new block at
  1000–1049**. Not 900: `RUN_CELL_BASE` is 800 bounded by
  `MAX_LISTED_CELLS = 200`, so it reaches 1000, and **950–999 is left as the
  gap that proves somebody checked.** Extend
  `no_menu_action_id_falls_inside_the_recent_files_window` and the range
  sweeps rather than working around them (R011 trap 1).
- **`every_menu()`** gains the menu **twice** — everything off, everything on.
  The Run menu was missing from that sweep from the day it was written and
  carried a 52-character label past a 42-character budget for four sessions; a
  menu whose rows depend on a value is not covered by building it one way
  ([ADR-0051](../docs/decisions/ADR-0051.md)).

---

## 4. The feedback system

**Ask 3.** The design follows from one constraint and one lesson.

The constraint is [ADR-0006](../docs/decisions/ADR-0006.md): nothing leaves the
machine without a per-use gesture ([ADR-0033](../docs/decisions/ADR-0033.md)).

The lesson is the one this product has already paid for: **a signing passphrase
was typed into the open document in plain text**, and 1,812 passing tests did
not see it. A feedback bundle is a machine for collecting exactly what that
defect produced. **Build it as though it will be pasted into a public issue
tracker, because it will be.**

### The shape

**Help ▸ Report a Problem…**, the row at the foot of the Experimental menu, and
a link from any error dialog.

1. **The user types what happened.** One box — the only free text.
2. **The product assembles a bundle**: `diagnostics_report()` plus what a
   triager needs —
   - version, commit, target, renderer, Slint version;
   - **every experimental flag that is on** (rule 3);
   - the security profile in force, and whether Privacy Mode is on;
   - the active document's **format, size, encoding and line ending** — never
     its path, never its text;
   - the last N status-bar lines, which are already written to be safe to
     quote (`bp-audit`'s passphrase module says so in as many words);
   - the last menu action, by **id and label**.
3. **The user sees the whole bundle, rendered, before anything happens.** Not a
   summary; not a checkbox reading "include diagnostics". The literal text.
4. **Then three buttons:** *Copy*, *Save as file…*, *Open in browser* (a
   pre-filled issue URL, per D20).

### The four things it must never do

- **Never include document text** — not a snippet, not a first line, not a hash
  of it. The log-hygiene gate stage (R011 trap 10) already fails a build for a
  `tracing` macro naming an identifier carrying document text; **point the same
  scan at the bundle builder.** That is the most important test in this
  subsystem.
- **Never include a path.** A path is a name, a project, an employer, sometimes
  a client. Report the *format* and the *size*.
- **Never send anything by itself**, including an update check.
- **Never run under a profile that forbids it without saying so.** Under
  Confidential, Maximum and Privacy Mode the bundle reduces to version and
  platform, and **the dialog names the fields withheld and why.** A bundle
  silently smaller than it looks is worse than a refusal.

### Where it lives

A new crate, **`bp-feedback`** — and **no new dependency**, because it composes
`bp-platform`'s capability report, `bp-config`'s resolved settings,
`bp-secrets`' scanner run over the bundle itself as a last line of defence, and
`bp-redaction`'s marker. **The scanner finding something in a bundle is a
build-failing test**, not a runtime warning.

**Testable without a window**, which is R011's one rule: the crate takes a
struct describing state and returns a `String`. `bp-ui` collects the struct,
shows the dialog, and does nothing else.

---

## 5. What gets built, and what gets refused

Ask 4 asked for breadth *including ideas that turn out to be wrong*. Here is
the catalogue, sorted by shelf. **The refusals are the part worth reading** — a
plan that refuses nothing has not been thought about.

### 5a. Ships in the product

| Feature | Why it belongs in the product rather than behind a flag |
| --- | --- |
| **The thirteen caret rows** | They are already in the menus. §0a |
| **Command palette** (Ctrl+Shift+P) | The action-id map **is** an inventory of every command, already written and already swept by tests. The cheapest large feature available, and it cannot go stale |
| **Session restore** | Last session's tabs. `recent.toml` exists; this is a sibling with the same guard. **Off under Confidential and Maximum**, like the recovery journal |
| **Auto-save timer** | Off by default. `bp-history` already holds a pending checkpoint |
| **Tab reordering by drag** | `DropArea` works for drags that *start inside* a Slint window — which is why file-drop is impossible and this is not. The distinction was in the record; nobody had noticed the good half |
| **Regex replace with capture groups** | The find bar already has a regex toggle. Replace probably does not honour `$1`; pass A finds out |
| **Bracket matching, auto-indent on newline** | `bp-editor` owns the commands; leading whitespace is a rope query |
| **Diff two open tabs** | Read-only, into the results panel Related Notes and cross-file search already share. No dependency — a line-based LCS is fifty testable lines |
| **Reading view** | Markdown with markers stripped and text reflowed. **No rich text, so Slint's refusal does not apply.** ADR-0048 declined *preview*; this is most of what a reader wanted, and it is representable |

### 5b. Ships behind the Experimental flag (may well be wrong)

| Flag | The risk on its own row | Why not in the product |
| --- | --- | --- |
| **Multiple carets** | *may corrupt undo history* | `bp-editor` must carry a set of carets through every command, selection and undo entry. Genuinely hard, genuinely wanted, and the failure mode is silent |
| **Semantic search** | *builds an index on disk* | [E1](WORK_QUEUE.md) plus a local provider; a dependency ADR and weights that never fetch themselves ([ADR-0056](../docs/decisions/ADR-0056.md)) |
| **Local assist** | *a model on your disk reads your document* | E2. Offline once present, absent when it is not |
| **Macro record and replay** | *replays commands, not intentions* | Over the `Command` stream. Almost certainly wrong in a way nobody predicts — which is what the shelf is for |
| **Split view** | *two views of one document may disagree* | Two panes over one rope. The interesting bug is scroll anchoring, not layout |
| **Typewriter scrolling, focus mode, word-count goal** | *changes how the window scrolls* | Cheap, divisive, perfect flag material |
| **Sticky heading** | *no known risk* | The current Markdown heading pinned above the text; `bp_semantic`'s outline already computes it |
| **Spell check** | *a dictionary is a dependency and a wordlist is an opinion* | Needs an ADR for both. Behind a flag until somebody has an opinion about which wordlist |
| **Export to PDF** | *layout is approximate* | The honest version of Print. Probably wrong; find out cheaply |

### 5c. Refused, with the reason

| Idea | Refused because |
| --- | --- |
| **Telemetry, usage analytics, crash auto-report** | ADR-0006 — and because §4 is the version of this that respects the user. **Any "anonymous" counter is a network call this product does not make** |
| **Cloud sync, an account, a licence check** | ADR-0006. An editor that stops working when a server does is a worse editor |
| **A plugin system or embedded scripting** | Arbitrary code execution wearing a hat, in a product whose notebook cells demand a `UserGesture` **by value** so ADR-0011 is enforced by the type system. A plugin API discards that in one commit |
| **AI writing assistance in the editor** | ADR-0006 is explicit and this is the thing it is about. `Embeddings` behind an explicit policy is the far edge of what this product does |
| **A "recover my passphrase" flow** | There is nothing to recover from. Composing vetted primitives correctly ([ADR-0011](../docs/decisions/ADR-0011.md)) means the passphrase is the only way in, and anything named this is a backdoor |
| **Self-signing the release** | [ADR-0055](../docs/decisions/ADR-0055.md), and downstream ADR-0067 agrees: it teaches a user to install a root certificate in order to run a text editor. **Worse than unsigned** |
| **Markdown preview with rich text** | Slint 1.17.1 has no styled runs. Reading view (5a) is the possible part |
| **File drag and drop to open** | The winit backend has no file-drop plumbing. **Delete the promise from `specs.md`** rather than carry it |
| **Run All in a notebook** | ADR-0038's model is a fresh subprocess per cell, so cell two cannot see cell one's variables. The name means something specific to everyone who has used a notebook, and it is not what this would do |

**Two of these are probably wrong and are refused anyway.** Export to PDF may
be genuinely wanted, and it is on the experimental shelf rather than the
product's; spell check may be table stakes for a text editor in 2026 and is
behind a flag because *which wordlist* is a decision nobody has made. Ask 4
said to include ideas that end up illogical: **§5b is where they go, and §5c is
the price of having a §5b at all.** A shelf for unproven work is only credible
if some things are refused outright.

---

## 6. The thirty days

**Each week ends green on both legs** — `./scripts/Invoke-LocalCI.ps1 -Linux`,
backgrounded, ~13 minutes, **never piped through `tail`**: a failure's detail
is above the summary.

### Week 1 (days 1–7) — find out what is true

| Day | Work | Who |
| --- | --- | --- |
| 1–3 | **The inventory**, passes A, C, D | agent |
| 1–2 | **Pass E, the back-port survey.** Nineteen downstream ADRs, one row each: adopt, adapt, or reject with a reason | agent |
| 2–3 | **Pass B, at the keyboard.** The `rfd` rows, and the typing/resize/wheel check unchecked since 2026-08-21 | **Daniel** |
| 3–5 | ~~**The caret work.**~~ **Landed 2026-09-10, [ADR-0080](../docs/decisions/ADR-0080.md).** `Buffer::{char_of_byte, byte_of_char}`, `Editor::{set_cursor_at_byte, select_bytes, bytes_of_chars}`, two `AppWindow` properties, `dispatch::{adopt_widget_caret, hand_caret_back}`, and the thirteen rows un-gated. **Plus a defect found on the way**: Find handed character ranges to a toolkit that selects by **bytes**, so it highlighted the wrong span on any document with a multi-byte character before the match | **done** |
| 5 | ~~**A test that fails loudly on a Slint upgrade.**~~ **Answered, and the answer beat a test.** An `out` property that stops existing is a **compile error in `app.slint`**, not a silent zero, so the gate's first stage is the guard. What got tests is our own conversion, in `bp-buffer`, where it needs no window | **done** |
| 6–7 | **The removals**, one ADR each. `specs.md` §4's drag-and-drop promise goes here | agent |
| 7 | **D21 answered** on the week's evidence rather than on a plan | **Daniel** |

**Week 1 contains no new features, deliberately.** Everything after it is sized
on what week 1 finds.

### Week 2 (days 8–14) — the two subsystems

| Day | Work |
| --- | --- |
| 8–9 | `bp-config`'s `[experimental]` table and the flag registry, unknown-key path tested |
| 9–11 | `menus::experimental`, the 1000–1049 block, `every_menu()` twice, the status-bar readout, Turn Everything Off |
| 11–14 | **`bp-feedback`** — the bundle, the profile-aware reduction, the secret scan over its own output, the dialog, and **the log-hygiene scan pointed at the bundle builder** |
| 14 | If D21 retired the flag: **the `--editor-view` removal.** A deletion, and it wants its own gate run |

### Week 3 (days 15–21) — what people will notice

| Day | Work |
| --- | --- |
| 15–16 | Command palette over the action map |
| 16–17 | Session restore; auto-save timer |
| 17–18 | Reading view; regex replace capture groups; bracket matching and auto-indent |
| 18–19 | Diff two tabs; tab reordering |
| 19–21 | The cheap half of the experimental shelf — sticky heading, typewriter scrolling, focus mode, word-count goal, split view. **Each ships with its risk sentence or it does not ship** |

**Week 3 is the one that gets cut** if weeks 1 and 2 overrun. Cut from the
bottom: nothing in §5b blocks a release, by construction.

### Week 4 (days 22–27) — make it a release

**Shorter than it would have been, because of §0b.**

| Day | Work | Who |
| --- | --- | --- |
| 22 | **Version to 1.1.1** (D17) — one line. Confirm `--version` and the archive name follow; the script reads the binary, so they should | agent |
| 22 | **The licence, settled** (D26) — the manifests, both texts in the archive, and what `--version` prints | **Daniel decides**, agent lands |
| 22–23 | **Back-port the icon**: ADR-0068's reasoning, both assets, and `b474291` — **the window icon must be the PNG, because Slint cannot decode an ICO**. The Windows build-time resource is a dependency, so **an ADR naming the crate and the reason** | agent |
| 23 | **Back-port ADR-0067** (no installer) and **ADR-0072** (macOS declined), adapted where Lite's reasoning was about a reduced product | agent |
| 23–24 | **D25** — hosted CI, or a recorded decision not to. This edits `CLAUDE.md` | **Daniel**, then agent |
| 24–25 | **Upgrade testing.** `what_an_earlier_build_wrote.rs` gains a fifth fixture: a `config.toml` carrying an `[experimental]` table a later build does not recognise. **Add beside, never edit** — those literals record what shipped |  agent |
| 25–26 | **M1** — measure warm start or change the 75 ms target with an ADR. The memory drift gets a number too | agent |
| 26–27 | **The full manual pass**, both platforms, every row, all of §5a by hand | **Daniel** |

### Days 28–30 — freeze and ship

| Day | Work | Who |
| --- | --- | --- |
| 28 | **Freeze. Documentation only.** README's current-state table, ROADMAP's statuses, MENU_MAP's every row, DECISIONS.md's lessons — including §0b's, which is the one this project will need again. **Six sentences in the record are known false today** and every one is corrected here, or the release ships a lie | agent |
| 29 | **Release candidate.** Build both archives, extract both, run both, and walk a first run on a machine with **no profile directory** | agent, then **Daniel** |
| 29 | **Open the PR into `main`** ([ADR-0053](../docs/decisions/ADR-0053.md)). PR #2 has been open since 2026-08-22 | agent opens, **Daniel merges** |
| 29 | **The website** — the download, the version, and **D27's countdown constant**. Downstream ADR-0076 keeps the page and the release in one repository so a link cannot name a version that does not exist; **this release must not break that property** | agent, **Daniel publishes** |
| 30 | **Tag, publish, announce.** The archives are unsigned and say so, every run, in the same words | **Daniel** |

---

## 7. What can move the date

| Risk | Likelihood | Cost | What we do |
| --- | --- | --- | --- |
| **The caret finding does not survive contact** — the property is stale when a row is clicked, or the byte offset maps badly across a rope | **Medium.** It compiles; that is not the same as being right at the moment of a click | Thirteen rows | **Found by day 5, not day 25.** If it fails, the thirteen rows are **removed** under ask 1 rather than fixed, and D21 answers itself the other way |
| **Pass B finds something like the passphrase defect** | **Medium.** Six passes, six defects, never missed | Unknown by definition | Week 3 is the buffer, and it is scheduled last for this reason |
| **The back-port survey finds Lite's ADRs contradict this product's scope** | **High, and it is fine.** Lite is a reduction; several of its decisions are right there and wrong here | A day of reading | Pass E's output is *adopt, adapt or reject with a reason* — a rejection is a result |
| **D26 (licence) goes the long way** | Medium | It ships in the artefact, so it cannot be deferred past day 22 | Escalate on day 8, not day 22 |
| **A Linux-only gate failure** | Low. One happened on 2026-08-22 and never again, and **its name is unknown because the output was truncated** | A day | **Keep the whole output** |
| **Thirty days is optimistic** | It is | — | **Cut week 3 from the bottom.** §5b and the back half of §5a are severable by construction |

**What does not move the date:** D4's outside review of the `.bpadx` envelope
([ADR-0050](../docs/decisions/ADR-0050.md)). It is prepared and needs a
reviewer, which no agent can be and no schedule can conjure. **1.1.1 must
therefore not claim the envelope is externally reviewed**, and the README must
say what it does claim: fourteen standard pitfalls self-audited, **four
conceded rather than mitigated**.

---

## 8. Definition of done for 1.1.1

From [DEFINITION_OF_DONE.md](../docs/governance/DEFINITION_OF_DONE.md), plus
what this release adds:

- [ ] **Every menu row does what it says in the build a user runs** — not in
      one behind a flag. Passes A and B both clean
- [ ] **Nothing greyed for a missing decision.** Greyed for a missing
      *prerequisite* is fine, and says so on the row
- [ ] **Every experimental flag off in a fresh profile**, asserted by a test
- [ ] **A feedback bundle contains no document text and no path**, enforced at
      build time by the log-hygiene scan
- [ ] **`what_an_earlier_build_wrote.rs` covers five files**, including an
      `[experimental]` table from an unknown build
- [ ] **The record is true** — every sentence §0 disproved is corrected, and
      §0b's lesson is in `DECISIONS.md`
- [ ] **The licence in the manifests, the archive and `--version` agree** (D26)
- [ ] **Both archives extracted and run** on a machine with no profile
      directory
- [ ] **The website names a version the release actually produced**
- [ ] **Green on both legs**, whole output kept
- [ ] **Merged to `main` by a human** (ADR-0053)

---

## 9. What 1.1.1 is deliberately not

Named so nobody reads an absence as an oversight, and so 1.2 has a start:

- **Signed binaries** (D22). Start the certificate now if you want them
- **An installer** — downstream ADR-0067's answer, adopted
- **Semantic search in the product.** On the experimental shelf; E1's store
  work is what 1.2 builds on
- **Multi-caret in the product.** Same shelf, and the honest place for it
- **Print** — platform work with no cross-platform story. Export to PDF is the
  experiment that finds out whether anybody wanted it
- **macOS** — declined, with the cost written down (downstream ADR-0072)
- **An externally reviewed envelope.** Prepared, undischarged, and 1.1.1 says
  so rather than implying otherwise
