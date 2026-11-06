param(
    [int]$Ticks = 50000,
    [int]$Interval = 100,
    [int]$SeedStart = 12345,
    [int]$SeedCount = 5,
    [string]$ConfigFile = "config/phase06214-validation.json"
)
$ErrorActionPreference = 'Stop'
if ($Ticks -lt 1 -or $Interval -lt 1 -or $SeedCount -lt 1) { throw 'Ticks, Interval, SeedCount must be positive.' }
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root
$outDir = Join-Path $root 'results/phase06215'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$summary = @()
for ($index = 0; $index -lt $SeedCount; $index++) {
    $seed = $SeedStart + $index
    $config = Get-Content (Join-Path $root $ConfigFile) -Raw -Encoding UTF8 | ConvertFrom-Json
    $config.seed = $seed
    $configPath = Join-Path $outDir "config-$seed.json"
    $csvPath = Join-Path $outDir "seed-$seed.csv"
    [System.IO.File]::WriteAllText($configPath, ($config | ConvertTo-Json -Depth 50), (New-Object System.Text.UTF8Encoding($false)))
    Write-Host "[Phase 06.2.15] Seed $seed ($($index+1)/$SeedCount)"
    & cargo run --release -p sim-core --example carnivore_verification -- $configPath $Ticks $Interval $csvPath
    if ($LASTEXITCODE -ne 0) { throw "Simulation failed for seed $seed" }
    $rows = @(Import-Csv $csvPath)
    if ($rows.Count -eq 0) { throw "CSV is empty for seed $seed" }
    $last = $rows[-1]
    $zero = $rows | Where-Object { [int]$_.carnivores -eq 0 } | Select-Object -First 1
    $summary += [pscustomobject]@{
        seed=$seed; tick=$last.tick; plants=$last.plants; herbivores=$last.herbivores; carnivores=$last.carnivores
        carnivore_extinction_tick=$(if($zero){$zero.tick}else{''})
        carnivore_births=$last.carnivore_births; carnivore_deaths=$last.carnivore_deaths
        hunt_attempts=$last.hunt_attempts; prey_kills=$last.prey_hp_zero_after_attack
        hunt_energy=$last.hunt_energy; energy_basal_spent=$last.energy_basal_spent
        energy_movement_spent=$last.energy_movement_spent; predator_movement_distance=$last.predator_movement_distance
        predator_search_ticks=$last.predator_search_ticks; predator_target_ticks=$last.predator_target_ticks
        persistent_search_ticks=$last.persistent_search_ticks; remembered_prey_ticks=$last.remembered_prey_ticks
        search_heading_changes=$last.search_heading_changes; search_wall_turns=$last.search_wall_turns
        nutrient_error=$last.nutrient_error; energy_error=$last.energy_error
    }
}
$summary | Export-Csv (Join-Path $outDir 'summary.csv') -NoTypeInformation -Encoding UTF8
Write-Host "Completed: $outDir"
