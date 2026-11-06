//! Phase 07.4.5: read-only plant energy snapshots and parent-trait seed outcomes.
//! cargo run --release -p sim-core --example plant_energy_0745 -- config.json ticks interval out.csv
use sim_core::{SimulationConfig,World};
use std::{env,fs::File,io::{BufWriter,Write}};
fn main()->Result<(),Box<dyn std::error::Error>> {
 let args:Vec<String>=env::args().collect();
 if args.len()!=5 {return Err("usage: plant_energy_0745 config.json ticks interval out.csv".into());}
 let config:SimulationConfig=serde_json::from_str(&std::fs::read_to_string(&args[1])?)?;
 let ticks:u64=args[2].parse()?; let interval:u64=args[3].parse()?;
 if interval==0 {return Err("interval must be positive".into());}
 let mut world=World::new(config)?;
 let mut out=BufWriter::new(File::create(&args[4])?);
 writeln!(out,"seed,tick,trait,alive,energy_mean,energy_min,energy_max,energy_below_seed_threshold,mature,cooldown_ready,resource_ready,seed_attempts,seed_resource_blocked,seed_success,trait_births,age_deaths,consumed_deaths,herbivores")?;
 loop {
  let d=&world.diagnostics;
  for toxic in [true,false] {
   let mut count=0u64; let mut sum=0.;let mut min=f64::INFINITY;let mut max=f64::NEG_INFINITY;
   let mut low=0u64;let mut mature=0u64;let mut cooldown=0u64;let mut ready=0u64;
   let p=&world.config.plants;
   for plant in world.plants.items.iter().filter(|x|x.produces_toxin==toxic) {
    count+=1;
    let e=world.resources.energy_balance(plant.reserve).unwrap_or(0.);
    let rn=world.resources.nutrient_balance(plant.reserve).unwrap_or(0.);
    let bn=world.resources.nutrient_balance(plant.body).unwrap_or(0.);
    sum+=e;min=min.min(e);max=max.max(e);
    if e<p.seed_energy+p.seed_work_energy {low+=1;}
    if plant.growth>=1. {mature+=1;
     if plant.seed_cooldown==0 {cooldown+=1;
      if bn>=p.adult_body_nutrients*0.9 && rn>=p.seed_body_nutrients && e>=p.seed_energy+p.seed_work_energy {ready+=1;}
     }
    }
   }
   let (attempts,blocked,success,births,age,consumed)=if toxic {
    (d.toxic_seed_attempts,d.toxic_seed_resource_blocked,d.toxic_seed_success,d.toxic_plant_births,d.toxic_plant_age_deaths,d.toxic_plant_consumed_deaths)
   } else {
    (d.nontoxic_seed_attempts,d.nontoxic_seed_resource_blocked,d.nontoxic_seed_success,d.nontoxic_plant_births,d.nontoxic_plant_age_deaths,d.nontoxic_plant_consumed_deaths)
   };
   writeln!(out,"{},{},{},{},{:.9},{:.9},{:.9},{},{},{},{},{},{},{},{},{},{},{}",world.config.seed,world.tick,if toxic {"toxic"} else {"nontoxic"},count,if count>0 {sum/count as f64} else {0.},if count>0 {min} else {0.},if count>0 {max} else {0.},low,mature,cooldown,ready,attempts,blocked,success,births,age,consumed,world.animals.items.len())?;
  }
  if world.tick>=ticks {break;}
  let n=(ticks-world.tick).min(interval).min(world.config.simulation.max_steps_per_call as u64) as u32;
  world.step(n)?;
 }
 out.flush()?;Ok(())
}
