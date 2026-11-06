//! Phase 07.2.15 observation-only reproductive diagnostics. No simulation-rule changes.
use sim_core::{SimulationConfig, World};
use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        return Err(
            "usage: resistance_reproduction_07215 config.json ticks interval output.csv".into(),
        );
    }
    let cfg: SimulationConfig = serde_json::from_str(&std::fs::read_to_string(&args[1])?)?;
    let ticks: u64 = args[2].parse()?;
    let interval: u64 = args[3].parse()?;
    if ticks == 0 || interval == 0 {
        return Err("ticks and interval must be positive".into());
    }
    let mut w = World::new(cfg)?;
    let mut out = BufWriter::new(File::create(&args[4])?);
    writeln!(out,"seed,tick,herbivores,age_lt_min,age_min_to_2min,age_ge_2min,cooldown_ready_snapshot,herbivore_births_delta,herbivore_reproduction_attempts_delta,phase24_repro_samples_delta,phase24_repro_age_ready_delta,phase24_repro_cooldown_ready_delta,phase24_repro_hp_ready_delta,phase24_repro_energy_ready_delta,phase24_repro_satiety_ready_delta,phase24_repro_body_n_ready_delta,phase24_repro_reserve_n_ready_delta,phase24_repro_transfer_e_ready_delta,phase24_repro_ready_all_delta,phase20_repro_age_blocked_delta,phase20_repro_cooldown_blocked_delta,phase20_repro_hp_blocked_delta,phase20_repro_energy_gate_blocked_delta,phase20_repro_satiety_blocked_delta,phase20_repro_food_blocked_delta,herbivore_reproduction_resource_blocked_delta,herbivore_reproduction_capacity_blocked_delta,herbivore_age_deaths_delta,herbivore_hp_deaths_delta,toxin_exposure_events_delta")?;
    let mut prev = [0u64; 23];
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
            writeln!(out, "{}", row)?;
            prev = current;
        }
        if tick < ticks {
            w.step(1)?;
        }
    }
    Ok(())
}
