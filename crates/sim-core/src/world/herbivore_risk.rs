//! Issue #80: herbivores and the danger of carnivores.
//!
//! Observation (no behavior change): switches between fleeing and pursuing food from one Tick
//! to the next, and Ticks in danger with the share spent pursuing food, by reserve fraction.
use super::{behavior::Action, World};
use crate::entity::EntityId;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Reserve-fraction bins: below 0.4, 0.4 up to 0.7, and 0.7 or more.
fn reserve_bin(fraction: f64) -> usize {
    if fraction < 0.4 {
        0
    } else if fraction < 0.7 {
        1
    } else {
        2
    }
}

/// Observation only; not part of the state audit.
#[derive(Debug, Default)]
pub struct RiskBook {
    observe: bool,
    last: BTreeMap<EntityId, Action>,
    herbivore_ticks: u64,
    switches: u64,
    flee_ticks: u64,
    danger_ticks: [u64; 3],
    pursue_in_danger: [u64; 3],
}

impl RiskBook {
    pub(super) fn remove(&mut self, id: EntityId) {
        self.last.remove(&id);
    }
}

impl World {
    /// Start counting flee/pursue switches and pursuit in danger (observation only).
    pub fn enable_risk_observation(&mut self) {
        self.risk.observe = true;
    }

    /// Record herbivore `id`'s action this Tick (no-op unless observation is on).
    pub(super) fn observe_risk_action(
        &mut self,
        id: EntityId,
        action: Action,
        in_danger: bool,
        reserve_fraction: f64,
    ) {
        let book = &mut self.risk;
        if !book.observe {
            return;
        }
        book.herbivore_ticks += 1;
        if action == Action::Flee {
            book.flee_ticks += 1;
        }
        if let Some(previous) = book.last.insert(id, action) {
            let pair = (previous, action);
            if pair == (Action::Flee, Action::Pursue) || pair == (Action::Pursue, Action::Flee) {
                book.switches += 1;
            }
        }
        if in_danger {
            let bin = reserve_bin(reserve_fraction);
            book.danger_ticks[bin] += 1;
            if action == Action::Pursue {
                book.pursue_in_danger[bin] += 1;
            }
        }
    }

    /// Cumulative observation as JSON (null unless observation is on). Bins of
    /// `danger_ticks` and `pursue_in_danger`: reserve below 0.4, 0.4..0.7, 0.7 or more.
    pub fn risk_observation_json(&self) -> Value {
        let b = &self.risk;
        if !b.observe {
            return Value::Null;
        }
        json!({
            "herbivore_ticks": b.herbivore_ticks,
            "switches": b.switches,
            "flee_ticks": b.flee_ticks,
            "danger_ticks": b.danger_ticks,
            "pursue_in_danger": b.pursue_in_danger,
        })
    }
}
