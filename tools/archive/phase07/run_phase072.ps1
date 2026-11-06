param(
 [int]$Ticks=50000,
 [int]$Interval=100,
 [int]$SeedStart=12345,
 [int]$SeedCount=5,
 [string[]]$Conditions=@('off','toxin_no_resistance','toxin_resistance'),
 [switch]$Force
)
$ErrorActionPreference='Stop'
if($Ticks -lt 1 -or $Interval -lt 1 -or $SeedCount -lt 1){throw 'Invalid ticks/interval/seed count'}
$root=Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root
$valid=@('off','toxin_no_resistance','toxin_resistance')
foreach($condition in $Conditions){if($condition -notin $valid){throw "Unknown condition: $condition"}}
$sourceFiles=Get-ChildItem 'crates/sim-core/src' -Recurse -File | Sort-Object FullName
$sourceFiles+=Get-Item 'crates/sim-core/examples/carnivore_verification.rs'
$hashInput=($sourceFiles|ForEach-Object {(Get-FileHash $_.FullName -Algorithm SHA256).Hash}) -join '|'
$sha=[System.Security.Cryptography.SHA256]::Create()
function HashText([string]$s){$b=[Text.Encoding]::UTF8.GetBytes($s);return ([BitConverter]::ToString($sha.ComputeHash($b))).Replace('-','').ToLowerInvariant()}
$sourceHash=HashText $hashInput
foreach($condition in $Conditions){
 $dir=Join-Path $root "results/phase072/$condition"
 New-Item -ItemType Directory -Force $dir|Out-Null
 for($i=0;$i -lt $SeedCount;$i++){
  $seed=$SeedStart+$i
  $cfg=Get-Content "config/phase072-$condition.json" -Raw -Encoding UTF8|ConvertFrom-Json
  $cfg.seed=$seed
  $json=$cfg|ConvertTo-Json -Depth 50 -Compress
  $key=HashText "$json|$Ticks|$Interval|$sourceHash"
  $configPath=Join-Path $dir "config-$seed.json"
  $csv=Join-Path $dir "seed-$seed.csv"
  $meta="$csv.complete.json"
  $skip=$false
  if(-not $Force -and (Test-Path $csv) -and (Test-Path $meta) -and (Test-Path "$csv.life.csv")){
   try{
    $m=Get-Content $meta -Raw|ConvertFrom-Json
    $head=(Get-Content $csv -TotalCount 1) -split ','
    $lastRaw=Get-Content $csv -Tail 1
    $last=$lastRaw|ConvertFrom-Csv -Header $head
    $lines=(Get-Content $csv|Measure-Object -Line).Lines
    $expected=[int][math]::Ceiling($Ticks/[double]$Interval)+2
    $skip=($m.key -eq $key -and [long]$last.tick -eq $Ticks -and $lines -eq $expected -and $head.Count -eq @($lastRaw -split ',').Count)
   }catch{$skip=$false}
  }
  if($skip){Write-Host "[SKIP] $condition $seed";continue}
  Write-Host "[RUN] $condition $seed"
  $tmp="$csv.partial";$tmpCfg="$configPath.partial"
  Remove-Item $tmp,$tmpCfg,"$tmp.life.csv" -ErrorAction SilentlyContinue
  [IO.File]::WriteAllText($tmpCfg,$json,(New-Object Text.UTF8Encoding($false)))
  try{
   & cargo run --release -p sim-core --example carnivore_verification -- $tmpCfg $Ticks $Interval $tmp
   if($LASTEXITCODE -ne 0){throw "cargo failed: $condition $seed"}
   if(-not (Test-Path "$tmp.life.csv")){throw "Missing life CSV: $condition $seed"}
   $head=(Get-Content $tmp -TotalCount 1) -split ','
   $lastRaw=Get-Content $tmp -Tail 1
   $last=$lastRaw|ConvertFrom-Csv -Header $head
   $lines=(Get-Content $tmp|Measure-Object -Line).Lines
   $expected=[int][math]::Ceiling($Ticks/[double]$Interval)+2
   if([long]$last.tick -ne $Ticks -or $lines -ne $expected -or $head.Count -ne @($lastRaw -split ',').Count){throw "Incomplete CSV: $condition $seed"}
   Move-Item -Force $tmpCfg $configPath
   Move-Item -Force "$tmp.life.csv" "$csv.life.csv"
   Move-Item -Force $tmp $csv
   $m=@{key=$key;condition=$condition;seed=$seed;ticks=$Ticks;interval=$Interval;source_hash=$sourceHash}|ConvertTo-Json -Compress
   [IO.File]::WriteAllText($meta,$m,(New-Object Text.UTF8Encoding($false)))
  }finally{Remove-Item $tmp,$tmpCfg,"$tmp.life.csv" -ErrorAction SilentlyContinue}
 }
}
Write-Host 'Completed phase072'
