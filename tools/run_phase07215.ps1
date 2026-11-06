param(
 [int]$Ticks=50000,
 [int]$StartSeed=12345,
 [int]$SeedCount=20,
 [int]$Interval=100,
 [switch]$DryRun
)
$ErrorActionPreference='Stop'
if ($Ticks -le 0 -or $Interval -le 0 -or $SeedCount -le 0) { throw 'Invalid positive arguments' }
if ([long]$StartSeed + $SeedCount - 1 -gt [int]::MaxValue) { throw 'Seed range overflow' }
$root=Split-Path -Parent $PSScriptRoot
$out=Join-Path $root 'results/phase07215/runs'
if (-not $DryRun) { New-Item -ItemType Directory -Path $out -Force | Out-Null }
$utf8=New-Object System.Text.UTF8Encoding($false)
foreach ($condition in @('B_no_toxin_cost','F_strong_cost')) {
 $base=Join-Path $root "config/phase0722-$condition.json"
 if (!(Test-Path -LiteralPath $base)) { throw "Missing config: ${base}" }
 foreach ($seed in ($StartSeed..($StartSeed + $SeedCount - 1))) {
  $prefix=Join-Path $out "$condition-seed$seed"
  $cfgPath="$prefix.json"
  $result="$prefix.reproduction.csv"
  if ($DryRun) { Write-Host "DRY RUN: $condition seed=$seed ticks=$Ticks output=$result"; continue }
  if (Test-Path -LiteralPath $result) { throw "Existing result: ${result}. Move or back up it before rerunning." }
  $cfg=Get-Content -LiteralPath $base -Raw -Encoding UTF8 | ConvertFrom-Json
  $cfg.seed=$seed
  [System.IO.File]::WriteAllText($cfgPath,($cfg|ConvertTo-Json -Depth 100),$utf8)
  & cargo run --release -p sim-core --example resistance_reproduction_07215 -- $cfgPath $Ticks $Interval $result
  if ($LASTEXITCODE -ne 0) { throw "Simulation failed: $condition seed=$seed" }
  if (!(Test-Path -LiteralPath $result)) { throw "Missing output: $result" }
  $last=Import-Csv -LiteralPath $result | Select-Object -Last 1
  if ($null -eq $last -or [int]$last.tick -ne $Ticks) { throw "Wrong final tick for ${result}: $($last.tick)" }
 }
}
Write-Host 'Phase 07.2.15 observations finished.'
