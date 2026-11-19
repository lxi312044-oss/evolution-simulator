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

    /// Issue #76 (opt-in, carnivores): like `observe_damage`, but the harm written to memory
    /// and its per-color upper limit are multiplied by `gain`.
    pub(crate) fn observe_damage_with_gain(
        &mut self,
        toxin_type: usize,
        before: f64,
        after: f64,
        damage: f32,
        gain: f64,
    ) {
        if before <= 0. {
            return;
        }
        for entry in &mut self.experiences {
            let load = &mut entry.loads[toxin_type];
            let fraction = (*load / before).clamp(0., 1.);
            *load = fraction * after;
            entry.damage = (entry.damage + f64::from(damage) * gain * fraction).min(0.1 * gain);
        }
    }

    /// Harm experienced directly from an encounter with this color (e.g. a counterattack),
    /// without a toxin load. Uses the same 0.1 cap as toxin damage.
    pub(crate) fn observe_injury(&mut self, color: u32, harm: f64) {
        if harm <= 0. {
            return;
        }
        let color = color & 0xffffff00;
        if let Some(entry) = self
            .experiences
            .iter_mut()
            .find(|entry| entry.color == color)
        {
            entry.damage = (entry.damage + harm).min(0.1);
            return;
        }
        if self.experiences.len() == Self::CAPACITY {
            self.experiences.remove(0);
        }
        self.experiences.push(Experience {
            color,
            loads: [0.; 16],
            damage: harm.min(0.1),
        });
    }

    /// Opt-in extinction of avoidance: a toxin-free meal weakens learned harm for
    /// similar colors, using the same generalization width as `allows`.
    pub(crate) fn relieve_safe_meal(&mut self, color: u32, relief: f64) {
        for entry in &mut self.experiences {
            let similarity = (1. - color_distance(color, entry.color) / 0.25).max(0.);
            entry.damage *= 1. - relief * similarity;
        }
    }

    /// Opt-in forgetting: learned harm fades by a fixed fraction per tick.
    pub(crate) fn decay(&mut self, rate: f64) {
        for entry in &mut self.experiences {
            entry.damage *= 1. - rate;
        }
    }

    pub(super) fn harm_scores(&self) -> Vec<(u32, f64)> {
        self.experiences
            .iter()
            .filter(|entry| entry.damage > 0.)
            .map(|entry| (entry.color, entry.damage))
            .collect()
    }

    /// Read-only (Issue #76 diagnosis): would `ingest(color, ..)` create a new entry, and
    /// would that push out an entry carrying learned harm? Returns (new_entry, evicts_harm).
    pub(super) fn ingest_effect(&self, color: u32) -> (bool, bool) {
        let color = color & 0xffffff00;
        if self.experiences.iter().any(|entry| entry.color == color) {
            return (false, false);
        }
        let evicts = self.experiences.len() == Self::CAPACITY && self.experiences[0].damage > 0.;
        (true, evicts)
    }

    /// Read-only (Issue #76 diagnosis): number of remembered colors.
    pub(super) fn entry_count(&self) -> usize {
        self.experiences.len()
    }

    /// Read-only (Issue #76 diagnosis): the largest learned risk x similarity for a color.
    /// `allows(color, f)` is false exactly when this exceeds `risk_tolerance(f)`.
    pub(super) fn color_risk(&self, color: u32) -> f64 {
        self.experiences
            .iter()
            .map(|entry| {
                entry.damage * 10. * (1. - color_distance(color, entry.color) / 0.25).max(0.)
            })
            .fold(0., f64::max)
    }

    pub(super) fn has_harm_for_color(&self, color: u32) -> bool {
        self.experiences
            .iter()
            .any(|entry| entry.damage > 0. && color_distance(color, entry.color) < 0.25)
    }

    pub fn allows(&self, color: u32, reserve_fraction: f64) -> bool {
        // A hungrier animal accepts more risk. Only learned harm and visible color enter the choice.
        let tolerance = risk_tolerance(reserve_fraction);
        self.experiences.iter().all(|entry| {
            let risk = entry.damage * 10.;
            risk <= tolerance
                || risk * (1. - color_distance(color, entry.color) / 0.25).max(0.) <= tolerance
        })
    }
}

