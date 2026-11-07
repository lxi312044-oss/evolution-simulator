//! Diagnostic pilot: one founder trait intervention in otherwise identical worlds.
use serde_json::{json, Value};
use sim_core::{entity::EntityId, SimulationConfig, World};
use std::{env, fs};

pub(crate) fn prepare(
    mut config: SimulationConfig,
    environment: &str,
    toxin_type: usize,
    resistance: f32,
    defense: f64,
) -> Result<(World, EntityId, Value), String> {
    if !["A", "B", "off"].contains(&environment)
        || toxin_type > 1
        || ![0.2_f32, 0.8_f32].contains(&resistance)
        || ![0.0, 0.8].contains(&defense)
    {
        return Err("invalid diagnostic intervention".into());
    }
    let t = &mut config.toxin;
    t.enabled = environment != "off";
    t.resistance_enabled = true;
    t.herbivore_multi_toxin_enabled = true;
    t.plant_toxin_type_variation_enabled = true;
    t.plant_toxin_type_count = 2;
    t.initial_type0_fraction = if environment == "B" { 0. } else { 1. };
    t.plant_toxin_type_mutation_probability = 0.;
    t.resistance_mutation_probability = 0.;
    t.initial_resistance = 0.5;
    t.resistance_effectiveness = defense;
    t.multi_type_resistance_upkeep_multiplier = 0.5;
    t.additional_type_resistance_upkeep_multiplier = 1.;
    let mut world = World::new(config)?;
    let focal = world
        .animals
        .items
        .iter()
        .map(|a| a.id)
        .min()
        .ok_or("no founder")?;
    // Snapshot all initial state except config, before changing the one trait.
    // The Python runner stores its SHA-256 and discards the bulky debug payload.
    let background = json!({
        "animals": format!("{:?}", world.animals),
        "plants": format!("{:?}", world.plants),
        "carnivores": format!("{:?}", world.carnivores),
        "resources": format!("{:?}", world.resources),
        "land": format!("{:?}", world.land),
        "entities": format!("{:?}", world.entities),
        "rng": format!("{:?}", world.rng),
        "baseline_n": world.baseline.nutrients,
        "baseline_e": world.baseline.energy,
    });
    let animal = world
        .animals
        .items
        .iter_mut()
        .find(|a| a.id == focal)
        .unwrap();
    if toxin_type == 0 {
        animal.genes.toxin_resistance = resistance;
    }
    animal.genes.typed_resistance[toxin_type] = resistance;
    world.herbivore_terminal_logging = true;
    Ok((world, focal, background))
}

pub(crate) fn observe(world: &World, id: EntityId) -> Option<Value> {
    let a = world.animals.items.iter().find(|a| a.id == id)?;
    Some(json!({
        "tick": world.tick, "alive": true, "age": a.age, "hp": a.hp,
        "births": a.successful_births, "dose_a": a.lifetime_typed_dose[0],
        "dose_b": a.lifetime_typed_dose[1], "damage": a.lifetime_toxin_damage,
        "prevented": a.lifetime_prevented_damage, "cost": a.lifetime_resistance_cost,
        "trigger": "none",
        "resistance_a": a.genes.toxin_resistance,
        "resistance_b": a.genes.typed_resistance[1],
    }))
}

