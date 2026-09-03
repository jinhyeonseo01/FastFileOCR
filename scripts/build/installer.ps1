param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$root = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
Set-Location -LiteralPath $root
function Hash-File([string]$path) {
  $stream = [IO.File]::OpenRead($path)
  $sha = [Security.Cryptography.SHA256]::Create()
  try { return ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-','').ToLowerInvariant() }
  finally { $stream.Dispose(); $sha.Dispose() }
}
node scripts/build/check-resources.mjs
if ($LASTEXITCODE -ne 0) { throw 'Bundled resources are not ready. Run npm run resources:prepare.' }
if ($SkipBuild) {
  node scripts/build/prepare-nsis.mjs
  if ($LASTEXITCODE -ne 0) { throw 'NSIS preparation failed.' }
  & npx.cmd tauri bundle --bundles nsis
} else {
  & npx.cmd tauri build --bundles nsis
}
if ($LASTEXITCODE -ne 0) { throw 'Tauri NSIS build failed.' }
$version = (Get-Content -LiteralPath "$root/package.json" -Raw | ConvertFrom-Json).version
$output = "$root/src-tauri/target/release/bundle/nsis"
$installer = "$output/FastFileOCR_$($version)_x64-setup.exe"
if (!(Test-Path -LiteralPath $installer)) { throw 'NSIS installer output missing.' }
"$((Hash-File $installer))  $([IO.Path]::GetFileName($installer))" | Set-Content -LiteralPath "$installer.sha256" -Encoding ascii
@{
  version = $version
  installerType = 'nsis'
  installer = @{ file = [IO.Path]::GetFileName($installer); bytes = (Get-Item -LiteralPath $installer).Length; sha256 = (Hash-File $installer) }
  resourcesManifestSha256 = (Hash-File "$root/src-tauri/resources/bundle-manifest.json")
  setupHelperSha256 = (Hash-File "$root/src-tauri/resources/installer/fastfileocr-setup-helper.exe")
  tauriCli = (Get-Content -LiteralPath "$root/node_modules/@tauri-apps/cli/package.json" -Raw | ConvertFrom-Json).version
  templateSha256 = (Get-Content -LiteralPath "$root/installer/nsis/upstream.json" -Raw | ConvertFrom-Json).sha256
} | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath "$output/build-info.json" -Encoding utf8
Write-Host "NSIS installer ready: $installer"
