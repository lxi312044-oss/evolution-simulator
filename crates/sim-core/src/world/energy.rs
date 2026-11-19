//! Opt-in reserve-energy transactions; amounts come from the actual transfer requests.
use super::World;
use crate::entity::EntityId;
use serde_json::{json, Value};
use std::collections::BTreeSet;
#[derive(Debug)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub struct EnergyTrace {
    pub watched: BTreeSet<EntityId>,
    pub events: Vec<Value>,
    sequence: u64,
}
impl EnergyTrace {
    pub fn new(ids: impl IntoIterator<Item = EntityId>) -> Self {
        Self {
            watched: ids.into_iter().collect(),
            events: Vec::new(),
            sequence: 0,
        }
    }
}
impl World {
    pub(super) fn record_energy(
        &mut self,
        id: EntityId,
        phase: &str,
        before: f64,
        after: f64,
        delta: f64,
    ) {
        let health_seq = self.health_trace.as_ref().map_or(0, |t| t.sequence());
        if let Some(t) = &mut self.energy_trace {
            if t.watched.contains(&id) {
                t.sequence += 1;
                t.events.push(json!({"seq":t.sequence,"tick":self.tick,"id":[id.index,id.generation],
                    "phase":phase,"before_e":before,"after_e":after,"delta_e":delta,"health_seq":health_seq}));
            }
        }
    }
    /// End-of-tick checkpoints detect reserve changes missing from the transaction stream.
    pub fn checkpoint_energy(&mut self) {
        if self.energy_trace.is_none() {
            return;
        }
        for i in 0..self.animals.items.len() {
            let a = &self.animals.items[i];
            let e = self.resources.energy_balance(a.reserve).unwrap_or(0.);
            self.record_energy(a.id, "end_tick", e, e, 0.);
        }
    }
}
