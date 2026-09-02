param([string]$Python = 'python')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $root
$venv = Join-Path $root '.cache/layout-venv/Scripts/python.exe'
if (!(Test-Path -LiteralPath $venv)) {
  & $Python -m venv .cache/layout-venv
  if ($LASTEXITCODE -ne 0) { throw 'Python 3.12 is required for build-time layout graph export.' }
}
& $venv -m pip install torch==2.11.0 torchvision==0.26.0 --index-url https://download.pytorch.org/whl/cpu
if ($LASTEXITCODE -ne 0) { throw 'Build-time PyTorch installation failed.' }
& $venv -m pip install -r scripts/layout-requirements.txt
if ($LASTEXITCODE -ne 0) { throw 'Layout exporter dependency installation failed.' }
& $venv scripts/export-layout.py
if ($LASTEXITCODE -ne 0) { throw 'Layout graph export or numerical validation failed.' }
