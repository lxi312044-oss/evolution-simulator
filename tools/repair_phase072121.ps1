param([switch]$DryRun)
$ErrorActionPreference = 'Stop'
$script = Join-Path $PSScriptRoot 'run_phase07212.ps1'
if ($DryRun) {
  & $script -Ticks 50000 -StartSeed 12345 -SeedCount 1 -Interval 100 -Force -DryRun
} else {
  & $script -Ticks 50000 -StartSeed 12345 -SeedCount 1 -Interval 100 -Force
}
if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) { throw "Repair failed: $LASTEXITCODE" }
