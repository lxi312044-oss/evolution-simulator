//! Phase 07.4.2 observational CSV: trait frequencies and demographic differences.
//! Usage: cargo run --release -p sim-core --example plant_selection_0742 -- config.json ticks interval output.csv
use sim_core::{SimulationConfig,World};
use std::{env,fs::File,io::{BufWriter,Write}};
fn main()->Result<(),Box<dyn std::error::Error>> {
 let args:Vec<String>=env::args().collect();
 if args.len()!=5 {return Err("usage: plant_selection_0742 config.json ticks interval output.csv".into());}
 let config:SimulationConfig=serde_json::from_str(&std::fs::read_to_string(&args[1])?)?;
 let ticks:u64=args[2].parse()?;let interval:u64=args[3].parse()?;
 if interval==0 {return Err("interval must be > 0".into());}
 let mut world=World::new(config)?;
 let mut out=BufWriter::new(File::create(&args[4])?);
 writeln!(out,"seed,tick,toxic_alive,nontoxic_alive,toxic_fraction,toxic_births,nontoxic_births,toxic_age_deaths,nontoxic_age_deaths,toxic_consumed_deaths,nontoxic_consumed_deaths,toxic_mutated_births,nontoxic_mutated_births,toxin_plant_energy_cost_total,herbivores,plant_births_total,plant_age_deaths_total,plant_consumed_deaths_total")?;
 loop {
  let toxic=world.plants.items.iter().filter(|p|p.produces_toxin).count();
  let non=world.plants.items.len()-toxic;
  let d=&world.diagnostics;
  let fraction=if toxic+non>0 {toxic as f64/(toxic+non) as f64} else {f64::NAN};
  writeln!(out,"{},{},{},{},{:.9},{},{},{},{},{},{},{},{},{:.9},{},{},{},{}",world.config.seed,world.tick,toxic,non,fraction,d.toxic_plant_births,d.nontoxic_plant_births,d.toxic_plant_age_deaths,d.nontoxic_plant_age_deaths,d.toxic_plant_consumed_deaths,d.nontoxic_plant_consumed_deaths,d.toxic_plant_mutated_births,d.nontoxic_plant_mutated_births,d.toxin_plant_energy_cost_total,world.animals.items.len(),d.plant_births,d.plant_age_deaths,d.plant_consumed_deaths)?;
  if world.tick>=ticks {break;}
  let n=(ticks-world.tick).min(interval).min(world.config.simulation.max_steps_per_call as u64) as u32;
  world.step(n)?;
 }
 out.flush()?;
 Ok(())
}
