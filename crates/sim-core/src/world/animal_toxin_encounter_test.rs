use super::*;
use crate::{animals::color::BodyColor, SimulationConfig};
use serde_json::{json, Value};

fn fixture(learning: bool, blue: bool, right: bool, observed: bool) -> World {
    let mut cfg = SimulationConfig::default();
    cfg.seed = 12345;
    cfg.initial_population.plants = 1;
    cfg.initial_population.herbivores = 2;
    cfg.initial_population.carnivores = 1;
    cfg.herbivores.reproduction_enabled = false;
    cfg.herbivores.predator_evasion_enabled = false;
    cfg.carnivores.enabled = true;
    cfg.carnivores.reproduction_enabled = false;
    cfg.carnivores.initial_energy = 60.;
    cfg.carnivores.vision = 20.;
    cfg.carnivores.attack_range = 10.;
    cfg.carnivores.speed = 1.;
    cfg.carnivores.movement_strategy = 0;
    cfg.carnivores.bite_nutrients = 0.1;
    cfg.carnivores.bite_energy = 0.;
    cfg.carnivores.basal_energy_per_tick = 0.;
    cfg.carnivores.movement_energy_per_unit = 0.;
    cfg.carnivores.attack_cooldown_ticks = 0;
    cfg.carnivores.satiety_enabled = false;
    cfg.carnivores.digestion_enabled = false;
    cfg.carnivores.all_bites_to_stomach = false;
    cfg.carnivores.prey_refuge_enabled = false;
    cfg.animal_toxin.enabled = true;
    cfg.animal_toxin.color_learning_enabled = learning;
    cfg.toxin.enabled = true;
    cfg.toxin.resistance_enabled = false;
    cfg.toxin.initial_resistance = 0.;
    cfg.toxin.clearance_fraction_per_tick = 0.;
    cfg.toxin.damage_threshold = 0.;
    cfg.toxin.hp_damage_per_excess_unit = 1.;
    let mut w = World::new(cfg).unwrap();
    let color = if blue {
        BodyColor::new(0., 0., 1.)
    } else {
        BodyColor::new(1., 0., 0.)
    };
    for (i, prey) in w.animals.items.iter_mut().enumerate() {
        prey.x = if i == 0 {
            500.
        } else if right {
            700.
        } else {
            300.
        };
        prey.y = 500.;
        prey.genes.speed = 0.;
        prey.genes.body_color = color;
        let mut production = Production::founder(w.config.seed, prey.id, 71);
        production.ability.fill(0.);
        w.animal_toxins.herbivores.insert(prey.id, production);
    }
    assert_eq!(
        w.resources.nutrient_balance(w.animals.items[0].body),
        Some(30.)
    );
    w.animals.items[0].toxin_load = 30.;
    w.animal_toxins.plant_intake[0] = 30.; // Opening retained stock in the fixture's toxin balance.
    w.plants.items[0].x = 900.;
    w.plants.items[0].y = 900.;
    w.carnivores[0].x = 500.;
    w.carnivores[0].y = 500.;
    if observed {
        w.set_carnivore_toxin_trace(true);
        w.behavior_trace_window = Some((1, 301));
    }
    w
}

fn normalized(w: &mut World, ignore_memory: bool) -> Vec<u8> {
    let trace = w.animal_toxins.trace.take();
    let behavior = w.behavior_trace_window.take();
    let events = std::mem::take(&mut w.focal_trace);
    let learning = w.config.animal_toxin.color_learning_enabled;
    let id = w.carnivores[0].id;
    let memory = if ignore_memory {
        w.config.animal_toxin.color_learning_enabled = false;
        w.animal_toxins
            .predators
            .get_mut(&id)
            .map(|p| std::mem::take(&mut p.memory))
    } else {
        None
    };
    let bytes = w.audit_state(false, None);
    if let Some(memory) = memory {
        w.animal_toxins.predators.get_mut(&id).unwrap().memory = memory;
    }
    w.config.animal_toxin.color_learning_enabled = learning;
    w.focal_trace = events;
    w.behavior_trace_window = behavior;
    w.animal_toxins.trace = trace;
    bytes
}

