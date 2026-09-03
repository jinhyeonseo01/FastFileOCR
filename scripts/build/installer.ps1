param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$root = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
Set-Location -LiteralPath $root
$cache = Join-Path $root '.cache'
New-Item -ItemType Directory -Force -Path "$cache/bundle" | Out-Null
function Hash-File([string]$path) {
  $stream = [IO.File]::OpenRead($path)
  $sha = [Security.Cryptography.SHA256]::Create()
  try { return ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-','').ToLowerInvariant() }
  finally { $stream.Dispose(); $sha.Dispose() }
}
node scripts/build/installer-locales.mjs
if ($LASTEXITCODE -ne 0) { throw 'Installer locale generation failed.' }
node scripts/build/check-resources.mjs
if ($LASTEXITCODE -ne 0) { throw 'Bundled resources are not ready. Run npm run resources:prepare.' }
if (!$SkipBuild) {
  & npx.cmd tauri build --no-bundle
  if ($LASTEXITCODE -ne 0) { throw 'Application build failed.' }
}
$compiler = "$cache/inno-setup/ISCC.exe"
if (!(Test-Path -LiteralPath $compiler)) {
  $setup = "$cache/innosetup-7.1.0-x64.exe"
  $hash = '0362a383ed217d4c4239b5933866dd96d3eb2102737da92f80f6057a4b40df2f'
  if (!(Test-Path -LiteralPath $setup) -or (Hash-File $setup) -ne $hash) {
    Invoke-WebRequest -UseBasicParsing 'https://github.com/jrsoftware/issrc/releases/download/is-7_1_0/innosetup-7.1.0-x64.exe' -OutFile "$setup.part"
    if ((Hash-File "$setup.part") -ne $hash) { throw 'Inno Setup checksum mismatch.' }
    Move-Item -LiteralPath "$setup.part" -Destination $setup -Force
  }
  $process = Start-Process -FilePath $setup -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/SP-','/PORTABLE=1',('/DIR="' + "$cache/inno-setup" + '"')) -WindowStyle Hidden -Wait -PassThru
  if ($process.ExitCode -ne 0) { throw "Inno Setup preparation failed: $($process.ExitCode)" }
}
$webview = "$cache/bundle/MicrosoftEdgeWebView2RuntimeInstallerX64.exe"
if (!(Test-Path -LiteralPath $webview)) {
  $existing = Get-ChildItem -LiteralPath "$env:LOCALAPPDATA/tauri" -Recurse -File -Filter 'MicrosoftEdgeWebView2RuntimeInstallerX64.exe' -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($existing) { Copy-Item -LiteralPath $existing.FullName -Destination $webview }
  else { Invoke-WebRequest -UseBasicParsing 'https://go.microsoft.com/fwlink/?linkid=2124701' -OutFile $webview }
}
Import-Module (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Security/Microsoft.PowerShell.Security.psd1') -Force
$signature = Microsoft.PowerShell.Security\Get-AuthenticodeSignature -LiteralPath $webview
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') {
  throw 'The WebView2 installer must have a valid Microsoft signature. Remove the cached installer and retry.'
}
$version = (Get-Content -LiteralPath "$root/package.json" -Raw | ConvertFrom-Json).version
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Installer version must be major.minor.patch.' }
$output = "$root/src-tauri/target/release/bundle/inno"
& $compiler "/DRoot=$root" "/DAppVersion=$version" "/DWebViewInstaller=$webview" "$root/installer/fastfileocr.iss"
if ($LASTEXITCODE -ne 0) { throw 'Inno Setup compilation failed.' }
$installer = "$output/FastFileOCR_$($version)_x64-setup.exe"
if (!(Test-Path -LiteralPath $installer)) { throw 'Installer output missing.' }
"$((Hash-File $installer))  $([IO.Path]::GetFileName($installer))" | Set-Content -LiteralPath "$installer.sha256" -Encoding ascii
@{
  version = $version
  installer = @{ file = [IO.Path]::GetFileName($installer); bytes = (Get-Item -LiteralPath $installer).Length; sha256 = (Hash-File $installer) }
  webview2 = @{ version = (Get-Item -LiteralPath $webview).VersionInfo.FileVersion; sha256 = (Hash-File $webview) }
  resourcesManifestSha256 = (Hash-File "$root/src-tauri/resources/bundle-manifest.json")
  innoSetup = '7.1.0'
} | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath "$output/build-info.json" -Encoding utf8
Write-Host "Installer ready: $installer"
