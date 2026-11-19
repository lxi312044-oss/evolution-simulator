//! Issue #11: why do carnivores in the trough fail to store energy? Measurement only.
//!
//! Plan (fixed before the run):
//! https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6973191538
//!
//! Same settings as `stage4_field40` arm f40_E. sim-core is not changed: this example
//! only reads existing diagnostic counters and entity state every 100 ticks. Only `step`
//! changes the world.
//!
//! Usage:
//!   cargo run --release -p sim-core --example stage4_trough_budget -- \
//!     config/stage4-inherited-plant-colors.json <output-dir> [max-ticks]
use serde_json::json;
use sim_core::{SimulationConfig, World};
use std::{env, fs, path::Path};

const SEEDS: std::ops::RangeInclusive<u64> = 90001..=90008;

/// Integer counters read from `carnivore_debug`, in this order.
const COUNTS: [&str; 18] = [
    "phase17_rest_ticks",
    "phase17_active_search_ticks",
    "phase17_chase_ticks",
    "satiated_predator_ticks",
    "stomach_feeding_blocked_ticks",
    "p41_visible_samples",
    "prey_in_attack_range",
    "hunt_attempts",
    "hunt_contacts",
    "hunt_positive_energy",
    "prey_hp_zero_after_attack",
    "bite_stomach_capacity_blocked",
    "carnivore_energy_deaths",
    "carnivore_age_deaths",
    "reproduction_energy_blocked",
    "reproduction_cooldown_blocked",
    "reproduction_ready",
    "hunting_hunger_override_ticks",
];
/// Float counters read from `carnivore_debug`, in this order.
const AMOUNTS: [&str; 8] = [
    "hunt_energy",
    "stomach_energy_ingested",
    "stomach_energy_digested",
    "stomach_nutrients_ingested",
    "energy_basal_spent",
    "energy_movement_spent",
    "energy_reproduction_spent",
    "predator_movement_distance",
];
/// Snapshot of living carnivores, in this order.
const SNAPSHOT: [&str; 7] = [
    "mean_reserve_energy",
    "max_reserve_energy",
    "mean_stomach_energy",
    "mean_stomach_nutrients",
    "feeding_active_fraction",
    "mean_nearest_herbivore_distance",
    "mean_herbivores_within_vision",
];

fn field40(config: &mut SimulationConfig) {
    let w = &mut config.world;
    w.width *= 2.;
    w.height *= 2.;
    w.land_columns *= 2;
    w.land_rows *= 2;
    w.max_living_entities *= 4;
    let p = &mut config.initial_population;
    p.plants *= 4;
    p.herbivores *= 4;
    p.carnivores *= 4;
    config.herbivores.plant_protection_threshold *= 4;
}

fn counts(w: &World) -> Vec<u64> {
    let d = &w.carnivore_debug;
    vec![
        d.phase17_rest_ticks,
        d.phase17_active_search_ticks,
        d.phase17_chase_ticks,
        d.satiated_predator_ticks,
        d.stomach_feeding_blocked_ticks,
        d.p41_visible_samples,
        d.prey_in_attack_range,
        d.hunt_attempts,
        d.hunt_contacts,
        d.hunt_positive_energy,
        d.prey_hp_zero_after_attack,
        d.bite_stomach_capacity_blocked,
        d.carnivore_energy_deaths,
        d.carnivore_age_deaths,
        d.reproduction_energy_blocked,
        d.reproduction_cooldown_blocked,
        d.reproduction_ready,
        d.hunting_hunger_override_ticks,
    ]
}

fn amounts(w: &World) -> Vec<f64> {
    let d = &w.carnivore_debug;
    vec![
        d.hunt_energy,
        d.stomach_energy_ingested,
        d.stomach_energy_digested,
        d.stomach_nutrients_ingested,
        d.energy_basal_spent,
        d.energy_movement_spent,
        d.energy_reproduction_spent,
        d.predator_movement_distance,
    ]
}

fn snapshot(w: &World) -> Vec<f64> {
    let n = w.carnivores.len();
    if n == 0 {
        return vec![0.; SNAPSHOT.len()];
    }
    let (mut re, mut rmax, mut se, mut sn, mut fa, mut dist, mut within) =
        (0., 0f64, 0., 0., 0., 0., 0.);
    for c in &w.carnivores {
        let e = w.resources.energy_balance(c.reserve).unwrap_or(0.);
        re += e;
        rmax = rmax.max(e);
        se += w.resources.energy_balance(c.stomach).unwrap_or(0.);
        sn += w.resources.nutrient_balance(c.stomach).unwrap_or(0.);
        fa += f64::from(u8::from(c.feeding_active));
        let mut best = f32::INFINITY;
        for h in &w.animals.items {
            let d = ((h.x - c.x).powi(2) + (h.y - c.y).powi(2)).sqrt();
            best = best.min(d);
            if d < c.vision {
                within += 1.;
            }
        }
        if best.is_finite() {
            dist += f64::from(best);
        }
    }
    let k = n as f64;
    vec![re / k, rmax, se / k, sn / k, fa / k, dist / k, within / k]
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let config_path = args.get(1).ok_or("config path is required")?;
    let out = Path::new(args.get(2).ok_or("output directory is required")?);
    let max_ticks: u64 = args
        .get(3)
        .map_or(Ok(20_000), |t| t.parse())
        .map_err(|e| format!("{e}"))?;
    let text = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    for seed in SEEDS {
        let mut config: SimulationConfig =
            serde_json::from_str(&text).map_err(|e| e.to_string())?;
        config.seed = seed;
        field40(&mut config);
        // f40_E
        config.carnivores.speed = 5.25;
        config.herbivores.color_memory_decay_per_tick = 0.0007;
        config.carnivores.feeding_resume_fraction = 0.2;
        config.carnivores.food_limited_reproduction_enabled = true;
        config.carnivores.prey_trend_reproduction_enabled = true;
        config.carnivores.basal_energy_per_tick = 0.0072;
        let mut world = World::new(config.clone())?;
        let (mut prev_n, mut prev_a) = (counts(&world), amounts(&world));
        let mut rows = Vec::new();
        let mut tick = 0;
        while tick < max_ticks && !world.carnivores.is_empty() {
            world.step(100)?;
            tick += 100;
            let (now_n, now_a) = (counts(&world), amounts(&world));
            let dn: Vec<u64> = now_n.iter().zip(&prev_n).map(|(a, b)| a - b).collect();
            let da: Vec<f64> = now_a.iter().zip(&prev_a).map(|(a, b)| a - b).collect();
            (prev_n, prev_a) = (now_n, now_a);
            rows.push(json!([
                tick,
                world.carnivores.len(),
                world.herbivore_count(),
                world.plants.items.len(),
                dn,
                da,
                snapshot(&world)
            ]));
        }
        let record = json!({
            "issue": 11, "arm": "f40_E", "seed": seed, "max_ticks": max_ticks,
            "last_tick": tick, "effective_config": config,
            "row_fields": ["tick", "carnivores", "herbivores", "plants", COUNTS, AMOUNTS, SNAPSHOT],
            "rows": rows,
        });
        fs::write(out.join(format!("trough_s{seed}.json")), record.to_string())
            .map_err(|e| e.to_string())?;
        println!(
            "seed={seed} last_tick={tick} carnivores={} herbivores={}",
            world.carnivores.len(),
            world.herbivore_count()
        );
    }
    Ok(())
}
