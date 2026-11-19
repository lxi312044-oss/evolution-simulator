//! Phase 07.5.9: current-tick pre-removal terminal telemetry; no ecosystem changes.
//! Usage: cargo run --release -p sim-core --example toxin_evolution_0759 -- config.json ticks interval output-prefix
use sim_core::{SimulationConfig, World};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fs::File,
    io::{BufWriter, Write},
};

type Id = (u32, u32);
#[derive(Clone)]
struct AnimalSample {
    genes: [f32; 16],
    loads: [f64; 16],
    age: u32,
    births: u64,
    doses: [f64; 16],
    hp: f32,
    damage: f64,
    prevented: f64,
    cost: f64,
}
fn capture(w: &World) -> BTreeMap<Id, AnimalSample> {
    w.animals
        .items
        .iter()
        .map(|a| {
            let mut genes = a.genes.typed_resistance;
            genes[0] = a.genes.toxin_resistance;
            let mut loads = a.typed_load;
            loads[0] = a.toxin_load;
            (
                (a.id.index, a.id.generation),
                AnimalSample {
                    genes,
                    loads,
                    age: a.age,
                    births: a.successful_births,
                    doses: a.lifetime_typed_dose,
                    hp: a.hp,
                    damage: a.lifetime_toxin_damage,
                    prevented: a.lifetime_prevented_damage,
                    cost: a.lifetime_resistance_cost,
                },
            )
        })
        .collect()
}
fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        0.
    } else {
        xs.iter().sum::<f64>() / xs.len() as f64
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        return Err("usage: toxin_evolution_0759 config.json ticks interval output-prefix".into());
    }
    let config: SimulationConfig = serde_json::from_str(&std::fs::read_to_string(&args[1])?)?;
    if !config.toxin.enabled
        || !config.toxin.resistance_enabled
        || !config.toxin.herbivore_multi_toxin_enabled
        || !config.toxin.plant_toxin_type_variation_enabled
        || config.toxin.plant_toxin_type_count < 2
    {
        return Err("Phase 07.5.3 requires toxin, resistance, herbivore_multi_toxin and plant_toxin_type_variation enabled with at least two types".into());
    }
    let ticks: u64 = args[2].parse()?;
    let interval: u64 = args[3].parse()?;
    if ticks == 0 || interval == 0 {
        return Err("ticks and interval must be positive".into());
    }
    let count = config.toxin.plant_toxin_type_count as usize;
    let mut w = World::new(config)?;
    w.resistance_event_logging = true;
    w.herbivore_terminal_logging = true;
    let mut samples = BufWriter::new(File::create(format!("{}.types.csv", args[4]))?);
    let mut births = BufWriter::new(File::create(format!("{}.births.csv", args[4]))?);
    let mut outcomes = BufWriter::new(File::create(format!("{}.outcomes.csv", args[4]))?);
    writeln!(samples,"seed,tick,toxin_type,herbivores,toxic_plants,mean_resistance,resistance_variance,mean_load,high_resistance_fraction,mean_parent_resistance,selection_differential,births_in_window,deaths_in_window,mean_resistance_of_deaths,mean_resistance_of_newborns,cumulative_herbivore_births,cumulative_herbivore_deaths")?;
    writeln!(births,"seed,tick,toxin_type,parent_id,parent_generation,child_id,child_generation,parent_resistance,child_resistance,delta,mutation_observed")?;
    writeln!(
        outcomes,
        "seed,tick,toxin_type,id_index,id_generation,event,resistance,age,successful_births,load,observation_phase,removal_trigger,hp"
    )?;
    let mut exposure = BufWriter::new(File::create(format!("{}.individuals.csv", args[4]))?);
    writeln!(exposure,"seed,tick,event,toxin_type,id_index,id_generation,resistance,lifetime_exposure,age,successful_births,load,observation_phase,removal_trigger,hp,toxin_damage_total,prevented_damage_total,resistance_cost_total")?;
    let mut before = capture(&w);
    let mut born: Vec<Vec<f64>> = vec![Vec::new(); count];
    let mut parents: Vec<Vec<f64>> = vec![Vec::new(); count];
    let mut died: Vec<Vec<f64>> = vec![Vec::new(); count];
    for tick in 0..=ticks {
        if tick > 0 {
            w.step(1)?;
            let after = capture(&w);
            let events = std::mem::take(&mut w.resistance_birth_events);
            for event in events {
                let pid = (event.parent_id.index, event.parent_id.generation);
                let cid = (event.child_id.index, event.child_id.generation);
                // Parent may die in the same tick; use the pre-step snapshot.
                let parent = before.get(&pid).or_else(|| after.get(&pid));
                let child = after.get(&cid);
                if let (Some(p), Some(c)) = (parent, child) {
                    for ty in 0..count {
                        let pv = p.genes[ty] as f64;
                        let cv = c.genes[ty] as f64;
                        parents[ty].push(pv);
                        born[ty].push(cv);
                        writeln!(
                            births,
                            "{},{},{},{},{},{},{},{:.9},{:.9},{:.9},{}",
                            w.config.seed,
                            tick,
                            ty,
                            pid.0,
                            pid.1,
                            cid.0,
                            cid.1,
                            pv,
                            cv,
                            cv - pv,
                            pv != cv
                        )?;
                    }
                }
            }
            let terminals = std::mem::take(&mut w.herbivore_terminal_events);
            let disappeared: BTreeSet<_> = before
                .keys()
                .filter(|id| !after.contains_key(id))
                .copied()
                .collect();
            let recorded: BTreeSet<_> = terminals
                .iter()
                .map(|e| (e.id.index, e.id.generation))
                .collect();
            if recorded != disappeared || recorded.len() != terminals.len() {
                return Err(format!("terminal event coverage mismatch at tick {tick}").into());
            }
            for terminal in terminals {
                for (ty, deaths) in died.iter_mut().enumerate() {
                    deaths.push(terminal.resistance[ty] as f64);
                    writeln!(exposure, "{},{},death,{},{},{},{:.9},{:.9},{},{},{:.9},pre_removal_current_tick,{},{:.9},{:.9},{:.9},{:.9}",
                        w.config.seed, terminal.tick, ty, terminal.id.index, terminal.id.generation,
                        terminal.resistance[ty], terminal.doses[ty], terminal.age, terminal.successful_births,
                        terminal.loads[ty], terminal.removal_trigger, terminal.hp, terminal.toxin_damage_total,
                        terminal.prevented_damage_total, terminal.resistance_cost_total)?;
                    writeln!(
                        outcomes,
                        "{},{},{},{},{},death,{:.9},{},{},{:.9},pre_removal_current_tick,{},{:.9}",
                        w.config.seed,
                        terminal.tick,
                        ty,
                        terminal.id.index,
                        terminal.id.generation,
                        terminal.resistance[ty],
                        terminal.age,
                        terminal.successful_births,
                        terminal.loads[ty],
                        terminal.removal_trigger,
                        terminal.hp
                    )?;
                }
            }
            before = after;
        }
        if tick % interval == 0 || tick == ticks {
            let n = before.len();
            for (id, a) in &before {
                for ty in 0..count {
                    writeln!(
                        exposure,
                        "{},{},alive,{},{},{},{:.9},{:.9},{},{},{:.9},end_tick,none,{:.9},{:.9},{:.9},{:.9}",
                        w.config.seed,
                        tick,
                        ty,
                        id.0,
                        id.1,
                        a.genes[ty],
                        a.doses[ty],
                        a.age,
                        a.births,
                        a.loads[ty], a.hp, a.damage, a.prevented, a.cost
                    )?;
                }
            }
            for ty in 0..count {
                let rs: Vec<f64> = before.values().map(|a| a.genes[ty] as f64).collect();
                let avg = mean(&rs);
                let variance = mean(&rs.iter().map(|r| (r - avg) * (r - avg)).collect::<Vec<_>>());
                let load = mean(&before.values().map(|a| a.loads[ty]).collect::<Vec<_>>());
                let high = if n == 0 {
                    0.
                } else {
                    rs.iter().filter(|&&r| r >= 0.5).count() as f64 / n as f64
                };
                let plant_count = w
                    .plants
                    .items
                    .iter()
                    .filter(|p| p.produces_toxin && p.toxin_type.0 as usize == ty)
                    .count();
                let parent_mean = mean(&parents[ty]);
                let selection = if parents[ty].is_empty() {
                    0.
                } else {
                    parent_mean - avg
                };
                writeln!(
                    samples,
                    "{},{},{},{},{},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{},{},{:.9},{:.9},{},{}",
                    w.config.seed,
                    tick,
                    ty,
                    n,
                    plant_count,
                    avg,
                    variance,
                    load,
                    high,
                    parent_mean,
                    selection,
                    born[ty].len(),
                    died[ty].len(),
                    mean(&died[ty]),
                    mean(&born[ty]),
                    w.diagnostics.herbivore_births,
                    w.diagnostics.herbivore_age_deaths + w.diagnostics.herbivore_hp_deaths
                )?;
                born[ty].clear();
                parents[ty].clear();
                died[ty].clear();
            }
        }
    }
    samples.flush()?;
    births.flush()?;
    outcomes.flush()?;
    exposure.flush()?;
    Ok(())
}
