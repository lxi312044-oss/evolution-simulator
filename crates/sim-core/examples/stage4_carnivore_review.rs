//! Issue #11: pre-registered review of carnivore settings in the fixed stage 4 environment.
//!
//! Plans (fixed before each run):
//! round 1 (s* arms): https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6968323277
//! round 2 (v* arms): https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6968964204
//! round 3 (w* arms, 40000 ticks): https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6971350554
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
/// One compared setting. `None` keeps the value from the base config.
struct Arm {
    name: &'static str,
    speed: f32,
    decay: f64,
    sustainable: Option<bool>,
    resume_fraction: Option<f64>,
    food_limited: Option<bool>,
    prey_trend: Option<bool>,
    basal: Option<f64>,
}

const fn arm(name: &'static str, speed: f32, decay: f64) -> Arm {
    Arm {
        name,
        speed,
        decay,
        sustainable: None,
        resume_fraction: None,
        food_limited: None,
        prey_trend: None,
        basal: None,
    }
}

const fn round2(name: &'static str, speed: f32, sustainable: bool, resume: f64) -> Arm {
    Arm {
        name,
        speed,
        decay: 0.0007,
        sustainable: Some(sustainable),
        resume_fraction: Some(resume),
        food_limited: None,
        prey_trend: None,
        basal: None,
    }
}

/// Round 3: speed 5.25, resume 0.2 and herbivore decay 0.0007 (round 2 v525_rest), with
/// the carnivore reproduction brakes and basal metabolism varied.
const fn round3(name: &'static str, food_limited: bool, prey_trend: bool, basal: f64) -> Arm {
    Arm {
        name,
        speed: 5.25,
        decay: 0.0007,
        sustainable: Some(false),
        resume_fraction: Some(0.2),
        food_limited: Some(food_limited),
        prey_trend: Some(prey_trend),
        basal: Some(basal),
    }
}

const ARMS: [Arm; 22] = [
    // Round 1: carnivore speed x herbivore color memory decay.
    arm("s32", 3.2, 0.),
    arm("s42", 4.2, 0.),
    arm("s525", 5.25, 0.),
    arm("s32_decay", 3.2, 0.0007),
    arm("s42_decay", 4.2, 0.0007),
    arm("s525_decay", 5.25, 0.0007),
    // Round 2: sustainable reproduction (minimum prey per predator) and a later
    // hunting restart after a meal (feeding_resume_fraction 0.4 -> 0.2).
    round2("v42", 4.2, false, 0.4),
    round2("v42_sustain", 4.2, true, 0.4),
    round2("v42_rest", 4.2, false, 0.2),
    round2("v42_both", 4.2, true, 0.2),
    round2("v525", 5.25, false, 0.4),
    round2("v525_sustain", 5.25, true, 0.4),
    round2("v525_rest", 5.25, false, 0.2),
    round2("v525_both", 5.25, true, 0.2),
    // Round 3: F = food-limited reproduction, T = prey-trend reproduction penalty,
    // E = carnivore basal energy per tick (0.0144 -> 0.0072).
    round3("w_base", true, true, 0.0144),
    round3("w_noF", false, true, 0.0144),
    round3("w_noT", true, false, 0.0144),
    round3("w_noFT", false, false, 0.0144),
    round3("w_E", true, true, 0.0072),
    round3("w_noF_E", false, true, 0.0072),
    round3("w_noT_E", true, false, 0.0072),
    round3("w_noFT_E", false, false, 0.0072),
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
        let arm = a.name;
        for seed in SEEDS {
            let mut config: SimulationConfig =
                serde_json::from_str(&text).map_err(|e| e.to_string())?;
            config.seed = seed;
            config.carnivores.speed = a.speed;
            config.herbivores.color_memory_decay_per_tick = a.decay;
            if let Some(on) = a.sustainable {
                config.carnivores.sustainable_reproduction_enabled = on;
            }
            if let Some(fraction) = a.resume_fraction {
                config.carnivores.feeding_resume_fraction = fraction;
            }
            if let Some(on) = a.food_limited {
                config.carnivores.food_limited_reproduction_enabled = on;
            }
            if let Some(on) = a.prey_trend {
                config.carnivores.prey_trend_reproduction_enabled = on;
            }
            if let Some(basal) = a.basal {
                config.carnivores.basal_energy_per_tick = basal;
            }
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
                "carnivore_speed": a.speed, "herbivore_decay": a.decay,
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
