param([string]$Input = "results/phase072221/analysis/individual_fitness.csv", [string]$Out = "results/phase07224/analysis")
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Push-Location $Root
try { python tools/analyze_phase07224.py --input $Input --out $Out; if ($LASTEXITCODE -ne 0) { throw "Analysis failed: $LASTEXITCODE" } } finally { Pop-Location }
