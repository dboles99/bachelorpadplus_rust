# Decisions waiting on a human

The standing queue of questions only Daniel can answer.

**Why this file exists:** across five sessions, work has been blocked by
decisions far more often than by effort. Four open questions once gated three
sessions; they were answered in a single message and unblocked a day of work.
The bottleneck is not how fast code gets written, it is how long a question
sits.

---

## The batch — ask this first, verbatim, then start work anyway

**This section is written to be pasted into a message.** `CLAUDE.md`'s "How a
session opens" makes asking it the first thing a session does, because the
reason these rows sit unanswered is that a session opens by *reading* rather
than by *asking*. Do not wait for the answer before starting; do not stop
halfway through an item to ask one thing.

> **Eleven rows opened on 2026-09-10**, all of them from
> [V1_1_1_PLAN.md](V1_1_1_PLAN.md) -- a thirty-day plan to ship **v1.1.1 on
> 2026-10-10**. **Every one has a default in the plan's section 1**, so silence
> is an answer to eight of them and work does not stop.
>
> **Three cannot be defaulted. All are now answered:**
>
> - **D25 -- hosted CI?** Answered yes 2026-10-29. Actions enabled via API.
> - **D26 -- which licence?** Answered. All manifests updated to GPL-3.0-only.
> - **D21 -- does `--editor-view` survive?** Answered: retire the flag, keep the surface.
>
> **D18 (experimental flags) is also answered:** persist via config file and environment, displayed in status bar and Help>Diagnostics.
> **D19/D20 (tickets) are also answered:** Report Issue menu opens pre-filled GitHub URL.
> **D22 (unsigned) is also answered:** README documents Windows SmartScreen click-through.
> **D27 is also answered:** the countdown counts to the release ([ADR-0089](../docs/decisions/ADR-0089.md)).
>
> **Two rows opened 2026-10-29, both about the merge D13 gates, and neither can
> be defaulted:**
>
> - **D30 -- what is the merge plan?** The branch and `main` have *diverged* (90
>   and 109 commits each); `main` also has no `LICENSE` and says
>   `MIT OR Apache-2.0` while the branch is GPL-3.0-only.
> - **D31 -- after Actions is paid for again, does `main`'s gate script and
>   `ci.yml` replace the branch's?** Proposed: yes.

## How this is used

- **Anything here blocks something, or will.** If a question stops mattering,
  delete it rather than leaving it to be re-read.
- **An answer goes into the ADR it belongs in immediately**, and the row is
  deleted from both the batch above and the table below. This file is a queue,
  not a record — `DECISIONS.md` is the record.
- A question that turns out to have an obvious default is not a question. Pick
  the default, say so, and move on.
- **Keep the batch and the table in step.** They are the same questions
  written twice, once to ask and once to reason about, and a batch that has
  drifted from the table is worse than no batch.

## Open

**All eleven rows from the 2026-09-10 v1.1.1 batch are now answered or done.**
Full reasoning for each in [V1_1_1_PLAN.md](V1_1_1_PLAN.md) section 1.

Remaining items below are from earlier plans and kept for context.

| # | Question | Default in the plan | Whose |
| --- | --- | --- | --- |
| D17 | ~~`1.1.1` from `0.1.0`, with nothing ever shipped?~~ | **Done.** Version bumped to 1.1.1 in workspace and all crates | -- |
| D18 | ~~Does an experimental flag survive a restart?~~ | **Done.** Persist via config file and environment variable, displayed in status bar and Help>Diagnostics | -- |
| D19 | ~~Does a ticket ever leave the machine by itself?~~ | **Done.** No, and never. Report Issue menu opens browser with pre-filled URL | -- |
| D20 | ~~Where do tickets go?~~ | **Done.** Pre-filled GitHub issue URL via Help>Report an Issue | -- |
| D21 | ~~Does `--editor-view` survive 1.1.1?~~ | **Done.** Flag retired, custom surface is now default. CJK input limitation notice added ([D21_D25_FINDINGS.md](../project/D21_D25_FINDINGS.md)) | -- |
| D22 | ~~Unsigned at 1.1.1?~~ | **Done.** README documents Windows SmartScreen click-through ([ADR-0055](../docs/decisions/ADR-0055.md)) | -- |
| D23 | ~~The icon~~ | **Done.** ADR-0086: icon ships beside the executable; PNG for Slint window icon | -- |
| D24 | ~~Installer or zip?~~ | **Done.** ADR-0090: no installer, zip archive only | -- |
| D25 | ~~Hosted CI, against ADR-0016?~~ | **Answered yes.** Actions enabled via API, scoped to pull requests and main ([ADR-0082](../docs/decisions/ADR-0082.md)). Repository setting flipped 2026-10-29 | -- |
| D26 | ~~`MIT OR Apache-2.0`, or GPL-3.0-only?~~ **Answered.** All manifests updated to GPL-3.0-only 2026-10-29 ([ADR-0081](../docs/decisions/ADR-0081.md)) | -- |
| D29 | ~~There is no `THIRD-PARTY-NOTICES.md`.~~ | **Done.** File generated, CI gate in place | -- |
| D27 | ~~What does the countdown count to?~~ | **Answered.** The release ([ADR-0089](../docs/decisions/ADR-0089.md)) | -- |
| D28 | ~~ADR numbering collides with the derived repository~~ | **Answered 2026-09-10 and closed the same day.** This sequence continues at **0080**; 0057-0079 are deliberately unused, because `bachelorpluslite` holds those numbers for different decisions and this project's records link to several of them. Recorded in [ADR-0080](../docs/decisions/ADR-0080.md) itself, on the far side of the gap, rather than left for somebody to notice | -- |
| D30 | What is the merge plan for D13, now the branch and `main` have diverged? | **No safe default.** `main` has 109 commits the branch lacks and the branch 90 that `main` lacks; a trial merge conflicts in dozens of files; `main` has no `LICENSE` and declares `MIT OR Apache-2.0` against the branch's GPL-3.0-only ([ADR-0081](../docs/decisions/ADR-0081.md)), in public. Proposed: merge `main` *into* the branch, resolve there, then one pull request | Daniel |
| D31 | After Actions is paid for again, does `main`'s gate script and `ci.yml` replace the branch's? | **Proposed: yes.** `main`'s ran on both OSes (xvfb launch, `cargo deny`) and failed only on a real advisory; the branch script's `launch` stage hard-codes `bachelorpad.exe` ([ADR-0091](../docs/decisions/ADR-0091.md)) | Daniel |

