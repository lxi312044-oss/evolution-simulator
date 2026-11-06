param(
 [int]$Ticks=50000,
 [int]$Interval=100,
 [int]$SeedStart=12345,
 [int]$SeedCount=5,
 [string[]]$Modes=@('genetic'),
 [switch]$Force
)
$ErrorActionPreference='Stop'
if($Ticks -lt 1 -or $Interval -lt 1 -or $SeedCount -lt 1){throw 'Ticks, Interval and SeedCount must be positive.'}
$root=Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root
foreach($mode in $Modes){if($mode -notin @('genetic')){throw "Unknown mode: $mode"}}
$outDir=Join-Path $root 'results/phase06243'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
# Include source version in the cache key. Rebuilds after source changes must not reuse old results.
$sourceFiles=Get-ChildItem (Join-Path $root 'crates/sim-core/src') -Recurse -File | Sort-Object FullName
$sourceFiles+=Get-Item (Join-Path $root 'crates/sim-core/examples/carnivore_verification.rs')
$sourceHashInput=($sourceFiles | ForEach-Object { (Get-FileHash $_.FullName -Algorithm SHA256).Hash }) -join '|'
$sha=[System.Security.Cryptography.SHA256]::Create()
function HashText([string]$value){
 $bytes=[System.Text.Encoding]::UTF8.GetBytes($value)
 return ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-','').ToLowerInvariant()
}
$sourceHash=HashText $sourceHashInput
foreach($mode in $Modes){
 $modeDir=Join-Path $outDir $mode
 New-Item -ItemType Directory -Force -Path $modeDir | Out-Null
 for($index=0;$index -lt $SeedCount;$index++){
  $seed=$SeedStart+$index
  $config=Get-Content (Join-Path $root "config/phase06243-$mode.json") -Raw -Encoding UTF8 | ConvertFrom-Json
  $config.seed=$seed
  $configJson=$config | ConvertTo-Json -Depth 50 -Compress
  $key=HashText "$configJson|$Ticks|$Interval|$sourceHash"
  $configPath=Join-Path $modeDir "config-$seed.json"
  $csvPath=Join-Path $modeDir "seed-$seed.csv"
  $metaPath="$csvPath.complete.json"
  $complete=$false
  if(-not $Force -and (Test-Path $csvPath) -and (Test-Path $metaPath)){
   try {
    $meta=Get-Content $metaPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $header=Get-Content $csvPath -TotalCount 1
    $last=Get-Content $csvPath -Tail 1 | ConvertFrom-Csv -Header ($header -split ',')
    $lines=(Get-Content $csvPath | Measure-Object -Line).Lines
    $expectedLines=[int][math]::Ceiling($Ticks/[double]$Interval)+1
    # Includes header; exporter writes tick zero, each interval, and final tick.
    $expectedLines+=1
    $complete=($meta.key -eq $key -and [long]$last.tick -eq $Ticks -and $lines -eq $expectedLines -and ($header -split ',').Count -eq @((Get-Content $csvPath -Tail 1) -split ',').Count)
   }catch {$complete=$false}
  }
  if($complete -and -not (Test-Path "$csvPath.life.csv")){$complete=$false}
  if($complete){Write-Host "[SKIP] $mode seed=$seed complete";continue}
  Write-Host "[RUN] $mode seed=$seed"
  $tempCsv="$csvPath.partial"
  $tempConfig="$configPath.partial"
  Remove-Item $tempCsv,$tempConfig,"$tempCsv.life.csv" -ErrorAction SilentlyContinue
  [System.IO.File]::WriteAllText($tempConfig,$configJson,(New-Object System.Text.UTF8Encoding($false)))
  try {
   & cargo run --release -p sim-core --example carnivore_verification -- $tempConfig $Ticks $Interval $tempCsv
   if($LASTEXITCODE -ne 0){throw "Simulation failed: $mode $seed"}
   $header=Get-Content $tempCsv -TotalCount 1
   $last=Get-Content $tempCsv -Tail 1 | ConvertFrom-Csv -Header ($header -split ',')
   $lines=(Get-Content $tempCsv | Measure-Object -Line).Lines
   $expectedLines=[int][math]::Ceiling($Ticks/[double]$Interval)+2
   $headerCount=@($header -split ',').Count
   $dataCount=@((Get-Content $tempCsv -Tail 1) -split ',').Count
   $lastTick=[long]$last.tick
   Write-Host "CSV check: lastTick=$lastTick expectedTick=$Ticks; lines=$lines expectedLines=$expectedLines; headerColumns=$headerCount dataColumns=$dataCount"
   if($lastTick -ne $Ticks -or $lines -ne $expectedLines -or $headerCount -ne $dataCount){throw "Incomplete CSV: $mode $seed"}
   Move-Item -Force $tempConfig $configPath
   if(-not (Test-Path "$tempCsv.life.csv")){throw "Missing life event CSV: $mode $seed"}
   Move-Item -Force "$tempCsv.life.csv" "$csvPath.life.csv"
   Move-Item -Force $tempCsv $csvPath
   $metadata=@{key=$key;mode=$mode;seed=$seed;ticks=$Ticks;interval=$Interval;source_hash=$sourceHash} | ConvertTo-Json -Compress
   [System.IO.File]::WriteAllText($metaPath,$metadata,(New-Object System.Text.UTF8Encoding($false)))
  } finally {Remove-Item $tempCsv,$tempConfig,"$tempCsv.life.csv" -ErrorAction SilentlyContinue}
 }
}
Write-Host "Completed phase 06.2.43: $outDir"
