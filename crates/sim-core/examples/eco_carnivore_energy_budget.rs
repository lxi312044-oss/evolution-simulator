//! Issue #11: POST-HOC (not pre-registered) carnivore energy budget for the base arm of
//! `eco_carnivore_extinction`, seeds 90001-90016, until carnivore extinction or 20000 ticks.
//!
//! Usage: cargo run --release -p sim-core --example eco_carnivore_energy_budget -- \
//!   config/stage4-inherited-plant-colors.json
use sim_core::{SimulationConfig, World};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let text = std::fs::read_to_string(&a[1]).unwrap();
    let (
        mut ct,
        mut basal,
        mut movement,
        mut repro,
        mut hunt_e,
        mut bonus,
        mut digested,
        mut emergency,
        mut cool,
        mut chase_mv,
        mut chase_basal,
        mut dist,
    ) = (0u64, 0., 0., 0., 0., 0., 0., 0., 0u64, 0., 0., 0.);
    for seed in 90001..=90016u64 {
        let mut c: SimulationConfig = serde_json::from_str(&text).unwrap();
        c.seed = seed;
        let mut w = World::new(c).unwrap();
        for _ in 0..200 {
            w.step(100).unwrap();
            ct += w.carnivores.len() as u64 * 100;
            if w.carnivores.is_empty() {
                break;
            }
        }
        let d = &w.carnivore_debug;
        basal += d.energy_basal_spent;
        movement += d.energy_movement_spent;
        repro += d.energy_reproduction_spent;
        hunt_e += d.hunt_energy;
        bonus += d.meal_bonus_energy;
        digested += d.stomach_energy_digested;
        emergency += d.emergency_digestion_energy;
        cool += d.hunt_cooldown_blocked;
        chase_mv += d.p41_chase_movement_energy;
        chase_basal += d.p41_chase_basal_energy;
        dist += d.predator_movement_distance;
    }
    let cfg: SimulationConfig = serde_json::from_str(&text).unwrap();
    println!("carnivore_ticks {ct}");
    println!("spent: basal {basal:.1} movement {movement:.1} reproduction {repro:.1} (chase movement {chase_mv:.1}, chase basal {chase_basal:.1})");
    println!("gained: hunt_energy {hunt_e:.1} meal_bonus {bonus:.1} stomach_digested {digested:.1} emergency_digestion {emergency:.1}");
    println!("per carnivore-tick: basal {:.4} movement {:.4} digested {:.4}  distance/tick {:.3}  cooldown_blocked {cool}", basal/ct as f64, movement/ct as f64, digested/ct as f64, dist/ct as f64);
    let c = &cfg.carnivores;
    let h = &cfg.herbivores;
    println!(
        "carnivore speed {} basal {} move/unit {} | herbivore speed {}..{}",
        c.speed, c.basal_energy_per_tick, c.movement_energy_per_unit, h.speed_min, h.speed_max
    );
}
