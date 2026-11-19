//! Issue #70: optional plant color mutation with inherited experimental colors.
use sim_core::{SimulationConfig, World};

const STANDARD: &str = include_str!("../../../config/stage4-standard.json");
const TRIAL: &str = include_str!("../../../config/stage4-standard-plant-color-mutation.json");

fn colors(w: &World) -> std::collections::BTreeSet<u32> {
    w.plants.items.iter().map(|p| p.color).collect()
}

#[test]
fn trial_config_differs_from_the_standard_only_in_plant_color_mutation() {
    let mut trial: serde_json::Value = serde_json::from_str(TRIAL).unwrap();
    let standard: serde_json::Value = serde_json::from_str(STANDARD).unwrap();
    assert_eq!(trial["plants"]["color_mutation_probability"], 0.2);
    assert_eq!(trial["plants"]["color_mutation_scale"], 0.25);
    trial["plants"]["color_mutation_probability"] =
        standard["plants"]["color_mutation_probability"].clone();
    trial["plants"]["color_mutation_scale"] = standard["plants"]["color_mutation_scale"].clone();
    let strip = |mut v: serde_json::Value| {
        for k in ["color_mutation_probability", "color_mutation_scale"] {
            v["plants"].as_object_mut().unwrap().remove(k);
        }
        v
    };
    assert_eq!(strip(trial), strip(standard));
}

#[test]
fn mutation_adds_plant_colors_and_zero_keeps_two() {
    let run = |text: &str| {
        let c: SimulationConfig = serde_json::from_str(text).unwrap();
        let mut w = World::new(c).unwrap();
        for _ in 0..10 {
            w.step(100).unwrap();
        }
        colors(&w)
    };
    assert!(run(STANDARD).len() <= 2);
    assert!(run(TRIAL).len() > 10);
}
