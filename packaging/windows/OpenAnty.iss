; Inno Setup script for OpenAnty easy installer (G11 / F-051)
; Build: install Inno Setup, then:
;   iscc packaging\windows\OpenAnty.iss
; Requires release binaries built first (see build-portable.ps1)

#define MyAppName "openanty"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "OpenAnty Contributors"
#define MyAppURL "https://github.com/openanty/openanty"
#define MyAppExeName "openanty.exe"

[Setup]
AppId={{A7F3C2E1-9B4D-4E8A-9C1F-OpenAnty0001}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
DefaultDirName={autopf}\OpenAnty
DefaultGroupName=OpenAnty
DisableProgramGroupPage=yes
LicenseFile=..\..\LICENSE
InfoBeforeFile=..\..\RESPONSIBLE_USE.md
OutputDir=..\out
OutputBaseFilename=OpenAnty-Setup-{#MyAppVersion}-windows-x64
Compression=lzma
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "addpath"; Description: "Add OpenAnty to user PATH"; GroupDescription: "Options:"; Flags: checkedonce

[Files]
Source: "..\..\target\release\openantyd.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "..\..\target\release\openanty.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\RESPONSIBLE_USE.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\docs\quick-start.md"; DestDir: "{app}"; DestName: "QUICKSTART.md"; Flags: ignoreversion

[Icons]
Name: "{group}\OpenAnty Doctor"; Filename: "{app}\bin\openanty.exe"; Parameters: "doctor"
Name: "{group}\OpenAnty MCP Config"; Filename: "{app}\bin\openanty.exe"; Parameters: "mcp-config"
Name: "{group}\Uninstall OpenAnty"; Filename: "{uninstallexe}"
Name: "{autodesktop}\OpenAnty Doctor"; Filename: "{app}\bin\openanty.exe"; Parameters: "doctor"; Tasks: desktopicon

[Run]
Filename: "{app}\bin\openanty.exe"; Parameters: "init"; StatusMsg: "Initializing OpenAnty data directory..."; Flags: runhidden waituntilterminated
Filename: "{app}\bin\openanty.exe"; Parameters: "doctor"; Description: "Run OpenAnty Doctor"; Flags: postinstall nowait skipifsilent

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
