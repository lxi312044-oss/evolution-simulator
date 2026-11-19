//! Issue #76: comparisons toward the Batesian-mimicry goal on the stage-4 standard config.
//! Usage: mimicry_goal <arm> <seed> <ticks> [every]   (CSV to stdout, default every 500 ticks)
//!        mimicry_goal <arm> config                    (effective config JSON to stdout)
//! An arm is a list of tokens joined by `_` applied to config/stage4-standard-20261119.json:
//!   std  no change          B    no change (plant toxicity mutation is already standard)
//!   T03 / T01 / T00  carnivore damage threshold 0.03 / 0.01 / 0
//!   D01 / D02 / D005 / D002  herbivore toxin decay per tick 0.01 / 0.02 / 0.005 / 0.002
//!   H10 / H50  carnivore HP damage per excess unit 0.1 / 0.5
//!   NL   carnivore color learning off (control)
//!   G<n> carnivore learning gain n (e.g. G30), Issue #76 round 1
//!   F<n> herbivore reproduction food target n plants (standard 8), Issue #76 round 3
//!   C<n> herbivore body-color mutation probability n % per channel, Issue #76 round 4
//!   X<n> herbivore toxic trait: founders toxic with n %, offspring flip with 0.5 %
//!   P<n> plant-derived toxin moves to carnivores only from a load of n/100 per type
//!   R<n> carnivores resume hunting at stomach fullness n % (standard 20)
//! Example: B_T03_D01_H50_NL. Names from Issue #74 (std, B_T03_D01, ...) keep their meaning.
use sim_core::{SimulationConfig, World};

fn config_for(arm: &str) -> SimulationConfig {
    let mut c: SimulationConfig = serde_json::from_str(include_str!(
        "../../../config/stage4-standard-20261119.json"
    ))
    .unwrap();
    for token in arm.split('_') {
        let a = &mut c.animal_toxin;
        match token {
            "std" | "B" => {}
            "T03" => a.carnivore_damage_threshold = Some(0.03),
            "T01" => a.carnivore_damage_threshold = Some(0.01),
            "T00" => a.carnivore_damage_threshold = Some(0.),
            "D01" => a.herbivore_toxin_decay_per_tick = 0.01,
            "D02" => a.herbivore_toxin_decay_per_tick = 0.02,
            "D005" => a.herbivore_toxin_decay_per_tick = 0.005,
            "D002" => a.herbivore_toxin_decay_per_tick = 0.002,
            "H10" => a.carnivore_hp_damage_per_excess_unit = Some(0.1),
            "H50" => a.carnivore_hp_damage_per_excess_unit = Some(0.5),
            "NL" => a.color_learning_enabled = false,
            g if g.starts_with('G') && g[1..].parse::<f64>().is_ok() => {
                a.carnivore_learning_gain = Some(g[1..].parse().unwrap())
            }
            f if f.starts_with('F') && f[1..].parse::<u32>().is_ok() => {
                c.herbivores.reproduction_food_target = f[1..].parse().unwrap()
            }
            x if x.starts_with('X') && x[1..].parse::<u32>().is_ok() => {
                let percent: u32 = x[1..].parse().unwrap();
                a.herbivore_toxic_initial_fraction = Some(f64::from(percent) / 100.);
                a.herbivore_toxic_mutation_probability = 0.005;
            }
            p if p.starts_with('P') && p[1..].parse::<u32>().is_ok() => {
                let percent: u32 = p[1..].parse().unwrap();
                a.plant_toxin_transfer_threshold = f64::from(percent) / 100.;
            }
            r if r.starts_with('R') && r[1..].parse::<u32>().is_ok() => {
                let percent: u32 = r[1..].parse().unwrap();
                c.carnivores.feeding_resume_fraction = f64::from(percent) / 100.;
            }
            m if m.starts_with('C') && m[1..].parse::<u32>().is_ok() => {
                let percent: u32 = m[1..].parse().unwrap();
                c.herbivores.body_color_mutation_probability = Some(percent as f32 / 100.)
            }
            other => panic!("unknown arm token {other}"),
        }
    }
    c
}

const G: [&str; 3] = ["defended", "mimic", "other"];
const PROFILE: [&str; 11] = [
    "toxic_trait",
    "age",
    "juvenile",
    "energy_fraction",
    "low_energy",
    "near_carnivore",
    "speed",
    "adult_max_hp",
    "toxin_ability",
    "toxin_stock",
    "plant_toxin_load",
];
const MEMORY: [&str; 8] = [
    "carnivores_with_state",
    "mean_entries",
    "mean_harmful_entries",
    "mean_max_harm",
    "mean_tolerance",
    "mean_hp",
    "mean_load",
    "avoids_own_harm_color",
];

