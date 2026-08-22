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

> Three decisions are open, none of them blocking today's work:
>
> 1. **Does `main` get this product, and how?** ~120 commits live on
>    `feat/phase-01-foundation-ui`; `main` has none of them. Fast-forward
>    (keeps every commit, admits the branch name was wrong), squash (clean
>    history, loses a record this project has been unusually careful to
>    build), or leave it — which is also a decision and should be a stated
>    one. **(D13, five sessions old.)**
> 2. **What should Markdown preview be?** Slint has no HTML view. Render to
>    styled Slint elements (real work, full control), shell out to the system
>    browser (leaves the app), or decline permanently and say so — which is
>    honest if a `.md` is meant to be read as its own source. **(D14.)**
> 3. **Is `--editor-view`'s two-view fork still worth keeping?** Answered once
>    as "keep it opt-in". Four features work in only one view, down from five.
>    IME cannot work on Slint 1.17.1, so parity is unreachable until Slint
>    changes. **(D2 — re-ask only if the tax has become visible.)**
>
> Not a question, but the largest unstarted work in the project: **D4, an
> outside review of `bp-crypto`'s `.bpadx` envelope**, answered yes five
> sessions ago and never done. Every primitive is vetted RustCrypto; the
> *composition* is this project's own design, which is exactly what an author
> cannot catch by re-reading their own work.

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
| D13 | **Does `main` get this product, and how?** 111 commits live on `feat/phase-01-foundation-ui`; `main` has none of them. | Nothing today. Everything, the first time a clone is made from `main`, or the branch is lost | `README.md`, `ROADMAP.md` and this project's own housekeeping have called it "the item most likely to hurt and least likely to be noticed" for four sessions. It is not defaultable because the options differ in what they say: a fast-forward merge keeps every commit and admits the branch name was wrong; a squash makes `main` a clean history and loses a record this project has been unusually careful to build; leaving it is a decision too, and should be a stated one rather than a habit |
| D14 | **What should Markdown preview be?** Slint has no HTML view. | `MENU_MAP.md`'s preview row, and how far the literate-document work of ADR-0043/0045 can go | Three shapes, none obviously right: render to styled Slint elements (real work, no HTML, full control); shell out to the system browser (leaves the app, and ADR-0006 says core editing works offline — a local file is not a network call, but it is a second window); or decline it permanently and say so, which is honest if a `.md` is meant to be read as its own source. ADR-0045 made a `.md` more useful without touching this, which makes the question less urgent and no less open |

## Answered, for context

Kept briefly so a later session does not re-ask. Full reasoning lives in the
ADRs.

| Question | Answer | Where it landed |
| --- | --- | --- |
| Custom cryptography — compose or implement? | Compose vetted primitives; never implement them | ADR-0011, clarified |
| D6: what is a line? | `\n` or `\r\n` and nothing else; the rope drops `unicode_lines`, which was ropey's default and never a choice | ADR-0029 |
| D1: what may leave the machine? | OpenAI's API, opt-in per use, never automatic. Does not authorise `bp-research` -- that stays offline under ADR-0006 | ADR-0033 |
| D4: outside review of `bp-crypto`'s envelope before trusting real documents? | Yes, before phase 15 trusts real documents to it. Not yet done -- see `project/NEXT_SESSION.md`'s housekeeping | this file, and NEXT_SESSION.md |
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
