//! Issue #54 step 3: herbivore counterattack and carnivore learning from injury
//! (Issue 11 round 3 w_E settings, 20x20).
//!
//! Plan (fixed before the run):
//! https://github.com/lxi312044-oss/evolution-simulator/issues/54#issuecomment-6976559257
//! Threshold, carnivore memory decay and small counterattack arms (t_*):
//! https://github.com/lxi312044-oss/evolution-simulator/issues/54#issuecomment-6977189837
//! Retreat arms (r_*):
//! https://github.com/lxi312044-oss/evolution-simulator/issues/54#issuecomment-6977713545
//!
//! Usage:
//!   cargo run --release -p sim-core --example stage4_counterattack -- \
//!     config/stage4-inherited-plant-colors.json <output-dir> [max-ticks] [arm,...]
//!
//! Runs up to `max-ticks` (default 40000) and stops after tick 20000 once carnivores or
//! herbivores are extinct. Rows use the `stage4_carnivore_review` layout plus defense
//! counters and herbivore gene statistics. Herbivores are followed by id every 100 ticks
//! to record genes, color, age and births at death. Only `step` changes the world.
use serde_json::{json, Value};
use sim_core::{entity::EntityId, SimulationConfig, World};
use std::{collections::HashMap, env, fs, path::Path};

const SEEDS: std::ops::RangeInclusive<u64> = 90001..=90016;
const JUDGE_TICK: u64 = 20_000;

struct Arm {
    name: &'static str,
    counterattack: f64,
    learning: bool,
    threshold: f64,
    carnivore_decay: f64,
    retreat: u32,
}

const fn arm(name: &'static str, counterattack: f64, learning: bool) -> Arm {
    Arm {
        name,
        counterattack,
        learning,
        threshold: 0.,
        carnivore_decay: 0.,
        retreat: 0,
    }
}

const fn t_arm(name: &'static str, counterattack: f64, threshold: f64, decay: f64) -> Arm {
    Arm {
        name,
        counterattack,
        learning: true,
        threshold,
        carnivore_decay: decay,
        retreat: 0,
    }
}

const fn r_arm(name: &'static str, counterattack: f64, decay: f64, retreat: u32) -> Arm {
    Arm {
        name,
        counterattack,
        learning: true,
        threshold: if counterattack > 0. { 0.7 } else { 0. },
        carnivore_decay: decay,
        retreat,
    }
}

const ARMS: [Arm; 15] = [
    arm("d_ctrl", 0., true),
    arm("d_c2", 2., true),
    arm("d_c5", 5., true),
    arm("d_c5_nolearn", 5., false),
    t_arm("t_ctrl", 0., 0., 0.),
    t_arm("t_c05", 0.5, 0., 0.),
    t_arm("t_c2_th", 2., 0.7, 0.),
    t_arm("t_c5_th", 5., 0.7, 0.),
    t_arm("t_c2_th_dec", 2., 0.7, 0.0007),
    t_arm("t_c5_th_dec", 5., 0.7, 0.0007),
    r_arm("r_ctrl", 0., 0., 0),
    r_arm("r_c2_th_dec", 2., 0.0007, 0),
    r_arm("r_c2_th_dec_ret", 2., 0.0007, 300),
    r_arm("r_c5_th_dec_ret", 5., 0.0007, 300),
    r_arm("r_c2_th_ret", 2., 0., 300),
];

