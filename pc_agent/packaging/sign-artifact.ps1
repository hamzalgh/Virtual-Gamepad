[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$SignTool,
    [Parameter(Mandatory)][string]$Candidate,
    [Parameter(Mandatory)][ValidatePattern('^[A-Fa-f0-9]{40}$')][string]$CertificateThumbprint,
    [Parameter(Mandatory)][uri]$TimestampUrl
)
$ErrorActionPreference = 'Stop'
if ($TimestampUrl.Scheme -ne 'https') { throw 'Use the certificate provider approved HTTPS timestamp service' }
$path = (Resolve-Path -LiteralPath $Candidate).Path
if ([IO.Path]::GetExtension($path) -ne '.exe') { throw 'Only staged executable artifacts are supported' }
# Explicit invocation signs a supplied artifact using an existing certificate. No key creation.
& $SignTool sign /sha1 $CertificateThumbprint /fd SHA256 /tr $TimestampUrl.AbsoluteUri /td SHA256 $path
if ($LASTEXITCODE -ne 0) { throw 'Signing failed' }
& $SignTool verify /pa /all /v $path
if ($LASTEXITCODE -ne 0) { throw 'Signature verification failed' }
$signature = Get-AuthenticodeSignature -LiteralPath $path
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Thumbprint -ne $CertificateThumbprint -or !$signature.TimeStamperCertificate) { throw 'Expected signer or trusted timestamp missing' }
Get-FileHash -LiteralPath $path -Algorithm SHA256
