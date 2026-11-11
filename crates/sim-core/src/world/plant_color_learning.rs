/// A bounded record of ingested doses and the damage experienced with each color.
#[derive(Debug, Default)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub struct PlantColorMemory {
    experiences: Vec<Experience>,
}

#[derive(Debug)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
struct Experience {
    color: u32,
    loads: [f64; 16],
    damage: f64,
}

impl PlantColorMemory {
    const CAPACITY: usize = 30;

    pub(crate) fn ingest(&mut self, color: u32, toxin_type: usize, dose: f64) {
        let color = color & 0xffffff00;
        if let Some(entry) = self
            .experiences
            .iter_mut()
            .find(|entry| entry.color == color)
        {
            entry.loads[toxin_type] += dose;
            return;
        }
        if self.experiences.len() == Self::CAPACITY {
            self.experiences.remove(0);
        }
        let mut loads = [0.; 16];
        loads[toxin_type] = dose;
        self.experiences.push(Experience {
            color,
            loads,
            damage: 0.,
        });
    }

    pub(crate) fn observe_damage(
        &mut self,
        toxin_type: usize,
        before: f64,
        after: f64,
        damage: f32,
    ) {
        if before <= 0. {
            return;
        }
        for entry in &mut self.experiences {
            let load = &mut entry.loads[toxin_type];
            // Untracked load (e.g. an evicted color) remains in the denominator.
            let fraction = (*load / before).clamp(0., 1.);
            *load = fraction * after;
            entry.damage = (entry.damage + f64::from(damage) * fraction).min(0.1);
        }
    }

    pub fn allows(&self, color: u32, reserve_fraction: f64) -> bool {
        // A hungrier animal accepts more risk. Only learned harm and visible color enter the choice.
        let tolerance = 1. - 0.75 * reserve_fraction.clamp(0., 1.);
        self.experiences.iter().all(|entry| {
            let risk = entry.damage * 10.;
            risk <= tolerance
                || risk * (1. - color_distance(color, entry.color) / 0.25).max(0.) <= tolerance
        })
    }
}

fn color_distance(a: u32, b: u32) -> f64 {
    let squared: f64 = [8, 16, 24]
        .into_iter()
        .map(|shift| {
            let x = ((a >> shift) & 255) as f64;
            let y = ((b >> shift) & 255) as f64;
            (x - y).powi(2)
        })
        .sum();
    squared.sqrt() / 255.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delayed_damage_follows_the_consumed_colors_and_types() {
        let mut memory = PlantColorMemory::default();
        memory.ingest(0xff0000ff, 0, 2.);
        memory.observe_damage(0, 2., 1., 0.);
        memory.ingest(0x0000ffff, 1, 1.);
        assert!(memory.allows(0xff0000ff, 1.));
        memory.observe_damage(0, 1., 0.5, 0.1);
        assert!(!memory.allows(0xff0000ff, 1.));
        assert!(!memory.allows(0xef0000ff, 1.));
        assert!(memory.allows(0x0000ffff, 1.));
        assert!(memory.allows(0xff0000ff, 0.));
    }

    #[test]
    fn same_type_damage_is_shared_by_remaining_load() {
        let mut memory = PlantColorMemory::default();
        memory.ingest(0xff0000ff, 0, 2.);
        memory.observe_damage(0, 2., 1., 0.);
        memory.ingest(0x0000ffff, 0, 3.);
        memory.observe_damage(0, 4., 2., 0.08);
        assert!(memory.allows(0xff0000ff, 1.));
        assert!(!memory.allows(0x0000ffff, 1.));
    }

    #[test]
    fn memory_bounds_the_number_of_colors() {
        let mut memory = PlantColorMemory::default();
        for color in 0..40 {
            memory.ingest(color << 8, 0, 1.);
        }
        assert_eq!(memory.experiences.len(), 30);
    }
}
