# Contributing to BachelorPad+

This file is for people working on BachelorPad+ itself: building it, running
the gate, and finding their way around the repository and its records. If
you want to use the editor, start with the [README](README.md) and the
[user guide](docs/user/GUIDE.md).

Security problems are not reported here; see [SECURITY.md](SECURITY.md).

## Build and run

No release has been published yet, so today you build from source. The
packaging exists: `packaging/windows/` builds the per-user Inno Setup
installer and the portable zip (not yet run on Windows), and
`packaging/linux/` builds the `.deb`, `.rpm`, `.tar.gz` and AUR `PKGBUILD`
inside a pinned Debian 12 container. `scripts/Invoke-Release.ps1` drives
both and records each run under `artifacts/releases/` (ADR-0016
amendment). Each directory's README says how.

The toolchain is pinned in `rust-toolchain.toml` (Rust 1.94.1, with rustfmt
and clippy); rustup picks it up by itself.

```sh
cargo run --release                       # the app
cargo run --release -- --editor-view      # the experimental custom editor view (ADR-0018)
cargo run --release -- --line 42 notes.txt
cargo run --release -- --help             # every command-line option
```

A release build on Windows is a GUI-subsystem binary. `--help` and
`--version` attach to the parent console through `bp-platform-windows`
(ADR-0033, release plan M2-03) so that they print when run from a shell;
this has not yet been run on a Windows machine. A debug build and Linux print
them directly.

**Linux.** Building needs a C toolchain and the development headers listed in
[docs/governance/LOCAL_CI.md](docs/governance/LOCAL_CI.md) ("Native Linux";
on Ubuntu, `build-essential pkg-config libfontconfig1-dev libxkbcommon-dev`
and the rest of that line). At run time the binary links `libfontconfig` and
`libfreetype`, and loads `libxkbcommon-x11` (plus X11 or Wayland client
libraries) when the window opens; install `libxkbcommon-x11-0` if your
desktop does not already have it. The editor's font, DejaVu Sans Mono, is
bundled, so no font package is needed. Message dialogs are drawn in the
window; file pickers go through the XDG desktop portal.

**Windows.** rustup's MSVC toolchain and the Visual Studio C++ build tools
(`bp-storage` compiles its bundled SQLite from C). Nothing built since the
sixth session has been run on Windows yet (release plan M0-12).

## Development

```powershell
./scripts/Install-GitHooks.ps1   # once per clone
./scripts/Invoke-LocalCI.ps1     # the gate: format, locked check, clippy, test, launch, log hygiene, fuzz
```

On Linux run the same gate under PowerShell 7, natively:
`pwsh -NoProfile -File scripts/Invoke-LocalCI.ps1`.

This project has **no hosted CI**. `scripts/Invoke-LocalCI.ps1` is the
authoritative gate and runs from `pre-commit` and `pre-push`. See
[docs/governance/LOCAL_CI.md](docs/governance/LOCAL_CI.md) and ADR-0016.
Releases are built locally too, not by a hosted workflow (release plan
decision 19).

```sh
scripts/smoke-linux.sh           # Linux: drive the built debug binary on Xvfb, both views
./scripts/Measure-Startup.ps1    # Windows: startup and idle memory vs specs.md §22
```

The smoke script is not a gate stage. It tests the binary it is given and
builds nothing, and it needs `xvfb`, `python3-xlib` and `libxkbcommon-x11-0`.

Never share one `CARGO_TARGET_DIR` between two worktrees of this repository:
cargo names workspace crates by their path relative to the workspace root,
so one tree's build is silently reused by the other.

## Working with coding agents

Much of this repository is written by coding agents working in parallel,
under rules that keep them out of each other's way.

- [project/WORK_QUEUE.md](project/WORK_QUEUE.md) lists what is ready to take,
  how to take an item, and the collision rules (which files one agent at a
  time may touch).
- [project/RELEASE_PLAN.md](project/RELEASE_PLAN.md) is the road to RC1: the
  milestones, their lanes, and the owner's decisions.
- [project/NEXT_SESSION.md](project/NEXT_SESSION.md) is the plan for picking
  the work up again.
- [prompts/rosettas/](prompts/rosettas/) holds the reusable task prompts.
  Read `R011_WIRE_CAPABILITY_INTO_SHELL.md` before touching `crates/bp-ui/`.

