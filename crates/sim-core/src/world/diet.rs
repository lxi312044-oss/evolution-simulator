//! Issue #78 (opt-in): a heritable diet value from 0 (herbivore) to 1 (carnivore) on every
//! animal. Values come from a stateless stream keyed by the entity id, so carrying them never
//! consumes ecology RNG. With `diet_gene.effects` off the value is observed only.
use super::World;
use crate::entity::EntityId;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const SALT: u64 = 0x6469_6574_5f67_656e;
const FOUNDER_STREAM: u64 = 1;
const MOVE_STREAM: u64 = 2;
const STEP_STREAM: u64 = 4;
/// Largest herbivore value: a herbivore body always carries a value below 0.5.
const HERBIVORE_MAX: f64 = 0.5 - 1e-9;

fn draw(seed: u64, id: EntityId, stream: u64) -> f64 {
    let identity = (u64::from(id.index) << 32) | u64::from(id.generation);
    crate::toxin::herbivore_type_draw(seed ^ SALT, identity, 0, stream)
}

/// Body index used by the birth counters: 0 herbivore, 1 carnivore.
fn body(carnivore: bool) -> usize {
    usize::from(carnivore)
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub struct DietBook {
    /// Values of animals born to a parent (founders are drawn from the seed when read).
    value: BTreeMap<EntityId, f64>,
    /// Births by [parent body][child body] (observation only).
    births: [[u64; 2]; 2],
}

impl DietBook {
    pub(super) fn remove(&mut self, id: EntityId) {
        self.value.remove(&id);
    }

    #[cfg(feature = "state-audit")]
    pub(super) fn is_empty(&self) -> bool {
        self.value.is_empty() && self.births == [[0; 2]; 2]
    }
}

fn summary(values: &[f64]) -> Value {
    let n = values.len();
    if n == 0 {
        let empty = [0u64; 10];
        return json!({"n": 0, "mean": null, "sd": null, "hist": empty});
    }
    let mean = values.iter().sum::<f64>() / n as f64;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    let mut hist = [0u64; 10];
    for v in values {
        hist[((v * 10.) as usize).min(9)] += 1;
    }
    json!({"n": n, "mean": mean, "sd": var.sqrt(), "hist": hist})
}

impl World {
    /// Diet value of an animal; founders draw theirs from the seed.
    pub(super) fn diet_value(&self, id: EntityId, carnivore: bool) -> Option<f64> {
        let d = self.config.diet_gene.as_ref()?;
        Some(self.diet.value.get(&id).copied().unwrap_or_else(|| {
            let centre = if carnivore {
                d.initial_carnivore
            } else {
                d.initial_herbivore
            };
            let v =
                centre + d.initial_spread * (2. * draw(self.config.seed, id, FOUNDER_STREAM) - 1.);
            if carnivore {
                v.clamp(0.5, 1.)
            } else {
                v.clamp(0., HERBIVORE_MAX)
            }
        }))
    }

    /// Value of a newborn `child` of `parent`: the parent's value, moved by a normal step
    /// with the configured probability. None when the diet gene is off.
    pub(super) fn diet_child_value(
        &self,
        parent: EntityId,
        parent_carnivore: bool,
        child: EntityId,
    ) -> Option<f64> {
        let d = self.config.diet_gene.as_ref()?;
        let value = self.diet_value(parent, parent_carnivore)?;
        let seed = self.config.seed;
        if draw(seed, child, MOVE_STREAM) >= d.mutation_probability {
            return Some(value);
        }
        // Box-Muller from two stateless uniforms; 1 - u lies in (0, 1].
        let radius = (-2. * (1. - draw(seed, child, STEP_STREAM)).ln()).sqrt();
        let z = radius * (std::f64::consts::TAU * draw(seed, child, STEP_STREAM + 1)).cos();
        Some((value + d.mutation_scale * z).clamp(0., 1.))
    }

    /// Record a birth (no-op without a diet value).
    pub(super) fn diet_record_birth(
        &mut self,
        parent_carnivore: bool,
        child: EntityId,
        value: Option<f64>,
        child_carnivore: bool,
    ) {
        if let Some(v) = value {
            self.diet.value.insert(child, v);
            self.diet.births[body(parent_carnivore)][body(child_carnivore)] += 1;
        }
    }

    /// Diet observation as JSON (Issue #78): value summaries per body, the share of animals
    /// with intermediate values (0.3..0.7) and births by parent and child body.
    pub fn diet_json(&self) -> String {
        self.diet_value_json().to_string()
    }

    pub fn diet_value_json(&self) -> Value {
        if self.config.diet_gene.is_none() {
            return Value::Null;
        }
        let herbivores: Vec<f64> = self
            .animals
            .items
            .iter()
            .filter_map(|h| self.diet_value(h.id, false))
            .collect();
        let carnivores: Vec<f64> = self
            .carnivores
            .iter()
            .filter_map(|c| self.diet_value(c.id, true))
            .collect();
        let all = herbivores.len() + carnivores.len();
        let intermediate = herbivores
            .iter()
            .chain(&carnivores)
            .filter(|v| (0.3..0.7).contains(*v))
            .count();
        let share = (all > 0).then(|| intermediate as f64 / all as f64);
        let b = &self.diet.births;
        json!({
            "herbivore": summary(&herbivores),
            "carnivore": summary(&carnivores),
            "intermediate_share": share,
            "births": {
                "herbivore_to_herbivore": b[0][0],
                "herbivore_to_carnivore": b[0][1],
                "carnivore_to_carnivore": b[1][1],
                "carnivore_to_herbivore": b[1][0],
            },
        })
    }
}
