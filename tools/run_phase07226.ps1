param(
    [Parameter(Mandatory=$true)][string]$InputZip,
    [string]$Output = 'results/phase07226/analysis'
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not [System.IO.Path]::IsPathRooted($Output)) { $Output = Join-Path $root $Output }
if (-not (Test-Path -LiteralPath $InputZip)) { throw "Input ZIP not found: $InputZip" }
& python (Join-Path $PSScriptRoot 'analyze_phase07226.py') --zip $InputZip --out $Output
if ($LASTEXITCODE -ne 0) { throw "Phase 07.2.26 analysis failed: $LASTEXITCODE" }
Write-Host "Phase 07.2.26 analysis saved: $Output"
