//! Issue #11: opt-in hunting bouts (rest after hunting at an energy loss).
use sim_core::{SimulationConfig, World};

/// One carnivore and one herbivore at a chosen distance.
fn pair(distance: f32, bout: u32) -> World {
    let mut cfg = SimulationConfig {
        seed: 777,
        ..Default::default()
    };
    cfg.initial_population.plants = 1;
    cfg.initial_population.herbivores = 1;
    cfg.initial_population.carnivores = 1;
    cfg.herbivores.reproduction_enabled = false;
    cfg.herbivores.predator_evasion_enabled = false;
    cfg.carnivores.enabled = true;
    cfg.carnivores.reproduction_enabled = false;
    cfg.carnivores.initial_energy = 30.;
    cfg.carnivores.vision = 20.;
    cfg.carnivores.attack_range = 10.;
    cfg.carnivores.speed = 1.;
    cfg.carnivores.movement_strategy = 0;
    cfg.carnivores.basal_energy_per_tick = 0.01;
    cfg.carnivores.movement_energy_per_unit = 0.01;
    cfg.carnivores.bite_energy = 2.;
    cfg.carnivores.attack_cooldown_ticks = 0;
    cfg.carnivores.satiety_enabled = false;
    cfg.carnivores.digestion_enabled = false;
    cfg.carnivores.all_bites_to_stomach = false;
    cfg.carnivores.prey_refuge_enabled = false;
    cfg.carnivores.hunt_bout_ticks = bout;
    let mut w = World::new(cfg).unwrap();
    let (x, y) = (w.animals.items[0].x, w.animals.items[0].y);
    w.animals.items[0].genes.speed = 0.;
    let width = w.config.world.width;
    w.carnivores[0].x = if x + distance <= width {
        x + distance
    } else {
        x - distance
    };
    w.carnivores[0].y = y;
    w
}

#[test]
fn default_is_off_and_not_serialized() {
    let text = serde_json::to_string(&SimulationConfig::default()).unwrap();
    assert!(!text.contains("hunt_bout_ticks"));
    let mut w = pair(200., 0);
    w.step(30).unwrap();
    assert!(w.carnivore_hunt_bouts.is_empty());
    assert_eq!(w.defense.hunt_bouts_completed, 0);
}

#[test]
fn losing_bout_makes_the_carnivore_rest_in_place() {
    // Prey far outside vision: hunting only costs energy.
    let mut w = pair(200., 10);
    w.step(10).unwrap();
    assert_eq!(w.defense.hunt_bouts_completed, 1);
    assert_eq!(w.defense.hunt_bouts_rested, 1);
    let before = (w.carnivores[0].x, w.carnivores[0].y);
    w.step(9).unwrap();
    assert_eq!(
        (w.carnivores[0].x, w.carnivores[0].y),
        before,
        "moved while resting"
    );
    w.step(3).unwrap();
    assert_ne!(
        (w.carnivores[0].x, w.carnivores[0].y),
        before,
        "did not resume hunting"
    );
}

#[test]
fn profitable_bout_does_not_rest() {
    // Prey within attack range: each bite gains more than a tick costs.
    let mut w = pair(3., 5);
    w.step(5).unwrap();
    assert!(w.carnivore_debug.hunt_contacts > 0);
    assert_eq!(w.defense.hunt_bouts_completed, 1);
    assert_eq!(w.defense.hunt_bouts_rested, 0);
}
