//! Issue #11 (ECO-CARNIVORE): read-only diagnosis of carnivore extinction in the
//! fixed stage 4 environment.
//!
//! Plan (fixed before the run):
//! https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6967911307
//!
//! Usage:
//!   cargo run --release -p sim-core --example eco_carnivore_extinction -- \
//!     config/stage4-inherited-plant-colors.json <output-dir> [ticks] [arm,...]
//!
//! Only `step` changes the world; counters are read between steps. One JSON file per run.
use serde_json::{json, Value};
use sim_core::{SimulationConfig, World};
use std::{env, fs, path::Path};

const SEEDS: std::ops::RangeInclusive<u64> = 90001..=90016;
const ARMS: [&str; 5] = [
    "base",
    "carn_learn_off",
    "herb_decay",
    "herb_learn_off",
    "animal_toxin_off",
];

/// Cumulative counters, in the order of `FIELDS`.
const FIELDS: [&str; 26] = [
    "carnivore_births",
    "carnivore_energy_deaths",
    "carnivore_age_deaths",
    "carnivore_toxin_deaths",
    "hunt_attempts",
    "kills_with_meal",
    "hunt_positive_energy",
    "prey_visible_while_low_energy",
    "predator_low_energy_no_visible_prey",
    "color_skips",
    "hunger_reapproach_opportunities",
    "hunger_reapproaches",
    "learned_attacks",
    "learned_feeds",
    "reproduction_energy_blocked",
    "reproduction_cooldown_blocked",
    "reproduction_age_blocked",
    "reproduction_body_n_blocked",
    "reproduction_reserve_n_blocked",
    "reproduction_capacity_blocked",
    "reproduction_food_blocked",
    "reproduction_trend_blocked",
    "stomach_reproduction_blocked",
    "sustainable_reproduction_blocked",
    "predator_death_nearest_prey_samples",
    "predator_death_distance_to_nearest_prey_total_x100",
];

fn arm_config(base: &str, arm: &str, seed: u64) -> Result<SimulationConfig, String> {
    let mut c: SimulationConfig = serde_json::from_str(base).map_err(|e| e.to_string())?;
    c.seed = seed;
    match arm {
        "base" => {}
        "carn_learn_off" => c.animal_toxin.color_learning_enabled = false,
        "herb_decay" => c.herbivores.color_memory_decay_per_tick = 0.0007,
        "herb_learn_off" => c.herbivores.color_learning_enabled = false,
        "animal_toxin_off" => {
            c.animal_toxin.enabled = false;
            c.animal_toxin.color_learning_enabled = false;
        }
        other => return Err(format!("unknown arm: {other}")),
    }
    Ok(c)
}

fn counters(world: &World) -> Vec<u64> {
    let d = &world.carnivore_debug;
    let toxin: Value = serde_json::from_str(&world.animal_toxin_snapshot_json()).unwrap();
    let t = |key: &str| toxin[key].as_u64().unwrap_or(0);
    vec![
        world.carnivore_births,
        d.carnivore_energy_deaths,
        d.carnivore_age_deaths,
        t("toxin_deaths"),
        d.hunt_attempts,
        d.kills_with_meal,
        d.hunt_positive_energy,
        d.prey_visible_while_low_energy,
        d.predator_low_energy_no_visible_prey,
        t("color_skips"),
        t("hunger_reapproach_opportunities"),
        t("hunger_reapproaches"),
        t("learned_attacks"),
        t("learned_feeds"),
        d.reproduction_energy_blocked,
        d.reproduction_cooldown_blocked,
        d.reproduction_age_blocked,
        d.reproduction_body_n_blocked,
        d.reproduction_reserve_n_blocked,
        d.reproduction_capacity_blocked,
        d.reproduction_food_blocked,
        d.reproduction_trend_blocked,
        d.stomach_reproduction_blocked,
        d.sustainable_reproduction_blocked,
        d.predator_death_nearest_prey_samples,
        (d.predator_death_distance_to_nearest_prey_total * 100.).round() as u64,
    ]
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let config_path = args.get(1).ok_or("config path is required")?;
    let out = Path::new(args.get(2).ok_or("output directory is required")?);
    let ticks: u64 = args
        .get(3)
        .map_or(Ok(20_000), |t| t.parse())
        .map_err(|e| format!("{e}"))?;
    let selected: Vec<&str> = args
        .get(4)
        .map_or(ARMS.to_vec(), |a| a.split(',').collect());
    if let Some(bad) = selected.iter().find(|n| !ARMS.contains(n)) {
        return Err(format!("unknown arm: {bad}"));
    }
    let text = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    for arm in ARMS.iter().filter(|a| selected.contains(a)) {
        for seed in SEEDS {
            let config = arm_config(&text, arm, seed)?;
            let mut world = World::new(config.clone())?;
            let mut previous = counters(&world);
            let mut rows = Vec::new();
            let mut tick = 0;
            while tick < ticks {
                world.step(100)?;
                tick += 100;
                let now = counters(&world);
                let delta: Vec<u64> = now.iter().zip(&previous).map(|(a, b)| a - b).collect();
                previous = now;
                rows.push(json!([
                    tick,
                    world.carnivores.len(),
                    world.herbivore_count(),
                    world.plants.items.len(),
                    delta
                ]));
            }
            let record = json!({
                "issue": 11, "arm": arm, "seed": seed, "ticks": ticks,
                "effective_config": config,
                "row_fields": ["tick", "carnivores", "herbivores", "plants", FIELDS],
                "rows": rows,
            });
            fs::write(out.join(format!("{arm}_s{seed}.json")), record.to_string())
                .map_err(|e| e.to_string())?;
            let extinct = rows.iter().find(|r| r[1] == 0).map(|r| r[0].clone());
            println!("{arm} seed={seed} carnivore_extinct={extinct:?}");
        }
    }
    Ok(())
}
