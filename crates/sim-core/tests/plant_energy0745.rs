use sim_core::{SimulationConfig,World};
#[test]
fn parent_trait_seed_counters_are_bounded_and_deterministic() {
 let mut cfg=SimulationConfig::default();
 cfg.toxin.enabled=true; cfg.toxin.plant_variation_enabled=true;
 cfg.toxin.initial_toxic_plant_fraction=0.5;
 let mut a=World::new(cfg.clone()).unwrap();let mut b=World::new(cfg).unwrap();
 a.step(200).unwrap();b.step(200).unwrap();
 let x=&a.diagnostics;let y=&b.diagnostics;
 assert_eq!(x.toxic_seed_success,y.toxic_seed_success);
 assert_eq!(x.nontoxic_seed_success,y.nontoxic_seed_success);
 assert_eq!(x.toxic_seed_attempts+x.nontoxic_seed_attempts,x.seed_attempts);
 assert_eq!(x.toxic_seed_resource_blocked+x.nontoxic_seed_resource_blocked,x.seed_resource_blocked);
 assert!(x.toxic_seed_success+x.nontoxic_seed_success<=x.seed_attempts);
}
