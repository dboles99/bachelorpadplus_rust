#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Write THIRD-PARTY-NOTICES.txt: every component that ships, with its
    licence text (release plan M1-14).

.DESCRIPTION
    Three parts, in order:
      1. cargo-about's rendering of about.hbs for the crates the product
         links (about.toml: no dev or build dependencies; the workspace's own
         crates are covered by LICENSE-MIT and LICENSE-APACHE);
      2. Slint, used under its Royalty-free Licence 2.0 (decision 2,
         ADR-0031). Its text is read from the `slint` crate cargo resolves,
         so it is the licence of the version that is built, not a copy that
         can go stale; about.toml says why cargo-about cannot do this part;
      3. DejaVu Sans Mono, bundled in the binary and not a crate.

    The file is committed so every package can ship it without running
    cargo-about. -Check regenerates it in memory and fails if the committed
    copy differs, which is how the release script catches a dependency
    change nobody regenerated for.

.PARAMETER Check
    Compare instead of writing. Exit 1 on any difference.
#>
param([switch]$Check)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$target = Join-Path $root 'THIRD-PARTY-NOTICES.txt'

Push-Location $root
try {
    if (-not (Get-Command cargo-about -ErrorAction SilentlyContinue)) {
        throw "cargo-about is not installed: cargo install --locked cargo-about --features cli"
    }
    # Through a file rather than captured stdout: captured, cargo-about
    # exits 1 with no output under pwsh, and a file is what it is built to
    # write. Its warnings about Slint are expected (about.toml says why).
    $rendered = [System.IO.Path]::GetTempFileName()
    $log = [System.IO.Path]::GetTempFileName()
    try {
        cargo about generate about.hbs -o $rendered 2>$log
        if ($LASTEXITCODE -ne 0) { throw "cargo about generate failed" }
        $generated = [System.IO.File]::ReadAllText($rendered)
        # The Slint crates cargo-about could not place are exactly the ones
        # that ship: it walked the same filtered graph (targets, no dev or
        # build dependencies). cargo metadata lists every Slint crate in the
        # lockfile, test backends included, which would overstate it.
        $slintUsers = @(Select-String -Path $log -Pattern "LicenseRef-Slint-Royalty-free-2\.0 has no license file for crate '([^']+)'" |
                ForEach-Object { "  $($_.Matches[0].Groups[1].Value)" } | Sort-Object -Unique) -join "`n"
    }
    finally {
        Remove-Item -Force $rendered, $log -ErrorAction SilentlyContinue
    }
    if (-not $slintUsers) { throw "cargo-about reported no Slint crates; about.toml or cargo-about changed" }

    # The licence text from the `slint` crate that is built, so it is the
    # text of the version that ships.
    $meta = (cargo metadata --format-version 1 --locked) -join "`n" | ConvertFrom-Json
    $slint = $meta.packages | Where-Object { $_.name -eq 'slint' } | Select-Object -First 1
    if (-not $slint) { throw "no slint package in cargo metadata" }
    $licence = Join-Path (Split-Path -Parent $slint.manifest_path) 'LICENSES/LicenseRef-Slint-Royalty-free-2.0.md'
    $slintText = (Get-Content -Raw -Path $licence).TrimEnd()

    $fontText = (Get-Content -Raw -Path (Join-Path $root 'crates/bp-ui/fonts/LICENSE-DejaVu.txt')).TrimEnd()
    $rule = '=' * 80
    $thin = '-' * 80

    $text = @(
        $generated.TrimEnd()
        ''
        $rule
        'Slint Royalty-free Desktop, Mobile, and Web Applications Licence 2.0'
        'Used by:'
        $slintUsers
        $thin
        $slintText
        ''
        $rule
        'DejaVu Sans Mono (the bundled editor font)'
        $thin
        $fontText
        ''
    ) -join "`n"

    # Some upstream licence files have CRLF endings. One line-ending
    # convention for the whole file, so the committed copy and -Check agree
    # whatever git's autocrlf does on the machine that runs it.
    $text = $text -replace "`r`n", "`n"

    if ($Check) {
        $committed = if (Test-Path $target) { (Get-Content -Raw -Path $target) -replace "`r`n", "`n" } else { '' }
        if ($committed -ne $text) {
            Write-Host "THIRD-PARTY-NOTICES.txt is out of date: run scripts/New-ThirdPartyNotices.ps1 and commit it." -ForegroundColor Red
            exit 1
        }
        Write-Host 'THIRD-PARTY-NOTICES.txt is current.'
    }
    else {
        [System.IO.File]::WriteAllText($target, $text)
        Write-Host "wrote $target"
    }
}
finally {
    Pop-Location
}
