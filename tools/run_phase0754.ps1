param(
 [int]$Ticks = 10000,
 [int]$Interval = 100
)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
New-Item -ItemType Directory -Force -Path 'results/phase0754' | Out-Null
foreach ($seed in @(12345,12346)) {
 $config = "docs/phase07/phase0753-pilot-seed$seed.json"
 $prefix = "results/phase0754/seed$seed"
 cargo run --release -p sim-core --example toxin_diagnosis_0754 -- $config $Ticks $Interval $prefix
 if ($LASTEXITCODE -ne 0) { throw "Phase 07.5.4 failed for seed $seed" }
}
Write-Host 'PASS Phase 07.5.4 paired scenario CSV generated'
