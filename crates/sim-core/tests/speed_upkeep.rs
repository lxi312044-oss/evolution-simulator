//! Issue #11: opt-in speed upkeep (basal energy scaled by individual speed).
use sim_core::animals::speed_upkeep_factor;
use sim_core::{SimulationConfig, World};

const STAGE4: &str = include_str!("../../../config/stage4-inherited-plant-colors.json");

/// Round 3 w_E settings on the stage 4 config.
fn w_e() -> SimulationConfig {
    let mut config: SimulationConfig = serde_json::from_str(STAGE4).unwrap();
    config.seed = 90001;
    config.carnivores.speed = 5.25;
    config.herbivores.color_memory_decay_per_tick = 0.0007;
    config.carnivores.sustainable_reproduction_enabled = false;
    config.carnivores.feeding_resume_fraction = 0.2;
    config.carnivores.food_limited_reproduction_enabled = true;
    config.carnivores.prey_trend_reproduction_enabled = true;
    config.carnivores.basal_energy_per_tick = 0.0072;
    config
}

fn populations(config: SimulationConfig) -> Vec<(usize, u32, usize)> {
    let mut world = World::new(config).unwrap();
    (0..8)
        .map(|_| {
            world.step(500).unwrap();
            (
                world.carnivores.len(),
                world.herbivore_count(),
                world.plants.items.len(),
            )
        })
        .collect()
}

#[test]
fn factor_is_one_at_zero_slope_and_scales_with_speed() {
    for speed in [0.5_f32, 1., 2.5, 4., 8.] {
        assert_eq!(speed_upkeep_factor(0., speed, 2.5), 1.);
    }
    assert!((speed_upkeep_factor(0.5, 4., 2.5) - 1.3).abs() < 1e-9);
    assert!((speed_upkeep_factor(0.5, 1., 2.5) - 0.7).abs() < 1e-9);
    assert!((speed_upkeep_factor(1., 2.5, 2.5) - 1.).abs() < 1e-9);
    assert_eq!(speed_upkeep_factor(5., 0.5, 2.5), 0.);
}

#[test]
fn zero_slope_matches_previous_behavior_and_is_not_serialized() {
    let text = serde_json::to_string(&SimulationConfig::default()).unwrap();
    assert!(!text.contains("speed_upkeep_slope"));
    let mut explicit = w_e();
    explicit.herbivores.speed_upkeep_slope = 0.;
    explicit.carnivores.speed_upkeep_slope = 0.;
    assert_eq!(populations(w_e()), populations(explicit));
}

#[test]
fn faster_herbivores_pay_more_basal_energy() {
    let mut config = SimulationConfig::default();
    config.initial_population.plants = 0;
    config.initial_population.herbivores = 2;
    config.initial_population.carnivores = 0;
    config.herbivores.reproduction_enabled = false;
    config.herbivores.movement_energy_per_unit = 0.;
    config.herbivores.speed_upkeep_slope = 1.;
    let mut world = World::new(config).unwrap();
    let speeds = [1.5_f32, 3.5];
    for (h, s) in world.animals.items.iter_mut().zip(speeds) {
        h.genes.speed = s;
        h.genes.adult_max_hp = 60.;
    }
    let energy = |w: &World| -> Vec<f64> {
        w.animals
            .items
            .iter()
            .map(|h| w.resources.energy_balance(h.reserve).unwrap())
            .collect()
    };
    let before = energy(&world);
    world.step(10).unwrap();
    let after = energy(&world);
    let spent: Vec<f64> = before.iter().zip(&after).map(|(b, a)| b - a).collect();
    assert!(spent[1] > spent[0] * 1.5, "spent {spent:?}");
}

#[test]
fn invalid_slopes_are_rejected() {
    for (h, c) in [(-0.1, 0.), (0., 5.5), (f64::NAN, 0.)] {
        let mut config = w_e();
        config.herbivores.speed_upkeep_slope = h;
        config.carnivores.speed_upkeep_slope = c;
        let Err(err) = World::new(config) else {
            panic!("should be rejected");
        };
        assert!(err.contains("speed_upkeep_slope"), "{err}");
    }
}
