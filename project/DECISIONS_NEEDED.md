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

> Two decisions are open, neither blocking today's work:
>
> 1. **Does `main` get the last 49 commits, and by PR again?** `main` already
>    holds this work up to `31f6858`, "Merge phases 1-19 into main (PR #1)";
>    the branch has run 49 commits past it since. So the established pattern
>    is a pull request, and the question is only whether to open the second
>    one now or keep going. **A fast-forward is not available** — the merge
>    commit is on `main` and not on the branch. **(D13, five sessions old,
>    and re-stated on 2026-08-22: the record had it wrong — see below.)**
> 2. **Is `--editor-view`'s two-view fork still worth keeping?** Answered once
>    as "keep it opt-in". Four features work in only one view, down from five.
>    IME cannot work on Slint 1.17.1, so parity is unreachable until Slint
>    changes. **(D2 — re-ask only if the tax has become visible.)**
>
> Not a question, and no longer unstarted: **D4, an outside review of
> `bp-crypto`'s `.bpadx` envelope**. The brief is written --
> [`docs/architecture/BPADX_ENVELOPE_REVIEW.md`](../docs/architecture/BPADX_ENVELOPE_REVIEW.md),
> ADR-0050 -- so what remains is **finding a reviewer**, which is the half only
> you can do. It is an hour of somebody's time now, not a project. Every
> primitive is vetted RustCrypto; the *composition* is this project's own
> design, which is exactly what an author cannot catch by re-reading their own
> work.

---

## How this is used

- **Anything here blocks something, or will.** If a question stops mattering,
  delete it rather than leaving it to be re-read.
- **An answer goes into the ADR it belongs in immediately**, and the row is
  deleted from both the batch above and the table below. This file is a queue,
  not a record — `DECISIONS.md` is the record.
- A question that turns out to have an obvious default is not a question. Pick
  the default, say so, and move on.
- **Keep the batch and the table in step.** They are the same three questions
  written twice, once to ask and once to reason about, and a batch that has
  drifted from the table is worse than no batch.

## Open

| # | Question | Blocks | Why it cannot be defaulted |
| --- | --- | --- | --- |
| D2 | **Is `--editor-view`'s two-view fork worth keeping indefinitely?** IME cannot work on Slint 1.17.1, so parity is unreachable until Slint changes. | Every future wiring item pays a small tax; **four** features work in only one view, down from five | Answered once already (keep opt-in). Re-ask only if the tax becomes visible, or Slint ships composition support. **The count fell on 2026-08-22** and the reason is worth weighing before re-asking: Go to Line was on that list by mistake, needing the caret *moved* rather than *read*. The remaining four genuinely need it read |
| D13 | **Does `main` get the last 49 commits, and by PR again?** `main` is at `31f6858` — "Merge phases 1-19 into main (PR #1)" — so it already holds this work up to that point; the branch has run 49 commits past it. **The premise was re-checked on 2026-08-22 before the question was re-asked**, which is the habit trap 6 exists to install: `git log --oneline main..HEAD` counts 49 and `git merge-base --is-ancestor main HEAD` is false. | Nothing today. Everything, the first time a clone is made from `main` | **The record had this wrong for five sessions and nobody checked.** It said "111 commits live on the branch; `main` has none of them", which is false: `git merge-base --is-ancestor` puts most of the branch's history inside `main` already. Corrected 2026-08-22. What remains is genuinely a choice — open PR #2 now, or keep accumulating and merge at a milestone — but it is a much smaller one than the framing implied, and a fast-forward is simply unavailable because the merge commit sits on `main` alone |

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
| Parity vs retiring the custom editor view | Keep it opt-in, revisit on a Slint upgrade | ROADMAP, WORK_QUEUE |
| D11: may a huge document be viewable only under `--editor-view`? | No — it opens in the custom surface, read-only, in *every* build. The flag is about editing, and the reason it is opt-in (IME) has nothing to say about a read-only view | ADR-0030 |
| D5: where does a signing key live? | Sealed in a `.bpadx` envelope under a passphrase. Contents protected rather than permissions, which closes the Windows/Linux asymmetry ADR-0026 measured | ADR-0031 |
| D9: `bachelorpad` or reverse-DNS? | `io.github.dboles99.BachelorPadPlus`, and it stops being the same constant as `APP_DIR` | ADR-0032 |
| D3: may `bp-storage` be wired in? | Yes, *after* a design pass. ADR-0019 stands as written | this file, narrowed |
