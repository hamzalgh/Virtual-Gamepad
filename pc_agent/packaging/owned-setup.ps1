[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('CheckStopped','AllowPrivateUdp','RemoveOwned')][string]$Action,
    [Parameter(Mandatory)][string]$Program,
    [ValidateRange(1,65535)][int]$Port = 8765,
    [switch]$RemoveCurrentUserData
)
$ErrorActionPreference = 'Stop'
$programPath = [IO.Path]::GetFullPath($Program)
if ([IO.Path]::GetFileName($programPath) -ne 'virtual-gamepad-agent.exe') { throw 'Unexpected owned executable' }
$ownedRule = 'VirtualGamepad.Desktop.UDP'
function Assert-Stopped {
    # Fail closed if inventory is unavailable. Never terminate a process.
    $processes = @(Get-CimInstance Win32_Process -Filter "Name='virtual-gamepad-agent.exe'")
    foreach ($process in $processes) {
        if (!$process.ExecutablePath) { throw 'Cannot identify an agent process. Exit all agent instances first.' }
        if ([IO.Path]::GetFullPath($process.ExecutablePath) -eq $programPath) { throw 'Exit Virtual Gamepad and wait for neutral/detach cleanup before setup.' }
    }
}
Assert-Stopped
if ($Action -eq 'CheckStopped') { return }
$rules = @(Get-NetFirewallRule -PolicyStore PersistentStore -Name $ownedRule -ErrorAction SilentlyContinue)
foreach ($rule in $rules) {
    $filter = $rule | Get-NetFirewallApplicationFilter
    if (@($filter).Count -ne 1 -or [IO.Path]::GetFullPath($filter.Program) -ne $programPath) { throw 'Firewall identifier belongs to another path; preserved.' }
}
if ($Action -eq 'AllowPrivateUdp') {
    if ($rules.Count) { $rules | Remove-NetFirewallRule }
    New-NetFirewallRule -PolicyStore PersistentStore -Name $ownedRule -DisplayName 'Virtual Gamepad private UDP' -Direction Inbound -Action Allow -Protocol UDP -LocalPort $Port -Program $programPath -Profile Private -RemoteAddress LocalSubnet | Out-Null
    return
}
if ($rules.Count) { $rules | Remove-NetFirewallRule }
$runPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$entry = Get-ItemProperty -LiteralPath $runPath -Name 'VirtualGamepad.Desktop' -ErrorAction SilentlyContinue
$expected = '"' + $programPath + '" --tray'
if ($entry -and $entry.'VirtualGamepad.Desktop' -eq $expected) { Remove-ItemProperty -LiteralPath $runPath -Name 'VirtualGamepad.Desktop' }
if ($RemoveCurrentUserData) {
    # Explicitly selected current elevated account only; never enumerate other profiles.
    $localRoot = [IO.Path]::GetFullPath([Environment]::GetFolderPath('LocalApplicationData'))
    $dataPath = [IO.Path]::GetFullPath((Join-Path $localRoot 'VirtualGamepad'))
    if ([IO.Path]::GetDirectoryName($dataPath) -ne $localRoot) { throw 'Unexpected data root' }
    if (Test-Path -LiteralPath $dataPath) {
        $allItems = @((Get-Item -LiteralPath $dataPath)) + @(Get-ChildItem -LiteralPath $dataPath -Force -Recurse)
        if ($allItems | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }) { throw 'Linked data path preserved; inspect manually.' }
        # A stopped desktop still owns this file. Refuse data deletion until it exits.
        $lockPath = Join-Path $dataPath 'owner.lock'
        if (Test-Path -LiteralPath $lockPath) { $lock = [IO.File]::Open($lockPath,'Open','ReadWrite','None'); $lock.Dispose() }
        Remove-Item -LiteralPath $dataPath -Recurse -Force
    }
}
