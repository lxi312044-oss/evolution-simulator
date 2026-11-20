//! Issue #82 (opt-in): a heritable tendency to hunt other carnivores. Every animal carries a
//! value from 0 to 1 (herbivores without using it); offspring inherit it with rare normal steps
//! from a stateless stream keyed by the entity id, so carrying it never consumes ecology RNG.
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
