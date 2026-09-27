# Authenticode signing for the release (release plan M3-04, decision 3).
#
# Dot-sourced by scripts/Invoke-Release.ps1 and Build-Installer.ps1. Nothing
# here holds a secret: the identity is configured on the signing machine,
# through environment variables, and never enters the repository.
#
# Two routes, one tool. Both use signtool with SHA-256 digests and an
# RFC 3161 timestamp, so a signature outlives the certificate that made it:
#
#   Azure Artifact Signing (formerly Trusted Signing; the owner's choice)
#     BP_SIGN_METADATA  path to metadata.json: Endpoint, CodeSigningAccountName,
#                       CertificateProfileName (Microsoft's format; the
#                       endpoint must be the account's region)
#     BP_SIGN_DLIB      optional; Azure.CodeSigning.Dlib.dll (default: where
#                       `winget install Microsoft.Azure.ArtifactSigningClientTools`
#                       puts it)
#     Authentication is Azure's own: `az login` for a person, or
#     AZURE_CLIENT_ID / AZURE_TENANT_ID / AZURE_CLIENT_SECRET.
#     Timestamp: http://timestamp.acs.microsoft.com
#
#   An OV certificate from a CA (a token or a CA's cloud signer that
#   installs into the certificate store)
#     BP_SIGN_CERT_SHA1 the certificate's thumbprint
#     BP_SIGN_TIMESTAMP the CA's RFC 3161 timestamp URL (required)
#
#   BP_SIGNTOOL        optional; signtool.exe (default: the Artifact Signing
#                      client tools' copy, then PATH, then the newest
#                      Windows Kits x64 one)
#
# Microsoft's documentation: "Set up signing integrations to use Artifact
# Signing" on learn.microsoft.com. At the time of writing, Artifact Signing
# is open to individual developers in the USA and Canada only.

$script:ArtifactSigningTimestamp = 'http://timestamp.acs.microsoft.com'
$script:ArtifactSigningTools = 'C:\Program Files (x86)\Microsoft\ArtifactSigningClientTools\bin'

function Find-SignTool {
    if ($env:BP_SIGNTOOL) {
        if (-not (Test-Path -LiteralPath $env:BP_SIGNTOOL)) { throw "BP_SIGNTOOL names $env:BP_SIGNTOOL, which does not exist" }
        return $env:BP_SIGNTOOL
    }
    $bundled = Join-Path $script:ArtifactSigningTools 'signtool.exe'
    if (Test-Path -LiteralPath $bundled) { return $bundled }
    $onPath = Get-Command signtool.exe -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($onPath) { return $onPath.Source }
    $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $newest = Get-ChildItem -Path $kits -Filter signtool.exe -Recurse -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match '\\x64\\' } |
        Sort-Object FullName -Descending | Select-Object -First 1
    if ($newest) { return $newest.FullName }
    throw "signtool.exe not found: install the Artifact Signing client tools or the Windows SDK, or set BP_SIGNTOOL"
}

# The signtool arguments for this machine's configured identity, without the
# file name, or $null when no identity is configured.
function Get-SigningArguments {
    if ($env:BP_SIGN_METADATA -and $env:BP_SIGN_CERT_SHA1) {
        throw "both BP_SIGN_METADATA and BP_SIGN_CERT_SHA1 are set; configure one signing route"
    }
    if ($env:BP_SIGN_METADATA) {
        if (-not (Test-Path -LiteralPath $env:BP_SIGN_METADATA)) { throw "BP_SIGN_METADATA names $env:BP_SIGN_METADATA, which does not exist" }
        $dlib = if ($env:BP_SIGN_DLIB) { $env:BP_SIGN_DLIB } else { Join-Path $script:ArtifactSigningTools 'Azure.CodeSigning.Dlib.dll' }
        if (-not (Test-Path -LiteralPath $dlib)) { throw "the Artifact Signing dlib is not at $dlib; set BP_SIGN_DLIB" }
        return @('sign', '/v', '/fd', 'SHA256', '/tr', $script:ArtifactSigningTimestamp, '/td', 'SHA256',
            '/dlib', $dlib, '/dmdf', $env:BP_SIGN_METADATA)
    }
    if ($env:BP_SIGN_CERT_SHA1) {
        if (-not $env:BP_SIGN_TIMESTAMP) { throw "BP_SIGN_CERT_SHA1 is set but BP_SIGN_TIMESTAMP is not: an untimestamped signature dies with its certificate" }
        return @('sign', '/v', '/fd', 'SHA256', '/tr', $env:BP_SIGN_TIMESTAMP, '/td', 'SHA256',
            '/sha1', $env:BP_SIGN_CERT_SHA1)
    }
    return $null
}

function Test-SigningConfigured { [bool](Get-SigningArguments) }

# Sign one file, then verify it against the Windows trust policy. Both are
# fatal: a release that says it is signed and is not is worse than one that
# says it is not.
function Invoke-FileSigning {
    param([Parameter(Mandatory)][string]$Path)
    $arguments = Get-SigningArguments
    if (-not $arguments) { throw "no signing identity is configured (see packaging/windows/Signing.ps1)" }
    $signtool = Find-SignTool
    & $signtool @arguments $Path | Write-Host
    if ($LASTEXITCODE -ne 0) { throw "signtool sign failed for $Path (exit $LASTEXITCODE)" }
    & $signtool verify /pa /v $Path | Write-Host
    if ($LASTEXITCODE -ne 0) { throw "signtool verify /pa rejected $Path (exit $LASTEXITCODE)" }
}

# The whole command as Inno Setup's /S wants it, with $f where the file goes,
# so ISCC signs the setup and its uninstaller as it writes them.
function Get-InnoSignCommand {
    $arguments = Get-SigningArguments
    if (-not $arguments) { return $null }
    $quoted = $arguments | ForEach-Object { if ($_ -match '\s') { "`"$_`"" } else { $_ } }
    "`"$(Find-SignTool)`" $($quoted -join ' ') `$f"
}
