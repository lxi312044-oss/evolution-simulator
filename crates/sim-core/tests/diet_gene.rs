//! Issue #78: heritable diet value.
use sim_core::{config::DietGeneSettings, SimulationConfig, World};

/// A quarter of the current standard world (same cell size and densities) with a few
/// carnivores from the start, so the test sees carnivore births and stays fast.
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
