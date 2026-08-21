# Decisions waiting on a human

The standing queue of questions only Daniel can answer. **Kept current as work
happens**, so that a session can open by asking everything at once rather than
stopping halfway through to ask one thing.

Why this file exists: across four sessions, work has been blocked by decisions
far more often than by effort. Four open questions gated three sessions; they
were answered in a single message and unblocked a day of work. The bottleneck
is not how fast code gets written, it is how long a question sits.

## How this is used

- **Anything here blocks something.** If a question stops mattering, delete it
  rather than leaving it to be re-read.
- **Before a long session, everything here is asked in one batch.** Not one at
  a time as each is reached.
- **An answer goes into the ADR or the doc it belongs in immediately**, and
  the row is deleted. This file is a queue, not a record — `DECISIONS.md` is
  the record.
- A question that turns out to have an obvious default is not a question. Pick
  the default, say so, and move on.

## Open

| # | Question | Blocks | Why it cannot be defaulted |
| --- | --- | --- | --- |
| D2 | **Is `--editor-view`'s two-view fork worth keeping indefinitely?** IME cannot work on Slint 1.17.1, so parity is unreachable until Slint changes. | Every future wiring item pays a small tax; five features work in only one view | Answered once already (keep opt-in). Re-ask only if the tax becomes visible, or Slint ships composition support |

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
