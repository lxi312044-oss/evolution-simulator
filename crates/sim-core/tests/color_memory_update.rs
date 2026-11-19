//! Issue #2 (ECO-001): opt-in weakening of learned plant-color harm.
use sim_core::{SimulationConfig, World};

const RED: u32 = 0xff0000ff;
const BLUE: u32 = 0x0000ffff;

fn fixture(relief: f64, decay: f64) -> World {
    let mut config = SimulationConfig::default();
    config.initial_population.plants = 1;
    config.initial_population.herbivores = 1;
    config.initial_population.carnivores = 0;
    let h = &mut config.herbivores;
    h.color_learning_enabled = true;
    h.color_memory_safe_meal_relief = relief;
    h.color_memory_decay_per_tick = decay;
    h.reproduction_enabled = false;
    h.feeding_nutrients_per_tick = 0.35;
    h.basal_energy_per_tick = 0.;
    h.movement_energy_per_unit = 0.;
    config.toxin.enabled = true;
    config.toxin.resistance_enabled = false;
    config.toxin.dose_per_nutrient = 2.;
    config.toxin.damage_threshold = 0.85;
    config.toxin.hp_damage_per_excess_unit = 1.;
    let mut world = World::new(config).unwrap();
    let animal = &mut world.animals.items[0];
    animal.genes.speed = 0.;
    world.plants.items[0].x = animal.x;
    world.plants.items[0].y = animal.y;
    world.plants.items[0].color = RED;
    world.plants.items[0].produces_toxin = true;
    world
}

/// Learned harm for each remembered color, as exposed to the Web inspector.
fn harm(world: &World, rgb_red: f64) -> f64 {
    let id = world.animals.items[0].id;
    let view = world.inspect_entity(id.index, id.generation);
    view["memory"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| (m["rgb"][0].as_f64().unwrap() - rgb_red).abs() < 1e-9)
        .map_or(0., |m| m["harm"].as_f64().unwrap())
}

/// Learn RED as harmful, then offer a toxin-free plant while the animal is starving
/// (tolerance 1), so it eats regardless of its memory.
fn learn_then_safe_meal(world: &mut World, safe_color: u32) {
    world.step(2).unwrap();
    assert!(!world.plant_color_learning[&world.animals.items[0].id].allows(RED, 1.));
    // Let the toxin load clear so that no further damage is attributed.
    world.animals.items[0].toxin_load = 0.;
    world.plants.items[0].produces_toxin = false;
    world.plants.items[0].color = safe_color;
    let reserve = world.animals.items[0].reserve;
    let energy = world.resources.energy_balance(reserve).unwrap();
    world
        .ledger
        .dissipate_heat(&mut world.resources, reserve, energy)
        .unwrap();
    let before = world.diagnostics.herbivore_feeding_events;
    world.step(1).unwrap();
    assert_eq!(world.diagnostics.herbivore_feeding_events, before + 1);
}

#[test]
fn defaults_keep_learned_harm_after_a_safe_meal() {
    let mut world = fixture(0., 0.);
    learn_then_safe_meal(&mut world, RED);
    assert!(harm(&world, 1.) > 0.);
    let mut relieved = fixture(0.5, 0.);
    learn_then_safe_meal(&mut relieved, RED);
    assert!(harm(&relieved, 1.) > 0.);
    assert!((harm(&relieved, 1.) - 0.5 * harm(&world, 1.)).abs() < 1e-12);
}

#[test]
fn a_safe_meal_of_a_distant_color_does_not_relieve() {
    let mut baseline = fixture(0., 0.);
    learn_then_safe_meal(&mut baseline, BLUE);
    let mut relieved = fixture(0.5, 0.);
    learn_then_safe_meal(&mut relieved, BLUE);
    assert!(harm(&baseline, 1.) > 0.);
    assert_eq!(harm(&relieved, 1.), harm(&baseline, 1.));
}

#[test]
fn toxic_meals_do_not_count_as_safe_evidence() {
    let mut baseline = fixture(0., 0.);
    let mut relieved = fixture(0.5, 0.);
    for world in [&mut baseline, &mut relieved] {
        world.step(3).unwrap();
    }
    assert_eq!(harm(&relieved, 1.), harm(&baseline, 1.));
}

#[test]
fn decay_fades_learned_harm_each_tick() {
    let mut baseline = fixture(0., 0.);
    let mut decaying = fixture(0., 0.01);
    for world in [&mut baseline, &mut decaying] {
        world.step(2).unwrap();
        world.animals.items[0].toxin_load = 0.;
        world.plants.items[0].color = BLUE;
        world.plants.items[0].produces_toxin = false;
    }
    let start = harm(&decaying, 1.);
    assert_eq!(start, harm(&baseline, 1.));
    for world in [&mut baseline, &mut decaying] {
        world.step(10).unwrap();
    }
    assert_eq!(harm(&baseline, 1.), start);
    assert!((harm(&decaying, 1.) - start * 0.99_f64.powi(10)).abs() < 1e-12);
}

#[test]
fn settings_are_validated_and_omitted_when_zero() {
    let config = SimulationConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.contains("color_memory_safe_meal_relief"));
    assert!(!json.contains("color_memory_decay_per_tick"));
    for (relief, decay, ok) in [
        (0., 0., true),
        (1., 1., true),
        (0.05, 0.0007, true),
        (-0.1, 0., false),
        (0., 1.5, false),
        (f64::NAN, 0., false),
    ] {
        let mut c = SimulationConfig::default();
        c.herbivores.color_memory_safe_meal_relief = relief;
        c.herbivores.color_memory_decay_per_tick = decay;
        assert_eq!(c.validate().is_ok(), ok, "relief={relief} decay={decay}");
    }
    let mut c = SimulationConfig::default();
    c.herbivores.color_memory_safe_meal_relief = 0.05;
    let json = serde_json::to_string(&c).unwrap();
    assert!(json.contains("\"color_memory_safe_meal_relief\":0.05"));
    let back: SimulationConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.herbivores.color_memory_safe_meal_relief, 0.05);
}
