param([int]$Ticks = 10000, [int]$Interval = 100)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
if ($Ticks -le 0 -or $Interval -le 0) { throw 'Ticks and Interval must be positive' }
$dir = 'results/phase0756'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
foreach ($seed in @(12345,12346)) {
  $source = "docs/phase07/phase0753-pilot-seed$seed.json"
  foreach ($factor in @('1.0','0.5','0.25')) {
    foreach ($mode in @('protected','no_protection')) {
      $json = Get-Content -Raw -Encoding UTF8 $source | ConvertFrom-Json
      $json.toxin | Add-Member -NotePropertyName multi_type_resistance_upkeep_multiplier -NotePropertyValue ([double]::Parse($factor,[System.Globalization.CultureInfo]::InvariantCulture)) -Force
      $json.toxin.additional_type_resistance_upkeep_multiplier = 1.0
      if ($mode -eq 'no_protection') { $json.toxin.resistance_effectiveness = 0.0 }
      $name = "seed${seed}_cost${factor}_${mode}"
      $cfg = Join-Path $dir "$name.json"
      [System.IO.File]::WriteAllText((Join-Path (Get-Location).Path $cfg), ($json | ConvertTo-Json -Depth 50), (New-Object System.Text.UTF8Encoding($false)))
      $prefix = Join-Path $dir $name
      cargo run --release -p sim-core --example toxin_evolution_0753 -- $cfg $Ticks $Interval $prefix
      if ($LASTEXITCODE -ne 0) { throw "Experiment failed: $name" }
    }
  }
}
python tools/analyze_phase0756.py $dir
if ($LASTEXITCODE -ne 0) { throw 'Analysis failed' }
Write-Host 'PASS Phase 07.5.6 paired toxin-resistance selection pilot completed'
