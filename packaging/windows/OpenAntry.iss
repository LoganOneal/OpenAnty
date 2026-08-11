; Inno Setup script for OpenAntry easy installer (G11 / F-051)
; Build: install Inno Setup, then:
;   iscc packaging\windows\OpenAntry.iss
; Requires release binaries built first (see build-portable.ps1)

#define MyAppName "openantry"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "OpenAntry Contributors"
#define MyAppURL "https://github.com/openantry/openantry"
#define MyAppExeName "openantry.exe"

[Setup]
AppId={{A7F3C2E1-9B4D-4E8A-9C1F-OpenAntry0001}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
DefaultDirName={autopf}\OpenAntry
DefaultGroupName=OpenAntry
DisableProgramGroupPage=yes
LicenseFile=..\..\LICENSE
InfoBeforeFile=..\..\RESPONSIBLE_USE.md
OutputDir=..\out
OutputBaseFilename=OpenAntry-Setup-{#MyAppVersion}-windows-x64
Compression=lzma
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "addpath"; Description: "Add OpenAntry to user PATH"; GroupDescription: "Options:"; Flags: checkedonce

[Files]
Source: "..\..\target\release\openantryd.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "..\..\target\release\openantry.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\RESPONSIBLE_USE.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\docs\quick-start.md"; DestDir: "{app}"; DestName: "QUICKSTART.md"; Flags: ignoreversion

[Icons]
Name: "{group}\OpenAntry Doctor"; Filename: "{app}\bin\openantry.exe"; Parameters: "doctor"
Name: "{group}\OpenAntry MCP Config"; Filename: "{app}\bin\openantry.exe"; Parameters: "mcp-config"
Name: "{group}\Uninstall OpenAntry"; Filename: "{uninstallexe}"
Name: "{autodesktop}\OpenAntry Doctor"; Filename: "{app}\bin\openantry.exe"; Parameters: "doctor"; Tasks: desktopicon

[Run]
Filename: "{app}\bin\openantry.exe"; Parameters: "init"; StatusMsg: "Initializing OpenAntry data directory..."; Flags: runhidden waituntilterminated
Filename: "{app}\bin\openantry.exe"; Parameters: "doctor"; Description: "Run OpenAntry Doctor"; Flags: postinstall nowait skipifsilent

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
