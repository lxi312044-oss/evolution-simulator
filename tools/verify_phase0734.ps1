param(
  [int]$Ticks = 30000,
  [int]$Interval = 100,
  [int]$Seed = 12345
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = Split-Path -Parent $PSScriptRoot
$base = Join-Path $root 'docs/phase07/phase0731-baseline'
$metadata = Get-Content -LiteralPath (Join-Path $base 'run-metadata.json') -Raw -Encoding UTF8 | ConvertFrom-Json
if ($Ticks -lt 1 -or $Interval -lt 1) { throw 'Ticks and Interval must be positive.' }
if ($Seed -ne [int]$metadata.start_seed) { throw 'This validation is fixed to baseline seed 12345.' }
$dir = Join-Path $root 'results/phase0734'
New-Item -ItemType Directory -Path $dir -Force | Out-Null
$cfg = Join-Path $base "standard-toxin-seed$Seed.json"
$ci = [System.Globalization.CultureInfo]::InvariantCulture
Push-Location $root
try {
  Write-Host '=== Phase 07.3.4: frozen 10,000Tick regression + cargo tests ==='
  & (Join-Path $PSScriptRoot 'verify_phase0733.ps1')
  if ($LASTEXITCODE -ne 0) { throw 'Phase 07.3.3 baseline regression failed.' }
  Write-Host '=== Phase 07.3.4: same-seed long run (two repetitions) ==='
  $outputs = @()
  foreach ($run in @(1,2)) {
    $resource = Join-Path $dir "run$run-resource-reproduction.csv"
    $life = Join-Path $dir "run$run-life-events.csv"
    & cargo run --release -p sim-core --example resistance_life_07222 -- $cfg $Ticks $Interval $resource $metadata.germination_mode ([double]$metadata.soil_threshold).ToString($ci) ([double]$metadata.germination_probability).ToString($ci) ([int]$metadata.check_interval) ([int]$metadata.max_births) $life
    if ($LASTEXITCODE -ne 0) { throw "Long-run simulation failed: run $run" }
    $outputs += $resource
  }
  & python (Join-Path $PSScriptRoot 'analyze_phase0734.py') --first $outputs[0] --second $outputs[1] --seed $Seed --ticks $Ticks --interval $Interval --output (Join-Path $dir 'REPORT.json')
  if ($LASTEXITCODE -ne 0) { throw 'Long-run analysis failed.' }
  Write-Host 'PASS Phase 07.3.4 verification. Review results/phase0734/REPORT.json'
} finally { Pop-Location }
