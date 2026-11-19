//! Issue #66/#67: prints the species ranking and events for a config (calibration aid).
//! Usage: cargo run --example species_probe -- <config.json> <ticks> [report_every]
use sim_core::{SimulationConfig, World};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let text = std::fs::read_to_string(&args[1]).unwrap();
    let config: SimulationConfig = serde_json::from_str(&text).unwrap();
    let ticks: u64 = args[2].parse().unwrap();
    let every: u64 = args.get(3).map_or(5000, |v| v.parse().unwrap());
    let mut world = World::new(config).unwrap();
    let mut events = 0usize;
    let mut by_code = std::collections::BTreeMap::<String, usize>::new();
    while world.tick < ticks {
        if world.step(100).is_err() {
            println!("step error at {}", world.tick);
            break;
        }
        let taken: serde_json::Value = serde_json::from_str(&world.take_events_json()).unwrap();
        for e in taken.as_array().unwrap() {
            events += 1;
            *by_code
                .entry(e["code"].as_str().unwrap().into())
                .or_default() += 1;
            if world.tick <= 2000 || world.tick.is_multiple_of(every) {
                println!("  [{}] {}", e["tick"], e["message"].as_str().unwrap());
            }
        }
        if world.tick.is_multiple_of(every) {
            let s: serde_json::Value = serde_json::from_str(&world.species_summary_json()).unwrap();
            println!("tick {}", world.tick);
            for kind in ["plant", "herbivore", "carnivore"] {
                let k = &s["kinds"][kind];
                let top: Vec<String> = k["species"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .take(5)
                    .map(|v| {
                        format!(
                            "{}:{}({:.0}%)",
                            v["name"].as_str().unwrap(),
                            v["count"],
                            v["share"].as_f64().unwrap() * 100.
                        )
                    })
                    .collect();
                println!(
                    "  {kind}: total {} living {} extinct {} | {}",
                    k["total"],
                    k["living_species"],
                    k["extinct_species"],
                    top.join(" ")
                );
            }
        }
    }
    println!("events {events} {by_code:?}");
}
