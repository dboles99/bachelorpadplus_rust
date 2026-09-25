# 11-security

Roadmap phases 14 (security foundation), 15 (`.bpadx`) and 16 (advanced
security). **Mostly done.** Open for RC1: the Argon2 cost ceiling (M2-09, D10),
Sign Document (M2-10, D5 answered), the "what leaves this machine" readout
(M2-11, D1) and the `.bpadx` header half of profile persistence (M1-19).
OS key stores are deferred to 1.x (ADR-0030, S4). Status checked against the
code on 2026-09-25.

| Task | Status |
| --- | --- |
| profiles | Done — `bp-security`: Standard, Private, Confidential, Maximum (ADR-0020), governing the journal, clipboard history, recent list and metadata store; remembered per file under the state directory (M1-19, store half). Which axes are enforced and where each stops: `docs/architecture/SECURITY_MODEL.md` |
| bpadx | Done — `bp-crypto` (ADR-0021): Security ▸ Encrypt Document, unlock on open, saves stay encrypted; the recovery journal is sealed with it (ADR-0022). Not reviewed by anyone outside the project (D4) |
| key-protection | Not built. DPAPI, Windows Hello and the Linux keyring: 1.x (ADR-0030, S4). The signing seed sealed in a passphrase envelope: M2-10 |
| secret-scanning | Done — `bp-secrets`: Scan for Secrets, which never repeats what it found |
| hash-sign-redact | Hash, Verify Signature and Redact Found Secrets done (`bp-integrity`, `bp-redaction`); Sign Document greyed with the reason until M2-10 |
| privacy-mode | Done — a session override that can only tighten |

Also done and not a task file: Inspect Metadata and Security History
(`bp-audit`, ADR-0024).

The six task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).
