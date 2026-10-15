[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Compiler,
    [Parameter(Mandatory)][ValidatePattern('^[A-Fa-f0-9]{64}$')][string]$CompilerSha256,
    [Parameter(Mandatory)][string]$Stage,
    [switch]$Official,
    [string]$CertificateThumbprint
)
$ErrorActionPreference = 'Stop'
$compilerFile = Get-Item -LiteralPath $Compiler
# ISCC's PE version is 0.0.0.0; the script checks the actual compiler Ver.
if ((Get-FileHash -LiteralPath $compilerFile.FullName -Algorithm SHA256).Hash -ne $CompilerSha256) { throw 'Compiler hash mismatch' }
$stagePath = (Resolve-Path -LiteralPath $Stage).Path
foreach ($name in @('virtual-gamepad-agent.exe','LICENSE','NOTICE','THIRD_PARTY_NOTICES.txt')) {
    if (!(Test-Path -LiteralPath (Join-Path $stagePath $name) -PathType Leaf)) { throw "Missing staged $name" }
}
$exe = Join-Path $stagePath 'virtual-gamepad-agent.exe'
$version = (Get-Item -LiteralPath $exe).VersionInfo.ProductVersion
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Version metadata is missing or invalid' }
$flavor = 'unsigned-review'
if ($Official) {
    $sig = Get-AuthenticodeSignature -LiteralPath $exe
    if (!$CertificateThumbprint -or $sig.Status -ne 'Valid' -or $sig.SignerCertificate.Thumbprint -ne $CertificateThumbprint) { throw 'Official packaging requires the expected valid executable signature' }
    $flavor = 'signed-candidate'
}
& $compilerFile.FullName "/DStage=$stagePath" "/DVersion=$version" "/DFlavor=$flavor" (Join-Path $PSScriptRoot 'virtual-gamepad.iss')
if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed' }
Get-ChildItem -LiteralPath (Join-Path $stagePath 'installer') -Filter '*.exe' | Get-FileHash -Algorithm SHA256
Write-Output 'Review candidate only. Official installers must still be signed, verified and manually accepted.'
