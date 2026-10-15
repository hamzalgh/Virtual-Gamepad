# Read-only static checks plus mocked ownership scenarios. Never runs setup or touches system policy.
$ErrorActionPreference = 'Stop'
foreach ($file in Get-ChildItem -LiteralPath $PSScriptRoot -Filter '*.ps1') {
    $tokens=$null; $errors=$null
    [void][Management.Automation.Language.Parser]::ParseFile($file.FullName,[ref]$tokens,[ref]$errors)
    if ($errors) { throw ($errors | Out-String) }
}
$source=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'virtual-gamepad.iss') -Raw
foreach ($required in @('CloseApplications=no','RestartApplications=no','Flags: unchecked','AppMutex=Global\VirtualGamepad.Desktop.Runtime','SetupMutex=Global\VirtualGamepad.Desktop.Setup','runasoriginaluser','MB_DEFBUTTON2','CheckStopped')) {
    if (!$source.Contains($required)) { throw "Missing setup guard: $required" }
}
$owned=Join-Path $PSScriptRoot 'owned-setup.ps1'
$initialization = ($source -split 'function InitializeUninstall\(\): Boolean;',2)[1] -split 'procedure CurUninstallStepChanged',2
if ($initialization.Count -ne 2 -or $initialization[0].Contains("'RemoveOwned'") -or !$initialization[1].Contains('if CurUninstallStep <> usUninstall then Exit;')) { throw 'Uninstall cleanup must follow confirmation' }
if (!$source.Contains('#if Ver != 0x06070300')) { throw 'Compiler version guard missing' }
$program='C:\Program Files\VirtualGamepad\virtual-gamepad-agent.exe'
$packagingTestState = @{Processes=@();Rules=@();RuleProgram='';Removed=0;Created=$null;Startup=$null;StartupRemoved=0}
# All OS-facing operations used by these scenarios are shadowed in this scope.
function Get-CimInstance {param($ClassName,$Filter) $packagingTestState.processes}
function Get-NetFirewallRule {param($PolicyStore,$Name,$ErrorAction) $packagingTestState.rules}
function Get-NetFirewallApplicationFilter {param([Parameter(ValueFromPipeline)]$InputObject) process {[pscustomobject]@{Program=$packagingTestState.ruleProgram}}}
function Remove-NetFirewallRule {param([Parameter(ValueFromPipeline)]$InputObject) process {$packagingTestState.removed++}}
function New-NetFirewallRule {param($PolicyStore,$Name,$DisplayName,$Direction,$Action,$Protocol,$LocalPort,$Program,$Profile,$RemoteAddress) $packagingTestState.created=$PSBoundParameters}
function Get-ItemProperty {param($LiteralPath,$Name,$ErrorAction) $packagingTestState.startup}
function Remove-ItemProperty {param($LiteralPath,$Name) $packagingTestState.startupRemoved++}
function Expect-Failure([scriptblock]$Body) {$failed=$false;try{& $Body}catch{$failed=$true};if(!$failed){throw 'Expected rejection'}}
$packagingTestState.processes=@([pscustomobject]@{ExecutablePath=$program})
Expect-Failure {& $owned -Action CheckStopped -Program $program}
$packagingTestState.processes=@([pscustomobject]@{ExecutablePath=$null})
Expect-Failure {& $owned -Action CheckStopped -Program $program}
$packagingTestState.processes=@();$packagingTestState.rules=@([pscustomobject]@{Name='VirtualGamepad.Desktop.UDP'});$packagingTestState.ruleProgram='C:\Other\virtual-gamepad-agent.exe';$packagingTestState.removed=0
Expect-Failure {& $owned -Action AllowPrivateUdp -Program $program}
if ($packagingTestState.removed) {throw 'Foreign rule changed'}
$packagingTestState.rules=@();$packagingTestState.created=$null
& $owned -Action AllowPrivateUdp -Program $program -Port 9876
if ($packagingTestState.created.Profile -ne 'Private' -or $packagingTestState.created.RemoteAddress -ne 'LocalSubnet' -or $packagingTestState.created.Program -ne $program -or $packagingTestState.created.LocalPort -ne 9876 -or $packagingTestState.created.Protocol -ne 'UDP') {throw 'Firewall scope mismatch'}
$packagingTestState.startup=[pscustomobject]@{'VirtualGamepad.Desktop'='"C:\Other\virtual-gamepad-agent.exe" --tray'};$packagingTestState.startupRemoved=0
& $owned -Action RemoveOwned -Program $program
if ($packagingTestState.startupRemoved) {throw 'Foreign startup entry removed'}
$packagingTestState.startup=[pscustomobject]@{'VirtualGamepad.Desktop'=('"'+$program+'" --tray')}
& $owned -Action RemoveOwned -Program $program
if ($packagingTestState.startupRemoved -ne 1) {throw 'Owned startup entry was not selected'}
'PASS: PowerShell parsing, installer guards, running/unknown process refusal, foreign ownership preservation, Private UDP scope and exact startup cleanup (mocked only).'
