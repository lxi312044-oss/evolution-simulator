//! Observation-only herbivore resistance evolution diagnostics.
use sim_core::{SimulationConfig,World};
use std::{collections::HashMap,env,fs::File,io::{BufWriter,Write}};
#[derive(Clone,Copy)]
struct Seen {resistance:f32,age:u32,hp:f32,energy:f64,toxin:f64}
fn bin(x:f32)->usize { ((x.clamp(0.,1.)*5.) as usize).min(4) }
fn main()->Result<(),Box<dyn std::error::Error>> {
 let a:Vec<String>=env::args().collect();
 if a.len()!=5 {return Err("usage: resistance_evolution_0721 config.json ticks interval output-prefix".into());}
 let cfg:SimulationConfig=serde_json::from_str(&std::fs::read_to_string(&a[1])?)?;
 let ticks:u64=a[2].parse()?;let interval:u64=a[3].parse()?;
 if interval==0||ticks==0{return Err("ticks and interval must be positive".into());}
 let mut w=World::new(cfg)?;
 let mut samples=BufWriter::new(File::create(format!("{}.resistance.csv",a[4]))?);
 let mut events=BufWriter::new(File::create(format!("{}.resistance-events.csv",a[4]))?);
 writeln!(samples,"tick,population,mean,variance,min,max,bin0,bin1,bin2,bin3,bin4,toxin_load_mean,energy_mean,hp_mean,cumulative_births,cumulative_deaths,cumulative_mutations,cumulative_resistance_cost,cumulative_prevented_damage,cumulative_toxin_damage,cumulative_toxin_exposure")?;
 writeln!(events,"tick,event,id_index,id_generation,resistance,age,hp,energy,toxin_load,possible_parent_id_index,possible_parent_id_generation,mutation_status")?;
 let mut previous:HashMap<(u32,u32),Seen>=HashMap::new();
 for tick in 0..=ticks {
  let mut current=HashMap::new();
  for a in &w.animals.items {
   current.insert((a.id.index,a.id.generation),Seen{resistance:a.genes.toxin_resistance,age:a.age,hp:a.hp,energy:w.resources.energy_balance(a.reserve).unwrap_or(0.),toxin:a.toxin_load});
  }
  if tick>0 {
   for (id,s) in &current {if !previous.contains_key(id){writeln!(events,"{},birth,{},{},{:.8},{},{:.8},{:.9},{:.9},,,unknown",tick,id.0,id.1,s.resistance,s.age,s.hp,s.energy,s.toxin)?;}}
   for (id,s) in &previous {if !current.contains_key(id){writeln!(events,"{},death,{},{},{:.8},{},{:.8},{:.9},{:.9},,,unknown",tick,id.0,id.1,s.resistance,s.age,s.hp,s.energy,s.toxin)?;}}
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
 Ok(())
}
