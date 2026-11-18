//! Issue #11: pre-registered review of carnivore settings in the fixed stage 4 environment.
//!
//! Plan (fixed before the run):
//! https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6968323277
//!
//! Usage:
//!   cargo run --release -p sim-core --example stage4_carnivore_review -- \
//!     config/stage4-inherited-plant-colors.json <output-dir> [ticks] [arm,...]
//!
//! Only `step` changes the world; counters are read between steps. One JSON file per run.
use serde_json::{json, Value};
use sim_core::{SimulationConfig, World};
use std::{env, fs, path::Path};

const SEEDS: std::ops::RangeInclusive<u64> = 90001..=90016;
/// (name, carnivore speed, herbivore color memory decay per tick)
const ARMS: [(&str, f32, f64); 6] = [
    ("s32", 3.2, 0.),
    ("s42", 4.2, 0.),
    ("s525", 5.25, 0.),
    ("s32_decay", 3.2, 0.0007),
    ("s42_decay", 4.2, 0.0007),
    ("s525_decay", 5.25, 0.0007),
];

/// Integer counters, in this order.
const COUNTS: [&str; 13] = [
    "carnivore_births",
    "carnivore_energy_deaths",
    "carnivore_age_deaths",
    "carnivore_toxin_deaths",
    "hunt_attempts",
    "kills_with_meal",
    "color_skips",
    "learned_attacks",
    "learned_feeds",
    "hunger_reapproach_opportunities",
    "herbivore_births",
    "herbivore_age_deaths",
    "herbivore_hp_deaths",
];
/// Carnivore energy flows (E), in this order.
const ENERGY: [&str; 4] = [
    "stomach_energy_digested",
    "energy_basal_spent",
    "energy_movement_spent",
    "energy_reproduction_spent",
];

fn counts(world: &World) -> Vec<u64> {
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
        t("color_skips"),
        t("learned_attacks"),
        t("learned_feeds"),
        t("hunger_reapproach_opportunities"),
        world.diagnostics.herbivore_births,
        world.diagnostics.herbivore_age_deaths,
        world.diagnostics.herbivore_hp_deaths,
    ]
}

fn energy(world: &World) -> Vec<f64> {
    let d = &world.carnivore_debug;
    vec![
        d.stomach_energy_digested,
        d.energy_basal_spent,
        d.energy_movement_spent,
        d.energy_reproduction_spent,
    ]
}

fn plant_classes(world: &World) -> [u64; 4] {
    // Gn, Gt, Pn, Pt
    let mut n = [0; 4];
    for p in &world.plants.items {
        let purple = p.color == 0xb86be3ff;
        n[usize::from(purple) * 2 + usize::from(p.produces_toxin)] += 1;
    }
    n
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let config_path = args.get(1).ok_or("config path is required")?;
    let out = Path::new(args.get(2).ok_or("output directory is required")?);
    let ticks: u64 = args
        .get(3)
        .map_or(Ok(20_000), |t| t.parse())
        .map_err(|e| format!("{e}"))?;
    let names: Vec<&str> = ARMS.iter().map(|a| a.0).collect();
    let selected: Vec<&str> = args
        .get(4)
        .map_or(names.clone(), |a| a.split(',').collect());
    if let Some(bad) = selected.iter().find(|n| !names.contains(n)) {
        return Err(format!("unknown arm: {bad}"));
    }
    let text = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    for (arm, speed, decay) in ARMS.iter().filter(|a| selected.contains(&a.0)) {
        for seed in SEEDS {
            let mut config: SimulationConfig =
                serde_json::from_str(&text).map_err(|e| e.to_string())?;
            config.seed = seed;
            config.carnivores.speed = *speed;
            config.herbivores.color_memory_decay_per_tick = *decay;
            let mut world = World::new(config.clone())?;
            let initial_plants = plant_classes(&world);
            let (mut prev_n, mut prev_e) = (counts(&world), energy(&world));
            let mut rows = Vec::new();
            let mut tick = 0;
            while tick < ticks {
                world.step(100)?;
                tick += 100;
                let (now_n, now_e) = (counts(&world), energy(&world));
                let dn: Vec<u64> = now_n.iter().zip(&prev_n).map(|(a, b)| a - b).collect();
                let de: Vec<f64> = now_e.iter().zip(&prev_e).map(|(a, b)| a - b).collect();
                (prev_n, prev_e) = (now_n, now_e);
                rows.push(json!([
                    tick,
                    world.carnivores.len(),
                    world.herbivore_count(),
                    world.plants.items.len(),
                    dn,
                    de
                ]));
            }
            let record = json!({
                "issue": 11, "arm": arm, "seed": seed, "ticks": ticks,
                "carnivore_speed": speed, "herbivore_decay": decay,
                "effective_config": config,
                "row_fields": ["tick", "carnivores", "herbivores", "plants", COUNTS, ENERGY],
                "initial_plants_gn_gt_pn_pt": initial_plants,
                "final_plants_gn_gt_pn_pt": plant_classes(&world),
                "rows": rows,
            });
            fs::write(out.join(format!("{arm}_s{seed}.json")), record.to_string())
                .map_err(|e| e.to_string())?;
            let alive = |i: usize| rows.last().is_some_and(|r| r[i] != 0);
            println!(
                "{arm} seed={seed} end carnivores={} herbivores={} plants={}",
                alive(1),
                alive(2),
                alive(3)
            );
        }
    }
    Ok(())
}
