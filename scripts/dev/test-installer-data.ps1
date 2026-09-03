param()
$ErrorActionPreference = 'Stop'
$root = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$testRoot = [IO.Path]::GetFullPath((Join-Path $root '.cache/installer-data-tests'))
if (!$testRoot.StartsWith([IO.Path]::GetFullPath($root) + [IO.Path]::DirectorySeparatorChar)) { throw 'Invalid test root' }
$runRoot = Join-Path $testRoot ([guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $runRoot | Out-Null
$compiler = Join-Path $root '.cache/inno-setup/ISCC.exe'
if (!(Test-Path -LiteralPath $compiler)) { throw 'Build the installer before running its data tests.' }
node (Join-Path $root 'scripts/build/installer-locales.mjs')
if ($LASTEXITCODE -ne 0) { throw 'Locale generation failed' }
$production = [IO.File]::ReadAllText((Join-Path $root 'installer/data.iss'))
$encoding = [Text.UTF8Encoding]::new($false)
function Write-Text($Path, $Value) { [IO.File]::WriteAllText($Path,$Value,$encoding) }
function Assert-Exists($Path) { if (!(Test-Path -LiteralPath $Path)) { throw "Expected retained data: $Path" } }
function Assert-Missing($Path) { if (Test-Path -LiteralPath $Path) { throw "Unexpected retained data: $Path" } }
function Run-TestProcess($File, $Arguments) {
  $process = Start-Process -FilePath $File -ArgumentList $Arguments -WindowStyle Hidden -Wait -PassThru
  return $process.ExitCode
}
foreach ($scenario in @('fresh-retain','remove-data')) {
  $case = Join-Path $runRoot $scenario
  $data = Join-Path $case 'data'
  $app = Join-Path $case 'app'
  New-Item -ItemType Directory -Force -Path "$data/models","$data/workspaces","$data/logs","$data/updates" | Out-Null
  Write-Text "$data/.fastfileocr-data" 'FastFileOCR data v1'
  Write-Text "$data/settings.json" '{"language":"ko","schemaVersion":1}'
  Write-Text "$data/models/retained.part" 'model data'
  Write-Text "$data/workspaces/document.txt" 'document data'
  Write-Text "$data/unmanaged.txt" 'unrelated data'
  Write-Text "$case/outside.txt" 'outside data'
  Write-Text "$case/payload.txt" 'installer test'
  $code = $production
  if ($scenario -eq 'fresh-retain') {
    # Simulate choosing Fresh and confirming it. All data operations are production code.
    $code = $code.Replace('ModePage.SelectedValueIndex := 0;', 'ModePage.SelectedValueIndex := 1;')
    $code = $code.Replace("Result := SuppressibleMsgBox(CustomMessage('installResetConfirm'), mbConfirmation, MB_YESNO or MB_DEFBUTTON2, IDNO) = IDYES;", 'Result := True;')
  } else {
    # Simulate both deletion choices; no real user data is used by this test harness.
    $code = $code.Replace('RemoveUserData := False; RemoveDocuments := False;', 'RemoveUserData := True; RemoveDocuments := True;')
  }
  Write-Text "$case/data.iss" $code
  $source = @"
[Setup]
AppId=FastFileOCR-Data-Test-$([guid]::NewGuid().ToString('N'))
AppName=FastFileOCR data lifecycle test
AppVersion=0.0.0
DefaultDirName=$app
PrivilegesRequired=lowest
OutputDir=$case
OutputBaseFilename=test-setup
Compression=none
DisableWelcomePage=yes
DisableDirPage=yes
DisableProgramGroupPage=yes
[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"
#include "$root\.cache\installer\messages.iss"
[Files]
Source: "$case\payload.txt"; DestDir: "{app}"; Flags: ignoreversion
[INI]
Filename: "{app}\data-location.ini"; Section: "Data"; Key: "Directory"; String: "{code:DataDirectory}"; Flags: uninsdeletesection
[Code]
#include "$case\data.iss"
function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := ValidateDataDirectory;
end;
"@
  Write-Text "$case/test.iss" $source
  & $compiler /Qp "$case/test.iss"
  if ($LASTEXITCODE -ne 0) { throw "Harness compilation failed: $scenario" }
  $exit = Run-TestProcess "$case/test-setup.exe" @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/SP-',('/DATADIR="' + $data + '"'),('/LOG="' + $case + '\install.log"'))
  if ($exit -ne 0) { throw "Harness install failed ($exit): $scenario" }
  if ($scenario -eq 'fresh-retain') {
    Assert-Missing "$data/settings.json"
    Assert-Exists "$data/.fresh-settings"
    $backups = @(Get-ChildItem -LiteralPath "$data/settings-backups" -File)
    if ($backups.Count -ne 1 -or [IO.File]::ReadAllText($backups[0].FullName) -ne '{"language":"ko","schemaVersion":1}') { throw 'Fresh-install backup mismatch' }
    Assert-Exists "$data/models/retained.part"
    Assert-Exists "$data/workspaces/document.txt"
  }
  $exit = Run-TestProcess "$app/unins000.exe" @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',('/LOG="' + $case + '\uninstall.log"'))
  if ($exit -ne 0) { throw "Harness uninstall failed ($exit): $scenario" }
  if ($scenario -eq 'fresh-retain') {
    Assert-Exists "$data/models/retained.part"
    Assert-Exists "$data/workspaces/document.txt"
    Assert-Exists "$data/settings-backups"
  } else {
    foreach ($child in @('settings.json','models','workspaces','logs','updates')) { Assert-Missing "$data/$child" }
  }
  Assert-Exists "$data/unmanaged.txt"
  Assert-Exists "$data/.fastfileocr-data"
  Assert-Exists "$case/outside.txt"
  Write-Host "Passed: $scenario"
  # Selecting a populated parent creates a dedicated child, preserving unrelated files.
  $unowned = Join-Path $case 'unowned'
  New-Item -ItemType Directory -Path $unowned | Out-Null
  Write-Text "$unowned/personal.txt" 'must remain'
  $arguments = @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/SP-',('/DATADIR="' + $unowned + '"'),('/LOG="' + $case + '\parent.log"'))
  $exit = Run-TestProcess "$case/test-setup.exe" $arguments
  if ($exit -ne 0) { throw 'Populated parent selection was rejected' }
  Assert-Exists "$unowned/personal.txt"
  Assert-Missing "$unowned/.fastfileocr-data"
  Assert-Exists "$unowned/FastFileOCR/.fastfileocr-data"
  $exit = Run-TestProcess "$app/unins000.exe" @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART')
  if ($exit -ne 0) { throw 'Parent-folder fixture uninstall failed' }
  Assert-Exists "$unowned/personal.txt"
  # If that dedicated child is already unrelated data, refuse to claim it.
  $blocked = Join-Path $case 'blocked'
  New-Item -ItemType Directory -Path "$blocked/FastFileOCR" -Force | Out-Null
  Write-Text "$blocked/FastFileOCR/personal.txt" 'must remain'
  $exit = Run-TestProcess "$case/test-setup.exe" @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/SP-',('/DATADIR="' + $blocked + '"'),('/LOG="' + $case + '\blocked.log"'))
  if ($exit -eq 0) { throw 'Unowned dedicated folder was accepted' }
  Assert-Exists "$blocked/FastFileOCR/personal.txt"
  Assert-Missing "$blocked/FastFileOCR/.fastfileocr-data"

}
Write-Host 'Installer fresh-start, retention, deletion, parent-folder and ownership tests passed.'
