# Windows packaging

Release plan M3-01 and the Windows half of M2-16 (decision 18: an Inno Setup
per-user installer and a portable zip). Registration on Windows is the
installer's job, never the application's (ADR-0032).

| File | What it is |
| --- | --- |
| `bachelorpad.iss` | The Inno Setup script. Two blocks in it are generated -- see below |
| `bpad.cmd` | The `bpad` short command: runs `bachelorpad.exe` from its own directory with every argument, and returns its exit code. CRLF (`.gitattributes`) |
| `Build-Installer.ps1` | Compiles the installer from an already-built release exe |
| `New-PortableZip.ps1` | Builds the portable zip from the same exe |
| `PackagingCommon.ps1` | Shared by the two build scripts (version, licences, CRLF text) |
| `README-portable.txt` | The zip's `README.txt` |
| `Signing.ps1` | The Authenticode signing step (M3-04): finds signtool, signs through Azure Artifact Signing or an OV certificate, verifies with `signtool verify /pa`. Dot-sourced by `Invoke-Release.ps1` and `Build-Installer.ps1` |
| `Test-Install.ps1` | The install/uninstall acceptance test, for a person on a Windows VM |

## Building

Needs Inno Setup 6.3 or later (`winget install JRSoftware.InnoSetup`),
cargo on `PATH` (for `cargo metadata`), and `LICENSE-MIT` and
`LICENSE-APACHE` at the repository root -- both scripts stop if either is
missing. Windows PowerShell 5.1 or PowerShell 7.

```powershell
cargo build --release --locked -p bachelorpad
./packaging/windows/Build-Installer.ps1 -ExePath target/release/bachelorpad.exe -OutputDir out
./packaging/windows/New-PortableZip.ps1 -ExePath target/release/bachelorpad.exe -OutputDir out
```

Neither script runs cargo to build anything: the caller passes the exe it
built (and, for a release, signed). Both read the version from
`Cargo.toml` through `cargo metadata`, and on Windows both refuse an exe
whose version resource says otherwise, which catches a stale exe.

For the release script (M3-03):

| | `Build-Installer.ps1` | `New-PortableZip.ps1` |
| --- | --- | --- |
| Required | `-ExePath`, `-OutputDir` | `-ExePath`, `-OutputDir` |
| Optional | `-IsccPath`, `-Version`, `-OutputBaseName` | `-Version`, `-OutputBaseName` |
| Writes | `<OutputDir>/BachelorPadPlus-<version>-windows-x64-setup.exe` | `<OutputDir>/BachelorPadPlus-<version>-windows-x64-portable.zip` |
| Pipeline output | the installer's `FileInfo`, nothing else | the zip's `FileInfo`, nothing else |
| Failure | throws (non-zero exit under `pwsh -File`) | throws |

ISCC is found from `-IsccPath`, then `PATH`, then the default Inno Setup 6
locations (Program Files (x86), Program Files, `%LOCALAPPDATA%\Programs`).
`-Version` exists for a caller that has already read the version; it must
still match the exe. With `-Sign`, ISCC signs the setup and the uninstaller
it embeds as it writes them; the caller signs the exe beforehand
(`Invoke-Release.ps1 -Sign` does both).

## Signing

`Signing.ps1` holds the signing step (M3-04). It reads the identity from
environment variables on the signing machine, so no secret enters the
repository. For Azure Artifact Signing (decision 3):

```powershell
winget install -e --id Microsoft.Azure.ArtifactSigningClientTools
az login
$env:BP_SIGN_METADATA = 'C:\signing\metadata.json'
./scripts/Invoke-Release.ps1 -Tag v1.0.0-rc.1 -Sign
```

`metadata.json` is Microsoft's format:

```json
{
  "Endpoint": "https://<region>.codesigning.azure.net",
  "CodeSigningAccountName": "<account>",
  "CertificateProfileName": "<profile>"
}
```

For an OV certificate from a CA instead, set `BP_SIGN_CERT_SHA1` (the
thumbprint in the certificate store) and `BP_SIGN_TIMESTAMP` (the CA's
RFC 3161 URL). Setting both routes is an error. Every signature is checked
with `signtool verify /pa`, and a failure stops the release. A Windows
release without `-Sign` must say `-AllowUnsigned`, and its manifest records
`windows_signed: false`.

The zip holds one folder, named like the zip, with `bachelorpad.exe`,
`bpad.cmd`, both licence files and `README.txt`. The script reads the zip
back and fails if it holds anything else.

## The generated blocks

The `[Registry]` section and the list of the user's directories in `[Code]`
come from `bp_platform::editor::inno`, between `BEGIN GENERATED` and
`END GENERATED` lines. Do not edit them; change the generator and run

```sh
cargo run -p bp-platform --example regenerate_windows_installer
```

`crates/bp-platform/tests/windows_installer.rs` fails if either block differs
from the generator by a byte. The script is kept to ASCII, and the tests
also hold it per-user (`PrivilegesRequired=lowest`, no override, every entry
under `HKCU`).

## What the installer writes

Per user, with no administrator rights:

- the files, in `%LOCALAPPDATA%\Programs\BachelorPad+`: `bachelorpad.exe`,
  `bpad.cmd`, `LICENSE-MIT`, `LICENSE-APACHE`, and Inno Setup's uninstaller;
