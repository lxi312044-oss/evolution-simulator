//! Issue #80: herbivores weigh the danger of carnivores against hunger (opt-in).
use sim_core::{config::HerbivoreRiskSettings, SimulationConfig, World};

/// A quarter of the current standard world (same cell size and densities) with a few
/// carnivores from the start, so herbivores meet carnivores at once.
fn quarter_standard() -> SimulationConfig {
    let mut c: SimulationConfig =
        serde_json::from_str(include_str!("../../../config/stage4-standard.json")).unwrap();
    c.world.width = 1000.;
    c.world.height = 1000.;
    c.world.land_columns = 20;
    c.world.land_rows = 20;
    c.initial_population.plants = 300;
    c.initial_population.herbivores = 80;
    c.initial_population.carnivores = 5;
    c.herbivore_risk = None;
    c
}

fn run(config: SimulationConfig) -> World {
    let mut w = World::new(config).unwrap();
    w.enable_risk_observation();
    for _ in 0..10 {
        // Fails if any Tick's conservation check fails.
        w.step(100).unwrap();
    }
    w
}

fn pursuit_in_danger(w: &World) -> u64 {
    let r = w.risk_observation_json();
    (0..3)
        .map(|b| r["pursue_in_danger"][b].as_u64().unwrap())
        .sum()
}

/// Without the setting nothing changes (the setting is absent from the serialized config);
/// with it, hungry herbivores go for food near carnivores, which the previous rule never does.
#[test]
fn herbivore_risk_is_opt_in() {
    let control = run(quarter_standard());
    let s = serde_json::to_string(&quarter_standard()).unwrap();
    assert!(!s.contains("herbivore_risk"));
    assert_eq!(pursuit_in_danger(&control), 0);
    let mut c = quarter_standard();
    c.herbivore_risk = Some(HerbivoreRiskSettings::default());
    let risky = run(c);
    let r = risky.risk_observation_json();
    assert!(
        r["flee_ticks"].as_u64().unwrap() > 0,
        "the test needs danger"
    );
    assert!(pursuit_in_danger(&risky) > 0);
}

/// Invalid settings are rejected.
#[test]
fn herbivore_risk_settings_are_validated() {
    let with = |change: &dyn Fn(&mut HerbivoreRiskSettings)| {
        let mut c = quarter_standard();
        let mut r = HerbivoreRiskSettings::default();
        change(&mut r);
        c.herbivore_risk = Some(r);
        c
    };
    assert!(World::new(with(&|_| {})).is_ok());
    for bad in [
        with(&|r| r.hunger_weight = -1.),
        with(&|r| r.flee_until_factor = 0.5),
        with(&|r| r.memory_ticks = 200_000),
        with(&|r| r.plant_danger_factor = f64::NAN),
    ] {
        assert!(World::new(bad).is_err());
    }
}
