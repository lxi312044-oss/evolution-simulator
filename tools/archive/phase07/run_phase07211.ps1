param([int]$Ticks=50000,[int]$Interval=100,[int]$SeedStart=12345,[int]$SeedCount=5,[string[]]$Conditions=@('off','toxin_no_resistance','toxin_resistance'),[switch]$Force)
$ErrorActionPreference='Stop'
$projectRoot = $PSScriptRoot
Set-Location -LiteralPath $projectRoot
if($Ticks -lt 1 -or $Interval -lt 1 -or $SeedCount -lt 1){throw 'Invalid arguments'}
foreach($condition in $Conditions){
 if($condition -notin @('off','toxin_no_resistance','toxin_resistance')){throw "Invalid condition: $condition"}
 $dir="results/phase07211/$condition"; New-Item -ItemType Directory -Force $dir|Out-Null
 for($i=0;$i -lt $SeedCount;$i++){
  $seed=$SeedStart+$i
  $config=Get-Content "config/phase072-$condition.json" -Raw -Encoding UTF8|ConvertFrom-Json
  $config.seed=$seed
  $path="$dir/config-$seed.json";$prefix="$dir/seed-$seed"
  if(-not $Force -and (Test-Path "$prefix.resistance.csv") -and (Test-Path "$prefix.resistance-events.csv") -and (Test-Path "$prefix.genealogy.csv")){Write-Host "[SKIP] $condition $seed (use -Force to regenerate)";continue}
  $json = $config | ConvertTo-Json -Depth 50
  $fullPath = Join-Path $projectRoot ($path -replace "/", [System.IO.Path]::DirectorySeparatorChar)
  $parentDir = [System.IO.Path]::GetDirectoryName($fullPath)
  [System.IO.Directory]::CreateDirectory($parentDir) | Out-Null
  [System.IO.File]::WriteAllText($fullPath, $json, (New-Object System.Text.UTF8Encoding($false)))
  Write-Host "[RUN] $condition $seed"
  & cargo run --release -p sim-core --example resistance_evolution_07211 -- $path $Ticks $Interval $prefix
  if($LASTEXITCODE -ne 0){throw "Simulation failed: $condition $seed"}
 }
}
Write-Host 'Phase 07.2.1.1 genealogy diagnostics completed'
