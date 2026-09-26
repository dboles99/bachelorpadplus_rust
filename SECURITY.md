# Security policy

## Supported versions

BachelorPad+ has not reached 1.0. Until it does, only the **latest release
candidate** receives security fixes; a fix ships as the next release
candidate. There are no releases yet: the first will be `1.0.0-rc.1`.

| Version | Supported |
| --- | --- |
| Latest `1.0.0-rc.N` | Yes |
| Earlier release candidates | No: upgrade to the latest |
| Builds from source between releases | Best effort |

## Reporting a vulnerability

Please report vulnerabilities privately, through GitHub's private
vulnerability reporting:

1. Go to <https://github.com/dboles99/bachelorpadplus_rust>.
2. Open the **Security** tab.
3. Choose **Report a vulnerability**.

Do not open a public issue for a vulnerability. Include the version (Help ▸
About BachelorPad+ shows it, with the build's commit), your operating system,
and the steps that show the problem. If a document is involved, send a
minimal example rather than a real document, and never send a real
passphrase or key.

This is a small project maintained by one person. Reports are read and
answered as soon as practical; there is no bug bounty.

## What BachelorPad+ protects, and what it does not

The full, per-feature account is in
[docs/architecture/SECURITY_MODEL.md](docs/architecture/SECURITY_MODEL.md).
In short:

**What it is designed to do**

- Keep what the editor itself writes about a document (recovery journal,
  clipboard history, recent-files list, security history) in line with the
  document's security profile, and with Privacy Mode, which can only tighten.
- Encrypt documents at rest in the `.bpadx` format, with a passphrase.
- Keep a signing key sealed under a passphrase, and sign and verify
  documents with Ed25519.
- Find likely credentials in a document and redact them, without ever
  displaying the matched text.
- Save atomically, so an interrupted save does not leave a half-written
  file.

**What it does not do**

- It does not protect you from anything with control of your user account
  or your machine: malware, a keylogger, an administrator or root, or
  someone at your unlocked computer.
- It does not wipe all document text from memory. The document's text, its
  undo history and the text widget's copy are freed without being
  overwritten; only the passphrase and the plaintext copies a save makes are
  wiped.
- It does not securely erase files. Encrypting a document writes a new
  `.bpadx` file and leaves the original, unencrypted file where it was.
- It does not control copies other programs make. A clipboard manager that
  ignores the "exclude from history" marking keeps its own copy, and for up
  to a minute the system clipboard holds whatever you copied.
- A valid signature says which key signed a file, not whose key it is.
- The atomic save briefly writes a temporary file beside the document (for
  an encrypted document, it holds ciphertext).
- The signing key and encrypted documents are protected by passphrases
  only; Windows DPAPI, Windows Hello and the Linux keyring are not used yet.

## Nothing leaves the machine

**BachelorPad+ version 1 contains no network code.** It does not check for
updates, send telemetry or crash reports, fetch anything, or send a document
anywhere. Crash reports and logs are written to a folder on your machine
(Tools ▸ Open Log Folder) and go nowhere unless you send them yourself.

The Security menu says so: under Standard and Private its readout is
"Leaves this machine: nothing (no network code)", and under Confidential and
Maximum "never (forbidden by this profile)". The profile setting is a
constraint on future features, not a permission anything uses today
(ADR-0034).

## The `.bpadx` encryption format has not been independently reviewed

Encrypted documents use a format designed by this project (ADR-0021): the
passphrase is turned into a key with **Argon2id**, and the document is
encrypted in chunks with **XChaCha20-Poly1305** (files written with
**AES-256-GCM** can also be read), with the header and every chunk's
position authenticated. The cryptographic primitives come from established,
widely used Rust libraries (RustCrypto); none is implemented here.

**The way those primitives are combined has not had an independent
cryptographic review.** Until it has, do not rely on `.bpadx` encryption to
protect a document against a determined, well-resourced attacker. It is a
reasonable protection against casual access to a lost laptop or a copied
folder. The Encrypt Document prompt says the same in one line.

Whether an independent review happens before 1.0 is the project owner's
decision. If it happens, its result will be recorded here.

Some consequences of the format that are by design:

- A wrong passphrase and a damaged file produce the same error; the format
  cannot tell them apart.
- There is no way to recover a document whose passphrase is forgotten.

## Signing keys

Security ▸ Sign Document creates an Ed25519 signing key the first time it is
used. The key is kept in the **data directory**
(`%LOCALAPPDATA%\bachelorpad\data\` on Windows, `$XDG_DATA_HOME/bachelorpad/`
on Linux) as `signing-key.bpadx`: the key's seed, **sealed in the same
passphrase envelope as an encrypted document**, with the passphrase you chose
when the key was made. The public half is beside it as `signing-key.pub`.

The key is unsealed only while a document is being signed. Anyone who copies
`signing-key.bpadx` can try to guess its passphrase offline, so choose a
strong one. If you forget it, make a new key; signatures made with the old
key will still verify against the old public key, but you can no longer
sign with it.
