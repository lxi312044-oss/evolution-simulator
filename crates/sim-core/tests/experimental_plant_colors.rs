#![cfg(feature = "state-audit")]

use sim_core::{world::SpontaneousGermination, SimulationConfig, World};

#[test]
fn germination_color_is_opt_in_and_consumes_no_ecology_randomness() {
    for toxic in [false, true] {
        let mut config = SimulationConfig::default();
        config.initial_population.plants = 0;
        config.initial_population.herbivores = 0;
        config.initial_population.carnivores = 0;
        config.world.land_columns = 1;
        config.world.land_rows = 1;
        config.toxin.enabled = true;
        config.toxin.plant_variation_enabled = true;
        config.toxin.initial_toxic_plant_fraction = if toxic { 1. } else { 0. };
        let mut legacy = World::new(config.clone()).unwrap();
        config.toxin.experimental_plant_toxicity_colors = true;
        let mut mixed = World::new(config).unwrap();
        let germination = SpontaneousGermination {
            soil_threshold: 20.,
            probability_per_cell: 1.,
            check_interval: 20,
            max_births_per_check: 1,
        };
        for world in [&mut legacy, &mut mixed] {
            world.spontaneous_germination = Some(germination);
            world.step(20).unwrap();
            assert_eq!(world.spontaneous_births, 1);
            assert_eq!(world.plants.items[0].produces_toxin, toxic);
        }
        assert_eq!(legacy.plants.items[0].color, 0x4ecb70ff);
        assert_eq!(
            mixed.plants.items[0].color,
            if toxic { 0xb86be3ff } else { 0x4ecb70ff }
        );
        mixed.config.toxin.experimental_plant_toxicity_colors = false;
        mixed.plants.items[0].color = 0x4ecb70ff;
        assert_eq!(
            legacy.audit_state(false, None),
            mixed.audit_state(false, None)
        );
    }
}
