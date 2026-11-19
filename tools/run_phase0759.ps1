param([int]$Ticks = 10000, [int]$Interval = 100, [string]$OutputDirectory = 'results/phase0759',
      [ValidateSet('toxin_evolution_0758','toxin_evolution_0759')][string]$ExampleName = 'toxin_evolution_0758')
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
if ($Ticks -le 0 -or $Interval -le 0) { throw 'Ticks and Interval must be positive' }
$dir = $OutputDirectory
if ((Test-Path -LiteralPath $dir) -and (Get-ChildItem -LiteralPath $dir -Force | Select-Object -First 1)) {
  throw 'Output directory is not empty. Choose a new -OutputDirectory to preserve previous evidence.'
}
New-Item -ItemType Directory -Force -Path $dir | Out-Null
cargo build --locked --release -p sim-core --example $ExampleName
if ($LASTEXITCODE -ne 0) { throw 'Observation example build failed' }
$cargoMetadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Cargo target directory lookup failed' }
$binary = Join-Path $cargoMetadata.target_directory "release/examples/$ExampleName.exe"
foreach ($seed in @(12345,12346)) {
  foreach ($condition in @(@('A80',0.8),@('B80',0.2),@('equal',0.5))) {
    foreach ($protection in @(@('on',0.8),@('off',0.0))) {
      $name = [string]$condition[0]
      $fraction = [double]$condition[1]
      $status = [string]$protection[0]
      $effect = [double]$protection[1]
      $source = "docs/phase07/phase0753-pilot-seed$seed.json"
      $json = Get-Content -LiteralPath $source -Raw -Encoding UTF8 | ConvertFrom-Json
      $json.toxin | Add-Member -NotePropertyName 'initial_type0_fraction' -NotePropertyValue $fraction -Force
      $json.toxin | Add-Member -NotePropertyName 'additional_type_resistance_upkeep_multiplier' -NotePropertyValue 1.0 -Force
      $json.toxin | Add-Member -NotePropertyName 'multi_type_resistance_upkeep_multiplier' -NotePropertyValue 0.5 -Force
      $json.toxin | Add-Member -NotePropertyName 'resistance_effectiveness' -NotePropertyValue $effect -Force
      $run = "seed${seed}_${name}_${status}"
      $cfg = Join-Path $dir "$run.json"
      [System.IO.File]::WriteAllText([System.IO.Path]::GetFullPath($cfg),($json | ConvertTo-Json -Depth 50),(New-Object System.Text.UTF8Encoding($false)))
      $metadata = Join-Path $dir "$run.metadata.json"
      python tools/run_metadata.py begin --config $cfg --binary $binary --output $metadata --ticks $Ticks --interval $Interval --example-name $ExampleName
      if ($LASTEXITCODE -ne 0) { throw "Metadata initialization failed: $run" }
      & $binary $cfg $Ticks $Interval (Join-Path $dir $run)
      if ($LASTEXITCODE -ne 0) { throw "Experiment failed: $run" }
      python tools/run_metadata.py complete --config $cfg --binary $binary --output $metadata --ticks $Ticks --interval $Interval --example-name $ExampleName
      if ($LASTEXITCODE -ne 0) { throw "Metadata validation failed: $run" }
    }
  }
}
python tools/analyze_phase0759.py $dir --ticks $Ticks --interval $Interval
if ($LASTEXITCODE -ne 0) { throw 'Phase 07.5.9 analysis failed' }
Write-Host 'PASS Phase 07.5.9 paired intervention experiments completed'
