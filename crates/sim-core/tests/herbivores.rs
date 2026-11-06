use sim_core::{SimulationConfig, World};
#[test]
fn eighty_herbivores_spawn_and_move() {
    let mut w = World::new(SimulationConfig::default()).unwrap();
    assert_eq!(w.herbivore_count(), 80);
    let before = w.herbivore_snapshot();
    w.step(10).unwrap();
    assert!(w
        .herbivore_snapshot()
        .iter()
        .zip(before.iter())
        .any(|(a, b)| a[0] != b[0] || a[1] != b[1]));
}
#[test]
fn nutrient_and_energy_ledgers_survive_feeding() {
    let mut w = World::new(SimulationConfig::default()).unwrap();
    let n = w.resources.total_nutrients();
    w.step(1000).unwrap();
    assert!((w.resources.total_nutrients() - n).abs() < 1e-5);
    assert!(w.ledger.heat_out > 0.);
}
#[test]
fn starvation_causes_death_without_creating_nutrients() {
    let mut c = SimulationConfig::default();
    c.initial_population.plants = 0;
    c.herbivores.initial_energy_fraction = 0.;
    c.herbivores.starvation_hp_fraction = 0.5;
    let mut w = World::new(c).unwrap();
    let n = w.resources.total_nutrients();
    w.step(3).unwrap();
    assert_eq!(w.herbivore_count(), 0);
    assert!((w.resources.total_nutrients() - n).abs() < 1e-5);
}
#[test]
fn same_seed_reproduces_herbivore_positions() {
    let mut a = World::new(SimulationConfig::default()).unwrap();
    let mut b = World::new(SimulationConfig::default()).unwrap();
    a.step(100).unwrap();
    b.step(100).unwrap();
    assert_eq!(a.herbivore_snapshot(), b.herbivore_snapshot());
}

#[test]
fn prolonged_starvation_movement_never_overdraws_energy() {
    let mut c = SimulationConfig::default();
    c.initial_population.plants = 0;
    c.herbivores.initial_energy_fraction = 0.000001;
    c.herbivores.basal_energy_per_tick = 0.0;
    c.herbivores.starvation_hp_fraction = 0.0;
    let mut w = World::new(c).unwrap();
    let n = w.resources.total_nutrients();
    for _ in 0..20 {
        w.step(100).unwrap();
    }
    assert!((w.resources.total_nutrients() - n).abs() < 1e-5);
    assert!(w.herbivore_energy() >= 0.0);
}
#[test]
fn default_ecosystem_survives_3000_ticks_without_resource_overdraw() {
    let mut w = World::new(SimulationConfig::default()).unwrap();
    for _ in 0..30 {
        w.step(100).unwrap();
    }
}

#[test]
fn initial_herbivore_lifetimes_are_not_synchronized() {
    let w = World::new(SimulationConfig::default()).unwrap();
    let first = w.animals.items[0].max_age;
    assert!(w.animals.items.iter().any(|a| a.max_age != first));
    assert!(w.animals.items.iter().any(|a| a.age > 0));
}
#[test]
fn empty_world_exploration_is_directionally_persistent() {
    let mut c = SimulationConfig::default();
    c.initial_population.plants = 0;
    c.initial_population.herbivores = 1;
    c.herbivores.basal_energy_per_tick = 0.;
    let mut w = World::new(c).unwrap();
    let a = &w.animals.items[0];
    let (x0, y0) = (a.x, a.y);
    w.step(10).unwrap();
    let a = &w.animals.items[0];
    let displacement = ((a.x - x0).powi(2) + (a.y - y0).powi(2)).sqrt();
    assert!(
        displacement > 10.,
        "expected sustained exploration, got {displacement}"
    );
}
#[test]
fn zero_energy_does_not_move_or_overdraw() {
    let mut c = SimulationConfig::default();
    c.initial_population.plants = 0;
    c.initial_population.herbivores = 1;
    c.herbivores.initial_energy_fraction = 0.;
    c.herbivores.starvation_hp_fraction = 0.;
    let mut w = World::new(c).unwrap();
    let (x, y) = (w.animals.items[0].x, w.animals.items[0].y);
    w.step(10).unwrap();
    assert_eq!((x, y), (w.animals.items[0].x, w.animals.items[0].y));
}
#[test]
fn lifetime_does_not_kill_every_survivor_at_tick_6000() {
    let mut c = SimulationConfig::default();
    c.initial_population.plants = 0;
    // Test the distribution of initial lifetimes independently of phase-05 reproduction.
    c.herbivores.reproduction_enabled = false;
    c.herbivores.initial_energy_fraction = 1.;
    c.herbivores.basal_energy_per_tick = 0.;
    c.herbivores.movement_energy_per_unit = 0.;
    c.herbivores.starvation_hp_fraction = 0.;
    let mut w = World::new(c).unwrap();
    w.step(1000).unwrap();
    for _ in 1..6 {
        w.step(1000).unwrap();
    }
    assert!(
        w.herbivore_count() > 0,
        "all herbivores died synchronously at tick 6000"
    );
    assert!(
        w.herbivore_count() < 80,
        "individual lifetimes should be distributed"
    );
}
