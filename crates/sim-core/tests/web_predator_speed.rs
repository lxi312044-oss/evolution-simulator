use sim_core::{SimulationConfig, World};

#[test]
fn representative_flee_and_pursuit_use_configured_speed_and_movement_cost() {
    for speed in [4.2, 5.25] {
        let mut config: SimulationConfig =
            serde_json::from_str(include_str!("../../../config/standard-toxin.json")).unwrap();
        config.initial_population.plants = 0;
        config.initial_population.herbivores = 1;
        config.initial_population.carnivores = 1;
        config.carnivores.speed = speed;
        let mut world = World::new(config).unwrap();
        let prey = &mut world.animals.items[0];
        (prey.x, prey.y) = (540., 500.);
        prey.genes.speed = 2.5;
        prey.genes.fear = 1.;
        prey.genes.aggression = 0.;
        (world.carnivores[0].x, world.carnivores[0].y) = (500., 500.);

        world.step(10).unwrap();

        assert_eq!(world.carnivore_debug.herbivore_predator_detected_ticks, 10);
        assert_eq!(world.carnivore_debug.predator_target_ticks, 10);
        assert!((world.animals.items[0].x - 575.).abs() < 0.001);
        assert!(
            (world.carnivore_debug.predator_movement_distance - f64::from(speed) * 10.).abs()
                < 0.001
        );
        assert!(
            (world.carnivore_debug.energy_movement_spent - f64::from(speed) * 10. * 0.008).abs()
                < 0.00001
        );
        assert_eq!(world.carnivore_hunts, 0);
    }
}
