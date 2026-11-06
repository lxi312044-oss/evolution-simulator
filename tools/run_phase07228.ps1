param(
    [string]$InputZip = "",
    [string]$Out = "results/phase07228/analysis"
)
$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($InputZip)) {
    $InputZip = Join-Path $project 'phase07225.zip'
}
if (-not (Test-Path -LiteralPath $InputZip -PathType Leaf)) {
    throw "Input ZIP not found: $InputZip"
}
$script = Join-Path $PSScriptRoot 'analyze_phase07228.py'
$dest = if ([System.IO.Path]::IsPathRooted($Out)) { $Out } else { Join-Path $project $Out }
python $script --zip $InputZip --out $dest
if ($LASTEXITCODE -ne 0) { throw "Analysis failed: exit code $LASTEXITCODE" }
Write-Host "Phase 07.2.28 analysis: $dest"
