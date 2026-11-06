use sim_core::{SimulationConfig,World};
#[test]
fn adjustable_growth_and_feeding_keep_nutrients_conserved(){
    for (growth,uptake,seed,feeding) in [(0.003,0.1,0.35,1.5),(0.009,0.3,0.65,0.35),(0.02,0.6,0.8,0.15)] {
        let mut c=SimulationConfig::default();
        c.plants.growth_per_tick=growth;
        c.plants.nutrient_uptake_max=uptake;
        c.plants.seed_probability=seed;
        c.herbivores.feeding_nutrients_per_tick=feeding;
        c.validate().unwrap();
        let mut w=World::new(c).unwrap();
        let initial=w.resources.total_nutrients();
        w.step(1000).unwrap();
        assert!((w.resources.total_nutrients()-initial).abs()<1e-4);
    }
}
#[test]
fn invalid_growth_setting_is_rejected(){
    let mut c=SimulationConfig::default();
    c.plants.growth_per_tick=f32::NAN;
    assert!(c.validate().is_err());
}
