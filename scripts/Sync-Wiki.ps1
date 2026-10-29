<#
.SYNOPSIS
    Regenerates the GitHub Wiki from the repository's canonical documents.

.DESCRIPTION
    The wiki owns nothing. Every page is either a mirror of one canonical file
    (README.md, ROADMAP.md, docs/...) or an index derived from the source tree
    (the crate table from each `//!` header, the ADR index from docs/decisions).
    Edit the source, never the wiki page; the next run overwrites it.

    Modes:
      (default)  write the pages into -WikiPath and refresh metadata/wiki_manifest.json
      -Check     write nothing; exit 1 if the wiki clone has drifted from the sources
      -Publish   generate, validate, then commit and push the wiki clone

    Deterministic by design: no timestamps, no model calls. Running it twice on
    unchanged sources produces no diff (docs/governance/WIKI_SYNC.md).
#>
[CmdletBinding()]
param(
    [string]$WikiPath = (Join-Path (Split-Path $PSScriptRoot -Parent | Split-Path -Parent) 'bachelorpadplus_rust.wiki'),
    # Branch the blob links point at. Change to main once D13 merges the work there.
    [string]$Ref,
    [switch]$Check,
    [switch]$Publish,
    # Treat a link from a mirrored page to a file that does not exist as a failure.
    [switch]$Strict
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path $PSScriptRoot -Parent
$slug = 'dboles99/bachelorpadplus_rust'
if (-not $Ref) { $Ref = (git -C $repoRoot rev-parse --abbrev-ref HEAD).Trim() }
$blob = "https://github.com/$slug/blob/$Ref"
$tree = "https://github.com/$slug/tree/$Ref"

# Mirrors: wiki page -> canonical source. Order is the order in the sidebar.
$mirrors = @(
    @{ Group = 'Start here';  Page = 'Overview';        Src = 'README.md' },
    @{ Group = 'Start here';  Page = 'Product-Spec';    Src = 'specs.md' },
    @{ Group = 'Start here';  Page = 'Latest-Release';  Src = 'RELEASE_SUMMARY_v1.1.1.md' },
    @{ Group = 'Start here';  Page = 'Roadmap';         Src = 'ROADMAP.md' },
    @{ Group = 'Design';      Page = 'Architecture';    Src = 'docs/architecture/ARCHITECTURE.md' },
    @{ Group = 'Design';      Page = 'Security-Model';  Src = 'docs/architecture/SECURITY_MODEL.md' },
    @{ Group = 'Design';      Page = 'Platform-Matrix'; Src = 'docs/architecture/PLATFORM_MATRIX.md' },
    @{ Group = 'Design';      Page = 'Data-Model';      Src = 'docs/architecture/DATA_MODEL.md' },
    @{ Group = 'Design';      Page = 'Menu-Map';        Src = 'docs/product/MENU_MAP.md' },
    @{ Group = 'Memory';      Page = 'Decision-Register-and-Lessons'; Src = 'DECISIONS.md' },
    @{ Group = 'Memory';      Page = 'Project-Memory';  Src = 'PROJECT_MEMORY.md' },
    @{ Group = 'Working on it'; Page = 'Local-CI';      Src = 'docs/governance/LOCAL_CI.md' },
    @{ Group = 'Working on it'; Page = 'Work-Model';    Src = 'docs/governance/WORK_MODEL.md' },
    @{ Group = 'Working on it'; Page = 'Definition-of-Done'; Src = 'docs/governance/DEFINITION_OF_DONE.md' },
    # CLAUDE.md, not AGENTS.md: AGENTS.md is a gitignored local copy, so a blob
    # link to it 404s on GitHub and the mirror would depend on an untracked file.
    @{ Group = 'Working on it'; Page = 'Agents';        Src = 'CLAUDE.md' },
    @{ Group = 'Working on it'; Page = 'Wiki-Sync';     Src = 'docs/governance/WIKI_SYNC.md' }
)
# Pages derived from the tree rather than mirrored from one file.
$derived = @(
    @{ Group = 'Design';  Page = 'Crates' },
    @{ Group = 'Memory';  Page = 'Decision-Records' }
)

$sourceToPage = @{}
foreach ($m in $mirrors) { $sourceToPage[$m.Src] = $m.Page }
$knownPages = @('Home') + ($mirrors | ForEach-Object Page) + ($derived | ForEach-Object Page)

function Get-Sha([string]$text) {
    $bytes = [Text.Encoding]::UTF8.GetBytes($text)
    -join ([Security.Cryptography.SHA256]::HashData($bytes) | ForEach-Object { $_.ToString('x2') })
}
function Read-Lf([string]$path) { ((Get-Content -LiteralPath $path -Raw -Encoding utf8) -replace "`r`n", "`n") }
function Rel([string]$full) { ([IO.Path]::GetRelativePath($repoRoot, $full)) -replace '\\', '/' }

$brokenLinks = [System.Collections.Generic.List[string]]::new()

# A relative link in a repo file means nothing in the wiki: point it at the
# sibling wiki page if there is one, otherwise at the file on GitHub.
function Convert-Link([string]$target, [string]$srcRel) {
    if ($target -match '^(https?:|mailto:|#|/)') { return $target }
    $path = $target; $anchor = ''
    if ($target -match '^([^#]*)(#.*)$') { $path = $Matches[1]; $anchor = $Matches[2] }
    if (-not $path) { return $target }
    $path = [Uri]::UnescapeDataString($path)
    $full = [IO.Path]::GetFullPath((Join-Path (Join-Path $repoRoot (Split-Path $srcRel -Parent)) $path))
    $rel = Rel $full
    if ($rel.StartsWith('..')) { return $target }
    if ($sourceToPage.ContainsKey($rel)) { return "$($sourceToPage[$rel])$anchor" }
    if (Test-Path -LiteralPath $full -PathType Container) { return "$tree/$rel$anchor" }
    if (-not (Test-Path -LiteralPath $full)) { $brokenLinks.Add("$srcRel -> $target") }
    return "$blob/$rel$anchor"
}

function Convert-Body([string]$text, [string]$srcRel) {
    $inFence = $false
    $out = foreach ($line in ($text -split "`n")) {
        if ($line -match '^\s*(```|~~~)') { $inFence = -not $inFence; $line; continue }
        if ($inFence) { $line; continue }
        [regex]::Replace($line, '\]\(([^)\s]+)((?:\s+"[^"]*")?)\)', {
            param($m) "](" + (Convert-Link $m.Groups[1].Value $srcRel) + $m.Groups[2].Value + ")"
        })
    }
    $out -join "`n"
}

$pages = [ordered]@{}   # page name -> content
$manifest = [System.Collections.Generic.List[object]]::new()

foreach ($m in $mirrors) {
    $srcPath = Join-Path $repoRoot $m.Src
    if (-not (Test-Path -LiteralPath $srcPath)) {
        throw "Mirror source missing: $($m.Src) (page $($m.Page))"
    }
    $raw = Read-Lf $srcPath
    $sha = Get-Sha $raw
    $banner = "> Mirrored from [``$($m.Src)``]($blob/$($m.Src)) (content sha256 ``$($sha.Substring(0,12))``) by ``scripts/Sync-Wiki.ps1``. **Edit the source, not this page** -- the next sync overwrites it.`n`n"
    $pages[$m.Page] = $banner + (Convert-Body $raw $m.Src).TrimEnd() + "`n"
    $manifest.Add([ordered]@{ page = $m.Page; kind = 'mirror'; source = $m.Src; source_sha256 = $sha })
}

# --- Crates: one row per crate, the first sentence of its own `//!` header.
$crateRows = foreach ($dir in Get-ChildItem (Join-Path $repoRoot 'crates') -Directory | Sort-Object Name) {
    $entry = @('src/lib.rs', 'src/main.rs') | ForEach-Object { Join-Path $dir.FullName $_ } | Where-Object { Test-Path $_ } | Select-Object -First 1
    $summary = '*(no crate-level `//!` documentation)*'
    if ($entry) {
        $doc = Get-Content -LiteralPath $entry -Encoding utf8 | Where-Object { $_ -match '^//!' } |
            Select-Object -First 8 | ForEach-Object { ($_ -replace '^//! ?', '').Trim() }
        $para = @(); foreach ($l in $doc) { if ($l -eq '' -and $para.Count) { break }; if ($l) { $para += $l } }
        if ($para.Count) { $summary = (($para -join ' ') -replace '\|', '\|') }
    }
    "| [``$($dir.Name)``]($tree/crates/$($dir.Name)) | $summary |"
}
$cratesBody = @"
# Crates

> Derived from each crate's own ``//!`` header by ``scripts/Sync-Wiki.ps1``. The header is the source; fix it there. For how the crates fit together, see [Architecture](Architecture).

| Crate | What it owns |
| --- | --- |
$($crateRows -join "`n")
"@
$pages['Crates'] = $cratesBody.TrimEnd() + "`n"
$manifest.Add([ordered]@{ page = 'Crates'; kind = 'derived'; source = 'crates/*/src/{lib,main}.rs'; source_sha256 = Get-Sha $cratesBody })

# --- ADR index: number, title and status straight from each record.
$adrRows = foreach ($f in Get-ChildItem (Join-Path $repoRoot 'docs/decisions') -Filter 'ADR-*.md' | Sort-Object Name) {
    $lines = Get-Content -LiteralPath $f.FullName -Encoding utf8
    $title = (($lines | Select-Object -First 1) -replace '^#\s*', '') -replace '^ADR-\d+:\s*', ''
    # Two house formats: an inline `**Status:** ...` line, or a `## Status` heading.
    $status = ($lines | Where-Object { $_ -match '^\*\*Status:?\*\*:?' } | Select-Object -First 1)
    if ($status) { $status = $status -replace '^\*\*Status:?\*\*:?\s*', '' }
    else {
        $h = [array]::IndexOf($lines, ($lines | Where-Object { $_ -match '^##\s+Status\s*$' } | Select-Object -First 1))
        if ($h -ge 0) { $status = $lines[($h + 1)..($lines.Count - 1)] | Where-Object { $_.Trim() } | Select-Object -First 1 }
    }
    # Only the leading keyword is indexed; the qualification belongs to the record.
    $status = ($status -replace '\*', '').Trim()
    $status = if (-not $status) { '*(none recorded)*' }
              elseif ($status -match '^(\w+)\s*[.]?\s*$') { $Matches[1] }
              elseif ($status -match '^(\w+)[\s—–-]*(implemented\s+)?\d{4}-\d{2}-\d{2}\.?$') { $Matches[1] }
              elseif ($status -match '^(\w+)') { "$($Matches[1]) *(qualified -- see record)*" }
              else { $status }
    $id = $f.BaseName
    "| [$id]($blob/docs/decisions/$($f.Name)) | $(($title -replace '\|','\|')) | $(($status -replace '\|','\|')) |"
}
$adrBody = @"
# Decision records

> Derived from ``docs/decisions/ADR-*.md`` by ``scripts/Sync-Wiki.ps1``. Each row links to the record, which holds the reasoning. Lessons learned across them live in [Decision-Register-and-Lessons](Decision-Register-and-Lessons).

$(@($adrRows).Count) records.

| Record | Decision | Status |
| --- | --- | --- |
$($adrRows -join "`n")
"@
$pages['Decision-Records'] = $adrBody.TrimEnd() + "`n"
$manifest.Add([ordered]@{ page = 'Decision-Records'; kind = 'derived'; source = 'docs/decisions/ADR-*.md'; source_sha256 = Get-Sha $adrBody })

# --- Home, sidebar, footer.
$tag = (git -C $repoRoot describe --tags --abbrev=0 2>$null)
$nav = foreach ($g in (($mirrors + $derived) | ForEach-Object Group | Select-Object -Unique)) {
    "### $g`n" + ((($mirrors + $derived) | Where-Object Group -eq $g | ForEach-Object { "- [$($_.Page -replace '-', ' ')]($($_.Page))" }) -join "`n")
}
$pages['Home'] = @"
# BachelorPad+

*Notepad when you want it. More when you need it.*

A Rust/Slint text editor for Windows 10, Windows 11 and Linux. Latest release: **$tag**. New here? Start with [Overview](Overview).

This wiki is a generated reading surface. The repository is the source of truth; see [Wiki Sync](Wiki-Sync) for how the two are kept in step.

$($nav -join "`n`n")
"@.TrimEnd() + "`n"
$pages['_Sidebar'] = ("**[BachelorPad+](Home)**`n`n" + ($nav -join "`n`n")).TrimEnd() + "`n"
$pages['_Footer'] = "Generated from [$slug@$Ref]($tree) by ``scripts/Sync-Wiki.ps1``. Licence: GPL-3.0-only.`n"

# --- Validate before anything is written or published.
$problems = [System.Collections.Generic.List[string]]::new()
foreach ($name in $pages.Keys) {
    foreach ($mt in [regex]::Matches($pages[$name], '\]\(([A-Za-z0-9_-]+)(#[^)]*)?\)')) {
        if ($mt.Groups[1].Value -notin $knownPages) { $problems.Add("$name links to unknown wiki page '$($mt.Groups[1].Value)'") }
    }
    if ($pages[$name] -match '(?i)(sk-[A-Za-z0-9]{20,}|ghp_[A-Za-z0-9]{30,}|AKIA[0-9A-Z]{16}|BEGIN [A-Z ]*PRIVATE KEY)') {
        $problems.Add("$name matches a secret pattern -- refusing to publish")
    }
}
if ($Strict) { foreach ($b in $brokenLinks) { $problems.Add("broken source link: $b") } }
if ($problems.Count) { $problems | ForEach-Object { Write-Error $_ -ErrorAction Continue }; exit 1 }
if ($brokenLinks.Count) {
    Write-Warning "$($brokenLinks.Count) link(s) in source documents point at files that do not exist (pass -Strict to fail):"
    $brokenLinks | Sort-Object -Unique | ForEach-Object { Write-Warning "  $_" }
}

foreach ($e in $manifest) { $e['page_sha256'] = Get-Sha $pages[$e.page] }
$manifestJson = ([ordered]@{
    generator = 'scripts/Sync-Wiki.ps1'
    ref       = $Ref
    pages     = $manifest
} | ConvertTo-Json -Depth 5) -replace "`r`n", "`n"
$manifestPath = Join-Path $repoRoot 'metadata/wiki_manifest.json'

if (-not (Test-Path $WikiPath)) { throw "Wiki clone not found at $WikiPath. Clone https://github.com/$slug.wiki.git there first." }

# Pages not in this run's set (and not .git) are stale: a mirror was removed from the map.
$expected = $pages.Keys | ForEach-Object { "$_.md" }
$stale = Get-ChildItem $WikiPath -File | Where-Object { $_.Extension -eq '.md' -and $_.Name -notin $expected }

$drift = [System.Collections.Generic.List[string]]::new()
foreach ($name in $pages.Keys) {
    $p = Join-Path $WikiPath "$name.md"
    if (-not (Test-Path $p) -or (Read-Lf $p) -ne $pages[$name]) { $drift.Add($name) }
}
foreach ($s in $stale) { $drift.Add("$($s.Name) (stale)") }
$manifestDrift = -not (Test-Path $manifestPath) -or ((Read-Lf $manifestPath).TrimEnd() -ne $manifestJson.TrimEnd())

if ($Check) {
    if ($drift.Count -or $manifestDrift) {
        Write-Host "DRIFT: $($drift.Count) wiki page(s)$(if ($manifestDrift) { ' + manifest' }) differ from the sources:"
        $drift | ForEach-Object { Write-Host "  $_" }
        exit 1
    }
    Write-Host "OK: $($pages.Count) pages match their sources (ref $Ref)."
    exit 0
}

$utf8 = [Text.UTF8Encoding]::new($false)
foreach ($name in $pages.Keys) { [IO.File]::WriteAllText((Join-Path $WikiPath "$name.md"), $pages[$name], $utf8) }
foreach ($s in $stale) { Remove-Item -LiteralPath $s.FullName }
[IO.File]::WriteAllText($manifestPath, $manifestJson.TrimEnd() + "`n", $utf8)
Write-Host "Wrote $($pages.Count) pages to $WikiPath ($($drift.Count) changed) and metadata/wiki_manifest.json."

if ($Publish) {
    git -C $WikiPath config core.autocrlf false
    git -C $WikiPath add -A
    if (-not (git -C $WikiPath status --porcelain)) { Write-Host 'Wiki already up to date; nothing to publish.'; exit 0 }
    $head = (git -C $repoRoot rev-parse --short HEAD).Trim()
    git -C $WikiPath commit -q -m "Sync wiki from $slug@$head ($Ref)"
    git -C $WikiPath push origin HEAD
    if ($LASTEXITCODE) { throw 'Wiki push failed.' }
    Write-Host "Published: $(git -C $WikiPath rev-parse --short HEAD)"
}
