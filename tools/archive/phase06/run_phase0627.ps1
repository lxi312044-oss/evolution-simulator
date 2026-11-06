param(
    [int]$Ticks = 50000,
    [int]$Interval = 100,
    [int]$SeedStart = 12345,
    [int]$SeedCount = 30
)
$ErrorActionPreference = 'Stop'
if ($Ticks -lt 1 -or $Interval -lt 1 -or $SeedCount -lt 1) { throw 'Ticks, Interval and SeedCount must be positive' }
$dir = 'results/phase0627'
New-Item -ItemType Directory -Force $dir | Out-Null
$summary = @()
foreach ($seed in $SeedStart..($SeedStart + $SeedCount - 1)) {
    $configPath = Join-Path $dir "config-$seed.json"
    $config = Get-Content 'config/phase0625-natural.json' -Raw | ConvertFrom-Json
    $config.seed = $seed
    [System.IO.File]::WriteAllText((Join-Path (Get-Location) $configPath), ($config | ConvertTo-Json -Depth 50), (New-Object System.Text.UTF8Encoding($false)))
    $csvPath = Join-Path $dir "seed-$seed.csv"
    cargo run --release -p sim-core --example carnivore_verification -- $configPath $Ticks $Interval $csvPath
    if ($LASTEXITCODE -ne 0) { throw "Seed $seed failed" }
    $rows = Import-Csv $csvPath
    $last = $rows[-1]
    $firstCarnivoreZero = $rows | Where-Object { [int]$_.carnivores -eq 0 } | Select-Object -First 1
    $summary += [pscustomobject]@{
        seed = $seed
        plants = $last.plants
        herbivores = $last.herbivores
        carnivores = $last.carnivores
        carnivore_extinction_tick = if ($firstCarnivoreZero) { $firstCarnivoreZero.tick } else { '' }
        hunts = $last.hunt_attempts
        prey_visible_out_of_range = $last.prey_visible_out_of_range
        prey_in_attack_range = $last.prey_in_attack_range
        prey_hp_zero_after_attack = $last.prey_hp_zero_after_attack
        energy_gained = $last.hunt_energy
        energy_basal = $last.energy_basal_spent
        energy_movement = $last.energy_movement_spent
        energy_reproduction = $last.energy_reproduction_spent
        death_nearest_prey_distance_total = $last.predator_death_nearest_prey_distance_total
        death_nearest_prey_samples = $last.predator_death_nearest_prey_samples
        nutrient_error_final = $last.nutrient_error
        energy_error_final = $last.energy_error
    }
    Write-Host "Seed $seed complete: P/H/C=$($last.plants)/$($last.herbivores)/$($last.carnivores)"
}
$summary | Export-Csv (Join-Path $dir 'summary.csv') -NoTypeInformation -Encoding UTF8
Write-Host "Complete: $dir/summary.csv"
