# Windows bring-up: the first run on a real machine

Nothing built since the sixth session has run on Windows: not the gate,
not the Windows-only crate (`bp-platform-windows`), not the installer, not
the portable zip, not the signing step. This is the owner's run-book for
the first pass on a Windows 10 or 11 laptop, in the order that finds
problems cheapest first. It is release plan M0-12, the Windows half of
M3-01 and M3-03, and the start of M4.

Everything here runs as an ordinary user. Nothing needs administrator
rights except installing the tools in step 1.

## 1. Install the tools (once)

| Tool | Why | How |
| --- | --- | --- |
| Git | the clone | `winget install -e --id Git.Git` |
| PowerShell 7 | every script here | `winget install -e --id Microsoft.PowerShell` |
| Visual Studio 2022 Build Tools, "Desktop development with C++" | the MSVC linker, and `bp-storage` compiles SQLite from C | `winget install -e --id Microsoft.VisualStudio.2022.BuildTools`, then tick the C++ workload in the Visual Studio Installer |
| rustup | the toolchain; `rust-toolchain.toml` pins 1.94.1 and rustup fetches it on first use | `winget install -e --id Rustlang.Rustup` |
| cargo-deny | the gate's `deny` stage | `cargo install --locked cargo-deny` |
| cargo-about | the release script's notices check | `cargo install --locked cargo-about --features cli` |
| cargo-auditable, cargo-cyclonedx | the release build and its SBOM | `cargo install --locked cargo-auditable cargo-cyclonedx` |
| Inno Setup 6 | compiles the installer | `winget install -e --id JRSoftware.InnoSetup` |
| WSL with Ubuntu (optional) | the gate's `-Linux` leg | `wsl --install -d Ubuntu`, then inside it the Linux prerequisites in CONTRIBUTING.md |

Open a new PowerShell 7 window after installing, so `PATH` picks them up.

## 2. Clone and run the gate

```powershell
git clone https://github.com/dboles99/bachelorpadplus_rust
cd bachelorpadplus_rust
./scripts/Install-GitHooks.ps1
./scripts/Invoke-LocalCI.ps1          # add -Linux if WSL is set up
```

Expect the first run to take a while: it builds the workspace twice
(check, then clippy and test) and runs the fuzz regression tests.

**What to watch.** These have only ever been compiled for Windows, never run:

- `bp-platform-windows`, the one crate with `unsafe` (ADR-0033): the
  clipboard-history exclusion, the Settings deep link, the console attach.
- The launch stage asserts the exe's VERSIONINFO, icon and manifest.
- `bp-files`' long-path (`\\?\`) rename, and the ADR-0029 move of the
  recovery journal, recent list and security history into `%LOCALAPPDATA%`.
- The window tests in `bp-ui` (`window_tests.rs`), which run Slint's
  software renderer in-process.
- Whether `println!` after `AttachConsole` reaches the shell (`bachelorpad
  --version` from a release build).

If a stage fails, the gate stops and names it. Paste the failing stage's
output into the next session, or commit the log (step 5).

## 3. Use it for ten minutes

```powershell
cargo run --release
cargo run --release -- --editor-view
```

Type, open a few files, save, use the right-click menu, Encrypt and reopen
a copy, open a file over 32 MiB (it should open read-only in the viewer),
try Set as Default Editor. Write down what feels wrong, not only what
breaks: the custom view's caret against the pointer and the wheel direction
have never been checked by a person on either OS.

## 4. The release dry run and the install test

```powershell
./scripts/Invoke-Release.ps1 -Tag v1.0.0-rc.0 -DryRun
```

This runs the release checks and the gate again, builds with
`cargo auditable`, compiles the installer and the portable zip (unsigned; a
dry run does not ask for `-Sign`), writes the SBOM and SHA256SUMS, and
records `artifacts/releases/v1.0.0-rc.0/windows/manifest.json`. Add
`-SkipGate` if the gate has just passed on the same commit. ISCC has
never compiled `bachelorpad.iss`, so its `[Code]` Pascal is the likeliest
thing to fail here.

Then install, check and uninstall:

```powershell
./packaging/windows/Test-Install.ps1 -SetupPath .\artifacts\releases\v1.0.0-rc.0\windows\files\BachelorPadPlus-1.0.0-rc.0-windows-x64-setup.exe
```

It installs silently for the current user, checks every registry value,
file, shortcut and the optional PATH entry, uninstalls silently and checks
that everything it wrote is gone and your files are untouched. It refuses
to start if BachelorPad+ is already installed or registered, so run it
before (or after uninstalling) any copy you installed by hand. On a laptop
you use every day it is safe: the install is per-user, and a silent
uninstall never deletes user data. A spare VM is better still, and is what
the release checklist asks for later.

## 5. Bring the results back

Commit the evidence on a branch and push it, or paste it into the next
session:

- the gate's summary (the `PASS`/`FAIL` lines) and any failing stage's output;
- `artifacts/releases/v1.0.0-rc.0/windows/manifest.json` and `SHA256SUMS`
  (the `files/` folder is ignored by git and stays on the laptop);
- `Test-Install.ps1`'s summary, from the evidence folder it prints;
- your notes from step 3.

## 6. Later: signing

When a signing identity exists (decision 3, and question 3a in
`project/DECISIONS_NEEDED.md`), `packaging/windows/README.md` ("Signing")
has the setup. A real release is then:

```powershell
./scripts/Invoke-Release.ps1 -Tag v1.0.0-rc.1 -Sign
```

Secrets never go in the repository: the signing identity lives in
environment variables and Azure's own login on the signing machine.
