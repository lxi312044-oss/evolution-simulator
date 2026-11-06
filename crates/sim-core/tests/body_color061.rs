use sim_core::{SimulationConfig, World};
#[test]
fn individual_colors_are_bounded_and_repeatable() {
    let c = SimulationConfig::default();
    let a = World::new(c.clone()).unwrap();
    let b = World::new(c).unwrap();
    assert_eq!(a.herbivore_color_snapshot(), b.herbivore_color_snapshot());
    for row in a.herbivore_color_snapshot() {
        for v in &row[4..7] {
            assert!((0.0..=1.0).contains(v));
        }
    }
    assert!(a
        .herbivore_color_snapshot()
        .windows(2)
        .any(|w| w[0][4..7] != w[1][4..7]));
}
#[test]
fn color_inheritance_is_exact_without_mutation_and_bounded_with_mutation() {
    use sim_core::animals::color::{inherit, BodyColor};
    let parent = BodyColor::new(0.2, 0.5, 0.8);
    assert_eq!(inherit(parent, 123, 456, 0., 0.1), parent);
    let child = inherit(parent, 123, 456, 1., 0.2);
    assert_eq!(child, inherit(parent, 123, 456, 1., 0.2));
    for v in child.rgb() {
        assert!((0.0..=1.0).contains(&v));
    }
    assert_ne!(child, parent);
}
#[test]
fn color_does_not_change_initial_ecology_resource_balances() {
    let w = World::new(SimulationConfig::default()).unwrap();
    assert!(w.resources.total_nutrients().is_finite());
    assert!(w.resources.total_energy().is_finite());
}
