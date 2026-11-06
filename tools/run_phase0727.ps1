param([int]$Ticks=50000,[int]$SeedCount=5,[switch]$Force)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$seeds=@(12345,12346,12347,12348,12349)
if($SeedCount -lt 1 -or $SeedCount -gt 5){throw 'SeedCount must be 1..5'}
if($Ticks -lt 1){throw 'Ticks must be positive'}
$out=Join-Path $root 'results/phase0727'
[void][System.IO.Directory]::CreateDirectory($out)
$utf8=New-Object System.Text.UTF8Encoding($false)
foreach($condition in @('A_no_toxin_no_cost','B_no_toxin_cost','C_toxin_no_cost','D_toxin_cost','E_strong_no_cost','F_strong_cost')){
 $base=Join-Path $root "config/phase0722-$condition.json"
 if(!(Test-Path $base)){throw "Missing config: $base"}
 foreach($seed in $seeds[0..($SeedCount-1)]){
  $prefix=Join-Path $out "$condition-seed$seed"
  if((Test-Path "$prefix.individual-events.csv") -and !$Force){Write-Host "Skip $prefix";continue}
  $cfg=Get-Content -LiteralPath $base -Raw | ConvertFrom-Json
  $cfg.seed=$seed
  $cfgPath=Join-Path $out "$condition-seed$seed.json"
  [System.IO.File]::WriteAllText($cfgPath,($cfg|ConvertTo-Json -Depth 100),$utf8)
  & cargo run --release -p sim-core --example resistance_evolution_0727 -- $cfgPath $Ticks 100 $prefix
  if($LASTEXITCODE -ne 0){throw "Simulation failed: $condition seed=$seed"}
 }
}
