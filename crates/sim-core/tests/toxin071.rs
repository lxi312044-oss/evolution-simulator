use sim_core::{SimulationConfig,World};

fn simple_world(enabled:bool)->World {
 let mut c=SimulationConfig::default();
 c.seed=24680;
 c.initial_population.plants=1;
 c.initial_population.herbivores=1;
 c.initial_population.carnivores=0;
 c.herbivores.reproduction_enabled=false;
 c.toxin.enabled=enabled;
 World::new(c).unwrap()
}
fn assert_conservation(w:&World) {
 assert!((w.resources.total_nutrients()-w.baseline.nutrients).abs()<1e-5);
 let current=w.resources.total_energy();
 let expected=w.baseline.energy+w.ledger.sunlight_total()-w.ledger.heat_total();
 assert!((current-expected).abs()<1e-4,"energy: {current} vs {expected}");
}
#[test]
fn toxin_disabled_is_identical_even_with_nonzero_parameters(){
 let mut a=simple_world(false);
 let mut b=simple_world(false);
 b.config.toxin.dose_per_nutrient=999.;
 b.config.toxin.plant_energy_cost_per_tick=999.;
 b.config.toxin.hp_damage_per_excess_unit=999.;
 for _ in 0..100 {a.step(1).unwrap();b.step(1).unwrap();}
 assert_eq!(a.animals.items.len(),b.animals.items.len());
 assert_eq!(a.plants.items.len(),b.plants.items.len());
 assert_eq!(a.resources.total_energy(),b.resources.total_energy());
 assert_eq!(a.resources.total_nutrients(),b.resources.total_nutrients());
 assert_eq!(a.animals.items[0].hp,b.animals.items[0].hp);
 assert_eq!(a.animals.items[0].toxin_load,0.);
 assert_eq!(b.animals.items[0].toxin_load,0.);
}
#[test]
fn toxin_clearance_and_threshold_damage(){
 let mut w=simple_world(true);
 w.config.toxin.dose_per_nutrient=0.;
 w.config.toxin.plant_energy_cost_per_tick=0.;
 w.config.toxin.clearance_fraction_per_tick=0.25;
 w.config.toxin.damage_threshold=2.;
 w.config.toxin.hp_damage_per_excess_unit=1.;
 w.animals.items[0].toxin_load=8.;
 let hp=w.animals.items[0].hp;
 w.step(1).unwrap();
 assert!((w.animals.items[0].toxin_load-6.).abs()<1e-8);
 // Damage 4 HP above threshold, in addition to any ordinary physiology.
 assert!(w.animals.items[0].hp<=hp-4.+1e-4);
 assert_conservation(&w);
}
#[test]
fn plant_synthesis_cost_is_accounted_for_as_heat(){
 let mut a=simple_world(false);
 let mut b=simple_world(true);
 b.config.toxin.dose_per_nutrient=0.;
 b.config.toxin.hp_damage_per_excess_unit=0.;
 b.config.toxin.plant_energy_cost_per_tick=0.1;
 a.config.plants.photosynthesis_per_tick=0.;
 b.config.plants.photosynthesis_per_tick=0.;
 a.step(1).unwrap();b.step(1).unwrap();
 assert!(b.ledger.heat_out>a.ledger.heat_out);
 assert_conservation(&b);
}
#[test]
fn toxin_exposure_from_feeding(){
 let mut w=simple_world(true);
 w.config.toxin.dose_per_nutrient=10.;
 w.config.toxin.clearance_fraction_per_tick=0.;
 w.config.toxin.damage_threshold=1e9;
 w.config.toxin.plant_energy_cost_per_tick=0.;
 let (x,y)=(w.plants.items[0].x,w.plants.items[0].y);
 w.animals.items[0].x=x;w.animals.items[0].y=y;
 // Keep the target available; prevent it from escaping before the feeding decision.
 w.animals.items[0].genes.speed=0.;
 for _ in 0..100 {w.step(1).unwrap();if w.animals.items.is_empty(){break;} if w.animals.items[0].toxin_load>0. {break;}}
 assert!(!w.animals.items.is_empty());
 assert!(w.animals.items[0].toxin_load>0.,"herbivore should ingest plant toxin");
 assert_conservation(&w);
}
#[test]
fn invalid_toxin_configuration_is_rejected(){
 let mut c=SimulationConfig::default();
 c.toxin.clearance_fraction_per_tick=1.1;
 assert!(c.validate().is_err());
 c.toxin.clearance_fraction_per_tick=0.2;
 c.toxin.dose_per_nutrient=f64::NAN;
 assert!(c.validate().is_err());
}

#[test]
fn toxin_diagnostics_are_observational_and_zero_when_disabled(){
 let mut w=simple_world(false);
 w.step(10).unwrap();
 assert_eq!(w.diagnostics.toxin_exposure_events,0);
 assert_eq!(w.diagnostics.toxin_exposure_total,0.);
 assert_eq!(w.diagnostics.toxin_hp_damage_total,0.);
 assert_eq!(w.diagnostics.toxin_plant_energy_cost_total,0.);
}
#[test]
fn toxin_diagnostics_track_clearance_and_damage(){
 let mut w=simple_world(true);
 w.config.toxin.dose_per_nutrient=0.;
 w.config.toxin.plant_energy_cost_per_tick=0.;
 w.config.toxin.clearance_fraction_per_tick=0.25;
 w.config.toxin.damage_threshold=2.;
 w.config.toxin.hp_damage_per_excess_unit=1.;
 w.animals.items[0].toxin_load=8.;
 w.step(1).unwrap();
 assert!((w.diagnostics.toxin_clearance_total-2.).abs()<1e-8);
 assert!((w.diagnostics.toxin_hp_damage_total-4.).abs()<1e-5);
 assert_eq!(w.diagnostics.toxin_damage_ticks,1);
 assert_conservation(&w);
}
