//! Issue #2 (ECO-001): compare opt-in plant-color memory updates.
//!
//! Plan (fixed before the run):
//! https://github.com/lxi312044-oss/evolution-simulator/issues/2#issuecomment-6967338200
//!
//! Usage:
//!   cargo run --release -p sim-core --example eco001_color_memory_update -- \
//!     config/stage4-inherited-plant-colors.json results/eco001-color-memory-update [ticks]
//!
//! Observation is read-only between `step` calls. One JSON file is written per run.
use serde_json::{json, Value};
use sim_core::{SimulationConfig, World};
use std::{env, fs, path::Path};

const SEEDS: std::ops::RangeInclusive<u64> = 90001..=90016;
const PURPLE: [f64; 3] = [184. / 255., 107. / 255., 227. / 255.];
const GREEN: [f64; 3] = [78. / 255., 203. / 255., 112. / 255.];
/// (name, color learning, safe-meal relief, decay per tick)
const ARMS: [(&str, bool, f64, f64); 5] = [
    ("on", true, 0., 0.),
    ("off", false, 0., 0.),
    ("relief", true, 0.05, 0.),
    ("decay", true, 0., 0.0007),
    ("both", true, 0.05, 0.0007),
];

fn distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// Learned risk for a color, using the same generalization as `PlantColorMemory::allows`.
fn risk(memory: &Value, color: &[f64]) -> f64 {
    memory.as_array().map_or(0., |entries| {
        entries
            .iter()
            .map(|m| {
                let rgb: Vec<f64> = m["rgb"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap())
                    .collect();
                m["risk_same_color"].as_f64().unwrap() * (1. - distance(&rgb, color) / 0.25).max(0.)
            })
            .fold(0., f64::max)
    })
}

fn counters(world: &World) -> [u64; 9] {
    let d = &world.diagnostics;
    [
        d.herbivore_births,
        d.herbivore_age_deaths,
        d.herbivore_hp_deaths,
        d.phase19_hp_zero_energy_deaths,
        d.toxic_plant_consumed_deaths,
        d.nontoxic_plant_consumed_deaths,
        d.toxic_plant_age_deaths,
        d.nontoxic_plant_age_deaths,
        d.herbivore_feeding_events,
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

fn herbivore_snapshot(world: &World) -> Value {
    let (mut total, mut ready, mut both, mut purple, mut green) = (0, 0, 0, 0, 0);
    for a in &world.animals.items {
        let view = world.inspect_entity(a.id.index, a.id.generation);
        let fraction =
            view["reserve_e"].as_f64().unwrap() / view["reserve_e_max"].as_f64().unwrap();
        let (p, g) = (
            risk(&view["memory"], &PURPLE),
            risk(&view["memory"], &GREEN),
        );
        total += 1;
        ready += u64::from(fraction >= 0.8);
        purple += u64::from(p >= 0.5);
        green += u64::from(g >= 0.5);
        both += u64::from(p >= 0.5 && g >= 0.5);
    }
    json!({"herbivores":total,"reserve_ready":ready,"risky_both":both,"risky_purple":purple,"risky_green":green})
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let config_path = args.get(1).ok_or("config path is required")?;
    let out = Path::new(args.get(2).ok_or("output directory is required")?);
    let ticks: u64 = args
        .get(3)
        .map_or(Ok(20_000), |t| t.parse())
        .map_err(|e| format!("{e}"))?;
    let text = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    for (arm, learning, relief, decay) in ARMS {
        for seed in SEEDS {
            let mut config: SimulationConfig =
                serde_json::from_str(&text).map_err(|e| e.to_string())?;
            config.seed = seed;
            config.herbivores.color_learning_enabled = learning;
            config.herbivores.color_memory_safe_meal_relief = relief;
            config.herbivores.color_memory_decay_per_tick = decay;
            let mut world = World::new(config.clone())?;
            let mut previous = counters(&world);
            let mut rows = Vec::new();
            let mut snapshots = Vec::new();
            let initial_plants = plant_classes(&world);
            let mut tick = 0;
            while tick < ticks {
                world.step(100)?;
                tick += 100;
                let now = counters(&world);
                let delta: Vec<u64> = now.iter().zip(previous).map(|(a, b)| a - b).collect();
                previous = now;
                rows.push(json!([
                    tick,
                    world.herbivore_count(),
                    world.carnivores.len(),
                    world.plants.items.len(),
                    delta
                ]));
                if tick % 500 == 0 {
                    let mut s = herbivore_snapshot(&world);
                    s["tick"] = json!(tick);
                    s["plants"] = json!(plant_classes(&world));
                    snapshots.push(s);
                }
            }
            let record = json!({
                "issue": 2, "arm": arm, "seed": seed, "ticks": ticks,
                "learning": learning, "relief": relief, "decay": decay,
                "effective_config": config,
                "row_fields": ["tick","herbivores","carnivores","plants",
                    ["herbivore_births","herbivore_age_deaths","herbivore_hp_deaths","hp_deaths_zero_reserve_e",
                     "toxic_plant_consumed_deaths","nontoxic_plant_consumed_deaths","toxic_plant_age_deaths",
                     "nontoxic_plant_age_deaths","herbivore_feeding_events"]],
                "initial_plants_gn_gt_pn_pt": initial_plants,
                "rows": rows, "snapshots": snapshots,
            });
            let path = out.join(format!("{arm}_s{seed}.json"));
            fs::write(&path, record.to_string()).map_err(|e| e.to_string())?;
            let extinct = rows.iter().find(|r| r[1] == 0).map(|r| r[0].clone());
            println!("{arm} seed={seed} herbivore_extinct={extinct:?}");
        }
    }
    Ok(())
}
