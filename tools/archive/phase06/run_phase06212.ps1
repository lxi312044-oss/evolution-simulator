param([int]$Ticks=50000,[int]$Interval=100,[int]$SeedStart=12345,[int]$SeedCount=10)
$ErrorActionPreference='Stop'
if($Ticks -lt 1 -or $Interval -lt 1 -or $SeedCount -lt 1){throw 'Positive values required'}
$modes=@('control','early-50','reserve-aware','forecast-aware')
foreach($mode in $modes){
 $dir="results/phase06212/$mode"
 New-Item -ItemType Directory -Force $dir | Out-Null
 $summary=@()
 foreach($seed in $SeedStart..($SeedStart+$SeedCount-1)){
  $config=Get-Content "config/phase06212-$mode.json" -Raw | ConvertFrom-Json
  $config.seed=$seed
  $path="$dir/config-$seed.json"
  [System.IO.File]::WriteAllText((Join-Path (Get-Location) $path),($config | ConvertTo-Json -Depth 50),(New-Object System.Text.UTF8Encoding($false)))
  $csv="$dir/seed-$seed.csv"
  cargo run --release -p sim-core --example carnivore_verification -- $path $Ticks $Interval $csv
  if($LASTEXITCODE -ne 0){throw "Failed $mode seed $seed"}
  $rows=Import-Csv $csv; $last=$rows[-1]
  $zero=$rows | Where-Object {[int]$_.carnivores -eq 0} | Select-Object -First 1
  $summary += [pscustomobject]@{mode=$mode;seed=$seed;plants=$last.plants;herbivores=$last.herbivores;carnivores=$last.carnivores;extinction_tick=$(if($zero){$zero.tick}else{''});births=$last.carnivore_births;deaths=$last.carnivore_deaths;hunts=$last.hunt_attempts;kills=$last.prey_hp_zero_after_attack;meals=$last.kills_with_meal;satiated_ticks=$last.satiated_predator_ticks;energy_gained=$last.hunt_energy;energy_basal=$last.energy_basal_spent;energy_movement=$last.energy_movement_spent;energy_reproduction=$last.energy_reproduction_spent;nutrient_error=$last.nutrient_error;energy_error=$last.energy_error;bite_stomach_energy=$last.bite_stomach_energy;bite_stomach_nutrients=$last.bite_stomach_nutrients;capacity_blocks=$last.bite_stomach_capacity_blocked;sustainable_birth_blocks=$last.sustainable_reproduction_blocked;emergency_digestion_ticks=$last.emergency_digestion_ticks;emergency_digestion_energy=$last.emergency_digestion_energy;death_stomach_energy=$last.predator_death_stomach_energy;death_stomach_nutrients=$last.predator_death_stomach_nutrients;deaths_with_stomach_energy=$last.predator_deaths_with_stomach_energy;low_reserve_with_stomach_ticks=$last.predator_low_reserve_with_stomach_ticks;starvation_with_stomach_energy=$last.predator_starvation_with_stomach_energy;hunger_override_ticks=$last.hunting_hunger_override_ticks;hunger_override_no_capacity_ticks=$last.hunting_hunger_override_no_capacity_ticks;hunger_override_attacks=$last.hunting_hunger_override_attacks}
  Write-Host "$mode seed $seed complete: P/H/C=$($last.plants)/$($last.herbivores)/$($last.carnivores)"
 }
 $summary | Export-Csv "$dir/summary.csv" -NoTypeInformation -Encoding UTF8
}
Write-Host 'Complete: results/phase06212'
