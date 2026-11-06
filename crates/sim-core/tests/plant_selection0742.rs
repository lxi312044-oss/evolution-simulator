use sim_core::{SimulationConfig, World};
#[test]
fn trait_demography_accounts_for_all_births_and_deaths() {
    let mut c = SimulationConfig::default();
    c.toxin.enabled = true;
    c.toxin.plant_variation_enabled = true;
    c.toxin.initial_toxic_plant_fraction = 0.5;
    let mut w = World::new(c).unwrap();
    for _ in 0..100 {
        w.step(1).unwrap();
    }
    let d = &w.diagnostics;
    assert_eq!(
        d.plant_births,
        d.toxic_plant_births + d.nontoxic_plant_births
    );
    assert_eq!(
        d.plant_age_deaths,
        d.toxic_plant_age_deaths + d.nontoxic_plant_age_deaths
    );
    assert_eq!(
        d.plant_consumed_deaths,
        d.toxic_plant_consumed_deaths + d.nontoxic_plant_consumed_deaths
    );
}
#[test]
fn trait_observations_do_not_change_seed_reproducibility() {
    let mut c = SimulationConfig::default();
    c.toxin.enabled = true;
    c.toxin.plant_variation_enabled = true;
    let mut a = World::new(c.clone()).unwrap();
    let mut b = World::new(c).unwrap();
    a.step(100).unwrap();
    b.step(100).unwrap();
    assert_eq!(
        a.diagnostics.toxic_plant_births,
        b.diagnostics.toxic_plant_births
    );
    assert_eq!(
        a.diagnostics.nontoxic_plant_births,
        b.diagnostics.nontoxic_plant_births
    );
    assert_eq!(
        a.plants
            .items
            .iter()
            .map(|p| p.produces_toxin)
            .collect::<Vec<_>>(),
        b.plants
            .items
            .iter()
            .map(|p| p.produces_toxin)
            .collect::<Vec<_>>()
    );
}
