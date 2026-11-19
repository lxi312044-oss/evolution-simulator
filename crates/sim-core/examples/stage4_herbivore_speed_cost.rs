//! Issue #11: is herbivore speed a disadvantage without predators? Measurement only.
//!
//! Plan (fixed before the run):
//! https://github.com/lxi312044-oss/evolution-simulator/issues/11#issuecomment-6974147996
//!
//! Round 3 w_E settings on the stage 4 config (20x20). `nopred` starts with no
//! carnivores (carnivores disabled); `pred` keeps them. sim-core is not changed: this example only reads
//! herbivore state every 100 ticks. Only `step` changes the world.
//!
//! Usage:
//!   cargo run --release -p sim-core --example stage4_herbivore_speed_cost -- \
//!     config/stage4-inherited-plant-colors.json <output-dir> [ticks] [arm,...]
use serde_json::json;
use sim_core::{entity::EntityId, SimulationConfig, World};
use std::{collections::HashMap, env, fs, path::Path};

const SEEDS: std::ops::RangeInclusive<u64> = 90001..=90016;
const ARMS: [&str; 2] = ["nopred", "pred"];

/// Last observation of a herbivore: speed, age, successful births, present at tick 0.
#[derive(Clone, Copy)]
struct Seen {
    speed: f32,
    age: u32,
    births: u64,
    initial: bool,
}

fn record(s: Seen, tick: u64) -> serde_json::Value {
    json!([s.speed, s.age, s.births, s.initial, tick])
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
            if *arm == "nopred" {
                // Zero carnivores requires carnivores to be disabled (config validation).
                config.carnivores.enabled = false;
                config.initial_population.carnivores = 0;
            }
            let flee = config.herbivores.predator_flee_speed_multiplier;
            let carnivore_speed = config.carnivores.speed;
            let mut world = World::new(config.clone())?;
            let mut seen: HashMap<EntityId, Seen> = world
                .animals
                .items
                .iter()
                .map(|h| {
                    let s = Seen {
                        speed: h.genes.speed,
                        age: h.age,
                        births: h.successful_births,
                        initial: true,
                    };
                    (h.id, s)
                })
                .collect();
            let stats = |w: &World| {
                let n = w.animals.items.len();
                if n == 0 {
                    return (f64::NAN, f64::NAN);
                }
                let k = n as f64;
                let mean = w
                    .animals
                    .items
                    .iter()
                    .map(|h| f64::from(h.genes.speed))
                    .sum::<f64>()
                    / k;
                let faster = w
                    .animals
                    .items
                    .iter()
                    .filter(|h| h.genes.speed * flee > carnivore_speed)
                    .count() as f64
                    / k;
                (mean, faster)
            };
            let (m0, f0) = stats(&world);
            let mut rows = vec![json!([
                0,
                world.carnivores.len(),
                world.herbivore_count(),
                world.plants.items.len(),
                m0,
                f0
            ])];
            let mut deaths = Vec::new();
            let mut tick = 0;
            while tick < ticks {
                world.step(100)?;
                tick += 100;
                let mut now: HashMap<EntityId, Seen> = HashMap::with_capacity(seen.len());
                for h in &world.animals.items {
                    let initial = seen.get(&h.id).is_some_and(|s| s.initial);
                    now.insert(
                        h.id,
                        Seen {
                            speed: h.genes.speed,
                            age: h.age,
                            births: h.successful_births,
                            initial,
                        },
                    );
                }
                let mut gone: Vec<(EntityId, Seen)> = seen
                    .iter()
                    .filter(|(id, _)| !now.contains_key(id))
                    .map(|(id, s)| (*id, *s))
                    .collect();
                gone.sort_by_key(|(id, _)| *id);
                deaths.extend(gone.into_iter().map(|(_, s)| record(s, tick)));
                seen = now;
                let (mean, faster) = stats(&world);
                rows.push(json!([
                    tick,
                    world.carnivores.len(),
                    world.herbivore_count(),
                    world.plants.items.len(),
                    mean,
                    faster
                ]));
            }
            let mut alive: Vec<(EntityId, Seen)> = seen.into_iter().collect();
            alive.sort_by_key(|(id, _)| *id);
            let alive: Vec<_> = alive.into_iter().map(|(_, s)| record(s, tick)).collect();
            let doc = json!({
                "issue": 11, "arm": arm, "seed": seed, "ticks": ticks,
                "effective_config": config,
                "row_fields": ["tick", "carnivores", "herbivores", "plants",
                               "herbivore_speed_mean", "herbivores_fleeing_faster_than_carnivore_speed"],
                "individual_fields": ["speed", "age_last_seen", "births_last_seen", "initial", "tick"],
                "rows": rows,
                "deaths": deaths,
                "alive_at_end": alive,
            });
            fs::write(out.join(format!("{arm}_s{seed}.json")), doc.to_string())
                .map_err(|e| e.to_string())?;
            println!(
                "{arm} seed={seed} herbivores={} speed_mean {m0:.2} -> {:.2}",
                world.herbivore_count(),
                stats(&world).0
            );
        }
    }
    Ok(())
}
