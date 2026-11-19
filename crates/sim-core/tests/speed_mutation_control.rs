//! Issue #11: experimental control that stops herbivore speed mutation.
use sim_core::{SimulationConfig, World};

const STAGE4: &str = include_str!("../../../config/stage4-inherited-plant-colors.json");

fn fixed_speed(mutation: bool) -> World {
    let mut c: SimulationConfig = serde_json::from_str(STAGE4).unwrap();
    c.seed = 90001;
    c.herbivores.speed_min = 2.5;
    c.herbivores.speed_max = 2.5;
    c.herbivores.mutation_probability = 1.;
    c.herbivores.speed_mutation_enabled = mutation;
    let mut w = World::new(c).unwrap();
    for _ in 0..6 {
        w.step(500).unwrap();
    }
    w
}

#[test]
fn default_is_on_and_not_serialized() {
    let c = SimulationConfig::default();
    assert!(c.herbivores.speed_mutation_enabled);
    let text = serde_json::to_string(&c).unwrap();
    assert!(!text.contains("speed_mutation_enabled"));
}

#[test]
fn off_keeps_every_speed_at_the_initial_value() {
    let w = fixed_speed(false);
    assert!(w.diagnostics.herbivore_births > 0);
    assert!(w.animals.items.iter().all(|h| h.genes.speed == 2.5));
    // Other genes still mutate.
    assert!(w.diagnostics.herbivore_mutations > 0);
}

#[test]
fn on_lets_speed_mutate() {
    let w = fixed_speed(true);
    assert!(w.diagnostics.herbivore_births > 0);
    assert!(w.animals.items.iter().any(|h| h.genes.speed != 2.5));
}
