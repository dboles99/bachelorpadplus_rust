# Security Model

## Security profiles

- Standard
- Private
- Confidential
- Maximum
- Custom

## Propagation: what a protected document's policy reaches

Only what is enforced today is listed here (release plan M2-12). The table
below says which code honours each axis and where it stops.

- **The primary file**: an encrypted document is saved through the same
  `.bpadx` envelope every time, with the passphrase held for the session.
- **The recovery journal**: plaintext, sealed with the document's
  passphrase, or not kept, per the profile.
- **The clipboard**: the in-app history keeps nothing under a profile that
  forbids it. Copies from a Private or stricter document are marked to stay
  out of the OS clipboard history, and are cleared off the OS clipboard
  after a minute if it still holds them.
- **Metadata the product keeps about files**: the recent-files list, the
  security history and the document-profile store (a digest of the path).
- **Audit events**: every security-relevant action appends to the security
  history, which names no document content.

Not reached, because the thing does not exist in 1.0: revision history
(deferred to 1.x), semantic metadata and embeddings (the crates are not
wired to the shell), and network eligibility (nothing in the product sends a
document anywhere; ADR-0030). Temporary files are an accepted deviation: the
atomic save writes its temporary copy beside the document.

## What each axis does today, and where it stops

A profile is a set of seven axes (`bp_security::Policy`). An axis that
nothing reads looks like a guarantee but is only a label, which is worse
than not having the axis at all. So this table says which part of the
product honours each one, and where each one stops. Release plan M1-20,
2026-09-25.

| Axis | Honoured by | Limit |
|---|---|---|
| `recovery` | The recovery journal: plaintext, sealed with the document's passphrase, or none (ADR-0020, ADR-0022) | None known |
| `clipboard` | The in-app clipboard history; marking private copies to stay out of the OS history (Win+V, cloud clipboard, Klipper); clearing a private copy off the OS clipboard after `bp_clipboard::PRIVATE_CLEAR_AFTER` (a minute) if it still holds it (M2-08) | For that minute the OS clipboard holds the text, and a clipboard manager that ignores the exclusion hint keeps its own copy. A private *cut* in the default view reaches the OS clipboard unmarked (the widget owns cut) |
| `metadata` | The recent-files list, the security history, the document-profile store (digest only) | The profile store keeps a digest of each protected file's path. Anyone who guesses a path can confirm the guess (`bp_history::profiles`) |
| `network` | Shown in the Security menu | Nothing in the product sends a document anywhere yet, so there is nothing for this axis to stop |
| `embeddings` | **Nothing yet** | No embeddings are computed yet. Until they are, this axis is a promise about future code, and this row says so |
| `temporary_files` | **Nothing enforces `Denied`** | Every save is atomic: it writes a temporary file *in the document's own folder*, then renames it over the document. **This is an accepted deviation.** The temporary file holds exactly the bytes the document is about to hold, sits beside it, and does not survive a completed save; a crash can leave one behind. No save writes document content anywhere else (no system temp directory). For an encrypted document the temporary file holds the ciphertext |
| `zeroise` | **Partly** | See below |

### Zeroise: what is wiped and what cannot be

`On` asks that buffers holding document content be wiped after use (specs
section 15: "where practical"). Here is what this build does:

- **Wiped when dropped:** the passphrase held for an encrypted document, for
  the whole session; the plaintext copies a save makes (the text and its
  encoded bytes); and the sealed output. This happens for every document,
  not only under `On`, because it costs one pass over memory that is being
  freed anyway.
- **Not wiped: the document text itself.** It lives in `ropey`'s rope, whose
  chunks cannot be written in place from safe code. It also lives in the
  undo history, and in the default view in Slint's own `TextInput`. Freeing
  those returns their memory to the allocator unwiped. Wiping them would take
  either a zeroing global allocator or a rope with a wiping API. Both need
  `unsafe`, which ADR-0033 allows only in `bp-platform-windows`, so this is
  left for a decision rather than done quietly.
- **Not wiped: the recovery journal's in-memory copy**, while it is being
  serialised and sealed.

So under `On`, nothing on disk outlives the document. In memory, what can
be wiped is wiped. The document text is not, and this section is where that
is said.

## Encrypted format

Extension: `.bpadx`

Portable encryption:
- passphrase
- Argon2id or reviewed equivalent KDF
- established AEAD algorithm/library
- versioned envelope
- authenticated metadata handling

Platform key protection:
- Windows: DPAPI / Windows Hello-assisted workflows where appropriate
- Linux: Secret Service / desktop keyring integrations where appropriate

## Notebook execution

Never auto-run:
- opened notebook
- pasted code
- restored executable cell

Execution UI must show interpreter, environment and working directory.
