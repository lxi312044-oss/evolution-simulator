use sim_core::{SimulationConfig,World};
#[test]
fn read_only_statistics_are_deterministic_and_non_mutating(){
 let a=World::new(SimulationConfig::default()).unwrap();
 assert_eq!(a.color_statistics(),a.color_statistics());
 let s=a.color_statistics();assert_eq!(s.population,a.herbivore_count() as u64);
 assert_eq!(s.inheritances,0);assert_eq!(s.mutated_births,0);
 for x in s.mean.iter().chain(s.variance.iter()){assert!(x.is_finite());}
}
#[test]
fn event_toggle_and_drain_do_not_affect_initial_state(){
 let cfg=SimulationConfig::default();
 let mut a=World::new(cfg.clone()).unwrap();let b=World::new(cfg).unwrap();
 a.set_color_event_logging(true);
 assert!(a.take_color_events().is_empty());
 assert_eq!(a.herbivore_color_snapshot(),b.herbivore_color_snapshot());
 a.set_color_event_logging(false);
 assert!(a.take_color_events().is_empty());
}
#[test]
fn mutation_flags_match_count_and_are_repeatable(){
 use sim_core::animals::color::{BodyColor,inherit_with_flags};
 let parent=BodyColor::new(0.3,0.4,0.5);
 let a=inherit_with_flags(parent,42,7,1.,0.1);
 assert_eq!(a,inherit_with_flags(parent,42,7,1.,0.1));
 assert_eq!(a.1,[true,true,true]);
 let b=inherit_with_flags(parent,42,7,0.,0.1);
 assert_eq!(b.0,parent);assert_eq!(b.1,[false,false,false]);
}
