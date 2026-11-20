//! Issue #78: heritable diet value.
use sim_core::{config::DietGeneSettings, SimulationConfig, World};

/// A quarter of the 2026-11-20 standard world (same cell size and densities, no diet value)
/// with a few carnivores from the start, so the test sees carnivore births and stays fast.
fn quarter_standard() -> SimulationConfig {
    let mut c: SimulationConfig = serde_json::from_str(include_str!(
        "../../../config/stage4-standard-20261120.json"
    ))
    .unwrap();
    c.world.width = 1000.;
    c.world.height = 1000.;
    c.world.land_columns = 20;
    c.world.land_rows = 20;
    c.initial_population.plants = 300;
    c.initial_population.herbivores = 80;
    c.initial_population.carnivores = 5;
    c.herbivores.carnivore_offspring_probability = 0.02;
    c
}

/// Stage 0: observing the diet value changes nothing, and its birth counts match the
/// existing birth counters.
#[test]
fn diet_observation_changes_nothing() {
    let mut observed_config = quarter_standard();
    observed_config.diet_gene = Some(DietGeneSettings::default());
    let mut observed = World::new(observed_config).unwrap();
    let mut plain = World::new(quarter_standard()).unwrap();
    assert!(plain.diet_value_json().is_null());
    for _ in 0..12 {
        observed.step(100).unwrap();
        plain.step(100).unwrap();
        assert_eq!(observed.animals.items.len(), plain.animals.items.len());
        assert_eq!(observed.carnivores.len(), plain.carnivores.len());
        assert_eq!(observed.carnivore_births, plain.carnivore_births);
        assert_eq!(
            observed.resources.total_energy().to_bits(),
            plain.resources.total_energy().to_bits()
        );
        assert_eq!(format!("{:?}", observed.rng), format!("{:?}", plain.rng));
    }
    let d = observed.diet_value_json();
    let n = |body: &str| d[body]["n"].as_u64().unwrap() as usize;
    assert_eq!(n("herbivore"), observed.animals.items.len());
    assert_eq!(n("carnivore"), observed.carnivores.len());
    for body in ["herbivore", "carnivore"] {
        let hist: u64 = (0..10).map(|i| d[body]["hist"][i].as_u64().unwrap()).sum();
        assert_eq!(hist as usize, n(body));
    }
    let births = |k: &str| d["births"][k].as_u64().unwrap();
    assert_eq!(
        births("herbivore_to_herbivore"),
        observed.diagnostics.herbivore_births
    );
    assert_eq!(
        births("herbivore_to_carnivore"),
        observed.defense.carnivores_born_from_herbivores
    );
    assert_eq!(
        births("herbivore_to_carnivore") + births("carnivore_to_carnivore"),
        observed.carnivore_births
    );
    assert!(
        births("herbivore_to_herbivore") > 0,
        "the test needs births"
    );
    assert_eq!(
        d["crossing_parents"]["herbivore_to_carnivore"]["n"].as_u64(),
        Some(births("herbivore_to_carnivore"))
    );
    assert_eq!(births("carnivore_to_herbivore"), 0);
    // Founder herbivores start around 0.1 and the values stay in [0, 1].
    let mean = d["herbivore"]["mean"].as_f64().unwrap();
    assert!((0. ..=1.).contains(&mean));
    let s = serde_json::to_string(&quarter_standard()).unwrap();
    assert!(!s.contains("diet_gene"));
}

/// Invalid diet settings are rejected; letting the value act needs carnivores on and the
/// fixed herbivore-to-carnivore probability at 0.
#[test]
fn diet_settings_are_validated() {
    let with = |change: &dyn Fn(&mut DietGeneSettings)| {
        let mut c = quarter_standard();
        let mut d = DietGeneSettings::default();
        change(&mut d);
        c.diet_gene = Some(d);
        c
    };
    assert!(World::new(with(&|_| {})).is_ok());
    for bad in [
        with(&|d| d.initial_herbivore = 0.5),
        with(&|d| d.initial_carnivore = 0.4),
        with(&|d| d.initial_spread = 0.6),
        with(&|d| d.mutation_probability = 1.5),
        with(&|d| d.mutation_scale = f64::NAN),
        with(&|d| d.efficiency_exponent = -1.),
        // effects with the fixed probability still set
        with(&|d| d.effects = true),
    ] {
        assert!(World::new(bad).is_err());
    }
    let mut effects = with(&|d| d.effects = true);
    effects.herbivores.carnivore_offspring_probability = 0.;
    effects.initial_population.carnivores = 0;
    assert!(World::new(effects.clone()).is_ok());
    effects.carnivores.enabled = false;
    assert!(World::new(effects).is_err());
}

