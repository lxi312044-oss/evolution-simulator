param(
 [int]$Ticks = 50000,
 [int]$StartSeed = 12350,
 [int]$SeedCount = 15,
 [switch]$Force,
 [switch]$DryRun
)
$ErrorActionPreference = 'Stop'
if ($Ticks -lt 1001) { throw 'Ticks must be >= 1001 to observe the 1000-tick horizon.' }
if ($SeedCount -lt 1 -or $SeedCount -gt 1000) { throw 'SeedCount must be 1..1000.' }
if ($StartSeed -lt 0 -or ([long]$StartSeed + $SeedCount - 1) -gt [int]::MaxValue) { throw 'Invalid seed range.' }
$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root 'results/phase07210/runs'
if (-not $DryRun) { [void][System.IO.Directory]::CreateDirectory($out) }
$utf8 = New-Object System.Text.UTF8Encoding($false)
$conditions = @('B_no_toxin_cost', 'F_strong_cost')
foreach ($condition in $conditions) {
 $base = Join-Path $root "config/phase0722-$condition.json"
 if (!(Test-Path -LiteralPath $base)) { throw "Missing config: $base" }
 foreach ($seed in ($StartSeed..($StartSeed + $SeedCount - 1))) {
  $prefix = Join-Path $out "$condition-seed$seed"
  $cfgPath = "$prefix.json"
  $eventPath = "$prefix.individual-events.csv"
  if ((Test-Path -LiteralPath $eventPath) -and !$Force) { Write-Host "Skip existing $eventPath"; continue }
  if ($DryRun) { Write-Host "DRY RUN: $condition seed=$seed ticks=$Ticks -> $prefix"; continue }
  $cfg = Get-Content -LiteralPath $base -Raw -Encoding UTF8 | ConvertFrom-Json
  $cfg.seed = $seed
  [System.IO.File]::WriteAllText($cfgPath, ($cfg | ConvertTo-Json -Depth 100), $utf8)
  & cargo run --release -p sim-core --example resistance_evolution_0727 -- $cfgPath $Ticks 100 $prefix
  if ($LASTEXITCODE -ne 0) { throw "Simulation failed: $condition seed=$seed" }
  if (!(Test-Path -LiteralPath $eventPath)) { throw "Missing event CSV: $eventPath" }
 }
}
Write-Host 'Phase 07.2.10 pilot execution finished.'
