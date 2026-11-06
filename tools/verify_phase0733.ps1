param([int]$Ticks=10000,[int]$Interval=100,[int]$Seed=12345)
$ErrorActionPreference='Stop'
& (Join-Path $PSScriptRoot 'verify_phase0732.ps1') -Ticks $Ticks -Interval $Interval -Seed $Seed
if ($LASTEXITCODE -ne 0) {exit $LASTEXITCODE}
