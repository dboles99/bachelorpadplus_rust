# BachelorPad+ user guide

This guide describes what the editor does today, task by task. It covers
only features you can reach in the program; planned features are listed in
the [README](../../README.md#known-gaps) as gaps, not described here.

Menu paths are written like **File ▸ Save As**. Keyboard shortcuts use Ctrl
on both Windows and Linux.

- [Opening and saving](#opening-and-saving)
- [Encodings and line endings](#encodings-and-line-endings)
- [The two editor views](#the-two-editor-views)
- [Editing](#editing)
- [Find, replace, and find in files](#find-replace-and-find-in-files)
- [Clipboard history and privacy](#clipboard-history-and-privacy)
- [Structured data: the Data menu](#structured-data-the-data-menu)
- [Notes and file names](#notes-and-file-names)
- [Security profiles and Privacy Mode](#security-profiles-and-privacy-mode)
- [Encrypted documents (.bpadx)](#encrypted-documents-bpadx)
- [Secrets, metadata and hashes](#secrets-metadata-and-hashes)
- [Signing and verifying](#signing-and-verifying)
- [Setting BachelorPad+ as the default editor](#setting-bachelorpad-as-the-default-editor)
- [Recovering after a crash](#recovering-after-a-crash)
- [Settings and where things live](#settings-and-where-things-live)
- [The command line](#the-command-line)
- [Keyboard shortcuts](#keyboard-shortcuts)

## Opening and saving

- **Open** a file with **File ▸ Open...** (Ctrl+O). You can select several
  files at once; each opens in its own tab. Recently opened files are listed
  in the File menu, below Open, with their folder beside them.
- **Save** with Ctrl+S. A document that has never been saved asks for a
  name first. **Save As...** (Ctrl+Shift+S) saves under a new name, and the
  tab follows the new file. **Save All** saves every tab with unsaved
  changes.
- **Save a Copy...** writes the document to another file without changing
  which file the tab is attached to, and without marking the document saved.
- **Save as New Revision** writes the next revision beside the file, adding
  or raising a `_v2`, `_v3`... part in the name, so the earlier version stays
  as it was.
- **Reload from Disk** throws away your unsaved edits and reads the file
  again.
- **Close Tab** (Ctrl+W). Right-click a tab for Close Tab, Close Other Tabs,
  Close All Tabs and Copy Full Path.

Every save is **atomic**: the new contents are written to a temporary file in
the same folder and then moved over the old file, so a crash or a full disk
leaves either the old file or the new one, never a mixture. The file's full
path is always shown in the status bar.

When you close a tab or quit with unsaved changes, the editor asks
**Save / Don't Save / Cancel** inside its own window. Enter saves, Escape
cancels. If another program changes a file you have open, the status bar
says so and points you to File ▸ Reload from Disk; if you save anyway, you
are asked first (**Save Anyway / Cancel**, where Enter means Cancel).

Files larger than 32 MiB are not opened for editing: the editor refuses them
with a notice instead of loading them. You can raise or lower that limit in
**Tools ▸ Settings...** ("Largest file opened for editing").

## Encodings and line endings

The status bar shows each document's encoding and line ending.

| When a file is... | It opens as | And saves as |
| --- | --- | --- |
| UTF-8, with or without a byte-order mark (BOM) | UTF-8 / UTF-8 with BOM | the same |
| UTF-16, little- or big-endian, with a BOM | UTF-16 LE / BE | the same |
| none of those | Windows-1252 (Notepad's "ANSI") | Windows-1252, byte for byte |

A UTF-16 file that is cut short or malformed is refused with the reason,
rather than opened with damaged characters. So is a file that looks binary
(it is not UTF-8 and contains a NUL byte), and a file that starts with a
UTF-8 byte-order mark but is not valid UTF-8. UTF-16 is recognised only by
its byte-order mark.

**Format ▸ UTF-8 / UTF-8 with BOM / Windows-1252** changes the encoding the
document will be saved in. Windows-1252 holds only 256 characters, so
choosing it for a document with a character it cannot hold is refused, and
the status bar names the character and its line. If you type such a
character into a Windows-1252 document, the save stops and offers
**Save as UTF-8** instead of writing a `?` in its place.

**Format ▸ LF / CRLF** sets the line ending the document is saved with (LF is
the Linux convention, CRLF the Windows one). A new document starts with the
convention of the system you are on. Changing it marks the document as
unsaved. Otherwise a save writes the line endings back as they were read.

## The two editor views

The **default view** uses the text-editing widget of Slint, the toolkit the
editor is built on. It supports input methods (for typing Chinese, Japanese,
Korean and other scripts) and is the view to use.

The **custom editor view** is experimental. Turn it on with
`--editor-view` on the command line for one run, or with **Tools ▸
Settings... ▸ Custom editor view** from the next launch. It draws the text
itself, and it **cannot accept input-method text**, so it is not suitable if
you type in a script that needs one.

Every menu row and shortcut in this guide works in both views.

## Editing

- **Undo / Redo** (Ctrl+Z / Ctrl+Y), **Cut / Copy / Paste** (Ctrl+X / Ctrl+C
  / Ctrl+V, and also Shift+Delete / Ctrl+Insert / Shift+Insert), **Select
  All** (Ctrl+A).
- **Insert** switches overwrite mode on and off; `OVR` appears in the status
  bar while it is on.
- **Edit ▸ Go to Line...** (Ctrl+G) opens a small bar. A line number past
  the end moves to the last line and says how many lines there are.
- **Edit ▸ Duplicate Line** (Ctrl+D), **Move Line Up / Down** (Alt+Up /
  Alt+Down).
- **Edit ▸ Sort Lines A → Z / Z → A, Remove Duplicate Lines, Reverse Lines,
  Trim Trailing Whitespace** act on the whole document.
- **Insert ▸ Date / Time / Date and Time / ISO 8601 / Filename Date** inserts
  a stamp at the caret. Each row shows what it will insert.
- **View ▸ Line Numbers** and **Word Wrap** switch the gutter and wrapping.
- **View ▸ Zoom In / Zoom Out / Reset Zoom** (Ctrl+= or Ctrl++, Ctrl+-,
  Ctrl+0).
- **View ▸ Light / Dark / Organic / Green** choose a theme; **Follow System**
  uses the desktop's light or dark setting where the desktop reports one (the
  row says when it does not). Clicking the theme name in the status bar moves
  to the next theme.
- **Format ▸ Indent with Tabs / Spaces** and **Tab Width 2 / 4 / 8** set what
  the Tab key inserts and how wide a tab is drawn.

Themes, line numbers, word wrap, zoom and indentation are remembered between
sessions.

## Find, replace, and find in files

**Ctrl+F** opens the find bar.

- Type in **Find**; Enter or **▸** moves to the next match, **◂** to the
  previous one. The bar shows which match you are on, such as "3 of 12".
- **Aa** makes the search case-sensitive, **Word** matches whole words only,
  and **.\*** treats the query as a regular expression. Changing one runs the
  search again.
- Type the replacement in **Replace** and choose **Replace All**. A dialog
  shows how many replacements there will be and a preview of the first
  changes; nothing changes until you confirm.
- **In Folder...** asks for a folder and searches the files in it and its
  subfolders for the query, using the same options. Results appear in a
  panel under the editor, and the window stays responsive while the search
  runs. Click a result to open the file at the match. Closing the panel stops
  a search that is still running.

  To keep the search useful, it skips binary files, files over 5 MiB, hidden
  folders (names starting with a dot), and build and dependency folders such
  as `.git`, `target`, `node_modules`, `build` and `dist`. It stops after 500
  matches.
- Escape, or **×**, closes the bar.

Find in Files can place the caret on the wrong line in a file that uses a
bare carriage return (old Mac style) as a line break. This is a known gap.

## Clipboard history and privacy

The **Edit** menu lists what you have copied in this session, newest first,
under the Select All row. Choosing an entry pastes it at the caret. Below
each entry, indented rows offer **paste transformations** that suit it, such
as **Join Lines**, **Strip Markdown**, **Pretty-Print JSON**, **Minify
JSON**, **Forward Slashes**, **As Bullet List** and **As Code Block**. A
transformation is offered only when it would change the text.

What is recorded:

- **Copies and cuts you make inside the editor** are recorded, unless the
  document's security profile or Privacy Mode forbids it (see below).
- **Copies made in other programs are not recorded** unless you turn on
  **Tools ▸ Settings... ▸ Clipboard history records other programs**. It is
  off by default so that what a password manager copies does not end up in
  the Edit menu.
- The history is kept **in memory only** and is gone when you quit.

When you copy from a document whose profile is **Private or stricter**, the
editor also:

- marks the copy so that clipboard managers which honour the marking leave
  it out of their history (Windows clipboard history and cloud clipboard,
  and KDE's Klipper);
- clears it off the system clipboard after one minute, if the clipboard
  still holds it.

A clipboard manager that ignores the marking may keep its own copy. In the
default view, a **cut** (as opposed to a copy) from such a document reaches
the system clipboard without the marking.

## Structured data: the Data menu

The **Data** menu depends on the kind of document in the active tab. For a
document that is not one of these formats it says there is nothing to do.

| Format | Rows |
| --- | --- |
| JSON | Validate, Format, Minify, Sort Keys, Convert to JSON Lines, Convert to YAML |
| JSON Lines | Validate, Convert to JSON |
| YAML | Validate, Format, Minify, Convert to JSON |
| TOML | Validate, Format |
| CSV, TSV | Report Shape, Column Types, Convert to JSON, Convert to JSON Lines |

- **Validate** and **Report Shape** report in the status bar and change
  nothing.
- **Format**, **Minify**, **Sort Keys** and the conversions **replace the
  text in the tab**. The file on disk is not touched until you save. After a
  conversion the tab still has its old file name, so use **Save As** to give
  the new format its own file.
- JSON **Format** keeps your keys in the order you wrote them; **Sort Keys**
  sorts them at every level.
- YAML **Format**, **Minify** and **Convert to JSON**, and TOML **Format**,
  **do not keep comments** (the rows say so). TOML Format also sorts the
  keys. A YAML file that nests too deeply, expands aliases too far, or
  repeats a key is refused with the reason, and the document is left
  unchanged.
- **Convert to JSON** from YAML refuses a file that holds more than one YAML
  document; Validate tells you how many it holds.

## Notes and file names

The **Note** menu works out facts about the document from its text, on your
machine: **Suggest Title**, **Summary**, **Keywords**, **Outline** (from
Markdown headings; headings inside code blocks are ignored) and **Document
Statistics**.

When you save a new document, the editor suggests a file name of the form
`Title_DDMMMYYYY.ext`, for example `Meeting Notes_26SEP2026.txt`, using a
title taken from the text where it can find one. The suggestion never names
a file that already exists in that folder. **Note ▸ Semantic Rename...** is a
Save As with that suggested name; nothing is renamed until you confirm the
dialog.

## Security profiles and Privacy Mode

Each document has a **security profile**, chosen in the **Security** menu.
The profile decides what the editor keeps about that document. Below the
profile rows, greyed readout rows say what the profile in force means for
this document: the recovery journal, the clipboard history, and whether
anything may leave the machine.

| | Recovery journal | Clipboard history | Recent files, security history |
| --- | --- | --- | --- |
| **Standard** (default) | kept, unencrypted | kept in memory | recorded |
| **Private** | kept only if the document is encrypted, sealed with its passphrase | kept in memory; copies marked private and cleared after a minute | recent files recorded; security history sealed with the passphrase |
| **Confidential** | kept only if the document is encrypted, sealed with its passphrase | not kept | not recorded |
| **Maximum** | not kept | not kept | not recorded |

- A document's profile is remembered for that file between sessions. The
  editor stores a digest of the file's path, not the path itself.
- **Tools ▸ Settings... ▸ Security profile for new documents** sets the
  profile a newly opened document starts with.
- Under Private and Confidential an **unencrypted** document has no crash
  recovery. The Security menu says "off until the document is encrypted";
  encrypting the document fixes it.
- The status bar shows the profile when it is not Standard.

**Privacy Mode** (**Security ▸ Privacy Mode**) is a switch for the whole
session, useful before sharing your screen or when working on someone
else's machine. It can only make things stricter: while it is on, no
document gets a recovery journal or clipboard history, and nothing is added
to the recent-files list or the security history, whatever each document's
profile says. The status bar shows "Privacy Mode" while it is on. It is off
again at the next start.

**Nothing leaves the machine.** Version 1 has no network code. Under
Standard and Private the readout row says "Leaves this machine: nothing (no
network code)"; under Confidential and Maximum it says "never (forbidden by
this profile)", which is the rule any future networked feature will have to
obey.

The full account of what each profile setting does, and where each stops, is
in [SECURITY_MODEL.md](../architecture/SECURITY_MODEL.md).

## Encrypted documents (.bpadx)

**The `.bpadx` encryption format has not had an independent cryptographic
review.** It uses Argon2id to turn your passphrase into a key and
XChaCha20-Poly1305 to encrypt (AES-256-GCM files can be read too), all from
standard libraries, but the way they are combined has been reviewed only by
this project. Do not rely on it against a determined, well-resourced
attacker until it has been independently reviewed. See
[SECURITY.md](../../SECURITY.md).

**To encrypt a document:**

1. Choose **Security ▸ Encrypt Document...**.
2. Choose where to save the encrypted file. `.bpadx` is added to the name if
   you leave it off.
3. A passphrase bar opens above the editor. Its first prompt notes that the
   format has not had an independent review. Type a passphrase and press
   Enter, then type it again to confirm. If the two differ, you start again
   and nothing is written.

The encrypted file is written and the tab now belongs to it: every later
save of that tab is encrypted with the same passphrase, which is held in
memory until you close the tab. The row then reads "Encrypted (.bpadx)".

**The original, unencrypted file is left where it was.** Delete it yourself
if you no longer want it, bearing in mind that deleting a file does not
securely erase it from the disk.

**To open an encrypted document**, open it as any other file. The editor
recognises it by its contents, asks for the passphrase in the bar, and opens
the tab only once it is unlocked. A wrong passphrase and a damaged file give
the same message: the format cannot tell them apart. Escape abandons the
entry.

Things to know:

- **Forgetting the passphrase loses the document.** There is no recovery.
- **Save As** keeps an encrypted document encrypted, whatever name you give
  it. Keep the `.bpadx` extension so that you and other programs can tell
  what the file is.
- **Save a Copy** of an encrypted document is encrypted too, with the same
  passphrase, and is given the `.bpadx` extension. There is no way to write
  a decrypted copy from inside the program.
- Crash recovery for an encrypted document is sealed with its passphrase;
  see [Recovering after a crash](#recovering-after-a-crash).

## Secrets, metadata and hashes

- **Security ▸ Scan for Secrets** looks for things that resemble passwords,
  API keys, tokens and private keys. It reports how many it found, of what
  kinds, and their line numbers; it **never shows the matched text**.
- **Security ▸ Redact Found Secrets...** replaces each finding with
  `[REDACTED: kind]`. A confirmation lists what will be replaced and says
  plainly that this changes the document, not the file: until you save, the
  file on disk still holds the originals, and so may the recovery journal,
  the undo history and the clipboard. A private key block is not redacted
  (only its first line is detected, and removing that alone would leave the
  key); the editor says so.
- **Security ▸ Inspect Metadata** reports metadata found in the text (by kind
  and line, never its value). For formats it cannot look inside, such as
  `.docx` or `.pdf`, it says so rather than reporting that nothing was
  found.
- **Security ▸ Hash Document (SHA-256)** shows the document's SHA-256 digest,
  computed over the bytes the file would be saved as, so it matches
  `sha256sum` or `Get-FileHash` on the saved file. It warns when you have
  unsaved changes.
- **Security ▸ Security History...** lists what the rows above, profile
  changes and Privacy Mode have recorded. Nothing is recorded under
  Confidential, Maximum or Privacy Mode.

## Signing and verifying

Signatures use Ed25519. A signature is a small text file beside the
document, named by adding `.sig` to the document's full name: `report.txt`
is signed by `report.txt.sig`.

**To sign**, save the document, then choose **Security ▸ Sign Document...**.
A document that has never been saved, or has unsaved changes, cannot be
signed, because the signature covers the file on disk.

- The first time, there is no signing key yet. The passphrase bar says it is
  about to create one and asks for a new passphrase twice. The key is kept
  **sealed with that passphrase** in `signing-key.bpadx` in the data
  directory (see [Settings and where things live](#settings-and-where-things-live)),
  and its public half is written beside it as `signing-key.pub`.
- Later, Sign Document asks for that passphrase. A wrong one keeps the bar
  open.
- The status bar names your public key. Give `signing-key.pub` (or the hex
  text inside it) to anyone who needs to check your signatures.
- **If you forget the key's passphrase, you need a new key**, and signatures
  made with the old one can no longer be matched to a key you control.

**To verify**, open the signed document and choose **Security ▸ Verify
Signature...**. The editor looks for the `.sig` file beside it; a missing
signature file is a failure, not a pass. If the signature holds together,
you are asked for the signer's public key file. Choosing one tells you
whether the document was signed by that key; cancelling leaves the answer
at "intact, signed by the key named in the signature file", with a warning
that anyone who changes a document can sign it again with their own key.

A valid signature says that a particular key signed these bytes. It does not
say whose key that is: you need to know that the public key you chose really
belongs to the person you think it does.

## Setting BachelorPad+ as the default editor

**File ▸ Set as Default Editor...** offers the "Notepad Replacement" set of
file types: `.txt`, `.text`, `.log`, `.ini`, `.conf`, `.cfg` and `.bpadx`. It first shows what opens each of
those types today, then what it will do, and does nothing until you confirm.

**On Linux**, confirming writes a per-user desktop entry and file-type
definition (unless an installed package already provides them), then runs
`update-desktop-database`, `update-mime-database` and `xdg-mime default`,
and reports the result of each. Double-clicking a text file in your file
manager then opens BachelorPad+.

**File ▸ Remove Registration...** (Linux only) undoes that: it removes only
the files Set as Default wrote, and leaves your `mimeapps.list` choices
alone. It is greyed when there is nothing of its own to remove, for example
after a package install. In a Flatpak both rows are greyed, because the
sandbox manages file types itself.

**On Windows**, the editor never writes the registry itself. The installer
registers it; the row then opens **Settings ▸ Default apps**, at
BachelorPad+'s own page on Windows 11 and at the general page on Windows
10, where you choose it. A portable copy has no installer, so the row
instead offers **Save Files...**, which saves a pair of `.reg` files: one
that registers the editor and one that removes the registration. Read them,
then run the first to register. Registration never touches another
program's settings.

The Windows behaviour has not yet been tested on a Windows machine.

## Recovering after a crash

While you work, unsaved changes are written to a **recovery journal** every
few seconds, for documents whose profile allows it. A normal save or close
removes them.

If the editor stops without saving, for example because of a crash or a
power cut, the next start shows **Unsaved work recovered**, listing the
documents:

- **Restore** opens them, marked as unsaved.
- **Discard** deletes the recovered work for good.
- **Not Now** keeps it and asks again next time. Escape does the same.

A second window running at the same time never offers or deletes the first
window's recovered work.

**Encrypted documents** are different: their recovered work is sealed with
the document's passphrase, so nothing can show it at startup. Instead the
status bar says how many encrypted documents have recovered work, without
naming them. Open the document and unlock it, and its recovered work is
restored into the tab. Moving or renaming an encrypted file loses its
recovered work.

There is no recovery journal under Maximum, in Privacy Mode, or under
Private or Confidential for a document that is not encrypted.

## Settings and where things live

**Tools ▸ Settings...** (Ctrl+,) opens the settings sheet: Theme, Font size,
Word wrap, Line numbers, Indent with, Tab width, Security profile for new
documents, Clipboard history records other programs, Largest file opened for
editing, and Custom editor view. Up and Down choose a setting, Left and
Right change it, and **Save** writes your changes.

Settings are kept in a `config.toml` file. The editor writes only what you
change and keeps anything you wrote in the file by hand, including comments.
A mistake in the file produces a warning in the status bar, never a refusal
to start.

| What | Windows | Linux |
| --- | --- | --- |
| Settings: `config.toml` | `%APPDATA%\bachelorpad\` | `$XDG_CONFIG_HOME/bachelorpad/`, usually `~/.config/bachelorpad/` |
| Recent files, security history (`security-history.log`), document profiles (`document-profiles.json`), logs and crash reports (`logs\`) | `%LOCALAPPDATA%\bachelorpad\state\` | `$XDG_STATE_HOME/bachelorpad/`, usually `~/.local/state/bachelorpad/` |
| Recovery journal (`recovery\`), signing key (`signing-key.bpadx`, `signing-key.pub`) | `%LOCALAPPDATA%\bachelorpad\data\` | `$XDG_DATA_HOME/bachelorpad/`, usually `~/.local/share/bachelorpad/` |

Only the settings roam with a Windows profile. **Tools ▸ Open Config
Folder** and **Tools ▸ Open Log Folder** open those folders. The
`BACHELORPAD_CONFIG` environment variable points the editor at a different
settings file.

Where a setting is given in more than one place, the first of these wins:
the command line, then environment variables (`BACHELORPAD_THEME`,
`BACHELORPAD_RENDERER`, `BACHELORPAD_LOG`, `BACHELORPAD_FONT_SIZE`,
`BACHELORPAD_TAB_WIDTH`, `BACHELORPAD_INDENT_SPACES`), then `config.toml`,
then the defaults. A setting given on the command line applies to that run
only and is never saved.

## The command line

```text
bachelorpad [OPTIONS] [--] [FILE]...
```

Each FILE opens in a tab. The main options:

| Option | Effect |
| --- | --- |
| `--line N` | put the caret on line N of the first file |
| `--theme NAME` | colour theme for this run (`Light`, `Dark`, `Organic`, `Green`, or `System`) |
| `--font-size POINTS`, `--tab-width COLUMNS`, `--indent-spaces true\|false` | editor settings for this run |
| `--editor-view` | use the experimental custom editor view |
| `--help`, `-h` / `--version`, `-V` | print help or the version and exit |
| `--` | treat every later argument as a file, even one starting with `-` |

`--readonly` and `--large-file` are accepted but not applied yet; the
program says so when you use them. `bachelorpad --help` lists every option.

## Keyboard shortcuts

**Help ▸ Keyboard Shortcuts** shows the main ones in the program.

| Keys | Action |
| --- | --- |
| Ctrl+N | New document |
| Ctrl+O | Open |
| Ctrl+S | Save |
| Ctrl+Shift+S | Save As |
| Ctrl+W | Close tab |
| Ctrl+Tab / Ctrl+Shift+Tab | Next / previous tab |
| Ctrl+F | Find and replace |
| Ctrl+G | Go to line |
| Ctrl+Z / Ctrl+Y | Undo / Redo |
| Ctrl+X / Ctrl+C / Ctrl+V | Cut / Copy / Paste |
| Shift+Delete / Ctrl+Insert / Shift+Insert | Cut / Copy / Paste |
| Ctrl+A | Select all |
| Ctrl+D | Duplicate line |
| Alt+Up / Alt+Down | Move line up / down |
| Insert | Overwrite on / off |
| Ctrl+= (or Ctrl++) / Ctrl+- / Ctrl+0 | Zoom in / out / reset |
| Ctrl+, | Settings |
| Escape | Close the find, go-to-line or passphrase bar and return to the text |
| F10 | Open the menu bar at File |
| Alt+F, E, V, I, O | Open File, Edit, View, Insert, Format |
| Alt+D, N, S, T, H | Open Data, Note, Security, Tools, Help |

**In an open menu:** Up and Down move between rows, skipping greyed ones and
wrapping round; Left and Right move to the neighbouring menu; Enter chooses
the row; Escape closes the menu and returns to the text.

**In a question dialog:** Enter chooses the highlighted button and Escape
chooses the safe one (Cancel, or Not Now).
