param([int]$Ticks=50000,[int]$StartSeed=12345,[int]$SeedCount=20,[int]$Interval=100,[double]$SoilThreshold=20,[double]$Probability=0.02,[int]$CheckInterval=20,[int]$MaxBirths=20,[switch]$DryRun)
$ErrorActionPreference='Stop'
if ($Ticks -le 0 -or $SeedCount -le 0 -or $Interval -le 0 -or $Ticks % $Interval -ne 0 -or $SoilThreshold -lt 0 -or $Probability -lt 0 -or $Probability -gt 1 -or $CheckInterval -le 0 -or $MaxBirths -le 0) {throw 'Invalid parameters'}
$root=Split-Path -Parent $PSScriptRoot
$out=Join-Path $root 'results/phase07219/runs'
if (-not $DryRun) {New-Item -ItemType Directory -Path $out -Force | Out-Null}
$utf8=New-Object System.Text.UTF8Encoding($false)
$culture=[System.Globalization.CultureInfo]::InvariantCulture
foreach ($condition in @('B_no_toxin_cost','F_strong_cost')) {
 $base=Join-Path $root "config/phase0722-$condition.json"
 if (!(Test-Path -LiteralPath $base)) {throw "Missing config: $base"}
 foreach ($seed in ($StartSeed..($StartSeed+$SeedCount-1))) {
  foreach ($mode in @('off','on')) {
   $prefix=Join-Path $out "$condition-seed$seed-$mode"
   $cfgPath="$prefix.json";$result="$prefix.resource-reproduction.csv"
   if ($DryRun) {Write-Host "DRY RUN: $condition seed=$seed mode=$mode result=$result";continue}
   if (Test-Path -LiteralPath $result) {throw "Existing result: $result"}
   $cfg=Get-Content -LiteralPath $base -Raw -Encoding UTF8 | ConvertFrom-Json
   $cfg.seed=$seed
   [System.IO.File]::WriteAllText($cfgPath,($cfg|ConvertTo-Json -Depth 100),$utf8)
   & cargo run --release -p sim-core --example resource_reproduction_07217 -- $cfgPath $Ticks $Interval $result $mode $SoilThreshold.ToString($culture) $Probability.ToString($culture) $CheckInterval $MaxBirths
   if ($LASTEXITCODE -ne 0) {throw "Simulation failed: $condition seed=$seed mode=$mode"}
  }
 }
}
if (-not $DryRun) {
 & python (Join-Path $root 'tools/analyze_phase07219.py') --root $out --out (Join-Path $root 'results/phase07219/analysis') --ticks $Ticks --interval $Interval --expected-seeds $SeedCount
 if ($LASTEXITCODE -ne 0) {throw 'Analysis quality check failed'}
}
