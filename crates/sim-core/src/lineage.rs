//! Shared read-only genealogy observation for Native and WASM.
use crate::{entity::EntityId, World};
use serde_json::{json, Value};
use std::collections::BTreeMap;
fn id(a: EntityId) -> [u32; 2] {
    [a.index, a.generation]
}
fn bin(r: f32) -> usize {
    if r >= 0.6 {
        2
    } else if r <= 0.4 {
        0
    } else {
        1
    }
}
fn emit(events: &mut Vec<Value>, row: Value) -> Result<(), String> {
    events.push(row);
    Ok(())
}
#[derive(Default)]
pub struct LineageObservation {
    target_ticks: u64,
    finished: bool,
    pub events: Vec<Value>,
    generations: BTreeMap<EntityId, u32>,
    adult_ticks: [u64; 3],
    births_by_bin: [u64; 3],
    adult_r_sum: f64,
    parent_r_sum: f64,
    child_r_sum: f64,
    births: u64,
    removals: u64,
    mutations: u64,
    max_generation: u32,
    removed_totals: [f64; 3],
    capacity_ticks: u64,
    max_n: f64,
    max_e: f64,
}
impl LineageObservation {
    fn sample(&self, w: &World) -> Value {
        let n = w.animals.items.len();
        let sum = w
            .animals
            .items
            .iter()
            .map(|a| a.genes.toxin_resistance as f64)
            .sum::<f64>();
        let mean = if n > 0 { Some(sum / n as f64) } else { None };
        let high = w
            .animals
            .items
            .iter()
            .filter(|a| a.genes.toxin_resistance >= 0.6)
            .count();
        let positive: Vec<_> = w.animals.items.iter().filter(|a| a.hp > 0.).collect();
        let positive_mean = if positive.is_empty() {
            None
        } else {
            Some(
                positive
                    .iter()
                    .map(|a| a.genes.toxin_resistance as f64)
                    .sum::<f64>()
                    / positive.len() as f64,
            )
        };
        let positive_high = positive
            .iter()
            .filter(|a| a.genes.toxin_resistance >= 0.6)
            .count();
        let live_dose = w
            .animals
            .items
            .iter()
            .map(|a| a.lifetime_typed_dose[0])
            .sum::<f64>();
        let live_damage = w
            .animals
            .items
            .iter()
            .map(|a| a.lifetime_toxin_damage)
            .sum::<f64>();
        let live_cost = w
            .animals
            .items
            .iter()
            .map(|a| a.lifetime_resistance_cost)
            .sum::<f64>();
        json!({"kind":"snapshot","tick":w.tick,"present":n,"positive":positive.len(),"plants":w.plants.len(),"carnivores":w.carnivores.len(),
   "resistance_mean":mean,"variance":mean.map(|m|w.animals.items.iter().map(|a|(a.genes.toxin_resistance as f64-m).powi(2)).sum::<f64>()/n as f64),
   "high":high,"high_fraction":if n>0{Some(high as f64/n as f64)}else{None},"positive_mean":positive_mean,
   "positive_high_fraction":if positive.is_empty(){None}else{Some(positive_high as f64/positive.len() as f64)},
   "max_generation":self.max_generation,"living_mean_generation":if n>0{Some(w.animals.items.iter().map(|a|self.generations[&a.id] as f64).sum::<f64>()/n as f64)}else{None},
   "births":self.births,"removals":self.removals,"mutation_attempts":self.mutations,"adult_ticks":self.adult_ticks,"births_by_bin":self.births_by_bin,
   "adult_resistance_sum":self.adult_r_sum,"parent_resistance_sum":self.parent_r_sum,"child_resistance_sum":self.child_r_sum,
   "dose":self.removed_totals[0]+live_dose,"damage":self.removed_totals[1]+live_damage,"upkeep":self.removed_totals[2]+live_cost,
   "germination_births":w.spontaneous_births,"protected_ticks":w.diagnostics.herbivore_plant_protection_blocked_ticks,
   "starvation_ticks":w.diagnostics.phase21_starvation_damage_ticks,"capacity_ticks":self.capacity_ticks,"max_n_error":self.max_n,"max_e_error":self.max_e})
    }
}

