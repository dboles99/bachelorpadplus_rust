# Changelog

All notable changes to BachelorPad+ are recorded in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
There has been no release yet; the first will be `1.0.0-rc.1`.

## [Unreleased]

Everything below is new since development began. "Changed" and "Fixed"
describe behaviour that earlier development builds had.

### Added

- A plain-text editor for Windows 10, Windows 11 and Linux, with tabs,
  Ctrl+Tab and Ctrl+Shift+Tab, undo and redo, overwrite mode, word wrap, line
  numbers, zoom, and four themes (Light, Dark, Organic, Green) plus Follow
  System.
- File ▸ Open takes several files at once; a recent-files list; Save All,
  Save a Copy, Save as New Revision and Reload from Disk.
- Atomic saves, detection of changes made on disk by another program, and
  the file's full path always shown in the status bar.
- UTF-16 (both byte orders) opens and saves; files that are not UTF-8 or
  UTF-16 open as Windows-1252 and save back unchanged.
- Line and column in the status bar, Go to Line (Ctrl+G), Duplicate Line
  (Ctrl+D), Move Line Up and Down (Alt+Up, Alt+Down), sort, deduplicate,
  reverse and trim lines, and date and time insertion.
- Find and replace with case, whole-word and regular-expression options; a
  preview before Replace All; Find in Files, which runs without freezing the
  window and can be stopped.
- The Data menu: validate, format, minify and convert JSON, JSON Lines, YAML,
  TOML, CSV and TSV.
- The Note menu: suggested title, summary, keywords, outline and document
  statistics; suggested file names in the form `Title_DDMMMYYYY.ext`.
- A clipboard history of copies made in the editor, with paste
  transformations; recording other programs' copies is an opt-in setting.
- A context menu in the editor (right-click, the Menu key or Shift+F10), in
  both editor views: Cut, Copy, Paste, Select All, Paste Special...,
  Clipboard History, Pin and Clear Clipboard History. Paste Special... and
  Clear Clipboard History are in the Edit menu too.
- A crash-recovery journal, offered back at the next start with Restore,
  Discard and Not Now.
- Encrypted `.bpadx` documents (Security ▸ Encrypt Document), unlocked when
  opened and kept encrypted on every save.
- Per-document security profiles (Standard, Private, Confidential, Maximum),
  remembered for each file, and a session-wide Privacy Mode.
- Scan for Secrets, Redact Found Secrets, Inspect Metadata, Hash Document
  (SHA-256), Verify Signature and Security History.
- Sign Document, with the signing key sealed under a passphrase in the data
  directory.
- File ▸ Set as Default Editor on Windows and Linux, and File ▸ Remove
  Registration on Linux.
- Tools ▸ Settings (Ctrl+,), Open Log Folder and Open Config Folder; Help ▸
  Keyboard Shortcuts and About.
- The menu bar from the keyboard (F10, and Alt with a menu's letter), and
  labels for screen readers on the menus, tabs and status bar.
- A read-only large-file viewer: a file over the editable limit (32 MiB,
  configurable in Settings) opens in a viewer tab with a banner saying why,
  instead of being refused. It reads only the lines on screen, indexes the
  file and searches it in the background (Find Next and Previous, stoppable),
  and supports Go to Line, Ctrl+Home/End, the wheel and a scrollbar.
  Everything that would change or write the document is greyed.
- The command-line options `--line N`, `--readonly` and `--large-file` (both
  open the files in the viewer), `--help` and `--version`.
- A log file and, after a crash, a crash report in the state directory.
- A bundled monospace font (DejaVu Sans Mono) and an application icon.
- The Linux desktop entry, file-type definition and AppStream metadata, under
  the application id `io.github.dboles99.BachelorPad`.

### Changed

- Every question (unsaved changes, recovery, Replace All, registration) is
  asked inside the editor's window, with buttons that say what they do. On
  Linux the editor no longer needs `zenity`.
- Choices made in the View and Format menus and in Settings are saved to
  your `config.toml`, keeping anything you wrote there by hand.
- Recent files, the security history and the recovery journal moved out of
  the roaming settings folder into per-machine folders. They are moved
  automatically at the first start.
- Menus show only what works; rows for planned features are gone.
- A status-bar notice appears beside the file's path instead of replacing
  it.
- Save As suggests a name that is not already taken.
- Recovery checkpoints are written in the background, and only for documents
  that changed since the last one.

### Fixed

- Recovered work could be deleted at startup on Linux when no dialog program
  was installed; now only an explicit Discard deletes it.
- A second running window could offer, or delete, the first window's
  recovered work.
- Files too large to edit were read whole into memory; they are now opened
  in the read-only viewer without being read whole.
- Opening any file read it whole once just to check whether it was
  encrypted; now only its first bytes are read.
- JSON Format reordered keys, and Sort Keys did not sort; TOML Format now
  warns that it drops comments.
- `--line 427 file` opened a file named `427`.
- Legacy ANSI (Windows-1252) text files were refused.
- Saving a file with mixed line endings rewrote the minority line ending.
- Closing a tab with Don't Save left its recovery checkpoint behind, so the
  discarded work was offered back at the next start.
- Find selected the wrong text after accented letters; Page Up and Page Down
  did nothing; Alt+Up and Alt+Down did not move lines; a question opened from
  a menu did not get the keyboard.
- The custom editor view lost the keys typed before the first click.
- Linux drew the editor in a proportional font.
- Saving to a name such as `con.txt` on Windows wrote to a device and
  reported success; such names are now refused.
- A very long path could not be saved on Windows.
- The Markdown outline listed comments inside code blocks as headings.
- Menu readouts that described features as not yet available after they
  had shipped.
- Find in Files, the secret scanner, the outline and document statistics
  could name a different line from the editor in a file with old Mac line
  endings, and the editor counted a form feed or a Unicode line separator as
  a new line. A line now ends at LF, CRLF or a bare carriage return, and
  nothing else, everywhere.
- A clipboard history row could paste a different entry from the one it
  showed, if something was copied while the menu was open. The menus now
  show the history as it was when they opened, and a row whose entry has
  gone says so and does nothing.

### Security

- Save a Copy of an encrypted document now writes an encrypted copy with the
  same passphrase. It used to write the text unencrypted, without warning.
- The `.bpadx` envelope uses Argon2id and XChaCha20-Poly1305 (AES-256-GCM
  files can also be read), with every chunk's position authenticated. **It
  has not had an independent cryptographic review**; the Encrypt Document
  prompt and [SECURITY.md](SECURITY.md) say so.
- Opening a `.bpadx` file whose stored key-derivation cost is above four
  times the default now asks first, and one asking for more than 256 MiB,
  8 passes or 16 lanes is refused. A damaged or hostile file could previously make the
  editor spend over a minute and a gigabyte of memory before reporting it.
- The recovery journal for an encrypted document is sealed with its
  passphrase, and under Private and Confidential no unencrypted journal is
  written.
- The clipboard history records only copies made in the editor unless you
  opt in. Copies from Private or stricter documents are marked to stay out of
  clipboard managers' history (Windows clipboard history and cloud clipboard,
  KDE Klipper) and are cleared off the system clipboard after a minute.
- The recent-files list honours the document's profile and Privacy Mode, and
  is written atomically.
- A save wipes its in-memory plaintext copies of the document when it is
  done.
- Secret scanning and metadata inspection report positions and kinds, never
  the matched text.
- There is no network code: nothing leaves the machine.

[Unreleased]: https://github.com/dboles99/bachelorpadplus_rust/commits/main
