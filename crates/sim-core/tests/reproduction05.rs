use sim_core::{SimulationConfig,World};
#[test] fn reproduction_preserves_resources_and_generates_offspring(){
 let mut c=SimulationConfig::default();c.initial_population.plants=0;c.initial_population.herbivores=1;
 c.herbivores.reproduction_min_age=0;c.herbivores.reproduction_cooldown_ticks=1;
 c.herbivores.reproduction_min_energy_fraction=0.;c.herbivores.basal_energy_per_tick=0.;c.herbivores.movement_energy_per_unit=0.;
 c.herbivores.reproduction_min_hp_fraction=0.;c.herbivores.offspring_body_nutrients=1.;c.herbivores.offspring_reserve_nutrients=1.;c.herbivores.offspring_energy=1.;c.herbivores.reproduction_work_energy=0.;
 let mut w=World::new(c).unwrap();let n=w.resources.total_nutrients();let e=w.resources.total_energy();
 w.step(1).unwrap();assert!(w.herbivore_count()>1);assert!(w.diagnostics.herbivore_births>0);
 assert!((w.resources.total_nutrients()-n).abs()<1e-5);
 assert!((w.resources.total_energy()+w.ledger.heat_out-w.ledger.sunlight_in-e).abs()<1e-5);
}
#[test] fn disabling_reproduction_keeps_population_constant(){
 let mut c=SimulationConfig::default();c.herbivores.reproduction_enabled=false;
 let mut w=World::new(c).unwrap();w.step(100).unwrap();assert_eq!(w.herbivore_count(),80);
}
#[test] fn same_seed_reproduces_genetic_results(){
 let mut c=SimulationConfig::default();c.herbivores.reproduction_min_age=0;c.herbivores.reproduction_cooldown_ticks=1;
 c.herbivores.reproduction_min_energy_fraction=0.;c.herbivores.reproduction_min_hp_fraction=0.;
 c.herbivores.offspring_body_nutrients=1.;c.herbivores.offspring_reserve_nutrients=1.;c.herbivores.offspring_energy=1.;c.herbivores.reproduction_work_energy=0.;
 let mut a=World::new(c.clone()).unwrap();let mut b=World::new(c).unwrap();a.step(3).unwrap();b.step(3).unwrap();
 assert_eq!(a.herbivore_snapshot(),b.herbivore_snapshot());assert_eq!(a.diagnostics.herbivore_mutations,b.diagnostics.herbivore_mutations);
}
#[test] fn resource_dependent_reproduction_blocks_when_no_local_plants(){
 let mut c=SimulationConfig::default();c.initial_population.plants=0;c.initial_population.herbivores=1;
 c.herbivores.resource_dependent_reproduction=true;
 c.herbivores.reproduction_min_age=0;c.herbivores.reproduction_cooldown_ticks=1;
 c.herbivores.reproduction_min_energy_fraction=0.;c.herbivores.reproduction_min_hp_fraction=0.;
 c.herbivores.basal_energy_per_tick=0.;c.herbivores.movement_energy_per_unit=0.;
 c.herbivores.offspring_body_nutrients=1.;c.herbivores.offspring_reserve_nutrients=1.;
 c.herbivores.offspring_energy=1.;c.herbivores.reproduction_work_energy=0.;
 let mut w=World::new(c).unwrap();let n=w.resources.total_nutrients();
 w.step(1).unwrap();assert_eq!(w.herbivore_count(),1);
 assert!(w.diagnostics.herbivore_reproduction_food_blocked>0);
 assert!((w.resources.total_nutrients()-n).abs()<1e-5);
}
