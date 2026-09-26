<#
.SYNOPSIS
    Install BachelorPad+ silently, check what it wrote, uninstall silently,
    and check that it is all gone -- on a Windows 10 or 11 VM, run by a person.

.DESCRIPTION
    Release plan M3-01's acceptance test. Run it as an ordinary user (not
    elevated) on a machine where BachelorPad+ is not installed and has not
    been registered with a .reg file.

    1. Reads the expected registry entries from the generated [Registry]
       block of bachelorpad.iss (which a cargo test holds equal to
       bp-platform's registry_values), and the user's directories from the
       generated [Code] block.
    2. Before: saves `reg query` snapshots, fails if any expected value is
       already there, records every other value in the shared keys and the
       user PATH, and drops a marker file into each of the user's
       directories.
    3. Installs with /VERYSILENT /CURRENTUSER /SUPPRESSMSGBOXES, with the
       optional "add bpad to PATH" task selected so its removal is tested.
    4. Checks every expected value, the files, the Start menu shortcut, the
       PATH entry, and `bpad --version` through the installed bpad.cmd.
    5. Uninstalls silently from the uninstall string Setup registered.
    6. Checks every value and owned key is gone, every other value in the
       shared keys is exactly as before, PATH is as before, and the marker
       files -- the user's data -- are untouched.

    Exits 0 when every check passed and 1 otherwise. Evidence (the
    snapshots, both Setup logs and a summary) is written to -EvidenceDir.

.PARAMETER SetupPath
    The installer built by Build-Installer.ps1.

.PARAMETER ExpectedVersion
    The version `bpad --version` must print. Defaults to the DisplayVersion
    Setup registered, which checks the exe agrees with its installer.

.PARAMETER EvidenceDir
    Where snapshots and logs go. Default: a new directory under %TEMP%.

.PARAMETER IssPath
    The installer script to read the expected entries from. Default: the
    bachelorpad.iss beside this script.

.EXAMPLE
    ./packaging/windows/Test-Install.ps1 -SetupPath .\out\BachelorPadPlus-1.0.0-windows-x64-setup.exe
#>
#Requires -Version 5.1
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$SetupPath,
    [string]$ExpectedVersion,
    [string]$EvidenceDir,
    [string]$IssPath = (Join-Path $PSScriptRoot 'bachelorpad.iss')
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if ([System.Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT) {
    throw 'Test-Install.ps1 installs and uninstalls for real, so it runs on Windows only.'
}

$script:failures = New-Object System.Collections.Generic.List[string]
function Pass([string]$What) { Write-Host "  ok    $What" -ForegroundColor Green }
function Fail([string]$What) { $script:failures.Add($What); Write-Host "  FAIL  $What" -ForegroundColor Red }
function Note([string]$What) { Write-Host "  note  $What" -ForegroundColor DarkGray }
function Check([bool]$Condition, [string]$What) { if ($Condition) { Pass $What } else { Fail $What } }

# --- what the installer should write, from the generated blocks -------------

$iss = [System.IO.File]::ReadAllText((Resolve-Path -LiteralPath $IssPath).Path)

function Get-GeneratedLines([string]$Name, [string]$Comment) {
    $pattern = '(?ms)^' + [regex]::Escape("$Comment BEGIN GENERATED ${Name}:") + '[^\r\n]*\r?\n(.*?)^' +
        [regex]::Escape("$Comment END GENERATED $Name") + '\r?$'
    $match = [regex]::Match($iss, $pattern)
    if (-not $match.Success) { throw "$IssPath has no generated $Name block" }
    @($match.Groups[1].Value -split '\r?\n' | Where-Object { $_ -ne '' })
}

# Inno Setup's escaping undone: "" is ", {{ is {, and {app} (the only
# constant the generator leaves unescaped) is the install directory.
function Expand-InnoText([string]$Text, [string]$App) {
    $placeholder = [char]0xE000
    $text = $Text.Replace('""', '"').Replace('{{', [string]$placeholder)
    if ($text -match '\{(?!app\})') { throw "unexpected Inno Setup constant in: $Text" }
    $text.Replace('{app}', $App).Replace([string]$placeholder, '{')
}

$entryPattern = '^Root: HKCU; Subkey: "(?<subkey>(?:[^"]|"")*)"; ValueType: (?<type>none|string)' +
    '(?:; ValueName: "(?<name>(?:[^"]|"")*)"; ValueData: "(?<data>(?:[^"]|"")*)")?' +
    '; Flags: (?<flag>uninsdeletekey|uninsdeletevalue)$'

function Get-ExpectedEntries([string]$App) {
    foreach ($line in Get-GeneratedLines 'registry' ';') {
        $m = [regex]::Match($line, $entryPattern)
        if (-not $m.Success) { throw "cannot read generated [Registry] line: $line" }
        [pscustomobject]@{
            Subkey  = Expand-InnoText $m.Groups['subkey'].Value $App
            KeyOnly = $m.Groups['type'].Value -eq 'none'
            Name    = Expand-InnoText $m.Groups['name'].Value $App
            Data    = Expand-InnoText $m.Groups['data'].Value $App
            Owned   = $m.Groups['flag'].Value -eq 'uninsdeletekey'
        }
    }
}

function Get-UserDirectories {
    foreach ($line in Get-GeneratedLines 'user-directories' '//') {
        if ($line -match "^\s+(?<name>\w+)Directory = '(?<path>[^']*)';$" -and $Matches['name'] -ne 'ProductLocal') {
            $path = $Matches['path'].Replace('{localappdata}', $env:LOCALAPPDATA).Replace('{userappdata}', $env:APPDATA)
            [pscustomobject]@{ Kind = $Matches['name']; Path = $path }
        }
    }
}

if ($iss -notmatch '(?m)^AppId=\{\{(?<guid>[0-9A-Fa-f-]{36})\}\r?$') { throw "$IssPath has no AppId" }
$uninstallKey = "Software\Microsoft\Windows\CurrentVersion\Uninstall\{$($Matches['guid'])}_is1"
$defaultApp = Join-Path $env:LOCALAPPDATA 'Programs\BachelorPad+'

# --- reading the registry ------------------------------------------------------

function Open-UserKey([string]$Subkey) { [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($Subkey) }

function Test-UserKey([string]$Subkey) {
    $key = Open-UserKey $Subkey
    if ($null -eq $key) { return $false }
    $key.Dispose()
    $true
}

# Every value of a key as name -> data, unexpanded; empty if the key is absent.
function Get-UserValues([string]$Subkey) {
    $values = @{}
    $key = Open-UserKey $Subkey
    if ($null -eq $key) { return $values }
    try {
        foreach ($name in $key.GetValueNames()) {
            $values[$name] = [string]$key.GetValue($name, $null, 'DoNotExpandEnvironmentNames')
        }
    }
    finally { $key.Dispose() }
    $values
}

function Get-UserValue([string]$Subkey, [string]$Name) {
    $values = Get-UserValues $Subkey
    if ($values.ContainsKey($Name)) { return $values[$Name] }
    $null
}

function Save-Snapshot([string]$Label, [string[]]$Keys) {
    # reg.exe reports a missing key on stderr, which Windows PowerShell 5.1
    # turns into a terminating error under 'Stop'. A missing key is an
    # answer here, not a failure.
    $ErrorActionPreference = 'Continue'
    $file = Join-Path $EvidenceDir "reg-$Label.txt"
    foreach ($key in $Keys) {
        "=== reg query HKCU\$key /s" | Out-File -FilePath $file -Append -Encoding utf8
        & reg.exe query "HKCU\$key" /s 2>&1 | Out-File -FilePath $file -Append -Encoding utf8
    }
    Note "reg query snapshot: $file"
}

function Get-PathEntries {
    $path = Get-UserValue 'Environment' 'Path'
    if ($null -eq $path) { return @() }
    @($path -split ';' | Where-Object { $_ -ne '' })
}

function Test-SameEntries([string[]]$A, [string[]]$B) {
    ($A -join "`n") -eq ($B -join "`n")
}

# Setup and the uninstaller each hand over to a second process; wait for the
# visible result rather than trusting the first one's exit.
function Wait-Until([scriptblock]$Condition, [int]$Seconds) {
    $deadline = (Get-Date).AddSeconds($Seconds)
    while ((Get-Date) -lt $deadline) {
        if (& $Condition) { return $true }
        Start-Sleep -Milliseconds 500
    }
    & $Condition
}

# --- before ----------------------------------------------------------------------

if (-not $EvidenceDir) {
    $EvidenceDir = Join-Path ([System.IO.Path]::GetTempPath()) ("bachelorpad-install-test-" + (Get-Date -Format 'yyyyMMdd-HHmmss'))
}
New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null
$setup = (Resolve-Path -LiteralPath $SetupPath).Path

# Entries resolved against the default directory for the pre-check; the real
# directory is read back from Setup's own registration after install.
$expected = @(Get-ExpectedEntries $defaultApp)
$snapshotKeys = @($expected | ForEach-Object { $_.Subkey } | Sort-Object -Unique) + @($uninstallKey, 'Environment')
$sharedKeys = @($expected | Where-Object { -not $_.Owned } | ForEach-Object { $_.Subkey } | Sort-Object -Unique)
$ownedKeys = @($expected | Where-Object { $_.Owned } | ForEach-Object { $_.Subkey } | Sort-Object -Unique)

Write-Host "Before install"
Save-Snapshot 'before' $snapshotKeys
$present = @($expected | Where-Object { -not $_.KeyOnly -and $null -ne (Get-UserValue $_.Subkey $_.Name) })
if ($present.Count -gt 0 -or (Test-UserKey $uninstallKey)) {
    foreach ($entry in $present) { Note "already present: HKCU\$($entry.Subkey) [$($entry.Name)]" }
    throw ("BachelorPad+ is already installed or registered on this machine. Uninstall it, or run the " +
        "removal .reg, and try again.")
}
Pass "no BachelorPad+ value is present before install"

$sharedBefore = @{}
foreach ($key in $sharedKeys) { $sharedBefore[$key] = Get-UserValues $key }
$pathBefore = Get-PathEntries
$pathValueBefore = Get-UserValue 'Environment' 'Path'

# A marker in each of the user's directories. Silent uninstall must leave
# every one of them where it was.
$marker = "Test-Install-marker-$([guid]::NewGuid()).txt"
$directories = @(Get-UserDirectories)
$created = @()
foreach ($dir in $directories) {
    if (-not (Test-Path -LiteralPath $dir.Path)) {
        New-Item -ItemType Directory -Path $dir.Path | Out-Null
        $created += $dir.Path
    }
    Set-Content -LiteralPath (Join-Path $dir.Path $marker) -Value 'user data; must survive uninstall'
}
Pass "marker files written to $($directories.Count) user directories"

# --- install ------------------------------------------------------------------------

Write-Host "Installing $setup"
$setupLog = Join-Path $EvidenceDir 'setup.log'
$process = Start-Process -FilePath $setup -Wait -PassThru -ArgumentList @(
    '/VERYSILENT', '/CURRENTUSER', '/SUPPRESSMSGBOXES', '/NORESTART', '/MERGETASKS="addtopath"', "/LOG=`"$setupLog`"")
Check ($process.ExitCode -eq 0) "Setup exited 0 (got $($process.ExitCode))"
Check (Wait-Until { Test-UserKey $uninstallKey } 60) "Setup registered its uninstaller (HKCU\$uninstallKey)"

$uninstallValues = Get-UserValues $uninstallKey
$app = if ($uninstallValues.ContainsKey('InstallLocation')) { $uninstallValues['InstallLocation'].TrimEnd('\') } else { $defaultApp }
Check ($app -eq $defaultApp) "installed to $defaultApp (got $app)"
$expected = @(Get-ExpectedEntries $app)

try {
    Write-Host "After install"
    Save-Snapshot 'installed' $snapshotKeys

    foreach ($entry in $expected) {
        if ($entry.KeyOnly) {
            Check (Test-UserKey $entry.Subkey) "key HKCU\$($entry.Subkey) exists"
            continue
        }
        $actual = Get-UserValue $entry.Subkey $entry.Name
        $label = if ($entry.Name -eq '') { '(default)' } else { $entry.Name }
        Check ($null -ne $actual -and $actual -ceq $entry.Data) "HKCU\$($entry.Subkey) [$label] = $($entry.Data)"
    }

    foreach ($file in 'bachelorpad.exe', 'bpad.cmd', 'LICENSE-MIT', 'LICENSE-APACHE', 'THIRD-PARTY-NOTICES.txt') {
        Check (Test-Path -LiteralPath (Join-Path $app $file) -PathType Leaf) "installed $file"
    }
    $shortcut = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\BachelorPad+.lnk'
    Check (Test-Path -LiteralPath $shortcut) "Start menu shortcut $shortcut"
    Check ((Get-PathEntries) -contains $app) "user PATH contains $app"

    if (-not $ExpectedVersion) { $ExpectedVersion = $uninstallValues['DisplayVersion'] }
    $bpad = Join-Path $app 'bpad.cmd'
    $ErrorActionPreference = 'Continue'
    $output = (& $bpad --version 2>&1 | Out-String).Trim()
    $exit = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    Check ($exit -eq 0) "bpad --version exited 0 (got $exit)"
    Check ($output -eq "bachelorpad $ExpectedVersion") "bpad --version printed 'bachelorpad $ExpectedVersion' (got '$output')"
}
finally {
    # --- uninstall ------------------------------------------------------------------
    Write-Host "Uninstalling"
    $uninstaller = (Get-UserValues $uninstallKey)['UninstallString']
    if ($uninstaller) {
        $uninstallLog = Join-Path $EvidenceDir 'uninstall.log'
        $process = Start-Process -FilePath $uninstaller.Trim('"') -Wait -PassThru -ArgumentList @(
            '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/LOG=`"$uninstallLog`"")
        Check ($process.ExitCode -eq 0) "uninstaller exited 0 (got $($process.ExitCode))"
        Check (Wait-Until { -not (Test-UserKey $uninstallKey) } 120) "the uninstaller removed its own registration"
    }
    else {
        Fail "no UninstallString under HKCU\$uninstallKey; uninstall by hand before running this again"
    }
}

# --- after -----------------------------------------------------------------------------

Write-Host "After uninstall"
Save-Snapshot 'after' $snapshotKeys

foreach ($entry in $expected | Where-Object { -not $_.KeyOnly }) {
    $label = if ($entry.Name -eq '') { '(default)' } else { $entry.Name }
    Check ($null -eq (Get-UserValue $entry.Subkey $entry.Name)) "gone: HKCU\$($entry.Subkey) [$label]"
}
foreach ($key in $ownedKeys) {
    Check (-not (Test-UserKey $key)) "gone: key HKCU\$key"
}
foreach ($key in $sharedKeys) {
    $before = $sharedBefore[$key]
    $after = Get-UserValues $key
    $same = $before.Count -eq $after.Count
    foreach ($name in $before.Keys) {
        if (-not $after.ContainsKey($name) -or $after[$name] -cne $before[$name]) { $same = $false }
    }
    Check $same "every other value in HKCU\$key is as it was ($($before.Count) value(s))"
    if ($after.Count -eq 0 -and (Test-UserKey $key)) {
        Note "HKCU\$key is left as an empty key: it is shared, so only our value is removed"
    }
}
Check (Test-SameEntries $pathBefore (Get-PathEntries)) "user PATH entries are as they were"
Check (($null -eq $pathValueBefore) -eq ($null -eq (Get-UserValue 'Environment' 'Path'))) "the user PATH value exists exactly when it did before"
Check (-not (Test-Path -LiteralPath $app)) "install directory $app is gone"
Check (-not (Test-Path -LiteralPath (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\BachelorPad+.lnk'))) "Start menu shortcut is gone"
foreach ($dir in $directories) {
    Check (Test-Path -LiteralPath (Join-Path $dir.Path $marker) -PathType Leaf) "silent uninstall kept the user's $($dir.Kind) directory ($($dir.Path))"
}

# --- tidy up the markers; never anything the user had -----------------------------------

foreach ($dir in $directories) {
    Remove-Item -LiteralPath (Join-Path $dir.Path $marker) -Force -ErrorAction SilentlyContinue
}
foreach ($path in $created) {
    if (-not (Get-ChildItem -LiteralPath $path -Force -ErrorAction SilentlyContinue)) {
        Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
    }
}
$productLocal = Join-Path $env:LOCALAPPDATA 'bachelorpad'
if ($created -and (Test-Path -LiteralPath $productLocal) -and -not (Get-ChildItem -LiteralPath $productLocal -Force)) {
    Remove-Item -LiteralPath $productLocal -Force -ErrorAction SilentlyContinue
}

$summary = Join-Path $EvidenceDir 'summary.txt'
if ($script:failures.Count -eq 0) {
    "PASS: every check passed" | Set-Content -LiteralPath $summary
    Write-Host "PASS -- evidence in $EvidenceDir" -ForegroundColor Green
    exit 0
}
(@("FAIL: $($script:failures.Count) check(s) failed") + $script:failures) | Set-Content -LiteralPath $summary
Write-Host "FAIL -- $($script:failures.Count) check(s) failed; evidence in $EvidenceDir" -ForegroundColor Red
exit 1
