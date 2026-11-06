//! Phase 07.2.17 same-run resource and reproductive diagnostics. No simulation-rule changes.
use sim_core::{SimulationConfig, World};
use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 && args.len() != 10 {
        return Err("usage: resource_reproduction_07217 config.json ticks interval output.csv [off|on threshold probability check_interval max_births]".into());
    }
    let cfg: SimulationConfig = serde_json::from_str(&std::fs::read_to_string(&args[1])?)?;
    let ticks: u64 = args[2].parse()?;
    let interval: u64 = args[3].parse()?;
    if ticks == 0 || interval == 0 {
        return Err("ticks and interval must be positive".into());
    }
    let mut w = World::new(cfg)?;
    if args.len() == 10 {
        match args[5].as_str() {
            "off" => {}
            "on" | "p50" | "p100" | "p200" => {
                let g = sim_core::world::SpontaneousGermination {
                    soil_threshold: args[6].parse()?,
                    probability_per_cell: args[7].parse()?,
                    check_interval: args[8].parse()?,
                    max_births_per_check: args[9].parse()?,
                };
                g.validate()?;
                w.spontaneous_germination = Some(g);
            }
            _ => return Err("mode must be on or off".into()),
        }
    }
    w.resistance_event_logging = true;
    let mut out = BufWriter::new(File::create(&args[4])?);
    writeln!(out,"seed,tick,herbivores,age_lt_min,age_min_to_2min,age_ge_2min,cooldown_ready_snapshot,herbivore_births_delta,herbivore_reproduction_attempts_delta,phase24_repro_samples_delta,phase24_repro_age_ready_delta,phase24_repro_cooldown_ready_delta,phase24_repro_hp_ready_delta,phase24_repro_energy_ready_delta,phase24_repro_satiety_ready_delta,phase24_repro_body_n_ready_delta,phase24_repro_reserve_n_ready_delta,phase24_repro_transfer_e_ready_delta,phase24_repro_ready_all_delta,phase20_repro_age_blocked_delta,phase20_repro_cooldown_blocked_delta,phase20_repro_hp_blocked_delta,phase20_repro_energy_gate_blocked_delta,phase20_repro_satiety_blocked_delta,phase20_repro_food_blocked_delta,herbivore_reproduction_resource_blocked_delta,herbivore_reproduction_capacity_blocked_delta,herbivore_age_deaths_delta,herbivore_hp_deaths_delta,toxin_exposure_events_delta,plants,soil_n,detritus_n,plant_body_n,plant_reserve_n,plant_reserve_e,plant_births_delta,plant_age_deaths_delta,plant_consumed_deaths_delta,seed_attempts_delta,seed_resource_blocked_delta,seed_capacity_blocked_delta,plant_uptake_n_delta,plant_consumed_n_delta,local_plant_count_mean,local_plant_n_mean,local_plant_e_mean,spontaneous_births_delta,resistance_mean,resistance_variance,resistance_min,resistance_max,resistance_birth_mean,resistance_parent_birth_mean,resistance_mutation_attempts_delta,resistance_birth_events_delta,toxin_dose_cumulative,toxin_damage_cumulative,resistance_low_alive,resistance_mid_alive,resistance_high_alive,resistance_low_parent_births_delta,resistance_mid_parent_births_delta,resistance_high_parent_births_delta,resistance_low_damage_alive,resistance_mid_damage_alive,resistance_high_damage_alive")?;
    let mut event_prev = 0usize;
    let mut prev = [0u64; 23];
    let mut spontaneous_prev = 0u64;
    let mut plant_prev = [0u64; 6];
    let mut flow_prev = [0f64; 2];
    let mut local_prev = [0f64; 3];
    for tick in 0..=ticks {
        if tick % interval == 0 || tick == ticks {
            let min_age = w.config.herbivores.reproduction_min_age;
            let young = w.animals.items.iter().filter(|a| a.age < min_age).count();
            let mid = w
                .animals
                .items
                .iter()
                .filter(|a| a.age >= min_age && a.age < min_age.saturating_mul(2))
                .count();
            let old = w
                .animals
                .items
                .iter()
                .filter(|a| a.age >= min_age.saturating_mul(2))
                .count();
            let cooldown = w
                .animals
                .items
                .iter()
                .filter(|a| a.reproduction_cooldown == 0)
                .count();
            let d = &w.diagnostics;
            let current = [
                d.herbivore_births,
                d.herbivore_reproduction_attempts,
                d.phase24_repro_samples,
                d.phase24_repro_age_ready,
                d.phase24_repro_cooldown_ready,
                d.phase24_repro_hp_ready,
                d.phase24_repro_energy_ready,
                d.phase24_repro_satiety_ready,
                d.phase24_repro_body_n_ready,
                d.phase24_repro_reserve_n_ready,
                d.phase24_repro_transfer_e_ready,
                d.phase24_repro_ready_all,
                d.phase20_repro_age_blocked,
                d.phase20_repro_cooldown_blocked,
                d.phase20_repro_hp_blocked,
                d.phase20_repro_energy_gate_blocked,
                d.phase20_repro_satiety_blocked,
                d.phase20_repro_food_blocked,
                d.herbivore_reproduction_resource_blocked,
                d.herbivore_reproduction_capacity_blocked,
                d.herbivore_age_deaths,
                d.herbivore_hp_deaths,
                d.toxin_exposure_events,
            ];
            let delta: Vec<u64> = current
                .iter()
                .zip(prev.iter())
                .map(|(n, p)| n.saturating_sub(*p))
                .collect();
            let mut row = format!(
                "{},{},{},{},{},{},{}",
                w.config.seed,
                tick,
                w.animals.items.len(),
                young,
                mid,
                old,
                cooldown
            );
            for v in &delta {
                row.push_str(&format!(",{}", v));
            }
            let plant_current = [
                d.plant_births,
                d.plant_age_deaths,
                d.plant_consumed_deaths,
                d.seed_attempts,
                d.seed_resource_blocked,
                d.seed_capacity_blocked,
            ];
            let plant_delta: Vec<u64> = plant_current
                .iter()
                .zip(plant_prev.iter())
                .map(|(n, p)| n.saturating_sub(*p))
                .collect();
            let flow_current = [d.plant_uptake_n, d.plant_consumed_n];
            let flow_delta: Vec<f64> = flow_current
                .iter()
                .zip(flow_prev.iter())
                .map(|(n, p)| n - p)
                .collect();
            let local_current = [
                d.phase24_repro_local_plant_count_total as f64,
                d.phase24_repro_local_plant_n_total,
                d.phase24_repro_local_plant_e_total,
            ];
            let samples = delta[2] as f64;
            let local_mean: Vec<f64> = local_current
                .iter()
                .zip(local_prev.iter())
                .map(|(n, p)| {
                    if samples > 0. {
                        (n - p) / samples
                    } else {
                        f64::NAN
                    }
                })
                .collect();
            let plant_body_n: f64 = w
                .plants
                .items
                .iter()
                .map(|p| w.resources.nutrient_balance(p.body).unwrap_or(0.))
                .sum();
            let plant_reserve_n: f64 = w
                .plants
                .items
                .iter()
                .map(|p| w.resources.nutrient_balance(p.reserve).unwrap_or(0.))
                .sum();
            let plant_reserve_e: f64 = w
                .plants
                .items
                .iter()
                .map(|p| w.resources.energy_balance(p.reserve).unwrap_or(0.))
                .sum();
            row.push_str(&format!(
                ",{},{:.9},{:.9},{:.9},{:.9},{:.9}",
                w.plant_count(),
                w.total_soil(),
                w.total_detritus(),
                plant_body_n,
                plant_reserve_n,
                plant_reserve_e
            ));
            for v in &plant_delta {
                row.push_str(&format!(",{}", v));
            }
            for v in &flow_delta {
                row.push_str(&format!(",{:.9}", v));
            }
            for v in &local_mean {
                row.push_str(&format!(",{:.9}", v));
            }
            row.push_str(&format!(",{}", w.spontaneous_births - spontaneous_prev));
            spontaneous_prev = w.spontaneous_births;
            let n = w.animals.items.len();
            let values: Vec<f64> = w
                .animals
                .items
                .iter()
                .map(|a| a.genes.toxin_resistance as f64)
                .collect();
            let mean = if n > 0 {
                values.iter().sum::<f64>() / n as f64
            } else {
                f64::NAN
            };
            let variance = if n > 0 {
                values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64
            } else {
                f64::NAN
            };
            let minimum = values.iter().copied().reduce(f64::min).unwrap_or(f64::NAN);
            let maximum = values.iter().copied().reduce(f64::max).unwrap_or(f64::NAN);
            let events = &w.resistance_birth_events[event_prev..];
            let count = events.len();
            let birth_mean = if count > 0 {
                events
                    .iter()
                    .map(|e| e.child_resistance as f64)
                    .sum::<f64>()
                    / count as f64
            } else {
                f64::NAN
            };
            let parent_mean = if count > 0 {
                events
                    .iter()
                    .map(|e| e.parent_resistance as f64)
                    .sum::<f64>()
                    / count as f64
            } else {
                f64::NAN
            };
            let mutations = events.iter().filter(|e| e.mutation_attempted).count();
            let dose: f64 = w.animals.items.iter().map(|a| a.lifetime_toxin_dose).sum();
            let damage: f64 = w
                .animals
                .items
                .iter()
                .map(|a| a.lifetime_toxin_damage)
                .sum();
            row.push_str(&format!(",{mean:.9},{variance:.9},{minimum:.9},{maximum:.9},{birth_mean:.9},{parent_mean:.9},{mutations},{count},{dose:.9},{damage:.9}"));
            let mut alive = [0usize; 3];
            let mut damages = [0f64; 3];
            let mut parent_births = [0usize; 3];
            for a in &w.animals.items {
                let bin = if a.genes.toxin_resistance < 0.4 {
                    0
                } else if a.genes.toxin_resistance < 0.6 {
                    1
                } else {
                    2
                };
                alive[bin] += 1;
                damages[bin] += a.lifetime_toxin_damage;
            }
            for e in events {
                let bin = if e.parent_resistance < 0.4 {
                    0
                } else if e.parent_resistance < 0.6 {
                    1
                } else {
                    2
                };
                parent_births[bin] += 1;
            }
            write!(out, "{}", row)?;
            writeln!(
                out,
                ",{},{},{},{},{},{},{:.9},{:.9},{:.9}",
                alive[0],
                alive[1],
                alive[2],
                parent_births[0],
                parent_births[1],
                parent_births[2],
                damages[0],
                damages[1],
                damages[2]
            )?;
            event_prev = w.resistance_birth_events.len();
            prev = current;
            plant_prev = plant_current;
            flow_prev = flow_current;
            local_prev = local_current;
        }
        if tick < ticks {
            w.step(1)?;
        }
    }
    Ok(())
}
