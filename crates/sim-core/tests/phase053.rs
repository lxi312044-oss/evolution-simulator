use sim_core::config::SimulationConfig;
#[test]
fn phase053_defaults_and_validation() {
    let mut c=SimulationConfig::default();
    assert!(!c.herbivores.satiety_reproduction_enabled);
    c.herbivores.satiety_reproduction_enabled=true;
    c.herbivores.reproduction_satiety_threshold=0.5;
    c.herbivores.reproduction_well_fed_ticks=100;
    assert!(c.validate().is_ok());
    c.herbivores.reproduction_well_fed_ticks=0;
    assert!(c.validate().is_err());
}
