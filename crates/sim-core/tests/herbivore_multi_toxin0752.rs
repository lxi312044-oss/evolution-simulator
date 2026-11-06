use sim_core::{SimulationConfig,World};
fn world()->World {
 let mut c=SimulationConfig::default();
 c.seed=12345;c.initial_population.plants=1;c.initial_population.herbivores=1;c.initial_population.carnivores=0;
 c.herbivores.reproduction_enabled=false;
 c.toxin.enabled=true;c.toxin.resistance_enabled=true;
 c.toxin.plant_toxin_type_variation_enabled=true;c.toxin.plant_toxin_type_count=2;
 c.toxin.herbivore_multi_toxin_enabled=true;
 c.toxin.plant_energy_cost_per_tick=0.;c.toxin.dose_per_nutrient=0.;
 c.toxin.clearance_fraction_per_tick=0.;c.toxin.damage_threshold=0.;c.toxin.hp_damage_per_excess_unit=1.;
 c.toxin.resistance_energy_cost_per_tick=0.;
 World::new(c).unwrap()
}
#[test] fn type_one_load_does_not_change_type_zero() {
 let mut w=world();w.animals.items[0].typed_load[1]=5.;
 w.animals.items[0].genes.typed_resistance[1]=1.;w.animals.items[0].genes.toxin_resistance=0.;
 w.step(1).unwrap();
 assert_eq!(w.animals.items[0].toxin_load,0.);
 assert_eq!(w.animals.items[0].typed_load[1],5.);
 assert!((w.animals.items[0].lifetime_toxin_damage-1.).abs()<0.0001);
}
#[test] fn no_cross_resistance_in_world() {
 let mut a=world();let mut b=world();
 for w in [&mut a,&mut b] {w.animals.items[0].typed_load[1]=4.;w.animals.items[0].genes.typed_resistance[1]=0.;}
 a.animals.items[0].genes.toxin_resistance=0.;b.animals.items[0].genes.toxin_resistance=1.;
 a.step(1).unwrap();b.step(1).unwrap();
 assert_eq!(a.animals.items[0].lifetime_toxin_damage,b.animals.items[0].lifetime_toxin_damage);
}
#[test] fn disabled_multi_toxin_keeps_legacy_path() {
 let mut a=world();let mut b=world();
 a.config.toxin.herbivore_multi_toxin_enabled=false;
 b.config.toxin.herbivore_multi_toxin_enabled=false;
 b.animals.items[0].typed_load[1]=8.;
 a.step(1).unwrap();b.step(1).unwrap();
 assert_eq!(a.animals.items[0].hp,b.animals.items[0].hp);
 assert_eq!(a.animals.items[0].toxin_load,b.animals.items[0].toxin_load);
}
#[test] fn deterministic_mutation_stream_is_bounded() {
 for ty in 1..16 {for birth in 0..100 {
  let x=sim_core::toxin::herbivore_type_draw(42,birth,ty,0);
  assert!(x>=0. && x<1.);
  assert_eq!(x,sim_core::toxin::herbivore_type_draw(42,birth,ty,0));
 }}
}
