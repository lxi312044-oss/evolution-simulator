use sim_core::{world::SpontaneousGermination, SimulationConfig, World};
fn setup(soil: f64) -> World {
    let mut c = SimulationConfig::default();
    c.initial_population.plants = 0;
    c.initial_population.herbivores = 0;
    c.initial_population.carnivores = 0;
    c.world.land_columns = 1;
    c.world.land_rows = 1;
    c.plants.initial_soil_nutrients_per_cell = soil;
    c.plants.initial_detritus_per_cell = 0.;
    let mut w = World::new(c).unwrap();
    w.spontaneous_germination = Some(SpontaneousGermination {
        soil_threshold: 20.,
        probability_per_cell: 1.,
        check_interval: 20,
        max_births_per_check: 1,
    });
    w
}
#[test]
fn zero_plants_recover_without_a_parent_and_preserve_accounts() {
    let mut w = setup(30.);
    let n = w.resources.total_nutrients();
    let e = w.resources.total_energy();
    w.step(19).unwrap();
    assert!(w.plants.is_empty());
    w.step(1).unwrap();
    assert_eq!(w.plants.len(), 1);
    assert_eq!(w.spontaneous_births, 1);
    assert_eq!(
        w.resources
            .nutrient_balance(w.plants.items[0].body)
            .unwrap(),
        w.config.plants.initial_body_nutrients
    );
    assert_eq!(
        w.resources.nutrient_balance(w.land.cells[0].soil).unwrap(),
        30. - w.config.plants.initial_body_nutrients
    );
    assert!((w.resources.total_nutrients() - n).abs() < 1e-10);
    assert!(
        (w.resources.total_energy() - e - w.ledger.sunlight_total() + w.ledger.heat_total()).abs()
            < 1e-10
    );
    assert_eq!(w.ledger.sunlight_total(), w.config.plants.initial_energy);
}
#[test]
fn disabled_or_insufficient_soil_or_zero_probability_prevents_recovery() {
    for mode in 0..3 {
        let mut w = setup(if mode == 1 { 19. } else { 30. });
        if mode == 0 {
            w.spontaneous_germination = None;
        }
        if mode == 2 {
            w.spontaneous_germination
                .as_mut()
                .unwrap()
                .probability_per_cell = 0.;
        }
        w.step(40).unwrap();
        assert!(w.plants.is_empty());
        assert_eq!(w.spontaneous_births, 0);
    }
}
#[test]
fn full_entity_capacity_prevents_germination() {
    let mut w = setup(30.);
    w.config.world.max_living_entities = 0;
    w.step(20).unwrap();
    assert!(w.plants.is_empty());
}
#[test]
fn body_n_requirement_can_exceed_soil_threshold() {
    let mut w = setup(30.);
    w.config.plants.initial_body_nutrients = 31.;
    w.step(20).unwrap();
    assert!(w.plants.is_empty());
}

#[test]
fn historical_germination_parameters_recover_from_zero() {
    let mut c = SimulationConfig {
        seed: 34507,
        ..Default::default()
    };
    c.initial_population.plants = 0;
    c.initial_population.herbivores = 0;
    c.initial_population.carnivores = 0;
    let mut w = World::new(c).unwrap();
    let g = SpontaneousGermination {
        soil_threshold: 20.,
        probability_per_cell: 0.02,
        check_interval: 20,
        max_births_per_check: 20,
    };
    g.validate().unwrap();
    w.spontaneous_germination = Some(g);
    w.step(19).unwrap();
    assert_eq!(w.plants.len(), 0);
    w.step(1).unwrap();
    assert!(w.spontaneous_births > 0);
    assert_eq!(w.plants.len() as u64, w.spontaneous_births);
}
