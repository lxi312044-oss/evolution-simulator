//! Issue #54: opt-in herbivore counterattack and carnivore learning from injury.
use sim_core::{animals::color::BodyColor, SimulationConfig, World};

const STAGE4: &str = include_str!("../../../config/stage4-inherited-plant-colors.json");

/// One herbivore next to one hungry carnivore. `defending` sets genes that make it stand
/// ground; otherwise it flees.
fn duel_with(counterattack: f64, learning: bool, defending: bool) -> World {
    let mut cfg = SimulationConfig {
        seed: 12345,
        ..Default::default()
    };
    cfg.initial_population.plants = 1;
    cfg.initial_population.herbivores = 1;
    cfg.initial_population.carnivores = 1;
    cfg.herbivores.reproduction_enabled = false;
    cfg.herbivores.predator_evasion_enabled = true;
    cfg.herbivores.counterattack_energy = counterattack;
    cfg.herbivores.counterattack_learning_enabled = learning;
    cfg.carnivores.enabled = true;
    cfg.carnivores.reproduction_enabled = false;
    cfg.carnivores.initial_energy = 30.;
    cfg.carnivores.vision = 20.;
    cfg.carnivores.attack_range = 10.;
    cfg.carnivores.speed = 1.;
    cfg.carnivores.movement_strategy = 0;
    cfg.carnivores.basal_energy_per_tick = 0.;
    cfg.carnivores.movement_energy_per_unit = 0.;
    cfg.carnivores.bite_energy = 0.;
    cfg.carnivores.attack_cooldown_ticks = 0;
    cfg.carnivores.satiety_enabled = false;
    cfg.carnivores.digestion_enabled = false;
    cfg.carnivores.all_bites_to_stomach = false;
    cfg.carnivores.prey_refuge_enabled = false;
    cfg.animal_toxin.enabled = true;
    cfg.animal_toxin.color_learning_enabled = true;
    let mut w = World::new(cfg).unwrap();
    let h = &mut w.animals.items[0];
    h.genes.fear = if defending { 0. } else { 1. };
    h.genes.aggression = 1.;
    h.genes.adult_max_hp = 120.;
    h.hp = 120.;
    h.genes.speed = 0.5;
    h.genes.body_color = BodyColor::new(1., 0., 0.);
    let (x, y) = (h.x, h.y);
    w.carnivores[0].x = x + 3.;
    w.carnivores[0].y = y;
    w
}

fn duel(counterattack: f64, learning: bool) -> World {
    duel_with(counterattack, learning, true)
}

fn reserve_energy(w: &World) -> f64 {
    w.resources
        .energy_balance(w.carnivores[0].reserve)
        .unwrap_or(0.)
}

#[test]
fn defending_herbivore_drains_carnivore_energy() {
    let mut off = duel(0., true);
    let mut on = duel(2., true);
    off.step(5).unwrap();
    on.step(5).unwrap();
    assert_eq!(off.defense.counterattacks, 0);
    assert!(on.defense.counterattacks > 0, "no counterattack happened");
    // strength = aggression 1 x (120 / 60) = 2, so each counterattack drains 2 x 2 = 4 E.
    let expected = 4. * on.defense.counterattacks as f64;
    assert!((on.defense.counterattack_energy - expected).abs() < 1e-9);
    assert!((reserve_energy(&off) - reserve_energy(&on) - expected).abs() < 1e-6);
}

#[test]
fn fleeing_herbivore_also_fights_back_when_bitten() {
    let mut w = duel_with(2., true, false);
    let mut bitten = false;
    for _ in 0..20 {
        let hunts = w.carnivore_debug.hunt_contacts;
        w.step(1).unwrap();
        bitten |= w.carnivore_debug.hunt_contacts > hunts;
        assert!(!w.animals.items.first().is_some_and(|h| h.defense_active));
    }
    assert!(bitten, "the slow fleeing herbivore was never bitten");
    assert!(w.defense.counterattacks > 0);
}

#[test]
fn counterattack_teaches_avoidance_only_with_learning() {
    let mut learned = duel(2., true);
    let mut plain = duel(2., false);
    learned.step(5).unwrap();
    plain.step(5).unwrap();
    assert!(learned.defense.counterattack_learning_events > 0);
    assert_eq!(plain.defense.counterattack_learning_events, 0);
    let skips = |w: &World| -> u64 {
        let v: serde_json::Value = serde_json::from_str(&w.animal_toxin_snapshot_json()).unwrap();
        v["color_skips"].as_u64().unwrap()
    };
    learned.step(20).unwrap();
    plain.step(20).unwrap();
    assert!(
        skips(&learned) > 0,
        "learned carnivore never skipped the red prey"
    );
    assert_eq!(skips(&plain), 0);
}

