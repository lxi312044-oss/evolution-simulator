use sim_core::{
    entity::EntityId,
    world::{ForagingTrace, ForagingWindow},
};
#[test]
fn observation_requires_both_exact_id_and_inclusive_tick_window() {
    let t = ForagingTrace::new(vec![ForagingWindow {
        id: [3, 2],
        start: 10,
        end: 12,
    }]);
    for tick in [10, 11, 12] {
        assert!(t.observes(
            EntityId {
                index: 3,
                generation: 2
            },
            tick
        ));
    }
    for tick in [9, 13] {
        assert!(!t.observes(
            EntityId {
                index: 3,
                generation: 2
            },
            tick
        ));
    }
    assert!(!t.observes(
        EntityId {
            index: 3,
            generation: 1
        },
        11
    ));
}

#[test]
fn plant_protection_blocks_food_even_when_resources_are_visible() {
    let mut c = sim_core::SimulationConfig::default();
    c.initial_population.plants = 1;
    c.initial_population.herbivores = 1;
    c.initial_population.carnivores = 0;
    c.herbivores.plant_protection_enabled = true;
    c.herbivores.plant_protection_threshold = 300;
    c.herbivores.satiety_enabled = false;
    c.herbivores.reproduction_enabled = false;
    let mut w = sim_core::World::new(c).unwrap();
    let a = &mut w.animals.items[0];
    a.x = 50.;
    a.y = 50.;
    a.genes.speed = 0.;
    w.plants.items[0].x = 50.;
    w.plants.items[0].y = 50.;
    let id = [a.id.index, a.id.generation];
    w.foraging_trace = Some(ForagingTrace::new(vec![ForagingWindow {
        id,
        start: 1,
        end: 1,
    }]));
    w.step(1).unwrap();
    let e = &w.foraging_trace.as_ref().unwrap().events[0];
    assert_eq!(e["plant_protected"], true);
    assert_eq!(e["satiety_blocked"], false);
    assert_eq!(e["visible_ne"], 1);
    assert_eq!(e["may_feed"], false);
    assert_eq!(e["feeding"]["attempted"], false);
    assert_eq!(e["feeding"]["gained_e"], 0.);
}
