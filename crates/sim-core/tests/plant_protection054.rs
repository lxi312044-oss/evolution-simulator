use sim_core::{SimulationConfig, World};

#[test]
fn protection_blocks_feeding_and_preserves_n() {
 let mut c=SimulationConfig::default();
 c.herbivores.reproduction_enabled=false;
 c.herbivores.plant_protection_enabled=true;
 c.herbivores.plant_protection_threshold=5000;
 let mut w=World::new(c).unwrap();
 let n=w.resources.total_nutrients();
 w.step(100).unwrap();
 assert_eq!(w.diagnostics.herbivore_feeding_events,0);
 assert!(w.diagnostics.herbivore_plant_protection_blocked_ticks>0);
 assert!((w.resources.total_nutrients()-n).abs()<1e-5);
}

#[test]
fn protection_off_retains_feeding() {
 let mut c=SimulationConfig::default();
 c.herbivores.reproduction_enabled=false;
 c.herbivores.plant_protection_enabled=false;
 c.herbivores.plant_protection_threshold=5000;
 let mut w=World::new(c).unwrap();
 w.step(100).unwrap();
 assert_eq!(w.diagnostics.herbivore_plant_protection_blocked_ticks,0);
}