fn acting(change: impl Fn(&mut SimulationConfig, &mut DietGeneSettings)) -> SimulationConfig {
    let mut c = quarter_standard();
    c.herbivores.carnivore_offspring_probability = 0.;
    let mut d = DietGeneSettings {
        effects: true,
        ..Default::default()
    };
    change(&mut c, &mut d);
    c.diet_gene = Some(d);
    c
}

/// Stage 1: herbivores at value 0 and carnivores at value 1 that never mutate live exactly as
/// without the diet value (same gains, same bodies at birth).
#[test]
fn pure_values_keep_current_gains() {
    let mut control_config = quarter_standard();
    control_config.herbivores.carnivore_offspring_probability = 0.;
    let mut control = World::new(control_config).unwrap();
    let mut pure = World::new(acting(|_, d| {
        d.initial_herbivore = 0.;
        d.initial_carnivore = 1.;
        d.initial_spread = 0.;
        d.mutation_probability = 0.;
    }))
    .unwrap();
    for _ in 0..12 {
        control.step(100).unwrap();
        pure.step(100).unwrap();
        assert_eq!(control.animals.items.len(), pure.animals.items.len());
        assert_eq!(control.carnivores.len(), pure.carnivores.len());
        assert_eq!(
            control.resources.total_energy().to_bits(),
            pure.resources.total_energy().to_bits()
        );
        assert_eq!(format!("{:?}", control.rng), format!("{:?}", pure.rng));
    }
    assert!(control.carnivore_hunts > 0, "the test needs bites");
    let lost = &pure.diet_value_json()["lost"];
    for k in [
        "herbivore_nutrients",
        "herbivore_energy",
        "carnivore_nutrients",
        "carnivore_energy",
    ] {
        assert_eq!(lost[k].as_f64(), Some(0.), "{k}");
    }
}

/// Stage 1: with values near 0.5, offspring cross the boundary in both directions, every
/// animal's body matches its value, intermediate values lose gains, and the conservation check
/// of every Tick passes (`step` fails otherwise).
#[test]
fn diet_effects_cross_both_ways() {
    let mut w = World::new(acting(|c, d| {
        c.carnivores.reproduction_cooldown_ticks = 150;
        c.carnivores.food_limited_reproduction_enabled = false;
        c.carnivores.prey_trend_reproduction_enabled = false;
        d.initial_herbivore = 0.45;
        d.initial_carnivore = 0.55;
        d.initial_spread = 0.04;
        d.mutation_probability = 1.;
        d.mutation_scale = 0.15;
        d.efficiency_exponent = 0.1;
    }))
    .unwrap();
    for _ in 0..15 {
        w.step(100).unwrap();
    }
    let d = w.diet_value_json();
    let births = |k: &str| d["births"][k].as_u64().unwrap();
    assert!(births("herbivore_to_carnivore") > 0);
    assert!(births("carnivore_to_herbivore") > 0);
    // Crossings come from parents near the boundary: a herbivore parent's value is below
    // 0.5 and a carnivore parent's at least 0.5.
    let parents = &d["crossing_parents"];
    for (k, lo, hi) in [
        ("herbivore_to_carnivore", 0., 0.5),
        ("carnivore_to_herbivore", 0.5, 1.),
    ] {
        assert_eq!(parents[k]["n"].as_u64(), Some(births(k)));
        let mean = parents[k]["mean"].as_f64().unwrap();
        assert!((lo..hi).contains(&mean) || mean == hi, "{k} {mean}");
    }
    for i in 0..5 {
        assert_eq!(d["herbivore"]["hist"][i + 5].as_u64(), Some(0));
        assert_eq!(d["carnivore"]["hist"][i].as_u64(), Some(0));
    }
    for k in ["herbivore_energy", "carnivore_energy"] {
        assert!(d["lost"][k].as_f64().unwrap() > 0., "{k}");
    }
}

/// The standard is the 2026-11-20 standard plus the condition that ranked first in Issue #78
/// stage 2 (`DE_H20_M15_K50` in `examples/diet_goal.rs`: carnivores are born from herbivores
/// through the diet value instead of a fixed probability) and the Issue #80 choice (`RK`:
/// herbivores weigh carnivore danger against hunger with the default values).
#[test]
fn standard_is_the_20261120_standard_plus_issue78_and_issue80() {
    let parse = |text: &str| serde_json::from_str::<SimulationConfig>(text).unwrap();
    let standard = parse(include_str!("../../../config/stage4-standard.json"));
    let mut expected = parse(include_str!(
        "../../../config/stage4-standard-20261120.json"
    ));
    expected.herbivores.carnivore_offspring_probability = 0.;
    expected.diet_gene = Some(DietGeneSettings {
        effects: true,
        initial_herbivore: 0.2,
        mutation_scale: 0.15,
        efficiency_exponent: 0.5,
        ..Default::default()
    });
    expected.herbivore_risk = Some(sim_core::config::HerbivoreRiskSettings::default());
    assert_eq!(
        serde_json::to_value(&standard).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );
    World::new(standard).unwrap();
}

