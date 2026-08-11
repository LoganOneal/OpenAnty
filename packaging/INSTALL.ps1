# Open Anty portable first-run helper (Windows)
$ErrorActionPreference = "Stop"
$Bin = Join-Path $PSScriptRoot "bin"
$env:PATH = "$Bin;$env:PATH"
Write-Host "Initializing Open Anty..."
& "$Bin\openanty.exe" init
Write-Host ""
Write-Host "Doctor:"
& "$Bin\openanty.exe" doctor
Write-Host ""
Write-Host "MCP config:"
& "$Bin\openanty.exe" mcp-config
Write-Host ""
Write-Host "Done. Add bin\ to PATH or use full path to openanty.exe / openantyd.exe"