**Twelve rows, not eleven.** D28 was raised and answered inside one session, so
it is listed above already closed rather than given a turn in the queue -- but it
is listed, because a decision that is only ever mentioned in a commit message is
a decision the register does not have.

**Two of the twelve were answered by looking rather than by asking**, which is
trap 8 arriving from a new direction: **D23 and D24 were both already decided,
with the reasoning written and the code shipped, in the repository this one
spawned twelve days earlier.** [V1_1_1_PLAN.md](V1_1_1_PLAN.md) section 0b has
the rest of what was found there -- nineteen ADRs, six of which answer questions
this file or `WORK_QUEUE.md` was still carrying.

**The lesson is the one worth keeping**, and it is in `DECISIONS.md`: *a derived
repository is a second reader of every decision this one made, and it re-checks
premises for free.* `NEXT_SESSION.md` recorded the clone as **inheriting** this
project's formats. The inheritance ran the other way too, and went unread.

## Answered the day it emptied, kept for the shape

**Empty from 2026-08-23 to 2026-09-10.** Both rows lasted less than a day, and
the second of them is the more interesting.

| # | Question | Answer | Where it landed |
| --- | --- | --- | --- |
| D15 | Buy a code-signing certificate, ship unsigned, or defer? | **Defer**, and refuse self-signing outright -- it is worse than unsigned, because it teaches a user to install a root certificate to run a text editor. Build the archive, the checksums and a signing step that is one command | [ADR-0055](../docs/decisions/ADR-0055.md) |
| D16 | Local model, opt-in network, or drop phase 10? | **All three, as choices** -- and looking found the choice already existed. `bp_security::Embeddings` has carried `Cloud`/`Local`/`None` since ADR-0020, with a per-profile mapping and a doc comment reading *"Phase 10 reads this"*. What was actually undecided was smaller: the profile is a ceiling, `None` is the default, and `Cloud` needs a per-use gesture on top of a policy that permits it | [ADR-0056](../docs/decisions/ADR-0056.md) |

**D16 is worth one more sentence than its row.** It sat for two sessions as
"the largest unasked question in the project" while its answer was in
`crates/bp-security/src/lib.rs`. The habit that catches that is trap 6 pointed
the other way: **before asking a question, grep for its answer.** A decision
recorded in a type is still a decision, and it is the kind least likely to be
re-read, because nothing about a queue of questions suggests looking in
`src/`.

## Answered on 2026-08-22, kept one more session

The last two rows before the file emptied:

| # | Question | Answer | Where it landed |
| --- | --- | --- | --- |
| D2 | Is `--editor-view`'s two-view fork worth keeping indefinitely? | **Keep it opt-in, unchanged.** Neither re-ask condition has been met: composition is still impossible on Slint 1.17.1, and the tax *fell* from five features to four | [ADR-0018](../docs/decisions/ADR-0018.md), amended |
| D13 | Does `main` get this work, and by PR again? | **Yes, by pull request.** A fast-forward was never available; the merge commit sits on `main` alone. Opening the PR is an agent's job, clicking merge is not | [ADR-0053](../docs/decisions/ADR-0053.md) |

