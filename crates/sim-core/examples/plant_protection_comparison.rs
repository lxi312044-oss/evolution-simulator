//! Paired plant-protection / soil-germination experiment. Core defaults stay unchanged.
#[allow(dead_code)]
#[path = "resistance_focal_pilot.rs"]
mod pilot;
use serde_json::{json, Value};
use sim_core::{world::SpontaneousGermination, SimulationConfig};
use std::collections::BTreeSet;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        return Err(
            "usage: plant_protection_comparison config resistance protection_bool germination_bool"
                .into(),
        );
    }
    let mut c: SimulationConfig = serde_json::from_str(&std::fs::read_to_string(&args[1])?)?;
    let resistance: f32 = args[2].parse()?;
    let protection: bool = args[3].parse()?;
    let germination: bool = args[4].parse()?;
    c.herbivores.plant_protection_enabled = protection;
    let (mut w, focal, background) = pilot::prepare(c, "A", 0, resistance, 0.8)?;
    if germination {
        let g = SpontaneousGermination {
            soil_threshold: 20.,
            probability_per_cell: 0.02,
            check_interval: 20,
            max_births_per_check: 20,
        };
        g.validate()?;
        w.spontaneous_germination = Some(g);
    }
    w.resistance_event_logging = true;
    w.herbivore_terminal_logging = true;
    let initial = pilot::observe(&w, focal).unwrap();
    let mut lineage = BTreeSet::from([focal]);
    let mut lineage_births = 0_u64;
    let mut focal_births = 0_u64;
    let mut lineage_removals = 0_u64;
    let mut min_plants = w.plants.len();
    let mut zero_ticks = 0_u64;
    let mut zero_start = None;
    let mut zero_episodes = Vec::<Value>::new();
    let mut max_n_error = 0_f64;
    let mut max_e_error = 0_f64;
    let mut samples = Vec::<Value>::new();
    for tick in 1..=6000_u64 {
        w.step(1)?;
        let n = w.plants.len();
        min_plants = min_plants.min(n);
        if n == 0 {
            zero_ticks += 1;
            if zero_start.is_none() {
                zero_start = Some(tick);
            }
        } else if let Some(start) = zero_start.take() {
            zero_episodes.push(json!({"start":start,"recovery":tick}));
        }
        for event in std::mem::take(&mut w.resistance_birth_events) {
            if event.parent_id == focal {
                focal_births += 1;
            }
            if lineage.contains(&event.parent_id) {
                lineage.insert(event.child_id);
                lineage_births += 1;
            }
        }
        for event in std::mem::take(&mut w.herbivore_terminal_events) {
            if lineage.contains(&event.id) && event.id != focal {
                lineage_removals += 1;
            }
        }
        max_n_error = max_n_error.max((w.resources.total_nutrients() - w.baseline.nutrients).abs());
        max_e_error = max_e_error.max(
            ((w.resources.total_energy() - w.baseline.energy)
                - (w.ledger.sunlight_total() - w.ledger.heat_total()))
            .abs(),
        );
        if tick % 100 == 0 {
            samples.push(json!({"tick":tick,"plants":n,"herbivores":w.animals.items.len(),"herbivores_hp_positive":w.animals.items.iter().filter(|a|a.hp>0.).count(),"carnivores":w.carnivores.len(),"germination_births":w.spontaneous_births,"starvation_ticks":w.diagnostics.phase21_starvation_damage_ticks,"herbivore_births":w.diagnostics.herbivore_births,"plant_births":w.diagnostics.plant_births,"soil_n":w.land.cells.iter().map(|c|w.resources.nutrient_balance(c.soil).unwrap_or(0.)).sum::<f64>()}));
        }
    }
    if let Some(start) = zero_start {
        zero_episodes.push(json!({"start":start,"recovery":null}));
    }
    let descendants = w
        .animals
        .items
        .iter()
        .filter(|a| a.id != focal && lineage.contains(&a.id) && a.hp > 0.)
        .count();
    println!(
        "{}",
        json!({"seed":w.config.seed,"resistance":resistance,"protection":protection,"germination":germination,"ticks":w.tick,"interval":100,
  "germination_parameters":if germination{Some(json!({"soil_threshold":20,"probability_per_cell":0.02,"check_interval":20,"max_births_per_check":20}))}else{None},
  "initial":initial,"initial_background":background,"effective_config":w.config,"final_rng":format!("{:?}",w.rng),"final_ecology":w.ecology_snapshot(),
  "samples":samples,"min_plants":min_plants,"zero_plant_ticks":zero_ticks,"zero_episodes":zero_episodes,"germination_births":w.spontaneous_births,
  "max_n_error":max_n_error,"max_e_error":max_e_error,"diagnostics":w.diagnostics,"focal_births":focal_births,"lineage_births":lineage_births,"lineage_removals":lineage_removals,"lineage_positive_survivors":descendants})
    );
    Ok(())
}