/// Choice-scene and dose columns (Issue #76 revised M1 and diagnosis), appended at the end.
const CHOICE: [&str; 8] = [
    "choice_first",
    "choice_skipped",
    "choice_tolerance_sum",
    "choice_risk_sum",
    "choice_any_risk",
    "choice_refused_if_fed",
    "dose_produced",
    "dose_plant",
];

fn header() -> String {
    let mut h: Vec<String> = [
        "arm",
        "seed",
        "tick",
        "plants",
        "toxic_plants",
        "herbivores",
        "carnivores",
    ]
    .map(String::from)
    .to_vec();
    for k in ["n", "avoid", "attacks", "exposure"] {
        h.extend(G.map(|g| format!("{k}_{g}")));
    }
    h.extend(
        [
            "mimic_share",
            "mean_distance",
            "carn_damage",
            "transferred_produced",
            "transferred_residual",
            "carn_toxin_deaths",
            "color_skips",
            "herb_births",
            "herb_hp_deaths_zero_e",
            "herb_hp_deaths_positive_e",
            "herb_age_deaths",
            "predation_kills",
            "carn_births",
            "carn_from_herbivores",
            "carn_deaths",
            "carn_starvation_deaths",
            "color_neighbor_share",
            "harmful_evictions",
            "memory_entries_created",
        ]
        .map(String::from),
    );
    for g in G {
        h.extend(PROFILE.map(|k| format!("{k}_{g}")));
    }
    h.extend(MEMORY.map(|k| format!("mem_{k}")));
    for k in ["sa", "se"] {
        for g in G {
            h.extend((0..8).map(|s| format!("{k}_{g}_{s}")));
        }
    }
    for k in CHOICE {
        h.extend(G.map(|g| format!("{k}_{g}")));
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
    // Issue #76 revised M1: choice-scene observation (observation only).
    w.enable_mimicry_choice_observation();
    println!("{}", header());
    let f = |v: &serde_json::Value| v.as_f64().map_or(String::new(), |x| format!("{x:.5}"));
    loop {
        if w.tick.is_multiple_of(every) {
            let m = w.mimicry_value();
            let d = w.mimicry_diagnostics_value();
            let t: serde_json::Value =
                serde_json::from_str(&w.animal_toxin_snapshot_json()).unwrap();
            let sum = |k: &str| {
                t[k].as_array()
                    .map_or(0., |v| v.iter().filter_map(|x| x.as_f64()).sum::<f64>())
            };
            let mut row: Vec<String> = vec![
                arm.into(),
                w.config.seed.to_string(),
                w.tick.to_string(),
                w.plants.items.len().to_string(),
                w.plants
                    .items
                    .iter()
                    .filter(|p| p.produces_toxin)
                    .count()
                    .to_string(),
                w.animals.items.len().to_string(),
                w.carnivores.len().to_string(),
            ];
            row.extend(G.map(|g| m["counts"][g].to_string()));
            row.extend(G.map(|g| f(&m["avoided_by_carnivores"][g])));
            row.extend(G.map(|g| m["attacks"][g].to_string()));
            row.extend(G.map(|g| m["individual_ticks"][g].to_string()));
            let dg = &w.diagnostics;
            let cd = &w.carnivore_debug;
            row.extend([
                f(&m["mimic_share_of_undefended"]),
                f(&m["mean_distance_to_defended"]),
                format!("{:.5}", sum("damage")),
                format!("{:.5}", sum("transferred_produced")),
                format!("{:.5}", sum("transferred_residual")),
                t["toxin_deaths"].to_string(),
                t["color_skips"].to_string(),
                dg.herbivore_births.to_string(),
                dg.phase19_hp_zero_energy_deaths.to_string(),
                dg.phase19_hp_positive_energy_deaths.to_string(),
                dg.herbivore_age_deaths.to_string(),
                cd.prey_hp_zero_after_attack.to_string(),
                w.carnivore_births.to_string(),
                w.defense.carnivores_born_from_herbivores.to_string(),
                w.carnivore_deaths.to_string(),
                cd.p40_death_energy_starvation.to_string(),
                f(&d["color_neighbor_share"]),
                d["harmful_evictions"].to_string(),
                d["memory_entries_created"].to_string(),
            ]);
            for g in G {
                row.extend(PROFILE.map(|k| f(&d["profile"][g][k])));
            }
            row.extend(MEMORY.map(|k| f(&d["carnivore_memory"][k])));
            for k in ["strata_attacks", "strata_exposure"] {
                for g in 0..3 {
                    row.extend((0..8).map(|s| d[k][g][s].to_string()));
                }
            }
            for k in CHOICE {
                row.extend((0..3).map(|g| match d[k][g].as_u64() {
                    Some(n) => n.to_string(),
                    None => f(&d[k][g]),
                }));
            }
            println!("{}", row.join(","));
        }
        if w.tick >= ticks || w.step(100).is_err() {
            break;
        }
    }
}
