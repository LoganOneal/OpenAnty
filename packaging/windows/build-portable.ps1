# Build a portable Open Anty folder for local testing / installer staging.
# Official downloadable GitHub Release binaries are produced by:
#   .github/workflows/release.yml  (only on v* tags — not every CI run)
# See docs/releasing.md

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $Root

Write-Host "==> Building release binaries..."
cargo build --release -p openantyd -p openanty-cli
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$Out = Join-Path $Root "packaging\out\OpenAnty-windows-x64"
if (Test-Path $Out) { Remove-Item -Recurse -Force $Out }
New-Item -ItemType Directory -Path $Out | Out-Null
New-Item -ItemType Directory -Path (Join-Path $Out "bin") | Out-Null

Copy-Item "$Root\target\release\openantyd.exe" (Join-Path $Out "bin\openantyd.exe")
Copy-Item "$Root\target\release\openanty.exe" (Join-Path $Out "bin\openanty.exe")
Copy-Item "$Root\README.md" $Out
Copy-Item "$Root\LICENSE" $Out
Copy-Item "$Root\RESPONSIBLE_USE.md" $Out
Copy-Item "$Root\docs\quick-start.md" (Join-Path $Out "QUICKSTART.md")
Copy-Item "$Root\packaging\INSTALL.ps1" (Join-Path $Out "INSTALL.ps1")
Copy-Item "$PSScriptRoot\OpenAnty.iss" $Out -ErrorAction SilentlyContinue

Write-Host "==> Portable bundle: $Out"
Write-Host "Run INSTALL.ps1 inside the folder for first-run wizard equivalent."
