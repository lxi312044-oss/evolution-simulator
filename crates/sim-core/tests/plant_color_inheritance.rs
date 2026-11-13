use sim_core::{
    resources::ConservationBaseline, world::SpontaneousGermination, SimulationConfig, World,
};

const GREEN: u32 = 0x4ecb70ff;

fn founders() -> SimulationConfig {
    let mut c = SimulationConfig::default();
    c.initial_population.plants = 32;
    c.initial_population.herbivores = 0;
    c.initial_population.carnivores = 0;
    c.world.max_living_entities = 64;
    c.plants.seed_probability = 1.;
    c.plants.color_inheritance_enabled = true;
    c.plants.color_mutation_scale = 0.2;
    c.toxin.enabled = true;
    c.toxin.plant_variation_enabled = true;
    c.toxin.initial_toxic_plant_fraction = 0.;
    c.toxin.plant_toxic_mutation_probability = 0.5;
    c.toxin.plant_toxin_type_variation_enabled = true;
    c.toxin.plant_toxin_type_count = 2;
    c.toxin.plant_toxin_type_mutation_probability = 0.5;
    c
}

fn one_birth_tick(config: SimulationConfig, parent_color: u32) -> World {
    let mut world = World::new(config).unwrap();
    for p in &mut world.plants.items {
        assert_eq!(p.color, GREEN);
        assert!(!p.produces_toxin);
        p.color = parent_color;
        p.growth = 1.;
        p.seed_cooldown = 0;
        p.body = world.resources.open(100., 0.).unwrap();
        p.reserve = world.resources.open(100., 100.).unwrap();
    }
    world.baseline = ConservationBaseline::capture(&world.resources);
    let n = world.resources.total_nutrients();
    let e = world.resources.total_energy();
    world.step(1).unwrap();
    assert_eq!(world.plants.len(), 64);
    assert!(world.plants.items[..32]
        .iter()
        .all(|p| p.color == parent_color));
    assert!((world.resources.total_nutrients() - n).abs() < 1e-6);
    assert!(
        (world.resources.total_energy() - e - world.ledger.sunlight_total()
            + world.ledger.heat_total())
        .abs()
            < 1e-6
    );
    world.step(1).unwrap();
    assert_eq!(world.plants.len(), 64);
    world
}

#[test]
fn seed_mutates_visible_color_independently_of_toxicity_and_type() {
    let parent = 0x00ff80ff;
    let mut config = founders();
    let unchanged = one_birth_tick(config.clone(), parent);
    assert!(unchanged.plants.items[32..]
        .iter()
        .all(|p| p.color == parent));
    config.plants.color_mutation_probability = 1.;
    let mutated = one_birth_tick(config.clone(), parent);
    let repeated = one_birth_tick(config.clone(), parent);
    let mut observed = [false; 2];
    for ((a, b), c) in unchanged.plants.items[32..]
        .iter()
        .zip(&mutated.plants.items[32..])
        .zip(&repeated.plants.items[32..])
    {
        assert_eq!(a.id, b.id);
        assert_eq!(
            (a.produces_toxin, a.toxin_type),
            (b.produces_toxin, b.toxin_type)
        );
        assert_eq!(b.color, c.color);
        assert_eq!(b.color & 255, 255);
        for shift in [24, 16, 8] {
            let channel = ((b.color >> shift) & 255) as i32;
            let before = ((parent >> shift) & 255) as i32;
            assert!((channel - before).abs() <= 51);
        }
        observed[usize::from(b.produces_toxin)] |= b.color != parent;
    }
    assert_eq!(observed, [true, true]);
    config.toxin.plant_toxic_mutation_probability = 1.;
    config.toxin.plant_toxin_type_mutation_probability = 1.;
    let flipped = one_birth_tick(config, parent);
    for (a, b) in mutated.plants.items[32..]
        .iter()
        .zip(&flipped.plants.items[32..])
    {
        assert!(b.produces_toxin);
        assert_eq!(a.color, b.color);
    }
}

#[test]
fn germination_stays_green_and_configuration_rejects_conflicts() {
    let mut c = founders();
    c.plants.color_mutation_probability = 1.;
    c.toxin.experimental_plant_toxicity_colors = true;
    assert!(c.validate().unwrap_err().contains("同時に使えません"));
    c.toxin.experimental_plant_toxicity_colors = false;
    c.toxin.experimental_inherited_plant_colors = true;
    c.initial_population.plants = 300;
    c.world.max_living_entities = 320;
    c.toxin.plant_toxic_mutation_probability = 0.;
    c.toxin.plant_toxin_type_mutation_probability = 0.;
    assert!(c
        .validate()
        .unwrap_err()
        .contains("plants.color_inheritance_enabled"));
    c.toxin.experimental_inherited_plant_colors = false;
    c.initial_population.plants = 0;
    c.plants.color_mutation_scale = 1.1;
    assert!(c
        .validate()
        .unwrap_err()
        .contains("plants.color_mutation_scale"));
    c.plants.color_mutation_scale = 1.;
    let mut world = World::new(c).unwrap();
    world.spontaneous_germination = Some(SpontaneousGermination {
        soil_threshold: 0.,
        probability_per_cell: 1.,
        check_interval: 1,
        max_births_per_check: 1,
    });
    world.step(2).unwrap();
    assert!(world.spontaneous_births > 0);
    assert!(world.plants.items.iter().all(|p| p.color == GREEN));
}
