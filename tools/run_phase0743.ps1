param(
    [int]$Ticks = 10000,
    [int]$Interval = 100,
    [int[]]$Seeds = @(12345,12346,12347),
    [switch]$SkipRegression
)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
if ($Ticks -le 0 -or $Interval -le 0 -or $Seeds.Count -eq 0) { throw 'Invalid ticks, interval or seeds' }
$dir = 'results/phase0743'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
cargo test --workspace
if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed' }
python -m unittest discover -s tools -p 'test_phase0743.py'
if ($LASTEXITCODE -ne 0) { throw 'Analysis tests failed' }
if (-not $SkipRegression) {
    powershell.exe -ExecutionPolicy Bypass -File '.\tools\verify_phase0733.ps1'
    if ($LASTEXITCODE -ne 0) { throw 'Frozen baseline regression failed' }
}
cargo build --release -p sim-core --example plant_selection_0742
if ($LASTEXITCODE -ne 0) { throw 'Example build failed' }
$exe = '.\target\release\examples\plant_selection_0742.exe'
if (-not (Test-Path $exe)) { throw "Executable missing: $exe" }
$base = Get-Content -Raw -Encoding UTF8 'config/phase0741-mixed-plants.json' | ConvertFrom-Json
$cases = @(
    @{Name='zero';Cost=0.0},
    @{Name='standard';Cost=0.002},
    @{Name='high';Cost=0.02}
)
foreach ($seed in $Seeds) {
    foreach ($case in $cases) {
        $config = $base | ConvertTo-Json -Depth 100 | ConvertFrom-Json
        $config.seed = $seed
        $config.toxin.plant_energy_cost_per_tick = $case.Cost
        $cfg = Join-Path $dir ("config-{0}-seed{1}.json" -f $case.Name,$seed)
        $csv = Join-Path $dir ("{0}-seed{1}.csv" -f $case.Name,$seed)
        [System.IO.File]::WriteAllText((Join-Path (Get-Location).Path $cfg), ($config | ConvertTo-Json -Depth 100), (New-Object System.Text.UTF8Encoding($false)))
        Write-Host "Running seed=$seed case=$($case.Name) cost=$($case.Cost)"
        & $exe $cfg $Ticks $Interval $csv
        if ($LASTEXITCODE -ne 0) { throw "Simulation failed: $csv" }
    }
}
python tools/analyze_phase0743.py --results $dir --ticks $Ticks --seeds $Seeds
if ($LASTEXITCODE -ne 0) { throw 'Phase 07.4.3 analysis failed' }
Write-Host "PASS Phase 07.4.3 pilot. Review $dir/REPORT.json"