fn execute(
    config: SimulationConfig,
    environment: &str,
    ty: usize,
    resistance: f32,
    defense: f64,
    ticks: u64,
    interval: u64,
) -> Result<Value, String> {
    execute_traced(
        config,
        environment,
        ty,
        resistance,
        defense,
        ticks,
        interval,
        false,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn execute_traced(
    config: SimulationConfig,
    environment: &str,
    ty: usize,
    resistance: f32,
    defense: f64,
    ticks: u64,
    interval: u64,
    trace: bool,
    behavior_window: Option<(u64, u64, Option<EntityId>)>,
) -> Result<Value, String> {
    if ticks == 0 || interval == 0 {
        return Err("positive horizon required".into());
    }
    let (mut world, focal, background) = prepare(config, environment, ty, resistance, defense)?;
    if trace {
        world.focal_trace_id = Some(focal);
    }
    world.behavior_trace_window = behavior_window.map(|(start, end, _)| (start, end));
    world.behavior_trace_subject = behavior_window.and_then(|(_, _, id)| id);
    if behavior_window.is_some() {
        world.resistance_event_logging = true;
    }
    world.record_focal_state(focal, "initial");
    let initial = observe(&world, focal).unwrap();
    let mut terminal = None;
    let mut checkpoints = vec![initial.clone()];
    let mut max_n_error = 0_f64;
    let mut max_e_error = 0_f64;
    for tick in 1..=ticks {
        world.step(1)?;
        world.record_focal_state(focal, "end_tick");
        if behavior_window.is_some() {
            world
                .focal_trace
                .push(json!({"phase":"rng_end_tick","tick":tick,
                "rng":format!("{:?}",world.rng)}));
        }
        if behavior_window.is_some() {
            for e in std::mem::take(&mut world.resistance_birth_events) {
                world.focal_trace.push(json!({"phase":"birth","tick":e.tick,
                    "parent":[e.parent_id.index,e.parent_id.generation],
                    "child":[e.child_id.index,e.child_id.generation],
                    "parent_resistance":e.parent_resistance,"child_resistance":e.child_resistance}));
            }
        }
        for event in std::mem::take(&mut world.herbivore_terminal_events) {
            if world
                .behavior_trace_window
                .is_some_and(|(start, end)| (start..=end).contains(&tick))
            {
                world
                    .focal_trace
                    .push(json!({"phase":"terminal","tick":event.tick,
                    "id":[event.id.index,event.id.generation],"hp":event.hp,
                    "resistance":event.resistance[0],"damage":event.toxin_damage_total,
                    "cost":event.resistance_cost_total,"trigger":event.removal_trigger}));
            }
            if event.id == focal {
                if terminal.is_some() {
                    return Err("duplicate focal terminal".into());
                }
                terminal = Some(json!({
                    "tick": event.tick, "alive": false, "age": event.age, "hp": event.hp,
                    "births": event.successful_births, "dose_a": event.doses[0], "dose_b": event.doses[1],
                    "damage": event.toxin_damage_total, "prevented": event.prevented_damage_total,
                    "cost": event.resistance_cost_total, "trigger": event.removal_trigger,
                    "resistance_a": event.resistance[0], "resistance_b": event.resistance[1],
                }));
            }
        }
        max_n_error =
            max_n_error.max((world.resources.total_nutrients() - world.baseline.nutrients).abs());
        max_e_error = max_e_error.max(
            ((world.resources.total_energy() - world.baseline.energy)
                - (world.ledger.sunlight_total() - world.ledger.heat_total()))
            .abs(),
        );
        if tick % interval == 0 || tick == ticks {
            if let Some(sample) = observe(&world, focal) {
                checkpoints.push(sample);
            }
        }
    }
    let survivor = observe(&world, focal);
    if terminal.is_some() == survivor.is_some() {
        return Err("missing/ambiguous focal outcome".into());
    }
    let outcome = terminal.or(survivor).unwrap();
    let mut result = json!({
        "schema_version": 1, "seed": world.config.seed, "environment": environment,
        "toxin_type": ty, "resistance": resistance, "defense": defense,
        "ticks": ticks, "interval": interval, "completed_tick": world.tick,
        "focal_id": [focal.index, focal.generation],
        "initial": initial, "outcome": outcome, "checkpoints": checkpoints,
        "max_n_error": max_n_error, "max_e_error": max_e_error,
        "initial_background": background,
        "effective_config": world.config,
        "final_ecology": world.ecology_snapshot(),
    });
    if trace {
        result["trace"] = json!(world.focal_trace);
    }
    Ok(result)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 8
        && !(args.len() == 9 && args[8] == "--trace")
        && !([11, 13].contains(&args.len()) && args[8] == "--behavior-trace")
    {
        return Err("usage: resistance_focal_pilot config environment toxin_type resistance defense ticks interval".into());
    }
    let config = serde_json::from_str(&fs::read_to_string(&args[1])?)?;
    let result = if args.len() >= 9 {
        execute_traced(
            config,
            &args[2],
            args[3].parse()?,
            args[4].parse()?,
            args[5].parse()?,
            args[6].parse()?,
            args[7].parse()?,
            true,
            if args.len() >= 11 {
                Some((
                    args[9].parse()?,
                    args[10].parse()?,
                    if args.len() == 13 {
                        Some(EntityId {
                            index: args[11].parse()?,
                            generation: args[12].parse()?,
                        })
                    } else {
                        None
                    },
                ))
            } else {
                None
            },
        )?
    } else {
        execute(
            config,
            &args[2],
            args[3].parse()?,
            args[4].parse()?,
            args[5].parse()?,
            args[6].parse()?,
            args[7].parse()?,
        )?
    };
    println!("{}", result);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_does_not_change_ecology_or_rng() {
        let (mut plain, id, _) = prepare(SimulationConfig::default(), "A", 0, 0.8, 0.8).unwrap();
        let (mut traced, _, _) = prepare(SimulationConfig::default(), "A", 0, 0.8, 0.8).unwrap();
        traced.focal_trace_id = Some(id);
        traced.behavior_trace_window = Some((1, 1000));
        for _ in 0..1000 {
            plain.step(1).unwrap();
            traced.step(1).unwrap();
            traced.record_focal_state(id, "end_tick");
        }
        assert!(plain.focal_trace.is_empty());
        assert!(!traced.focal_trace.is_empty());
        assert_eq!(plain.ecology_snapshot(), traced.ecology_snapshot());
        assert_eq!(
            format!("{:?}", plain.animals),
            format!("{:?}", traced.animals)
        );
        assert_eq!(
            format!("{:?}", plain.resources),
            format!("{:?}", traced.resources)
        );
        assert_eq!(
            format!("{:?}", plain.carnivores),
            format!("{:?}", traced.carnivores)
        );
        assert_eq!(plain.rng.next_u64(), traced.rng.next_u64());
    }

    #[test]
    fn traced_output_preserves_existing_result_schema_values() {
        let config = SimulationConfig::default();
        let plain = execute(config.clone(), "A", 0, 0.8, 0.8, 100, 100).unwrap();
        let mut traced = execute_traced(config, "A", 0, 0.8, 0.8, 100, 100, true, None).unwrap();
        assert!(traced.as_object_mut().unwrap().remove("trace").is_some());
        assert_eq!(plain, traced);
    }

    #[test]
    fn only_focal_resistance_changes_initial_world() {
        for ty in 0..2 {
            let (mut low, id, a) = prepare(SimulationConfig::default(), "A", ty, 0.2, 0.8).unwrap();
            let (mut high, id2, b) =
                prepare(SimulationConfig::default(), "A", ty, 0.8, 0.8).unwrap();
            assert_eq!(id, id2);
            assert_eq!(a, b);
            assert_eq!(low.rng.next_u64(), high.rng.next_u64());
            for (x, y) in low.animals.items.iter().zip(&high.animals.items) {
                assert_eq!(
                    (x.x, x.y, x.age, x.hp, x.genes.body_color),
                    (y.x, y.y, y.age, y.hp, y.genes.body_color)
                );
                if x.id != id {
                    assert_eq!(x.genes.toxin_resistance, y.genes.toxin_resistance);
                    assert_eq!(x.genes.typed_resistance, y.genes.typed_resistance);
                } else {
                    assert_eq!(x.genes.typed_resistance[ty], 0.2);
                    assert_eq!(y.genes.typed_resistance[ty], 0.8);
                    assert_eq!(
                        x.genes.typed_resistance[1 - ty],
                        y.genes.typed_resistance[1 - ty]
                    );
                }
            }
        }
    }

    #[test]
    fn pure_toxins_do_not_mix_and_mutation_is_disabled() {
        for (name, ty) in [("A", 0), ("B", 1)] {
            let (mut w, _, _) = prepare(SimulationConfig::default(), name, 0, 0.2, 0.8).unwrap();
            w.step(1000).unwrap();
            assert!(w.plants.items.iter().all(|p| p.toxin_type.0 == ty));
            assert_eq!(w.config.toxin.plant_toxin_type_mutation_probability, 0.);
            assert_eq!(w.config.toxin.resistance_mutation_probability, 0.);
        }
    }

    #[test]
    fn toxin_off_is_identical_under_defense_toggle() {
        let a = execute(SimulationConfig::default(), "off", 0, 0.2, 0., 1000, 100).unwrap();
        let b = execute(SimulationConfig::default(), "off", 0, 0.2, 0.8, 1000, 100).unwrap();
        for key in [
            "initial_background",
            "initial",
            "outcome",
            "checkpoints",
            "final_ecology",
            "max_n_error",
            "max_e_error",
        ] {
            assert_eq!(a[key], b[key], "{key}");
        }
    }

    #[test]
    fn focal_removal_is_recorded_without_survivor_bias() {
        let mut config = SimulationConfig::default();
        config.herbivores.max_age_ticks = 1;
        config.herbivores.reproduction_enabled = false;
        let result = execute(config, "off", 1, 0.8, 0., 10, 1).unwrap();
        assert_eq!(result["outcome"]["alive"], false);
        assert_eq!(result["outcome"]["births"], 0);
        assert_eq!(result["completed_tick"], 10);
    }
}
