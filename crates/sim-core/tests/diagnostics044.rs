use sim_core::{SimulationConfig, World};
#[test]
fn diagnostic_snapshot_does_not_change_deterministic_simulation() {
    let config = SimulationConfig::default();
    let mut a = World::new(config.clone()).unwrap();
    let mut b = World::new(config).unwrap();
    for _ in 0..10 {
        let snapshot = a.ecology_snapshot();
        assert!(snapshot["soil_min_cell_n"].as_f64().unwrap() >= 0.);
        a.step(10).unwrap();
        b.step(10).unwrap();
    }
    assert_eq!(a.tick, b.tick);
    assert_eq!(a.plant_count(), b.plant_count());
    assert_eq!(a.herbivore_count(), b.herbivore_count());
    assert_eq!(a.resources.total_nutrients(), b.resources.total_nutrients());
    assert_eq!(a.resources.total_energy(), b.resources.total_energy());
}
