param([switch]$SkipLayoutExport)
$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$root = Split-Path $PSScriptRoot -Parent
$resources = Join-Path $root 'src-tauri/resources'
$cache = Join-Path $root '.cache/bundle'
New-Item -ItemType Directory -Force -Path $cache,"$resources/runtime/cpu","$resources/runtime/vulkan","$resources/runtime/pdfium","$resources/licenses" | Out-Null
function Get-AssetHash([string]$LiteralPath, [string]$Algorithm = 'SHA256') {
  $stream = [IO.File]::OpenRead($LiteralPath)
  $sha = [Security.Cryptography.SHA256]::Create()
  try { return @{ Hash = ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-','') } }
  finally { $stream.Dispose(); $sha.Dispose() }
}
function Copy-Asset([string]$Source, [string]$Destination) {
  if ((Test-Path -LiteralPath $Destination) -and (Get-AssetHash -LiteralPath $Source).Hash -eq (Get-AssetHash -LiteralPath $Destination).Hash) { return }
  Copy-Item -LiteralPath $Source -Destination $Destination -Force
}
function Fetch-Checked($url, $path, $hash) {
  if (!(Test-Path -LiteralPath $path) -or (Get-AssetHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $hash) {
    Write-Host "Downloading $url"
    Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile "$path.part"
    if ((Get-AssetHash -LiteralPath "$path.part" -Algorithm SHA256).Hash.ToLowerInvariant() -ne $hash) { throw "Checksum mismatch: $url" }
    Move-Item -LiteralPath "$path.part" -Destination $path -Force
  }
}
$version = 'b10621'
$variants = @{
  cpu = '0e8b65e650e369f70f8307d890508886f171ef4fb00facccddd4a1b7ffdaca51'
  vulkan = '2672d85bf87c8280d94dee01eb6a86280046878f70a07d786a93637fa9081163'
}
foreach ($variant in @('cpu','vulkan')) {
  $name = "llama-$version-bin-win-$variant-x64.zip"
  Fetch-Checked "https://github.com/ggml-org/llama.cpp/releases/download/$version/$name" "$cache/$name" $variants[$variant]
  New-Item -ItemType Directory -Force -Path "$cache/$variant" | Out-Null
  tar -xf "$cache/$name" -C "$cache/$variant"
  if ($LASTEXITCODE -ne 0) { throw "llama.cpp extraction failed." }
  Get-ChildItem -LiteralPath "$cache/$variant" -Recurse -File | Where-Object { $_.Extension -eq '.dll' -or $_.Name -eq 'llama-server.exe' } | ForEach-Object { Copy-Asset $_.FullName (Join-Path "$resources/runtime/$variant" $_.Name) }
}
Fetch-Checked 'https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/8035/pdfium-win-x64.tgz' "$cache/pdfium.tgz" '61513d611ad200a383456140739be77d156f1e3a2eef22bd89f6c3bda79bdd41'
New-Item -ItemType Directory -Force -Path "$cache/pdfium" | Out-Null
tar -xzf "$cache/pdfium.tgz" -C "$cache/pdfium"
if ($LASTEXITCODE -ne 0) { throw 'PDFium extraction failed.' }
Copy-Asset "$cache/pdfium/bin/pdfium.dll" "$resources/runtime/pdfium/pdfium.dll"
Copy-Item -LiteralPath "$cache/pdfium/LICENSE" -Destination "$resources/licenses/PDFium.txt" -Force
if (Test-Path -LiteralPath "$cache/pdfium/licenses") {
  New-Item -ItemType Directory -Force -Path "$resources/licenses/pdfium-third-party" | Out-Null
  Get-ChildItem -LiteralPath "$cache/pdfium/licenses" -File | ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination "$resources/licenses/pdfium-third-party/" -Force }
}
Invoke-WebRequest -UseBasicParsing "https://raw.githubusercontent.com/ggml-org/llama.cpp/$version/LICENSE" -OutFile "$resources/licenses/llama.cpp-MIT.txt"
Invoke-WebRequest -UseBasicParsing 'https://raw.githubusercontent.com/PaddlePaddle/PaddleOCR/main/LICENSE' -OutFile "$resources/licenses/PaddleOCR-Apache-2.0.txt"


# Only graph metadata is packaged. Model weights are downloaded by the application.
if (!$SkipLayoutExport) { & (Join-Path $PSScriptRoot 'prepare-layout.ps1') }
Fetch-Checked 'https://github.com/microsoft/onnxruntime/releases/download/v1.24.4/onnxruntime-win-x64-1.24.4.zip' "$cache/onnxruntime.zip" 'd2319fddfb6ea4db99ccc4b60c85c517bcd855721f5daa6a06d40d7cb2ee2357'
New-Item -ItemType Directory -Force -Path "$cache/onnxruntime","$resources/runtime/onnxruntime" | Out-Null
tar -xf "$cache/onnxruntime.zip" -C "$cache/onnxruntime"
if ($LASTEXITCODE -ne 0) { throw 'ONNX Runtime extraction failed.' }
$ortRoot = "$cache/onnxruntime/onnxruntime-win-x64-1.24.4"
Get-ChildItem -LiteralPath "$ortRoot/lib" -File -Filter '*.dll' | ForEach-Object { Copy-Asset $_.FullName "$resources/runtime/onnxruntime/$($_.Name)" }
Copy-Item -LiteralPath "$ortRoot/LICENSE" -Destination "$resources/licenses/ONNX-Runtime-MIT.txt" -Force
Copy-Item -LiteralPath "$ortRoot/ThirdPartyNotices.txt" -Destination "$resources/licenses/ONNX-Runtime-ThirdPartyNotices.txt" -Force
Invoke-WebRequest -UseBasicParsing 'https://raw.githubusercontent.com/huggingface/transformers/v5.6.2/LICENSE' -OutFile "$resources/licenses/Transformers-Apache-2.0.txt"
# Remove obsolete generated copies from the earlier bundled-model distribution.
$obsolete = [IO.Path]::GetFullPath((Join-Path $resources 'models'))
$expected = [IO.Path]::GetFullPath((Join-Path $root 'src-tauri/resources/models'))
if ($obsolete -ne $expected -or !$obsolete.StartsWith([IO.Path]::GetFullPath($root) + [IO.Path]::DirectorySeparatorChar)) { throw 'Invalid generated model directory.' }
if (Test-Path -LiteralPath $obsolete) { Remove-Item -LiteralPath $obsolete -Recurse -Force }

# Include the app-local Microsoft C++ runtime for the app and llama sidecars.
$vswhere = [IO.Path]::Combine([Environment]::GetFolderPath('ProgramFilesX86'), 'Microsoft Visual Studio/Installer/vswhere.exe')
$vsRoot = & $vswhere -latest -property installationPath
if (!$vsRoot) { throw 'Visual Studio C++ Build Tools are required.' }
$redist = Get-ChildItem -LiteralPath "$vsRoot/VC/Redist/MSVC" -Directory |
  Where-Object Name -Match '^\d+\.\d+\.\d+$' | Sort-Object { [version]$_.Name } -Descending |
  ForEach-Object { Get-ChildItem -LiteralPath "$($_.FullName)/x64" -Directory -Filter 'Microsoft.VC*.CRT' -ErrorAction SilentlyContinue } |
  Select-Object -First 1
if (!$redist) { throw 'MSVC x64 redistributable CRT directory was not found.' }
New-Item -ItemType Directory -Force -Path "$resources/runtime/msvc" | Out-Null
foreach ($dll in (Get-ChildItem -LiteralPath $redist.FullName -File -Filter '*.dll')) {
  foreach ($variant in @('msvc','cpu','vulkan','onnxruntime')) { Copy-Asset $dll.FullName "$resources/runtime/$variant/$($dll.Name)" }
}
Copy-Item -LiteralPath "$cache/cpu/LICENSE-LLVM-OpenMP" -Destination "$resources/licenses/LLVM-OpenMP.txt" -Force
@'
Microsoft Visual C++ runtime libraries
Copyright (C) Microsoft Corporation. All rights reserved.
Redistributed app-locally from the licensed Visual Studio installation.
https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files

Microsoft Edge WebView2 Evergreen Runtime
Includes Microsoft's signed offline installer, installed only when absent.
Microsoft's WebView2 license terms apply.
https://developer.microsoft.com/en-us/microsoft-edge/webview2/
'@ | Set-Content -LiteralPath "$resources/licenses/Microsoft-runtimes.txt" -Encoding utf8
node (Join-Path $PSScriptRoot 'collect-notices.mjs')
if ($LASTEXITCODE -ne 0) { throw 'Third-party notice collection failed.' }

$records = Get-ChildItem -LiteralPath $resources -Recurse -File | Where-Object Name -ne 'bundle-manifest.json' | ForEach-Object {
  @{ path = $_.FullName.Substring($resources.Length + 1).Replace('\','/'); bytes = $_.Length; sha256 = (Get-AssetHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
}
@{ schemaVersion = 1; llama = $version; pdfium = 'chromium/8035'; model = 'PaddlePaddle/PaddleOCR-VL-1.6-GGUF'; files = @($records) } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath "$resources/bundle-manifest.json" -Encoding utf8
Write-Host 'Bundled resources ready. Run npm run installer.'
