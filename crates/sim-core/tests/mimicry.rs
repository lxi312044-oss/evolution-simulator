//! Issue #72: mimicry indicators are reported and consistent.
use sim_core::{SimulationConfig, World};

#[test]
fn mimicry_indicators_count_every_herbivore_once() {
    let c: SimulationConfig =
        serde_json::from_str(include_str!("../../../config/stage4-standard.json")).unwrap();
    let mut w = World::new(c).unwrap();
    for _ in 0..10 {
        w.step(100).unwrap();
    }
    let m: serde_json::Value = serde_json::from_str(&w.mimicry_json()).unwrap();
    let total: u64 = ["defended", "mimic", "other"]
        .iter()
        .map(|g| m["counts"][g].as_u64().unwrap())
        .sum();
    assert_eq!(total, w.animals.items.len() as u64);
    let exposure: u64 = ["defended", "mimic", "other"]
        .iter()
        .map(|g| m["individual_ticks"][g].as_u64().unwrap())
        .sum();
    assert!(exposure > 0);
    assert_eq!(m["definitions"]["mimic_distance"], 0.25);
}

/// Issue #74: herbivore toxin decay keeps stocks below the cap and the toxin balance closed.
#[test]
fn herbivore_toxin_decay_lowers_stock_and_keeps_balance() {
    let run = |decay: f64| {
        let mut c: SimulationConfig =
            serde_json::from_str(include_str!("../../../config/stage4-standard.json")).unwrap();
        c.animal_toxin.herbivore_toxin_decay_per_tick = decay;
        let mut w = World::new(c).unwrap();
        for _ in 0..10 {
            w.step(100).unwrap();
        }
        let t: serde_json::Value = serde_json::from_str(&w.animal_toxin_snapshot_json()).unwrap();
        for e in t["balance_error"].as_array().unwrap() {
            assert!(e.as_f64().unwrap().abs() < 1e-6, "balance {e}");
        }
        let m: serde_json::Value = serde_json::from_str(&w.mimicry_json()).unwrap();
        m["counts"]["defended"].as_f64().unwrap() / w.animals.items.len() as f64
    };
    assert!(run(0.) > 0.9);
    assert!(run(0.02) < 0.5);
    let mut bad: SimulationConfig =
        serde_json::from_str(include_str!("../../../config/stage4-standard.json")).unwrap();
    bad.animal_toxin.herbivore_toxin_decay_per_tick = 1.;
    assert!(World::new(bad).is_err());
}