**The queue emptied on 2026-08-22, refilled on 2026-08-23, and emptied again
the same day** -- and the cycle is the useful part, not the state at either
end. The answer to "what do you do when there are no questions left" is:
**take the work whose shape is known, and it will produce the next question.**
Phase 20 was the only *Not started* phase not blocked on a decision (ADR-0053
said so); starting it produced D15 within an afternoon, and D15 took one
message to answer.

**An empty queue is not an idle state and never has been.** Both times this
file has emptied, `WORK_QUEUE.md` had work in it that needed no decision at
all.

## Answered, for context

Kept briefly so a later session does not re-ask. Full reasoning lives in the
ADRs.

| Question | Answer | Where it landed |
| --- | --- | --- |
| D14: what should Markdown preview be? | **Declined, with the reason.** Slint 1.17.1 has no rich-text item, so inline formatting is not representable and a preview that dropped it would be worse than none; HTML in the browser means a plaintext temp file, which `Policy::temporary_files` forbids for a Confidential document. Revisit if Slint ships styled text | ADR-0048 |
| Custom cryptography — compose or implement? | Compose vetted primitives; never implement them | ADR-0011, clarified |
| D6: what is a line? | `\n` or `\r\n` and nothing else; the rope drops `unicode_lines`, which was ropey's default and never a choice | ADR-0029 |
| D1: what may leave the machine? | OpenAI's API, opt-in per use, never automatic. Does not authorise `bp-research` -- that stays offline under ADR-0006 | ADR-0033 |
| D4: outside review of `bp-crypto`'s envelope before trusting real documents? | Yes, before phase 15 trusts real documents to it. **Prepared, not discharged** -- the brief, the self-audit and the ten questions are written; a reviewer has still to be found | ADR-0050, `docs/architecture/BPADX_ENVELOPE_REVIEW.md` |
| D3: what does `bp-storage` record, and what does the user see? | Both a related-notes panel and duplicate detection, on the schema ADR-0019 already shipped | ADR-0037 |
| bp-storage's presentation: how do Related Notes and Duplicate Detection actually behave? | Related Notes opens a collapsible panel, the same pattern as cross-file search results. Duplicate Detection is both automatic (a save-time notice) and on-demand (the menu row) | ADR-0037 |
| D7: should `bp-crypto` expose a derive-once API? | No -- each line's salt is independently random by design, so no single derived key opens more than one line regardless of the API. The actual fix is reading `AuditLog` lazily, not a faster key | ADR-0036 |
| D8: how does Windows default-editor registration get written? | Confirmed as already shipped: a `.reg` handoff plus `ms-settings:defaultapps`, never a `winreg`/`unsafe` dependency | ADR-0034 |
| D10: where does the `.bpadx` validation ceiling sit? | Lowered from 1 GiB/64 iterations to 256 MiB/16 iterations -- still 13x/8x the OWASP-baseline default | ADR-0035 |
| Notebook mode: what is it? | A lightweight utility -- cells run one at a time, on request, no persistent kernel session | ADR-0038 |
| Research mode: what is it for? | Synthesising gathered data into design insights and recommendations, offline | ADR-0039 |
| D12: is `bp-research`'s bibliography model the right foundation for research mode? | No -- a new data model, separate from it. `bp-research` stays scholarly citation infrastructure, untouched | ADR-0039 |
| ADR-0036's lazy `AuditLog` read: what shape? | An iterator as the one primitive; `bp-ui` drives it two ways -- paged for ordinary scrolling, background-decrypt-with-progress for whole-history operations (export, full-history search) | ADR-0036 |
| `.bpadx` AEAD | Both XChaCha20-Poly1305 and AES-256-GCM, versioned in the envelope | ADR-0021 |
| `.bpadx` envelope shape | Chunked from the start, chunk index bound into the AAD | ADR-0021 |
| Argon2id cost | OWASP baseline (19 MiB, t=2, p=1), recorded per document | ADR-0021 |
| Platform key protection (DPAPI/Hello/keyring) | Phase 16, not 15 | ADR-0021 |
| Security profiles | All of them; profile model first | ADR-0020 |
| YAML parser | `saphyr` | Phase 6, when it is taken |
| Parity vs retiring the custom editor view (D2) | Keep it opt-in; re-asked and reaffirmed 2026-08-22, with the tax measured and falling | ADR-0018, amended |
| D11: may a huge document be viewable only under `--editor-view`? | No — it opens in the custom surface, read-only, in *every* build. The flag is about editing, and the reason it is opt-in (IME) has nothing to say about a read-only view | ADR-0030 |
| D5: where does a signing key live? | Sealed in a `.bpadx` envelope under a passphrase. Contents protected rather than permissions, which closes the Windows/Linux asymmetry ADR-0026 measured | ADR-0031 |
| D9: `bachelorpad` or reverse-DNS? | `io.github.dboles99.BachelorPadPlus`, and it stops being the same constant as `APP_DIR` | ADR-0032 |
| D3: may `bp-storage` be wired in? | Yes, *after* a design pass. ADR-0019 stands as written | this file, narrowed |
