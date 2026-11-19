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

/// Issue #76 (observation only): the strata partition the attack and exposure counters,
/// and observing does not change the run.
#[test]
fn mimicry_diagnostics_partition_counters_and_do_not_change_the_run() {
    let config = || {
        let mut c: SimulationConfig =
            serde_json::from_str(include_str!("../../../config/stage4-standard.json")).unwrap();
        c.animal_toxin.herbivore_toxin_decay_per_tick = 0.01;
        c.animal_toxin.carnivore_damage_threshold = Some(0.03);
        c.herbivores.carnivore_offspring_probability = 0.02;
        c
    };
    let mut observed = World::new(config()).unwrap();
    let mut plain = World::new(config()).unwrap();
    for _ in 0..30 {
        observed.step(100).unwrap();
        plain.step(100).unwrap();
        let _ = observed.mimicry_diagnostics_value();
        let _ = observed.mimicry_json();
        assert_eq!(observed.animals.items.len(), plain.animals.items.len());
        assert_eq!(observed.carnivores.len(), plain.carnivores.len());
        assert_eq!(
            observed.resources.total_energy().to_bits(),
            plain.resources.total_energy().to_bits()
        );
        assert_eq!(format!("{:?}", observed.rng), format!("{:?}", plain.rng));
    }
    let m = observed.mimicry_value();
    let d = observed.mimicry_diagnostics_value();
    assert!(observed.carnivore_hunts > 0, "the test needs attacks");
    for (g, name) in ["defended", "mimic", "other"].iter().enumerate() {
        for (total, strata) in [
            ("attacks", "strata_attacks"),
            ("individual_ticks", "strata_exposure"),
        ] {
            let sum: u64 = (0..8).map(|s| d[strata][g][s].as_u64().unwrap()).sum();
            assert_eq!(sum, m[total][name].as_u64().unwrap(), "{strata} {name}");
        }
    }
    let shares = d["carnivore_memory"]["avoids_own_harm_color"].clone();
    assert!(shares.is_null() || (0. ..=1.).contains(&shares.as_f64().unwrap()));
    assert!(
        d["memory_entries_created"].as_u64().unwrap() >= d["harmful_evictions"].as_u64().unwrap()
    );
}

/// Issue #76 round 1: the carnivore learning gain is opt-in; gain 1 equals the absent value,
/// and invalid gains are rejected.
#[test]
fn carnivore_learning_gain_is_opt_in() {
    let config = |gain: Option<f64>| {
        let mut c: SimulationConfig =
            serde_json::from_str(include_str!("../../../config/stage4-standard.json")).unwrap();
        c.animal_toxin.herbivore_toxin_decay_per_tick = 0.01;
        c.animal_toxin.carnivore_damage_threshold = Some(0.03);
        c.herbivores.carnivore_offspring_probability = 0.02;
        c.animal_toxin.carnivore_learning_gain = gain;
        c
    };
    let mut absent = World::new(config(None)).unwrap();
    let mut one = World::new(config(Some(1.))).unwrap();
    for _ in 0..30 {
        absent.step(100).unwrap();
        one.step(100).unwrap();
        assert_eq!(absent.animals.items.len(), one.animals.items.len());
        assert_eq!(absent.carnivores.len(), one.carnivores.len());
        assert_eq!(format!("{:?}", absent.rng), format!("{:?}", one.rng));
    }
    assert_eq!(absent.mimicry_json(), one.mimicry_json());
    for bad in [0., -1., f64::NAN, 1e9] {
        assert!(World::new(config(Some(bad))).is_err(), "gain {bad}");
    }
    let s = serde_json::to_string(&config(None)).unwrap();
    assert!(!s.contains("carnivore_learning_gain"));
}
