; Review source. Compile only with verified Inno Setup 6.7.3. No bundled driver.
#if Ver != 0x06070300
  #error Requires Inno Setup 6.7.3
#endif
#ifndef Stage
  #error Stage is required
#endif
#ifndef Version
  #error Version is required
#endif
#ifndef Flavor
  #define Flavor "unsigned-review"
#endif
[Setup]
AppId=VirtualGamepad.Desktop
AppName=Virtual Gamepad
AppVersion={#Version}
AppPublisher=Virtual Gamepad contributors
DefaultDirName={autopf}\VirtualGamepad
DefaultGroupName=Virtual Gamepad
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.19041
PrivilegesRequired=admin
CloseApplications=no
RestartApplications=no
AppMutex=Global\VirtualGamepad.Desktop.Runtime
SetupMutex=Global\VirtualGamepad.Desktop.Setup
Uninstallable=yes
OutputDir={#Stage}\installer
OutputBaseFilename=VirtualGamepad-{#Version}-x64-{#Flavor}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
LicenseFile={#Stage}\LICENSE
InfoAfterFile=troubleshooting.txt
SetupLogging=yes
[Tasks]
Name: privateudp; Description: "Allow UDP 8765 from the local subnet on Private networks for this program"; Flags: unchecked
[Files]
Source: "{#Stage}\virtual-gamepad-agent.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Stage}\LICENSE"; DestDir: "{app}"
Source: "{#Stage}\NOTICE"; DestDir: "{app}"
Source: "{#Stage}\THIRD_PARTY_NOTICES.txt"; DestDir: "{app}"
Source: "troubleshooting.txt"; DestDir: "{app}"
Source: "owned-setup.ps1"; DestDir: "{app}"
[Icons]
Name: "{group}\Virtual Gamepad"; Filename: "{app}\virtual-gamepad-agent.exe"; Parameters: "--tray"
Name: "{group}\Virtual Gamepad terminal"; Filename: "{app}\virtual-gamepad-agent.exe"
Name: "{group}\Troubleshooting"; Filename: "{app}\troubleshooting.txt"
[Run]
Filename: "{app}\virtual-gamepad-agent.exe"; Parameters: "--tray-after-setup"; Description: "Start Virtual Gamepad after setup closes"; Flags: postinstall nowait skipifsilent unchecked runasoriginaluser
[Code]
function SetupAction(const Action, Script: String; DeleteData: Boolean): Boolean;
var Code: Integer; Args: String;
begin
  Args := '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + Script + '" -Action ' + Action + ' -Program "' + ExpandConstant('{app}\virtual-gamepad-agent.exe') + '"';
  if DeleteData then Args := Args + ' -RemoveCurrentUserData';
  Result := Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'), Args, '', SW_HIDE, ewWaitUntilTerminated, Code) and (Code = 0);
end;
function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  ExtractTemporaryFile('owned-setup.ps1');
  if not SetupAction('CheckStopped', ExpandConstant('{tmp}\owned-setup.ps1'), False) then
    Result := 'Exit the installed Virtual Gamepad through Exit (or Ctrl+C), wait for cleanup, then retry. Setup never force-closes it.';
end;
procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('privateudp') then
    if not SetupAction('AllowPrivateUdp', ExpandConstant('{app}\owned-setup.ps1'), False) then
      MsgBox('The optional firewall rule was not configured. See Troubleshooting; setup did not broaden another program''s rule.', mbError, MB_OK);
end;
function InitializeUninstall(): Boolean;
begin
  if CheckForMutexes('Global\VirtualGamepad.Desktop.Setup') then begin
    MsgBox('Another setup is running. Wait for it to finish.', mbError, MB_OK);
    Result := False;
    Exit;
  end;
  CreateMutex('Global\VirtualGamepad.Desktop.Setup');
  Result := SetupAction('CheckStopped', ExpandConstant('{app}\owned-setup.ps1'), False);
  if not Result then begin
    MsgBox('Exit Virtual Gamepad and wait for controller cleanup before uninstalling.', mbError, MB_OK);
    Exit;
  end;
end;
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var DeleteData: Boolean;
begin
  if CurUninstallStep <> usUninstall then Exit;
  { This event is after the user's normal uninstall confirmation. }
  DeleteData := False;
  if not UninstallSilent then
    DeleteData := MsgBox('Also remove settings, pairings and logs for the account running this uninstaller (' + GetUserNameString + '), under ' + ExpandConstant('{localappdata}\VirtualGamepad') + '? Choose No to preserve data (recommended). Other accounts are never scanned.', mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES;
  if not SetupAction('RemoveOwned', ExpandConstant('{app}\owned-setup.ps1'), DeleteData) then
    RaiseException('Owned cleanup failed. Uninstall stopped; inspect setup logs before retrying.');
end;
