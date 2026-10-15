[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$CargoMetadata,
    [Parameter(Mandatory)][string]$AndroidInventory,
    [Parameter(Mandatory)][string]$ProtobufLicense
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$metadata = Get-Content -LiteralPath $CargoMetadata -Raw | ConvertFrom-Json
$windows = [Text.StringBuilder]::new()
[void]$windows.AppendLine('Virtual Gamepad Windows dependency notices. Generated from the locked Windows Cargo graph; includes build tools conservatively. Preserve full texts with redistribution.')
foreach ($package in ($metadata.packages | Sort-Object name,version)) {
    if ($package.name -eq 'virtual-gamepad-agent') { continue }
    if (!$package.license) { throw "Missing license: $($package.name)" }
    [void]$windows.AppendLine("`n=== $($package.name) $($package.version) | $($package.license) ===")
    $files = @(Get-ChildItem -LiteralPath (Split-Path $package.manifest_path) -File | Where-Object {$_.Name -match '^(LICENSE|COPYING|NOTICE)'} | Sort-Object Name)
    if (!$files.Count) { throw "Missing license text: $($package.name)" }
    foreach ($file in $files) { [void]$windows.AppendLine("--- $($file.Name) ---"); [void]$windows.AppendLine([IO.File]::ReadAllText($file.FullName)) }
}
[IO.File]::WriteAllText((Join-Path $PSScriptRoot 'THIRD_PARTY_NOTICES.txt'),$windows.ToString())
$android = [Text.StringBuilder]::new()
[void]$android.AppendLine('Virtual Gamepad Android release dependency notices. Coordinates and license declarations are from resolved Maven metadata. Apache 2.0 applies unless separately identified below.')
$seen = @{}
foreach ($row in (Get-Content -LiteralPath $AndroidInventory)) {
    $columns = $row -split "`t"
    $id = ($columns[0..2] -join ':')
    if ($seen.ContainsKey($id)) { continue }; $seen[$id] = $true
    $artifact = $columns[3]
    $poms = @(Get-ChildItem -LiteralPath (Split-Path (Split-Path $artifact)) -Filter '*.pom' -Recurse)
    if (!$poms.Count) { throw "Missing POM: $id" }
    [xml]$pom = Get-Content -LiteralPath $poms[0].FullName -Raw
    $licenses = @($pom.project.licenses.license | ForEach-Object {$_.name}) -join ', '
    if (!$licenses -and $id -eq 'com.google.guava:listenablefuture:1.0') { $licenses = 'Apache-2.0 (guava-parent 26.0-android / upstream Guava LICENSE)' }
    if (!$licenses -or $licenses -notmatch 'Apache|BSD-3-Clause') { throw "Review license for $id" }
    [void]$android.AppendLine("$id | $licenses")
    $zip = [IO.Compression.ZipFile]::OpenRead($artifact)
    try {
        foreach ($entry in $zip.Entries | Where-Object {$_.Name -match '^(LICENSE|NOTICE|COPYING)'}) {
            $reader = [IO.StreamReader]::new($entry.Open())
            try { [void]$android.AppendLine("--- $id / $($entry.FullName) ---"); [void]$android.AppendLine($reader.ReadToEnd()) } finally {$reader.Dispose()}
        }
    } finally {$zip.Dispose()}
}
[void]$android.AppendLine("`nApache License 2.0 full text:")
[void]$android.AppendLine([IO.File]::ReadAllText((Join-Path $repo 'LICENSE')))
[void]$android.AppendLine("`nAndroidX repackaged Protocol Buffers BSD notice (upstream protocolbuffers/protobuf LICENSE):")
[void]$android.AppendLine([IO.File]::ReadAllText((Resolve-Path -LiteralPath $ProtobufLicense)))
$assets = Join-Path $repo 'mobile_controller\app\src\main\assets'
[IO.Directory]::CreateDirectory($assets) | Out-Null
[IO.File]::WriteAllText((Join-Path $assets 'THIRD_PARTY_NOTICES.txt'),$android.ToString())
Copy-Item -LiteralPath (Join-Path $repo 'LICENSE') -Destination (Join-Path $assets 'LICENSE.txt')
Copy-Item -LiteralPath (Join-Path $repo 'NOTICE') -Destination (Join-Path $assets 'NOTICE.txt')
