$ErrorActionPreference = "Stop"
Set-Location (Split-Path -Parent $PSScriptRoot)
cargo test --workspace
if ($LASTEXITCODE -ne 0) { throw "Rust tests failed" }
powershell.exe -ExecutionPolicy Bypass -File ".\tools\verify_phase0733.ps1"
if ($LASTEXITCODE -ne 0) { throw "Legacy regression failed" }
Write-Host "PASS Phase 07.5.1 plant toxin type inheritance and legacy regression"
