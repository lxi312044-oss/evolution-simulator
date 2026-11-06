use sim_core::{SimulationConfig, World};
#[test]
fn initial_plants_and_soil_are_created() {
    let w = World::new(SimulationConfig::default()).unwrap();
    assert_eq!(w.plant_count(), 300);
    assert_eq!(w.land.cells.len(), 400);
    assert_eq!(w.animal_count(), 80);
    assert!(w.total_soil() > 0.);
}
#[test]
fn thousand_ticks_conserve_nutrients_and_account_for_energy() {
    let mut config = SimulationConfig::default();
    // This test isolates the plant ecosystem; herbivore predation is tested separately.
    config.initial_population.herbivores = 0;
    let mut w = World::new(config).unwrap();
    let original = w.resources.total_nutrients();
    w.step(1000).unwrap();
    assert_eq!(w.tick, 1000);
    assert!((w.resources.total_nutrients() - original).abs() < 1e-5);
    assert!(w.ledger.sunlight_in > 0.);
    assert!(w.ledger.heat_out > 0.);
    assert!(w.plant_count() > 0);
}
#[test]
fn plants_die_and_return_nutrients_to_detritus() {
    let mut c = SimulationConfig::default();
    c.plants.max_age_ticks = 2;
    c.plants.mineralization_rate = 0.;
    let mut w = World::new(c).unwrap();
    let original = w.resources.total_nutrients();
    w.step(2).unwrap();
    assert_eq!(w.plant_count(), 0);
    assert!(w.deaths >= 300);
    assert!((w.resources.total_nutrients() - original).abs() < 1e-5);
}
#[test]
fn same_seed_produces_same_state() {
    let mut a = World::new(SimulationConfig::default()).unwrap();
    let mut b = World::new(SimulationConfig::default()).unwrap();
    a.step(400).unwrap();
    b.step(400).unwrap();
    assert_eq!(a.render_snapshot(), b.render_snapshot());
    assert_eq!(a.births, b.births);
    assert_eq!(a.resources.total_nutrients(), b.resources.total_nutrients());
}
#[test]
fn seeds_use_parent_resources_not_new_material() {
    let mut c = SimulationConfig::default();
    // Isolate plant seed reproduction from herbivore grazing.
    c.initial_population.herbivores = 0;
    c.plants.seed_probability = 1.;
    c.plants.seed_cooldown_ticks = 1;
    c.plants.seed_body_nutrients = 0.5;
    c.plants.seed_energy = 0.5;
    c.plants.seed_work_energy = 0.1;
    let mut w = World::new(c).unwrap();
    let original = w.resources.total_nutrients();
    w.step(500).unwrap();
    assert!(w.births > 0);
    assert!((w.resources.total_nutrients() - original).abs() < 1e-5);
}