/// Issue #80: the flee/pursue observation changes nothing and its counters are consistent.
#[test]
fn risk_observation_changes_nothing() {
    let mut observed = World::new(quarter_standard()).unwrap();
    observed.enable_risk_observation();
    let mut plain = World::new(quarter_standard()).unwrap();
    assert!(plain.risk_observation_json().is_null());
    for _ in 0..10 {
        observed.step(100).unwrap();
        plain.step(100).unwrap();
        assert_eq!(observed.animals.items.len(), plain.animals.items.len());
        assert_eq!(observed.carnivores.len(), plain.carnivores.len());
        assert_eq!(
            observed.resources.total_energy().to_bits(),
            plain.resources.total_energy().to_bits()
        );
        assert_eq!(format!("{:?}", observed.rng), format!("{:?}", plain.rng));
    }
    let r = observed.risk_observation_json();
    let n = |k: &str| r[k].as_u64().unwrap();
    assert!(n("herbivore_ticks") > 0);
    assert!(n("switches") <= n("herbivore_ticks"));
    assert!(
        n("flee_ticks") > 0,
        "the test needs carnivores near herbivores"
    );
    let danger: u64 = (0..3).map(|b| r["danger_ticks"][b].as_u64().unwrap()).sum();
    let pursue: u64 = (0..3)
        .map(|b| r["pursue_in_danger"][b].as_u64().unwrap())
        .sum();
    assert!(danger >= n("flee_ticks") && pursue <= danger);
}

/// Issue #82 stage 0: observing the tendency to hunt carnivores changes nothing.
#[test]
fn apex_observation_changes_nothing() {
    let mut observed_config = quarter_standard();
    observed_config.apex_predation = Some(sim_core::config::ApexPredationSettings::default());
    let mut observed = World::new(observed_config).unwrap();
    let mut plain = World::new(quarter_standard()).unwrap();
    assert!(plain.apex_json().is_null());
    for _ in 0..10 {
        observed.step(100).unwrap();
        plain.step(100).unwrap();
        assert_eq!(observed.animals.items.len(), plain.animals.items.len());
        assert_eq!(observed.carnivores.len(), plain.carnivores.len());
        assert_eq!(
            observed.resources.total_energy().to_bits(),
            plain.resources.total_energy().to_bits()
        );
        assert_eq!(format!("{:?}", observed.rng), format!("{:?}", plain.rng));
    }
    let a = observed.apex_json();
    assert_eq!(
        a["carnivore"]["n"].as_u64(),
        Some(observed.carnivores.len() as u64)
    );
    assert_eq!(a["bites"].as_u64(), Some(0));
    let mean = a["herbivore"]["mean"].as_f64().unwrap();
    assert!((0. ..=1.).contains(&mean));
    assert!(!serde_json::to_string(&quarter_standard())
        .unwrap()
        .contains("apex_predation"));
}

/// Issue #82 stage 1: with the tendency acting but every value 0 nothing changes; with high
/// values carnivores may bite each other and every Tick's conservation check passes.
#[test]
fn apex_effects_with_zero_tendency_change_nothing() {
    let with = |initial_max: f64| {
        let mut c = quarter_standard();
        c.apex_predation = Some(sim_core::config::ApexPredationSettings {
            effects: true,
            initial_max,
            mutation_probability: 0.,
            ..Default::default()
        });
        c
    };
    let mut zero = World::new(with(0.)).unwrap();
    let mut plain = World::new(quarter_standard()).unwrap();
    let mut high = World::new(with(1.)).unwrap();
    for _ in 0..10 {
        zero.step(100).unwrap();
        plain.step(100).unwrap();
        high.step(100).unwrap();
        assert_eq!(zero.animals.items.len(), plain.animals.items.len());
        assert_eq!(zero.carnivores.len(), plain.carnivores.len());
        assert_eq!(format!("{:?}", zero.rng), format!("{:?}", plain.rng));
    }
    assert_eq!(zero.apex_json()["bites"].as_u64(), Some(0));
}
