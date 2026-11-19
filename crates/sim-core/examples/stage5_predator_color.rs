//! Issue #39 fixed-window observer. The only ecological intervention is carnivore learning.
use serde_json::{json, Value};
use sim_core::{entity::EntityId, SimulationConfig, World};
use std::{
    collections::{BTreeMap, BTreeSet},
    hash::{Hash, Hasher},
    time::{Duration, Instant},
};

const END: u64 = 6000;

fn id(value: EntityId) -> [u32; 2] {
    [value.index, value.generation]
}

fn sample(world: &World) -> Value {
    json!({"tick":world.tick,"plants":world.plants.len(),"herbivores":world.animals.items.len(),
        "carnivores":world.carnivores.len(),"births":world.diagnostics.herbivore_births,
        "removals":world.diagnostics.herbivore_hp_deaths + world.diagnostics.herbivore_age_deaths})
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let args: Vec<_> = std::env::args().collect();
    let seed: u64 = args[1].parse()?;
    assert!((1574032001..=1574032128).contains(&seed) || seed == 12345);
    let on = match args[2].as_str() {
        "on" => true,
        "off" => false,
        _ => return Err("expected on or off".into()),
    };
    let end = if seed == 12345 { 2000 } else { END };
    let mut config: SimulationConfig = serde_json::from_str(include_str!(
        "../../../config/stage4-inherited-predator-candidate.json"
    ))?;
    config.seed = seed;
    config.animal_toxin.color_learning_enabled = on;
    let cap = config.world.max_living_entities as usize;
    let mut world = World::new(config.clone())?;
    config.animal_toxin.color_learning_enabled = !on;
    let mut shadow = World::new(config)?;
    world.config.animal_toxin.color_learning_enabled = false;
    shadow.config.animal_toxin.color_learning_enabled = false;
    let initial = world.audit_state(false, None);
    assert_eq!(initial, shadow.audit_state(false, None));
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    initial.hash(&mut hasher);
    let initial_hash = format!("{:016x}", hasher.finish());
    world.config.animal_toxin.color_learning_enabled = on;
    shadow.config.animal_toxin.color_learning_enabled = on;
    let founders: Vec<_> = world
        .animals
        .items
        .iter()
        .map(|a| {
            let rgb = a.genes.body_color.rgb();
            json!({"id":id(a.id),"rgb":rgb,"rgb8":rgb.map(|v| (v * 255.).round() as u8),
            "load":[a.toxin_load,a.typed_load[1]],"stock":[0.,0.],
            "resistance":[a.genes.toxin_resistance,a.genes.typed_resistance[1]],"x":a.x,"y":a.y,
            "age":a.age,"hp":a.hp,"e":world.resources.energy_balance(a.reserve).unwrap(),
            "body_n":world.resources.nutrient_balance(a.body).unwrap(),
            "reserve_n":world.resources.nutrient_balance(a.reserve).unwrap()})
        })
        .collect();
    assert_eq!(founders.len(), 80);
    let mut samples = vec![sample(&world)];
    let mut births = Vec::new();
    let mut removals = Vec::new();
    let mut predator_events = Vec::new();
    let mut windows = BTreeMap::<u64, BTreeMap<&str, u64>>::new();
    let mut first_cases = BTreeSet::new();
    let mut max_population =
        world.plants.len() + world.animals.items.len() + world.carnivores.len();
    let mut max_n: f64 = 0.;
    let mut max_e: f64 = 0.;
    let mut max_toxin: f64 = 0.;
    let mut audit_points = vec![0];
    world.resistance_event_logging = true;
    world.herbivore_terminal_logging = true;
    world.set_color_event_logging(true);
    world.enable_carnivore_opportunity_trace();
    for tick in 1..=end {
        if start.elapsed() >= Duration::from_secs(120) {
            return Err(format!("120-second limit at Tick {tick}").into());
        }
        world.step(1)?;
        shadow.step(1)?;
        let n = world.resources.total_nutrients() - world.baseline.nutrients;
        let e = world.resources.total_energy()
            - world.baseline.energy
            - (world.ledger.sunlight_total() - world.ledger.heat_total());
        assert!(n.is_finite() && e.is_finite());
        max_n = max_n.max(n.abs());
        max_e = max_e.max(e.abs());
        let population = world.plants.len() + world.animals.items.len() + world.carnivores.len();
        assert!(population <= cap);
        assert!(shadow.plants.len() + shadow.animals.items.len() + shadow.carnivores.len() <= cap);
        max_population = max_population.max(population);

        let genealogy = std::mem::take(&mut world.resistance_birth_events);
        let colors = world.take_color_events();
        assert_eq!(genealogy.len(), colors.len());
        for (event, color) in genealogy.iter().zip(colors) {
            assert_eq!(
                (event.tick, event.parent_id, event.child_id),
                (color.tick, color.parent_id, color.child_id)
            );
            births.push(
                json!({"tick":event.tick,"parent":id(event.parent_id),"id":id(event.child_id),
                "rgb":color.child_rgb,"rgb8":color.child_rgb.map(|v| (v * 255.).round() as u8),
                "mutated":color.mutated}),
            );
        }
        for event in std::mem::take(&mut world.herbivore_terminal_events) {
            removals
                .push(json!({"tick":event.tick,"id":id(event.id),"reason":event.removal_trigger}));
        }
        let counters = windows.entry((tick - 1) / 500).or_default();
        for event in world.take_carnivore_toxin_trace() {
            match event["event"].as_str().unwrap() {
                "clearance" => {}
                "opportunity" => {
                    let attack = event["attacked"] == true;
                    if attack {
                        *counters.entry("attack").or_default() += 1;
                    }
                    let mut retain = attack;
                    for candidate in event["learned_ready"].as_array().unwrap() {
                        let relation = if candidate["original"] == true {
                            "original"
                        } else if candidate["color"] == candidate["memory_color"] {
                            "same"
                        } else {
                            "similar"
                        };
                        let prey = &candidate["prey"];
                        let selected = event["target"]["id"] == *prey;
                        let skip = event["selection_color_skips"]
                            .as_array()
                            .unwrap()
                            .contains(prey)
                            || selected && event["attack_color_rejected"] == true;
                        let outcome = if skip {
                            "rejected"
                        } else if selected && attack {
                            "attacked"
                        } else {
                            "other"
                        };
                        *counters.entry("attackable_learned").or_default() += 1;
                        *counters.entry(outcome).or_default() += 1;
                        retain |= first_cases.insert((event["id"].to_string(), relation, outcome));
                    }
                    if retain {
                        predator_events.push(event);
                    }
                }
                kind => {
                    match kind {
                        "transfer" => *counters.entry("transfer").or_default() += 1,
                        "damage" => *counters.entry("damage").or_default() += 1,
                        "decision" | "removal" | "learned_target" => {}
                        _ => {}
                    }
                    predator_events.push(event);
                }
            }
        }
        if tick % 500 == 0 {
            world.set_color_event_logging(false);
            assert_eq!(
                world.audit_without_carnivore_trace(),
                shadow.audit_state(false, None),
                "observation divergence at Tick {tick}"
            );
            world.set_color_event_logging(true);
            let toxins: Value = serde_json::from_str(&world.animal_toxin_snapshot_json())?;
            for error in toxins["balance_error"].as_array().unwrap() {
                let value = error.as_f64().unwrap().abs();
                assert!(value < 0.001);
                max_toxin = max_toxin.max(value);
            }
            audit_points.push(tick);
            samples.push(sample(&world));
        }
    }
    assert_eq!(births.len() as u64, world.diagnostics.herbivore_births);
    assert_eq!(
        removals.len() as u64,
        world.diagnostics.herbivore_hp_deaths + world.diagnostics.herbivore_age_deaths
    );
    let survivors: Vec<_> = world.animals.items.iter().map(|a| id(a.id)).collect();
    println!(
        "{}",
        json!({"seed":seed,"on":on,"tick":world.tick,"initial_hash":initial_hash,
        "founders":founders,"births":births,"removals":removals,"survivors":survivors,
        "predator_events":predator_events,"windows":windows,"samples":samples,
        "audit_points":audit_points,"max_n_error":max_n,"max_e_error":max_e,
        "max_toxin_error":max_toxin,"max_population":max_population,"seconds":start.elapsed().as_secs_f64()})
    );
    Ok(())
}
