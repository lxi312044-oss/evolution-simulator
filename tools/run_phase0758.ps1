param([int]$Ticks = 10000, [int]$Interval = 100)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
if ($Ticks -le 0 -or $Interval -le 0) { throw 'Ticks and Interval must be positive' }
$dir = 'results/phase0758'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
foreach ($seed in @(12345,12346)) {
  foreach ($condition in @(@('A80',0.8),@('B80',0.2),@('equal',0.5))) {
    $name = [string]$condition[0]
    $fraction = [double]$condition[1]
    $source = "docs/phase07/phase0753-pilot-seed$seed.json"
    $json = Get-Content -LiteralPath $source -Raw -Encoding UTF8 | ConvertFrom-Json
    $json.toxin | Add-Member -NotePropertyName 'initial_type0_fraction' -NotePropertyValue $fraction -Force
    $json.toxin | Add-Member -NotePropertyName 'additional_type_resistance_upkeep_multiplier' -NotePropertyValue 1.0 -Force
    $json.toxin | Add-Member -NotePropertyName 'multi_type_resistance_upkeep_multiplier' -NotePropertyValue 0.5 -Force
    $run = "seed${seed}_${name}"
    $cfg = Join-Path $dir "$run.json"
    [System.IO.File]::WriteAllText((Join-Path (Get-Location).Path $cfg),($json | ConvertTo-Json -Depth 50),(New-Object System.Text.UTF8Encoding($false)))
    cargo run --release -p sim-core --example toxin_evolution_0758 -- $cfg $Ticks $Interval (Join-Path $dir $run)
    if ($LASTEXITCODE -ne 0) { throw "Experiment failed: $run" }
  }
}
python tools/analyze_phase0758.py $dir
if ($LASTEXITCODE -ne 0) { throw 'Phase 07.5.8 analysis failed' }
Write-Host 'PASS Phase 07.5.8 observational selection diagnostics completed'
