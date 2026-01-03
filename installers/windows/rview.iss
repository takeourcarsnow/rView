; rView Inno Setup Script
; For creating Windows installer

#define MyAppName "rView"
#define MyAppVersion "2.0.0"
#define MyAppPublisher "rView Team"
#define MyAppURL "https://github.com/rview-app/rview"
#define MyAppExeName "rview.exe"

[Setup]
AppId={{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile=..\..\LICENSE
OutputDir=..\..\dist
OutputBaseFilename=rview-setup-{#MyAppVersion}
SetupIconFile=..\..\assets\rview.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "addtopath"; Description: "Add to PATH"; GroupDescription: "System integration:"; Flags: unchecked

[Files]
Source: "..\..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
; File associations
Root: HKCU; Subkey: "Software\Classes\.jpg\OpenWithProgids"; ValueType: string; ValueName: "rView.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.jpeg\OpenWithProgids"; ValueType: string; ValueName: "rView.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.png\OpenWithProgids"; ValueType: string; ValueName: "rView.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.gif\OpenWithProgids"; ValueType: string; ValueName: "rView.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.bmp\OpenWithProgids"; ValueType: string; ValueName: "rView.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.tiff\OpenWithProgids"; ValueType: string; ValueName: "rView.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.webp\OpenWithProgids"; ValueType: string; ValueName: "rView.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.cr2\OpenWithProgids"; ValueType: string; ValueName: "rView.RAW"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.nef\OpenWithProgids"; ValueType: string; ValueName: "rView.RAW"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.arw\OpenWithProgids"; ValueType: string; ValueName: "rView.RAW"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.dng\OpenWithProgids"; ValueType: string; ValueName: "rView.RAW"; ValueData: ""; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\.raf\OpenWithProgids"; ValueType: string; ValueName: "rView.RAW"; ValueData: ""; Flags: uninsdeletevalue

; ProgID for images
Root: HKCU; Subkey: "Software\Classes\rView.Image"; ValueType: string; ValueName: ""; ValueData: "Image File"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\rView.Image\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"
Root: HKCU; Subkey: "Software\Classes\rView.Image\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""

; ProgID for RAW files
Root: HKCU; Subkey: "Software\Classes\rView.RAW"; ValueType: string; ValueName: ""; ValueData: "RAW Image File"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\rView.RAW\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"
Root: HKCU; Subkey: "Software\Classes\rView.RAW\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
procedure CurStepChanged(CurStep: TSetupStep);
var
  Path: string;
begin
  if CurStep = ssPostInstall then
  begin
    if IsTaskSelected('addtopath') then
    begin
      RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path);
      if Pos(ExpandConstant('{app}'), Path) = 0 then
      begin
        Path := Path + ';' + ExpandConstant('{app}');
        RegWriteStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path);
      end;
    end;
  end;
end;
