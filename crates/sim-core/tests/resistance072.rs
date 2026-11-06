use sim_core::{SimulationConfig, World};
#[test]
fn resistance_parameters_are_validated() {
    let mut c = SimulationConfig::default();
    c.toxin.resistance_effectiveness = 1.1;
    assert!(c.validate().is_err());
    c.toxin.resistance_effectiveness = 0.8;
    c.toxin.resistance_energy_cost_per_tick = -1.;
    assert!(c.validate().is_err());
}
#[test]
fn inherited_resistance_is_bounded_and_upkeep_conserves_energy() {
    let mut c = SimulationConfig::default();
    c.toxin.enabled = true;
    c.toxin.resistance_enabled = true;
    c.toxin.initial_resistance = 0.75;
    c.initial_population.carnivores = 0;
    let mut w = World::new(c).unwrap();
    assert!(w
        .animals
        .items
        .iter()
        .all(|a| (a.genes.toxin_resistance - 0.75).abs() < 1e-6));
    for _ in 0..100 {
        w.step(1).unwrap();
    }
    assert!(w.diagnostics.resistance_energy_cost_total > 0.);
    assert!((w.resources.total_nutrients() - w.baseline.nutrients).abs() < 1e-5);
    let expected = w.baseline.energy + w.ledger.sunlight_total() - w.ledger.heat_total();
    assert!((w.resources.total_energy() - expected).abs() < 1e-4);
}
#[test]
fn resistance_reduces_toxin_hp_damage() {
    fn world(resistance: bool) -> World {
        let mut c = SimulationConfig::default();
        c.initial_population.plants = 0;
        c.initial_population.herbivores = 1;
        c.initial_population.carnivores = 0;
        c.toxin.enabled = true;
        c.toxin.resistance_enabled = resistance;
        c.toxin.initial_resistance = 1.;
        c.toxin.resistance_effectiveness = 1.;
        c.toxin.resistance_energy_cost_per_tick = 0.;
        c.toxin.clearance_fraction_per_tick = 0.;
        c.toxin.damage_threshold = 0.;
        c.toxin.hp_damage_per_excess_unit = 1.;
        let mut w = World::new(c).unwrap();
        w.animals.items[0].toxin_load = 5.;
        w
    }
    let mut a = world(false);
    let mut b = world(true);
    a.step(1).unwrap();
    b.step(1).unwrap();
    assert!(b.animals.items[0].hp > a.animals.items[0].hp + 4.);
    assert!(b.diagnostics.resistance_prevented_damage_total >= 4.9);
}
