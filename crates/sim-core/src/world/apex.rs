//! Issue #82 (opt-in): a heritable tendency to hunt other carnivores. Every animal carries a
//! value from 0 to 1 (herbivores without using it); offspring inherit it with rare normal steps
//! from a stateless stream keyed by the entity id, so carrying it never consumes ecology RNG.
//!
//! With `effects` on (stage 1) a hunting carnivore compares other carnivores, not of its own
//! species, at distance / tendency with the nearest herbivore. A bite on a carnivore moves the
//! same amounts as a bite on a herbivore (body nutrients, reserve energy, within the stomach's
//! room, scaled by the diet value), and the bitten carnivore fights back: the attacker loses
//! `counter_energy x prey reserve / (prey reserve + attacker reserve)` energy as heat. A
//! carnivore whose reserve runs out dies by the existing rule. Carnivore toxins do not move.
use super::World;
use crate::entity::EntityId;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const SALT: u64 = 0x6170_6578_5f70_7265;
const FOUNDER_STREAM: u64 = 1;
const MOVE_STREAM: u64 = 2;
const STEP_STREAM: u64 = 4;

fn draw(seed: u64, id: EntityId, stream: u64) -> f64 {
    let identity = (u64::from(id.index) << 32) | u64::from(id.generation);
    crate::toxin::herbivore_type_draw(seed ^ SALT, identity, 0, stream)
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub struct ApexBook {
    /// Values of animals born to a parent (founders are drawn from the seed when read).
    value: BTreeMap<EntityId, f64>,
    /// Bites of carnivores on carnivores, and what they moved (observation only).
    pub(super) bites: u64,
    pub(super) nutrients: f64,
    pub(super) energy: f64,
    pub(super) counter_energy: f64,
}

impl ApexBook {
    pub(super) fn remove(&mut self, id: EntityId) {
        self.value.remove(&id);
    }

    #[cfg(feature = "state-audit")]
    pub(super) fn is_empty(&self) -> bool {
        self.value.is_empty()
            && self.bites == 0
            && self.nutrients == 0.
            && self.energy == 0.
            && self.counter_energy == 0.
    }
}

impl World {
    /// The tendency of animal `id`; founders draw theirs from the seed. None when off.
    pub(super) fn apex_value(&self, id: EntityId) -> Option<f64> {
        let a = self.config.apex_predation.as_ref()?;
        Some(
            self.apex
                .value
                .get(&id)
                .copied()
                .unwrap_or_else(|| a.initial_max * draw(self.config.seed, id, FOUNDER_STREAM)),
        )
    }

    /// Whether the tendency acts on hunting (stage 1).
    pub(super) fn apex_effects(&self) -> bool {
        self.config
            .apex_predation
            .as_ref()
            .is_some_and(|a| a.effects)
    }

    /// The carnivore that carnivore `i` (at x, y, seeing `vision`) hunts instead of the
    /// herbivore at squared distance `herbivore_d2` (vision squared when none), if any.
    pub(super) fn apex_target(
        &self,
        i: usize,
        x: f32,
        y: f32,
        vision: f32,
        herbivore_d2: f32,
        reserve_e: f64,
    ) -> Option<usize> {
        let id = self.carnivores[i].id;
        let tendency = self.apex_value(id)? as f32;
        let own = self.species.species_of(id)?;
        if tendency <= 0. {
            return None;
        }
        let mut best = herbivore_d2;
        let mut target = None;
        for (k, other) in self.carnivores.iter().enumerate() {
            if k == i || self.species.species_of(other.id).is_none_or(|s| s == own) {
                continue;
            }
            let d2 = (other.x - x).powi(2) + (other.y - y).powi(2);
            if d2 >= vision * vision {
                continue;
            }
            let effective = d2 / (tendency * tendency);
            if effective < best && self.carnivore_allows_color(id, other.color, reserve_e) {
                best = effective;
                target = Some(k);
            }
        }
        target
    }

    /// Carnivore `i` bites carnivore `k` if it is in attack range and the attack is ready.
    pub(super) fn apex_bite(&mut self, i: usize, k: usize) -> Result<(), String> {
        let c = &self.config.carnivores;
        let (attacker, prey) = (&self.carnivores[i], &self.carnivores[k]);
        if (prey.x - attacker.x).powi(2) + (prey.y - attacker.y).powi(2)
            > c.attack_range * c.attack_range
            || attacker.attack_cooldown > 0
        {
            return Ok(());
        }
        let (stomach, reserve) = (attacker.stomach, attacker.reserve);
        let (source, prey_reserve) = (prey.body, prey.reserve);
        let target = if c.all_bites_to_stomach {
            stomach
        } else {
            reserve
        };
        let room = |balance: f64, capacity: f64| {
            if c.all_bites_to_stomach {
                (capacity - balance).max(0.)
            } else {
                f64::INFINITY
            }
        };
        let free_n = room(
            self.resources.nutrient_balance(stomach).unwrap_or(0.),
            c.stomach_capacity_nutrients,
        );
        let free_e = room(
            self.resources.energy_balance(stomach).unwrap_or(0.),
            c.stomach_capacity_energy,
        );
        let n = c
            .bite_nutrients
            .min(free_n)
            .min(self.resources.nutrient_balance(source).unwrap_or(0.));
        let e = c
            .bite_energy
            .min(free_e)
            .min(self.resources.energy_balance(prey_reserve).unwrap_or(0.));
        let (cooldown, bout, stop) = (
            c.attack_cooldown_ticks,
            c.hunt_bout_ticks,
            c.feeding_stop_fraction,
        );
        let (all_to_stomach, capacity_e, capacity_n) = (
            c.all_bites_to_stomach,
            c.stomach_capacity_energy,
            c.stomach_capacity_nutrients,
        );
        let counter = self
            .config
            .apex_predation
            .as_ref()
            .map_or(0., |a| a.counter_energy);
        self.carnivores[i].attack_cooldown = cooldown;
        self.resources
            .transfer_nutrients(source, target, n)
            .map_err(|err| format!("apex bite N: {err:?}"))?;
        self.resources
            .transfer_energy(prey_reserve, target, e)
            .map_err(|err| format!("apex bite E: {err:?}"))?;
        self.diet_lose_meal(i, target, n, e)?;
        // The bitten carnivore fights back in proportion to its share of both reserves.
        let own_e = self.resources.energy_balance(reserve).unwrap_or(0.);
        let prey_e = self.resources.energy_balance(prey_reserve).unwrap_or(0.);
        let share = if own_e + prey_e > 0. {
            prey_e / (own_e + prey_e)
        } else {
            0.
        };
        let lost = (counter * share).min(own_e).max(0.);
        if lost > 0. {
            self.ledger
                .dissipate_heat(&mut self.resources, reserve, lost)
                .map_err(|err| format!("apex counterattack: {err:?}"))?;
        }
        if all_to_stomach {
            let fullness = (self.resources.energy_balance(stomach).unwrap_or(0.) / capacity_e)
                .max(self.resources.nutrient_balance(stomach).unwrap_or(0.) / capacity_n);
            if fullness >= stop {
                self.carnivores[i].feeding_active = false;
            }
        }
        if bout > 0 {
            self.carnivore_hunt_bouts
                .entry(self.carnivores[i].id)
                .or_default()
                .gained += e;
        }
        self.apex.bites += 1;
        self.apex.nutrients += n;
        self.apex.energy += e;
        self.apex.counter_energy += lost;
        Ok(())
    }

    /// Record the value of a newborn `child` of `parent`: the parent's value, moved by a
    /// normal step with the configured probability (no-op when off).
    pub(super) fn apex_record_birth(&mut self, parent: EntityId, child: EntityId) {
        let (Some(a), Some(value)) = (self.config.apex_predation.as_ref(), self.apex_value(parent))
        else {
            return;
        };
        let seed = self.config.seed;
        let value = if draw(seed, child, MOVE_STREAM) < a.mutation_probability {
            // Box-Muller from two stateless uniforms; 1 - u lies in (0, 1].
            let radius = (-2. * (1. - draw(seed, child, STEP_STREAM)).ln()).sqrt();
            let z = radius * (std::f64::consts::TAU * draw(seed, child, STEP_STREAM + 1)).cos();
            (value + a.mutation_scale * z).clamp(0., 1.)
        } else {
            value
        };
        self.apex.value.insert(child, value);
    }

    /// Observation as JSON (null when off): carnivore and herbivore value summaries and the
    /// cumulative bites of carnivores on carnivores.
    pub fn apex_json(&self) -> Value {
        if self.config.apex_predation.is_none() {
            return Value::Null;
        }
        let summary = |values: Vec<f64>| {
            let n = values.len();
            let mean = (n > 0).then(|| values.iter().sum::<f64>() / n as f64);
            let max = values.iter().copied().reduce(f64::max);
            let above = values.iter().filter(|v| **v >= 0.2).count();
            json!({"n": n, "mean": mean, "max": max, "at_least_0_2": above})
        };
        let carnivores = self
            .carnivores
            .iter()
            .filter_map(|c| self.apex_value(c.id))
            .collect();
        let herbivores = self
            .animals
            .items
            .iter()
            .filter_map(|h| self.apex_value(h.id))
            .collect();
        let b = &self.apex;
        json!({
            "carnivore": summary(carnivores),
            "herbivore": summary(herbivores),
            "bites": b.bites,
            "nutrients": b.nutrients,
            "energy": b.energy,
            "counter_energy": b.counter_energy,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::ApexPredationSettings, SimulationConfig};

    /// Three carnivores in a row at x = 100, 105 and 130 (y = 100); 0 and 1 share a species,
    /// 2 is another one. Carnivore 0 has tendency `tendency`.
    fn world(tendency: f64) -> World {
        let mut c: SimulationConfig =
            serde_json::from_str(include_str!("../../../../config/stage4-standard.json")).unwrap();
        c.world.width = 1000.;
        c.world.height = 1000.;
        c.world.land_columns = 20;
        c.world.land_rows = 20;
        c.initial_population.plants = 300;
        c.initial_population.herbivores = 80;
        c.initial_population.carnivores = 3;
        c.apex_predation = Some(ApexPredationSettings {
            effects: true,
            ..Default::default()
        });
        let mut w = World::new(c).unwrap();
        for (k, x) in [100., 105., 130.].into_iter().enumerate() {
            w.carnivores[k].x = x;
            w.carnivores[k].y = 100.;
        }
        let ids: Vec<EntityId> = w.carnivores.iter().map(|c| c.id).collect();
        w.species.set_member_for_test(ids[0], 900);
        w.species.set_member_for_test(ids[1], 900);
        w.species.set_member_for_test(ids[2], 901);
        w.apex.value.insert(ids[0], tendency);
        w
    }

    #[test]
    fn apex_target_skips_own_species_and_zero_tendency() {
        let w = world(1.);
        let vision = w.carnivores[0].vision;
        // Carnivore 1 is nearer but of the same species; carnivore 2 is hunted.
        assert_eq!(
            w.apex_target(0, 100., 100., vision, vision * vision, 60.),
            Some(2)
        );
        // A herbivore nearer than 30 wins at tendency 1.
        assert_eq!(w.apex_target(0, 100., 100., vision, 25. * 25., 60.), None);
        // At tendency 0.5 the carnivore counts as 60 away.
        let half = world(0.5);
        assert_eq!(
            half.apex_target(0, 100., 100., vision, 50. * 50., 60.),
            None
        );
        assert_eq!(
            half.apex_target(0, 100., 100., vision, 70. * 70., 60.),
            Some(2)
        );
        assert_eq!(
            world(0.).apex_target(0, 100., 100., vision, vision * vision, 60.),
            None
        );
    }

    #[test]
    fn apex_bite_moves_food_and_the_prey_fights_back() {
        let mut w = world(1.);
        w.carnivores[2].x = 104.;
        let (reserve, stomach) = (w.carnivores[0].reserve, w.carnivores[0].stomach);
        let prey_reserve = w.carnivores[2].reserve;
        let energy = |w: &World, a| w.resources.energy_balance(a).unwrap();
        let prey_before = energy(&w, prey_reserve);
        let reserve_before = energy(&w, reserve);
        let stomach_before = energy(&w, stomach);
        w.apex_bite(0, 2).unwrap();
        let bitten = prey_before - energy(&w, prey_reserve);
        assert!(bitten > 0.);
        assert_eq!(w.apex.bites, 1);
        assert!(w.apex.counter_energy > 0.);
        // The stomach gets the bite (scaled by the diet value); the reserve pays the counter.
        assert!(energy(&w, stomach) > stomach_before);
        assert!(energy(&w, reserve) < reserve_before);
        assert_eq!(
            w.carnivores[0].attack_cooldown,
            w.config.carnivores.attack_cooldown_ticks
        );
        // Not ready again until the cooldown ends.
        w.apex_bite(0, 2).unwrap();
        assert_eq!(w.apex.bites, 1);
    }
}
