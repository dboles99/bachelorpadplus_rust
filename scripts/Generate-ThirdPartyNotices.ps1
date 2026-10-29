<#
.SYNOPSIS
    Generate THIRD-PARTY-NOTICES.md from cargo dependencies.

.DESCRIPTION
    Uses cargo-license to extract license information from all dependencies
    and generates a markdown file listing all third-party crates by license.
    
    This script is run as part of the CI gate to ensure the notices file
    stays in sync with the actual dependencies.

.NOTES
    Requires: cargo-license (installed via `cargo install cargo-license`)
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$OutputFile = "THIRD-PARTY-NOTICES.md"

# Run cargo-license to get JSON output
Write-Host "Generating dependency license data..."
$result = cargo license --json --all-features 2>$null
if ($LASTEXITCODE -ne 0) {
    throw "cargo license failed with exit code $LASTEXITCODE"
}

# Parse JSON and generate markdown
$json = $result | ConvertFrom-Json

$grouped = @{}
foreach ($crate in $json) {
    $license = $crate.license
    if (-not $license) { $license = "Unknown" }
    if (-not $grouped.ContainsKey($license)) {
        $grouped[$license] = @()
    }
    if (-not $grouped[$license] -contains $crate.name) {
        $grouped[$license] += $crate.name
    }
}

# Write markdown
$lines = @(
    "# Third-Party Notices",
    "",
    "This file contains license notices for third-party dependencies used by BachelorPad+.",
    "",
    "Generated automatically. Do not edit manually.",
    "",
    "---",
    ""
)

foreach ($license in ($grouped.Keys | Sort-Object)) {
    $lines += "## $license"
    $lines += ""
    foreach ($crate in ($grouped[$license] | Sort-Object)) {
        $lines += "- $crate"
    }
    $lines += ""
}

$lines | Out-File -FilePath $OutputFile -Encoding UTF8

Write-Host "Generated $OutputFile with $($json.Count) crates across $($grouped.Count) licenses"
