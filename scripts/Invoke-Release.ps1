#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Build, package, check and record a release for this machine's OS
    (release plan M3-03; decision 19, ADR-0016 amendment).

.DESCRIPTION
    One OS per run: Linux on a Linux host, Windows on a Windows host. Each
    run writes to artifacts/releases/<tag>/<os>/:

      files/          the packages, split debug info and the SBOM (ignored
                      by git; they go to the GitHub release)
      SHA256SUMS      one line per file in files/, `sha256sum -c` format
      SHA256SUMS.minisig   when -MinisignKey is given
      manifest.json   what was built from what, and by what

    SHA256SUMS, its signature and manifest.json are committed: they are the
    record of the release.

    The steps, each fatal on failure:
      1. The tag is v<workspace version>; outside -DryRun the tree is clean
         and HEAD is the tagged commit.
      2. cargo deny, the third-party notices check, and bp-platform's drift
         tests (the shipped .desktop, MIME, AppStream and .iss files are
         byte-for-byte what the code generates). The full gate too, unless
         -SkipGate.
      3. cargo auditable build --release --locked, with the builder's paths
         remapped out of the binary (see .cargo/config.toml for why that is
         here and not there). auditable embeds the dependency list, so the
         exe itself says what it was built from.
      4. Packages: on Linux, packaging/linux/build-packages.sh (inside the
         pinned container with -Container) and the glibc ceiling check; on
         Windows, the installer and the portable zip, signed with -Sign
         (packaging/windows/Signing.ps1).
      5. A CycloneDX SBOM, SHA256SUMS, the minisign signature, the manifest.
      6. With -Upload, gh release upload.

.PARAMETER Tag
    The release tag, e.g. v1.0.0-rc.0. Must equal "v" + the workspace
    version.

.PARAMETER DryRun
    Allow a dirty tree and an untagged HEAD, and record that they were.
    Everything else runs.

.PARAMETER Container
    Linux only: build inside packaging/linux/Dockerfile.release (Debian 12,
    glibc 2.36). Without it the build is native and the glibc check says
    whether this host's glibc is too new for the promise.

.PARAMETER SkipGate
    Skip the full local gate (step 2 still runs its release checks).

.PARAMETER MinisignKey
    Path to a minisign secret key; signs SHA256SUMS.

.PARAMETER IsccPath
    Windows only: Inno Setup's ISCC.exe, if not on PATH or in its usual
    place.

.PARAMETER Sign
    Windows only: sign the exe, then the setup and its uninstaller, with the
    identity configured on this machine (packaging/windows/Signing.ps1;
    Azure Artifact Signing or an OV certificate, both through signtool with
    an RFC 3161 timestamp), and verify each with `signtool verify /pa`.

.PARAMETER AllowUnsigned
    Windows only: allow a non-dry-run release without -Sign. Unsigned
    Windows binaries meet SmartScreen and Smart App Control; this exists
    for a release deliberately made before the identity is in place.

.PARAMETER Upload
    Upload files/ to the GitHub release named by -Tag with gh. Refused with
    -DryRun.

.EXAMPLE
    pwsh -NoProfile -File scripts/Invoke-Release.ps1 -Tag v1.0.0-rc.0 -DryRun -Container
#>
param(
    [Parameter(Mandatory)][string]$Tag,
    [switch]$DryRun,
    [switch]$Container,
    [switch]$SkipGate,
    [string]$MinisignKey,
    [string]$IsccPath,
    [switch]$Sign,
    [switch]$AllowUnsigned,
    [switch]$Upload
)

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$OnWindows = $IsWindows -or ($env:OS -eq 'Windows_NT')
$Os = if ($OnWindows) { 'windows' } else { 'linux' }
$started = Get-Date
$script:NativeGlibc = $null

function Step {
    param([string]$Name, [scriptblock]$Body)
    Write-Host "`n== $Name" -ForegroundColor Cyan
    & $Body
}

