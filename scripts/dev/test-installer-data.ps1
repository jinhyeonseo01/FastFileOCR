param()
$ErrorActionPreference = 'Stop'
$root = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
Set-Location -LiteralPath $root
cargo test --locked --manifest-path installer/helper/Cargo.toml --target-dir .cache/installer-helper-target
if ($LASTEXITCODE -ne 0) { throw 'Installer helper tests failed.' }
$sourceDir = Join-Path $root 'src-tauri/target/release/nsis/x64'
$compiler = Join-Path $env:LOCALAPPDATA 'tauri/NSIS/makensis.exe'
if (!(Test-Path -LiteralPath "$sourceDir/installer.nsi") -or !(Test-Path -LiteralPath $compiler)) {
  throw 'Build the NSIS installer before running installer tests.'
}
$id = [guid]::NewGuid().ToString('N')
$fixtureBase = [IO.Path]::GetFullPath((Join-Path $root '.cache/nsis-tests'))
$runRoot = [IO.Path]::GetFullPath((Join-Path $fixtureBase $id))
if (!$runRoot.StartsWith($fixtureBase + [IO.Path]::DirectorySeparatorChar)) { throw 'Invalid fixture root.' }
New-Item -ItemType Directory -Force -Path $runRoot | Out-Null
$regRoot = "Software\FastFileOCR-InstallerTests\$id"
$product = "FastFileOCR-Test-$id"
$helper = Join-Path $root 'src-tauri/resources/installer/fastfileocr-setup-helper.exe'
$encoding = [Text.UTF8Encoding]::new($false)
function Write-Text($Path, $Value) { [IO.File]::WriteAllText($Path, $Value, $encoding) }
function Assert-Exists($Path) { if (!(Test-Path -LiteralPath $Path)) { throw "Expected retained data: $Path" } }
function Assert-Missing($Path) { if (Test-Path -LiteralPath $Path) { throw "Unexpected retained data: $Path" } }
function Run-Setup($App, $Data, $Extra = @()) {
  $arguments = @('/S','/NS',('/DATADIR="' + $Data + '"')) + $Extra + @('/D=' + $App)
  $process = Start-Process -FilePath "$runRoot/setup.exe" -ArgumentList $arguments -WindowStyle Hidden -Wait -PassThru
  return $process.ExitCode
}
function Run-Uninstall($App, $Extra = @()) {
  $process = Start-Process -FilePath "$App/uninstall.exe" -ArgumentList (@('/S') + $Extra + @('_?=' + $App)) -WindowStyle Hidden -Wait -PassThru
  return $process.ExitCode
}
function Make-Data($Name) {
  $data = Join-Path $runRoot "$Name/data"
  New-Item -ItemType Directory -Force -Path "$data/models","$data/workspaces" | Out-Null
  Write-Text "$data/.fastfileocr-data" 'FastFileOCR data v1'
  Write-Text "$data/settings.json" '{"language":"ko","schemaVersion":1}'
  Write-Text "$data/models/keep.part" 'model'
  Write-Text "$data/workspaces/keep.txt" 'document'
  Write-Text "$data/personal.txt" 'unmanaged'
  return $data
}
# Use production NSIS pages and hooks, replacing only fixture identity and payload.
$source = [IO.File]::ReadAllText("$sourceDir/installer.nsi")
$defines = @{
  MANUFACTURER = "FastFileOCR-InstallerTests\$id"
  PRODUCTNAME = $product
  MAINBINARYNAME = 'fastfileocr-setup-helper'
  MAINBINARYSRCPATH = $helper
  BUNDLEID = "test.fastfileocr.$id"
  OUTFILE = "$runRoot/setup.exe"
  INSTALLWEBVIEW2MODE = 'skip'
  ESTIMATEDSIZE = '2048'
}
foreach ($key in $defines.Keys) {
  $value = $defines[$key]
  $source = [regex]::Replace($source, "(?m)^!define $key `"[^`"]*`"", [System.Text.RegularExpressions.MatchEvaluator]{ param($m) "!define $key `"$value`"" })
}
$source = "!define FFO_REGKEY `"$regRoot\Data`"`n" + $source
$source = $source.Replace('!include "utils.nsh"', "!include `"$sourceDir\utils.nsh`"").Replace('!include "FileAssociation.nsh"', "!include `"$sourceDir\FileAssociation.nsh`"")
$source = [regex]::Replace($source, '(?m)^    File /a[^\r\n]*', [System.Text.RegularExpressions.MatchEvaluator]{
  param($m)
  if ($m.Value.Contains('fastfileocr-setup-helper.exe')) { return $m.Value }
  return ''
})
$source = $source.Replace('SetCompressor /SOLID "lzma"', 'SetCompress off')
Write-Text "$runRoot/test.nsi" $source
& $compiler -INPUTCHARSET UTF8 -V2 "$runRoot/test.nsi" *> "$runRoot/compile.log"
if ($LASTEXITCODE -ne 0) { Get-Content "$runRoot/compile.log"; throw 'NSIS fixture compilation failed.' }
try {
  $data = Make-Data ('fresh-' + [char]0xD55C + [char]0xAE00 + ' ' + [char]0x65E5)
  $app = Join-Path $runRoot 'fresh/app folder'
  if ((Run-Setup $app $data @('/FRESH=1','/LANGUAGE=1041')) -ne 0) { throw 'Fresh install failed.' }
  Assert-Missing "$data/settings.json"
  Assert-Exists "$data/.fresh-settings"
  $backups = @(Get-ChildItem -LiteralPath "$data/settings-backups" -File)
  if ($backups.Count -ne 1 -or [IO.File]::ReadAllText($backups[0].FullName) -ne '{"language":"ko","schemaVersion":1}') { throw 'Settings backup mismatch.' }
  $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("$regRoot\Data")
  try {
    if ($key.GetValue('Language') -ne 'ja' -or $key.GetValue('DataDir') -ne $data) { throw 'Initial language/data directory mismatch.' }
  } finally { $key.Dispose() }
  Write-Text "$data/settings.json" 'saved settings'
  if ((Run-Setup $app $data @('/LANGUAGE=1042')) -ne 0) { throw 'Reinstall failed.' }
  if ([IO.File]::ReadAllText("$data/settings.json") -ne 'saved settings') { throw 'Reinstall changed settings.' }
  if ((Run-Uninstall $app) -ne 0) { throw 'Default uninstall failed.' }
  Assert-Exists "$data/models/keep.part"
  Assert-Exists "$data/workspaces/keep.txt"
  Write-Host 'Passed: fresh backup, language, reinstall and default retention'
  foreach ($case in @('data-only','documents-only','both','update-retains')) {
    $data = Make-Data $case
    $app = Join-Path $runRoot "$case/app"
    if ((Run-Setup $app $data @('/LANGUAGE=1033')) -ne 0) { throw "Install failed: $case" }
    $flags = @()
    if ($case -in @('data-only','both','update-retains')) { $flags += '/REMOVEUSERDATA=1' }
    if ($case -in @('documents-only','both','update-retains')) { $flags += '/REMOVEDOCUMENTS=1' }
    if ($case -eq 'update-retains') { $flags += '/UPDATE' }
    if ((Run-Uninstall $app $flags) -ne 0) { throw "Uninstall failed: $case" }
    if ($case -in @('data-only','both')) { Assert-Missing "$data/models"; Assert-Missing "$data/settings.json" }
    else { Assert-Exists "$data/models/keep.part"; Assert-Exists "$data/settings.json" }
    if ($case -in @('documents-only','both')) { Assert-Missing "$data/workspaces" }
    else { Assert-Exists "$data/workspaces/keep.txt" }
    Assert-Exists "$data/personal.txt"
    Assert-Exists "$data/.fastfileocr-data"
    Write-Host "Passed: $case"
  }
  $parent = Join-Path $runRoot 'parent'
  New-Item -ItemType Directory -Path $parent | Out-Null
  Write-Text "$parent/personal.txt" 'keep'
  $app = Join-Path $runRoot 'parent-app'
  if ((Run-Setup $app $parent) -ne 0) { throw 'Populated parent install failed.' }
  Assert-Exists "$parent/FastFileOCR/.fastfileocr-data"
  Assert-Missing "$parent/.fastfileocr-data"
  Assert-Exists "$parent/personal.txt"
  if ((Run-Uninstall $app) -ne 0) { throw 'Parent fixture uninstall failed.' }
  $blocked = Join-Path $runRoot 'blocked'
  New-Item -ItemType Directory -Path "$blocked/FastFileOCR" -Force | Out-Null
  Write-Text "$blocked/FastFileOCR/personal.txt" 'keep'
  if ((Run-Setup (Join-Path $runRoot 'blocked-app') $blocked) -eq 0) { throw 'Unowned folder was accepted.' }
  Assert-Missing "$blocked/FastFileOCR/.fastfileocr-data"
  Assert-Exists "$blocked/FastFileOCR/personal.txt"
  Write-Host 'Passed: populated parent resolution and unsafe-folder rejection'
} finally {
  [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree($regRoot, $false)
  [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree("Software\Microsoft\Windows\CurrentVersion\Uninstall\$product", $false)
}
Write-Host "NSIS lifecycle checks passed. Fixtures: $runRoot"