## Current state (2026-09-25)

**24 crates, over 2,000 tests, green on Linux** (the sixth and seventh
sessions' changes have not yet been through the Windows leg). The app opens, edits and
saves atomically, and does rather more than that:

| Area | What works |
| --- | --- |
| Editing | Tabs, four themes plus Follow System, zoom, indentation (tabs or soft tabs, 2/4/8), honest save state, atomic save, undo/cut/copy/paste (and Shift+Insert / Ctrl+Insert / Shift+Delete), Insert toggles overwrite (`OVR` in the status bar), Page Up/Down and a view that follows the caret, sort / deduplicate / reverse / trim lines, duplicate and move lines (Ctrl+D, Alt+Up/Down), go to line (Ctrl+G), line and column in the status bar -- all in the default view, not only `--editor-view` |
| Storage | A rope buffer holds every document; whole-document operations are one undo step |
| Safety | Unsaved-changes questions asked inside the window (Save / Don't Save / Cancel; no dependency on zenity or `MessageBoxW`), external-change detection, crash recovery journal written only when a document has changed — encrypted and recovered at unlock for encrypted documents ([ADR-0022](docs/decisions/ADR-0022.md)) |
| Files | Open (several at once)/Save/Save As/Save All/Save a Copy/Save as New Revision/Reload, a Save As suggestion that never names a file already there, recent files, tab context menu, Ctrl+Tab, command-line file opening. UTF-8, UTF-8 with BOM and **UTF-16 LE/BE** all load and save; a truncated or malformed UTF-16 file is refused by name rather than repaired with replacement characters. A file that is not UTF-8 and has no byte-order mark opens as **Windows-1252** and saves back byte for byte; a character Windows-1252 cannot hold stops the save and offers UTF-8 rather than writing a `?` |
| Search | Find and replace with case-sensitive, whole-word and regex toggles, changes shown before they are applied, recursive cross-file search |
| Data | JSON / JSONL / TOML / **YAML** validate, format, minify, convert; RFC 4180 CSV/TSV shape report, conversion to JSON and JSON Lines, column types. YAML refuses deep nesting, alias bombs and duplicate keys in words that say what to do ([ADR-0023](docs/decisions/ADR-0023.md)) |
| Semantic | Title, keywords, summary and outline extracted from the document; document statistics; date and time insertion |
| Clipboard | History with kind detection, paste from history, format-aware paste transformations |
| Metadata | A SQLite store with migrations — built and tested, not yet wired in ([ADR-0019](docs/decisions/ADR-0019.md)) |
| Encryption | `.bpadx` documents — Security ▸ Encrypt Document, unlock on open, and saves stay encrypted. Argon2id, XChaCha20-Poly1305 or AES-256-GCM, chunked with position authenticated ([ADR-0021](docs/decisions/ADR-0021.md)) |
| Security | Per-document profiles (Standard / Private / Confidential / Maximum) governing the recovery journal, clipboard history and metadata store ([ADR-0020](docs/decisions/ADR-0020.md)), remembered per file between sessions under a digest of its path. What each axis does and does not do: [SECURITY_MODEL.md](docs/architecture/SECURITY_MODEL.md) |
| Settings | Tools ▸ Settings... (Ctrl+,) and the View and Format menus save into your own `config.toml`, keeping what you wrote there by hand; only what you changed is written |
| Security (phase 16) | Privacy Mode, a session override that can only tighten; Scan for Secrets, which reports where a credential is and never what it is; Redact Found Secrets, as an undoable edit with a consent step; Inspect Metadata; Hash Document; Sign Document, with the key sealed in a passphrase envelope in the data directory (release plan M2-10); Verify Signature; Security History, which every row above it writes into (ADR-0024) |

Startup, with the software renderer ([ADR-0017](docs/decisions/ADR-0017.md)):
**35.7 ms to window, 21.9 MB idle**, against targets of 150 ms and 50 MB,
measured on Windows 10 on 2026-08-19
(`artifacts/benchmarks/startup-20260819-053242.json`). Idle memory had risen
about 2.7 MB when `std-widgets` was first imported. Not measured since the
in-window dialogs, Settings, About and the bundled font landed, and never on
Linux or Windows 11 (release plan M4-01 to M4-03).

Configuration precedence: command line (`--theme=`, `--renderer=`, `--log=`,
`--font-size=`, `--tab-width=`, `--indent-spaces=`),
environment (`BACHELORPAD_*`), a TOML file
(`%APPDATA%\bachelorpad\config.toml`, or `$XDG_CONFIG_HOME` on Linux), then
defaults. Broken config warns and falls back; it never stops the editor
starting. Choices made in the window are saved back into that same file,
comments and all, and a one-off command-line override is never made
permanent.

Only settings roam. The recent list and the security history are kept in
`%LOCALAPPDATA%\bachelorpad\state` (`$XDG_STATE_HOME/bachelorpad`), and the
recovery journal in `%LOCALAPPDATA%\bachelorpad\data\recovery`
(`$XDG_DATA_HOME/bachelorpad/recovery`). A build that kept them beside
`config.toml` has them moved on first launch; a move that fails leaves them
where they are, in use, and says so ([ADR-0029](docs/decisions/ADR-0029.md)).

### Known gaps, in detail

The [README](README.md#known-gaps) has the short, user-facing list. This is
the developer's version.

- **Tested in a window, and on Windows by nobody yet.** `bp-ui`'s window
  tests build the real window on a headless software renderer and drive it
  with real key events: navigation, overwrite, every in-app question, the
  Settings sheet, Ctrl+S end to end. That is how Page Up doing nothing, a
  question from a menu losing the keyboard, and Find selecting the wrong text
  after accented letters were found. Every change has passed the Linux gate;
  **the Windows leg has not been run** (release plan M0-12).
- **Input-method composition does not work under `--editor-view`, and cannot
  on this Slint.** `FocusScope` rejects `UpdateComposition` and
  `CommitComposition` in both its handlers and exposes no callback for either;
  `TextInput` is the only item in the toolkit that consumes them. Without it
  CJK entry does not work at all, so the default stays `TextInput`. **This was
  the second of two parity conditions, and it is now a blocker rather than a
  task** -- see `project/WORK_QUEUE.md`.
- **Drag and drop to open does not work, and cannot yet.** specs §4 wants it.
  On Slint 1.17.1 the winit backend has no file-drop plumbing and
  `DataTransfer` carries only plain text or an image, so there is no channel a
  dropped file's path could arrive through. Blocked on Slint, not on effort.
- **The crates are tested where they meet, and that is where the defects
  are.** Eleven files of cross-crate tests join load/rope/atomic save, the
  `.bpadx` envelope over a real file, the security profile against all three
  of its dependants, format detection against the parser that then handles
  the file, search against the buffer, journal recovery, the notebook through
  the file layer, the audit log, redaction, signing, and the filename grammar
  against the platform's own rules. Between them they have found seven
  defects that no unit test did. **Six are fixed, and the seventh is a
  decision rather than a defect.** The most serious was `bp-naming` asked whether a *whole* sanitised title was a Windows
  device name, while Win32 asks only about the stem before the first dot --
  so `con.txt`, `aux.log` and `NUL.dat` passed untouched, and on Windows
  saving to one of those writes **the device**, reports success, and the
  document is gone. The sanitiser now tests the stem, its list gained
  `CONIN$` and `CONOUT$`, and a property compares its verdict against
  `bp-platform`'s stem by stem in both directions so the two lists cannot
  drift apart quietly.

  **The other half of that hole was a name nobody sanitised.** `bp-naming`
  defends the name the product *suggests*; the one a user types over the top
  of it in a Save dialog had never been checked by anything. `bp-files`
  depends on `bp-platform` now and `atomic_write` refuses a device name
  outright -- the only refusal in that module that is checked rather than
  attempted, because attempting it does not fail: the open succeeds, the
  read-back verifies against the console, and the status bar reports a
  document that is nowhere. The platform is a parameter rather than a `cfg`,
  so `con.txt` stays an ordinary file on Linux and both CI legs execute both
  rule sets.

  The sixth session fixed the two that were left. `bp-notebook` could not
  tell a cell that said nothing about its language from one that named a
  language it cannot map, and ran the second through the kernel's
  interpreter; it now keeps such a cell `Raw`, with a warning, as ADR-0025
  always said. And `bp-files` never applied `\\?\`, so a filename at
  `SemanticName::to_filename`'s own maximum was unwritable in a deep enough
  directory and blamed on a read-only file -- `std::fs` lengthens long paths
  by itself, but `tempfile`'s rename calls `MoveFileExW` with the path as
  given. The last `#[ignore]`d test, on what a line is, runs again: D6 was
  answered on 2026-09-26 with the Notepad rule (a line ends at LF, CRLF or a
  bare carriage return, and nothing else, everywhere). `bp-buffer`'s rope is
  built with ropey's `cr_lines` and `bp-search` counts the same set, so Find
  in Files and the editor agree.
- **Three crates and one module are built and unreachable, and this list is
  now counted rather than remembered.** `bp-research`, `bp-notebook` and
  `bp-storage` have zero reverse dependencies anywhere in the application:
  nothing in `bp-ui`, in `apps/bachelorpad`, or in any other `bp-*` crate
  names them. `bp-notebook` is reached only by the fuzz harness and the
  cross-crate tests, `bp-storage` only by the latter. `bp-audit`
  came off this list when Security ▸ Security History shipped. Add `bp-buffer`'s
  large-file engine -- depended on, and its `SizeClass` now drives the open
  path's size guard, but the `LargeFile` viewer appears nowhere outside its
  own crate, so a file past the 32 MiB editable limit is refused with a
  notice rather than opened (the viewer is release-plan item M1-06) --
  and it is over ten thousand lines, with their unit tests, that no user
  can reach. It is still the largest thing standing between this
  repository and a product.

  | Crate | Reached by |
  | --- | --- |
  | `bp-research` | nothing (unwired for RC1, ADR-0030) |
  | `bp-notebook` | the fuzz harness and cross-crate tests only (1.1, ADR-0030) |
  | `bp-buffer` ▸ `large.rs` | nothing yet (the viewer, M1-06) |
  | `bp-storage` | the cross-crate tests only, by decision (ADR-0019; kept unwired for RC1 by ADR-0030) |

  Line and test counts are left out because they drift.

  `bp-integrity`, `bp-platform` and `bp-redaction` came off this list --
  Verify Signature, Set as Default Editor, redaction and the metadata
  inspector all have rows now.
- **Still barely used by a person.** The tests cover the pieces and the
  seams, the window tests drive the real window, and `scripts/smoke-linux.sh`
  drives the real binary on Xvfb. A person driving the application is
  recorded once, on 2026-08-17. The Security menu's rows (Scan for Secrets,
  Redact, Inspect Metadata, Hash, Verify Signature, Security History) are
  tested through `bp-ui`'s state, not through the window, and no record says
  a person has clicked them. The [RC checklist](docs/release/RC_CHECKLIST.md)
  is where that changes.
- **`bp-storage` is still not called by the application.** That is now a
  product decision rather than a security one: ADR-0020 permits recording a
  summary under Standard, and `record_document` honours the policy. What
  remains is ADR-0019's judgement plus a design pass.
- **A plaintext document under Private or Confidential gets no crash
  recovery.** The journal for those profiles is sealed with the document's
  own passphrase ([ADR-0022](docs/decisions/ADR-0022.md)), so a document
  that is not encrypted has no key to use. The status bar says what fixes
  it: encrypt the document.
- **Recovery for an encrypted document is invisible until you open it.** Its
  journal is filed under a digest of its path and can only be read once you
  have unlocked the document. Nothing prompts for a passphrase at startup --
  deliberately -- but a notice counts the encrypted documents with recovered
  work, without naming them (release plan M2-15).

## Where to read more

[ROADMAP.md](ROADMAP.md) has per-phase status;
[MENU_MAP.md](docs/product/MENU_MAP.md) says which menu rows are real;
[ARCHITECTURE.md](docs/architecture/ARCHITECTURE.md) has the crate map and the
editor-view constraint; [WORK_QUEUE.md](project/WORK_QUEUE.md) lists what is
ready to take and what cannot run in parallel;
[NEXT_SESSION.md](project/NEXT_SESSION.md) is the plan for picking this up
again. [DECISIONS.md](DECISIONS.md) is the decision register and
[docs/decisions/](docs/decisions/) holds the ADRs.

## Governance hierarchy

Where logical and useful, work follows:

```text
project
  -> task
    -> step
      -> prompts/rosettas
        -> artifacts
          -> metadata
            -> database
              -> note
```

See `docs/governance/WORK_MODEL.md`.
