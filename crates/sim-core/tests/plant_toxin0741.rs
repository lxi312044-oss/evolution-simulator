use sim_core::{config::SimulationConfig,world::World};

#[test]
fn mixed_plant_traits_are_deterministic_and_affect_synthesis_cost() {
    let mut c=SimulationConfig::default();
    c.toxin.enabled=true;
    c.toxin.plant_variation_enabled=true;
    c.toxin.initial_toxic_plant_fraction=0.0;
    c.initial_population.herbivores=0;
    c.initial_population.carnivores=0;
    let mut w=World::new(c.clone()).unwrap();
    assert!(w.plants.items.iter().all(|p|!p.produces_toxin));
    w.step(1).unwrap();
    assert_eq!(w.diagnostics.toxin_plant_energy_cost_total,0.0);
    c.toxin.initial_toxic_plant_fraction=1.0;
    let mut toxic=World::new(c).unwrap();
    assert!(toxic.plants.items.iter().all(|p|p.produces_toxin));
    toxic.step(1).unwrap();
    assert!(toxic.diagnostics.toxin_plant_energy_cost_total>0.0);
}

#[test]
fn legacy_mode_stays_all_toxic() {
    let mut c=SimulationConfig::default();
    c.toxin.enabled=true;
    c.toxin.plant_variation_enabled=false;
    c.toxin.initial_toxic_plant_fraction=0.0;
    let w=World::new(c).unwrap();
    assert!(w.plants.items.iter().all(|p|p.produces_toxin));
}
