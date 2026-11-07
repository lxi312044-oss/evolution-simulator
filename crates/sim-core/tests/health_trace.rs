use serde_json::json;
use sim_core::{world::HealthTrace, SimulationConfig, World};

fn world(poison: bool, predators: bool) -> World {
    let mut c = SimulationConfig::default();
    c.initial_population.plants = 0;
    c.initial_population.herbivores = 1;
    c.initial_population.carnivores = u32::from(predators);
    c.herbivores.reproduction_enabled = false;
    c.herbivores.predator_evasion_enabled = false;
    c.toxin.enabled = poison;
    c.toxin.resistance_enabled = false;
    c.toxin.damage_threshold = 0.;
    c.toxin.clearance_fraction_per_tick = 0.;
    c.toxin.hp_damage_per_excess_unit = 1.;
    c.carnivores.enabled = predators;
    c.carnivores.prey_refuge_enabled = false;
    c.carnivores.satiety_enabled = false;
    c.carnivores.digestion_enabled = false;
    let mut w = World::new(c).unwrap();
    let a = &mut w.animals.items[0];
    a.age = 0;
    a.max_age = 10000;
    a.hp = 1.;
    a.genes.speed = 0.;
    a.x = 50.;
    a.y = 50.;
    if predators {
        w.carnivores[0].x = 50.;
        w.carnivores[0].y = 50.;
    }
    let id = w.animals.items[0].id;
    w.health_trace = Some(HealthTrace::new([id], (3500, 5100)));
    w
}

fn empty_reserve(w: &mut World) {
    let reserve = w.animals.items[0].reserve;
    let e = w.resources.energy_balance(reserve).unwrap();
    w.ledger
        .dissipate_heat(&mut w.resources, reserve, e)
        .unwrap();
}

#[test]
fn toxin_crossing_is_observed_at_the_actual_hp_write() {
    let mut w = world(true, false);
    w.animals.items[0].toxin_load = 10.;
    w.step(1).unwrap();
    let events = &w.health_trace.as_ref().unwrap().events;
    let cross = events.iter().find(|x| x["crossed_hp0"] == true).unwrap();
    assert_eq!(cross["phase"], "toxin");
    assert!(cross["hp_before"].as_f64().unwrap() > 0.);
    assert!(cross["hp_after"].as_f64().unwrap() <= 0.);
    assert!(events
        .iter()
        .any(|x| x["phase"] == "pre_removal" && x["tick"] == 1));
}

#[test]
fn zero_energy_crossing_is_distinct_from_toxin() {
    let mut w = world(false, false);
    w.animals.items[0].hp = 0.001;
    empty_reserve(&mut w);
    w.step(1).unwrap();
    let e = w
        .health_trace
        .as_ref()
        .unwrap()
        .events
        .iter()
        .find(|x| x["crossed_hp0"] == true)
        .unwrap();
    assert_eq!(e["phase"], "starvation");
    assert_eq!(e["reserve_e"], 0.);
}

#[test]
fn attack_crossing_precedes_next_tick_removal() {
    let mut w = world(false, true);
    w.step(1).unwrap();
    assert!(w
        .health_trace
        .as_ref()
        .unwrap()
        .events
        .iter()
        .any(|x| x["phase"] == "predation" && x["crossed_hp0"] == true));
    assert_eq!(w.animals.items[0].hp, 0.);
    w.step(1).unwrap();
    assert!(w
        .health_trace
        .as_ref()
        .unwrap()
        .events
        .iter()
        .any(|x| x["phase"] == "pre_removal" && x["tick"] == 2));
}

#[test]
fn earlier_toxin_and_later_starvation_are_both_retained() {
    let mut w = world(true, false);
    w.animals.items[0].toxin_load = 0.5;
    w.animals.items[0].hp = 0.50001;
    empty_reserve(&mut w);
    w.step(1).unwrap();
    let events = &w.health_trace.as_ref().unwrap().events;
    assert!(events.iter().any(|x| x["phase"] == "toxin"
        && x["crossed_hp0"] == false
        && x["hp_before"] != x["hp_after"]));
    assert_eq!(
        events.iter().find(|x| x["crossed_hp0"] == true).unwrap()["phase"],
        "starvation"
    );
}