function Invoke-Native {
    # A native command's failure as an exception, so every step is fatal
    # without an `if ($LASTEXITCODE)` after each line.
    param([string]$What, [scriptblock]$Command)
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$What failed (exit $LASTEXITCODE)" }
}

function Get-Sha256 {
    param([string]$Path)
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Require-Tool {
    param([string]$Name, [string]$Install)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name is not installed: $Install"
    }
}

if ($Upload -and $DryRun) { throw "-Upload with -DryRun would publish a dry run; refused" }
if ($Container -and $OnWindows) { throw "-Container is for the Linux build only" }
if (($Sign -or $AllowUnsigned) -and -not $OnWindows) { throw "-Sign and -AllowUnsigned are for the Windows build only" }
if ($OnWindows -and -not $Sign -and -not $DryRun -and -not $AllowUnsigned) {
    throw "a Windows release is signed: pass -Sign (or -AllowUnsigned to release unsigned on purpose)"
}

Push-Location $Root
try {
    # --- 1. what is being released -------------------------------------
    $meta = (cargo metadata --format-version 1 --no-deps --locked) -join "`n" | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw "cargo metadata failed" }
    $version = ($meta.packages | Where-Object { $_.name -eq 'bachelorpad' }).version
    $commit = (git rev-parse HEAD).Trim()
    $dirty = [bool](git status --porcelain --untracked-files=no)

    Step "release $Tag ($Os) from $commit" {
        if ($Tag -notmatch '^v\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$') { throw "$Tag is not a vX.Y.Z[-pre] tag" }
        if ($Tag -ne "v$version") {
            throw "the tag is $Tag but the workspace version is $version; bump Cargo.toml or use v$version"
        }
        if ($dirty) {
            if (-not $DryRun) { throw "the tree has uncommitted changes; commit them, or use -DryRun" }
            Write-Host "  dry run: the tree has uncommitted changes (recorded)" -ForegroundColor Yellow
        }
        $tagged = (git rev-list -n 1 $Tag 2>$null)
        if (-not $DryRun -and $tagged -ne $commit) { throw "HEAD is not tagged $Tag" }
    }

    $out = Join-Path $Root "artifacts/releases/$Tag/$Os"
    $files = Join-Path $out 'files'
    if (Test-Path $files) { Remove-Item -Recurse -Force $files }
    New-Item -ItemType Directory -Force -Path $files | Out-Null

    # --- 2. release checks ---------------------------------------------
    Step 'licences, advisories and generated files' {
        Require-Tool cargo-deny 'cargo install --locked cargo-deny'
        Invoke-Native 'cargo deny check' { cargo deny check }
        Invoke-Native 'third-party notices check' {
            pwsh -NoProfile -File (Join-Path $PSScriptRoot 'New-ThirdPartyNotices.ps1') -Check
        }
        Invoke-Native 'bp-platform drift tests' { cargo test -p bp-platform --locked --quiet }
    }
    if ($SkipGate) {
        Write-Host "`n== gate: skipped (-SkipGate)" -ForegroundColor Yellow
    }
    else {
        Step 'local gate' {
            Invoke-Native 'the gate' { pwsh -NoProfile -File (Join-Path $PSScriptRoot 'Invoke-LocalCI.ps1') }
        }
    }

    # --- 3 and 4. build and package ------------------------------------
    if ($OnWindows) {
        Step 'build (cargo auditable, paths remapped)' {
            Require-Tool cargo-auditable 'cargo install --locked cargo-auditable'
            $cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $HOME '.cargo' }
            # Joined with the +crt-static in .cargo/config.toml, not
            # replacing it: see that file.
            $remap = "target.'cfg(all())'.rustflags = ['--remap-path-prefix=$($cargoHome -replace '\\','/')=/cargo', '--remap-path-prefix=$($Root -replace '\\','/')=/bachelorpad']"
            Invoke-Native 'cargo auditable build' {
                cargo auditable build --release --locked -p bachelorpad --config $remap
            }
        }
        $targetDir = ((cargo metadata --format-version 1 --no-deps) -join "`n" | ConvertFrom-Json).target_directory
        $exe = Join-Path $targetDir 'release/bachelorpad.exe'
        if ($Sign) {
            Step 'sign the exe (Authenticode, timestamped)' {
                . (Join-Path $Root 'packaging/windows/Signing.ps1')
                Invoke-FileSigning -Path $exe
            }
        }
        Step "installer and portable zip ($(if ($Sign) { 'signed' } else { 'unsigned' }))" {
            $installerArgs = @{ ExePath = $exe; OutputDir = $files; Version = $version }
            if ($IsccPath) { $installerArgs.IsccPath = $IsccPath }
            if ($Sign) { $installerArgs.Sign = $true }
            & (Join-Path $Root 'packaging/windows/Build-Installer.ps1') @installerArgs | Out-Null
            & (Join-Path $Root 'packaging/windows/New-PortableZip.ps1') -ExePath $exe -OutputDir $files -Version $version | Out-Null
            $pdb = Join-Path $targetDir 'release/bachelorpad.pdb'
            if (Test-Path $pdb) {
                Copy-Item $pdb (Join-Path $files "bachelorpad-$version-windows-x64.pdb")
            }
        }
    }
    else {
        if ($Container) {
            Step 'Linux packages (pinned Debian 12 container)' {
                # Builds with cargo auditable and the path remap inside, checks
                # the glibc ceiling before packaging, and fails on a missing
                # licence or notices file (packaging/linux/README.md).
                Invoke-Native 'build-in-container.sh' {
                    bash (Join-Path $Root 'packaging/linux/build-in-container.sh') --out $files -- --require-notices
                }
            }
        }
        else {
            Step 'Linux packages (native build on this host)' {
                Require-Tool cargo-deb 'cargo install --locked cargo-deb'
                Require-Tool rpmbuild 'apt install rpm'
                $cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $HOME '.cargo' }
                $remap = "target.'cfg(all())'.rustflags = ['--remap-path-prefix=$cargoHome=/cargo', '--remap-path-prefix=$Root=/bachelorpad']"
                Invoke-Native 'build-packages.sh' {
                    bash (Join-Path $Root 'packaging/linux/build-packages.sh') --out $files --require-notices -- --config $remap
                }
                # The container checks this before packaging; a native build
                # can only report it. This host's glibc is whatever it is,
                # which is exactly why a release uses -Container.
                $targetDir = ((cargo metadata --format-version 1 --no-deps) -join "`n" | ConvertFrom-Json).target_directory
                $glibcReport = @(bash (Join-Path $Root 'packaging/linux/check-glibc.sh') (Join-Path $targetDir 'release/bachelorpad'))
                $glibcStatus = $LASTEXITCODE
                $glibcReport | ForEach-Object { Write-Host "  $_" }
                # The first line is the figure; recorded in the manifest as
                # the container records it, so a native dry run shows how far
                # this host is from the promise.
                $script:NativeGlibc = $glibcReport | Select-Object -First 1
                if ($glibcStatus -ne 0) {
                    if (-not $DryRun) { throw "the binary needs a newer glibc than the 2.36 promise; build with -Container" }
                    Write-Host "  dry run: glibc above the 2.36 ceiling on this host (recorded); a release uses -Container" -ForegroundColor Yellow
                }
            }
        }
    }

    # --- 5. SBOM, sums, signature, manifest ----------------------------
    Step 'SBOM (CycloneDX)' {
        Require-Tool cargo-cyclonedx 'cargo install --locked cargo-cyclonedx'
        $triple = if ($OnWindows) { 'x86_64-pc-windows-msvc' } else { 'x86_64-unknown-linux-gnu' }
        Push-Location (Join-Path $Root 'apps/bachelorpad')
        try {
            Invoke-Native 'cargo cyclonedx' {
                cargo cyclonedx --format json --target $triple --override-filename "bachelorpad-$version-$Os.cdx"
            }
        }
        finally {
            Pop-Location
        }
        # cargo-cyclonedx has no package filter in a workspace: it writes one
        # SBOM into every member's directory. The application's is the one
        # that describes what ships (its dependency graph includes every
        # crate it links); the rest are removed rather than left in the tree.
        $sbomName = "bachelorpad-$version-$Os.cdx.json"
        Move-Item -Force (Join-Path $Root "apps/bachelorpad/$sbomName") $files
        Get-ChildItem -Path $Root -Recurse -File -Filter $sbomName |
            Where-Object { $_.FullName -notlike "$files*" } |
            Remove-Item -Force
    }

    $artefacts = @(Get-ChildItem -LiteralPath $files -File | Sort-Object Name)
    if ($artefacts.Count -eq 0) { throw "nothing was produced in $files" }

    Step 'SHA256SUMS' {
        $sums = ($artefacts | ForEach-Object { "$(Get-Sha256 $_.FullName)  $($_.Name)" }) -join "`n"
        [System.IO.File]::WriteAllText((Join-Path $out 'SHA256SUMS'), "$sums`n")
        Get-Content (Join-Path $out 'SHA256SUMS') | ForEach-Object { Write-Host "  $_" }
    }

    $signed = $false
    if ($MinisignKey) {
        Step 'minisign' {
            Require-Tool minisign 'apt install minisign, or https://jedisct1.github.io/minisign/'
            Invoke-Native 'minisign' {
                minisign -S -s $MinisignKey -m (Join-Path $out 'SHA256SUMS') -t "BachelorPad+ $Tag $Os"
            }
        }
        $signed = $true
    }
    else {
        Write-Host "`n== minisign: no -MinisignKey, SHA256SUMS is unsigned" -ForegroundColor Yellow
    }

    Step 'manifest.json' {
        $manifest = [ordered]@{
            tag         = $Tag
            version     = $version
            os          = $Os
            commit      = $commit
            clean_tree  = -not $dirty
            dry_run     = [bool]$DryRun
            built_at    = $started.ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
            host        = [ordered]@{
                os      = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
                machine = [System.Environment]::MachineName
            }
            container   = [bool]$Container
            glibc_required = $(
                $glibc = Join-Path $files 'glibc-required.txt'
                if (Test-Path $glibc) { (Get-Content -Raw $glibc).Trim() } else { $script:NativeGlibc }
            )
            rustc       = ((rustc -vV) -join "`n")
            cargo       = ((cargo -V) -join '')
            cargo_lock_sha256 = Get-Sha256 (Join-Path $Root 'Cargo.lock')
            sums_signed = $signed
            windows_signed = [bool]($OnWindows -and $Sign)
            artefacts   = @($artefacts | ForEach-Object {
                    [ordered]@{ name = $_.Name; bytes = $_.Length; sha256 = Get-Sha256 $_.FullName }
                })
        }
        $json = $manifest | ConvertTo-Json -Depth 5
        [System.IO.File]::WriteAllText((Join-Path $out 'manifest.json'), "$json`n")
        Write-Host "  $(Join-Path $out 'manifest.json')"
    }

    # --- 6. upload -----------------------------------------------------
    if ($Upload) {
        Step "upload to the GitHub release $Tag" {
            Require-Tool gh 'https://cli.github.com/'
            $upload = @($artefacts.FullName) + @(Join-Path $out 'SHA256SUMS')
            if ($signed) { $upload += Join-Path $out 'SHA256SUMS.minisig' }
            Invoke-Native 'gh release upload' { gh release upload $Tag @upload --clobber }
        }
    }

    Write-Host "`nRelease $Tag ($Os) written to $out" -ForegroundColor Green
}
finally {
    Pop-Location
}
