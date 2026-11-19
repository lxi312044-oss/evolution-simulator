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
