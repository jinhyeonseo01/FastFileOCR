#ifndef Root
  #error Build using npm run installer.
#endif
[Setup]
AppId={{ADE3206D-9414-480F-A2F9-7BCEBF939D7F}
AppName=FastFileOCR
AppVersion={#AppVersion}
AppPublisher=FastFileOCR
DefaultDirName={localappdata}\Programs\FastFileOCR
DefaultGroupName=FastFileOCR
DisableProgramGroupPage=yes
DisableDirPage=no
ShowLanguageDialog=yes
LanguageDetectionMethod=none
UsePreviousLanguage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
SetupArchitecture=x64
MinVersion=10.0
OutputDir={#Root}\src-tauri\target\release\bundle\inno
OutputBaseFilename=FastFileOCR_{#AppVersion}_x64-setup
SetupIconFile={#Root}\src-tauri\icons\icon.ico
UninstallDisplayIcon={app}\glyph-ocr.exe
Compression=lzma2/normal
SolidCompression=yes
DiskSpanning=no
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
UninstallDisplayName=FastFileOCR
VersionInfoProductName=FastFileOCR
VersionInfoDescription=FastFileOCR offline installer

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

#include Root + "\.cache\installer\messages.iss"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#Root}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\NOTICE.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\src-tauri\target\release\glyph-ocr.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\src-tauri\resources\runtime\msvc\*.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\src-tauri\resources\*"; DestDir: "{app}\resources"; Flags: ignoreversion recursesubdirs createallsubdirs; Excludes: "*.gguf,*.safetensors,*.part"
Source: "{#WebViewInstaller}"; DestName: "WebView2Setup.exe"; Flags: dontcopy noencryption

[Registry]
Root: HKCU; Subkey: "Software\FastFileOCR"; ValueType: string; ValueName: "DataDir"; ValueData: "{code:DataDirectory}"
Root: HKCU; Subkey: "Software\FastFileOCR"; ValueType: string; ValueName: "Language"; ValueData: "{code:InitialLanguage}"

[INI]
Filename: "{app}\data-location.ini"; Section: "Data"; Key: "Directory"; String: "{code:DataDirectory}"; Flags: uninsdeletesection

[Icons]
Name: "{autoprograms}\FastFileOCR"; Filename: "{app}\glyph-ocr.exe"
Name: "{autodesktop}\FastFileOCR"; Filename: "{app}\glyph-ocr.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\glyph-ocr.exe"; Description: "{cm:LaunchProgram,FastFileOCR}"; Flags: nowait postinstall skipifsilent

[Code]
#include Root + "\installer\data.iss"
function HasWebView2: Boolean;
var
  Version: String;
  Key: String;
begin
  Key := 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}';
  Result := (RegQueryStringValue(HKLM32, Key, 'pv', Version) and (Version <> '') and (Version <> '0.0.0.0'));
  if not Result then
    Result := (RegQueryStringValue(HKCU32, Key, 'pv', Version) and (Version <> '') and (Version <> '0.0.0.0'));
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ExitCode: Integer;
begin
  Result := ValidateDataDirectory;
  if Result <> '' then Exit;
  if not HasWebView2 then begin
    ExtractTemporaryFile('WebView2Setup.exe');
    if not Exec(ExpandConstant('{tmp}\WebView2Setup.exe'), '/silent /install', '', SW_HIDE, ewWaitUntilTerminated, ExitCode) then
      Result := CustomMessage('webviewStartError')
    else if not HasWebView2 then
      Result := CustomMessage('webviewInstallError');
  end;
end;
