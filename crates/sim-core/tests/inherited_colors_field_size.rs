//! Issue #11: the inherited-color founder assignment works for any multiple of
//! 10 initial plants, so the stage 4 environment can be scaled to 40x40 at the
//! same density.
use sim_core::{SimulationConfig, World};

fn stage4() -> SimulationConfig {
    serde_json::from_str(include_str!(
        "../../../config/stage4-inherited-plant-colors.json"
    ))
    .unwrap()
}

/// Gn, Gt, Pn, Pt
fn classes(world: &World) -> [usize; 4] {
    let mut n = [0; 4];
    for p in &world.plants.items {
        let purple = p.color == 0xb86be3ff;
        n[usize::from(purple) * 2 + usize::from(p.produces_toxin)] += 1;
    }
    n
}

#[test]
fn founder_counts_scale_with_multiples_of_ten() {
    for plants in [300, 1200] {
        let mut config = stage4();
        config.initial_population.plants = plants;
        config.world.max_living_entities = 20_000;
        let world = World::new(config).unwrap();
        let block = (plants / 10) as usize;
        assert_eq!(classes(&world), [4 * block, block, block, 4 * block]);
    }
}

#[test]
fn founder_plant_counts_must_be_positive_multiples_of_ten() {
    for (plants, ok) in [
        (300, true),
        (1200, true),
        (10, true),
        (0, false),
        (299, false),
        (1205, false),
    ] {
        let mut config = stage4();
        config.initial_population.plants = plants;
        config.world.max_living_entities = 20_000;
        let result = config.validate();
        assert_eq!(result.is_ok(), ok, "plants={plants}");
        if !ok {
            assert!(result
                .unwrap_err()
                .contains("initial_population.plants を10の倍数"));
        }
    }
}
