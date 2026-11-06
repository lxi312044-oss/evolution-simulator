//! Phase 07.2.12: compact, observation-only seed-level diagnostics.
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
            "usage: resistance_covariates_07212 config.json ticks interval output-prefix".into(),
        );
    }
    let cfg: SimulationConfig = serde_json::from_str(&std::fs::read_to_string(&args[1])?)?;
    let ticks: u64 = args[2].parse()?;
    let interval: u64 = args[3].parse()?;
    if ticks == 0 || interval == 0 {
        return Err("ticks and interval must be positive".into());
    }
    let mut w = World::new(cfg)?;
    let mut initial = BufWriter::new(File::create(format!("{}.initial-genetics.csv", args[4]))?);
    writeln!(initial,"seed,founder_count,mean_resistance,variance_resistance,min_resistance,max_resistance,bin0,bin1,bin2,bin3,bin4")?;
    let mut bins = [0u64; 5];
    let mut sum = 0f64;
    let mut squares = 0f64;
    let mut min = 1f64;
    let mut max = 0f64;
    for h in &w.animals.items {
        let v = h.genes.toxin_resistance as f64;
        sum += v;
        squares += v * v;
        min = min.min(v);
        max = max.max(v);
        bins[((v.clamp(0., 1.) * 5.) as usize).min(4)] += 1;
    }
    let n = w.animals.items.len();
    let mean = if n > 0 { sum / n as f64 } else { 0. };
    let var = if n > 0 {
        (squares / n as f64 - mean * mean).max(0.)
    } else {
        0.
    };
    if n == 0 {
        min = 0.;
    }
    writeln!(
        initial,
        "{},{},{:.12},{:.12},{:.12},{:.12},{},{},{},{},{}",
        w.config.seed, n, mean, var, min, max, bins[0], bins[1], bins[2], bins[3], bins[4]
    )?;
    let mut out = BufWriter::new(File::create(format!(
        "{}.population-pressure.csv",
        args[4]
    ))?);
    writeln!(out,"seed,tick,plants,herbivores,carnivores,herbivore_births_cumulative,herbivore_deaths_cumulative,herbivore_births_delta,herbivore_deaths_delta,carnivore_hunts_cumulative,hunt_attempts_cumulative,hunt_positive_energy_cumulative,prey_hp_zero_after_attack_cumulative,hunt_attempts_delta,hunt_positive_energy_delta,prey_hp_zero_after_attack_delta,toxin_exposure_events_cumulative,toxin_exposure_total_cumulative,herbivore_extinct")?;
    let mut old_birth = 0;
    let mut old_death = 0;
    let mut old_attempt = 0;
    let mut old_positive = 0;
    let mut old_hpzero = 0;
    for tick in 0..=ticks {
        if tick % interval == 0 || tick == ticks {
            let d = &w.diagnostics;
            let c = &w.carnivore_debug;
            let deaths = d.herbivore_age_deaths + d.herbivore_hp_deaths;
            writeln!(
                out,
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.12},{}",
                w.config.seed,
                tick,
                w.plants.items.len(),
                w.animals.items.len(),
                w.carnivores.len(),
                d.herbivore_births,
                deaths,
                d.herbivore_births.saturating_sub(old_birth),
                deaths.saturating_sub(old_death),
                w.carnivore_hunts,
                c.hunt_attempts,
                c.hunt_positive_energy,
                c.prey_hp_zero_after_attack,
                c.hunt_attempts.saturating_sub(old_attempt),
                c.hunt_positive_energy.saturating_sub(old_positive),
                c.prey_hp_zero_after_attack.saturating_sub(old_hpzero),
                d.toxin_exposure_events,
                d.toxin_exposure_total,
                w.animals.items.is_empty()
            )?;
            old_birth = d.herbivore_births;
            old_death = deaths;
            old_attempt = c.hunt_attempts;
            old_positive = c.hunt_positive_energy;
            old_hpzero = c.prey_hp_zero_after_attack;
        }
        if tick < ticks {
            w.step(1)?;
        }
    }
    Ok(())
}