#[test]
fn crossing_and_positive_recovery_flags_keep_both_crossings() {
    let mut w = world(false, false);
    let a = &mut w.animals.items[0];
    let mut trace = HealthTrace::new([a.id], (3500, 5100));
    a.hp = 0.;
    trace.record(
        1,
        a,
        &w.resources,
        "predation",
        Some(1.),
        json!({"damage":1.}),
    );
    a.hp = 2.;
    trace.record(
        2,
        a,
        &w.resources,
        "synthetic_recovery",
        Some(0.),
        json!({}),
    );
    a.hp = -1.;
    trace.record(3, a, &w.resources, "toxin", Some(2.), json!({"damage":3.}));
    assert_eq!(
        trace
            .events
            .iter()
            .filter(|x| x["crossed_hp0"] == true)
            .count(),
        2
    );
    assert_eq!(
        trace
            .events
            .iter()
            .filter(|x| x["recovered_positive"] == true)
            .count(),
        1
    );
}

#[test]
fn trace_is_off_by_default() {
    let w = World::new(SimulationConfig::default()).unwrap();
    assert!(w.health_trace.is_none());
}

#[test]
fn energy_trace_keeps_bite_lethal_meal_and_removal_separate() {
    let mut w = world(false, true);
    w.config.carnivores.satiety_enabled = true;
    w.config.carnivores.kill_bonus_energy = 3.;
    let id = w.animals.items[0].id;
    w.energy_trace = Some(sim_core::world::EnergyTrace::new([id]));
    w.step(1).unwrap();
    let hp_zero = w
        .health_trace
        .as_ref()
        .unwrap()
        .events
        .iter()
        .find(|e| e["crossed_hp0"] == true)
        .unwrap()["seq"]
        .as_u64()
        .unwrap();
    let events = &w.energy_trace.as_ref().unwrap().events;
    let bite = events
        .iter()
        .find(|e| e["phase"] == "predation_bite")
        .unwrap();
    let meal = events
        .iter()
        .find(|e| e["phase"] == "predation_lethal_meal")
        .unwrap();
    assert!(bite["delta_e"].as_f64().unwrap() < 0.);
    assert_eq!(meal["delta_e"], -3.);
    assert!(bite["health_seq"].as_u64().unwrap() < hp_zero);
    assert!(meal["health_seq"].as_u64().unwrap() >= hp_zero);
    w.step(1).unwrap();
    let events = &w.energy_trace.as_ref().unwrap().events;
    assert!(events.iter().any(|e| e["phase"] == "removal_heat"));
    assert_eq!(events.last().unwrap()["phase"], "removal");
    for e in events {
        assert!(
            (e["before_e"].as_f64().unwrap() + e["delta_e"].as_f64().unwrap()
                - e["after_e"].as_f64().unwrap())
            .abs()
                < 1e-10
        );
    }
}

#[test]
fn energy_trace_records_parent_payments_and_child_allocation() {
    let mut c = SimulationConfig::default();
    c.initial_population.plants = 0;
    c.initial_population.herbivores = 1;
    c.herbivores.reproduction_min_age = 0;
    c.herbivores.reproduction_cooldown_ticks = 1;
    c.herbivores.reproduction_min_energy_fraction = 0.;
    c.herbivores.reproduction_min_hp_fraction = 0.;
    c.herbivores.offspring_body_nutrients = 1.;
    c.herbivores.offspring_reserve_nutrients = 1.;
    c.herbivores.offspring_energy = 1.;
    c.herbivores.reproduction_work_energy = 0.5;
    let mut w = World::new(c).unwrap();
    let parent = w.animals.items[0].id;
    w.energy_trace = Some(sim_core::world::EnergyTrace::new([parent]));
    w.step(1).unwrap();
    assert_eq!(w.animals.items.len(), 2);
    let events = &w.energy_trace.as_ref().unwrap().events;
    assert_eq!(
        events
            .iter()
            .find(|e| e["phase"] == "offspring_transfer")
            .unwrap()["delta_e"],
        -1.
    );
    assert_eq!(
        events
            .iter()
            .find(|e| e["phase"] == "reproduction_work")
            .unwrap()["delta_e"],
        -0.5
    );
    // Replay with every possible live entity ID selected, including the not-yet-born child.
    let ids: Vec<_> = w.animals.items.iter().map(|a| a.id).collect();
    let mut replay = World::new(w.config.clone()).unwrap();
    replay.energy_trace = Some(sim_core::world::EnergyTrace::new(ids));
    replay.step(1).unwrap();
    let birth = replay
        .energy_trace
        .as_ref()
        .unwrap()
        .events
        .iter()
        .find(|e| e["phase"] == "birth")
        .unwrap();
    assert_eq!(birth["before_e"], 0.);
    assert_eq!(birth["after_e"], 1.);
    assert_eq!(birth["delta_e"], 1.);
}
