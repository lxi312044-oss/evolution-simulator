param([int]$Ticks=10000,[int]$Interval=100)
$ErrorActionPreference="Stop"
Set-Location (Split-Path -Parent $PSScriptRoot)
New-Item -ItemType Directory -Force -Path "results/phase0742" | Out-Null
cargo test --workspace
if ($LASTEXITCODE -ne 0) { throw "Rust tests failed" }
# Compare the legacy model with Phase 07.3.1 baseline first.
powershell.exe -ExecutionPolicy Bypass -File ".\tools\verify_phase0733.ps1"
if ($LASTEXITCODE -ne 0) { throw "Baseline regression failed" }
cargo run --release -p sim-core --example plant_selection_0742 -- "config/phase0741-mixed-plants.json" $Ticks $Interval "results/phase0742/mixed-seed12345.csv"
if ($LASTEXITCODE -ne 0) { throw "Mixed trait observation failed" }
Write-Host "PASS Phase 07.4.2: results/phase0742/mixed-seed12345.csv"
