param(
 [int]$Ticks=50000,
 [int]$Interval=100,
 [int]$SeedStart=12345,
 [int]$SeedCount=5,
 [string[]]$Modes=@("control","basal-80","movement-80","combined-80")
)
$ErrorActionPreference='Stop'
if ($Ticks -lt 1 -or $Interval -lt 1 -or $SeedCount -lt 1) {throw 'Ticks, Interval, SeedCount must be positive.'}
$root=Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root
$outDir=Join-Path $root 'results/phase06231'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
foreach($mode in $Modes){
 if($mode -notin @('control','basal-80','movement-80','combined-80')) {throw "Unknown mode: $mode"}
 $modeDir=Join-Path $outDir $mode
 New-Item -ItemType Directory -Force -Path $modeDir | Out-Null
 for($index=0;$index -lt $SeedCount;$index++){
  $seed=$SeedStart+$index
  $config=Get-Content (Join-Path $root "config/phase06231-$mode.json") -Raw -Encoding UTF8 | ConvertFrom-Json
  $config.seed=$seed
  $configPath=Join-Path $modeDir "config-$seed.json"
  $csvPath=Join-Path $modeDir "seed-$seed.csv"
  [System.IO.File]::WriteAllText($configPath,($config | ConvertTo-Json -Depth 50),(New-Object System.Text.UTF8Encoding($false)))
  Write-Host "[06.2.31 $mode] Seed $seed ($($index+1)/$SeedCount)"
  & cargo run --release -p sim-core --example carnivore_verification -- $configPath $Ticks $Interval $csvPath
  if($LASTEXITCODE -ne 0){throw "Simulation failed: $mode $seed"}
  $last=Import-Csv $csvPath | Select-Object -Last 1
  if(-not $last -or [int]$last.tick -ne $Ticks){throw "Incomplete CSV: $mode $seed"}
 }
}
Write-Host "Completed phase 06.2.31: $outDir"
