param([int]$Ticks=50000,[int]$SeedCount=5)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root
$seeds=@(12345,12346,12347,12348,12349)
foreach($mode in @('baseline','evasion','defense')) {
  $template=Get-Content -Raw -Encoding UTF8 "config/phase06233-$mode.json" | ConvertFrom-Json
  for($i=0;$i -lt [Math]::Min($SeedCount,5);$i++) {
    $seed=$seeds[$i]
    $dir="results/phase06233/$mode";New-Item -ItemType Directory -Force $dir | Out-Null
    $template.seed=$seed
    $config="$dir/config-$seed.json";$csv="$dir/seed-$seed.csv"
    [System.IO.File]::WriteAllText((Join-Path $root $config), ($template | ConvertTo-Json -Depth 50), (New-Object System.Text.UTF8Encoding($false)))
    Write-Host "[06.2.33 $mode] Seed $seed"
    cargo run --release -p sim-core --example carnivore_verification -- $config $Ticks 100 $csv
    if($LASTEXITCODE -ne 0){throw "Simulation failed: $mode $seed"}
  }
}
