use sim_core::{config::SimulationConfig, toxin::{ToxinTypeId,founder_plant_toxin_type,offspring_plant_toxin_type}};
#[test] fn legacy_type_zero_when_disabled() {
    for i in 0..200 { assert_eq!(founder_plant_toxin_type(123,i,false,2),ToxinTypeId::LEGACY); assert_eq!(offspring_plant_toxin_type(123,i,ToxinTypeId(1),false,2,1.),ToxinTypeId::LEGACY); }
}
#[test] fn inheritance_without_mutation() {
    for i in 0..200 {assert_eq!(offspring_plant_toxin_type(123,i,ToxinTypeId(1),true,2,0.),ToxinTypeId(1));}
}
#[test] fn mutation_always_switches_to_other_type() {
    for i in 0..200 {assert_eq!(offspring_plant_toxin_type(123,i,ToxinTypeId(0),true,2,1.),ToxinTypeId(1)); assert_eq!(offspring_plant_toxin_type(123,i,ToxinTypeId(1),true,2,1.),ToxinTypeId(0));}
}
#[test] fn founder_distribution_is_deterministic_and_bounded() {
    let a:Vec<_>=(0..1000).map(|i|founder_plant_toxin_type(123,i,true,3)).collect();
    let b:Vec<_>=(0..1000).map(|i|founder_plant_toxin_type(123,i,true,3)).collect();
    assert_eq!(a,b);assert!(a.iter().all(|v|v.0<3));assert!(a.iter().any(|v|v.0==2));
}
#[test] fn invalid_config_is_rejected() {
    let mut c=SimulationConfig::default();c.toxin.plant_toxin_type_count=0;assert!(c.validate().is_err());
    c.toxin.plant_toxin_type_count=17;assert!(c.validate().is_err());
    c.toxin.plant_toxin_type_count=2;c.toxin.plant_toxin_type_mutation_probability=1.1;assert!(c.validate().is_err());
}
