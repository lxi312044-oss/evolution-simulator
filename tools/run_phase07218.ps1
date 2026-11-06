param([int]$Ticks=50000,[int]$StartSeed=12345,[int]$SeedCount=20,[int]$Interval=100,[switch]$DryRun)
$ErrorActionPreference='Stop'
if ($Ticks -le 0 -or $SeedCount -le 0 -or $Interval -le 0 -or $Ticks % $Interval -ne 0) {throw 'Invalid ticks/interval/seed count'}
$root=Split-Path -Parent $PSScriptRoot
$out=Join-Path $root 'results/phase07218/runs'
if (-not $DryRun) {New-Item -ItemType Directory -Path $out -Force | Out-Null}
$utf8=New-Object System.Text.UTF8Encoding($false)
foreach ($condition in @('B_no_toxin_cost','F_strong_cost')) {
 $base=Join-Path $root "config/phase0722-$condition.json"
 if (!(Test-Path -LiteralPath $base)) {throw "Missing config: $base"}
 foreach ($seed in ($StartSeed..($StartSeed+$SeedCount-1))) {
  $prefix=Join-Path $out "$condition-seed$seed"
  $cfgPath="$prefix.json";$result="$prefix.resource-reproduction.csv"
  if ($DryRun) {Write-Host "DRY RUN: $condition seed=$seed result=$result";continue}
  if (Test-Path -LiteralPath $result) {throw "Existing result: $result"}
  $cfg=Get-Content -LiteralPath $base -Raw -Encoding UTF8 | ConvertFrom-Json
  $cfg.seed=$seed
  [System.IO.File]::WriteAllText($cfgPath,($cfg|ConvertTo-Json -Depth 100),$utf8)
  & cargo run --release -p sim-core --example resource_reproduction_07217 -- $cfgPath $Ticks $Interval $result
  if ($LASTEXITCODE -ne 0) {throw "Simulation failed: $condition seed=$seed"}
 }
}
& python (Join-Path $root 'tools/analyze_phase07218.py') --root $out --out (Join-Path $root 'results/phase07218/analysis') --end $Ticks --interval $Interval --expected-seeds $SeedCount
if ($LASTEXITCODE -ne 0) {throw 'Analysis quality check failed'}
