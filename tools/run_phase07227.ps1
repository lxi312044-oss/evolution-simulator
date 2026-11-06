param(
    [string]$InputZip = "phase07225.zip",
    [string]$OutputDir = "results/phase07227/analysis"
)
$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location $projectRoot
python tools/analyze_phase07227.py --zip $InputZip --out $OutputDir
if ($LASTEXITCODE -ne 0) { throw "Phase 07.2.27 analysis failed (exit $LASTEXITCODE)" }
