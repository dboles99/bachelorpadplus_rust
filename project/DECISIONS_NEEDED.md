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

> **Two questions, and the first one is small.**
>
> **D15 — signing needs a certificate, and only you can get one.** Phase 20
> started on 2026-08-23 ([ADR-0054](../docs/decisions/ADR-0054.md)) and the
> artefact half of it is ordinary work except for this. A self-signed
> certificate makes Windows SmartScreen *worse*, not better, so the real
> options are: **buy an OV or EV code-signing certificate** (a few hundred a
> year, and EV needs a hardware token), **ship unsigned** and accept the
> SmartScreen warning on every download, or **defer signing** and package
> unsigned for now with the rest built so signing is one step when a
> certificate exists. Linux is unaffected -- a detached signature and a
> checksum need no authority. Which?
>
> **D16 — phase 10 needs embeddings, and nothing here computes one.** This is
> the question `DECISIONS_NEEDED.md` said on 2026-08-22 had not been asked
> yet, and it is the last genuinely large unscoped thing in the product.
> Semantic search needs a vector per document. Computing one either runs a
> **model locally** -- a large dependency, a model file to ship or download,
> and a first-run cost, but ADR-0006 holds untouched -- or sends document
> text to a **network service**, which ADR-0033 permits only opt-in per use
> and never automatically, and which ADR-0006 forbids for anything core.
> A third answer is real: **drop phase 10** and let the lexical search that
> already ships be the search this product has. Which?

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

**Two rows, opened 2026-08-23.** The file was empty for one day, which is
exactly as long as it took to find work whose next step is a decision.

| # | Question | Why it blocks | Default if you would rather not decide |
| --- | --- | --- | --- |
| D15 | Buy a code-signing certificate, ship unsigned, or defer? | The rest of phase 20's artefact work can proceed either way, but the *release* cannot be described honestly until this is answered -- "signed" and "unsigned" are different products to a Windows user | **Defer.** Build the archive, the checksums and the Linux detached signature; leave a signing step that is one command when a certificate exists. Nothing is wasted and nothing is claimed |
| D16 | Local model, opt-in network, or drop phase 10? | Phase 10 cannot be sized at all until it is answered. This is the "undecided reads like large" pattern in `DECISIONS.md`, and it has caught this project four times | **None.** This one genuinely needs you: it is a decision about what the product *is*, and all three answers are defensible |

## Answered on 2026-08-22, kept one more session

The last two rows before the file emptied:

| # | Question | Answer | Where it landed |
| --- | --- | --- | --- |
| D2 | Is `--editor-view`'s two-view fork worth keeping indefinitely? | **Keep it opt-in, unchanged.** Neither re-ask condition has been met: composition is still impossible on Slint 1.17.1, and the tax *fell* from five features to four | [ADR-0018](../docs/decisions/ADR-0018.md), amended |
| D13 | Does `main` get this work, and by PR again? | **Yes, by pull request.** A fast-forward was never available; the merge commit sits on `main` alone. Opening the PR is an agent's job, clicking merge is not | [ADR-0053](../docs/decisions/ADR-0053.md) |

**The queue emptied on 2026-08-22 and refilled on 2026-08-23**, and the
mechanism is worth naming because it is the answer to "what do you do when
there are no questions left": **take the work whose shape is known, and it
will produce the next question.** Phase 20 was the only *Not started* phase
not blocked on a decision (ADR-0053 said so); starting it produced D15 within
an afternoon. D16 was not found this way -- it was already named as unasked --
but it is now written where it can be answered rather than described.

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
