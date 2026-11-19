use sim_core::{resources::ConservationBaseline, SimulationConfig, World};

#[test]
fn carnivore_parent_reserve_is_a_gate_and_is_not_spent() {
    for (floor, energy, birth) in [(0., 24., true), (4., 27.999, false), (4., 28., true)] {
        let mut config = SimulationConfig::default();
        config.initial_population.plants = 0;
        config.initial_population.herbivores = 0;
        config.initial_population.carnivores = 1;
        let c = &mut config.carnivores;
        c.enabled = true;
        c.reproduction_parent_reserve_energy = floor;
        c.initial_energy = energy;
        c.initial_reserve_nutrients = 20.;
        c.offspring_energy = 22.;
        c.basal_energy_per_tick = 0.;
        c.movement_energy_per_unit = 0.;
        let mut world = World::new(config).unwrap();
        let parent = &mut world.carnivores[0];
        parent.age = 1000;
        parent.cooldown = 0;
        let reserve = parent.reserve;
        let stomach = parent.stomach;
        // An inaccessible stomach balance must not count toward this gate.
        world
            .ledger
            .capture_sunlight(&mut world.resources, stomach, 10.)
            .unwrap();
        world.ledger = Default::default();
        world.baseline = ConservationBaseline::capture(&world.resources);
        world.step(1).unwrap();
        assert_eq!(world.carnivore_births, u64::from(birth));
        // With no reserve left the parent is removed in the legacy 24 E case.
        if floor > 0. {
            assert_eq!(
                world.resources.energy_balance(reserve).unwrap(),
                energy - if birth { 24. } else { 0. }
            );
        }
        if birth {
            let child = world
                .carnivores
                .iter()
                .find(|animal| animal.parent_id.is_some())
                .unwrap();
            assert_eq!(world.resources.energy_balance(child.reserve).unwrap(), 22.);
            assert_eq!(world.carnivore_debug.energy_reproduction_spent, 2.);
        }
    }
}