- a Start menu shortcut, and a desktop one if that task is ticked;
- `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{97D93154-2AAB-4EA8-BAB8-43B7869AA059}_is1`,
  Inno Setup's own uninstall entry (the `AppId` is fixed for the life of the
  product);
- the registration `bp_platform::editor::windows::registry_values` produces
  for `{app}\bachelorpad.exe` and the Notepad Replacement preset (plain
  text, logs, configuration files and `.bpadx`) -- the same preset File >
  Set as Default Editor uses:
  - `HKCU\Software\Classes\BachelorPadPlus.<ext>` -- one ProgID per type:
    name, `FriendlyTypeName`, `DefaultIcon`, `shell\open\command`;
  - one value, named for the ProgID, in `HKCU\Software\Classes\.<ext>\OpenWithProgids`;
  - `HKCU\Software\BachelorPad+\Capabilities` (name, description, icon,
    `FileAssociations`);
  - one value, `BachelorPad+`, in `HKCU\Software\RegisteredApplications`;
- only if "Add the bpad command to PATH" is ticked (it is not by default):
  the install directory appended to the user `PATH` (`HKCU\Environment`).

It never writes an extension's default value, a `UserChoice` key, anything
of another application's, or anything under `HKEY_LOCAL_MACHINE`
(ADR-0012; the generator refuses such a key). `ChangesAssociations=yes`
makes Setup send `SHChangeNotify(SHCNE_ASSOCCHANGED)` so Explorer notices.
The user still chooses the default in Settings > Apps > Default apps, where
BachelorPad+ now appears.

## What the uninstaller removes

- Every file it installed, the shortcuts, and its uninstall entry.
- The product's own keys, whole (`uninsdeletekey`): each
  `BachelorPadPlus.<ext>` ProgID and `HKCU\Software\BachelorPad+`.
- From each shared key, only the value it added (`uninsdeletevalue`): the
  ProgID value in `.<ext>\OpenWithProgids` and the `BachelorPad+` value in
  `RegisteredApplications`. The keys themselves stay, because other
  applications use them; one the installer created (for example
  `.bpadx\OpenWithProgids`) is left empty.
- Its entry in the user `PATH`, if there is one, leaving every other entry
  as it was.

Then, **only in an interactive uninstall**, it asks -- with No as the
default -- whether to delete the data, state and cache directories in
`%LOCALAPPDATA%\bachelorpad` (the recovery journal, the Sign Document key,
the recent-files list, the security history). The configuration directory,
`%APPDATA%\bachelorpad`, is never deleted: it roams with the Windows profile
(ADR-0029) and may belong to a copy installed on another computer. A silent
uninstall (`/SILENT`, `/VERYSILENT`) never deletes user data. The directory
paths are generated from `bp_platform::dirs`.

## The portable zip and registration

The zip contains no `.reg` files, which departs from the plan's wording
("portable zip with `bpad.cmd` and the `.reg` pair"). A `.reg` file has to
name the exe's absolute path, and nobody knows that path until the zip has
been unpacked, so a pair generated at build time would register a program
that is not there. `README.txt` tells the user to run File > Set as Default
Editor > Save Files..., which writes the pair for wherever the exe really
is. To use `bpad` from a terminal, the user adds the folder to `PATH`.

## Testing

On a Windows 10 or 11 VM, as an ordinary (not elevated) user, with
BachelorPad+ not installed:

```powershell
./packaging/windows/Test-Install.ps1 -SetupPath out/BachelorPadPlus-<version>-windows-x64-setup.exe
```

It snapshots the relevant keys with `reg query`, installs with
`/VERYSILENT /CURRENTUSER /SUPPRESSMSGBOXES` and the PATH task, checks every
expected value (read from the generated block), the files, the shortcut, the
PATH entry and `bpad --version`, uninstalls silently, and checks that every
value and owned key is gone, that every other value in the shared keys and
PATH is as it was, and that marker files in the user's directories survived.
It exits non-zero on any failure and leaves the snapshots, both Setup logs
and a summary in `-EvidenceDir` (default: a new directory in `%TEMP%`).

## What has not been run

As of 2026-09-27, nothing here has run on Windows. On Linux: the generator,
its escaping, the uninstall flags and the drift test run in
`cargo test -p bp-platform`; the five scripts parse; `New-PortableZip.ps1`
built a zip from a stand-in exe and read it back; `Build-Installer.ps1` was
run against a stand-in ISCC to check the arguments it passes and the
not-found message; and `Test-Install.ps1`'s reading of the generated blocks
was exercised. Never run: ISCC on `bachelorpad.iss` (so its `[Code]` has not
been compiled), the installer, the uninstaller, `bpad.cmd`, the exe
version-resource check, `Test-Install.ps1` end to end, and a Windows
`Invoke-Release.ps1` dry run (the recorded dry run is the Linux one).
`Signing.ps1` has never signed anything: there is no signing identity yet
(decision 3; question 3a is open), so signtool, the Artifact Signing dlib
and `signtool verify /pa` are all untested.

The owner's run-book for all of it is `docs/release/WINDOWS_BRINGUP.md`: the
first Windows gate run (M0-12), then `Invoke-Release.ps1 -DryRun` (which
compiles the installer and the zip) and `Test-Install.ps1`, with signing
last, once an identity exists.
