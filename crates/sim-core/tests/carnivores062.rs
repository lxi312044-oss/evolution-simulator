use sim_core::{SimulationConfig, World};
#[test]
fn default_off_preserves_existing_world() {
    let c = SimulationConfig::default();
    let w = World::new(c).unwrap();
    assert!(w.carnivores.is_empty());
}
#[test]
fn enabled_carnivores_are_deterministic_and_conservative() {
    let mut c = SimulationConfig::default();
    c.carnivores.enabled = true;
    c.initial_population.carnivores = 3;
    let mut a = World::new(c.clone()).unwrap();
    let mut b = World::new(c).unwrap();
    assert_eq!(a.carnivore_snapshot(), b.carnivore_snapshot());
    for _ in 0..200 {
        a.step(1).unwrap();
        b.step(1).unwrap();
    }
    assert_eq!(a.carnivore_snapshot(), b.carnivore_snapshot());
    assert_eq!(a.carnivore_hunts, b.carnivore_hunts);
    assert!((a.resources.total_nutrients() - a.baseline.nutrients).abs() < 1e-3);
}
