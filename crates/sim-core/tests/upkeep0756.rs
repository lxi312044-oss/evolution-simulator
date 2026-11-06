use sim_core::{SimulationConfig, World};
#[test]
fn fair_multiplier_defaults_and_validation() {
    let mut c = SimulationConfig::default();
    assert_eq!(c.toxin.multi_type_resistance_upkeep_multiplier, 1.0);
    for f in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
        c.toxin.multi_type_resistance_upkeep_multiplier = f;
        assert!(c.validate().is_err());
    }
    for f in [0., 0.25, 0.5, 1.] {
        c.toxin.multi_type_resistance_upkeep_multiplier = f;
        assert!(c.validate().is_ok());
    }
}
#[test]
fn legacy_single_type_unaffected() {
    let mut c = SimulationConfig::default();
    c.toxin.enabled = true;
    c.toxin.resistance_enabled = true;
    let mut d = c.clone();
    d.toxin.multi_type_resistance_upkeep_multiplier = 0.;
    let mut a = World::new(c).unwrap();
    let mut b = World::new(d).unwrap();
    for _ in 0..200 {
        a.step(1).unwrap();
        b.step(1).unwrap();
    }
    assert_eq!(
        a.diagnostics.resistance_energy_cost_total,
        b.diagnostics.resistance_energy_cost_total
    );
    assert_eq!(a.animals.items.len(), b.animals.items.len());
}
#[test]
fn multi_type_upkeep_monotonic_and_symmetric() {
    let mut c = SimulationConfig::default();
    c.toxin.enabled = true;
    c.toxin.resistance_enabled = true;
    c.toxin.herbivore_multi_toxin_enabled = true;
    c.toxin.plant_toxin_type_count = 2;
    c.toxin.resistance_energy_cost_per_tick = 0.1;
    let mut costs = Vec::new();
    for f in [0., 0.25, 0.5, 1.] {
        let mut v = c.clone();
        v.toxin.multi_type_resistance_upkeep_multiplier = f;
        let mut w = World::new(v).unwrap();
        w.step(1).unwrap();
        costs.push(w.diagnostics.resistance_energy_cost_total);
    }
    assert!(costs.windows(2).all(|p| p[0] <= p[1]), "{costs:?}");
    assert!(costs[0] < costs[3], "{costs:?}");
}
