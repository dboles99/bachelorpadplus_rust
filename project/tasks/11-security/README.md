# 11-security

Roadmap phases 14 (security foundation), 15 (`.bpadx`) and 16 (advanced
security). **Done for RC1 except one item:** the `.bpadx` header half of
profile persistence (M1-19). The Argon2 cost ceiling (M2-09, D10), Sign
Document (M2-10, D5) and the "what leaves this machine" readout (M2-11,
ADR-0034) are done. OS key stores are deferred to 1.x (ADR-0030, S4). Status
checked against the code on 2026-09-27.

| Task | Status |
| --- | --- |
| profiles | Done — `bp-security`: Standard, Private, Confidential, Maximum (ADR-0020), governing the journal, clipboard history, recent list and metadata store; remembered per file under the state directory (M1-19, store half). Which axes are enforced and where each stops: `docs/architecture/SECURITY_MODEL.md` |
| bpadx | Done — `bp-crypto` (ADR-0021): Security ▸ Encrypt Document, unlock on open, saves stay encrypted; the recovery journal is sealed with it (ADR-0022). A file asking for more than 256 MiB, 8 passes or 16 lanes is refused, and one above 4x the default cost asks first (M2-09). Not reviewed by anyone outside the project (D4). A `.bpadx` opened from Recent Files, the command line or a Find in Files hit skips the passphrase flow; use File ▸ Open (`project/WORK_QUEUE.md`, "Follow-ups from M3") |
| key-protection | DPAPI, Windows Hello and the Linux keyring: not built, 1.x (ADR-0030, S4). The signing seed is sealed in a passphrase envelope (M2-10) |
| secret-scanning | Done — `bp-secrets`: Scan for Secrets, which never repeats what it found |
| hash-sign-redact | Done — Hash, Sign Document, Verify Signature and Redact Found Secrets (`bp-integrity`, `bp-redaction`). Sign Document's key is made on first use and sealed at `signing-key.bpadx` in the data directory (M2-10) |
| privacy-mode | Done — a session override that can only tighten |

Also done and not a task file: Inspect Metadata and Security History
(`bp-audit`, ADR-0024).

The six task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).