fn step(w: &mut World, shadow: &mut World) {
    w.step(1).unwrap();
    shadow.step(1).unwrap();
    for world in [&*w, &*shadow] {
        assert!(
            world.plants.len() + world.animals.items.len() + world.carnivores.len()
                <= world.config.world.max_living_entities as usize
        );
        let toxin: Value = serde_json::from_str(&world.animal_toxin_snapshot_json()).unwrap();
        assert!(toxin["balance_error"].as_array().unwrap().iter().all(|e| e
            .as_f64()
            .unwrap()
            .abs()
            < 1e-9));
    }
    assert_eq!(normalized(w, false), normalized(shadow, false));
}

fn present(w: &mut World, right: bool) {
    let pred = &w.carnivores[0];
    w.animals.items[0].x = 800.;
    w.animals.items[1].x = pred.x + if right { 5. } else { -5. };
    w.animals.items[1].y = pred.y;
}

fn valid(w: &World, probe: EntityId) -> (bool, &'static str) {
    let Some(prey) = w.animals.items.iter().find(|p| p.id == probe && p.hp > 0.) else {
        return (false, "no_live_probe");
    };
    let Some(pred) = w.carnivores.first() else {
        return (false, "no_predator");
    };
    let c = &w.config.carnivores;
    let distance = (prey.x - pred.x).hypot(prey.y - pred.y);
    if distance >= pred.vision {
        return (false, "not_visible");
    }
    if distance > c.attack_range
        || pred.attack_cooldown > 0
        || w.resources.nutrient_balance(prey.body).unwrap() <= 0.
        || w.resources.energy_balance(pred.reserve).unwrap() <= 0.
    {
        return (false, "non_color_gate");
    }
    assert!(
        !c.satiety_enabled
            && !c.digestion_enabled
            && !c.all_bites_to_stomach
            && !c.prey_refuge_enabled
    );
    (true, "eligible")
}

#[test]
fn stage4_fixed_predator_reencounters() {
    let mut results = Vec::new();
    for blue in [false, true] {
        for right in [false, true] {
            let mut pair = Vec::new();
            for learning in [false, true] {
                let mut w = fixture(learning, blue, right, true);
                let mut shadow = fixture(learning, blue, right, false);
                assert_eq!(normalized(&mut w, false), normalized(&mut shadow, false));
                let predator = w.carnivores[0].id;
                let probe = w.animals.items[1].id;
                step(&mut w, &mut shadow);
                let damage = w.animal_toxins.damage[0];
                let memory = w.animal_toxins.predators[&predator].memory.harm_scores();
                let hp = w.animal_toxins.predators[&predator].hp;
                present(&mut w, right);
                present(&mut shadow, right);
                let shared = normalized(&mut w, true);
                if let Some(previous) = pair.pop() {
                    assert_eq!(shared, previous);
                } else {
                    pair.push(shared);
                }
                let mut reason = "no_reencounter";
                let mut encounter = None;
                let mut blocked = false;
                for _ in 0..300 {
                    let (eligible, status) = valid(&w, probe);
                    blocked |= status == "non_color_gate";
                    let attacks = w.carnivore_debug.hunt_attempts;
                    let eaten = w.carnivore_debug.hunt_nutrients;
                    let before_x = w.carnivores[0].x;
                    step(&mut w, &mut shadow);
                    if eligible {
                        reason = "observed";
                        encounter = Some(
                            json!({"tick":w.tick,"probe":trace_id(probe),"attack":w.carnivore_debug.hunt_attempts>attacks,
                            "bite_n":w.carnivore_debug.hunt_nutrients-eaten,"x_before":before_x,"x_after":w.carnivores[0].x,
                            "probe_x":w.animals.items.iter().find(|p| p.id==probe).map(|p|p.x)}),
                        );
                        break;
                    }
                }
                if encounter.is_none() && blocked {
                    reason = "non_color_gate";
                }
                results.push(json!({"blue":blue,"right":right,"learning":learning,
                    "experience_damage":damage,"experience_hp":hp,"memory":memory,"status":reason,
                    "encounter":encounter,"trace":w.take_carnivore_toxin_trace(),"behavior":w.focal_trace}));
            }
        }
    }
    println!("STAGE4_ENCOUNTER {}", json!(results));
}
