use sim_core::{SimulationConfig,World};
#[test]
fn circulation_parameters_preserve_nutrients_and_energy(){
    for (mineralization,diffusion) in [(0.002,0.0),(0.012,0.08),(0.05,0.3)] {
        let mut c=SimulationConfig::default();
        c.plants.mineralization_rate=mineralization;
        c.plants.soil_diffusion_rate=diffusion;
        c.plants.growth_per_tick=0.009;
        c.plants.nutrient_uptake_max=0.3;
        c.plants.seed_probability=0.65;
        c.plants.seed_dispersal_radius=110.;
        c.herbivores.feeding_nutrients_per_tick=0.35;
        let mut w=World::new(c).unwrap();
        let initial_n=w.resources.total_nutrients();
        w.step(1000).unwrap();
        assert!((w.resources.total_nutrients()-initial_n).abs()<1e-4);
        assert!(w.diagnostics.mineralized_n>=0.);
        assert!(w.diagnostics.soil_diffused_n>=0.);
    }
}
#[test]
fn invalid_diffusion_is_rejected(){
    for rate in [-0.1,1.1,f64::NAN] {
        let mut c=SimulationConfig::default();c.plants.soil_diffusion_rate=rate;
        assert!(c.validate().is_err());
    }
}
#[test]
fn diffusion_is_deterministic(){
    let mut c=SimulationConfig::default();c.plants.soil_diffusion_rate=0.08;
    let mut a=World::new(c.clone()).unwrap();let mut b=World::new(c).unwrap();
    a.step(500).unwrap();b.step(500).unwrap();
    assert_eq!(a.plant_count(),b.plant_count());
    assert_eq!(a.total_soil(),b.total_soil());
}
