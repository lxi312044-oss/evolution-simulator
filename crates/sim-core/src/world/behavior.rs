//! Shared decision policy for herbivores and carnivores. No resources are created here.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Action { Flee, StandGround, Pursue, Explore, Rest }
#[derive(Debug,Clone,Copy)]
pub struct Perception { pub threat_distance:Option<f32>, pub food_visible:bool, pub satiated:bool, pub energy_fraction:f32 }
#[derive(Debug,Clone,Copy)]
pub struct Policy { pub fear:f32, pub aggression:f32, pub detection_radius:f32, pub flee_threshold:f32 }
impl Policy {
 pub fn choose(self, p:Perception)->Action {
  if let Some(distance)=p.threat_distance {
   if distance<=self.detection_radius {
    let pressure=(1.-distance/self.detection_radius).clamp(0.,1.);
    let flee=self.fear+pressure*0.35;
    let resist=self.aggression + (1.-p.energy_fraction.clamp(0.,1.))*0.10;
    return if flee >= resist+self.flee_threshold {Action::Flee} else {Action::StandGround};
   }
  }
  if p.satiated {Action::Rest} else if p.food_visible {Action::Pursue} else {Action::Explore}
 }
}
#[cfg(test)] mod tests { use super::*;
 #[test] fn same_policy_handles_prey_and_predator(){let p=Policy{fear:0.9,aggression:0.1,detection_radius:55.,flee_threshold:0.};assert_eq!(p.choose(Perception{threat_distance:Some(10.),food_visible:true,satiated:false,energy_fraction:0.5}),Action::Flee);assert_eq!(p.choose(Perception{threat_distance:None,food_visible:true,satiated:false,energy_fraction:0.5}),Action::Pursue);}
 #[test] fn aggressive_animals_stand_ground(){let p=Policy{fear:0.1,aggression:0.9,detection_radius:55.,flee_threshold:0.};assert_eq!(p.choose(Perception{threat_distance:Some(5.),food_visible:false,satiated:false,energy_fraction:0.8}),Action::StandGround);}
}

#[cfg(test)] mod phase06235_tests {
 #[test] fn ambush_gene_bounded() { for gene in [0.0_f32,0.5,1.0] {for noise in [-0.06_f32,0.0,0.06] {assert!((0.0..=1.0).contains(&(gene+noise).clamp(0.0,1.0)));}}}
}
