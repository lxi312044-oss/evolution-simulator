use sim_core::{SimulationConfig, World};
#[test]
fn trait_snapshot_is_read_only_and_deterministic() {
    let config=SimulationConfig::default();
    let mut a=World::new(config.clone()).unwrap();
    let mut b=World::new(config).unwrap();
    assert_eq!(a.behavior_genetics_snapshot(), b.behavior_genetics_snapshot());
    let before=a.behavior_genetics_snapshot();
    assert_eq!(before,a.behavior_genetics_snapshot());
    a.step(20).unwrap(); b.step(20).unwrap();
    assert_eq!(a.behavior_genetics_snapshot(),b.behavior_genetics_snapshot());
}
#[test]
fn zero_herbivores_has_finite_statistics() {
    let mut config=SimulationConfig::default();
    config.initial_population.herbivores=0;
    let world=World::new(config).unwrap();
    let snapshot=world.behavior_genetics_snapshot();
    assert_eq!(snapshot["herbivores"],0);
    assert_eq!(snapshot["fear_mean"],0.0);
}
