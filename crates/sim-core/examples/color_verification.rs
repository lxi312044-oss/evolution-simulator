//! Run: cargo run --release -p sim-core --example color_verification -- config/phase055-longrun.json 50000 100 12345 results/color0611 true
use sim_core::{SimulationConfig, World};
use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 7 {
        return Err("usage: color_verification <config.json> <ticks> <interval> <seed> <output_dir> <events:true|false>".into());
    }
    let mut cfg: SimulationConfig = serde_json::from_str(&fs::read_to_string(&args[1])?)?;
    let target: u64 = args[2].parse()?;
    let interval: u64 = args[3].parse()?;
    cfg.seed = args[4].parse()?;
    if target == 0 || interval == 0 {
        return Err("ticks and interval must be positive".into());
    }
    let out = Path::new(&args[5]);
    fs::create_dir_all(out)?;
    let detailed: bool = args[6].parse()?;
    let mut world = World::new(cfg)?;
    world.set_color_event_logging(detailed);
    let mut stats = BufWriter::new(File::create(out.join("color_statistics.csv"))?);
    writeln!(stats,"seed,tick,herbivores,mean_r,mean_g,mean_b,var_r,var_g,var_b,color_inheritances,color_mutated_births,color_mutated_channels,herbivore_births,herbivore_mutations,total_n,total_e")?;
    let mut events = if detailed {
        Some(BufWriter::new(File::create(
            out.join("color_mutation_events.csv"),
        )?))
    } else {
        None
    };
    if let Some(f) = events.as_mut() {
        writeln!(f,"seed,tick,parent_index,parent_generation,child_index,child_generation,parent_r,parent_g,parent_b,child_r,child_g,child_b,mutated_r,mutated_g,mutated_b")?;
    }
    loop {
        let c = world.color_statistics();
        writeln!(
            stats,
            "{},{},{},{:.8},{:.8},{:.8},{:.10},{:.10},{:.10},{},{},{},{},{},{:.9},{:.9}",
            world.config.seed,
            world.tick,
            c.population,
            c.mean[0],
            c.mean[1],
            c.mean[2],
            c.variance[0],
            c.variance[1],
            c.variance[2],
            c.inheritances,
            c.mutated_births,
            c.mutated_channels,
            world.diagnostics.herbivore_births,
            world.diagnostics.herbivore_mutations,
            world.resources.total_nutrients(),
            world.resources.total_energy()
        )?;
        if let Some(f) = events.as_mut() {
            for e in world.take_color_events() {
                writeln!(
                    f,
                    "{},{},{},{},{},{},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{},{},{}",
                    world.config.seed,
                    e.tick,
                    e.parent_id.index,
                    e.parent_id.generation,
                    e.child_id.index,
                    e.child_id.generation,
                    e.parent_rgb[0],
                    e.parent_rgb[1],
                    e.parent_rgb[2],
                    e.child_rgb[0],
                    e.child_rgb[1],
                    e.child_rgb[2],
                    e.mutated[0] as u8,
                    e.mutated[1] as u8,
                    e.mutated[2] as u8
                )?;
            }
        }
        if world.tick >= target {
            break;
        }
        let steps = (target - world.tick)
            .min(interval)
            .min(world.config.simulation.max_steps_per_call as u64);
        for _ in 0..steps {
            world
                .step(1)
                .map_err(|e| format!("failed at tick {}: {e}", world.tick))?;
        }
    }
    stats.flush()?;
    if let Some(mut f) = events {
        f.flush()?;
    }
    eprintln!(
        "completed tick={} births={} color_mutated_births={} color_mutated_channels={}",
        world.tick,
        world.diagnostics.color_inheritances,
        world.diagnostics.color_mutated_births,
        world.diagnostics.color_mutated_channels
    );
    Ok(())
}
