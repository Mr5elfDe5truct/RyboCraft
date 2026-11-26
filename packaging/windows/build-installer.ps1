# Build RyboCraft's installer: dist\RyboCraft-Setup-<version>.exe
#   powershell -ExecutionPolicy Bypass -File packaging\windows\build-installer.ps1
# Needs only Rust (MSVC). -SkipBuild packs the programs already built.
param([switch] $SkipBuild)
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
Set-Location $Root
$Version = ([regex]'(?m)^version = "([^"]+)"').Match((Get-Content Cargo.toml -Raw)).Groups[1].Value
# Static CRT, so nothing else needs installing.
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = '-C target-feature=+crt-static'
$Bin = 'target\x86_64-pc-windows-msvc\release'
if (-not $SkipBuild) {
  cargo build --release --target x86_64-pc-windows-msvc -p rybocraft -p rybocraft-cli
  if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
}
$Stage = Join-Path $Root 'target\installer-stage'
New-Item -ItemType Directory -Force $Stage, dist | Out-Null
Copy-Item "$Bin\rybocraft.exe", "$Bin\rybocraft-cli.exe", LICENSE-MIT, LICENSE-APACHE, NOTICE, THIRD-PARTY-LICENSES.txt, ATTRIBUTION.md $Stage -Force
$env:RYBOCRAFT_PAYLOAD = $Stage
cargo build --release --target x86_64-pc-windows-msvc -p rybocraft-setup
if ($LASTEXITCODE -ne 0) { throw 'installer build failed' }
Copy-Item "$Bin\rybocraft-setup.exe" "dist\RyboCraft-Setup-$Version.exe" -Force
Get-Item "dist\RyboCraft-Setup-$Version.exe"
