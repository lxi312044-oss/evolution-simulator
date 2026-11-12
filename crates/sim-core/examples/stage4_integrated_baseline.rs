//! Fixed-window Stage 4 baseline. The only override is the prescribed seed.
use serde_json::{json, Value};
use sim_core::{SimulationConfig, World};
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};

fn colors(values: impl Iterator<Item = [f32; 3]>) -> Value {
    let values: Vec<_> = values.collect();
    let distinct = values
        .iter()
        .map(|rgb| rgb.map(f32::to_bits))
        .collect::<BTreeSet<_>>();
    let channels: Vec<_> = (0..3)
        .map(|i| {
            let sum: f64 = values.iter().map(|rgb| f64::from(rgb[i])).sum();
            let mean = sum / values.len() as f64;
            let variance = values
                .iter()
                .map(|rgb| (f64::from(rgb[i]) - mean).powi(2))
                .sum::<f64>()
                / values.len() as f64;
            json!({"min":values.iter().map(|rgb| rgb[i]).reduce(f32::min),
               "max":values.iter().map(|rgb| rgb[i]).reduce(f32::max),
               "mean":mean,"variance":variance})
        })
        .collect();
    json!({"count":values.len(),"distinct":distinct.len(),"channels_rgb":channels})
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let args: Vec<_> = std::env::args().collect();
    let mut config: SimulationConfig = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    config.seed = args[2].parse()?;
    assert!((12345..=12347).contains(&config.seed));
    let opportunities = args.get(3).is_some_and(|arg| arg == "opportunities");
    let mut shadow = if opportunities {
        Some(World::new(config.clone())?)
    } else {
        None
    };
    let mut world = World::new(config)?;
    if opportunities {
        assert!(world
            .animals
            .items
            .iter()
            .all(|a| a.toxin_load == 0. && a.typed_load.iter().all(|&v| v == 0.)));
        world.enable_carnivore_opportunity_trace();
    }
    let trace_enabled = args.get(3).is_some_and(|arg| arg == "trace");
    if !opportunities {
        world.set_carnivore_toxin_trace(trace_enabled);
    }
    let mut events = Vec::new();
    let initial_colors = json!({
        "plant": colors(world.plants.items.iter().map(|p| [24,16,8].map(|shift| ((p.color >> shift) & 255) as f32 / 255.))),
        "herbivore": colors(world.animals.items.iter().map(|a| a.genes.body_color.rgb())),
        "carnivore": colors(world.carnivores.iter().map(|a| a.color.rgb())),
    });
    let (n0, e0) = (
        world.resources.total_nutrients(),
        world.resources.total_energy(),
    );
    let mut max_n: f64 = 0.;
    let mut max_e: f64 = 0.;
    let mut max_toxin: f64 = 0.;
    let mut max_total = 0;
    let mut extinction = [None; 3];
    let mut rows = Vec::new();
    for tick in 0..=6000 {
        if start.elapsed() >= Duration::from_secs(120) {
            return Err(format!("time limit at Tick {tick}").into());
        }
        if tick > 0 {
            world.step(1)?;
            if let Some(shadow) = &mut shadow {
                shadow.step(1)?;
                assert!(
                    shadow.plants.len() + shadow.animals.items.len() + shadow.carnivores.len()
                        <= shadow.config.world.max_living_entities as usize
                );
            }
        }
        let counts = [
            world.plants.len(),
            world.animals.items.len(),
            world.carnivores.len(),
        ];
        let total = counts.iter().sum::<usize>();
        assert!(
            total <= world.config.world.max_living_entities as usize,
            "capacity at Tick {tick}"
        );
        max_total = max_total.max(total);
        for (species, count) in counts.iter().enumerate() {
            if *count == 0 && extinction[species].is_none() {
                extinction[species] = Some(tick);
            }
        }
        let n = world.resources.total_nutrients() - n0;
        let e = world.resources.total_energy() - e0 - world.ledger.sunlight_total()
            + world.ledger.heat_total();
        assert!(n.is_finite() && e.is_finite(), "nonfinite at Tick {tick}");
        max_n = max_n.max(n.abs());
        max_e = max_e.max(e.abs());
        if tick % 500 == 0 {
            #[cfg(feature = "state-audit")]
            if let Some(shadow) = &shadow {
                assert!(
                    world.audit_without_carnivore_trace() == shadow.audit_state(false, None),
                    "observation divergence at Tick {tick}"
                );
            }
            let toxin: Value = serde_json::from_str(&world.animal_toxin_snapshot_json())?;
            for error in toxin["balance_error"].as_array().unwrap() {
                let error = error.as_f64().ok_or("nonfinite toxin balance")?.abs();
                assert!(error < 0.001, "toxin balance at Tick {tick}");
                max_toxin = max_toxin.max(error);
            }
            events.extend(world.take_carnivore_toxin_trace());
            let mut reserves: Vec<_> = world
                .carnivores
                .iter()
                .map(|predator| world.resources.energy_balance(predator.reserve).unwrap())
                .collect();
            reserves.sort_by(f64::total_cmp);
            let a = world
                .plants
                .items
                .iter()
                .filter(|p| p.toxin_type.0 == 0)
                .count();
            rows.push(json!({"tick":tick,"counts":counts,"plant_types":[a,counts[0]-a],
                "animal_toxin":toxin,"diagnostics":world.diagnostics,"carnivore":world.carnivore_debug,
                "carnivore_births":world.carnivore_births,"carnivore_deaths":world.carnivore_deaths,
                "carnivore_reserve_e":reserves,"rng":format!("{:?}",world.rng)}));
        }
    }
    world.set_carnivore_toxin_trace(false);
    #[cfg(feature = "state-audit")]
    let terminal_state = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        world.audit_state(false, None).hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    };
    #[cfg(not(feature = "state-audit"))]
    let terminal_state = "";
    println!(
        "{}",
        json!({"config":world.config,"initial_colors":initial_colors,"first_zero":extinction,
        "max_n_error":max_n,"max_e_error":max_e,"max_sampled_toxin_error":max_toxin,
        "max_population":max_total,"seconds":start.elapsed().as_secs_f64(),"rows":rows,
        "trace":events,"terminal_state_hash":terminal_state})
    );
    Ok(())
}
