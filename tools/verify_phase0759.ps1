$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
python -m unittest discover -s tools -p 'test_phase0759*.py'
if ($LASTEXITCODE -ne 0) { throw 'Intervention analysis tests failed' }
cargo test --workspace
if ($LASTEXITCODE -ne 0) { throw 'Workspace tests failed' }
cargo check -p sim-core --example toxin_evolution_0758
if ($LASTEXITCODE -ne 0) { throw 'Observation example compilation failed' }
powershell.exe -ExecutionPolicy Bypass -File '.\tools\verify_phase0733.ps1'
if ($LASTEXITCODE -ne 0) { throw 'Legacy regression failed' }
Write-Host 'PASS Phase 07.5.9 intervention setup and legacy regression'
