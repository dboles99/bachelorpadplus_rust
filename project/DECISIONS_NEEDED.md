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
| D1 | **What may leave the machine?** Which providers, if any, are acceptable destinations for document content or embeddings. | Phases 8 and 10; `Network::Allowed` currently means only "this profile does not forbid it" | Baking in a provider list would be an answer nobody gave, about data leaving the user's computer |
| D2 | **Is `--editor-view`'s two-view fork worth keeping indefinitely?** IME cannot work on Slint 1.17.1, so parity is unreachable until Slint changes. | Every future wiring item pays a small tax; five features work in only one view | Answered once already (keep opt-in). Re-ask only if the tax becomes visible, or Slint ships composition support |
| D4 | **Should `bp-crypto`'s envelope get an outside review before anything real is encrypted with it?** Every primitive is composed from RustCrypto and nothing is reimplemented, but the envelope design is mine, and envelope design is where composed-primitive formats usually fail. | Trusting `.bpadx` with real documents | Not something the author of the design can answer by looking harder at it |
| D3 | **May `bp-storage` be wired into the application?** ADR-0020 permits `Metadata::Summary` under Standard, so the security half is settled. What remains is a product decision plus a design pass. | Phase 9, `bp-organize` | ADR-0019 explicitly says not to "just wire it up" |
| D6 | **What is a line?** `bp-search` counts `\n`; `bp-buffer`'s rope also breaks on bare `\r`, `\x0B`, `\x0C`, U+0085, U+2028 and U+2029, because `ropey`'s `unicode_lines` is on by default. `bp-ui` uses both, in different views, so one document reports two line counts. | Find-in-files scrolling to the right line; the status bar agreeing with itself | Either answer is defensible and each changes observable behaviour in the other direction. Found by the cross-crate tests; pinned as an ignored test |
| D7 | **Should `bp-crypto` expose a derive-once API?** Today `derive` and `Key` are private, so every `seal` call pays a full Argon2id derivation. `bp-audit` seals one `.bpadx` envelope per line, so *reading* a sealed audit history costs ~45 ms per line — a few hundred lines is several seconds, and it grows over a document's life. | The security inspector staying usable on a long-lived document | Splitting `derive` from `seal_with_key` changes ADR-0021's surface, and the alternative — a cheaper KDF in `bp-audit` — would weaken every line against an offline attacker, which ADR-0020 forbids. Not an implementation detail |
| D10 | **A `.bpadx` header dictates its own Argon2id cost before anything is authenticated.** Unavoidable in principle — a tag check needs a key — but it means an attacker's header chooses the work. Measured in release: 8 KiB → 46 µs, 1 GiB → 2.1 s, 64 MiB at 64 passes → 4.7 s, so `validate`'s ceiling is roughly **75 s and a gigabyte resident per attempt** before the user is told the file is damaged. | Opening a hostile or corrupt `.bpadx` | ADR-0021's bound works as intended (4 GiB is refused in microseconds) so this is a question of where the ceiling sits, not whether one exists. Lowering it rejects legitimate documents sealed by a future stronger default |

## Answered, for context

Kept briefly so a later session does not re-ask. Full reasoning lives in the
ADRs.

| Question | Answer | Where it landed |
| --- | --- | --- |
| Custom cryptography — compose or implement? | Compose vetted primitives; never implement them | ADR-0011, clarified |
| `.bpadx` AEAD | Both XChaCha20-Poly1305 and AES-256-GCM, versioned in the envelope | ADR-0021 |
| `.bpadx` envelope shape | Chunked from the start, chunk index bound into the AAD | ADR-0021 |
| Argon2id cost | OWASP baseline (19 MiB, t=2, p=1), recorded per document | ADR-0021 |
| Platform key protection (DPAPI/Hello/keyring) | Phase 16, not 15 | ADR-0021 |
| Security profiles | All of them; profile model first | ADR-0020 |
| YAML parser | `saphyr` | Phase 6, when it is taken |
| Parity vs retiring the custom editor view | Keep it opt-in, revisit on a Slint upgrade | ROADMAP, WORK_QUEUE |
| RC1 scope (S1-S6) | Defer notebook execution, semantic organisation, personality modes, highlighting; mmap declined for good | ADR-0030 |
| D5: where a signing key lives | A seed sealed in a passphrase `.bpadx` envelope | ADR-0030 |
| Slint licence | Royalty-free 2.0, `AboutSlint` in the About window | ADR-0031 |
| D9: application id | `io.github.dboles99.BachelorPad` | ADR-0032 |
| D8: Windows registration | The per-user installer writes and removes it; the app deep-links to Settings | ADR-0032 |
| Monospace font on Linux | Bundle DejaVu Sans Mono everywhere | ADR-0032 |
| Win32 calls | Only `bp-platform-windows`, via `windows-sys`, confined `unsafe` | ADR-0033 |
