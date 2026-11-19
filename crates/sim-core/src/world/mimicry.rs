//! Issue #72: measuring whether Batesian mimicry is happening (observation only).
//!
//! Herbivores are grouped by their current state: defended (toxin stock >= `DEFENDED_STOCK`),
//! mimic (undefended, body color within `MIMIC_DISTANCE` of a defended herbivore) and other
//! undefended. Nothing here changes ecology, RNG or resources.
use super::plant_color_learning::color_distance;
use super::World;
use serde_json::{json, Value};

/// Toxin stock (sum over types) at or above which a herbivore counts as defended.
pub const DEFENDED_STOCK: f64 = 0.5;
/// Color distance below which an undefended herbivore resembles a defended one
/// (the width over which predator color memory generalizes).
pub const MIMIC_DISTANCE: f64 = 0.25;
/// Group labels in output order.
pub const GROUPS: [&str; 3] = ["defended", "mimic", "other"];

/// Issue #76 diagnosis: strata for attack rates (observation only).
/// bit 0: juvenile (age below the reproduction minimum age),
/// bit 1: low energy (reserve below half of the maximum),
/// bit 2: a carnivore within carnivore vision.
pub const STRATA: usize = 8;

/// Cumulative attack and exposure counters per group.
#[derive(Debug, Default)]
pub struct MimicryBook {
    defended_colors: Vec<u32>,
    attacks: [u64; 3],
    exposure: [u64; 3],
    strata_attacks: [[u64; STRATA]; 3],
    strata_exposure: [[u64; STRATA]; 3],
    /// Carnivore color-memory entries with learned harm that were pushed out by a new color.
    pub(super) harmful_evictions: u64,
    /// New carnivore color-memory entries (one per newly experienced prey color).
    pub(super) memory_entries_created: u64,
    /// Issue #76 (revised M1, opt-in observation): choice scenes, where a hunting carnivore
    /// sees both a mimic and an other undefended herbivore. Per group of the nearest
    /// herbivore: scenes, and scenes in which the carnivore passed it over because of color.
    choice_observation: bool,
    census_groups: std::collections::BTreeMap<crate::entity::EntityId, u8>,
    choice_first: [u64; 3],
    choice_skipped: [u64; 3],
}

impl MimicryBook {
    /// Count one carnivore attack on a herbivore of group `g` and stratum `s`.
    pub(super) fn count_attack(&mut self, g: usize, s: usize) {
        self.attacks[g] += 1;
        self.strata_attacks[g][s] += 1;
    }
}

/// Nearest-defended distance (None when there is no defended herbivore).
fn nearest(color: u32, defended: &[u32]) -> Option<f64> {
    defended
        .iter()
        .map(|&d| color_distance(color, d))
        .min_by(f64::total_cmp)
}

/// Group index for a herbivore given its stock, color and the defended colors.
fn group(stock: f64, color: u32, defended: &[u32]) -> usize {
    if stock >= DEFENDED_STOCK {
        0
    } else if nearest(color, defended).is_some_and(|d| d < MIMIC_DISTANCE) {
        1
    } else {
        2
    }
}

impl World {
    fn herbivore_visible_color(&self, index: usize) -> u32 {
        super::animal_toxin::visible_color(self.animals.items[index].genes.body_color)
    }

    fn defended_colors(&self) -> Vec<u32> {
        (0..self.animals.items.len())
            .filter(|&i| self.herbivore_toxin_stock(self.animals.items[i].id) >= DEFENDED_STOCK)
            .map(|i| self.herbivore_visible_color(i))
            .collect()
    }

    fn mimicry_groups(&self, defended: &[u32]) -> Vec<usize> {
        (0..self.animals.items.len())
            .map(|i| {
                group(
                    self.herbivore_toxin_stock(self.animals.items[i].id),
                    self.herbivore_visible_color(i),
                    defended,
                )
            })
            .collect()
    }

    /// Issue #76 diagnosis: stratum of herbivore `i` (see `STRATA`). Read-only.
    fn herbivore_stratum(&self, i: usize) -> usize {
        let a = &self.animals.items[i];
        let h = &self.config.herbivores;
        let juvenile = a.age < h.reproduction_min_age;
        let maximum = f64::from(a.genes.adult_max_hp) * h.max_energy_per_hp;
        let energy = self.resources.energy_balance(a.reserve).unwrap_or(0.);
        let low = maximum > 0. && energy < 0.5 * maximum;
        let vision = self.config.carnivores.vision;
        let near = self
            .carnivores
            .iter()
            .any(|c| (c.x - a.x).powi(2) + (c.y - a.y).powi(2) < vision * vision);
        usize::from(juvenile) | (usize::from(low) << 1) | (usize::from(near) << 2)
    }

