param(
 [Parameter(Mandatory=$true)][string]$Config,
 [int]$Ticks = 10000,
 [int]$Interval = 100,
 [string]$OutputPrefix = 'results/phase0753/pilot'
)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
$dir=Split-Path -Parent $OutputPrefix
if ($dir) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
cargo run --release -p sim-core --example toxin_evolution_0753 -- $Config $Ticks $Interval $OutputPrefix
if ($LASTEXITCODE -ne 0) { throw 'Phase 07.5.3 experiment failed' }
Write-Host "PASS Phase 07.5.3 CSV generated: $OutputPrefix"
