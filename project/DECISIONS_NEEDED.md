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
> **Three cannot be defaulted, because none is an agent's to make:**
>
> - **D25 -- hosted CI?** The downstream repository restored it
>   ([ADR-0073 there](../../bachelorpluslite/docs/decisions/ADR-0073.md)) on the
>   grounds that **ADR-0016's premise was never re-checked**. `CLAUDE.md` and
>   R011 both still assert there is none, so adopting it edits the entry point.
> - **D26 -- which licence?** Every manifest here claims `MIT OR Apache-2.0`;
>   the repository derived from this one is **GPL-3.0-only**. `--version`
>   prints it and the archive carries the texts, so a wrong answer ships in the
>   artefact. **Needed by day 22.**
> - **D21 -- does `--editor-view` survive?** *Do not answer before day 7.* The
>   evidence arrives then; answering now is answering on a claim.
>
> **And one is a public-facing contradiction**: D27, what the website's
> countdown counts to. Its own ADR says early-access signup closing; the
> instruction that produced the plan says the release.
>
> D4 is still prepared and needs a reviewer rather than a decision
> ([ADR-0050](../docs/decisions/ADR-0050.md)).

---

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

**Eleven rows, opened 2026-09-10.** Full reasoning, and a default for each, in
[V1_1_1_PLAN.md](V1_1_1_PLAN.md) section 1. Kept short here because the plan
owns the argument and this file owns the queue.

| # | Question | Default in the plan | Whose |
| --- | --- | --- | --- |
| D17 | `1.1.1` from `0.1.0`, with nothing ever shipped? | Take `1.1.1`; record why it starts there | either |
| D18 | Does an experimental flag survive a restart? | Persist, and show active flags in the status bar | either |
| D19 | Does a ticket ever leave the machine by itself? | **No, and never** | either |
| D20 | Where do tickets go? | A pre-filled GitHub issue URL | Daniel |
| D21 | Does `--editor-view` survive 1.1.1? | Retire the flag, keep the surface. **Not before day 7** | Daniel |
| D22 | Unsigned at 1.1.1? | Ship unsigned and say so; start a certificate now if you want one for 1.2 | Daniel |
| D23 | ~~The icon~~ | **Answered by looking.** It exists downstream, and so does the reason it must be a PNG | -- |
| D24 | ~~Installer or zip?~~ | **Answered by looking.** Downstream ADR-0067: no installer | -- |
| D25 | Hosted CI, against ADR-0016? | Read ADR-0073 there and adopt it. **Edits `CLAUDE.md`** | **Daniel** |
| D26 | ~~`MIT OR Apache-2.0`, or GPL-3.0-only?~~ **The question had a false premise.** Slint is `GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0` -- **no permissive option** -- so `MIT OR Apache-2.0` was never available and proprietary needs a Slint licence bought or registered. The real question is which of three routes ([ADR-0081](../docs/decisions/ADR-0081.md)) | **Answered in part:** the MIT claim must not ship, settled. **Daniel is looking into the Slint licence**, 2026-09-10, and the route waits on that. Meanwhile the defensible string is GPL-3.0-only | **Daniel** |
| D29 | **There is no `THIRD-PARTY-NOTICES.md`.** ~620 crates, mostly MIT or Apache-2.0, all requiring attribution when a binary is distributed | **Not a decision, a missing artefact** -- and it blocks a release under *every* D26 route, so it does not wait on D26. The derived repository generates one from a script with a gate stage that fails on drift; back-port that. Needs a dev tool (`cargo-about` or `cargo-license`), so it carries a dependency ADR | either |
| D27 | What does the countdown count to? | The release, and ADR-0076 is edited to say so | **Daniel** |
| D28 | ~~ADR numbering collides with the derived repository~~ | **Answered 2026-09-10 and closed the same day.** This sequence continues at **0080**; 0057-0079 are deliberately unused, because `bachelorpluslite` holds those numbers for different decisions and this project's records link to several of them. Recorded in [ADR-0080](../docs/decisions/ADR-0080.md) itself, on the far side of the gap, rather than left for somebody to notice | -- |

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
