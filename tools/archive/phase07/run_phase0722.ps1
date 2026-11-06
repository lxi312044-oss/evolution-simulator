param([int]$Ticks=50000,[int]$Interval=100,[int]$SeedStart=12345,[int]$SeedCount=5,[string[]]$Conditions=@('A_no_toxin_no_cost','B_no_toxin_cost','C_toxin_no_cost','D_toxin_cost','E_strong_no_cost','F_strong_cost'),[switch]$Force)
$ErrorActionPreference='Stop'
$root=$PSScriptRoot
Set-Location -LiteralPath $root
if($Ticks -lt 1 -or $Interval -lt 1 -or $SeedCount -lt 1){throw 'Ticks, Interval, SeedCount must be positive'}
foreach($condition in $Conditions){
 $template=Join-Path $root "config/phase0722-$condition.json"
 if(-not (Test-Path -LiteralPath $template)){throw "Unknown condition: $condition"}
 $dir=Join-Path $root "results/phase0722/$condition"
 [System.IO.Directory]::CreateDirectory($dir)|Out-Null
 for($i=0;$i -lt $SeedCount;$i++){
  $seed=$SeedStart+$i
  $prefix=Join-Path $dir "seed-$seed"
  if(-not $Force -and (Test-Path "$prefix.genealogy.csv") -and (Test-Path "$prefix.resistance.csv")){Write-Host "[SKIP] $condition $seed";continue}
  $cfg=Get-Content -LiteralPath $template -Raw -Encoding UTF8|ConvertFrom-Json
  $cfg.seed=$seed
  $configPath=Join-Path $dir "config-$seed.json"
  $json=$cfg|ConvertTo-Json -Depth 60
  [System.IO.File]::WriteAllText($configPath,$json,(New-Object System.Text.UTF8Encoding($false)))
  Write-Host "[RUN] $condition $seed"
  & cargo run --release -p sim-core --example resistance_evolution_07211 -- $configPath $Ticks $Interval $prefix
  if($LASTEXITCODE -ne 0){throw "Simulation failed: $condition $seed"}
 }
}
Write-Host 'Phase 07.2.2 experiments completed'
