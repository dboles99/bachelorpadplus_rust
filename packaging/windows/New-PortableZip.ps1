<#
.SYNOPSIS
    Build the BachelorPad+ portable zip from an already-built release exe.

.DESCRIPTION
    Release plan M3-01 (decision 18) and M2-16. The zip holds one folder,
    named like the zip, containing:

      bachelorpad.exe   the exe passed in
      bpad.cmd          the short command, with CRLF line endings
      LICENSE-MIT       both licence files from the repository root
      LICENSE-APACHE
      THIRD-PARTY-NOTICES.txt  every bundled component's licence
      README.txt        packaging/windows/README-portable.txt, CRLF

    No .reg files. A registration names the exe's absolute path, which is
    unknown until the zip is unpacked; a pair generated here would register
    a path that does not exist. README.txt sends the user to File > Set as
    Default Editor > Save Files..., which writes the pair for wherever the
    exe actually is.

    The version comes from Cargo.toml through `cargo metadata`. Writes the
    zip's path to the pipeline, and nothing else.

    Runs anywhere PowerShell does; the exe is only copied.

.PARAMETER ExePath
    The release bachelorpad.exe to package.

.PARAMETER OutputDir
    Where the zip is written. Created if missing; an existing zip of the
    same name is replaced.

.PARAMETER Version
    Overrides the version read from Cargo.toml. It must still match the
    exe's version resource where that can be read.

.PARAMETER OutputBaseName
    The zip's name without .zip, and its folder's name. Default:
    BachelorPadPlus-<version>-windows-x64-portable

.EXAMPLE
    ./packaging/windows/New-PortableZip.ps1 -ExePath target/release/bachelorpad.exe -OutputDir artifacts/releases/v1.0.0
#>
#Requires -Version 5.1
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$ExePath,
    [Parameter(Mandatory)][string]$OutputDir,
    [string]$Version,
    [string]$OutputBaseName
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'PackagingCommon.ps1')
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

$repoRoot = Get-RepoRoot
$exe = (Resolve-Path -LiteralPath $ExePath).Path
if (-not $Version) { $Version = Get-WorkspaceVersion -RepoRoot $repoRoot }
Assert-ExeVersion -ExePath $exe -Version $Version
if (-not $OutputBaseName) { $OutputBaseName = "BachelorPadPlus-$Version-windows-x64-portable" }
$licences = Get-LicenceFiles -RepoRoot $repoRoot

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$zip = Join-Path (Resolve-Path -LiteralPath $OutputDir).Path "$OutputBaseName.zip"
if (Test-Path -LiteralPath $zip) { Remove-Item -LiteralPath $zip -Force }

$staging = New-StagingDirectory
try {
    Copy-Item -LiteralPath $exe -Destination (Join-Path $staging 'bachelorpad.exe')
    Copy-BatchFile -Source (Join-Path $PSScriptRoot 'bpad.cmd') -Destination (Join-Path $staging 'bpad.cmd')
    foreach ($licence in $licences) {
        Copy-Item -LiteralPath $licence -Destination (Join-Path $staging (Split-Path -Leaf $licence))
    }
    $readme = [System.IO.File]::ReadAllText((Join-Path $PSScriptRoot 'README-portable.txt'))
    Write-WindowsText -Path (Join-Path $staging 'README.txt') -Text $readme

    # Entry by entry rather than CreateFromDirectory, so every name uses
    # forward slashes (Windows PowerShell 5.1's archive code has written
    # backslashes, which other unzip tools read as part of the file name)
    # and the order is fixed.
    $archive = [System.IO.Compression.ZipFile]::Open($zip, [System.IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($file in Get-ChildItem -LiteralPath $staging -File | Sort-Object Name) {
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
                $archive, $file.FullName, "$OutputBaseName/$($file.Name)",
                [System.IO.Compression.CompressionLevel]::Optimal) | Out-Null
        }
    }
    finally {
        $archive.Dispose()
    }
}
finally {
    Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
}

# Read back what was written: exactly the expected files, and no .reg.
$expected = @('LICENSE-APACHE', 'LICENSE-MIT', 'README.txt', 'THIRD-PARTY-NOTICES.txt', 'bachelorpad.exe', 'bpad.cmd') |
    ForEach-Object { "$OutputBaseName/$_" } | Sort-Object
$reader = [System.IO.Compression.ZipFile]::OpenRead($zip)
try {
    $actual = @($reader.Entries | ForEach-Object { $_.FullName } | Sort-Object)
}
finally {
    $reader.Dispose()
}
if (Compare-Object -ReferenceObject $expected -DifferenceObject $actual -CaseSensitive) {
    throw "$zip holds [$($actual -join ', ')], expected [$($expected -join ', ')]"
}
Get-Item -LiteralPath $zip
