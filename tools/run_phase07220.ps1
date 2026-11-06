param([int]$Ticks=50000,[int]$StartSeed=12345,[int]$SeedCount=20,[int]$Interval=100,[double]$SoilThreshold=20,[double[]]$Probabilities=@(0.0,0.005,0.01,0.02),[int]$CheckInterval=20,[int]$MaxBirths=20,[switch]$DryRun)
$ErrorActionPreference='Stop'
if ($Ticks -le 0 -or $SeedCount -le 0 -or $Interval -le 0 -or $Ticks % $Interval -ne 0 -or $SoilThreshold -lt 0 -or $CheckInterval -le 0 -or $MaxBirths -le 0 -or @($Probabilities | Where-Object {$_ -lt 0 -or $_ -gt 1}).Count -gt 0) {throw 'Invalid parameters'}
$root=Split-Path -Parent $PSScriptRoot
$out=Join-Path $root 'results/phase07220/runs'
if (-not $DryRun) {New-Item -ItemType Directory -Path $out -Force | Out-Null}
$utf8=New-Object System.Text.UTF8Encoding($false)
$culture=[System.Globalization.CultureInfo]::InvariantCulture
foreach ($condition in @('B_no_toxin_cost','F_strong_cost')) {
 $base=Join-Path $root "config/phase0722-$condition.json"
 if (!(Test-Path -LiteralPath $base)) {throw "Missing config: $base"}
 foreach ($seed in ($StartSeed..($StartSeed+$SeedCount-1))) {
  foreach ($prob in $Probabilities) {
   $mode=if ($prob -eq 0) {'off'} else { 'p'+[string][int][math]::Round($prob*10000) }
   $prefix=Join-Path $out "$condition-seed$seed-$mode"
   $cfgPath="$prefix.json";$result="$prefix.resource-reproduction.csv"
   if ($DryRun) {Write-Host "DRY RUN: $condition seed=$seed mode=$mode result=$result";continue}
   if (Test-Path -LiteralPath $result) {Write-Host "Skipping existing: $result";continue}
   $cfg=Get-Content -LiteralPath $base -Raw -Encoding UTF8 | ConvertFrom-Json
   $cfg.seed=$seed
   [System.IO.File]::WriteAllText($cfgPath,($cfg|ConvertTo-Json -Depth 100),$utf8)
   & cargo run --release -p sim-core --example resistance_germination_07220 -- $cfgPath $Ticks $Interval $result $mode $SoilThreshold.ToString($culture) $prob.ToString($culture) $CheckInterval $MaxBirths
   if ($LASTEXITCODE -ne 0) {throw "Simulation failed: $condition seed=$seed mode=$mode"}
  }
 }
}
if (-not $DryRun) {
 & python (Join-Path $root 'tools/analyze_phase07220.py') --root $out --out (Join-Path $root 'results/phase07220/analysis') --ticks $Ticks --interval $Interval --expected-seeds $SeedCount
 if ($LASTEXITCODE -ne 0) {throw 'Analysis quality check failed'}
}