pub(super) fn risk_tolerance(reserve_fraction: f64) -> f64 {
    1. - 0.75 * reserve_fraction.clamp(0., 1.)
}

pub(super) fn color_distance(a: u32, b: u32) -> f64 {
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

    fn learned(color: u32) -> PlantColorMemory {
        let mut memory = PlantColorMemory::default();
        memory.ingest(color, 0, 1.);
        memory.observe_damage(0, 1., 0.5, 0.1);
        memory
    }

    #[test]
    fn safe_meal_relief_scales_with_color_similarity() {
        let mut memory = learned(0xff0000ff);
        assert_eq!(memory.harm_scores(), vec![(0xff000000, 0.1)]);
        memory.relieve_safe_meal(0xff0000ff, 0.5);
        assert_eq!(memory.harm_scores()[0].1, 0.05);
        // A distant color carries no evidence about the learned one.
        memory.relieve_safe_meal(0x0000ffff, 0.5);
        assert_eq!(memory.harm_scores()[0].1, 0.05);
        // Half of the generalization width gives half of the relief.
        let half = 0xff000000 - ((0.125_f64 * 255.).round() as u32) * 0x0100_0000;
        let similarity = 1. - color_distance(half, 0xff000000) / 0.25;
        memory.relieve_safe_meal(half | 0xff, 0.5);
        assert!((memory.harm_scores()[0].1 - 0.05 * (1. - 0.5 * similarity)).abs() < 1e-12);
        assert!(similarity > 0.45 && similarity < 0.55);
    }

    #[test]
    fn zero_relief_and_decay_leave_memory_unchanged() {
        let mut memory = learned(0xff0000ff);
        memory.relieve_safe_meal(0xff0000ff, 0.);
        memory.decay(0.);
        assert_eq!(memory.harm_scores(), vec![(0xff000000, 0.1)]);
        assert!(!memory.allows(0xff0000ff, 0.5));
    }

    #[test]
    fn decay_restores_acceptance_over_time() {
        let mut memory = learned(0xff0000ff);
        assert!(!memory.allows(0xff0000ff, 1.));
        for _ in 0..1000 {
            memory.decay(0.001);
        }
        let harm = memory.harm_scores()[0].1;
        assert!((harm - 0.1 * 0.999_f64.powi(1000)).abs() < 1e-12);
        // risk = 10 * harm ~= 0.368 <= tolerance 1 - 0.75 * 0.8 = 0.4
        assert!(memory.allows(0xff0000ff, 0.8));
        assert!(!memory.allows(0xff0000ff, 1.));
    }

    #[test]
    fn learning_gain_scales_harm_and_its_limit() {
        let run = |gain: Option<f64>, damage: f32| {
            let mut memory = PlantColorMemory::default();
            memory.ingest(0xff0000ff, 0, 1.);
            match gain {
                None => memory.observe_damage(0, 1., 0.5, damage),
                Some(g) => memory.observe_damage_with_gain(0, 1., 0.5, damage, g),
            }
            memory.harm_scores()[0].1
        };
        assert_eq!(run(None, 0.003).to_bits(), run(Some(1.), 0.003).to_bits());
        assert!((run(Some(30.), 0.003) - 30. * run(None, 0.003)).abs() < 1e-12);
        assert_eq!(run(None, 1.), 0.1);
        assert_eq!(run(Some(30.), 1.), 3.);
        let mut memory = PlantColorMemory::default();
        memory.ingest(0xff0000ff, 0, 1.);
        memory.observe_damage_with_gain(0, 1., 0.5, 0.003, 100.);
        // risk = 10 x harm = 3 > tolerance even for an empty reserve
        assert!(!memory.allows(0xff0000ff, 0.));
    }

    #[test]
    fn color_risk_matches_the_choice_rule() {
        let mut memory = PlantColorMemory::default();
        memory.ingest(0xff0000ff, 0, 1.);
        memory.observe_damage_with_gain(0, 1., 0.5, 0.003, 100.);
        for color in [0xff000000_u32, 0xe0200000, 0xb0400000, 0x0000ff00] {
            for fraction in [0., 0.3, 0.7, 1.] {
                let refuse = memory.color_risk(color) > risk_tolerance(fraction);
                assert_eq!(
                    refuse,
                    !memory.allows(color, fraction),
                    "{color:x} {fraction}"
                );
            }
        }
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
