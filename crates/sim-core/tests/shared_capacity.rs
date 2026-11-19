use sim_core::{world::SpontaneousGermination, SimulationConfig, World};

fn mixed(cap: u32, seed_parents: bool) -> World {
    let mut config = SimulationConfig::default();
    config.initial_population.plants = 2;
    config.initial_population.herbivores = 2;
    config.initial_population.carnivores = 1;
    config.world.max_living_entities = cap;
    config.carnivores.enabled = true;
    config.carnivores.reproduction_enabled = false;
    config.herbivores.reproduction_enabled = !seed_parents;
    config.herbivores.reproduction_min_age = 0;
    config.herbivores.reproduction_cooldown_ticks = 1;
    config.herbivores.reproduction_min_energy_fraction = 0.;
    config.herbivores.reproduction_min_hp_fraction = 0.;
    config.herbivores.basal_energy_per_tick = 0.;
    config.herbivores.movement_energy_per_unit = 0.;
    config.herbivores.offspring_body_nutrients = 1.;
    config.herbivores.offspring_reserve_nutrients = 1.;
    config.herbivores.offspring_energy = 1.;
    config.herbivores.reproduction_work_energy = 0.5;
    config.herbivores.resource_dependent_reproduction = false;
    config.herbivores.plant_protection_enabled = true;
    config.herbivores.plant_protection_threshold = 10;
    config.plants.adult_body_nutrients = 10.;
    config.plants.seed_probability = 1.;
    config.plants.photosynthesis_per_tick = 0.;
    config.plants.basal_energy_per_tick = 0.;
    config.plants.nutrient_uptake_max = 0.;
    let mut world = World::new(config).unwrap();
    if seed_parents {
        for plant in &mut world.plants.items {
            plant.growth = 1.;
            plant.seed_cooldown = 0;
            world
                .resources
                .transfer_nutrients(
                    world.land.cells[plant.cell].soil,
                    plant.reserve,
                    world.config.plants.seed_body_nutrients,
                )
                .unwrap();
        }
    }
    world
}

fn reserve(world: &World, id: sim_core::resources::AccountId) -> (f64, f64) {
    (
        world.resources.nutrient_balance(id).unwrap(),
        world.resources.energy_balance(id).unwrap(),
    )
}

#[test]
fn seed_reservations_use_the_only_shared_slot_and_charge_only_one_parent() {
    for (cap, expected) in [(5, 0), (6, 1)] {
        let mut world = mixed(cap, true);
        let parents = [world.plants.items[0].reserve, world.plants.items[1].reserve];
        let before = parents.map(|id| reserve(&world, id));
        // A fertile soil check follows seed reproduction in the same Tick.
        world.spontaneous_germination = Some(SpontaneousGermination {
            soil_threshold: 0.,
            probability_per_cell: 1.,
            check_interval: 1,
            max_births_per_check: 4,
        });
        world.step(1).unwrap();
        assert_eq!(world.plants.len(), 2 + expected);
        assert_eq!(world.spontaneous_births, 0);
        assert_eq!(reserve(&world, parents[1]), before[1]);
        let p = &world.config.plants;
        let paid = if expected == 1 {
            (p.seed_body_nutrients, p.seed_energy + p.seed_work_energy)
        } else {
            (0., 0.)
        };
        let after = reserve(&world, parents[0]);
        assert!((before[0].0 - after.0 - paid.0).abs() < 1e-9);
        assert!((before[0].1 - after.1 - paid.1).abs() < 1e-9);
        assert_eq!(
            world.plants.len() + world.herbivore_count() as usize + world.carnivores.len(),
            cap as usize
        );
        if expected == 1 {
            let child = &world.plants.items[2];
            assert_eq!(
                world.resources.nutrient_balance(child.body).unwrap(),
                p.seed_body_nutrients
            );
            assert_eq!(
                world.resources.energy_balance(child.reserve).unwrap(),
                p.seed_energy
            );
        }
    }
}

#[test]
fn herbivore_birth_uses_only_one_shared_slot_and_charges_only_the_parent() {
    for (cap, expected) in [(5, 0), (6, 1)] {
        let mut world = mixed(cap, false);
        let parents = [
            world.animals.items[0].reserve,
            world.animals.items[1].reserve,
        ];
        let before = parents.map(|id| reserve(&world, id));
        world.step(1).unwrap();
        assert_eq!(world.herbivore_count(), 2 + expected);
        assert_eq!(world.diagnostics.herbivore_births, u64::from(expected));
        assert_eq!(reserve(&world, parents[1]), before[1]);
        let after = reserve(&world, parents[0]);
        assert!((before[0].0 - after.0 - f64::from(expected)).abs() < 1e-9);
        assert!((before[0].1 - after.1 - 1.5 * f64::from(expected)).abs() < 1e-9);
        assert_eq!(
            world.plants.len() + world.herbivore_count() as usize + world.carnivores.len(),
            cap as usize
        );
        if expected == 1 {
            let child = &world.animals.items[2];
            assert_eq!(world.resources.nutrient_balance(child.body).unwrap(), 1.);
            assert_eq!(reserve(&world, child.reserve), (1., 1.));
        }
    }
}

#[test]
fn pending_plant_removal_occupies_slot_until_herbivore_reproduction() {
    let mut world = mixed(5, true);
    world.config.herbivores.reproduction_enabled = true;
    world.plants.items[1].age = world.plants.items[1].max_age - 1;
    world.spontaneous_germination = Some(SpontaneousGermination {
        soil_threshold: 0.,
        probability_per_cell: 1.,
        check_interval: 1,
        max_births_per_check: 4,
    });
    world.step(1).unwrap();
    assert_eq!(world.plants.len(), 1);
    assert_eq!(world.diagnostics.plant_births, 0);
    assert_eq!(world.diagnostics.plant_age_deaths, 1);
    assert_eq!(world.herbivore_count(), 3);
    assert_eq!(world.carnivores.len(), 1);
}
