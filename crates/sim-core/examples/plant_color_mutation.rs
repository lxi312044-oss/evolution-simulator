//! Issue #70: plant color mutation on the stage-4 standard config.
//! Usage: plant_color_mutation <arm: control|p20|p05> <seed> <ticks>  (CSV rows to stdout)
use sim_core::{SimulationConfig, World};
use std::collections::HashMap;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let text = include_str!("../../../config/stage4-standard-20261119.json");
    let mut c: SimulationConfig = serde_json::from_str(text).unwrap();
    let (p, s) = match a[1].as_str() {
        "control" => (0., 0.),
        "p20" => (0.20, 0.25),
        "p05" => (0.05, 0.10),
        other => panic!("unknown arm {other}"),
    };
    c.plants.color_mutation_probability = p;
    c.plants.color_mutation_scale = s;
    c.seed = a[2].parse().unwrap();
    let ticks: u64 = a[3].parse().unwrap();
    let mut w = World::new(c).unwrap();
    println!(
        "arm,seed,tick,plants,toxic_plants,herbivores,carnivores,plant_colors,plant_species,energy"
    );
    loop {
        if w.tick.is_multiple_of(1000) {
            let mut bins: HashMap<u32, u32> = HashMap::new();
            for pl in &w.plants.items {
                let key = [24, 16, 8].map(|sh| ((pl.color >> sh) & 255) / 64);
                *bins.entry(key[0] * 16 + key[1] * 4 + key[2]).or_default() += 1;
            }
            let n = w.plants.items.len().max(1) as f64;
            let colors = bins.values().filter(|&&v| v as f64 / n >= 0.01).count();
            let summary: serde_json::Value =
                serde_json::from_str(&w.species_summary_json()).unwrap();
            println!(
                "{},{},{},{},{},{},{},{},{},{:.6}",
                a[1],
                w.config.seed,
                w.tick,
                w.plants.items.len(),
                w.plants.items.iter().filter(|p| p.produces_toxin).count(),
                w.animals.items.len(),
                w.carnivores.len(),
                colors,
                summary["kinds"]["plant"]["living_species"],
                w.resources.total_energy()
            );
        }
        if w.tick >= ticks || w.step(100).is_err() {
            break;
        }
    }
}
