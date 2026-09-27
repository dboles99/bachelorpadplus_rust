# Release candidate checklist

The run-through a person does on each operating system before a release
candidate is published (release plan item M4-04, exit criterion C16). The
automated gate proves the pieces; this proves the product, on a real machine,
the way a stranger would meet it.

Run the whole list once per environment, on the build that will be tagged.
An environment whose run is not complete, or has a failure that is not
fixed, is left out of that tag's release assets and listed as "not yet" in
the release notes.

Every tag's release notes also carry the Windows 10 support statement from
the README's Requirements section (release plan decision 21, W8): supported
on 22H2 with ESU or LTSC, the same binary as Windows 11, tested at each
release candidate, and no Windows-10-only fixes after 1.0.

## Environments

| ID | Environment | Notes |
| --- | --- | --- |
| `W11` | Windows 11 23H2 or 24H2, clean VM | **Smart App Control set to On** before the installer is copied over. No Visual C++ redistributable installed |
| `W10` | Windows 10 22H2, clean VM | No Visual C++ redistributable installed |
| `GW` | GNOME on Wayland (Ubuntu 24.04 or Fedora) | IBus for the input-method step |
| `KW` | KDE Plasma on Wayland (Fedora KDE or Arch) | Fcitx5 or IBus for the input-method step |
| `X11` | One X11 desktop (Xfce or Linux Mint Cinnamon) | |

Where a step differs by system, the table says which environments it
applies to. "Win" means `W11` and `W10`; "Linux" means `GW`, `KW` and `X11`.

## Recording evidence

1. Make a folder for this run:
   `artifacts/test-evidence/<tag>/<env>/`, for example
   `artifacts/test-evidence/v1.0.0-rc.1/W11/`. (Keep evidence in the
   per-tag subfolder: `.gitignore` ignores JSON files placed directly in
   `artifacts/test-evidence/`.)
