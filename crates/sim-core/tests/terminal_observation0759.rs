use sim_core::{SimulationConfig, World};

fn feeding_world(toxin_type: u16) -> World {
    let mut c = SimulationConfig::default();
    c.initial_population.plants = 1;
    c.initial_population.herbivores = 1;
    c.initial_population.carnivores = 0;
    c.herbivores.reproduction_enabled = false;
    c.herbivores.plant_protection_enabled = false;
    c.herbivores.satiety_enabled = false;
    c.toxin.enabled = true;
    c.toxin.herbivore_multi_toxin_enabled = true;
    c.toxin.plant_toxin_type_variation_enabled = true;
    c.toxin.plant_toxin_type_count = 2;
    c.toxin.dose_per_nutrient = 1.;
    c.toxin.damage_threshold = 0.;
    let mut w = World::new(c).unwrap();
    w.herbivore_terminal_logging = true;
    w.plants.items[0].toxin_type = sim_core::toxin::ToxinTypeId(toxin_type);
    let (x, y) = (w.plants.items[0].x, w.plants.items[0].y);
    let a = &mut w.animals.items[0];
    a.x = x;
    a.y = y;
    a.genes.speed = 0.;
    a.age = 0;
    a.max_age = 1;
    a.successful_births = 3;
    w
}

#[test]
fn current_tick_feeding_and_age_are_included_at_removal() {
    for ty in 0..2 {
        let mut w = feeding_world(ty);
        let id = w.animals.items[0].id;
        w.step(1).unwrap();
        assert!(w.animals.items.is_empty());
        let e = &w.herbivore_terminal_events[0];
        assert_eq!(e.id, id);
        assert_eq!(e.tick, 1);
        assert_eq!(e.age, 1);
        assert_eq!(e.successful_births, 3);
        assert!(e.doses[ty as usize] > 0., "same-tick dose omitted");
        assert!(e.loads[ty as usize] > 0.);
        assert!(e.toxin_damage_total > 0.);
        assert_eq!(e.removal_trigger, "age_limit");
        assert!(e.hp > 0.);
        w.step(1).unwrap();
        assert_eq!(w.herbivore_terminal_events.len(), 1);
    }
}

#[test]
fn nonpositive_hp_is_a_trigger_not_a_claim_of_toxin_causality() {
    let mut w = feeding_world(0);
    w.animals.items[0].max_age = 100;
    w.animals.items[0].toxin_load = 100.;
    w.config.toxin.hp_damage_per_excess_unit = 10.;
    w.step(1).unwrap();
    assert_eq!(
        w.herbivore_terminal_events[0].removal_trigger,
        "hp_nonpositive"
    );
    assert!(w.herbivore_terminal_events[0].hp <= 0.);
    assert!(w.herbivore_terminal_events[0].toxin_damage_total > 0.);
}

#[test]
fn disabled_terminal_logging_does_not_accumulate_events() {
    let mut w = feeding_world(0);
    w.herbivore_terminal_logging = false;
    w.resistance_event_logging = true;
    w.step(1).unwrap();
    assert!(w.animals.items.is_empty());
    assert!(w.herbivore_terminal_events.is_empty());
}

#[test]
fn telemetry_is_observational_for_resources_positions_and_rng() {
    let config = SimulationConfig::default();
    let mut off = World::new(config.clone()).unwrap();
    let mut on = World::new(config).unwrap();
    on.herbivore_terminal_logging = true;
    for _ in 0..1000 {
        off.step(1).unwrap();
        on.step(1).unwrap();
        assert_eq!(
            off.resources.total_nutrients(),
            on.resources.total_nutrients()
        );
        assert_eq!(off.resources.total_energy(), on.resources.total_energy());
        assert_eq!(off.ledger.sunlight_total(), on.ledger.sunlight_total());
        assert_eq!(off.ledger.heat_total(), on.ledger.heat_total());
        assert_eq!(off.animals.items.len(), on.animals.items.len());
        for (a, b) in off.animals.items.iter().zip(&on.animals.items) {
            assert_eq!((a.id, a.x, a.y, a.hp), (b.id, b.x, b.y, b.hp));
        }
        on.herbivore_terminal_events.clear();
    }
    assert_eq!(off.rng.unit(), on.rng.unit());
}