/// Defense counters and herbivore statistics, in this order.
const EXTRA: [&str; 10] = [
    "counterattacks",
    "counterattack_energy",
    "counterattack_learning_events",
    "hp_mean",
    "aggression_mean",
    "fear_mean",
    "speed_mean",
    "threatened",
    "standing_ground",
    "strong_fraction_0_7",
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

fn extra(w: &World, prev: &[f64; 3], radius: f32) -> (Vec<f64>, [f64; 3]) {
    let d = &w.defense;
    let now = [
        d.counterattacks as f64,
        d.counterattack_energy,
        d.counterattack_learning_events as f64,
    ];
    let mut out: Vec<f64> = now.iter().zip(prev).map(|(a, b)| a - b).collect();
    let n = w.animals.items.len();
    if n == 0 {
        out.extend([f64::NAN; 4]);
        out.extend([0., 0., f64::NAN]);
    } else {
        let k = n as f64;
        let mean = |f: fn(&sim_core::animals::Herbivore) -> f32| {
            w.animals.items.iter().map(|h| f64::from(f(h))).sum::<f64>() / k
        };
        out.push(mean(|h| h.genes.adult_max_hp));
        out.push(mean(|h| h.genes.aggression));
        out.push(mean(|h| h.genes.fear));
        out.push(mean(|h| h.genes.speed));
        let threatened: Vec<_> = w
            .animals
            .items
            .iter()
            .filter(|h| {
                w.carnivores
                    .iter()
                    .any(|c| (c.x - h.x).powi(2) + (c.y - h.y).powi(2) <= radius * radius)
            })
            .collect();
        out.push(threatened.len() as f64);
        out.push(threatened.iter().filter(|h| h.defense_active).count() as f64);
        let strong = w
            .animals
            .items
            .iter()
            .filter(|h| {
                f64::from(h.genes.aggression) * f64::from(h.genes.adult_max_hp) / 60. >= 0.7
            })
            .count();
        out.push(strong as f64 / k);
    }
    (out, now)
}

/// Last observation of a herbivore.
fn observe(h: &sim_core::animals::Herbivore, initial: bool) -> Value {
    let [r, g, b] = h.genes.body_color.rgb();
    json!([
        h.genes.adult_max_hp,
        h.genes.aggression,
        h.genes.fear,
        h.genes.speed,
        r,
        g,
        b,
        h.age,
        h.successful_births,
        initial
    ])
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
            // Issue 11 round 3 w_E.
            config.carnivores.speed = 5.25;
            config.herbivores.color_memory_decay_per_tick = 0.0007;
            config.carnivores.sustainable_reproduction_enabled = false;
            config.carnivores.feeding_resume_fraction = 0.2;
            config.carnivores.food_limited_reproduction_enabled = true;
            config.carnivores.prey_trend_reproduction_enabled = true;
            config.carnivores.basal_energy_per_tick = 0.0072;
            // Counterattack.
            config.herbivores.counterattack_energy = a.counterattack;
            config.herbivores.counterattack_learning_enabled = a.learning;
            config.herbivores.counterattack_strength_threshold = a.threshold;
            config.carnivores.color_memory_decay_per_tick = a.carnivore_decay;
            config.herbivores.counterattack_retreat_ticks = a.retreat;
            let radius = config.herbivores.predator_detection_radius;
            let mut world = World::new(config.clone())?;
            let mut seen: HashMap<EntityId, Value> = world
                .animals
                .items
                .iter()
                .map(|h| (h.id, observe(h, true)))
                .collect();
            let (mut prev_n, mut prev_e) = (counts(&world), energy(&world));
            let mut prev_d = [0.; 3];
            let mut rows = Vec::new();
            let mut deaths = Vec::new();
            let mut tick = 0;
            while tick < max_ticks {
                world.step(100)?;
                tick += 100;
                let (now_n, now_e) = (counts(&world), energy(&world));
                let dn: Vec<u64> = now_n.iter().zip(&prev_n).map(|(a, b)| a - b).collect();
                let de: Vec<f64> = now_e.iter().zip(&prev_e).map(|(a, b)| a - b).collect();
                (prev_n, prev_e) = (now_n, now_e);
                let (dx, now_d) = extra(&world, &prev_d, radius);
                prev_d = now_d;
                let mut now: HashMap<EntityId, Value> = HashMap::with_capacity(seen.len());
                for h in &world.animals.items {
                    let initial = seen.get(&h.id).is_some_and(|s| s[9] == json!(true));
                    now.insert(h.id, observe(h, initial));
                }
                let mut gone: Vec<(EntityId, Value)> = seen
                    .iter()
                    .filter(|(id, _)| !now.contains_key(id))
                    .map(|(id, v)| (*id, v.clone()))
                    .collect();
                gone.sort_by_key(|(id, _)| *id);
                for (_, mut v) in gone {
                    v.as_array_mut().unwrap().push(json!(tick));
                    deaths.push(v);
                }
                seen = now;
                rows.push(json!([
                    tick,
                    world.carnivores.len(),
                    world.herbivore_count(),
                    world.plants.items.len(),
                    dn,
                    de,
                    dx
                ]));
                if tick >= JUDGE_TICK
                    && (world.carnivores.is_empty() || world.herbivore_count() == 0)
                {
                    break;
                }
            }
            let mut alive: Vec<(EntityId, Value)> = seen.into_iter().collect();
            alive.sort_by_key(|(id, _)| *id);
            let alive: Vec<Value> = alive
                .into_iter()
                .map(|(_, mut v)| {
                    v.as_array_mut().unwrap().push(json!(tick));
                    v
                })
                .collect();
            let record = json!({
                "issue": 54, "arm": a.name, "seed": seed, "max_ticks": max_ticks,
                "last_tick": tick, "judge_tick": JUDGE_TICK,
                "effective_config": config,
                "row_fields": ["tick", "carnivores", "herbivores", "plants", COUNTS, ENERGY, EXTRA],
                "individual_fields": ["adult_max_hp", "aggression", "fear", "speed", "r", "g", "b",
                                      "age_last_seen", "births_last_seen", "initial", "tick"],
                "rows": rows,
                "deaths": deaths,
                "alive_at_end": alive,
            });
            fs::write(
                out.join(format!("{}_s{seed}.json", a.name)),
                record.to_string(),
            )
            .map_err(|e| e.to_string())?;
            println!(
                "{} seed={seed} last_tick={tick} carnivores={} herbivores={} plants={} counterattacks={}",
                a.name,
                world.carnivores.len(),
                world.herbivore_count(),
                world.plants.items.len(),
                world.defense.counterattacks
            );
        }
    }
    Ok(())
}
