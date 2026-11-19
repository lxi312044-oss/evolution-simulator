//! Observation-only long-run resistance study using the shared Native/WASM logger.
use sim_core::{
    lineage::LineageObservation, world::SpontaneousGermination, SimulationConfig, World,
};
use std::io::Write;
#[cfg(test)]
fn bin(r: f32) -> usize {
    if r >= 0.6 {
        2
    } else if r <= 0.4 {
        0
    } else {
        1
    }
}
fn germination() -> SpontaneousGermination {
    SpontaneousGermination {
        soil_threshold: 20.,
        probability_per_cell: 0.005,
        check_interval: 20,
        max_births_per_check: 20,
    }
}
fn drain(o: &mut LineageObservation, out: &mut dyn Write) -> Result<(), String> {
    for row in o.take_events() {
        serde_json::to_writer(&mut *out, &row).map_err(|e| e.to_string())?;
        out.write_all(b"\n").map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn run(config: SimulationConfig, ticks: u64, out: &mut dyn Write) -> Result<World, String> {
    let mut w = World::new(config)?;
    w.spontaneous_germination = Some(germination());
    let mut o = LineageObservation::new(&mut w, ticks)?;
    drain(&mut o, out)?;
    for _ in 0..ticks {
        o.before_step(&w)?;
        w.step(1)?;
        o.after_step(&mut w)?;
        drain(&mut o, out)?;
    }
    o.finish(&w)?;
    drain(&mut o, out)?;
    out.flush().map_err(|e| e.to_string())?;
    Ok(w)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: resistance_longterm config ticks".into());
    }
    let c: SimulationConfig = serde_json::from_str(&std::fs::read_to_string(&args[1])?)?;
    run(
        c,
        args[2].parse()?,
        &mut std::io::BufWriter::new(std::io::stdout().lock()),
    )?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn logging_preserves_entire_ecological_state_and_rng() {
        let mut c = SimulationConfig::default();
        c.initial_population.plants = 30;
        c.initial_population.herbivores = 4;
        c.initial_population.carnivores = 1;
        let logged = run(c.clone(), 100, &mut Vec::new()).unwrap();
        let mut plain = World::new(c).unwrap();
        plain.spontaneous_germination = Some(germination());
        plain.step(100).unwrap();
        assert_eq!(
            format!("{:?}", logged.animals),
            format!("{:?}", plain.animals)
        );
        assert_eq!(
            format!("{:?}", logged.plants),
            format!("{:?}", plain.plants)
        );
        assert_eq!(
            format!("{:?}", logged.carnivores),
            format!("{:?}", plain.carnivores)
        );
        assert_eq!(
            format!("{:?}", logged.resources),
            format!("{:?}", plain.resources)
        );
        assert_eq!(
            format!("{:?}", logged.ledger),
            format!("{:?}", plain.ledger)
        );
        assert_eq!(format!("{:?}", logged.rng), format!("{:?}", plain.rng));
    }
    #[test]
    fn resistance_groups_match_f32_web_threshold() {
        assert_eq!(bin(0.6), 2);
        assert_eq!(bin(0.4), 0);
        assert_eq!(bin(0.5), 1);
    }
}
