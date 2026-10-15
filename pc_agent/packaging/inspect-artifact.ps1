<#
Reads one Windows candidate without launching, installing, signing or changing it.
The result is review evidence only. It does not approve a release.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $CandidatePath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$candidate = Get-Item -LiteralPath $CandidatePath -Force
if ($candidate.PSIsContainer -or $candidate.Extension -ne '.exe') {
    throw 'Provide a single Windows executable or installer candidate (.exe).'
}
if (($candidate.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw 'Inspect the actual artifact, not a reparse point.'
}
$prerequisites = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'release-prerequisites.json') -Raw | ConvertFrom-Json
$hash = Get-FileHash -LiteralPath $candidate.FullName -Algorithm SHA256
$signature = Get-AuthenticodeSignature -LiteralPath $candidate.FullName
[pscustomobject]@{
    Artifact = $candidate.Name
    Bytes = $candidate.Length
    SHA256 = $hash.Hash
    SignatureStatus = [string] $signature.Status
    FileVersion = $candidate.VersionInfo.FileVersion
    ProductVersion = $candidate.VersionInfo.ProductVersion
    ArchitectureVerified = $false
    ReleaseApproved = $false
    PendingGates = $prerequisites.pendingGates
}
