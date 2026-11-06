$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
cargo test --workspace
if ($LASTEXITCODE -ne 0) { throw 'Workspace tests failed' }
cargo check -p sim-core --example toxin_evolution_0758
if ($LASTEXITCODE -ne 0) { throw 'Observation example compilation failed' }
powershell.exe -ExecutionPolicy Bypass -File '.\tools\verify_phase0733.ps1'
if ($LASTEXITCODE -ne 0) { throw 'Legacy regression failed' }
Write-Host 'PASS Phase 07.5.8 telemetry and legacy regression'
