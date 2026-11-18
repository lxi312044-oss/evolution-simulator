//! Issue #11: the stage 4 environment on a 40x40 field at the same density.
//!
//! Plan (fixed before the run) and its pre-run revision:
//! https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6970763006
//! https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6970805102
//! f40_E / f40_noFT_E (halved carnivore basal energy):
//! https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6971981987
//!
//! Usage:
//!   cargo run --release -p sim-core --example stage4_field40 -- \
//!     config/stage4-inherited-plant-colors.json <output-dir> [max-ticks] [arm,...]
//!
//! Runs up to `max-ticks` (default 40000) and stops after tick 20000 once carnivores or
//! herbivores are extinct. Only `step` changes the world. One JSON file per run.
use serde_json::{json, Value};
use sim_core::{SimulationConfig, World};
use std::{env, fs, path::Path};

const SEEDS: std::ops::RangeInclusive<u64> = 90001..=90008;
const JUDGE_TICK: u64 = 20_000;

struct Arm {
    name: &'static str,
    speed: f32,
    decay: f64,
    resume_fraction: f64,
    /// `None` keeps food-limited and prey-trend reproduction and basal energy from the config.
    brakes_and_basal: Option<(bool, bool, f64)>,
}

const ARMS: [Arm; 6] = [
    Arm {
        name: "f40_base",
        speed: 3.2,
        decay: 0.,
        resume_fraction: 0.4,
        brakes_and_basal: None,
    },
    Arm {
        name: "f40_decay",
        speed: 3.2,
        decay: 0.0007,
        resume_fraction: 0.4,
        brakes_and_basal: None,
    },
    Arm {
        name: "f40_v42_rest",
        speed: 4.2,
        decay: 0.0007,
        resume_fraction: 0.2,
        brakes_and_basal: None,
    },
    Arm {
        name: "f40_v525_rest",
        speed: 5.25,
        decay: 0.0007,
        resume_fraction: 0.2,
        brakes_and_basal: None,
    },
    // Round 3 settings with halved carnivore basal energy (20x20: w_E, w_noFT_E).
    Arm {
        name: "f40_E",
        speed: 5.25,
        decay: 0.0007,
        resume_fraction: 0.2,
        brakes_and_basal: Some((true, true, 0.0072)),
    },
    Arm {
        name: "f40_noFT_E",
        speed: 5.25,
        decay: 0.0007,
        resume_fraction: 0.2,
        brakes_and_basal: Some((false, false, 0.0072)),
    },
];

/// Scale a 20x20 config to 40x40 at the same density, like `web/src/field.ts`, and also
/// scale the population-count settings that the Web resize leaves unchanged.
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

/// Integer counters, in this order (same as `stage4_carnivore_review`).
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
            field40(&mut config);
            config.carnivores.speed = a.speed;
            config.herbivores.color_memory_decay_per_tick = a.decay;
            config.carnivores.feeding_resume_fraction = a.resume_fraction;
            if let Some((food_limited, prey_trend, basal)) = a.brakes_and_basal {
                config.carnivores.food_limited_reproduction_enabled = food_limited;
                config.carnivores.prey_trend_reproduction_enabled = prey_trend;
                config.carnivores.basal_energy_per_tick = basal;
            }
            let mut world = World::new(config.clone())?;
            let initial_plants = plant_classes(&world);
            let (mut prev_n, mut prev_e) = (counts(&world), energy(&world));
            let mut rows = Vec::new();
            let mut plants_at_judge = None;
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
                    de
                ]));
                if tick == JUDGE_TICK {
                    plants_at_judge = Some(plant_classes(&world));
                }
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
                "row_fields": ["tick", "carnivores", "herbivores", "plants", COUNTS, ENERGY],
                "initial_plants_gn_gt_pn_pt": initial_plants,
                "plants_at_judge_gn_gt_pn_pt": plants_at_judge,
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
