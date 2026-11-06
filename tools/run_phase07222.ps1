param([int]$Ticks=50000,[int]$StartSeed=12345,[int]$SeedCount=20,[int]$Interval=100,[double]$GerminationProbability=0.005,[double]$SoilThreshold=20,[int]$CheckInterval=20,[int]$MaxBirths=20,[switch]$DryRun)
$ErrorActionPreference='Stop'
if ($Ticks -le 0 -or $Interval -le 0 -or $Ticks % $Interval -ne 0 -or $SeedCount -le 0 -or $GerminationProbability -le 0 -or $GerminationProbability -gt 1) {throw 'Invalid parameters'}
$root=Split-Path -Parent $PSScriptRoot
$out=Join-Path $root 'results/phase07222/runs'
if (-not $DryRun) {New-Item -ItemType Directory -Path $out -Force | Out-Null}
$utf8=New-Object System.Text.UTF8Encoding($false)
$culture=[System.Globalization.CultureInfo]::InvariantCulture
foreach ($condition in @('B_no_toxin_cost','F_strong_cost')) {
 foreach ($mult in @(1.0,2.0,4.0)) {
  if ($condition -eq 'B_no_toxin_cost' -and $mult -ne 1.0) {continue}
  $level=if ($condition -eq 'B_no_toxin_cost') {'control'} else {"x$([int]$mult)"}
  foreach ($seed in ($StartSeed..($StartSeed+$SeedCount-1))) {
   $prefix=Join-Path $out "$condition-seed$seed-$level"
   $result="$prefix.resource-reproduction.csv"
   if ($DryRun) {Write-Host "DRY RUN $condition $level seed=$seed";continue}
   if ((Test-Path -LiteralPath $result) -and (Test-Path -LiteralPath "$prefix.life-events.csv")) {Write-Host "Skipping $result";continue}
   $cfg=Get-Content -LiteralPath (Join-Path $root "config/phase0722-$condition.json") -Raw -Encoding UTF8 | ConvertFrom-Json
   $cfg.seed=$seed
   if ($condition -eq 'F_strong_cost') {$cfg.toxin.hp_damage_per_excess_unit=[double]$cfg.toxin.hp_damage_per_excess_unit*$mult}
   $configPath="$prefix.json"
   [System.IO.File]::WriteAllText($configPath,($cfg|ConvertTo-Json -Depth 100),$utf8)
   & cargo run --release -p sim-core --example resistance_life_07222 -- $configPath $Ticks $Interval $result p50 $SoilThreshold.ToString($culture) $GerminationProbability.ToString($culture) $CheckInterval $MaxBirths "$prefix.life-events.csv"
   if ($LASTEXITCODE -ne 0) {throw "Simulation failed: $condition $level seed=$seed"}
  }
 }
}
if (-not $DryRun) {
 & python (Join-Path $root 'tools/analyze_phase072221.py') --root $out --out (Join-Path $root 'results/phase07222/analysis') --ticks $Ticks --interval $Interval --expected-seeds $SeedCount
 if ($LASTEXITCODE -ne 0) {throw 'Analysis failed'}
}
