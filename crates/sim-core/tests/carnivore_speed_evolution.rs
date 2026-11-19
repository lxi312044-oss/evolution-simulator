//! Issue #11: opt-in heritable carnivore speed with mutation.
use sim_core::animals::carnivore::{inherit_speed, initial_speed, SPEED_MAX, SPEED_MIN};
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

fn trajectory(config: SimulationConfig, ticks: u64) -> Vec<(usize, u32, usize, Vec<[f32; 3]>)> {
    let mut world = World::new(config).unwrap();
    let mut out = Vec::new();
    for _ in 0..ticks / 500 {
        world.step(500).unwrap();
        let carnivores = world
            .carnivores
            .iter()
            .map(|c| [c.x, c.y, c.speed])
            .collect();
        out.push((
            world.carnivores.len(),
            world.herbivore_count(),
            world.plants.items.len(),
            carnivores,
        ));
    }
    out
}

#[test]
fn on_without_spread_or_mutation_matches_off() {
    let off = w_e();
    let mut on = w_e();
    on.carnivores.speed_evolution_enabled = true;
    assert_eq!(trajectory(off, 4000), trajectory(on, 4000));
}

#[test]
fn defaults_leave_saved_config_unchanged() {
    let text = serde_json::to_string(&SimulationConfig::default().carnivores).unwrap();
    for key in [
        "speed_evolution_enabled",
        "initial_speed_spread",
        "speed_mutation_probability",
        "speed_mutation_scale",
    ] {
        assert!(
            !text.contains(key),
            "{key} should not be serialized by default"
        );
    }
}

#[test]
fn initial_spread_varies_speed_within_bounds() {
    let mut config = w_e();
    config.carnivores.speed_evolution_enabled = true;
    config.carnivores.initial_speed_spread = 0.25;
    let world = World::new(config).unwrap();
    let speeds: Vec<f32> = world.carnivores.iter().map(|c| c.speed).collect();
    assert!(speeds.len() > 1);
    assert!(speeds
        .iter()
        .all(|s| (5.25 * 0.75 - 1e-4..=5.25 * 1.25 + 1e-4).contains(s)));
    assert!(speeds.iter().any(|s| (s - speeds[0]).abs() > 1e-3));
}

#[test]
fn spread_does_not_shift_ecology_rng() {
    // Speed draws use a separate stream, so initial positions are unchanged.
    let off = World::new(w_e()).unwrap();
    let mut config = w_e();
    config.carnivores.speed_evolution_enabled = true;
    config.carnivores.initial_speed_spread = 0.25;
    let on = World::new(config).unwrap();
    let pos = |w: &World| w.carnivores.iter().map(|c| [c.x, c.y]).collect::<Vec<_>>();
    assert_eq!(pos(&off), pos(&on));
    let herb = |w: &World| {
        w.animals
            .items
            .iter()
            .map(|h| [h.x, h.y, h.genes.speed])
            .collect::<Vec<_>>()
    };
    assert_eq!(herb(&off), herb(&on));
}

#[test]
fn inheritance_keeps_or_mutates_within_scale() {
    for id in 0..200 {
        assert_eq!(inherit_speed(5.25, 0., 0.1, 7, id), (5.25, false));
        let (child, mutated) = inherit_speed(5.25, 0.999_999, 0.1, 7, id);
        assert!(mutated);
        assert!((5.25 * 0.9 - 1e-4..=5.25 * 1.1 + 1e-4).contains(&child));
        let (top, _) = inherit_speed(SPEED_MAX, 0.999_999, 0.5, 7, id);
        assert!(top <= SPEED_MAX);
        let (bottom, _) = inherit_speed(SPEED_MIN, 0.999_999, 0.5, 7, id);
        assert!(bottom >= SPEED_MIN);
        let start = initial_speed(5.25, 0., 7, id);
        assert_eq!(start, 5.25);
    }
}

#[test]
fn mutation_changes_speeds_over_generations() {
    let mut config = w_e();
    config.carnivores.speed_evolution_enabled = true;
    config.carnivores.speed_mutation_probability = 0.5;
    config.carnivores.speed_mutation_scale = 0.1;
    let mut world = World::new(config).unwrap();
    for _ in 0..8 {
        world.step(500).unwrap();
    }
    assert!(world
        .carnivores
        .iter()
        .any(|c| (c.speed - 5.25).abs() > 1e-3));
    assert!(world
        .carnivores
        .iter()
        .all(|c| (SPEED_MIN..=SPEED_MAX).contains(&c.speed)));
}

#[test]
fn invalid_speed_evolution_settings_are_rejected() {
    for (spread, probability, scale) in [(1., 0., 0.), (-0.1, 0., 0.), (0., 1.5, 0.), (0., 0., 1.)]
    {
        let mut config = w_e();
        config.carnivores.speed_evolution_enabled = true;
        config.carnivores.initial_speed_spread = spread;
        config.carnivores.speed_mutation_probability = probability;
        config.carnivores.speed_mutation_scale = scale;
        let Err(err) = World::new(config) else {
            panic!("should be rejected");
        };
        assert!(err.contains("carnivores.initial_speed_spread"), "{err}");
    }
}
