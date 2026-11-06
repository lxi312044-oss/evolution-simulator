//! Phase 06.2.6: paired-seed prey refuge evaluation.
//! cargo run --release -p sim-core --example refuge_batch_evaluation -- config/phase0626-off.json 50000 100 12345,12346 results/phase0626/off
use sim_core::{SimulationConfig, World};
use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};
fn escape(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = env::args().collect();
    if a.len() != 6 {
        return Err("usage: refuge_batch_evaluation <config.json> <ticks> <interval> <seeds_csv> <output_dir>".into());
    }
    let cfg: SimulationConfig = serde_json::from_str(&fs::read_to_string(&a[1])?)?;
    let target: u64 = a[2].parse()?;
    let interval: u64 = a[3].parse()?;
    if target == 0 || interval == 0 {
        return Err("ticks and interval must be > 0".into());
    }
    let seeds: Vec<u64> = a[4].split(',').map(str::parse).collect::<Result<_, _>>()?;
    if seeds.is_empty() {
        return Err("at least one seed required".into());
    }
    let dest = Path::new(&a[5]);
    fs::create_dir_all(dest)?;
    let mut out = BufWriter::new(File::create(dest.join("per_seed.csv"))?);
    writeln!(out,"seed,status,completed_tick,final_plants,final_herbivores,final_carnivores,plant_extinction_tick,herbivore_extinction_tick,carnivore_extinction_tick,herbivore_min,herbivore_max,carnivore_max,carnivore_births,carnivore_deaths,hunt_attempts,hunt_refuge_blocked,repro_trend_blocked,max_abs_nutrient_error,max_abs_energy_error,error")?;
    let mut finished = 0usize;
    let mut coexist = 0usize;
    let mut p_survive = 0usize;
    let mut h_survive = 0usize;
    let mut c_survive = 0usize;
    let mut blocked = 0u64;
    let mut max_n = 0f64;
    let mut max_e = 0f64;
    for seed in &seeds {
        let mut conf = cfg.clone();
        conf.seed = *seed;
        let mut w = World::new(conf)?;
        let (mut pe, mut he, mut ce) = (None, None, None);
        let (mut min_h, mut max_h, mut max_c) = (usize::MAX, 0usize, 0usize);
        let (mut err, mut this_max_n, mut this_max_e) = (None, 0f64, 0f64);
        loop {
            let p = w.plants.len();
            let h = w.animals.items.len();
            let c = w.carnivores.len();
            if p == 0 && pe.is_none() {
                pe = Some(w.tick)
            }
            if h == 0 && he.is_none() {
                he = Some(w.tick)
            }
            if c == 0 && ce.is_none() {
                ce = Some(w.tick)
            }
            min_h = min_h.min(h);
            max_h = max_h.max(h);
            max_c = max_c.max(c);
            let n = w.resources.total_nutrients() - w.baseline.nutrients;
            let e = w.resources.total_energy() - w.baseline.energy - w.ledger.sunlight_total()
                + w.ledger.heat_total();
            this_max_n = this_max_n.max(n.abs());
            this_max_e = this_max_e.max(e.abs());
            if w.tick >= target {
                break;
            }
            let steps = (target - w.tick)
                .min(interval)
                .min(w.config.simulation.max_steps_per_call as u64);
            if steps == 0 {
                err = Some("max_steps_per_call is zero".to_string());
                break;
            }
            let mut failed = false;
            for _ in 0..steps {
                if let Err(e) = w.step(1) {
                    err = Some(e);
                    failed = true;
                    break;
                }
            }
            if failed {
                break;
            }
        }
        let ok = err.is_none() && w.tick == target;
        if ok {
            finished += 1;
            coexist += usize::from(
                !w.plants.is_empty() && !w.animals.items.is_empty() && !w.carnivores.is_empty(),
            );
            p_survive += usize::from(!w.plants.items.is_empty());
            h_survive += usize::from(!w.animals.items.is_empty());
            c_survive += usize::from(!w.carnivores.is_empty());
        }
        blocked += w.carnivore_debug.hunt_refuge_blocked;
        max_n = max_n.max(this_max_n);
        max_e = max_e.max(this_max_e);
        let fmt = |v: Option<u64>| v.map(|x| x.to_string()).unwrap_or_default();
        writeln!(
            out,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.12},{:.12},{}",
            seed,
            if ok { "ok" } else { "error" },
            w.tick,
            w.plants.len(),
            w.animals.items.len(),
            w.carnivores.len(),
            fmt(pe),
            fmt(he),
            fmt(ce),
            min_h,
            max_h,
            max_c,
            w.carnivore_births,
            w.carnivore_deaths,
            w.carnivore_debug.hunt_attempts,
            w.carnivore_debug.hunt_refuge_blocked,
            w.carnivore_debug.reproduction_trend_blocked,
            this_max_n,
            this_max_e,
            escape(err.as_deref().unwrap_or(""))
        )?;
        eprintln!(
            "seed={} tick={} P/H/C={}/{}/{} refuge_blocked={} status={}",
            seed,
            w.tick,
            w.plants.len(),
            w.animals.items.len(),
            w.carnivores.len(),
            w.carnivore_debug.hunt_refuge_blocked,
            if ok { "ok" } else { "error" }
        );
    }
    out.flush()?;
    let mut summary = BufWriter::new(File::create(dest.join("summary.csv"))?);
    writeln!(summary,"requested_seeds,completed_seeds,failed_seeds,target_tick,three_species_survival_count,three_species_survival_rate,plant_survival_count,herbivore_survival_count,carnivore_survival_count,total_refuge_blocks,max_abs_nutrient_error,max_abs_energy_error")?;
    writeln!(
        summary,
        "{},{},{},{},{},{:.6},{},{},{},{},{:.12},{:.12}",
        seeds.len(),
        finished,
        seeds.len() - finished,
        target,
        coexist,
        if finished == 0 {
            0.
        } else {
            coexist as f64 / finished as f64
        },
        p_survive,
        h_survive,
        c_survive,
        blocked,
        max_n,
        max_e
    )?;
    summary.flush()?;
    if finished != seeds.len() {
        eprintln!(
            "WARNING: {} seeds failed; survival rate denominator is completed seeds",
            seeds.len() - finished
        )
    }
    Ok(())
}
