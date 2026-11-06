use sim_core::{SimulationConfig,World};

#[test]
fn satiety_enabled_conserves_nutrients_and_bounds_stomach() {
 let mut c=SimulationConfig::default();
 c.herbivores.satiety_enabled=true;
 c.herbivores.reproduction_enabled=false;
 c.herbivores.stomach_capacity_n=1.0;
 c.herbivores.digestion_n_per_tick=0.02;
 c.herbivores.feeding_nutrients_per_tick=1.5;
 let mut w=World::new(c).unwrap();
 let n=w.resources.total_nutrients();
 w.step(150).unwrap();
 assert!((w.resources.total_nutrients()-n).abs()<1e-5);
 for a in &w.animals.items {
  let contents=w.resources.nutrient_balance(a.stomach).unwrap();
  assert!(contents>=0. && contents<=1.0+1e-8, "stomach contents: {contents}");
 }
}

#[test]
fn satiety_off_keeps_stomach_empty() {
 let mut c=SimulationConfig::default();
 c.herbivores.satiety_enabled=false;
 c.herbivores.reproduction_enabled=false;
 let mut w=World::new(c).unwrap();
 w.step(100).unwrap();
 for a in &w.animals.items {assert_eq!(w.resources.nutrient_balance(a.stomach).unwrap(),0.);}
}

#[test]
fn satiety_invalid_thresholds_rejected() {
 let mut c=SimulationConfig::default();
 c.herbivores.feeding_resume_fraction=0.9;
 c.herbivores.feeding_stop_fraction=0.8;
 assert!(c.validate().is_err());
}
