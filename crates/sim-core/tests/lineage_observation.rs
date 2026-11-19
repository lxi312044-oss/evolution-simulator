use sim_core::{lineage::LineageObservation, SimulationConfig, World};
#[test]
fn observer_records_real_births_and_validates_lifecycle() {
    let mut c = SimulationConfig::default();
    c.initial_population.plants = 0;
    c.initial_population.herbivores = 1;
    c.initial_population.carnivores = 0;
    c.herbivores.reproduction_min_age = 0;
    c.herbivores.reproduction_cooldown_ticks = 1;
    c.herbivores.reproduction_min_energy_fraction = 0.;
    c.herbivores.reproduction_min_hp_fraction = 0.;
    c.herbivores.offspring_body_nutrients = 1.;
    c.herbivores.offspring_reserve_nutrients = 1.;
    c.herbivores.offspring_energy = 1.;
    c.herbivores.reproduction_work_energy = 0.;
    let mut w = World::new(c).unwrap();
    let mut o = LineageObservation::new(&mut w, 1).unwrap();
    assert_eq!(o.take_events().len(), 2);
    o.before_step(&w).unwrap();
    w.step(1).unwrap();
    o.after_step(&mut w).unwrap();
    o.finish(&w).unwrap();
    let rows = o.take_events();
    assert!(rows
        .iter()
        .any(|e| e["kind"] == "birth" && e["generation"] == 1));
    assert_eq!(rows.last().unwrap()["kind"], "complete");
    assert!(o.before_step(&w).is_err());
    assert!(o.finish(&w).is_err());
    assert!(o.take_events().is_empty());
}
#[test]
fn observer_rejects_late_start_zero_horizon_and_early_finish() {
    let mut w = World::new(SimulationConfig::default()).unwrap();
    assert!(LineageObservation::new(&mut w, 0).is_err());
    let mut o = LineageObservation::new(&mut w, 10).unwrap();
    assert!(o.finish(&w).is_err());
    w.step(1).unwrap();
    assert!(LineageObservation::new(&mut w, 10).is_err());
}
