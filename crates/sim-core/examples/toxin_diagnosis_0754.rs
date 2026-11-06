//! Phase 07.5.4 paired counterfactual diagnostic. No simulation logic modifications.
//! Usage: cargo run --release -p sim-core --example toxin_diagnosis_0754 -- config.json ticks interval output-prefix
use sim_core::{SimulationConfig, World};
use std::{env, fs::File, io::{BufWriter, Write}};

fn run(base:&serde_json::Value, scenario:&str, ticks:u64, interval:u64, prefix:&str)->Result<(),Box<dyn std::error::Error>> {
    let mut v=base.clone();
    let toxin=v.get_mut("toxin").and_then(|x|x.as_object_mut()).ok_or("missing toxin config")?;
    match scenario {
        "legacy_single"=> {toxin.insert("plant_toxin_type_variation_enabled".into(),false.into());toxin.insert("herbivore_multi_toxin_enabled".into(),false.into());},
        "mixed_single_burden"=> {toxin.insert("plant_toxin_type_variation_enabled".into(),true.into());toxin.insert("herbivore_multi_toxin_enabled".into(),false.into());},
        "mixed_multi_burden"=> {toxin.insert("plant_toxin_type_variation_enabled".into(),true.into());toxin.insert("herbivore_multi_toxin_enabled".into(),true.into());},
        "mixed_multi_upkeep_half"=> {toxin.insert("plant_toxin_type_variation_enabled".into(),true.into());toxin.insert("herbivore_multi_toxin_enabled".into(),true.into());toxin.insert("additional_type_resistance_upkeep_multiplier".into(),serde_json::json!(0.5));},
        "mixed_multi_upkeep_zero"=> {toxin.insert("plant_toxin_type_variation_enabled".into(),true.into());toxin.insert("herbivore_multi_toxin_enabled".into(),true.into());toxin.insert("additional_type_resistance_upkeep_multiplier".into(),serde_json::json!(0.0));},
        "mixed_no_toxin"=> {toxin.insert("plant_toxin_type_variation_enabled".into(),true.into());toxin.insert("herbivore_multi_toxin_enabled".into(),true.into());toxin.insert("enabled".into(),false.into());toxin.insert("resistance_enabled".into(),false.into());},
        _=>return Err("unknown scenario".into()),
    }
    let config:SimulationConfig=serde_json::from_value(v)?;
    let mut w=World::new(config)?;
    let file=File::create(format!("{}.{}.csv",prefix,scenario))?;
    let mut out=BufWriter::new(file);
    writeln!(out,"seed,scenario,tick,herbivores,plants,toxic_plants_type0,toxic_plants_type1,herbivore_births,herbivore_age_deaths,herbivore_hp_deaths,toxin_exposure_total,toxin_hp_damage_total,toxin_deaths_with_burden,resistance_energy_cost_total,starvation_hp_damage,feeding_events,plant_consumed_n,carnivore_hunts,mean_load_type0,mean_load_type1")?;
    for tick in 0..=ticks {
        if tick>0 {w.step(1)?;}
        if tick%interval!=0 && tick!=ticks {continue;}
        let n=w.animals.items.len();
        let mean0=if n==0 {0.}else{w.animals.items.iter().map(|a|a.toxin_load).sum::<f64>()/n as f64};
        let mean1=if n==0 {0.}else{w.animals.items.iter().map(|a|a.typed_load[1]).sum::<f64>()/n as f64};
        let p0=w.plants.items.iter().filter(|p|p.produces_toxin && p.toxin_type.0==0).count();
        let p1=w.plants.items.iter().filter(|p|p.produces_toxin && p.toxin_type.0==1).count();
        let d=&w.diagnostics;
        writeln!(out,"{},{},{},{},{},{},{},{},{},{},{:.9},{:.9},{},{:.9},{:.9},{},{:.9},{},{:.9},{:.9}",w.config.seed,scenario,tick,n,w.plants.items.len(),p0,p1,d.herbivore_births,d.herbivore_age_deaths,d.herbivore_hp_deaths,d.toxin_exposure_total,d.toxin_hp_damage_total,d.toxin_deaths_with_burden,d.resistance_energy_cost_total,d.phase21_starvation_hp_damage,d.herbivore_feeding_events,d.plant_consumed_n,w.carnivore_hunts,mean0,mean1)?;
    }
    out.flush()?;
    Ok(())
}
fn main()->Result<(),Box<dyn std::error::Error>> {
    let a:Vec<String>=env::args().collect();
    if a.len()!=5{return Err("usage: toxin_diagnosis_0754 config.json ticks interval output-prefix".into());}
    let base:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(&a[1])?)?;
    let ticks:u64=a[2].parse()?;let interval:u64=a[3].parse()?;
    if ticks==0||interval==0{return Err("ticks and interval must be positive".into());}
    for scenario in ["legacy_single","mixed_single_burden","mixed_multi_burden","mixed_multi_upkeep_half","mixed_multi_upkeep_zero","mixed_no_toxin"] {run(&base,scenario,ticks,interval,&a[4])?;}
    Ok(())
}
