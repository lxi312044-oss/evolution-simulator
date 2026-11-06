param([switch]$SkipRegression)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
cargo test --workspace
if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed' }
python -m unittest discover -s tools -p 'test_phase0744.py'
if ($LASTEXITCODE -ne 0) { throw 'Phase 07.4.4 analysis tests failed' }
if (-not $SkipRegression) {
    powershell.exe -ExecutionPolicy Bypass -File '.\tools\verify_phase0733.ps1'
    if ($LASTEXITCODE -ne 0) { throw 'Frozen baseline regression failed' }
}
if (-not (Test-Path 'results/phase0743/REPORT.json')) { throw 'Missing Phase 07.4.3 source results' }
python tools/analyze_phase0744.py --input results/phase0743 --output results/phase0744
if ($LASTEXITCODE -ne 0) { throw 'Phase 07.4.4 analysis failed' }
Write-Host 'PASS Phase 07.4.4 analysis; review results/phase0744/REPORT.json'
