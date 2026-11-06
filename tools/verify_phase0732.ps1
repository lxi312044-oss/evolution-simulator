param([int]$Ticks=10000,[int]$Interval=100,[int]$Seed=12345)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$root=Split-Path -Parent $PSScriptRoot
$base=Join-Path $root 'docs/phase07/phase0731-baseline'
$metadata=Get-Content -LiteralPath (Join-Path $base 'run-metadata.json') -Raw -Encoding UTF8 | ConvertFrom-Json
if ($Ticks -ne [int]$metadata.ticks -or $Interval -ne [int]$metadata.interval -or $Seed -ne [int]$metadata.start_seed) {throw 'Verification parameters must match the frozen baseline.'}
$dir=Join-Path $root 'results/phase0732/verify'
New-Item -ItemType Directory -Path $dir -Force | Out-Null
$cfg=Join-Path $base "standard-toxin-seed$Seed.json"
$resource=Join-Path $dir 'resource-reproduction.csv'
$life=Join-Path $dir 'life-events.csv'
$ci=[System.Globalization.CultureInfo]::InvariantCulture
Push-Location $root
try {
  & cargo test --workspace
  if ($LASTEXITCODE -ne 0) {throw 'cargo test failed'}
  & cargo run --release -p sim-core --example resistance_life_07222 -- $cfg $Ticks $Interval $resource $metadata.germination_mode ([double]$metadata.soil_threshold).ToString($ci) ([double]$metadata.germination_probability).ToString($ci) ([int]$metadata.check_interval) ([int]$metadata.max_births) $life
  if ($LASTEXITCODE -ne 0) {throw 'Baseline reproduction failed'}
  foreach($pair in @(@("standard-toxin-seed$Seed.resource-reproduction.csv",$resource),@("standard-toxin-seed$Seed.life-events.csv",$life))) {
    $expected=Join-Path $base $pair[0];$actual=$pair[1]
    if (-not (Test-Path -LiteralPath $actual)) {throw "Missing output $actual"}
    $a=(Get-FileHash -LiteralPath $expected -Algorithm SHA256).Hash
    $b=(Get-FileHash -LiteralPath $actual -Algorithm SHA256).Hash
    if ($a -ne $b) {
      Write-Host "DIFFERENT BYTES: $($pair[0]); checking CSV records..."
      $expectedRows=Import-Csv -LiteralPath $expected
      $actualRows=Import-Csv -LiteralPath $actual
      if ($expectedRows.Count -ne $actualRows.Count) {throw "Row count mismatch: $($pair[0])"}
      # Compare CSV rows as multisets: event emission order may vary while
      # every value and duplicate count must remain identical.
      $expectedCounts=New-Object 'System.Collections.Generic.Dictionary[string,int]' ([System.StringComparer]::Ordinal)
      $actualCounts=New-Object 'System.Collections.Generic.Dictionary[string,int]' ([System.StringComparer]::Ordinal)
      foreach($row in $expectedRows) {
        $key=$row | ConvertTo-Json -Compress -Depth 10
        if (-not $expectedCounts.ContainsKey($key)) {$expectedCounts[$key]=0}
        $expectedCounts[$key]++
      }
      foreach($row in $actualRows) {
        $key=$row | ConvertTo-Json -Compress -Depth 10
        if (-not $actualCounts.ContainsKey($key)) {$actualCounts[$key]=0}
        $actualCounts[$key]++
      }
      foreach($key in $expectedCounts.Keys) {
        if (-not $actualCounts.ContainsKey($key) -or $actualCounts[$key] -ne $expectedCounts[$key]) {
          throw "CSV record/count mismatch: $($pair[0]); expected=$($expectedCounts[$key]) actual=$($actualCounts[$key]); record=$key"
        }
      }
      foreach($key in $actualCounts.Keys) {
        if (-not $expectedCounts.ContainsKey($key)) {throw "Unexpected CSV record: $($pair[0]); record=$key"}
      }
      Write-Host "PASS equivalent CSV rows: $($pair[0])"
    } else { Write-Host "PASS identical SHA256: $($pair[0])" }
  }
  Write-Host 'PASS Phase 07.3.3 pre/post regression comparison.'
} finally {Pop-Location}
