//! Issue #78: comparisons for the heritable diet value on the current standard config.
//! Usage: diet_goal <arm> <seed> <ticks> [every]   (CSV to stdout, default every 500 ticks)
//!        diet_goal <arm> config                    (effective config JSON to stdout)
//! An arm is a list of tokens joined by `_` applied to config/stage4-standard-20261120.json
//! (the standard before the diet value; the current standard is its DE_H20_M15_K50):
//!   std  no change (carnivores are born from herbivores with probability 0.002)
//!   DO   diet value observed only (`diet_gene` with `effects` false, default values)
//!   DE   diet value decides the body and the feeding gains (`effects` true; the fixed
//!        herbivore-to-carnivore probability is set to 0)
//!   K<n> efficiency exponent n/100        M<n> mutation scale n/100
//!   Q<n> mutation probability n/100       H<n> founder herbivore value n/100
//!   W<n> founder spread n/100
//! K, M, Q, H and W add the diet value (observed only) when no D token came first.
//! A run that fails (for example a conservation check) prints the error to stderr and exits 2.
use sim_core::{config::DietGeneSettings, SimulationConfig, World};

fn config_for(arm: &str) -> SimulationConfig {
    let mut c: SimulationConfig = serde_json::from_str(include_str!(
        "../../../config/stage4-standard-20261120.json"
    ))
    .unwrap();
    for token in arm.split('_') {
        let percent = |t: &str| -> f64 {
            let n: u32 = t[1..].parse().unwrap_or_else(|_| panic!("bad token {t}"));
            f64::from(n) / 100.
        };
        match token {
            "std" => {}
            "DO" => c.diet_gene = Some(DietGeneSettings::default()),
            "DE" => {
                c.diet_gene = Some(DietGeneSettings {
                    effects: true,
                    ..c.diet_gene.take().unwrap_or_default()
                });
                c.herbivores.carnivore_offspring_probability = 0.;
            }
            t if t.len() > 1 && t[1..].bytes().all(|b| b.is_ascii_digit()) => {
                let d = c.diet_gene.get_or_insert_with(DietGeneSettings::default);
                match &t[..1] {
                    "K" => d.efficiency_exponent = percent(t),
                    "M" => d.mutation_scale = percent(t),
                    "Q" => d.mutation_probability = percent(t),
                    "H" => d.initial_herbivore = percent(t),
                    "W" => d.initial_spread = percent(t),
                    _ => panic!("unknown arm token {t}"),
                }
            }
            other => panic!("unknown arm token {other}"),
        }
    }
    c
}

const BODIES: [&str; 2] = ["herbivore", "carnivore"];
const LOST: [&str; 4] = [
    "herbivore_nutrients",
    "herbivore_energy",
    "carnivore_nutrients",
    "carnivore_energy",
];
const BIRTHS: [&str; 4] = [
    "herbivore_to_herbivore",
    "herbivore_to_carnivore",
    "carnivore_to_carnivore",
    "carnivore_to_herbivore",
];

fn header() -> String {
    let mut h: Vec<String> = [
        "arm",
        "seed",
        "tick",
        "plants",
        "herbivores",
        "carnivores",
        "herbivore_births",
        "carnivore_births",
        "carnivore_deaths",
        "carnivore_hunts",
        "carnivores_born_from_herbivores",
        "intermediate_share",
    ]
    .map(String::from)
    .to_vec();
    for b in BODIES {
        h.extend(["mean", "sd"].map(|k| format!("{b}_diet_{k}")));
        h.extend((0..10).map(|i| format!("{b}_diet_hist{i}")));
    }
    h.extend(BIRTHS.map(|k| format!("births_{k}")));
    h.extend(LOST.map(|k| format!("lost_{k}")));
    for k in ["herbivore_to_carnivore", "carnivore_to_herbivore"] {
        h.extend(["n", "mean"].map(|s| format!("parent_{k}_{s}")));
        h.extend((0..10).map(|i| format!("parent_{k}_hist{i}")));
    }
    h.join(",")
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arm = a[1].as_str();
    let mut c = config_for(arm);
    if a[2] == "config" {
        println!("{}", serde_json::to_string(&c).unwrap());
        return;
    }
    c.seed = a[2].parse().unwrap();
    let ticks: u64 = a[3].parse().unwrap();
    let every: u64 = a.get(4).map_or(500, |v| v.parse().unwrap());
    let mut w = World::new(c).unwrap();
    println!("{}", header());
    let f = |v: &serde_json::Value| v.as_f64().map_or(String::new(), |x| format!("{x:.5}"));
    loop {
        if w.tick.is_multiple_of(every) {
            let d = w.diet_value_json();
            let mut row: Vec<String> = vec![
                arm.into(),
                w.config.seed.to_string(),
                w.tick.to_string(),
                w.plants.items.len().to_string(),
                w.animals.items.len().to_string(),
                w.carnivores.len().to_string(),
                w.diagnostics.herbivore_births.to_string(),
                w.carnivore_births.to_string(),
                w.carnivore_deaths.to_string(),
                w.carnivore_hunts.to_string(),
                w.defense.carnivores_born_from_herbivores.to_string(),
                f(&d["intermediate_share"]),
            ];
            for b in BODIES {
                row.extend(["mean", "sd"].map(|k| f(&d[b][k])));
                row.extend((0..10).map(|i| d[b]["hist"][i].to_string()));
            }
            row.extend(BIRTHS.map(|k| d["births"][k].to_string()));
            row.extend(LOST.map(|k| f(&d["lost"][k])));
            for k in ["herbivore_to_carnivore", "carnivore_to_herbivore"] {
                let p = &d["crossing_parents"][k];
                row.extend([p["n"].to_string(), f(&p["mean"])]);
                row.extend((0..10).map(|i| p["hist"][i].to_string()));
            }
            println!("{}", row.join(","));
        }
        if w.tick >= ticks {
            break;
        }
        if let Err(e) = w.step(100) {
            eprintln!("tick {}: {e}", w.tick);
            std::process::exit(2);
        }
    }
}
