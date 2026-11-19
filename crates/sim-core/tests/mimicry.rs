//! Issue #72: mimicry indicators are reported and consistent.
use sim_core::{SimulationConfig, World};

#[test]
fn mimicry_indicators_count_every_herbivore_once() {
    let c: SimulationConfig =
        serde_json::from_str(include_str!("../../../config/stage4-standard-20261119.json")).unwrap();
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
            serde_json::from_str(include_str!("../../../config/stage4-standard-20261119.json")).unwrap();
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
        serde_json::from_str(include_str!("../../../config/stage4-standard-20261119.json")).unwrap();
    bad.animal_toxin.herbivore_toxin_decay_per_tick = 1.;
    assert!(World::new(bad).is_err());
}

/// Issue #76 (observation only): the strata partition the attack and exposure counters,
/// and observing does not change the run.
#[test]
fn mimicry_diagnostics_partition_counters_and_do_not_change_the_run() {
    let config = || {
        let mut c: SimulationConfig =
            serde_json::from_str(include_str!("../../../config/stage4-standard-20261119.json")).unwrap();
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
            serde_json::from_str(include_str!("../../../config/stage4-standard-20261119.json")).unwrap();
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

/// Issue #76 round 4: body-color mutation is opt-in, uses its own stream, and with carnivore
/// color learning off it changes colors only, not the ecology.
#[test]
fn body_color_mutation_is_opt_in_and_color_only_without_learning() {
    let config = |p: Option<f32>, learning: bool| {
        let mut c: SimulationConfig =
            serde_json::from_str(include_str!("../../../config/stage4-standard-20261119.json")).unwrap();
        c.herbivores.carnivore_offspring_probability = 0.02;
        c.animal_toxin.color_learning_enabled = learning;
        c.herbivores.body_color_mutation_probability = p;
        c
    };
    let colors = |w: &World| -> Vec<[u32; 3]> {
        w.animals
            .items
            .iter()
            .map(|a| a.genes.body_color.rgb().map(f32::to_bits))
            .collect()
    };
    let mut absent = World::new(config(None, true)).unwrap();
    let mut same = World::new(config(Some(0.05), true)).unwrap();
    let mut plain = World::new(config(None, false)).unwrap();
    let mut varied = World::new(config(Some(0.5), false)).unwrap();
    for _ in 0..20 {
        for w in [&mut absent, &mut same, &mut plain, &mut varied] {
            w.step(100).unwrap();
        }
        assert_eq!(colors(&absent), colors(&same));
        assert_eq!(format!("{:?}", absent.rng), format!("{:?}", same.rng));
        assert_eq!(plain.animals.items.len(), varied.animals.items.len());
        assert_eq!(plain.carnivores.len(), varied.carnivores.len());
        assert_eq!(format!("{:?}", plain.rng), format!("{:?}", varied.rng));
    }
    assert_ne!(colors(&plain), colors(&varied));
    let mut bad = config(Some(1.5), true);
    assert!(World::new(bad.clone()).is_err());
    bad.herbivores.body_color_mutation_probability = None;
    bad.herbivores.body_color_mutation_scale = Some(-0.1);
    assert!(World::new(bad).is_err());
    let s = serde_json::to_string(&config(None, true)).unwrap();
    assert!(!s.contains("body_color_mutation"));
}

/// Issue #76 revised M1: the choice-scene observation does not change the run, and without
/// carnivore color learning no prey is ever passed over because of its color.
#[test]
fn choice_scene_observation_is_observation_only() {
    let config = |learning: bool| {
        let mut c: SimulationConfig =
            serde_json::from_str(include_str!("../../../config/stage4-standard-20261119.json")).unwrap();
        c.herbivores.carnivore_offspring_probability = 0.02;
        c.animal_toxin.herbivore_toxin_decay_per_tick = 0.01;
        c.animal_toxin.carnivore_damage_threshold = Some(0.01);
        c.animal_toxin.carnivore_learning_gain = Some(100.);
        c.animal_toxin.color_learning_enabled = learning;
        c
    };
    let counts = |w: &World, key: &str| -> Vec<u64> {
        let d = w.mimicry_diagnostics_value();
        (0..3).map(|g| d[key][g].as_u64().unwrap()).collect()
    };
    let mut observed = World::new(config(true)).unwrap();
    observed.enable_mimicry_choice_observation();
    let mut plain = World::new(config(true)).unwrap();
    let mut control = World::new(config(false)).unwrap();
    control.enable_mimicry_choice_observation();
    for _ in 0..30 {
        for w in [&mut observed, &mut plain, &mut control] {
            w.step(100).unwrap();
        }
        assert_eq!(observed.animals.items.len(), plain.animals.items.len());
        assert_eq!(observed.carnivores.len(), plain.carnivores.len());
        assert_eq!(format!("{:?}", observed.rng), format!("{:?}", plain.rng));
        assert_eq!(observed.mimicry_json(), plain.mimicry_json());
    }
    assert_eq!(counts(&plain, "choice_first"), vec![0, 0, 0]);
    assert_eq!(counts(&control, "choice_skipped"), vec![0, 0, 0]);
    assert_eq!(counts(&control, "choice_any_risk"), vec![0, 0, 0]);
    let first = counts(&observed, "choice_first");
    let skipped = counts(&observed, "choice_skipped");
    let if_fed = counts(&observed, "choice_refused_if_fed");
    let any = counts(&observed, "choice_any_risk");
    for g in 0..3 {
        // A fed carnivore tolerates the least risk, so it refuses at least as often.
        assert!(skipped[g] <= if_fed[g] && if_fed[g] <= any[g] && any[g] <= first[g]);
    }
    // Dose accounting by prey group adds up to the toxin moved to carnivores.
    let d = observed.mimicry_diagnostics_value();
    let t: serde_json::Value =
        serde_json::from_str(&observed.animal_toxin_snapshot_json()).unwrap();
    for (by_group, total) in [
        ("dose_produced", "transferred_produced"),
        ("dose_plant", "transferred_residual"),
    ] {
        let a: f64 = (0..3).map(|g| d[by_group][g].as_f64().unwrap()).sum();
        let b: f64 = t[total]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .sum();
        assert!((a - b).abs() <= 1e-9 * b.max(1.), "{by_group} {a} vs {b}");
    }
}

/// Issue #76 rounds 6+: the herbivore toxic trait and the plant-toxin transfer threshold are
/// opt-in; non-toxic herbivores make no toxin, and a high threshold keeps plant toxin in prey.
#[test]
fn toxic_trait_and_plant_threshold_are_opt_in() {
    let config = |fraction: Option<f64>, threshold: f64| {
        let mut c: SimulationConfig =
            serde_json::from_str(include_str!("../../../config/stage4-standard-20261119.json")).unwrap();
        c.herbivores.carnivore_offspring_probability = 0.02;
        c.animal_toxin.herbivore_toxin_decay_per_tick = 0.01;
        c.animal_toxin.herbivore_toxic_initial_fraction = fraction;
        c.animal_toxin.plant_toxin_transfer_threshold = threshold;
        c
    };
    let mut w = World::new(config(Some(0.5), 100.)).unwrap();
    for _ in 0..25 {
        w.step(100).unwrap();
    }
    let d = w.mimicry_diagnostics_value();
    let t: serde_json::Value = serde_json::from_str(&w.animal_toxin_snapshot_json()).unwrap();
    let residual: f64 = t["transferred_residual"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .sum();
    assert_eq!(residual, 0.);
    for e in t["balance_error"].as_array().unwrap() {
        assert!(e.as_f64().unwrap().abs() < 1e-6, "balance {e}");
    }
    let shares: Vec<f64> = ["defended", "mimic", "other"]
        .iter()
        .filter_map(|g| d["profile"][g]["toxic_trait"].as_f64())
        .collect();
    assert!(!shares.is_empty());
    // Defended herbivores hold toxin, so they must carry the toxic trait.
    if let Some(share) = d["profile"]["defended"]["toxic_trait"].as_f64() {
        assert_eq!(share, 1.);
    }
    let m = w.mimicry_value();
    let nontoxic = shares.iter().any(|&s| s < 1.);
    assert!(nontoxic || m["counts"]["other"].as_u64() == Some(0));
    for bad in [config(Some(1.5), 0.), config(None, -1.), {
        let mut c = config(None, 0.);
        c.animal_toxin.herbivore_toxic_mutation_probability = 0.01;
        c
    }] {
        assert!(World::new(bad).is_err());
    }
    let s = serde_json::to_string(&config(None, 0.)).unwrap();
    assert!(!s.contains("herbivore_toxic") && !s.contains("plant_toxin_transfer"));
}
