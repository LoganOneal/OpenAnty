; Inno Setup script for GhostFox easy installer (G11 / F-051)
; Build: install Inno Setup, then:
;   iscc packaging\windows\GhostFox.iss
; Requires release binaries built first (see build-portable.ps1)

#define MyAppName "GhostFox"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "GhostFox Contributors"
#define MyAppURL "https://github.com/ghostfox/ghostfox"
#define MyAppExeName "ghostfox.exe"

[Setup]
AppId={{A7F3C2E1-9B4D-4E8A-9C1F-GHOSTFOX0001}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
DefaultDirName={autopf}\GhostFox
DefaultGroupName=GhostFox
DisableProgramGroupPage=yes
LicenseFile=..\..\LICENSE
InfoBeforeFile=..\..\RESPONSIBLE_USE.md
OutputDir=..\out
OutputBaseFilename=GhostFox-Setup-{#MyAppVersion}-windows-x64
Compression=lzma
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "addpath"; Description: "Add GhostFox to user PATH"; GroupDescription: "Options:"; Flags: checkedonce

[Files]
Source: "..\..\target\release\ghostfoxd.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "..\..\target\release\ghostfox.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\RESPONSIBLE_USE.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\docs\quick-start.md"; DestDir: "{app}"; DestName: "QUICKSTART.md"; Flags: ignoreversion

[Icons]
Name: "{group}\GhostFox Doctor"; Filename: "{app}\bin\ghostfox.exe"; Parameters: "doctor"
Name: "{group}\GhostFox MCP Config"; Filename: "{app}\bin\ghostfox.exe"; Parameters: "mcp-config"
Name: "{group}\Uninstall GhostFox"; Filename: "{uninstallexe}"
Name: "{autodesktop}\GhostFox Doctor"; Filename: "{app}\bin\ghostfox.exe"; Parameters: "doctor"; Tasks: desktopicon

[Run]
Filename: "{app}\bin\ghostfox.exe"; Parameters: "init"; StatusMsg: "Initializing GhostFox data directory..."; Flags: runhidden waituntilterminated
Filename: "{app}\bin\ghostfox.exe"; Parameters: "doctor"; Description: "Run GhostFox Doctor"; Flags: postinstall nowait skipifsilent

[Code]
procedure EnvAddPath(Path: string);
var
  Paths: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Paths) then
    Paths := '';
  if Pos(LowerCase(Path), LowerCase(Paths)) = 0 then
  begin
    if Paths <> '' then Paths := Paths + ';';
    Paths := Paths + Path;
    RegWriteStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Paths);
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    if WizardIsTaskSelected('addpath') then
      EnvAddPath(ExpandConstant('{app}\bin'));
  end;
end;
