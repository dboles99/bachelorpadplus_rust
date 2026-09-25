# Security Model

## Security profiles

- Standard
- Private
- Confidential
- Maximum
- Custom

## Propagation requirement

If a document is protected, the policy must cover:

- primary file
- autosave
- recovery journal
- revision history
- clipboard persistence
- semantic metadata
- embeddings
- temporary files
- audit events
- cloud/network eligibility

## What each axis does today, and where it stops

A profile is a set of seven axes (`bp_security::Policy`). An axis that
nothing reads looks like a guarantee but is only a label, which is worse
than not having the axis at all. So this table says which part of the
product honours each one, and where each one stops. Release plan M1-20,
2026-09-25.

| Axis | Honoured by | Limit |
|---|---|---|
| `recovery` | The recovery journal: plaintext, sealed with the document's passphrase, or none (ADR-0020, ADR-0022) | None known |
| `clipboard` | The in-app clipboard history, and marking copies to stay out of the OS history (Win+V, cloud clipboard, Klipper) | The OS clipboard itself holds whatever was copied until something replaces it |
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
