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

#[test]
fn inherited_colors_keep_four_classes_and_parent_traits() {
    for toxin_enabled in [true, false] {
        let mut config: SimulationConfig = serde_json::from_str(include_str!(
            "../../../config/stage4-inherited-plant-colors.json"
        ))
        .unwrap();
        config.toxin.enabled = toxin_enabled;
        config.initial_population.herbivores = 0;
        config.initial_population.carnivores = 0;
        config.carnivores.enabled = false;
        config.plants.seed_probability = 1.;
        config.world.max_living_entities = 601;
        let mut legacy_config = config.clone();
        legacy_config.toxin.experimental_inherited_plant_colors = false;
        let mut legacy = World::new(legacy_config).unwrap();
        let mut world = World::new(config.clone()).unwrap();
        let traits = |p: &sim_core::plants::Plant| (p.color, p.produces_toxin, p.toxin_type.0);
        let mut counts = [[0; 3]; 2];
        for (original, assigned) in legacy.plants.items.iter_mut().zip(&world.plants.items) {
            let (color, toxic, kind) = traits(assigned);
            counts[usize::from(color == 0xb86be3ff)][if toxic { 1 + kind as usize } else { 0 }] +=
                1;
            original.color = color;
            original.produces_toxin = toxic;
            original.toxin_type = assigned.toxin_type;
        }
        assert_eq!(counts, [[120, 15, 15], [30, 60, 60]]);
        legacy.config.toxin.experimental_inherited_plant_colors = true;
        assert_eq!(
            legacy.audit_state(false, None),
            world.audit_state(false, None)
        );
        let parents: Vec<_> = world.plants.items.iter().map(traits).collect();
        for plant in &mut world.plants.items {
            plant.growth = 1.;
            plant.seed_cooldown = 0;
            plant.body = world.resources.open(100., 0.).unwrap();
            plant.reserve = world.resources.open(100., 100.).unwrap();
        }
        world.spontaneous_germination = Some(SpontaneousGermination {
            soil_threshold: 0.,
            probability_per_cell: 1.,
            check_interval: 1,
            max_births_per_check: 1,
        });
        world.baseline = sim_core::resources::ConservationBaseline::capture(&world.resources);
        let n = world.resources.total_nutrients();
        let e = world.resources.total_energy();
        world.step(1).unwrap();
        assert_eq!(world.spontaneous_births, 0);
        assert_eq!(world.plants.len(), 600);
        assert_eq!(
            parents,
            world.plants.items[300..]
                .iter()
                .map(traits)
                .collect::<Vec<_>>()
        );
        assert!((world.resources.total_nutrients() - n).abs() < 1e-6);
        assert!(
            (world.resources.total_energy() - e - world.ledger.sunlight_total()
                + world.ledger.heat_total())
            .abs()
                < 1e-6
        );
        config.toxin.experimental_plant_toxicity_colors = true;
        assert!(World::new(config).is_err());
    }
}

#[test]
fn plant_toxin_toggle_preserves_founders_and_other_switches() {
    let on: SimulationConfig = serde_json::from_str(include_str!(
        "../../../config/stage4-inherited-predator-candidate.json"
    ))
    .unwrap();
    let mut off = on.clone();
    off.toxin.enabled = false;
    let mut disabled = World::new(off).unwrap();
    let enabled = World::new(on).unwrap();
    assert!(disabled.config.animal_toxin.enabled);
    assert!(disabled.config.toxin.resistance_enabled);
    assert!(disabled.config.herbivores.color_learning_enabled);
    assert!(disabled.config.animal_toxin.color_learning_enabled);
    disabled.config.toxin.enabled = true;
    assert_eq!(
        disabled.audit_state(false, None),
        enabled.audit_state(false, None)
    );
    disabled.config.toxin.enabled = false;
    disabled.step(100).unwrap();
    assert!(disabled
        .animals
        .items
        .iter()
        .all(|a| a.lifetime_toxin_dose == 0.));
}

#[test]
fn disabled_toxin_does_not_bypass_color_profile_validation() {
    let mut config: SimulationConfig = serde_json::from_str(include_str!(
        "../../../config/stage4-inherited-predator-candidate.json"
    ))
    .unwrap();
    config.toxin.enabled = false;
    config.initial_population.plants = 299;
    assert!(config
        .validate()
        .unwrap_err()
        .contains("initial_population.plants=300"));
    config.initial_population.plants = 300;
    config.toxin.experimental_plant_toxicity_colors = true;
    assert!(config.validate().unwrap_err().contains("同時に使えません"));
    config.toxin.experimental_inherited_plant_colors = false;
    assert!(World::new(config.clone()).is_ok());
    config.initial_population.plants = 299;
    assert!(config.validate().unwrap_err().contains("偶数"));
}
