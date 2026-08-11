# Build a portable OpenAntry folder suitable for zip distribution and installer staging.
# Easy installer requirement (G11): this script is the Windows packaging entry point.

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $Root

Write-Host "==> Building release binaries..."
cargo build --release -p openantryd -p openantry-cli
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$Out = Join-Path $Root "packaging\out\OpenAntry-windows-x64"
if (Test-Path $Out) { Remove-Item -Recurse -Force $Out }
New-Item -ItemType Directory -Path $Out | Out-Null
New-Item -ItemType Directory -Path (Join-Path $Out "bin") | Out-Null

Copy-Item "$Root\target\release\openantryd.exe" (Join-Path $Out "bin\openantryd.exe")
Copy-Item "$Root\target\release\openantry.exe" (Join-Path $Out "bin\openantry.exe")
Copy-Item "$Root\README.md" $Out
Copy-Item "$Root\LICENSE" $Out
Copy-Item "$Root\RESPONSIBLE_USE.md" $Out
Copy-Item "$Root\docs\quick-start.md" (Join-Path $Out "QUICKSTART.md")

$InstallPs1 = @"
# OpenAntry portable first-run
`$ErrorActionPreference = 'Stop'
`$Bin = Join-Path `$PSScriptRoot 'bin'
`$env:PATH = "`$Bin;`$env:PATH"
Write-Host 'Initializing OpenAntry...'
& "`$Bin\openantry.exe" init
Write-Host ''
Write-Host 'Doctor:'
& "`$Bin\openantry.exe" doctor
Write-Host ''
Write-Host 'MCP config snippet:'
& "`$Bin\openantry.exe" mcp-config
Write-Host ''
Write-Host 'Done. Add bin\ to PATH or use full path to openantry.exe / openantryd.exe'
"@
Set-Content -Path (Join-Path $Out "INSTALL.ps1") -Value $InstallPs1 -Encoding UTF8

# Stage Inno Setup script path note
Copy-Item "$PSScriptRoot\OpenAntry.iss" $Out -ErrorAction SilentlyContinue

Write-Host "==> Portable bundle: $Out"
Write-Host "Run INSTALL.ps1 inside the folder for first-run wizard equivalent."
