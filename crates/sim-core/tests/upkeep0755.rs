use sim_core::{SimulationConfig, World};
#[test]
fn multiplier_is_validated_and_legacy_default_is_one() {
    let mut c = SimulationConfig::default();
    assert_eq!(c.toxin.additional_type_resistance_upkeep_multiplier, 1.0);
    for x in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
        c.toxin.additional_type_resistance_upkeep_multiplier = x;
        assert!(c.validate().is_err());
    }
    for x in [0.0, 0.5, 1.0] {
        c.toxin.additional_type_resistance_upkeep_multiplier = x;
        assert!(c.validate().is_ok());
    }
}
#[test]
fn multiplier_does_not_change_single_type_ecology() {
    let mut a = SimulationConfig::default();
    a.toxin.enabled = true;
    a.toxin.resistance_enabled = true;
    let mut b = a.clone();
    b.toxin.additional_type_resistance_upkeep_multiplier = 0.;
    let mut x = World::new(a).unwrap();
    let mut y = World::new(b).unwrap();
    for _ in 0..200 {
        x.step(1).unwrap();
        y.step(1).unwrap();
    }
    assert_eq!(
        x.diagnostics.resistance_energy_cost_total,
        y.diagnostics.resistance_energy_cost_total
    );
    assert_eq!(x.animals.items.len(), y.animals.items.len());
}
#[test]
fn extra_type_upkeep_is_monotonic_in_one_tick() {
    let mut c = SimulationConfig::default();
    c.toxin.enabled = true;
    c.toxin.resistance_enabled = true;
    c.toxin.herbivore_multi_toxin_enabled = true;
    c.toxin.plant_toxin_type_count = 2;
    c.toxin.initial_resistance = 0.5;
    c.toxin.resistance_energy_cost_per_tick = 0.1;
    let mut costs = Vec::new();
    for factor in [0., 0.5, 1.] {
        let mut v = c.clone();
        v.toxin.additional_type_resistance_upkeep_multiplier = factor;
        let mut w = World::new(v).unwrap();
        w.step(1).unwrap();
        costs.push(w.diagnostics.resistance_energy_cost_total);
    }
    assert!(costs[0] <= costs[1] && costs[1] <= costs[2], "{costs:?}");
    assert!(costs[0] < costs[2], "{costs:?}");
}
