//! Issue #74: conditions for the mimicry prerequisites on the stage-4 standard config.
//! Usage: mimicry_prereq <arm> <seed> <ticks>   (CSV to stdout, every 1000 ticks)
//! arms: std, B, B_T03, B_T03_D01, B_T03_D02, B_T01_D02 (round 1);
//! B_T03_D01_H10, B_T03_D01_H50, B_T03_D02_H50 (round 2: carnivore damage coefficient)
use sim_core::{SimulationConfig, World};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut c: SimulationConfig =
        serde_json::from_str(include_str!("../../../config/stage4-standard.json")).unwrap();
    let arm = a[1].as_str();
    if arm != "std" {
        c.toxin.plant_toxic_mutation_probability = 0.005;
    }
    let (threshold, decay, coefficient) = match arm {
        "std" | "B" => (None, 0., None),
        "B_T03" => (Some(0.03), 0., None),
        "B_T03_D01" => (Some(0.03), 0.01, None),
        "B_T03_D02" => (Some(0.03), 0.02, None),
        "B_T01_D02" => (Some(0.01), 0.02, None),
        "B_T03_D01_H10" => (Some(0.03), 0.01, Some(0.1)),
        "B_T03_D01_H50" => (Some(0.03), 0.01, Some(0.5)),
        "B_T03_D02_H50" => (Some(0.03), 0.02, Some(0.5)),
        other => panic!("unknown arm {other}"),
    };
    if coefficient.is_some() {
        c.animal_toxin.carnivore_hp_damage_per_excess_unit = coefficient;
    }
    if threshold.is_some() {
        c.animal_toxin.carnivore_damage_threshold = threshold;
    }
    c.animal_toxin.herbivore_toxin_decay_per_tick = decay;
    c.seed = a[2].parse().unwrap();
    let ticks: u64 = a[3].parse().unwrap();
    let mut w = World::new(c).unwrap();
    println!("arm,seed,tick,plants,toxic_plants,herbivores,carnivores,defended,mimic,other,avoid_defended,avoid_mimic,avoid_other,rate_defended,rate_mimic,rate_other,carnivore_damage");
    let f = |v: &serde_json::Value| v.as_f64().map_or(String::new(), |x| format!("{x:.4}"));
    loop {
        if w.tick.is_multiple_of(1000) {
            let m = w.mimicry_value();
            let t: serde_json::Value =
                serde_json::from_str(&w.animal_toxin_snapshot_json()).unwrap();
            let damage: f64 = t["damage"]
                .as_array()
                .map_or(0., |d| d.iter().filter_map(|x| x.as_f64()).sum());
            let g = |k: &str| {
                ["defended", "mimic", "other"]
                    .map(|n| f(&m[k][n]))
                    .join(",")
            };
            println!(
                "{arm},{},{},{},{},{},{},{},{},{},{:.4}",
                w.config.seed,
                w.tick,
                w.plants.items.len(),
                w.plants.items.iter().filter(|p| p.produces_toxin).count(),
                w.animals.items.len(),
                w.carnivores.len(),
                ["defended", "mimic", "other"]
                    .map(|n| m["counts"][n].to_string())
                    .join(","),
                g("avoided_by_carnivores"),
                g("attacks_per_1000_individual_ticks"),
                damage
            );
        }
        if w.tick >= ticks || w.step(100).is_err() {
            break;
        }
    }
}
