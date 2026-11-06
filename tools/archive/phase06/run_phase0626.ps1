param(
    [int]$Ticks = 50000,
    [int]$Interval = 100,
    [int]$SeedStart = 12345,
    [int]$SeedCount = 30
)
$ErrorActionPreference = 'Stop'
if ($SeedCount -lt 1) { throw 'SeedCount must be positive' }
$seeds = (($SeedStart..($SeedStart + $SeedCount - 1)) -join ',')
foreach ($mode in @('off', 'refuge20', 'refuge50', 'refuge100')) {
    Write-Host "Evaluating $mode ($SeedCount seeds, $Ticks ticks each)..."
    cargo run --release -p sim-core --example refuge_batch_evaluation -- "config/phase0626-$mode.json" $Ticks $Interval $seeds "results/phase0626/$mode"
    if ($LASTEXITCODE -ne 0) { throw "Simulation failed: $mode (exit $LASTEXITCODE)" }
}
Write-Host 'Finished. Compare results/phase0626/*/summary.csv and per_seed.csv'
