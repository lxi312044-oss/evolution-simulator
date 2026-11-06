//! Run: cargo run --release -p sim-core --example batch_evaluation -- config/phase055-longrun.json 50000 100 12345,12346,12347 results/phase055
use sim_core::{SimulationConfig, World};
use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

#[derive(Clone)]
struct Run {
    seed: u64,
    completed: u64,
    plants: u32,
    herbivores: u32,
    plant_extinction: Option<u64>,
    herbivore_extinction: Option<u64>,
    min_plants: u32,
    max_plants: u32,
    min_herbivores: u32,
    max_herbivores: u32,
    sum_plants: f64,
    sum_herbivores: f64,
    samples: u64,
    error: Option<String>,
}
fn csv_escape(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}
fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}
fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 6 {
        return Err("usage: batch_evaluation <config.json> <ticks> <sample_interval> <seed1,seed2,...> <output_dir>".into());
    }
    let config: SimulationConfig = serde_json::from_str(&fs::read_to_string(&args[1])?)?;
    let target: u64 = args[2].parse()?;
    let interval: u64 = args[3].parse()?;
    if interval == 0 || target == 0 {
        return Err("ticks and sample_interval must be positive".into());
    }
    let seeds: Vec<u64> = args[4]
        .split(',')
        .map(str::parse)
        .collect::<Result<_, _>>()?;
    if seeds.is_empty() {
        return Err("at least one seed required".into());
    }
    let out = Path::new(&args[5]);
    fs::create_dir_all(out)?;
    let mut series = BufWriter::new(File::create(out.join("timeseries.csv"))?);
    writeln!(
        series,
        "seed,tick,plants,herbivores,births,deaths,soil_n,detritus_n"
    )?;
    let mut runs = Vec::new();
    for seed in seeds {
        let mut cfg = config.clone();
        cfg.seed = seed;
        let mut world = World::new(cfg)?;
        let mut run = Run {
            seed,
            completed: 0,
            plants: world.plant_count(),
            herbivores: world.herbivore_count(),
            plant_extinction: None,
            herbivore_extinction: None,
            min_plants: u32::MAX,
            max_plants: 0,
            min_herbivores: u32::MAX,
            max_herbivores: 0,
            sum_plants: 0.,
            sum_herbivores: 0.,
            samples: 0,
            error: None,
        };
        loop {
            let p = world.plant_count();
            let h = world.herbivore_count();
            run.plants = p;
            run.herbivores = h;
            run.completed = world.tick;
            run.min_plants = run.min_plants.min(p);
            run.max_plants = run.max_plants.max(p);
            run.min_herbivores = run.min_herbivores.min(h);
            run.max_herbivores = run.max_herbivores.max(h);
            run.sum_plants += p as f64;
            run.sum_herbivores += h as f64;
            run.samples += 1;
            if p == 0 && run.plant_extinction.is_none() {
                run.plant_extinction = Some(world.tick);
            }
            if h == 0 && run.herbivore_extinction.is_none() {
                run.herbivore_extinction = Some(world.tick);
            }
            writeln!(
                series,
                "{},{},{},{},{},{},{:.9},{:.9}",
                seed,
                world.tick,
                p,
                h,
                world.births,
                world.deaths,
                world.total_soil(),
                world.total_detritus()
            )?;
            if world.tick >= target {
                break;
            }
            let steps = (target - world.tick)
                .min(interval)
                .min(world.config.simulation.max_steps_per_call as u64)
                as u32;
            // Run one tick at a time so a failed seed can be reported with its precise failing tick.
            let mut failed = false;
            for _ in 0..steps {
                if let Err(e) = world.step(1) {
                    run.error = Some(e);
                    failed = true;
                    break;
                }
            }
            if failed {
                break;
            }
        }
        eprintln!(
            "seed={} completed={} plants={} herbivores={} status={}",
            seed,
            run.completed,
            run.plants,
            run.herbivores,
            if run.error.is_some() { "ERROR" } else { "OK" }
        );
        runs.push(run);
    }
    series.flush()?;
    let mut details = BufWriter::new(File::create(out.join("per_seed.csv"))?);
    writeln!(details,"seed,completed_tick,final_plants,final_herbivores,plant_extinction_tick,herbivore_extinction_tick,min_plants,max_plants,min_herbivores,max_herbivores,mean_sampled_plants,mean_sampled_herbivores,status,error")?;
    for r in &runs {
        writeln!(
            details,
            "{},{},{},{},{},{},{},{},{},{},{:.3},{:.3},{},{}",
            r.seed,
            r.completed,
            r.plants,
            r.herbivores,
            r.plant_extinction
                .map(|v| v.to_string())
                .unwrap_or_default(),
            r.herbivore_extinction
                .map(|v| v.to_string())
                .unwrap_or_default(),
            r.min_plants,
            r.max_plants,
            r.min_herbivores,
            r.max_herbivores,
            r.sum_plants / r.samples as f64,
            r.sum_herbivores / r.samples as f64,
            if r.error.is_some() { "error" } else { "ok" },
            csv_escape(r.error.as_deref().unwrap_or(""))
        )?;
    }
    details.flush()?;
    let completed: Vec<&Run> = runs
        .iter()
        .filter(|r| r.error.is_none() && r.completed == target)
        .collect();
    let ps: Vec<f64> = completed.iter().map(|r| r.plants as f64).collect();
    let hs: Vec<f64> = completed.iter().map(|r| r.herbivores as f64).collect();
    let both = completed
        .iter()
        .filter(|r| r.plants > 0 && r.herbivores > 0)
        .count();
    let mut summary = BufWriter::new(File::create(out.join("summary.csv"))?);
    writeln!(summary,"requested_seeds,completed_seeds,failed_seeds,target_tick,both_survival_count,both_survival_rate,final_plants_mean,final_plants_median,final_plants_min,final_plants_max,final_herbivores_mean,final_herbivores_median,final_herbivores_min,final_herbivores_max")?;
    writeln!(
        summary,
        "{},{},{},{},{},{:.6},{:.3},{:.3},{:.0},{:.0},{:.3},{:.3},{:.0},{:.0}",
        runs.len(),
        completed.len(),
        runs.len() - completed.len(),
        target,
        both,
        if completed.is_empty() {
            0.
        } else {
            both as f64 / completed.len() as f64
        },
        mean(&ps),
        median(&ps),
        ps.iter().copied().reduce(f64::min).unwrap_or(0.),
        ps.iter().copied().reduce(f64::max).unwrap_or(0.),
        mean(&hs),
        median(&hs),
        hs.iter().copied().reduce(f64::min).unwrap_or(0.),
        hs.iter().copied().reduce(f64::max).unwrap_or(0.)
    )?;
    summary.flush()?;
    if completed.len() != runs.len() {
        eprintln!(
            "WARNING: {} seed(s) failed; failures are excluded from survival statistics",
            runs.len() - completed.len()
        );
    }
    Ok(())
}
