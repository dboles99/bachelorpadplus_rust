# 08-clipboard

Roadmap phase 11. **Done for RC1.** In-app history, kind detection and
format-aware paste ship, as do the editor context menu with Pin and Clear
(release-plan M1-18) and timed clearing (M2-08). Windows' native clipboard
history is deferred to 1.x (release plan §3). Status checked against the
code on 2026-09-27.

| Task | Status |
| --- | --- |
| windows-history | Not started; deferred to 1.x |
| local-history | Done — `bp-clipboard`. Records in-app copies only; other programs' copies need an opt-in setting (M0-07) |
| paste-transformations | Done — join lines, strip Markdown, pretty or minified JSON, bullet list, code block, forward slashes, offered by the entry's kind |
| secure-clipboard | Done: copies from Private and stricter documents are marked for exclusion from Win+V, cloud sync and Klipper (M0-07), and cleared off the OS clipboard after a minute if it still holds them (M2-08) |

The four task files were empty templates and were deleted on 2026-09-25 (see
[../README.md](../README.md)).

## What exists

The Edit menu and the editor context menu (M1-18) list recent clipboard entries with their detected kind (URL,
path, JSON, code, Markdown, text). Choosing one in the default view puts it
back on the OS clipboard and invokes the widget's own paste; under
`--editor-view` it is inserted at the caret directly.

The history is a pure data structure, so its behaviour is tested without
touching an OS clipboard: newest first, re-copying moves rather than
duplicates, repeats of the front entry are ignored (when other programs'
copies are recorded the clipboard is polled, so the same content arrives
repeatedly), pinned entries survive the capacity
trim, and entries over 100k characters are truncated so the history is not a
memory leak with a menu attached.

Kind detection is structural and cheap. A URL or path must be a single line,
so a document merely mentioning `https://` is not a URL. JSON needs matching
delimiters. Code needs two markers, so prose with a stray semicolon is not
code.

## Outstanding

- **Nothing is persisted, deliberately.** specs.md section 14 makes persistent
  history opt-in, and a clipboard carries passwords, tokens and one-time codes
  constantly. The security profiles it would sit under exist now (phase 14,
  ADR-0020), so this is unbuilt rather than blocked. It is not in the release
  plan's RC1 scope.
- ~~Timed clearing~~ **Done** (M2-08): `bp_clipboard::PendingClear` keeps a
  fingerprint, not the text, and the shell clears after a minute.
- **Windows native clipboard history** is not surfaced. It needs WinRT, which
  the release plan defers to 1.x; `bp-platform-windows` (ADR-0033) holds
  Win32 shims only.
- ~~Pinning has no UI.~~ **Done** (M1-18): the editor context menu has Pin ▸
  and Clear Clipboard History. Every menu that shows clipboard rows takes a
  snapshot of the history when it opens (U5).
- **The context menu ignores the theme.** It is Slint's own popup, light
  under every theme (`project/WORK_QUEUE.md`, "Follow-ups from M3").
