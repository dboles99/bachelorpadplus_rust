# Shared by Build-Installer.ps1 and New-PortableZip.ps1; dot-source it.
#
# Kept to what both need and Windows PowerShell 5.1 can run, so a VM without
# PowerShell 7 can still build and check a package.

Set-StrictMode -Version Latest

# The repository root: this file lives in packaging/windows.
function Get-RepoRoot {
    (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
}

# The version the binary was built with, from Cargo.toml by way of cargo
# itself, so no script carries a second copy that could disagree.
function Get-WorkspaceVersion {
    param([Parameter(Mandatory)][string]$RepoRoot)

    $manifest = Join-Path $RepoRoot 'Cargo.toml'
    $json = & cargo metadata --format-version 1 --no-deps --locked --manifest-path $manifest
    if ($LASTEXITCODE -ne 0) {
        throw "cargo metadata failed (exit code $LASTEXITCODE) for $manifest"
    }
    $metadata = ($json -join "`n") | ConvertFrom-Json
    $package = @($metadata.packages | Where-Object { $_.name -eq 'bachelorpad' })
    if ($package.Count -ne 1) {
        throw "cargo metadata lists $($package.Count) packages named bachelorpad; expected exactly one"
    }
    $package[0].version
}

# major.minor.patch.0 for VERSIONINFO, which takes four numbers and no
# pre-release label: 1.0.0-rc.1 becomes 1.0.0.0.
function ConvertTo-NumericVersion {
    param([Parameter(Mandatory)][string]$Version)

    if ($Version -notmatch '^(\d+)\.(\d+)\.(\d+)(?:[-+][0-9A-Za-z.+-]*)?$') {
        throw "not a semantic version: $Version"
    }
    $parts = @($Matches[1], $Matches[2], $Matches[3]) | ForEach-Object { [int]$_ }
    foreach ($part in $parts) {
        if ($part -gt 65535) { throw "VERSIONINFO cannot hold $Version (each part is 16 bits)" }
    }
    "$($parts[0]).$($parts[1]).$($parts[2]).0"
}

# The version resource a Windows build embeds (apps/bachelorpad/build.rs),
# checked against the version being packaged. Catches packaging an exe left
# over from an earlier build. Only Windows can read the resource; elsewhere
# the check is skipped and says so.
function Assert-ExeVersion {
    param(
        [Parameter(Mandatory)][string]$ExePath,
        [Parameter(Mandatory)][string]$Version
    )

    $productVersion = (Get-Item -LiteralPath $ExePath).VersionInfo.ProductVersion
    if ([string]::IsNullOrEmpty($productVersion)) {
        Write-Warning "$ExePath carries no readable ProductVersion; not checked against $Version"
        return
    }
    if ($productVersion -ne $Version) {
        throw "$ExePath is version $productVersion, but Cargo.toml says $Version. Rebuild it."
    }
}

# The licence files at the repository root, which every package ships: the
# product's two (MIT OR Apache-2.0) and the third-party notices
# (scripts/New-ThirdPartyNotices.ps1).
function Get-LicenceFiles {
    param([Parameter(Mandatory)][string]$RepoRoot)

    $files = foreach ($name in 'LICENSE-MIT', 'LICENSE-APACHE', 'THIRD-PARTY-NOTICES.txt') {
        $path = Join-Path $RepoRoot $name
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "$path is missing. Every package ships LICENSE-MIT, LICENSE-APACHE and THIRD-PARTY-NOTICES.txt."
        }
        $path
    }
    @($files)
}

# Text written as UTF-8 with a byte-order mark and CRLF line endings: what
# Inno Setup's licence page and Notepad on Windows 10 both read without
# guessing, and what a batch file needs.
function Write-WindowsText {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][AllowEmptyString()][string]$Text,
        [switch]$NoByteOrderMark
    )

    $crlf = ($Text -replace "`r`n", "`n") -replace "`n", "`r`n"
    $encoding = New-Object System.Text.UTF8Encoding (-not $NoByteOrderMark)
    [System.IO.File]::WriteAllText($Path, $crlf, $encoding)
}

# bpad.cmd with CRLF endings, whatever the checkout did to it. No byte-order
# mark: cmd.exe would read one as part of the first command.
function Copy-BatchFile {
    param(
        [Parameter(Mandatory)][string]$Source,
        [Parameter(Mandatory)][string]$Destination
    )

    $text = [System.IO.File]::ReadAllText($Source)
    Write-WindowsText -Path $Destination -Text $text -NoByteOrderMark
}

# A fresh, empty staging directory under the system temporary directory.
function New-StagingDirectory {
    $path = Join-Path ([System.IO.Path]::GetTempPath()) ("bachelorpad-package-" + [guid]::NewGuid())
    New-Item -ItemType Directory -Path $path | Out-Null
    $path
}
