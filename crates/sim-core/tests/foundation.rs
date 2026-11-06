use sim_core::{SimulationConfig, World};
use sim_core::entity::EntityAllocator;
use sim_core::resources::{ResourceBank, ResourceLedger, ConservationBaseline, ResourceError};

#[test]
fn generational_ids_reject_stale_references() {
    let mut a = EntityAllocator::new();
    let first = a.spawn().unwrap();
    assert!(a.contains(first));
    assert!(a.despawn(first));
    assert!(!a.contains(first));
    assert!(!a.despawn(first));
    let next = a.spawn().unwrap();
    assert_eq!(first.index, next.index);
    assert_ne!(first.generation, next.generation);
    assert!(!a.contains(first));
    assert!(a.contains(next));
}
#[test]
fn transfer_is_atomic_and_conserves_nutrients() {
    let mut b = ResourceBank::new();
    let soil = b.open(100.0, 0.0).unwrap();
    let plant = b.open(0.0, 0.0).unwrap();
    let base = ConservationBaseline::capture(&b);
    b.transfer_nutrients(soil, plant, 20.0).unwrap();
    assert_eq!(b.nutrient_balance(soil), Some(80.0));
    assert_eq!(b.nutrient_balance(plant), Some(20.0));
    assert_eq!(b.transfer_nutrients(soil, plant, 90.0), Err(ResourceError::InsufficientBalance));
    assert_eq!(b.nutrient_balance(soil), Some(80.0));
    assert_eq!(b.transfer_nutrients(soil, plant, f64::NAN), Err(ResourceError::InvalidAmount));
    base.check(&b, &ResourceLedger::default(), 1e-9, 1e-9).unwrap();
}
#[test]
fn energy_ledger_accounts_for_sunlight_and_heat() {
    let mut b = ResourceBank::new();
    let plant = b.open(30.0, 10.0).unwrap();
    let animal = b.open(10.0, 0.0).unwrap();
    let base = ConservationBaseline::capture(&b);
    let mut ledger = ResourceLedger::default();
    ledger.capture_sunlight(&mut b, plant, 8.0).unwrap();
    b.transfer_energy(plant, animal, 5.0).unwrap();
    ledger.dissipate_heat(&mut b, animal, 2.0).unwrap();
    base.check(&b, &ledger, 1e-9, 1e-9).unwrap();
    assert_eq!(b.total_energy(), 16.0);
}
#[test]
fn world_steps_1000_ticks() {
    let mut config = SimulationConfig::default();
    // Foundation test exercises the plant-only lifecycle, not predation.
    config.initial_population.herbivores = 0;
    let mut world = World::new(config).unwrap();
    world.step(1000).unwrap();
    assert_eq!(world.tick, 1000);
    assert_eq!(world.animal_count(), 0);
    assert!(world.plant_count() > 0);
}
#[test]
fn invalid_config_is_rejected() {
    let mut config = SimulationConfig::default();
    config.world.width = f32::NAN;
    assert!(World::new(config).is_err());
}
