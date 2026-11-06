param(
  [switch]$LongRun,
  [switch]$WebBuild
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
  python .\tools\audit_phase075_gate.py
  if ($LASTEXITCODE -ne 0) { throw 'Phase 07.5 static audit failed' }
  python .\tools\audit_phase0746.py
  if ($LASTEXITCODE -ne 0) { throw 'Phase 07.4.6 contract audit failed' }
  cargo test --workspace
  if ($LASTEXITCODE -ne 0) { throw 'Rust workspace tests failed' }
  & powershell.exe -ExecutionPolicy Bypass -File '.\tools\verify_phase0733.ps1'
  if ($LASTEXITCODE -ne 0) { throw 'Frozen regression failed' }
  if ($LongRun) {
    & powershell.exe -ExecutionPolicy Bypass -File '.\tools\verify_phase0734.ps1'
    if ($LASTEXITCODE -ne 0) { throw 'Long run verification failed' }
  }
  if ($WebBuild) {
    wasm-pack build crates/sim-wasm --target web --out-dir ../../web/src/pkg
    if ($LASTEXITCODE -ne 0) { throw 'wasm-pack failed' }
    Push-Location web
    try {
      npm install
      if ($LASTEXITCODE -ne 0) { throw 'npm install failed' }
      npm run build
      if ($LASTEXITCODE -ne 0) { throw 'Web build failed' }
    } finally { Pop-Location }
  }
  $path = 'results/phase075-gate/REPORT.json'
  $report = Get-Content -LiteralPath $path -Raw -Encoding UTF8 | ConvertFrom-Json
  $report | Add-Member -NotePropertyName technical_status -NotePropertyValue 'PASS' -Force
  $report | Add-Member -NotePropertyName rust_tests -NotePropertyValue 'PASS' -Force
  $report | Add-Member -NotePropertyName frozen_regression -NotePropertyValue 'PASS' -Force
  if ($LongRun) { $report | Add-Member -NotePropertyName long_run -NotePropertyValue 'PASS' -Force }
  if ($WebBuild) { $report | Add-Member -NotePropertyName web_build -NotePropertyValue 'BUILD_PASS_BROWSER_UNVERIFIED' -Force }
  $report | ConvertTo-Json -Depth 15 | Set-Content -LiteralPath $path -Encoding UTF8
  Write-Host 'PASS Phase 07.5 technical integration verification'
  Write-Host 'NO-GO Phase 08 transition: official 07.5/07.6/07.7 acceptance conditions not met'
  Write-Host "Report: $path"
} finally { Pop-Location }
