use sim_core::{SimulationConfig, World};

const RED: u32 = 0xff0000ff;
const NEAR_RED: u32 = 0xef0000ff;
const BLUE: u32 = 0x0000ffff;

fn fixture(learning: bool) -> World {
    let mut config = SimulationConfig::default();
    config.initial_population.plants = 1;
    config.initial_population.herbivores = 1;
    config.initial_population.carnivores = 0;
    config.herbivores.color_learning_enabled = learning;
    config.herbivores.reproduction_enabled = false;
    config.herbivores.feeding_nutrients_per_tick = 0.35;
    config.herbivores.basal_energy_per_tick = 0.;
    config.herbivores.movement_energy_per_unit = 0.;
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

#[test]
fn delayed_experience_changes_actual_feeding_and_hunger_can_restore_it() {
    let mut on = fixture(true);
    let mut off = fixture(false);
    for world in [&mut on, &mut off] {
        world.step(1).unwrap();
        assert_eq!(world.diagnostics.herbivore_feeding_events, 1);
        assert_eq!(world.diagnostics.toxin_hp_damage_total, 0.);
        world.step(1).unwrap();
        assert_eq!(world.diagnostics.herbivore_feeding_events, 2);
        assert!(world.diagnostics.toxin_hp_damage_total > 0.);
    }

    on.step(1).unwrap();
    off.step(1).unwrap();
    assert_eq!(on.diagnostics.herbivore_feeding_events, 2);
    assert_eq!(off.diagnostics.herbivore_feeding_events, 3);
    assert_eq!(on.animals.items[0].target, None);

    on.plants.items[0].produces_toxin = false;
    for color in [RED, NEAR_RED] {
        on.plants.items[0].color = color;
        on.step(1).unwrap();
        assert_eq!(on.diagnostics.herbivore_feeding_events, 2);
    }
    on.plants.items[0].color = BLUE;
    on.step(1).unwrap();
    assert_eq!(on.diagnostics.herbivore_feeding_events, 3);

    let reserve = on.animals.items[0].reserve;
    let energy = on.resources.energy_balance(reserve).unwrap();
    on.ledger
        .dissipate_heat(&mut on.resources, reserve, energy)
        .unwrap();
    on.plants.items[0].color = RED;
    on.step(1).unwrap();
    assert_eq!(on.diagnostics.herbivore_feeding_events, 4);
}

#[test]
fn offspring_has_no_parent_color_memory() {
    let mut world = fixture(true);
    world.step(2).unwrap();
    let parent = world.animals.items[0].id;
    assert!(!world.plant_color_learning[&parent].allows(RED, 1.));
    let h = &mut world.config.herbivores;
    h.reproduction_enabled = true;
    h.reproduction_min_age = 0;
    h.reproduction_min_hp_fraction = 0.;
    h.reproduction_min_energy_fraction = 0.;
    h.resource_dependent_reproduction = false;
    h.offspring_body_nutrients = 1.;
    h.offspring_reserve_nutrients = 1.;
    h.offspring_energy = 1.;
    h.reproduction_work_energy = 0.;
    world.animals.items[0].reproduction_cooldown = 0;
    world.step(1).unwrap();
    assert_eq!(world.herbivore_count(), 2);
    assert!(!world
        .plant_color_learning
        .contains_key(&world.animals.items[1].id));
}

#[test]
fn learned_color_changes_the_approach_target_before_bite_range() {
    let mut world = fixture(true);
    world.step(2).unwrap();
    let plant = &mut world.plants.items[0];
    plant.x = 520.;
    plant.y = 500.;
    let plant_id = plant.id;
    let animal = &mut world.animals.items[0];
    animal.x = 500.;
    animal.y = 500.;
    animal.genes.vision = 100.;
    animal.genes.speed = 1.;
    world.step(1).unwrap();
    assert_eq!(world.animals.items[0].target, None);
    assert_eq!(world.diagnostics.herbivore_feeding_events, 2);

    world.plants.items[0].color = BLUE;
    let before_x = world.animals.items[0].x;
    world.step(1).unwrap();
    assert_eq!(world.animals.items[0].target, Some(plant_id));
    assert!(world.animals.items[0].x > before_x);
    assert_eq!(world.diagnostics.herbivore_feeding_events, 2);
}