impl LineageObservation {
    pub fn new(w: &mut World, ticks: u64) -> Result<Self, String> {
        if w.tick != 0 || ticks == 0 {
            return Err("lineage observation must start at Tick 0 with positive horizon".into());
        }
        let mut o = Self {
            target_ticks: ticks,
            ..Default::default()
        };
        w.resistance_event_logging = true;
        w.herbivore_terminal_logging = true;
        let founders: Vec<_> = w
            .animals
            .items
            .iter()
            .map(|a| {
                o.generations.insert(a.id, 0);
                json!({"id":id(a.id),"resistance":a.genes.toxin_resistance,"age":a.age})
            })
            .collect();
        let g=w.spontaneous_germination.map(|g|json!({"soil_threshold":g.soil_threshold,"probability_per_cell":g.probability_per_cell,"check_interval":g.check_interval,"max_births_per_check":g.max_births_per_check}));
        emit(
            &mut o.events,
            json!({"kind":"initial","config":w.config,"ticks":ticks,"interval":1000,"germination":g,"founders":founders,
   "initial_state":{"plants":format!("{:?}",w.plants),"animals":format!("{:?}",w.animals),"carnivores":format!("{:?}",w.carnivores),"resources":format!("{:?}",w.resources),"rng":format!("{:?}",w.rng)}}),
        )?;
        let initial = o.sample(w);
        emit(&mut o.events, initial)?;
        Ok(o)
    }
    pub fn before_step(&mut self, w: &World) -> Result<(), String> {
        if self.finished || w.tick >= self.target_ticks {
            return Err("lineage horizon already reached".into());
        }
        let o = self;
        for a in &w.animals.items {
            if a.hp > 0. && a.age >= w.config.herbivores.reproduction_min_age && a.age < a.max_age {
                o.adult_ticks[bin(a.genes.toxin_resistance)] += 1;
                o.adult_r_sum += a.genes.toxin_resistance as f64;
            }
        }
        Ok(())
    }
    pub fn after_step(&mut self, w: &mut World) -> Result<(), String> {
        let o = self;
        if w.plants.len() + w.animals.items.len() + w.carnivores.len()
            >= w.config.world.max_living_entities as usize
        {
            o.capacity_ticks += 1;
        }
        for e in std::mem::take(&mut w.resistance_birth_events) {
            let generation = o
                .generations
                .get(&e.parent_id)
                .ok_or("missing parent generation")?
                + 1;
            if o.generations.insert(e.child_id, generation).is_some() {
                return Err("duplicate child ID".into());
            }
            o.max_generation = o.max_generation.max(generation);
            o.births += 1;
            o.births_by_bin[bin(e.parent_resistance)] += 1;
            o.parent_r_sum += e.parent_resistance as f64;
            o.child_r_sum += e.child_resistance as f64;
            o.mutations += u64::from(e.mutation_attempted);
            emit(
                &mut o.events,
                json!({"kind":"birth","tick":e.tick,"id":id(e.child_id),"parent":id(e.parent_id),"resistance":e.child_resistance,"parent_resistance":e.parent_resistance,"generation":generation,"mutation_attempted":e.mutation_attempted}),
            )?;
        }
        for e in std::mem::take(&mut w.herbivore_terminal_events) {
            o.removals += 1;
            o.removed_totals[0] += e.doses[0];
            o.removed_totals[1] += e.toxin_damage_total;
            o.removed_totals[2] += e.resistance_cost_total;
            emit(
                &mut o.events,
                json!({"kind":"removal","tick":e.tick,"id":id(e.id),"resistance":e.resistance[0],"generation":o.generations[&e.id],"births":e.successful_births,"age":e.age,"hp":e.hp,"trigger":e.removal_trigger,"dose":e.doses[0],"damage":e.toxin_damage_total,"upkeep":e.resistance_cost_total}),
            )?;
        }
        o.max_n = o
            .max_n
            .max((w.resources.total_nutrients() - w.baseline.nutrients).abs());
        o.max_e = o.max_e.max(
            ((w.resources.total_energy() - w.baseline.energy)
                - (w.ledger.sunlight_total() - w.ledger.heat_total()))
            .abs(),
        );
        if w.tick.is_multiple_of(1000) || w.tick == o.target_ticks {
            let sample = o.sample(w);
            emit(&mut o.events, sample)?;
        }
        Ok(())
    }
    pub fn finish(&mut self, w: &World) -> Result<(), String> {
        if self.finished || w.tick != self.target_ticks {
            return Err("lineage finish requires exact horizon, once".into());
        }
        let o = self;
        for a in &w.animals.items {
            emit(
                &mut o.events,
                json!({"kind":"survivor","tick":w.tick,"id":id(a.id),"generation":o.generations[&a.id],"resistance":a.genes.toxin_resistance,"births":a.successful_births,"age":a.age,"hp":a.hp,"dose":a.lifetime_typed_dose[0],"damage":a.lifetime_toxin_damage,"upkeep":a.lifetime_resistance_cost}),
            )?;
        }
        emit(
            &mut o.events,
            json!({"kind":"complete","tick":w.tick,"births":o.births,"removals":o.removals,"final_rng":format!("{:?}",w.rng),"final_ecology":w.ecology_snapshot(),"max_n_error":o.max_n,"max_e_error":o.max_e}),
        )?;
        o.finished = true;
        Ok(())
    }
    pub fn take_events(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.events)
    }
}