    /// Every `CENSUS_TICKS`: refresh the defended colors and add individual-ticks.
    pub(super) fn mimicry_census_if_due(&mut self) {
        if !self.tick.is_multiple_of(super::species::CENSUS_TICKS) {
            return;
        }
        let defended = self.defended_colors();
        let groups = self.mimicry_groups(&defended);
        for (i, &g) in groups.iter().enumerate() {
            self.mimicry.exposure[g] += super::species::CENSUS_TICKS;
            let s = self.herbivore_stratum(i);
            self.mimicry.strata_exposure[g][s] += super::species::CENSUS_TICKS;
        }
        if self.mimicry.choice_observation {
            self.mimicry.census_groups = groups
                .iter()
                .enumerate()
                .map(|(i, &g)| (self.animals.items[i].id, g as u8))
                .collect();
        }
        self.mimicry.defended_colors = defended;
    }

    /// Issue #76 (revised M1): turn on the choice-scene observation. Observation only.
    pub fn enable_mimicry_choice_observation(&mut self) {
        self.mimicry.choice_observation = true;
    }

    /// Issue #76 (revised M1, observation only): called where carnivore `predator` chooses
    /// its prey (not satiated, not resting), with the reserve used for that choice. If the
    /// carnivore sees both a mimic and an other undefended herbivore (groups from the last
    /// census), count the group of the nearest herbivore and whether its color is refused.
    pub(super) fn observe_mimicry_choice(&mut self, predator: usize, reserve_e: f64) {
        if !self.mimicry.choice_observation {
            return;
        }
        let c = &self.carnivores[predator];
        let (x, y, vision2, id) = (c.x, c.y, c.vision * c.vision, c.id);
        let mut nearest: Option<(f32, usize, u8)> = None;
        let mut seen = [false; 3];
        for (j, h) in self.animals.items.iter().enumerate() {
            let d = (h.x - x).powi(2) + (h.y - y).powi(2);
            if d >= vision2 {
                continue;
            }
            let Some(&g) = self.mimicry.census_groups.get(&h.id) else {
                continue;
            };
            seen[usize::from(g)] = true;
            if nearest.is_none_or(|(best, _, _)| d < best) {
                nearest = Some((d, j, g));
            }
        }
        let Some((_, j, g)) = nearest.filter(|_| seen[1] && seen[2]) else {
            return;
        };
        let refused =
            !self.carnivore_allows_color(id, self.animals.items[j].genes.body_color, reserve_e);
        self.mimicry.choice_first[usize::from(g)] += 1;
        self.mimicry.choice_skipped[usize::from(g)] += u64::from(refused);
    }

    /// Group of herbivore `prey` for an attack (uses the last census defended colors).
    fn mimicry_attack_group(&self, prey: usize) -> usize {
        let stock = self.herbivore_toxin_stock(self.animals.items[prey].id);
        group(
            stock,
            self.herbivore_visible_color(prey),
            &self.mimicry.defended_colors,
        )
    }

    /// Group and stratum of herbivore `prey` for an attack (observation only).
    pub(super) fn mimicry_attack_class(&self, prey: usize) -> (usize, usize) {
        (
            self.mimicry_attack_group(prey),
            self.herbivore_stratum(prey),
        )
    }

