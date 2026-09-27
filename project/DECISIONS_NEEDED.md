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
| D2 | **Is `--editor-view`'s two-view fork worth keeping indefinitely?** IME cannot work on Slint 1.17.1, so parity is unreachable until Slint changes. | Every future wiring item pays a small tax. Since M1-04 the caret features (Ln/Col, Go to Line, Insert Date/Time, the line moves) work in both views, so none is confined to the custom one | Answered once already (keep opt-in). Re-ask only if the tax becomes visible, or Slint ships composition support |
| D4 | **Should `bp-crypto`'s envelope get an outside review before anything real is encrypted with it?** Every primitive is composed from RustCrypto and nothing is reimplemented, but the envelope design is mine, and envelope design is where composed-primitive formats usually fail. | Trusting `.bpadx` with real documents | Not something the author of the design can answer by looking harder at it. **For RC1 answered (decision 20, 2026-09-26):** RC1 ships saying the envelope is unreviewed, in SECURITY.md and in the Encrypt prompt; whether and when to commission a review before 1.0 GA is still the owner's |
| D3 | **May `bp-storage` be wired into the application?** ADR-0020 permits `Metadata::Summary` under Standard, so the security half is settled. What remains is a product decision plus a design pass. | Phase 9 (related notes, duplicates; the `bp-organize` crate that `ARCHITECTURE.md` plans and nobody has created) | ADR-0019 explicitly says not to "just wire it up". For RC1 it is settled: ADR-0030 keeps `bp-storage` unwired. The question stands for 1.2 |
| D7 | **Should `bp-crypto` expose a derive-once API?** Today `derive` and `Key` are private, so every `seal` call pays a full Argon2id derivation. `bp-audit` seals one `.bpadx` envelope per line, so *reading* a sealed audit history costs ~45 ms per line — a few hundred lines is several seconds, and it grows over a document's life. | The security inspector staying usable on a long-lived document | Splitting `derive` from `seal_with_key` changes ADR-0021's surface, and the alternative — a cheaper KDF in `bp-audit` — would weaken every line against an offline attacker, which ADR-0020 forbids. Not an implementation detail |
| D11 | **Should "Convert to LF" (and to CRLF) also rewrite bare CRs, and should the status bar name a CR-only file "CR"?** D6 made a bare CR a line break everywhere, but `bp-files`'s conversion still treats it as data and `LineEnding` has no CR variant, so a classic-Mac file converts only partly and is labelled with the platform default. | Line-ending conversion agreeing with D6; `bp-files::encode`, the status bar | Conversion changes bytes written to disk, and the module was designed around leaving a bare CR alone. **Recommended: yes to both**, since every other part of the product already treats a bare CR as a break |
| 3a | **Can the owner get Azure Artifact Signing, or is it an OV certificate?** Decision 3 chose Artifact Signing, but Microsoft currently accepts individual developers in the USA and Canada only, and identity validation checks the applicant's ID and address. | Every signed Windows build (M3-04 is wired for both routes) | It turns on the owner's residence and Microsoft's validation, not on code. If Artifact Signing refuses, buy an OV certificate whose key lives on a token or cloud signer that installs into the Windows certificate store; `Signing.ps1` takes it through `BP_SIGN_CERT_SHA1` and `BP_SIGN_TIMESTAMP` |
| U4 | **How is Markdown previewed?** Slint has no HTML view. | View ▸ Split / Preview, Markdown TOC | Drawing styled text in-app, handing off to a browser and deferring are all defensible; the plan recommends deferring past RC1 |

`project/RELEASE_PLAN.md` §4 orders these by the milestone each blocks, and
also carries the release-only questions not repeated here: the signing
identity (decision 3; its eligibility question is 3a above). Answered 2026-09-26 and removed from here:
D1 (ADR-0034, nothing leaves the machine), the copyright holder (Daniel
Boles), packaging formats (decision 18) and how release builds are made
(decision 19, a local script). Also answered that day, as the plan
recommended: D6 (a line ends at LF, CRLF or a bare CR, everywhere), D10
(decision 15: ask above 4x the default Argon2 cost, refuse above 256 MiB,
8 passes or 16 lanes), U3 (a read-only viewer tab for files over the editable limit), U5
(clipboard menus snapshot the history when they open), decision 21 (the
Windows 10 statement) and decision 3's route (Azure Trusted Signing, now
called Artifact Signing; no identity yet, and 3a above asks whether the
owner is eligible). The icon (M2-01) is approved.

## Answered, for context

Kept briefly so a later session does not re-ask. Full reasoning lives in the
ADRs.

| Question | Answer | Where it landed |
| --- | --- | --- |
| Custom cryptography — compose or implement? | Compose vetted primitives; never implement them | ADR-0011, clarified |
| `.bpadx` AEAD | Both XChaCha20-Poly1305 and AES-256-GCM, versioned in the envelope | ADR-0021 |
| `.bpadx` envelope shape | Chunked from the start, chunk index bound into the AAD | ADR-0021 |
| Argon2id cost | OWASP baseline (19 MiB, t=2, p=1), recorded per document | ADR-0021 |
| Platform key protection (DPAPI/Hello/keyring) | Deferred to 1.x (first answered as "phase 16, not 15") | ADR-0030 S4; ADR-0021 |
| Security profiles | All of them; profile model first | ADR-0020 |
| YAML parser | `saphyr`; done and wired | ADR-0023 |
| Parity vs retiring the custom editor view | Keep it opt-in, revisit on a Slint upgrade | ROADMAP, WORK_QUEUE |
| RC1 scope (S1-S6), including U1: memory-map large files? | Defer notebook execution, semantic organisation, personality modes, highlighting; mmap declined for good (ADR-0027's decline stands, chunked reads) | ADR-0030 |
| D5: where a signing key lives | A seed sealed in a passphrase `.bpadx` envelope | ADR-0030 |
| Slint licence | Royalty-free 2.0, `AboutSlint` in the About window | ADR-0031 |
| D9: application id | `io.github.dboles99.BachelorPad` | ADR-0032 |
| D8: Windows registration | The per-user installer writes and removes it; the app deep-links to Settings | ADR-0032 |
| U2: monospace font on Linux | Bundle DejaVu Sans Mono everywhere | ADR-0032 |
| Win32 calls | Only `bp-platform-windows`, via `windows-sys`, confined `unsafe` | ADR-0033 |
