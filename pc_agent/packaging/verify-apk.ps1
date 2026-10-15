[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$ApkSigner,
    [Parameter(Mandatory)][string]$Apk,
    [Parameter(Mandatory)][ValidatePattern('^[A-Fa-f0-9]{64}$')][string]$ExpectedCertificateSha256
)
$ErrorActionPreference = 'Stop'
$path = (Resolve-Path -LiteralPath $Apk).Path
$output = & $ApkSigner verify --verbose --print-certs $path 2>&1
if ($LASTEXITCODE -ne 0) { throw 'APK signature verification failed; no unsigned/debug fallback' }
$text = $output -join "`n"
if ($text -match 'CN=Android Debug' -or $text -notmatch ('(?i)certificate SHA-256 digest:\s*' + $ExpectedCertificateSha256)) { throw 'APK signer does not match the approved release identity' }
$output
Get-FileHash -LiteralPath $path -Algorithm SHA256
