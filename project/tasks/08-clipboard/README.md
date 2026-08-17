# 08-clipboard

Roadmap phase 11. **Started.**

| Task | Status |
| --- | --- |
| [windows-history](01-windows-history.md) | Not started — OS history not surfaced |
| [local-history](02-local-history.md) | Done — `bp-clipboard` |
| [paste-transformations](03-paste-transformations.md) | Kind detection done; transformations not |
| [secure-clipboard](04-secure-clipboard.md) | Not started — blocked on phase 14 |

## What exists

The Edit menu lists recent clipboard entries with their detected kind (URL,
path, JSON, code, Markdown, text). Choosing one puts it back on the OS
clipboard and invokes the widget's own paste — the one route to the caret
while the caret itself is unreadable.

The history is a pure data structure, so its behaviour is tested without
touching an OS clipboard: newest first, re-copying moves rather than
duplicates, repeats of the front entry are ignored (the clipboard is polled,
so the same content arrives repeatedly), pinned entries survive the capacity
trim, and entries over 100k characters are truncated so the history is not a
memory leak with a menu attached.

Kind detection is structural and cheap. A URL or path must be a single line,
so a document merely mentioning `https://` is not a URL. JSON needs matching
delimiters. Code needs two markers, so prose with a stray semicolon is not
code.

## Outstanding

- **Nothing is persisted, deliberately.** specs.md section 14 makes persistent
  history opt-in, and a clipboard carries passwords, tokens and one-time codes
  constantly. Persistence belongs behind a setting *and* under ADR-0011's
  security profile — which means it is blocked on phase 14, not merely
  unwritten.
- **Secure clipboard and timed clearing** (task 04) are the same blocker.
- **Windows native clipboard history** is not surfaced. The platform matrix
  says it is Windows-only, so it needs `bp-platform-windows` and the adapter
  seam from ADR-0001.
- **Paste transformations** — paste as plain text, as a quote, as a code
  block, path normalisation. Kind detection already identifies what an entry
  is; nothing acts on it yet.
- **Pinning has no UI.** `toggle_pin` exists and is tested, but no menu row
  calls it.
