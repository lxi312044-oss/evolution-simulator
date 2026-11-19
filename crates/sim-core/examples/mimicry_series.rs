//! Issue #72: mimicry indicators over time on the stage-4 standard config.
//! Usage: mimicry_series <seed> <ticks> [every]  (CSV to stdout)
use sim_core::{SimulationConfig, World};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut c: SimulationConfig =
        serde_json::from_str(include_str!("../../../config/stage4-standard.json")).unwrap();
    c.seed = a[1].parse().unwrap();
    let ticks: u64 = a[2].parse().unwrap();
    let every: u64 = a.get(3).map_or(1000, |v| v.parse().unwrap());
    let mut w = World::new(c).unwrap();
    println!("seed,tick,herbivores,carnivores,defended,mimic,other,mimic_share,mean_distance,avoid_defended,avoid_mimic,avoid_other,attacks_defended,attacks_mimic,attacks_other,rate_defended,rate_mimic,rate_other");
    let f = |v: &serde_json::Value| v.as_f64().map_or(String::new(), |x| format!("{x:.4}"));
    loop {
        if w.tick.is_multiple_of(every) {
            let m = w.mimicry_value();
            let g = |k: &str| {
                ["defended", "mimic", "other"]
                    .map(|n| f(&m[k][n]))
                    .join(",")
            };
            println!(
                "{},{},{},{},{},{},{},{},{},{}",
                w.config.seed,
                w.tick,
                w.animals.items.len(),
                w.carnivores.len(),
                ["defended", "mimic", "other"]
                    .map(|n| m["counts"][n].to_string())
                    .join(","),
                f(&m["mimic_share_of_undefended"]),
                f(&m["mean_distance_to_defended"]),
                g("avoided_by_carnivores"),
                ["defended", "mimic", "other"]
                    .map(|n| m["attacks"][n].to_string())
                    .join(","),
                g("attacks_per_1000_individual_ticks")
            );
        }
        if w.tick >= ticks || w.step(100).is_err() {
            break;
        }
    }
}
