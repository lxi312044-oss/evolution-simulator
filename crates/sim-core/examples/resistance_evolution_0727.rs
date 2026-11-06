//! Observation-only herbivore resistance evolution diagnostics.
use sim_core::{SimulationConfig,World};
use std::{collections::HashMap,env,fs::File,io::{BufWriter,Write}};
#[derive(Clone,Copy)]
struct Seen {resistance:f32,age:u32,hp:f32,energy:f64,toxin:f64,dose:f64,damage:f64,prevented:f64,cost:f64,mature:bool,births:u64}
fn bin(x:f32)->usize { ((x.clamp(0.,1.)*5.) as usize).min(4) }
fn main()->Result<(),Box<dyn std::error::Error>> {
 let a:Vec<String>=env::args().collect();
 if a.len()!=5 {return Err("usage: resistance_evolution_0721 config.json ticks interval output-prefix".into());}
 let cfg:SimulationConfig=serde_json::from_str(&std::fs::read_to_string(&a[1])?)?;
 let ticks:u64=a[2].parse()?;let interval:u64=a[3].parse()?;
 if interval==0||ticks==0{return Err("ticks and interval must be positive".into());}
 let mut w=World::new(cfg)?;
 w.resistance_event_logging=true;
 let mut genealogy=BufWriter::new(File::create(format!("{}.genealogy.csv",a[4]))?);
 writeln!(genealogy,"tick,event,id_index,id_generation,parent_id_index,parent_id_generation,parent_resistance,child_resistance,mutation_attempted,age_at_death,lifetime_births,final_status")?;
 let mut life=BufWriter::new(File::create(format!("{}.individual-life.csv",a[4]))?);
 writeln!(life,"tick,status,id_index,id_generation,resistance,age,hp,energy,toxin_load,cumulative_toxin_dose,cumulative_toxin_damage,cumulative_prevented_damage,cumulative_resistance_cost,reproduction_age_reached,successful_births")?;
 // Per-tick cumulative counter differences. Only positive increments are emitted;
 // absence of a row means zero for the observed event type, not missing follow-up.
 let mut timeline=BufWriter::new(File::create(format!("{}.individual-events.csv",a[4]))?);
 writeln!(timeline,"tick,event,id_index,id_generation,resistance,age,amount,parent_id_index,parent_id_generation")?;
 let mut offspring:HashMap<(u32,u32),u64>=HashMap::new();
 let mut parentage:HashMap<(u32,u32),(u32,u32,f32,bool)>=HashMap::new();
 let mut seen_births=0usize;
 let mut samples=BufWriter::new(File::create(format!("{}.resistance.csv",a[4]))?);
 let mut events=BufWriter::new(File::create(format!("{}.resistance-events.csv",a[4]))?);
 writeln!(samples,"tick,population,mean,variance,min,max,bin0,bin1,bin2,bin3,bin4,toxin_load_mean,energy_mean,hp_mean,cumulative_births,cumulative_deaths,cumulative_mutations,cumulative_resistance_cost,cumulative_prevented_damage,cumulative_toxin_damage,cumulative_toxin_exposure")?;
 writeln!(events,"tick,event,id_index,id_generation,resistance,age,hp,energy,toxin_load,possible_parent_id_index,possible_parent_id_generation,mutation_status")?;
 let mut previous:HashMap<(u32,u32),Seen>=HashMap::new();
 for tick in 0..=ticks {
  let mut current=HashMap::new();
  for a in &w.animals.items {
   current.insert((a.id.index,a.id.generation),Seen{resistance:a.genes.toxin_resistance,age:a.age,hp:a.hp,energy:w.resources.energy_balance(a.reserve).unwrap_or(0.),toxin:a.toxin_load,dose:a.lifetime_toxin_dose,damage:a.lifetime_toxin_damage,prevented:a.lifetime_prevented_damage,cost:a.lifetime_resistance_cost,mature:a.reproduction_age_reached,births:a.successful_births});
  }
  // Birth records come from the actual reproduction operation, not inferred proximity.
  for b in &w.resistance_birth_events[seen_births..] {
   let parent=(b.parent_id.index,b.parent_id.generation);
   let child=(b.child_id.index,b.child_id.generation);
   *offspring.entry(parent).or_default()+=1;
   parentage.insert(child,(parent.0,parent.1,b.parent_resistance,b.mutation_attempted));
   writeln!(genealogy,"{},birth,{},{},{},{},{:.9},{:.9},{},,,",b.tick,child.0,child.1,parent.0,parent.1,b.parent_resistance,b.child_resistance,b.mutation_attempted)?;
  }
  for b in &w.resistance_birth_events[seen_births..] {
   writeln!(timeline,"{},birth,{},{},{:.9},0,1,{},{}",b.tick,b.child_id.index,b.child_id.generation,b.child_resistance,b.parent_id.index,b.parent_id.generation)?;
   writeln!(timeline,"{},reproduction,{},{},{:.9},,1,,",b.tick,b.parent_id.index,b.parent_id.generation,b.parent_resistance)?;
  }
  seen_births=w.resistance_birth_events.len();
  if tick==0 {
   for (id,s) in &current {writeln!(genealogy,"0,founder,{},{},,,,{:.9},false,,,founder",id.0,id.1,s.resistance)?;}
  }
  if tick>0 {
   for (id,now) in &current {
    if let Some(prev)=previous.get(id) {
     for (event,delta) in [
      ("toxin_dose",now.dose-prev.dose),
      ("toxin_damage",now.damage-prev.damage),
      ("prevented_damage",now.prevented-prev.prevented),
      ("resistance_cost",now.cost-prev.cost),
     ] {
      if delta>0. {writeln!(timeline,"{},{},{},{},{:.9},{},{:.12},,",tick,event,id.0,id.1,now.resistance,now.age,delta)?;}
     }
    }
   }

   for (id,s) in &current {if !previous.contains_key(id){writeln!(events,"{},birth,{},{},{:.8},{},{:.8},{:.9},{:.9},,,unknown",tick,id.0,id.1,s.resistance,s.age,s.hp,s.energy,s.toxin)?;}}
   for (id,s) in &previous {if !current.contains_key(id){
    let lineage=parentage.get(id);
    let (pi,pg,pr,mutated)=lineage.map(|x|(x.0.to_string(),x.1.to_string(),format!("{:.9}",x.2),x.3.to_string())).unwrap_or_default();
    writeln!(timeline,"{},death,{},{},{:.9},{},1,,",tick,id.0,id.1,s.resistance,s.age)?;
    writeln!(life,"{},dead,{},{},{:.9},{},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{},{}",tick,id.0,id.1,s.resistance,s.age,s.hp,s.energy,s.toxin,s.dose,s.damage,s.prevented,s.cost,s.mature,s.births)?;
    writeln!(genealogy,"{},death,{},{},{},{},{},{:.9},{},{},{},dead",tick,id.0,id.1,pi,pg,pr,s.resistance,mutated,s.age,offspring.get(id).copied().unwrap_or(0))?;
    writeln!(events,"{},death,{},{},{:.8},{},{:.8},{:.9},{:.9},,,unknown",tick,id.0,id.1,s.resistance,s.age,s.hp,s.energy,s.toxin)?;}}
  }
  if tick%interval==0||tick==ticks {
   let n=current.len();let mut bins=[0usize;5];let mut sum=0.;let mut sq=0.;let mut lo=1f64;let mut hi=0f64;let mut toxin=0.;let mut energy=0.;let mut hp=0.;
   for s in current.values(){let v=s.resistance as f64;bins[bin(s.resistance)]+=1;sum+=v;sq+=v*v;lo=lo.min(v);hi=hi.max(v);toxin+=s.toxin;energy+=s.energy;hp+=s.hp as f64;}
   let mean=if n>0{sum/n as f64}else{0.};let var=if n>0{(sq/n as f64-mean*mean).max(0.)}else{0.};
   if n==0{lo=0.;}
   writeln!(samples,"{},{},{:.9},{:.9},{:.9},{:.9},{},{},{},{},{},{:.9},{:.9},{:.9},{},{},{},{:.9},{:.9},{:.9},{:.9}",tick,n,mean,var,lo,hi,bins[0],bins[1],bins[2],bins[3],bins[4],if n>0{toxin/n as f64}else{0.},if n>0{energy/n as f64}else{0.},if n>0{hp/n as f64}else{0.},w.diagnostics.herbivore_births,w.diagnostics.herbivore_age_deaths+w.diagnostics.herbivore_hp_deaths,w.diagnostics.resistance_mutations,w.diagnostics.resistance_energy_cost_total,w.diagnostics.resistance_prevented_damage_total,w.diagnostics.toxin_hp_damage_total,w.diagnostics.toxin_exposure_total)?;
  }
  previous=current;
  if tick<ticks{w.step(1)?;}
 }
 // Living individuals are right-censored: their lifetime reproduction is incomplete.
 for (id,s) in &previous {
  let lineage=parentage.get(id);
  let (pi,pg,pr,mutated)=lineage.map(|x|(x.0.to_string(),x.1.to_string(),format!("{:.9}",x.2),x.3.to_string())).unwrap_or_default();
  writeln!(timeline,"{},censored,{},{},{:.9},{},1,,",ticks,id.0,id.1,s.resistance,s.age)?;
  writeln!(life,"{},censored,{},{},{:.9},{},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{},{}",ticks,id.0,id.1,s.resistance,s.age,s.hp,s.energy,s.toxin,s.dose,s.damage,s.prevented,s.cost,s.mature,s.births)?;
  writeln!(genealogy,"{},censored,{},{},{},{},{},{:.9},{},{},{},alive",ticks,id.0,id.1,pi,pg,pr,s.resistance,mutated,s.age,offspring.get(id).copied().unwrap_or(0))?;
 }
 Ok(())
}
