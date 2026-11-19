//! Issue #11: carnivores born from herbivore offspring (simple version).
use sim_core::{SimulationConfig, World};

const STAGE4: &str = include_str!("../../../config/stage4-inherited-plant-colors.json");

fn config(probability: f64, initial_carnivores: u32) -> SimulationConfig {
    let mut c: SimulationConfig = serde_json::from_str(STAGE4).unwrap();
    c.seed = 90001;
    c.initial_population.carnivores = initial_carnivores;
    c.herbivores.carnivore_offspring_probability = probability;
    c
}

#[test]
fn default_is_off_not_serialized_and_still_requires_initial_carnivores() {
    let text = serde_json::to_string(&SimulationConfig::default()).unwrap();
    assert!(!text.contains("carnivore_offspring_probability"));
    let Err(err) = World::new(config(0., 0)) else {
        panic!("zero carnivores without emergence should be rejected");
    };
    assert!(err.contains("nonzero initial population"), "{err}");
    assert!(World::new(config(0.01, 0)).is_ok());
    let Err(err) = World::new(config(1.5, 0)) else {
        panic!("probability above 1 should be rejected");
    };
    assert!(err.contains("carnivore_offspring_probability"), "{err}");
}

#[test]
fn every_offspring_becomes_a_carnivore_with_the_inherited_color() {
    let mut w = World::new(config(1., 0)).unwrap();
    let parent_colors: Vec<[f32; 3]> = w
        .animals
        .items
        .iter()
        .map(|h| h.genes.body_color.rgb())
        .collect();
    // Step checks N/E conservation every tick and errors on a violation.
    for _ in 0..6 {
        w.step(500).unwrap();
    }
    assert!(w.carnivore_births > 0);
    assert_eq!(w.diagnostics.herbivore_births, 0);
    assert_eq!(
        w.defense.carnivores_born_from_herbivores,
        w.carnivore_births
    );
    // Colors come from herbivore parents (initial herbivores, possibly mutated slightly).
    let near = |a: [f32; 3]| {
        parent_colors
            .iter()
            .any(|p| p.iter().zip(a).all(|(x, y)| (x - y).abs() <= 0.11))
    };
    assert!(w.carnivores.iter().all(|c| near(c.color.rgb())));
}

#[test]
fn zero_probability_matches_previous_behavior() {
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
    let base: SimulationConfig = serde_json::from_str(STAGE4).unwrap();
    let mut explicit = base.clone();
    explicit.herbivores.carnivore_offspring_probability = 0.;
    assert_eq!(run(base), run(explicit));
}

/// The check-only Web config (Issue 11 tuning, not adopted as standard) must load and run.
#[test]
fn check_only_web_config_loads_and_runs() {
    let text = include_str!("../../../config/stage4-emergence-check-40x40.json");
    let c: SimulationConfig = serde_json::from_str(text).unwrap();
    assert_eq!(c.initial_population.carnivores, 0);
    assert!(c.herbivores.carnivore_offspring_probability > 0.);
    let mut w = World::new(c).unwrap();
    w.step(200).unwrap();
    assert!(w.herbivore_count() > 0);
}

/// Issue #66/#67: the species observer reports living individuals and focusable events.
#[test]
fn species_observer_reports_ranking_and_focusable_events() {
    let mut w = World::new(config(0.01, 0)).unwrap();
    for _ in 0..30 {
        w.step(100).unwrap();
    }
    let summary: serde_json::Value = serde_json::from_str(&w.species_summary_json()).unwrap();
    let herbivores = &summary["kinds"]["herbivore"];
    let counted: u64 = herbivores["species"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["count"].as_u64().unwrap())
        .sum();
    assert_eq!(counted, herbivores["total"].as_u64().unwrap());
    let events: serde_json::Value = serde_json::from_str(&w.take_events_json()).unwrap();
    let events = events.as_array().unwrap();
    assert!(events
        .iter()
        .any(|e| e["code"] == "carnivore_from_herbivore"));
    for e in events {
        assert!(e["message"].as_str().is_some_and(|m| !m.is_empty()));
    }
    let top = herbivores["species"][0]["id"].as_u64().unwrap() as u32;
    let first: serde_json::Value = serde_json::from_str(&w.species_member_json(top, 0)).unwrap();
    let id = sim_core::entity::EntityId {
        index: first[0].as_u64().unwrap() as u32,
        generation: first[1].as_u64().unwrap() as u32,
    };
    assert_eq!(
        w.entity_species_name(id).as_deref(),
        herbivores["species"][0]["name"].as_str()
    );
    let count = w.species_members(top).len() as u32;
    assert_eq!(
        w.species_member_json(top, count),
        w.species_member_json(top, 0)
    );
    assert_eq!(w.species_member_json(u32::MAX, 0), "null");
    let again: serde_json::Value = serde_json::from_str(&w.take_events_json()).unwrap();
    assert!(again.as_array().unwrap().is_empty());
}

/// The current standard (decided 2026-11-19) is the w_p002_b120 check config.
#[test]
fn standard_config_matches_the_w_p002_b120_check_config() {
    let standard = include_str!("../../../config/stage4-standard.json");
    let check = include_str!("../../../config/stage4-emergence-check-40x40.json");
    assert_eq!(standard, check);
    let c: SimulationConfig = serde_json::from_str(standard).unwrap();
    assert_eq!(c.herbivores.carnivore_offspring_probability, 0.002);
    assert_eq!(c.initial_population.carnivores, 0);
}