2. Copy the [results template](#results-template) below into that folder as
   `results.md`, and fill in the header.
3. For each step, save the evidence named in the step (a screenshot, a
   terminal transcript, or a short text note) into the folder, named after
   the step, such as `04-recover.png`. Fill in the step's row in
   `results.md`: pass, fail or blocked, and a sentence on anything unexpected.
4. Record each evidence file's provenance with
   [`scripts/New-ArtifactMetadata.ps1`](../../scripts/New-ArtifactMetadata.ps1),
   which writes a JSON record (path, SHA-256, time, status) into `metadata/`:

   ```powershell
   pwsh -NoProfile -File scripts/New-ArtifactMetadata.ps1 `
       -Project BachelorPad+ -Task M4-04 -Step W11-04 `
       -ArtifactPath artifacts/test-evidence/v1.0.0-rc.1/W11/04-recover.png `
       -Status pass
   ```

   Use the step ID prefixed with the environment ID as `-Step`, and `pass`,
   `fail` or `blocked` as `-Status`. Run it for `results.md` too, with the
   step `<env>-results`, once the run is finished.
5. Commit the evidence folder and the new `metadata/*.json` records
   together, with the environment and tag in the commit message.

A failure is not a reason to stop: finish the list, so one run reports
everything, then file the failures.

## Test files

Prepare these before starting, outside the evidence folder:

- `plain.txt`: a few lines of UTF-8 text.
- `ansi.txt` (Win): a file saved from Windows Notepad with the encoding set
  to ANSI, containing `café`, `€` and curly quotes.
- `cjk.txt`: empty; the input-method step types into it.
- `big.log`: about 1 GB of numbered lines. On Linux,
  `seq -f 'line %.0f of the large-file test' 1 30000000 > big.log`; on
  Windows, copy the one made on Linux. Note its SHA-256 (`sha256sum` on
  Linux, `Get-FileHash` in PowerShell) before step 11.
- `cr.txt`: three lines, `one`, `two` and `three`, separated by bare
  carriage returns and nothing else (an old Mac file). On Linux,
  `printf 'one\rtwo\rthree' > cr.txt`; in PowerShell,
  ``[IO.File]::WriteAllText("$PWD\cr.txt", "one`rtwo`rthree")``.

## The steps

### 01 Install

| Env | Do | Expected |
| --- | --- | --- |
| Win | Copy the installer to the VM and run it | No Smart App Control or SmartScreen block on `W11`; the publisher shown is the signing identity. No administrator (UAC) prompt: it is a per-user install. It finishes without error |
| Win | Unzip the portable zip into a folder of your choice and run `bachelorpad.exe` from it | It starts. The folder also holds the `bpad.cmd` shim, `LICENSE-MIT`, `LICENSE-APACHE`, `THIRD-PARTY-NOTICES.txt` and `README.txt`, and **no** `.reg` files: a registration has to name where the program is, so step 07 saves them with File ▸ Set as Default Editor... ▸ Save Files... |
| Linux | Install the package for the system: `sudo apt install ./<file>.deb`, `sudo dnf install ./<file>.rpm`, `makepkg -si` from the `PKGBUILD`, or unpack the `.tar.gz` | The package manager resolves every dependency; nothing is reported missing. BachelorPad+ appears in the application menu with its icon |
| All | In a terminal, run `bachelorpad --version`, then `bachelorpad --help` (on Windows, from `cmd` or PowerShell) | The version matches the tag; the help text lists the options. On Windows the text appears in the same console window, and no editor window opens |

Evidence: `01-install.png` (installer or package manager finishing),
`01-version.txt` (the terminal output).

### 02 First run

| Env | Do | Expected |
| --- | --- | --- |
| All | Start BachelorPad+ from the Start menu or application menu | A window with an empty "Untitled" tab opens within a second or two. The window, taskbar and Alt+Tab show the BachelorPad+ icon. No recovery question (nothing to recover) |
| All | Open Help ▸ About BachelorPad+ | The version and commit match the tag. Both licences, the Slint attribution and Notices are there |
| All | Open Tools ▸ Open Config Folder and Tools ▸ Open Log Folder | Each opens the folder the [user guide](../user/GUIDE.md#settings-and-where-things-live) names for this system. The log folder holds a log file |
| All | Press F10, then use the arrow keys, then Escape | The File menu opens; arrows move through rows and across menus; Escape returns to the text |

Evidence: `02-about.png`.

### 03 Open, type, save

| Env | Do | Expected |
| --- | --- | --- |
| All | Type a few lines into the Untitled tab and press Ctrl+S | A Save dialog opens, suggesting a name of the form `Title_DDMMMYYYY.txt` taken from the first line. Saving shows the full path in the status bar and the tab is no longer marked unsaved |
| All | Open the saved file in another editor or viewer | Exactly what was typed. Line endings are CRLF on Windows and LF on Linux |
| All | File ▸ Open `plain.txt`, add a line, close the tab | Save / Don't Save / Cancel is asked inside the window. Cancel keeps the tab; Save writes the file |
| All | Open `plain.txt`, change it in another program, then press Ctrl+S in BachelorPad+ | The status bar says it changed on disk; the save asks **Save Anyway / Cancel**, and Enter chooses Cancel |
| Win | Open `ansi.txt`, change one letter, save, and compare the file's bytes with the original | The status bar says Windows-1252; the characters display correctly; only the changed byte differs |
| All | Ctrl+F, find a word, then Replace All with another | A preview of the replacements appears before anything changes |

Evidence: `03-saved.png` (status bar with the path), `03-ansi.txt` (a byte
comparison, such as `fc /b` output) on Windows.

### 04 Kill and recover

| Env | Do | Expected |
| --- | --- | --- |
| All | Open a new tab, type a sentence, **wait at least ten seconds**, then kill the process: `taskkill /F /IM bachelorpad.exe` on Windows, `pkill -KILL bachelorpad` on Linux | The window disappears |
| All | Start BachelorPad+ again | **Unsaved work recovered** lists the document. Choose **Not Now** |
| All | Quit and start again | The same question is asked again: Not Now kept the work |
| All | Choose **Restore** | The document opens with the sentence, marked unsaved |
| All | Kill and restart once more, and choose **Discard** | The work is gone; the next start asks nothing |
| All | Start two windows (run `bachelorpad` twice), type in the second, wait, kill only the second, then start a third | The third offers the second's work. The first window, still running, is never asked about it and loses nothing |

Evidence: `04-recover.png` (the question), `04-restored.png`.

### 05 Encrypted document round trip

| Env | Do | Expected |
| --- | --- | --- |
| All | Open `plain.txt` and choose Security ▸ Encrypt Document..., saving as `secret` | The name becomes `secret.bpadx`. The passphrase bar opens; **its first prompt says the encryption format has not had an independent security review** |
| All | Enter a passphrase, then a different one to confirm | "those did not match" and the first question again; no file is written |
| All | Enter the passphrase twice correctly | `secret.bpadx` is written; the status bar says so; the Security menu row reads "Encrypted (.bpadx)". `plain.txt` is still there, unencrypted |
| All | Open `secret.bpadx` in another program | It is not readable text |
| All | Close the tab, open `secret.bpadx` with File ▸ Open, give a wrong passphrase, then the right one | The wrong one keeps the bar open with an error; the right one opens the text |
| All | Close the tab and open `secret.bpadx` from the recent files listed in the File menu | Expected once the fix lands: the passphrase bar opens as with File ▸ Open. **Known issue until then:** this path skips the passphrase flow. Record it as fail with "known issue" in the notes |
| All | Edit, press Ctrl+S, close and reopen | The change is there, and the file is still encrypted |
| All | Edit without saving, wait ten seconds, kill the process, restart | No recovery question names the document; a status-bar notice says one encrypted document has recovered work. Opening and unlocking `secret.bpadx` restores the edit, marked unsaved |

Evidence: `05-prompt.png` (the first prompt with the review note),
`05-reopened.png`.

### 06 Security profiles and Privacy Mode

| Env | Do | Expected |
| --- | --- | --- |
| All | Open `plain.txt`, choose Security ▸ Confidential | The status bar shows "Confidential". The readouts say the recovery journal is off until the document is encrypted and the clipboard history is not kept |
| All | Copy a line, then open the Edit menu | The clipboard history does not list it |
| All | Edit, wait ten seconds, kill, restart | Nothing is offered for recovery |
| All | Reopen `plain.txt` | It is still Confidential |
| All | Choose Security ▸ Private, copy a line, then check the system clipboard history (Win+V on Windows, Klipper on KDE) and paste into another program after one minute | The copy is not in the system clipboard history; after a minute the clipboard no longer holds it |
| All | Turn on Security ▸ Privacy Mode, then open another file | The status bar shows "Privacy Mode"; the file is not added to the File menu's recent files; nothing is added to Security ▸ Security History |
| All | Security ▸ Scan for Secrets on a document containing the line `client_secret = "Xk92mQvRt7LpZn3BwYs6HjD4fG8kMc1Vb5N"` | A finding is reported by line and kind; the value itself is never shown |

Evidence: `06-confidential.png` (the Security menu readouts),
`06-winv.png` or `06-klipper.png`.

### 07 Set as Default, and double-click from the file manager

| Env | Do | Expected |
| --- | --- | --- |
| Win (installed) | File ▸ Set as Default Editor... | The dialog lists what opens each file type now, then offers **Open Settings**. On `W11` Settings opens at BachelorPad+'s own Default apps page; on `W10` at the general Default apps page. Choose BachelorPad+ for `.txt` there |
| Win (installed) | In Explorer, look at a `.txt` file, then double-click it | Explorer shows the BachelorPad+ icon for it; the file opens in BachelorPad+ |
| Win (portable) | File ▸ Set as Default Editor... ▸ Save Files... | Two `.reg` files are saved: one registers, one removes. Importing the first, then the second, leaves no BachelorPad+ key under `HKCU\Software\Classes` (check with `reg query`) |
| Linux | File ▸ Set as Default Editor... | The dialog lists the current defaults, the files and every command it will run. After **Set as Default**, each command's result is reported, and what the desktop now says |
| Linux | Double-click a `.txt` file in Files (GNOME), Dolphin (KDE) or the X11 desktop's file manager | It opens in BachelorPad+ |
| Linux | File ▸ Remove Registration... | Greyed with the reason when the package installed the entry (nothing of its own to remove); otherwise it removes only the files Set as Default wrote |
| All | Double-click `secret.bpadx` (choose BachelorPad+ for `.bpadx` first if the system asks) | Expected once the fix lands: BachelorPad+ opens and asks for the passphrase. **Known issue until then:** a file opened from the command line, which is what a double-click is, skips the passphrase flow. Record it as fail with "known issue" in the notes |

Evidence: `07-dialog.png`, `07-explorer.png` or `07-files.png`.

### 08 HiDPI

Set the display scale in the system settings, restart BachelorPad+, and
check each scale in turn. On GNOME Wayland, 125 % and 150 % may need
fractional scaling turned on first (a switch in Settings ▸ Displays on
Ubuntu; mutter's `scale-monitor-framebuffer` experimental feature
elsewhere).

| Env | Do | Expected |
| --- | --- | --- |
| All | At **125 %**, **150 %** and **200 %**: open a document, the File and Security menus, the find bar, Tools ▸ Settings, Help ▸ About and a Save / Don't Save / Cancel question | Text is sharp, not blurred. Nothing is cut off; menus and dialog sheets fit inside the window; the status bar's path is readable. Clicking in the text puts the caret where you clicked |
| Win | Move the window between two monitors with different scales, if the VM allows | The window redraws at the new scale |

Evidence: `08-125.png`, `08-150.png`, `08-200.png`.

### 09 Input method

In the default editor view only. The experimental custom view is known not
to support input methods and is not tested here.

| Env | Do | Expected |
| --- | --- | --- |
| Win | Add Microsoft Pinyin (Chinese) or Microsoft IME (Japanese), switch to it, and type a phrase into `cjk.txt` | The composition appears as you type, the candidate window sits near the caret, and choosing a candidate inserts the characters |
| Linux | With IBus or Fcitx5 and a Pinyin, Anthy or Mozc engine, do the same | The same |
| All | Save, close and reopen `cjk.txt` | The characters are intact; the status bar says UTF-8 |

Evidence: `09-ime.png` (mid-composition, with the candidate window).

### 10 Screen reader smoke test

NVDA on Windows, Orca on Linux. The menus are exposed with the roles Slint
1.17 offers: a menu name is announced as a button that expands, and a row as
a list item.

| Env | Do | Expected |
| --- | --- | --- |
| All | With the screen reader running, start BachelorPad+ | The window title is read |
| All | Press F10, then Down and Right a few times, then Escape | Each menu name and row is read as focus moves; Escape returns to the text |
| All | Press Ctrl+S on an unsaved document, then Escape; then close a changed tab | The question sheet's title, text and buttons are read |
| All | Save a document | The status notice is read without interrupting |

Evidence: `10-reader.txt` (a short note of what was and was not read; NVDA's
speech viewer or Orca's log can be pasted in).

### 11 Large files: the read-only viewer

| Env | Do | Expected |
| --- | --- | --- |
| All | File ▸ Open `big.log` | A viewer tab opens within a few seconds, with a banner saying it is read-only and too large to edit, giving the file's size and the 32 MiB limit. The window stays responsive while the status bar reports indexing |
| All | Scroll with the wheel and the scrollbar, then press Ctrl+End, then Ctrl+Home | Lines appear as you scroll. Ctrl+End reaches the last line once indexing allows (`line 30000000 of the large-file test`, and the status bar agrees); Ctrl+Home returns to the first |
| All | Ctrl+G to line 15000000 | The caret is on `line 15000000 of the large-file test` |
| All | Ctrl+Home, then Ctrl+F for `line 29999999 of` and Enter | The find bar reports progress without freezing the window, then the match is shown highlighted. Escape during a second search stops it |
| All | Try typing and pasting, and open the File and Edit menus | Nothing changes; the first key says so in the status bar. Save, Save As and the editing rows are greyed with **read-only** |
| All | Close the tab and take the file's SHA-256 again | It matches the one taken before the step: the file is unchanged |

Evidence: `11-viewer.png` (the banner and the status bar after Ctrl+End),
`11-hash.txt` (both hashes).

### 12 Editor context menu

| Env | Do | Expected |
| --- | --- | --- |
| All | Open `plain.txt`, copy two different lines, then right-click in the text | A menu opens at the pointer with Cut, Copy, Paste, Select All, Paste Special..., Clipboard History, Pin and Clear Clipboard History. The selection and caret do not move |
| All | Escape, then press the Menu key (or Shift+F10 where the keyboard has none) | The same menu opens at the caret; the arrow keys move through it, Right opens a submenu, Escape closes it and returns to the text |
| All | Pin ▸ the first line you copied; then Clear Clipboard History; then open Clipboard History | The pinned line is still listed (ticked under Pin); the other is gone |
| All | Try each theme (View menu) with the context menu open | Note how the menu is drawn. **Known issue:** it is light under every theme; record what you see rather than failing the step for it |

Evidence: `12-context-menu.png`.

### 13 Old Mac line endings

| Env | Do | Expected |
| --- | --- | --- |
| All | File ▸ Open `cr.txt` with line numbers on (View menu), then Ctrl+End | Three numbered lines, `one`, `two` and `three`; the status bar says line 3 |
| All | Ctrl+G to line 2 | The caret is on `two` |
| All | Find in Files for `three` over the folder holding `cr.txt`, then open the result | The result says line 3, and opening it puts the caret on `three` |
| All | Note the line ending the status bar and Format menu show; choose Format ▸ LF, save, and look at the file's bytes | **Known gap (open decision D11):** the file is labelled with the system's default line ending, and the saved file still separates its lines with bare carriage returns. Record what you see; this is not a failure until D11 is decided |

Evidence: `13-cr.png` (line numbers and status bar), `13-bytes.txt` (a hex
dump of the saved file, such as `xxd` or `Format-Hex` output).

### 14 Uninstall

| Env | Do | Expected |
| --- | --- | --- |
| Win (installed) | Uninstall from Settings ▸ Apps | It asks before deleting settings and data. Afterwards no BachelorPad+ value remains under `HKCU\Software\Classes` or `RegisteredApplications` (check with `reg query`); Explorer no longer shows its icon for `.txt` |
| Win (portable) | Import the remove `.reg` file, then delete the folder | Nothing of BachelorPad+'s remains in the registry |
| Linux | Remove Registration first if Set as Default wrote files, then remove the package (`apt remove`, `dnf remove`, `pacman -R`, or delete the unpacked folder) | The application menu entry and icons are gone; double-clicking a `.txt` no longer opens BachelorPad+ |
| All | Look in the settings, state and data folders | Settings and data are kept unless you chose to delete them; nothing was deleted without being asked |

Evidence: `14-uninstall.png`, `14-registry.txt` on Windows.

## Results template

Copy this into `artifacts/test-evidence/<tag>/<env>/results.md`.

```markdown
# RC checklist results

- Tag and version (`bachelorpad --version`):
- Commit (Help ▸ About):
- Environment ID (W11, W10, GW, KW, X11):
- OS name, version and build:
- Desktop and session (Wayland or X11), or Windows edition:
- Display scale at the start:
- Installed from (installer, portable zip, deb, rpm, tar.gz, PKGBUILD):
- Tester and date:

| Step | Result (pass / fail / blocked) | Evidence file(s) | Notes |
| --- | --- | --- | --- |
| 01 Install | | | |
| 02 First run | | | |
| 03 Open, type, save | | | |
| 04 Kill and recover | | | |
| 05 Encrypted round trip | | | |
| 06 Profiles and Privacy Mode | | | |
| 07 Set as Default, double-click | | | |
| 08 HiDPI 125 / 150 / 200 % | | | |
| 09 Input method | | | |
| 10 Screen reader | | | |
| 11 Large-file viewer | | | |
| 12 Editor context menu | | | |
| 13 Old Mac line endings | | | |
| 14 Uninstall | | | |

Failures filed as:
```
