param(
    [string]$Input = "results/phase072221/analysis/individual_fitness.csv",
    [string]$Out = "results/phase07223/analysis"
)
$ErrorActionPreference = "Stop"
$project = Split-Path -Parent $PSScriptRoot
Push-Location $project
try {
    python tools/analyze_phase07223.py --input $Input --out $Out
    if ($LASTEXITCODE -ne 0) { throw "Phase 07.2.23 analysis failed ($LASTEXITCODE)" }
} finally { Pop-Location }