#[test]
fn zero_counterattack_matches_previous_behavior_and_is_not_serialized() {
    let text = serde_json::to_string(&SimulationConfig::default()).unwrap();
    assert!(!text.contains("counterattack"));
    let base: SimulationConfig = serde_json::from_str(STAGE4).unwrap();
    let mut explicit = base.clone();
    explicit.herbivores.counterattack_energy = 0.;
    explicit.herbivores.counterattack_learning_enabled = false;
    let run = |c: SimulationConfig| {
        let mut w = World::new(c).unwrap();
        (0..6)
            .map(|_| {
                w.step(500).unwrap();
                (
                    w.carnivores.len(),
                    w.herbivore_count(),
                    w.plants.items.len(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(base), run(explicit));
}

#[test]
fn invalid_counterattack_is_rejected() {
    for value in [-1., 101., f64::NAN] {
        let mut cfg: SimulationConfig = serde_json::from_str(STAGE4).unwrap();
        cfg.herbivores.counterattack_energy = value;
        let Err(err) = World::new(cfg) else {
            panic!("should be rejected");
        };
        assert!(err.contains("counterattack_energy"), "{err}");
    }
}

#[test]
fn herbivores_below_the_strength_threshold_do_not_fight_back() {
    // The duel herbivore has strength 1 x (120 / 60) = 2.
    let mut weak = duel(2., true);
    weak.config.herbivores.counterattack_strength_threshold = 2.5;
    let mut strong = duel(2., true);
    strong.config.herbivores.counterattack_strength_threshold = 1.5;
    weak.step(5).unwrap();
    strong.step(5).unwrap();
    assert_eq!(weak.defense.counterattacks, 0);
    assert!(strong.defense.counterattacks > 0);
}

#[test]
fn carnivore_color_memory_decay_reduces_avoidance() {
    let skips = |decay: f64| -> u64 {
        let mut w = duel(2., true);
        w.config.carnivores.color_memory_decay_per_tick = decay;
        w.step(25).unwrap();
        let v: serde_json::Value = serde_json::from_str(&w.animal_toxin_snapshot_json()).unwrap();
        v["color_skips"].as_u64().unwrap()
    };
    let kept = skips(0.);
    let fading = skips(0.9);
    assert!(kept > 0);
    assert!(
        fading < kept,
        "decay did not reduce avoidance: {fading} vs {kept}"
    );
}

#[test]
fn new_settings_default_to_previous_behavior() {
    let text = serde_json::to_string(&SimulationConfig::default()).unwrap();
    assert!(!text.contains("counterattack_strength_threshold"));
    assert!(!text.contains("\"color_memory_decay_per_tick\":0"));
    let mut cfg: SimulationConfig = serde_json::from_str(STAGE4).unwrap();
    cfg.carnivores.color_memory_decay_per_tick = 1.5;
    let Err(err) = World::new(cfg) else {
        panic!("should be rejected");
    };
    assert!(err.contains("color_memory_decay_per_tick"), "{err}");
}

#[test]
fn carnivore_breaks_off_from_a_prey_that_fought_back() {
    let mut w = duel(2., false);
    w.config.herbivores.counterattack_retreat_ticks = 50;
    let prey = w.animals.items[0].id;
    let predator = w.carnivores[0].id;
    let mut ticks = 0;
    while w.defense.counterattacks == 0 {
        w.step(1).unwrap();
        ticks += 1;
        assert!(ticks < 20, "no counterattack happened");
    }
    assert_eq!(w.defense.counterattack_retreats, 1);
    assert_eq!(w.carnivore_retreats.get(&predator).map(|r| r.0), Some(prey));
    // While retreating, the carnivore does not bite that prey again.
    w.step(45).unwrap();
    assert_eq!(w.defense.counterattacks, 1);
    // The retreat expires: either the entry is gone, or the carnivore attacked the prey
    // again and a new retreat started.
    w.step(10).unwrap();
    assert!(!w.carnivore_retreats.contains_key(&predator) || w.defense.counterattacks > 1);
    assert!(w.carnivore_retreats.get(&predator).is_none_or(|r| r.1 > 40));
}

#[test]
fn retreat_defaults_to_off() {
    let text = serde_json::to_string(&SimulationConfig::default()).unwrap();
    assert!(!text.contains("counterattack_retreat_ticks"));
    let mut w = duel(2., false);
    w.step(20).unwrap();
    assert!(w.defense.counterattacks > 1);
    assert!(w.carnivore_retreats.is_empty());
}
