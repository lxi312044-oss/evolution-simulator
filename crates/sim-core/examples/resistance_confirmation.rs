//! Streaming observation-only logger for the fixed Issue #7 protocol.
#[allow(dead_code)]
#[path = "resistance_focal_pilot.rs"]
mod pilot;
use serde_json::{json, Value};
use sim_core::{entity::EntityId, SimulationConfig, World};
use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, BufWriter, Write},
};

fn ident(id: EntityId) -> [u32; 2] {
    [id.index, id.generation]
}
fn emit(out: &mut dyn Write, value: Value) -> Result<(), String> {
    serde_json::to_writer(&mut *out, &value).map_err(|e| e.to_string())?;
    out.write_all(b"\n").map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
fn execute(
    config: SimulationConfig,
    environment: &str,
    ty: usize,
    resistance: f32,
    defense: f64,
    ticks: u64,
    interval: u64,
    out: &mut dyn Write,
) -> Result<World, String> {
    execute_with_health(
        config,
        environment,
        ty,
        resistance,
        defense,
        ticks,
        interval,
        out,
        None,
    )
}

#[derive(serde::Deserialize)]
struct HealthSpec {
    watched: Vec<[u32; 2]>,
    window: [u64; 2],
    #[serde(default)]
    energy: bool,
}

#[allow(clippy::too_many_arguments)]
fn execute_with_health(
    config: SimulationConfig,
    environment: &str,
    ty: usize,
    resistance: f32,
    defense: f64,
    ticks: u64,
    interval: u64,
    out: &mut dyn Write,
    health: Option<HealthSpec>,
) -> Result<World, String> {
    if ticks == 0 || interval == 0 {
        return Err("positive horizon/interval required".into());
    }
    let (mut w, focal, background) = pilot::prepare(config, environment, ty, resistance, defense)?;
    if let Some(spec) = health {
        if spec.window[0] > spec.window[1] {
            return Err("invalid trace window".into());
        }
        if spec.energy {
            w.energy_trace = Some(sim_core::world::EnergyTrace::new(spec.watched.iter().map(
                |id| EntityId {
                    index: id[0],
                    generation: id[1],
                },
            )));
        }
        w.health_trace = Some(sim_core::world::HealthTrace::new(
            spec.watched.into_iter().map(|id| EntityId {
                index: id[0],
                generation: id[1],
            }),
            (spec.window[0], spec.window[1]),
        ));
    }
    w.resistance_event_logging = true;
    w.focal_trace_id = Some(focal);
    let initial = pilot::observe(&w, focal).ok_or("missing focal")?;
    emit(
        out,
        json!({"kind":"initial", "seed":w.config.seed, "environment":environment,
            "toxin_type":ty,"resistance":resistance,"defense":defense,"ticks":ticks,"interval":interval,
            "focal_id":ident(focal),"initial":initial,"initial_background":background,
            "effective_config":w.config,
            "founders":w.animals.items.iter().map(|a|ident(a.id)).collect::<Vec<_>>(),
            "founder_traits":w.animals.items.iter().map(|a|json!({"id":ident(a.id),
                "resistance":[a.genes.toxin_resistance,a.genes.typed_resistance[1]]})).collect::<Vec<_>>()
        }),
    )?;
    let mut depths = BTreeMap::from([(focal, 0_u32)]);
    let mut born_by_depth = BTreeMap::<u32, u64>::new();
    let mut terminal: Option<Value> = None;
    let mut checkpoints = vec![initial.clone()];
    let mut max_n_error = 0_f64;
    let mut max_e_error = 0_f64;
    let mut births = 0_u64;
    let mut removals = 0_u64;
    let mut gate_failures = BTreeMap::<String, u64>::new();
    let mut gate_samples = 0_u64;
    let mut gate_unobserved = 0_u64;
    let mut first_attack_hp0: Option<Value> = None;
    for tick in 1..=ticks {
        w.step(1)?;
        w.checkpoint_energy();
        if let Some(trace) = &mut w.energy_trace {
            for event in std::mem::take(&mut trace.events) {
                emit(out, json!({"kind":"energy","event":event}))?;
            }
        }
        if let Some(trace) = &mut w.health_trace {
            for event in std::mem::take(&mut trace.events) {
                emit(out, json!({"kind":"health","event":event}))?;
            }
        }
        for event in std::mem::take(&mut w.resistance_birth_events) {
            let child = w
                .animals
                .items
                .iter()
                .find(|a| a.id == event.child_id)
                .ok_or("newborn missing before next removal phase")?;
            let depth = depths.get(&event.parent_id).map(|x| x + 1);
            if let Some(d) = depth {
                if depths.insert(child.id, d).is_some() {
                    return Err("duplicate lineage ID".into());
                }
                *born_by_depth.entry(d).or_default() += 1;
            }
            emit(
                out,
                json!({"kind":"birth","tick":event.tick,"parent":ident(event.parent_id),
                "child":ident(child.id),"resistance":[child.genes.toxin_resistance,child.genes.typed_resistance[1]],
                "depth":depth}),
            )?;
            births += 1;
        }
        for e in std::mem::take(&mut w.herbivore_terminal_events) {
            emit(
                out,
                json!({"kind":"removal","tick":e.tick,"id":ident(e.id),"hp":e.hp,
                "trigger":e.removal_trigger,"births":e.successful_births,
                "resistance":[e.resistance[0],e.resistance[1]]}),
            )?;
            removals += 1;
            if e.id == focal {
                if terminal.is_some() {
                    return Err("duplicate focal terminal".into());
                }
                terminal = Some(json!({"tick":e.tick,"alive":false,"age":e.age,"hp":e.hp,
                    "births":e.successful_births,"dose_a":e.doses[0],"dose_b":e.doses[1],
                    "damage":e.toxin_damage_total,"prevented":e.prevented_damage_total,
                    "cost":e.resistance_cost_total,"trigger":e.removal_trigger,
                    "resistance_a":e.resistance[0],"resistance_b":e.resistance[1]}));
            }
        }
        let mut observed_gate = false;
        for e in std::mem::take(&mut w.focal_trace) {
            if e["phase"] == "reproduction_gates" {
                gate_samples += 1;
                observed_gate = true;
                for (k, v) in e.as_object().ok_or("invalid gate")? {
                    if k.ends_with("_ok") && v == false {
                        *gate_failures.entry(k.clone()).or_default() += 1;
                    }
                }
            }
            if e["phase"] == "food_gate_failed" {
                *gate_failures.entry("food_gate_failed".into()).or_default() += 1;
            }
            if e["phase"] == "attack"
                && first_attack_hp0.is_none()
                && e["hp_before"].as_f64().unwrap_or(0.) > 0.
                && e["hp_after"].as_f64().unwrap_or(1.) <= 0.
            {
                first_attack_hp0 = Some(e);
            }
        }
        if !observed_gate && w.animals.items.iter().any(|a| a.id == focal) {
            gate_unobserved += 1;
        }
        max_n_error = max_n_error.max((w.resources.total_nutrients() - w.baseline.nutrients).abs());
        max_e_error = max_e_error.max(
            ((w.resources.total_energy() - w.baseline.energy)
                - (w.ledger.sunlight_total() - w.ledger.heat_total()))
            .abs(),
        );
        if tick % interval == 0 || tick == ticks {
            let focal_state = pilot::observe(&w, focal);
            if let Some(sample) = &focal_state {
                checkpoints.push(sample.clone());
            }
            let focal_births = focal_state
                .as_ref()
                .or(terminal.as_ref())
                .ok_or("missing focal endpoint")?["births"]
                .as_u64()
                .ok_or("missing birth count")?;
            let direct = born_by_depth.get(&1).copied().unwrap_or(0);
            if direct != focal_births {
                return Err("direct birth counter mismatch".into());
            }
            let mut living = BTreeMap::<u32, u64>::new();
            let mut pending = 0_u64;
            for a in &w.animals.items {
                if let Some(&d) = depths.get(&a.id) {
                    if d > 0 {
                        if a.hp > 0. {
                            *living.entry(d).or_default() += 1;
                        } else {
                            pending += 1;
                        }
                    }
                }
            }
            let live = living.values().sum::<u64>();
            let total = depths.len() as u64 - 1;
            let metrics = json!({"horizon":tick,"direct_births":direct,"lineage_births":total,
                "deeper_births":total-direct,"living_descendants":live,
                "present_hp_nonpositive_descendants":pending,"removed_descendants":total-live-pending,
                "born_by_depth":born_by_depth,"living_by_depth":living,
                "focal_living":focal_state.as_ref().is_some_and(|a|a["hp"].as_f64().unwrap_or(0.)>0.),
                "focal_present_hp_nonpositive":focal_state.as_ref().is_some_and(|a|a["hp"].as_f64().unwrap_or(0.)<=0.)});
            let present = if tick == 3000 || tick == ticks {
                Some(
                    w.animals
                        .items
                        .iter()
                        .map(|a| json!({"id":ident(a.id),"hp":a.hp}))
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            };
            emit(
                out,
                json!({"kind":"snapshot","tick":tick,"metrics":metrics,"present":present,
                "focal_birth_counter":focal_births,"focal":focal_state,"gate_samples":gate_samples,
                "gate_failures":gate_failures,"gate_unobserved_present_ticks":gate_unobserved}),
            )?;
        }
    }
    let survivor = pilot::observe(&w, focal);
    if terminal.is_some() == survivor.is_some() {
        return Err("ambiguous focal outcome".into());
    }
    let outcome = terminal.or(survivor).ok_or("missing focal outcome")?;
    let mut plant_types = [0_u64; 2];
    for p in &w.plants.items {
        if p.toxin_type.0 < 2 {
            plant_types[p.toxin_type.0 as usize] += 1;
        }
    }
    emit(
        out,
        json!({"kind":"complete","birth_event_count":births,"removal_event_count":removals,
        "final_population":w.animals.items.len(),"plant_type_counts":plant_types,
        "final_rng":format!("{:?}",w.rng),"first_attack_hp0":first_attack_hp0,
        "legacy_result":{"schema_version":1,"seed":w.config.seed,"environment":environment,
            "toxin_type":ty,"resistance":resistance,"defense":defense,"ticks":ticks,"interval":interval,
            "completed_tick":w.tick,"focal_id":ident(focal),"initial":initial,"outcome":outcome,
            "checkpoints":checkpoints,"max_n_error":max_n_error,"max_e_error":max_e_error,
            "effective_config":w.config,"final_ecology":w.ecology_snapshot()}}),
    )?;
    out.flush().map_err(|e| e.to_string())?;
    Ok(w)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 8 && args.len() != 9 {
        return Err(
            "usage: resistance_confirmation config env type resistance defense ticks interval"
                .into(),
        );
    }
    let config = serde_json::from_str(&fs::read_to_string(&args[1])?)?;
    if args.len() == 8 {
        execute(
            config,
            &args[2],
            args[3].parse()?,
            args[4].parse()?,
            args[5].parse()?,
            args[6].parse()?,
            args[7].parse()?,
            &mut BufWriter::new(io::stdout().lock()),
        )?;
        return Ok(());
    }
    execute_with_health(
        config,
        &args[2],
        args[3].parse()?,
        args[4].parse()?,
        args[5].parse()?,
        args[6].parse()?,
        args[7].parse()?,
        &mut BufWriter::new(io::stdout().lock()),
        if args.len() == 9 {
            Some(serde_json::from_str(&fs::read_to_string(&args[8])?)?)
        } else {
            None
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streaming_logger_is_read_only_including_rng() {
        let mut config: SimulationConfig = serde_json::from_str(include_str!(
            "../../../docs/phase07/phase0753-pilot-seed12345.json"
        ))
        .unwrap();
        config.seed = 23451;
        let (mut plain, id, _) = pilot::prepare(config.clone(), "B", 1, 0.8, 0.8).unwrap();
        plain.herbivore_terminal_logging = false;
        let mut bytes = Vec::new();
        let logged = execute_with_health(
            config,
            "B",
            1,
            0.8,
            0.8,
            3000,
            100,
            &mut bytes,
            Some(HealthSpec {
                watched: vec![ident(id)],
                window: [1, 3000],
                energy: true,
            }),
        )
        .unwrap();
        for _ in 0..3000 {
            plain.step(1).unwrap();
        }
        let rows: Vec<Value> = String::from_utf8(bytes)
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        let last = rows.last().unwrap();
        assert_eq!(last["final_rng"], format!("{:?}", plain.rng));
        assert_eq!(
            last["legacy_result"]["final_ecology"],
            plain.ecology_snapshot()
        );
        assert_eq!(
            format!("{:?}", logged.animals),
            format!("{:?}", plain.animals)
        );
        assert_eq!(
            format!("{:?}", logged.carnivores),
            format!("{:?}", plain.carnivores)
        );
        assert_eq!(
            format!("{:?}", logged.resources),
            format!("{:?}", plain.resources)
        );
        assert_eq!(
            format!("{:?}", logged.ledger),
            format!("{:?}", plain.ledger)
        );
        assert_eq!(last["legacy_result"]["focal_id"], json!(ident(id)));
        assert!(rows
            .iter()
            .any(|x| x["kind"] == "birth" && x["resistance"][1].as_f64().unwrap() > 0.79));
    }
}
