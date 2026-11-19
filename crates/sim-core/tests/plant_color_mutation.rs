//! Issue #70: optional plant color mutation with inherited experimental colors.
use sim_core::{SimulationConfig, World};

const STANDARD: &str = include_str!("../../../config/stage4-standard.json");
const TRIAL: &str = include_str!("../../../config/stage4-standard-plant-color-mutation.json");

fn colors(w: &World) -> std::collections::BTreeSet<u32> {
    w.plants.items.iter().map(|p| p.color).collect()
}

#[test]
fn standard_now_includes_the_trial_plant_color_mutation() {
    assert_eq!(STANDARD, TRIAL);
}

#[test]
fn mutation_adds_plant_colors_and_zero_keeps_two() {
    let run = |mutation: bool| {
        let mut c: SimulationConfig = serde_json::from_str(STANDARD).unwrap();
        if !mutation {
            c.plants.color_mutation_probability = 0.;
            c.plants.color_mutation_scale = 0.;
        }
        let mut w = World::new(c).unwrap();
        for _ in 0..10 {
            w.step(100).unwrap();
        }
        colors(&w)
    };
    assert!(run(false).len() <= 2);
    assert!(run(true).len() > 10);
}
