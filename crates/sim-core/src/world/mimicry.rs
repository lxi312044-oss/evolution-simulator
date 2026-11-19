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

/// Cumulative attack and exposure counters per group.
#[derive(Debug, Default)]
pub struct MimicryBook {
    defended_colors: Vec<u32>,
    attacks: [u64; 3],
    exposure: [u64; 3],
}

impl MimicryBook {
    /// Count one carnivore attack on a herbivore of group `g`.
    pub(super) fn count_attack(&mut self, g: usize) {
        self.attacks[g] += 1;
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

    /// Every `CENSUS_TICKS`: refresh the defended colors and add individual-ticks.
    pub(super) fn mimicry_census_if_due(&mut self) {
        if !self.tick.is_multiple_of(super::species::CENSUS_TICKS) {
            return;
        }
        let defended = self.defended_colors();
        for g in self.mimicry_groups(&defended) {
            self.mimicry.exposure[g] += super::species::CENSUS_TICKS;
        }
        self.mimicry.defended_colors = defended;
    }

    /// Group of herbivore `prey` for an attack (uses the last census defended colors).
    pub(super) fn mimicry_attack_group(&self, prey: usize) -> usize {
        let stock = self.herbivore_toxin_stock(self.animals.items[prey].id);
        group(
            stock,
            self.herbivore_visible_color(prey),
            &self.mimicry.defended_colors,
        )
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
