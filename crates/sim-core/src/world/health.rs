//! Opt-in read-only HP transition and resource observations for selected individuals.
use crate::{animals::Herbivore, entity::EntityId, resources::ResourceBank};
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[derive(Debug)]
pub struct HealthTrace {
    pub watched: BTreeSet<EntityId>,
    pub window: (u64, u64),
    pub events: Vec<Value>,
    pub observe_reproduction: bool,
    sequence: u64,
}

impl HealthTrace {
    pub fn new(ids: impl IntoIterator<Item = EntityId>, window: (u64, u64)) -> Self {
        Self {
            watched: ids.into_iter().collect(),
            window,
            events: Vec::new(),
            observe_reproduction: false,
            sequence: 0,
        }
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn watches_reproduction(&self, tick: u64, id: EntityId) -> bool {
        self.observe_reproduction
            && (self.window.0..=self.window.1).contains(&tick)
            && self.watched.contains(&id)
    }

    pub fn record_reproduction(&mut self, tick: u64, id: EntityId, phase: &str, details: Value) {
        self.sequence += 1;
        self.events.push(
            json!({"seq":self.sequence,"tick":tick,"id":[id.index,id.generation],
            "phase":phase,"details":details}),
        );
    }

    pub fn record(
        &mut self,
        tick: u64,
        animal: &Herbivore,
        resources: &ResourceBank,
        phase: &str,
        hp_before: Option<f32>,
        details: Value,
    ) {
        if !self.watched.contains(&animal.id) {
            return;
        }
        let changed = hp_before.is_some_and(|hp| hp != animal.hp);
        let periodic = (self.window.0..=self.window.1).contains(&tick) && tick.is_multiple_of(10);
        let significant = details["damage"].as_f64().is_some_and(|v| v > 0.)
            || (phase == "feeding" && details["eaten_n"].as_f64().is_some_and(|v| v > 0.))
            || (phase == "resistance_upkeep"
                && resources.energy_balance(animal.reserve).unwrap_or(0.) <= 1e-9);
        if !changed && !periodic && !significant && !["birth", "pre_removal"].contains(&phase) {
            return;
        }
        self.sequence += 1;
        self.events.push(json!({
            "seq":self.sequence,"tick":tick,"phase":phase,
            "id":[animal.id.index,animal.id.generation],"hp_before":hp_before,"hp_after":animal.hp,
            "crossed_hp0":hp_before.is_some_and(|hp|hp>0. && animal.hp<=0.),
            "recovered_positive":hp_before.is_some_and(|hp|hp<=0. && animal.hp>0.),
            "age":animal.age,"max_age":animal.max_age,"adult_max_hp":animal.genes.adult_max_hp,
            "position":[animal.x,animal.y],
            "reserve_e":resources.energy_balance(animal.reserve).unwrap_or(0.),
            "reserve_n":resources.nutrient_balance(animal.reserve).unwrap_or(0.),
            "stomach_e":resources.energy_balance(animal.stomach).unwrap_or(0.),
            "stomach_n":resources.nutrient_balance(animal.stomach).unwrap_or(0.),
            "body_n":resources.nutrient_balance(animal.body).unwrap_or(0.),
            "dose":[animal.lifetime_typed_dose[0],animal.lifetime_typed_dose[1]],
            "loads":[animal.toxin_load,animal.typed_load[1]],
            "toxin_damage":animal.lifetime_toxin_damage,"upkeep_paid":animal.lifetime_resistance_cost,
            "feeding_active":animal.feeding_active,"well_fed_ticks":animal.well_fed_ticks,
            "births":animal.successful_births,"details":details
        }));
    }
}
