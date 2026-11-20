//! Issue #80: herbivores and the danger of carnivores.
//!
//! Observation (no behavior change): switches between fleeing and pursuing food from one Tick
//! to the next, and Ticks in danger with the share spent pursuing food, by reserve fraction.
//!
//! Opt-in behavior (`herbivore_risk`): a herbivore remembers where it last saw a carnivore,
//! keeps treating that place as danger until it is far enough away, avoids plants near it
//! unless hunger outweighs its fear, and goes for food instead of fleeing when it accepts the
//! danger where it stands. Fear is the inherited fear gene plus learned fear; nothing here
//! draws random numbers or moves an animal for free.
use super::{behavior::Action, World};
use crate::{config::HerbivoreRiskSettings, entity::EntityId};
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

/// Where a herbivore last saw a carnivore, and for how many more unseen Ticks it remembers it.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub(super) struct DangerMemory {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) ticks_left: u32,
}

/// Whether a herbivore with this hunger (0 full .. 1 empty) and fear (0..1) accepts danger
/// `danger` (0 at the edge of the danger radius .. 1 at the carnivore).
pub(super) fn accepts(r: &HerbivoreRiskSettings, hunger: f64, fear: f32, danger: f32) -> bool {
    r.hunger_weight * hunger * f64::from(1. - fear) >= f64::from(danger) * f64::from(fear)
}

/// Danger of (x, y) from `point` within `radius`: 1 at the point, 0 at the radius and beyond.
pub(super) fn danger_at(x: f32, y: f32, point: (f32, f32), radius: f32) -> f32 {
    if radius <= 0. {
        return 0.;
    }
    let (dx, dy) = (x - point.0, y - point.1);
    (1. - (dx * dx + dy * dy).sqrt() / radius).clamp(0., 1.)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunger_and_fear_decide_the_accepted_danger() {
        let r = HerbivoreRiskSettings::default();
        // A full herbivore accepts no danger; a starving bold one accepts it all.
        assert!(!accepts(&r, 0., 0.6, 0.01));
        assert!(accepts(&r, 0., 0.6, 0.));
        assert!(accepts(&r, 1., 0.4, 1.));
        // The same hunger accepts less danger with more fear.
        assert!(accepts(&r, 0.5, 0.45, 0.5));
        assert!(!accepts(&r, 0.5, 0.85, 0.5));
        // Hunger weight 0: hunger never outweighs fear.
        let timid = HerbivoreRiskSettings {
            hunger_weight: 0.,
            ..Default::default()
        };
        assert!(!accepts(&timid, 1., 0.4, 0.1));
    }

    #[test]
    fn danger_falls_to_zero_at_the_radius() {
        assert_eq!(danger_at(0., 0., (0., 0.), 50.), 1.);
        assert_eq!(danger_at(25., 0., (0., 0.), 50.), 0.5);
        assert_eq!(danger_at(80., 0., (0., 0.), 50.), 0.);
        assert_eq!(danger_at(0., 0., (0., 0.), 0.), 0.);
    }
}
