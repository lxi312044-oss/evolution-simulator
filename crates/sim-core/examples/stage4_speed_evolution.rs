//! Issue #11: heritable carnivore speed with mutation (round 3 w_E settings, 20x20).
//!
//! Plan (fixed before the run):
//! https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6973653252
//!
//! Usage:
//!   cargo run --release -p sim-core --example stage4_speed_evolution -- \
//!     config/stage4-inherited-plant-colors.json <output-dir> [max-ticks] [arm,...]
//!
//! Runs up to `max-ticks` (default 40000) and stops after tick 20000 once carnivores or
//! herbivores are extinct. Rows use the `stage4_carnivore_review` layout plus speed
//! statistics. Only `step` changes the world.
use serde_json::{json, Value};
use sim_core::{SimulationConfig, World};
use std::{env, fs, path::Path};

const SEEDS: std::ops::RangeInclusive<u64> = 90001..=90016;
const JUDGE_TICK: u64 = 20_000;

struct Arm {
    name: &'static str,
    spread: f32,
    mutation_probability: f32,
    mutation_scale: f32,
}

const ARMS: [Arm; 3] = [
    Arm {
        name: "e_ctrl",
        spread: 0.,
        mutation_probability: 0.,
        mutation_scale: 0.,
    },
    Arm {
        name: "e_mut",
        spread: 0.,
        mutation_probability: 0.05,
        mutation_scale: 0.1,
    },
    Arm {
        name: "e_var",
        spread: 0.25,
        mutation_probability: 0.05,
        mutation_scale: 0.1,
    },
];

/// Speed statistics, in this order.
const SPEED: [&str; 5] = [
    "carnivore_speed_mean",
    "carnivore_speed_min",
    "carnivore_speed_max",
    "herbivore_speed_mean",
    "herbivores_fleeing_faster_than_mean_carnivore",
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

fn speeds(world: &World, flee_multiplier: f32) -> Vec<f64> {
    let n = world.carnivores.len();
    if n == 0 {
        return vec![f64::NAN; SPEED.len()];
    }
    let s: Vec<f32> = world.carnivores.iter().map(|c| c.speed).collect();
    let mean = s.iter().sum::<f32>() / n as f32;
    let min = s.iter().copied().fold(f32::INFINITY, f32::min);
    let max = s.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let h: Vec<f32> = world.animals.items.iter().map(|a| a.genes.speed).collect();
    let (hmean, faster) = if h.is_empty() {
        (f64::NAN, f64::NAN)
    } else {
        let k = h.len() as f64;
        (
            h.iter().map(|v| f64::from(*v)).sum::<f64>() / k,
            h.iter().filter(|v| **v * flee_multiplier > mean).count() as f64 / k,
        )
    };
    vec![
        f64::from(mean),
        f64::from(min),
        f64::from(max),
        hmean,
        faster,
    ]
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let config_path = args.get(1).ok_or("config path is required")?;
    let out = Path::new(args.get(2).ok_or("output directory is required")?);
    let max_ticks: u64 = args
        .get(3)
        .map_or(Ok(40_000), |t| t.parse())
        .map_err(|e| format!("{e}"))?;
    let names: Vec<&str> = ARMS.iter().map(|a| a.name).collect();
    let selected: Vec<&str> = args
        .get(4)
        .map_or(names.clone(), |a| a.split(',').collect());
    if let Some(bad) = selected.iter().find(|n| !names.contains(n)) {
        return Err(format!("unknown arm: {bad}"));
    }
    let text = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    for a in ARMS.iter().filter(|a| selected.contains(&a.name)) {
        for seed in SEEDS {
            let mut config: SimulationConfig =
                serde_json::from_str(&text).map_err(|e| e.to_string())?;
            config.seed = seed;
            // Round 3 w_E.
            config.carnivores.speed = 5.25;
            config.herbivores.color_memory_decay_per_tick = 0.0007;
            config.carnivores.sustainable_reproduction_enabled = false;
            config.carnivores.feeding_resume_fraction = 0.2;
            config.carnivores.food_limited_reproduction_enabled = true;
            config.carnivores.prey_trend_reproduction_enabled = true;
            config.carnivores.basal_energy_per_tick = 0.0072;
            // Speed evolution.
            config.carnivores.speed_evolution_enabled = true;
            config.carnivores.initial_speed_spread = a.spread;
            config.carnivores.speed_mutation_probability = a.mutation_probability;
            config.carnivores.speed_mutation_scale = a.mutation_scale;
            let flee = config.herbivores.predator_flee_speed_multiplier;
            let mut world = World::new(config.clone())?;
            let initial_speeds = speeds(&world, flee);
            let (mut prev_n, mut prev_e) = (counts(&world), energy(&world));
            let mut rows = Vec::new();
            let mut tick = 0;
            while tick < max_ticks {
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
                    de,
                    speeds(&world, flee)
                ]));
                if tick >= JUDGE_TICK
                    && (world.carnivores.is_empty() || world.herbivore_count() == 0)
                {
                    break;
                }
            }
            let record = json!({
                "issue": 11, "arm": a.name, "seed": seed, "max_ticks": max_ticks,
                "last_tick": tick, "judge_tick": JUDGE_TICK,
                "effective_config": config,
                "row_fields": ["tick", "carnivores", "herbivores", "plants", COUNTS, ENERGY, SPEED],
                "initial_speeds": initial_speeds,
                "final_plants_gn_gt_pn_pt": plant_classes(&world),
                "rows": rows,
            });
            fs::write(
                out.join(format!("{}_s{seed}.json", a.name)),
                record.to_string(),
            )
            .map_err(|e| e.to_string())?;
            println!(
                "{} seed={seed} last_tick={tick} carnivores={} herbivores={} plants={}",
                a.name,
                world.carnivores.len(),
                world.herbivore_count(),
                world.plants.items.len()
            );
        }
    }
    Ok(())
}
