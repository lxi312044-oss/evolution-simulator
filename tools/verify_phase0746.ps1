$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
python .\tools\audit_phase0746.py
if ($LASTEXITCODE -ne 0) { throw 'Static audit failed' }
cargo test --workspace
if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed' }
powershell.exe -ExecutionPolicy Bypass -File '.\tools\verify_phase0733.ps1'
if ($LASTEXITCODE -ne 0) { throw 'Legacy regression failed' }
Write-Host 'PASS Phase 07.4.6 design verification'
