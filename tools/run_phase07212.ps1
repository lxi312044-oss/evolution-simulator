param(
 [int]$Ticks = 50000,
 [int]$StartSeed = 12345,
 [int]$SeedCount = 20,
 [int]$Interval = 100,
 [int]$ExpectedTicks = 50000,
 [switch]$Force,
 [switch]$DryRun
)
$ErrorActionPreference = 'Stop'
if ($Ticks -lt 1 -or $Interval -lt 1 -or $ExpectedTicks -lt 1) { throw 'Ticks and Interval must be positive.' }
if ($SeedCount -lt 1 -or $SeedCount -gt 1000) { throw 'SeedCount must be 1..1000.' }
if ($StartSeed -lt 0 -or ([long]$StartSeed + $SeedCount - 1) -gt [int]::MaxValue) { throw 'Invalid seed range.' }
$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root 'results/phase07212/runs'
if (-not $DryRun) { [void][System.IO.Directory]::CreateDirectory($out) }
$utf8 = New-Object System.Text.UTF8Encoding($false)
foreach ($condition in @('B_no_toxin_cost', 'F_strong_cost')) {
 $base = Join-Path $root "config/phase0722-$condition.json"
 if (!(Test-Path -LiteralPath $base)) { throw "Missing config: $base" }
 foreach ($seed in ($StartSeed..($StartSeed + $SeedCount - 1))) {
  $prefix = Join-Path $out "$condition-seed$seed"
  $cfgPath = "$prefix.json"
  $result = "$prefix.population-pressure.csv"
  if ((Test-Path -LiteralPath $result) -and !$Force) {
   $last = Import-Csv -LiteralPath $result | Select-Object -Last 1
   if ($null -eq $last -or [int]$last.tick -ne $Ticks) {
    throw "Existing run has wrong duration: $result (last tick=$($last.tick), requested=$Ticks). Use -Force to regenerate."
   }
   if (!(Test-Path -LiteralPath "$prefix.initial-genetics.csv")) { throw "Missing genetics CSV: $prefix" }
   Write-Host "Skip verified existing $result (tick=$Ticks)"; continue
  }
  if ($DryRun) { Write-Host "DRY RUN: $condition seed=$seed ticks=$Ticks -> $prefix"; continue }
  $cfg = Get-Content -LiteralPath $base -Raw -Encoding UTF8 | ConvertFrom-Json
  $cfg.seed = $seed
  [System.IO.File]::WriteAllText($cfgPath, ($cfg | ConvertTo-Json -Depth 100), $utf8)
  & cargo run --release -p sim-core --example resistance_covariates_07212 -- $cfgPath $Ticks $Interval $prefix
  if ($LASTEXITCODE -ne 0) { throw "Simulation failed: $condition seed=$seed" }
  if (!(Test-Path -LiteralPath $result)) { throw "Missing CSV: $result" }
  if (!(Test-Path -LiteralPath "$prefix.initial-genetics.csv")) { throw "Missing genetics CSV: $prefix" }
  $last = Import-Csv -LiteralPath $result | Select-Object -Last 1
  if ($null -eq $last -or [int]$last.tick -ne $Ticks) { throw "Wrong final tick for $result: $($last.tick), expected $Ticks" }
 }
}
Write-Host 'Phase 07.2.12 compact observations finished.'
