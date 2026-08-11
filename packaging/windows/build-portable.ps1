# Build a portable GhostFox folder suitable for zip distribution and installer staging.
# Easy installer requirement (G11): this script is the Windows packaging entry point.

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $Root

Write-Host "==> Building release binaries..."
cargo build --release -p ghostfoxd -p ghostfox-cli
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$Out = Join-Path $Root "packaging\out\GhostFox-windows-x64"
if (Test-Path $Out) { Remove-Item -Recurse -Force $Out }
New-Item -ItemType Directory -Path $Out | Out-Null
New-Item -ItemType Directory -Path (Join-Path $Out "bin") | Out-Null

Copy-Item "$Root\target\release\ghostfoxd.exe" (Join-Path $Out "bin\ghostfoxd.exe")
Copy-Item "$Root\target\release\ghostfox.exe" (Join-Path $Out "bin\ghostfox.exe")
Copy-Item "$Root\README.md" $Out
Copy-Item "$Root\LICENSE" $Out
Copy-Item "$Root\RESPONSIBLE_USE.md" $Out
Copy-Item "$Root\docs\quick-start.md" (Join-Path $Out "QUICKSTART.md")

$InstallPs1 = @"
# GhostFox portable first-run
`$ErrorActionPreference = 'Stop'
`$Bin = Join-Path `$PSScriptRoot 'bin'
`$env:PATH = "`$Bin;`$env:PATH"
Write-Host 'Initializing GhostFox...'
& "`$Bin\ghostfox.exe" init
Write-Host ''
Write-Host 'Doctor:'
& "`$Bin\ghostfox.exe" doctor
Write-Host ''
Write-Host 'MCP config snippet:'
& "`$Bin\ghostfox.exe" mcp-config
Write-Host ''
Write-Host 'Done. Add bin\ to PATH or use full path to ghostfox.exe / ghostfoxd.exe'
"@
Set-Content -Path (Join-Path $Out "INSTALL.ps1") -Value $InstallPs1 -Encoding UTF8

# Stage Inno Setup script path note
Copy-Item "$PSScriptRoot\GhostFox.iss" $Out -ErrorAction SilentlyContinue

Write-Host "==> Portable bundle: $Out"
Write-Host "Run INSTALL.ps1 inside the folder for first-run wizard equivalent."
