# BachelorPad+

**Notepad when you want it. More when you need it.**

BachelorPad+ is a plain-text editor for **Windows 10, Windows 11 and Linux**,
written in Rust. It opens, edits and saves text files the way Notepad does,
and adds what people who live in text files tend to want next: tabs, find and
replace across a folder, tools for JSON, YAML, TOML and CSV, a clipboard
history, crash recovery, and local security features such as encrypted
documents, per-document security profiles, secret scanning and signatures.

Everything happens on your machine. **Version 1 contains no network code:
nothing you type or open is sent anywhere.**

It is pre-release software. The first release candidate (1.0.0-rc.1) is
being prepared; read [Known gaps](#known-gaps) before you rely on it.

- [User guide](docs/user/GUIDE.md): how to do things, task by task
- [CHANGELOG.md](CHANGELOG.md): what has changed
- [SECURITY.md](SECURITY.md): what the security features do and do not
  protect, and how to report a vulnerability
- [CONTRIBUTING.md](CONTRIBUTING.md): building from source, the test gate,
  and the project's records

## Install

Packages arrive with the first release candidate. They will be published on
the project's [GitHub releases page](https://github.com/dboles99/bachelorpadplus_rust/releases):

| System | Packages |
| --- | --- |
| Windows 10, Windows 11 | A per-user installer (no administrator rights needed), and a portable zip |
| Linux | `.deb` (Debian, Ubuntu), `.rpm` (Fedora), a `.tar.gz`, and an Arch `PKGBUILD` |

AppImage and Flatpak builds may follow later. Until the packages exist,
**build from source**: see [CONTRIBUTING.md](CONTRIBUTING.md#build-and-run).

## Requirements

**Windows.** Windows 11, or Windows 10 22H2, 64-bit. Nothing else to install:
the C runtime is built into the program.

Windows 10 is supported on 22H2 with Extended Security Updates, or on an LTSC
edition, since ordinary Windows 10 no longer receives security fixes. It runs
the same program as Windows 11 and is tested at every release candidate, but
after 1.0 there will be no fixes for problems that occur only on Windows 10. The current code compiles for
Windows but has not yet been run on a Windows machine; see
[Known gaps](#known-gaps).

**Linux.** A 64-bit desktop running Wayland or X11. The program needs these
shared libraries at run time (the packages will declare them):

- fontconfig and FreeType
- xkbcommon and **xkbcommon-x11** (on Debian and Ubuntu, `libxkbcommon-x11-0`;
  without it the program stops at startup)
- the X11 client libraries (X11, X11-xcb, Xcursor, Xi) and the Wayland
  client library
- D-Bus
- an XDG desktop portal backend (for example `xdg-desktop-portal-gtk` or
  `xdg-desktop-portal-kde`), which draws the Open and Save dialogs

The editor's font, DejaVu Sans Mono, is bundled, so no font package is
needed.

## What works

- **Editing.** Tabs (Ctrl+Tab to move between them), undo and redo, cut,
  copy and paste, overwrite mode (Insert), word wrap, line numbers, zoom, go
  to line (Ctrl+G), duplicate and move lines, sort, deduplicate, reverse and
  trim lines, date and time insertion, four themes (Light, Dark, Organic,
  Green) plus Follow System.
- **Files.** Open several files at once, Save, Save As, Save All, Save a
  Copy, Save as New Revision, Reload from Disk, and a recent-files list.
  Every save is atomic: the file on disk is either the old version or the new
  one, never half of each. The status bar always shows the file's full path.
  If another program changes the file while you have it open, you are told
  before you overwrite its changes.
- **Encodings and line endings.** UTF-8, UTF-8 with a byte-order mark, and
  UTF-16 (both byte orders) open and save as they came. A file that is none
  of those opens as Windows-1252 (what Notepad calls "ANSI") and saves back
  byte for byte. LF and CRLF line endings, switchable from the Format menu.
- **Find.** Find and replace in the current document, with case, whole-word
  and regular-expression options, and Replace All shows the changes before
  making them. Find in Files searches a folder and its subfolders without
  freezing the window.
- **Structured data.** For JSON, JSON Lines, YAML, TOML, CSV and TSV
  documents, the Data menu validates, formats, minifies and converts between
  formats. JSON keeps your key order unless you ask it to sort.
- **Notes.** Suggested title, summary, keywords, outline and statistics,
  worked out from the text on your machine. A suggested file name follows the
  pattern `Title_DDMMMYYYY.ext`.
- **Clipboard history.** What you copy inside the editor is kept for the
  session and can be pasted again from the Edit menu, with paste
  transformations such as "Join Lines" or "Pretty-Print JSON". Copies made in
  other programs are recorded only if you turn that on in Settings.
- **Crash recovery.** Unsaved changes are written to a recovery journal
  every few seconds. After a crash, the next start offers to restore them.
- **Security.** Per-document security profiles (Standard, Private,
  Confidential, Maximum) and a session-wide Privacy Mode that control what
  the editor keeps about a document; encrypted `.bpadx` documents; Scan for
  Secrets and Redact Found Secrets; Inspect Metadata; SHA-256 document hashes;
  Ed25519 signatures (Sign Document and Verify Signature); and a Security
  History. [SECURITY.md](SECURITY.md) says what each of these does and does
  not protect.
- **Default editor.** File ▸ Set as Default Editor shows what opens your
  text files today, then helps you change it. On Linux it registers the
  editor and makes it the default after you agree, and File ▸ Remove
  Registration undoes that. On Windows the program never writes the registry
  itself: the installer registers it, and the row opens the Default apps page
  in Settings (a portable copy saves registration files for you to run
  instead).
- **Keyboard and screen readers.** F10, or Alt with a menu's letter, opens
  the menus; the menu bar, tabs and status bar carry labels for screen
  readers.

## Known gaps

This is a release candidate in preparation, and some things are missing or
unproven. The honest list:

- **Not yet run on Windows.** The current code builds for Windows, but
  nobody has run it on a Windows 10 or 11 machine since the last Windows test
  run, many changes ago. The Windows-only parts (default-editor registration,
  the console for `--help`, the crash message box) have never run at all.
- **No installers or packages yet.** They arrive with the release candidate.
- **The `.bpadx` encryption format has not had an independent
  cryptographic review.** It is built from standard, widely used components,
  but the way they are put together has only been reviewed by this project.
  Do not rely on it against a determined, well-resourced attacker until it
  has been. See [SECURITY.md](SECURITY.md).
- **Files larger than 32 MiB open read-only**, in a viewer that reads only
  the lines on screen (the limit can be changed in Tools ▸ Settings). The
  viewer has no selection or copy, cannot show UTF-16 files, shows bytes
  that are not UTF-8 as `�`, and shows only the first 64 KiB of any one
  line. It has been tested with files of tens of megabytes; a 1 GB file on
  each operating system is still to be tried. `--readonly` opens any file
  in the same viewer.
- **No syntax highlighting.** Source files open as plain text. There is no
  Markdown preview, no printing, no multi-cursor or column selection, and no
  replace across files.
- **Drag and drop does not open files.** The toolkit the editor is built on
  does not pass dropped file names to the program yet. Use File ▸ Open or the
  command line.
- **Each file opened from your file manager opens a new window.** There is
  no hand-off to a window that is already running.
- **The default editor view draws line breaks as its toolkit does.** A line
  ends at LF, CRLF or a bare carriage return, and nothing else, for line
  numbers, Go to Line, Find and Find in Files. The toolkit behind the default
  view may still draw a form feed or a Unicode line separator as a new line,
  which the line numbers do not count. The custom editor view draws the lines
  that are counted.
- **Some profiles mean no crash recovery.** Under Private or Confidential, a
  document that is not encrypted has no recovery journal, because the journal
  would have to be sealed with the document's passphrase. Under Maximum, and
  in Privacy Mode, there is no recovery journal at all.
- **Recovered work for an encrypted document appears only when you open it**
  and give its passphrase. At startup you are told how many encrypted
  documents have recovered work, but not which. Moving an encrypted document
  loses its recovered work.
- **The experimental custom editor view** (`--editor-view`, or Settings ▸
  Custom editor view) cannot take input-method (for example Chinese, Japanese
  or Korean) text. The default view can.
- **Signing keys are protected by a passphrase only.** Windows Hello, DPAPI
  and the Linux keyring are not used yet. Forgetting the passphrase means
  making a new key.
- **Deferred to later versions:** notebook execution (1.1); related notes,
  duplicate detection and semantic search (1.2); personality modes and
  syntax highlighting (1.x).
- **Barely used by people yet.** Automated tests drive the real window, but
  very few hours of human use are on record. Please report what you find.

## Where your files and settings live

| What | Windows | Linux |
| --- | --- | --- |
| Settings (`config.toml`) | `%APPDATA%\bachelorpad\` | `$XDG_CONFIG_HOME/bachelorpad/` (usually `~/.config/bachelorpad/`) |
| Recent files, security history, document profiles, logs | `%LOCALAPPDATA%\bachelorpad\state\` | `$XDG_STATE_HOME/bachelorpad/` (usually `~/.local/state/bachelorpad/`) |
| Recovery journal, signing key | `%LOCALAPPDATA%\bachelorpad\data\` | `$XDG_DATA_HOME/bachelorpad/` (usually `~/.local/share/bachelorpad/`) |

Only the settings roam with a Windows profile; everything else stays on the
machine. Tools ▸ Open Config Folder and Tools ▸ Open Log Folder take you
there. The [user guide](docs/user/GUIDE.md#settings-and-where-things-live)
has the details.

## Licence

Copyright Daniel Boles.

BachelorPad+ is licensed under either the MIT licence or the Apache License,
Version 2.0, at your option (`MIT OR Apache-2.0`). The program includes
[Slint](https://slint.dev), used under the Slint Royalty-free Desktop, Mobile,
and Web Applications Licence 2.0, and the DejaVu Sans Mono font, whose licence
(the Bitstream Vera licence, with the DejaVu changes placed in the public
domain) is in [crates/bp-ui/fonts/LICENSE-DejaVu.txt](crates/bp-ui/fonts/LICENSE-DejaVu.txt).
Help ▸ About BachelorPad+ shows these terms in the program.
