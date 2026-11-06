param(
    [int]$Ticks = 50000,
    [int]$Interval = 100,
    [int]$SeedStart = 12345,
    [int]$SeedCount = 5,
    [string[]]$Modes = @("control", "plant-150", "herbivore-120", "combined")
)
$ErrorActionPreference = 'Stop'
if ($Ticks -lt 1 -or $Interval -lt 1 -or $SeedCount -lt 1) { throw 'Ticks, Interval, SeedCount must be positive.' }
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root
$outDir = Join-Path $root 'results/phase06220'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$summary = @()
foreach ($mode in $Modes) {
    if ($mode -notin @('control','plant-150','herbivore-120','combined')) { throw "Unknown mode: $mode" }
    $modeDir = Join-Path $outDir $mode
    New-Item -ItemType Directory -Force -Path $modeDir | Out-Null
    for ($index = 0; $index -lt $SeedCount; $index++) {
        $seed = $SeedStart + $index
        $sourceConfig = Join-Path $root "config/phase06220-$mode.json"
        $config = Get-Content $sourceConfig -Raw -Encoding UTF8 | ConvertFrom-Json
        $config.seed = $seed
        $configPath = Join-Path $modeDir "config-$seed.json"
        $csvPath = Join-Path $modeDir "seed-$seed.csv"
        [System.IO.File]::WriteAllText($configPath, ($config | ConvertTo-Json -Depth 50), (New-Object System.Text.UTF8Encoding($false)))
        Write-Host "[06.2.20 $mode] Seed $seed ($($index+1)/$SeedCount)"
        & cargo run --release -p sim-core --example carnivore_verification -- $configPath $Ticks $Interval $csvPath
        if ($LASTEXITCODE -ne 0) { throw "Simulation failed: $mode $seed" }
        $rows = @(Import-Csv $csvPath)
        if ($rows.Count -eq 0) { throw "CSV empty: $mode $seed" }
        $last = $rows[-1]
        $zero = $rows | Where-Object { [int]$_.carnivores -eq 0 } | Select-Object -First 1
        $peak = ($rows | ForEach-Object { [int]$_.carnivores } | Measure-Object -Maximum).Maximum
        $summary += [pscustomobject]@{
            phase20_feeding_no_target_ticks=$last.phase20_feeding_no_target_ticks; phase20_feeding_target_out_of_range_ticks=$last.phase20_feeding_target_out_of_range_ticks; phase20_feeding_in_range_zero_n_ticks=$last.phase20_feeding_in_range_zero_n_ticks; phase20_feeding_in_range_success_ticks=$last.phase20_feeding_in_range_success_ticks; phase20_repro_age_blocked=$last.phase20_repro_age_blocked; phase20_repro_cooldown_blocked=$last.phase20_repro_cooldown_blocked; phase20_repro_hp_blocked=$last.phase20_repro_hp_blocked; phase20_repro_energy_gate_blocked=$last.phase20_repro_energy_gate_blocked; phase20_repro_satiety_blocked=$last.phase20_repro_satiety_blocked; phase20_repro_food_blocked=$last.phase20_repro_food_blocked; phase20_repro_body_n_blocked=$last.phase20_repro_body_n_blocked; phase20_repro_reserve_n_blocked=$last.phase20_repro_reserve_n_blocked; phase20_repro_transfer_e_blocked=$last.phase20_repro_transfer_e_blocked; phase20_repro_success=$last.phase20_repro_success; phase20_herbivore_death_nearby_plants_total=$last.phase20_herbivore_death_nearby_plants_total; phase20_herbivore_death_samples=$last.phase20_herbivore_death_samples; phase20_herbivore_death_reserve_e_total=$last.phase20_herbivore_death_reserve_e_total; phase20_herbivore_death_stomach_n_total=$last.phase20_herbivore_death_stomach_n_total; phase19_hp_zero_energy_deaths=$last.phase19_hp_zero_energy_deaths; phase19_hp_positive_energy_deaths=$last.phase19_hp_positive_energy_deaths; phase19_plant_births=$last.phase19_plant_births; phase19_plant_age_deaths=$last.phase19_plant_age_deaths; phase19_plant_consumed_deaths=$last.phase19_plant_consumed_deaths; phase19_herbivore_births=$last.phase19_herbivore_births; phase19_herbivore_age_deaths=$last.phase19_herbivore_age_deaths; phase19_herbivore_hp_deaths=$last.phase19_herbivore_hp_deaths; phase19_plant_consumed_n=$last.phase19_plant_consumed_n; phase19_herbivore_feeding_e=$last.phase19_herbivore_feeding_e; phase19_herbivore_basal_e=$last.phase19_herbivore_basal_e; phase19_herbivore_movement_e=$last.phase19_herbivore_movement_e; phase19_plant_uptake_n=$last.phase19_plant_uptake_n; phase19_plant_photosynthesis_e=$last.phase19_plant_photosynthesis_e; phase17_rest_ticks=$last.phase17_rest_ticks; phase17_active_search_ticks=$last.phase17_active_search_ticks; phase17_chase_ticks=$last.phase17_chase_ticks; phase17_repro_resource_ready_ticks=$last.phase17_repro_resource_ready_ticks; phase17_repro_cooldown_only_ticks=$last.phase17_repro_cooldown_only_ticks; phase17_repro_age_only_ticks=$last.phase17_repro_age_only_ticks;
            energy_basal_spent=$last.energy_basal_spent; energy_movement_spent=$last.energy_movement_spent; energy_reproduction_spent=$last.energy_reproduction_spent; stomach_energy_digested=$last.stomach_energy_digested; stomach_energy_ingested=$last.stomach_energy_ingested; mode=$mode; seed=$seed; tick=$last.tick; plants=$last.plants; herbivores=$last.herbivores; carnivores=$last.carnivores
            peak_carnivores=$peak; carnivore_extinction_tick=$(if($zero){$zero.tick}else{''})
            carnivore_births=$last.carnivore_births; carnivore_deaths=$last.carnivore_deaths
            repro_ready=$last.repro_ready; repro_physical_ready_ticks=$last.repro_physical_ready_ticks
            repro_all_gates_passed_ticks=$last.repro_all_gates_passed_ticks
            repro_cooldown_blocked=$last.repro_cooldown_blocked; repro_age_blocked=$last.repro_age_blocked
            repro_body_n_blocked=$last.repro_body_n_blocked; repro_reserve_n_blocked=$last.repro_reserve_n_blocked
            repro_energy_blocked=$last.repro_energy_blocked; repro_capacity_blocked=$last.repro_capacity_blocked
            repro_food_blocked=$last.repro_food_blocked; repro_trend_blocked=$last.repro_trend_blocked
            stomach_reproduction_blocked=$last.stomach_reproduction_blocked
            sustainable_reproduction_blocked=$last.sustainable_reproduction_blocked
            carnivore_age_deaths=$last.carnivore_age_deaths; carnivore_energy_deaths=$last.carnivore_energy_deaths
            hunt_attempts=$last.hunt_attempts; prey_kills=$last.prey_hp_zero_after_attack
            hunt_energy=$last.hunt_energy; mean_carnivore_energy=$last.mean_carnivore_energy
            mean_carnivore_reserve_n=$last.mean_carnivore_reserve_n
            predator_death_age_total=$last.predator_death_age_total
            predator_deaths_with_stomach_energy=$last.predator_deaths_with_stomach_energy
            nutrient_error=$last.nutrient_error; energy_error=$last.energy_error
        }
    }
}
$summary | Export-Csv (Join-Path $outDir 'summary.csv') -NoTypeInformation -Encoding UTF8
Write-Host "Completed phase 06.2.20: $outDir"
