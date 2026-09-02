#ifndef Root
  #error Build using npm run installer.
#endif
[Setup]
AppId={{ADE3206D-9414-480F-A2F9-7BCEBF939D7F}
AppName=Glyph OCR
AppVersion={#AppVersion}
AppPublisher=Glyph OCR
DefaultDirName={localappdata}\Programs\Glyph OCR
DefaultGroupName=Glyph OCR
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
SetupArchitecture=x64
MinVersion=10.0
OutputDir={#Root}\src-tauri\target\release\bundle\inno
OutputBaseFilename=Glyph-OCR_{#AppVersion}_x64-setup
SetupIconFile={#Root}\src-tauri\icons\icon.ico
UninstallDisplayIcon={app}\glyph-ocr.exe
Compression=lzma2/normal
SolidCompression=yes
DiskSpanning=no
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
UninstallDisplayName=Glyph OCR
VersionInfoProductName=Glyph OCR
VersionInfoDescription=Glyph OCR offline installer

[Languages]
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#Root}\src-tauri\target\release\glyph-ocr.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\src-tauri\resources\runtime\msvc\*.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\src-tauri\resources\*"; DestDir: "{app}\resources"; Flags: ignoreversion recursesubdirs createallsubdirs; Excludes: "*.gguf,*.safetensors,*.part"
Source: "{#WebViewInstaller}"; DestName: "WebView2Setup.exe"; Flags: dontcopy noencryption

[Icons]
Name: "{autoprograms}\Glyph OCR"; Filename: "{app}\glyph-ocr.exe"
Name: "{autodesktop}\Glyph OCR"; Filename: "{app}\glyph-ocr.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\glyph-ocr.exe"; Description: "{cm:LaunchProgram,Glyph OCR}"; Flags: nowait postinstall skipifsilent

[Code]
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
  Result := '';
  if not HasWebView2 then begin
    ExtractTemporaryFile('WebView2Setup.exe');
    if not Exec(ExpandConstant('{tmp}\WebView2Setup.exe'), '/silent /install', '', SW_HIDE, ewWaitUntilTerminated, ExitCode) then
      Result := 'Could not start the included Microsoft WebView2 installer.'
    else if not HasWebView2 then
      Result := 'Microsoft WebView2 installation failed (code ' + IntToStr(ExitCode) + '). Restart Windows and run this installer again.';
  end;
end;