    /// Issue #76 diagnosis (observation only): stratified attack counters, per-group
    /// covariate means, carnivore color-memory summary and color crowding.
    pub fn mimicry_diagnostics_value(&self) -> Value {
        let defended = self.defended_colors();
        let groups = self.mimicry_groups(&defended);
        let h = &self.config.herbivores;
        const KEYS: [&str; 10] = [
            "age",
            "juvenile",
            "energy_fraction",
            "low_energy",
            "near_carnivore",
            "speed",
            "adult_max_hp",
            "toxin_ability",
            "toxin_stock",
            "plant_toxin_load",
        ];
        let mut sums = [[0f64; KEYS.len()]; 3];
        let mut counts = [0u64; 3];
        for (i, &g) in groups.iter().enumerate() {
            let a = &self.animals.items[i];
            let s = self.herbivore_stratum(i);
            let maximum = f64::from(a.genes.adult_max_hp) * h.max_energy_per_hp;
            let energy = self.resources.energy_balance(a.reserve).unwrap_or(0.);
            let plant_load = a.toxin_load + a.typed_load[1..].iter().sum::<f64>();
            let values = [
                f64::from(a.age),
                f64::from(u8::from(s & 1 != 0)),
                if maximum > 0. { energy / maximum } else { 0. },
                f64::from(u8::from(s & 2 != 0)),
                f64::from(u8::from(s & 4 != 0)),
                f64::from(a.genes.speed),
                f64::from(a.genes.adult_max_hp),
                self.herbivore_toxin_ability(a.id),
                self.herbivore_toxin_stock(a.id),
                plant_load,
            ];
            for (sum, v) in sums[g].iter_mut().zip(values) {
                *sum += v;
            }
            counts[g] += 1;
        }
        let profile: serde_json::Map<String, Value> = GROUPS
            .iter()
            .enumerate()
            .map(|(g, name)| {
                let means: serde_json::Map<String, Value> = KEYS
                    .iter()
                    .zip(sums[g])
                    .map(|(k, s)| {
                        let v = (counts[g] > 0).then(|| s / counts[g] as f64);
                        (k.to_string(), v.into())
                    })
                    .collect();
                (name.to_string(), means.into())
            })
            .collect();
        // Color crowding: mean share of other herbivores within MIMIC_DISTANCE (the width
        // over which carnivore memory generalizes). Deterministic subsample of at most 300.
        let colors: Vec<u32> = (0..self.animals.items.len())
            .map(|i| self.herbivore_visible_color(i))
            .collect();
        let step = colors.len().div_ceil(300).max(1);
        let mut shares = Vec::new();
        for (i, &c) in colors.iter().enumerate().step_by(step) {
            if colors.len() < 2 {
                break;
            }
            let near = colors
                .iter()
                .enumerate()
                .filter(|&(j, &d)| j != i && color_distance(c, d) < MIMIC_DISTANCE)
                .count();
            shares.push(near as f64 / (colors.len() - 1) as f64);
        }
        let m = &self.mimicry;
        json!({
            "tick": self.tick,
            "strata_attacks": m.strata_attacks,
            "strata_exposure": m.strata_exposure,
            "profile": profile,
            "color_neighbor_share": (!shares.is_empty())
                .then(|| shares.iter().sum::<f64>() / shares.len() as f64),
            "carnivore_memory": self.carnivore_memory_summary(),
            "harmful_evictions": m.harmful_evictions,
            "memory_entries_created": m.memory_entries_created,
            "choice_first": m.choice_first,
            "choice_skipped": m.choice_skipped,
        })
    }

    /// Current mimicry indicators as JSON (Issue #72).
    pub fn mimicry_json(&self) -> String {
        self.mimicry_value().to_string()
    }

    pub fn mimicry_value(&self) -> Value {
        let defended = self.defended_colors();
        let groups = self.mimicry_groups(&defended);
        let mut counts = [0u64; 3];
        let mut avoided = [0f64; 3];
        let mut distances = Vec::new();
        for (i, &g) in groups.iter().enumerate() {
            counts[g] += 1;
            let h = &self.animals.items[i];
            if g != 0 {
                if let Some(d) = nearest(self.herbivore_visible_color(i), &defended) {
                    distances.push(d);
                }
            }
            if !self.carnivores.is_empty() {
                let refusing = self
                    .carnivores
                    .iter()
                    .filter(|c| {
                        let reserve = self.resources.energy_balance(c.reserve).unwrap_or(0.);
                        !self.carnivore_allows_color(c.id, h.genes.body_color, reserve)
                    })
                    .count();
                avoided[g] += refusing as f64 / self.carnivores.len() as f64;
            }
        }
        let undefended = counts[1] + counts[2];
        let per_group = |f: &dyn Fn(usize) -> Value| -> Value {
            GROUPS
                .iter()
                .enumerate()
                .map(|(g, name)| (name.to_string(), f(g)))
                .collect::<serde_json::Map<_, _>>()
                .into()
        };
        let m = &self.mimicry;
        json!({
            "tick": self.tick,
            "counts": per_group(&|g| counts[g].into()),
            "mimic_share_of_undefended": (undefended > 0).then(|| counts[1] as f64 / undefended as f64),
            "mean_distance_to_defended": (!distances.is_empty())
                .then(|| distances.iter().sum::<f64>() / distances.len() as f64),
            "avoided_by_carnivores": per_group(&|g| (counts[g] > 0 && !self.carnivores.is_empty())
                .then(|| avoided[g] / counts[g] as f64).into()),
            "attacks": per_group(&|g| m.attacks[g].into()),
            "individual_ticks": per_group(&|g| m.exposure[g].into()),
            "attacks_per_1000_individual_ticks": per_group(&|g| (m.exposure[g] > 0)
                .then(|| m.attacks[g] as f64 * 1000. / m.exposure[g] as f64).into()),
            "definitions": {"defended_stock": DEFENDED_STOCK, "mimic_distance": MIMIC_DISTANCE},
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_follow_stock_and_distance_to_defended_colors() {
        let red = 0xff000000;
        let near_red = 0xf0100000;
        let blue = 0x0000ff00;
        assert_eq!(group(0.6, blue, &[red]), 0);
        assert_eq!(group(0.1, near_red, &[red]), 1);
        assert_eq!(group(0.1, blue, &[red]), 2);
        assert_eq!(group(0.1, near_red, &[]), 2);
        assert!(nearest(near_red, &[blue, red]).unwrap() < MIMIC_DISTANCE);
    }
}
