use super::{plant_color_learning::PlantColorMemory, World};
use crate::{
    animals::Herbivore,
    config::ToxinSettings,
    entity::EntityId,
    toxin::{advance_burden, herbivore_type_draw},
};
use std::collections::BTreeMap;

const TYPES: usize = 16;
const CAPACITY: f64 = 1.;
const RATE: f64 = 0.01;
const ENERGY_PER_UNIT: f64 = 1.;
const INITIAL_HP: f64 = 100.;

#[derive(Debug)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
struct Production {
    ability: [f64; TYPES],
    stock: [f64; TYPES],
}

#[derive(Debug)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
struct PredatorToxin {
    production: Production,
    load: [f64; TYPES],
    resistance: [f32; TYPES],
    hp: f64,
    memory: PlantColorMemory,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub(super) struct AnimalToxins {
    herbivores: BTreeMap<EntityId, Production>,
    predators: BTreeMap<EntityId, PredatorToxin>,
    produced: [f64; TYPES],
    pub(super) plant_intake: [f64; TYPES],
    pub(super) cleared: [f64; TYPES],
    discarded: [f64; TYPES],
    transferred_produced: [f64; TYPES],
    transferred_residual: [f64; TYPES],
    damage: [f64; TYPES],
    prevented_damage: [f64; TYPES],
    peak_cleared_load: [f64; TYPES],
    damaging_type_ticks: [u64; TYPES],
    production_energy: f64,
    resistance_energy: f64,
    pub(super) toxin_deaths: u64,
    color_skips: u64,
    last_color_skip: BTreeMap<EntityId, EntityId>,
    learned_target_ticks: u64,
    learned_approach_ticks: u64,
    learned_attacks: u64,
    learned_feeds: u64,
    hunger_reapproaches: u64,
    hunger_reapproach_opportunities: u64,
    threshold_exceedance_ticks: [u64; TYPES],
    trace: Option<CarnivoreToxinTrace>,
    /// Issue #76 (opt-in): inherited toxic trait of herbivores born to a parent (founders are
    /// drawn from the seed). Empty when the trait is off, so the audited state is unchanged.
    #[cfg_attr(
        feature = "state-audit",
        serde(skip_serializing_if = "BTreeMap::is_empty")
    )]
    toxic_trait: BTreeMap<EntityId, bool>,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
struct CarnivoreToxinTrace {
    events: Vec<serde_json::Value>,
    next_decision: std::collections::BTreeSet<EntityId>,
    opportunities: bool,
    first_prey_by_color: BTreeMap<(EntityId, u32), EntityId>,
}

fn trace_id(id: EntityId) -> [u32; 2] {
    [id.index, id.generation]
}

fn type_count(t: &ToxinSettings) -> usize {
    if t.herbivore_multi_toxin_enabled {
        usize::from(t.plant_toxin_type_count)
    } else {
        1
    }
}

fn trait_draw(seed: u64, id: EntityId, kind: usize, stream: u64) -> f64 {
    let identity = (u64::from(id.index) << 32) | u64::from(id.generation);
    herbivore_type_draw(seed, identity, kind as u16, stream)
}

fn inherited(
    value: f64,
    seed: u64,
    id: EntityId,
    kind: usize,
    stream: u64,
    t: &ToxinSettings,
) -> f64 {
    if trait_draw(seed, id, kind, stream) < t.resistance_mutation_probability {
        let step = 2. * trait_draw(seed, id, kind, stream + 1) - 1.;
        (value + step * t.resistance_mutation_scale).clamp(0., 1.)
    } else {
        value
    }
}

impl Production {
    fn founder(seed: u64, id: EntityId, stream: u64) -> Self {
        Self {
            ability: std::array::from_fn(|kind| trait_draw(seed, id, kind, stream)),
            stock: [0.; TYPES],
        }
    }

    fn offspring(&self, seed: u64, id: EntityId, stream: u64, t: &ToxinSettings) -> Self {
        Self {
            ability: std::array::from_fn(|kind| {
                inherited(self.ability[kind], seed, id, kind, stream, t)
            }),
            stock: [0.; TYPES],
        }
    }

    fn generate(&mut self, types: usize, available_e: f64) -> [f64; TYPES] {
        let mut units = [0.; TYPES];
        for (kind, unit) in units.iter_mut().enumerate().take(types) {
            *unit = (RATE * self.ability[kind]).min((CAPACITY - self.stock[kind]).max(0.));
        }
        let cost = units.iter().sum::<f64>() * ENERGY_PER_UNIT;
        let scale = if cost > available_e {
            available_e / cost
        } else {
            1.
        };
        for (unit, stock) in units.iter_mut().zip(&mut self.stock) {
            *unit *= scale;
            *stock += *unit;
        }
        units
    }
}

impl PredatorToxin {
    fn founder(seed: u64, id: EntityId, t: &ToxinSettings) -> Self {
        Self {
            production: Production::founder(seed, id, 83),
            load: [0.; TYPES],
            resistance: [t.initial_resistance as f32; TYPES],
            hp: INITIAL_HP,
            memory: Default::default(),
        }
    }
}

pub(super) fn visible_color(color: crate::animals::color::BodyColor) -> u32 {
    let [r, g, b] = color.rgb().map(|value| (value * 255.).round() as u32);
    (r << 24) | (g << 16) | (b << 8)
}

fn remove_fraction(stock: &mut f64, fraction: f64) -> f64 {
    let moved = *stock * fraction;
    *stock -= moved;
    moved
}

impl AnimalToxins {
    pub(super) fn remove_herbivore(&mut self, animal: &Herbivore) {
        self.toxic_trait.remove(&animal.id);
        let production = self.herbivores.remove(&animal.id);
        for (kind, discarded) in self.discarded.iter_mut().enumerate() {
            let residual = if kind == 0 {
                animal.toxin_load
            } else {
                animal.typed_load[kind]
            };
            *discarded += residual + production.as_ref().map_or(0., |p| p.stock[kind]);
        }
    }

    pub(super) fn remove_carnivore(&mut self, id: EntityId) {
        self.last_color_skip.remove(&id);
        if let Some(predator) = self.predators.remove(&id) {
            for (kind, discarded) in self.discarded.iter_mut().enumerate() {
                *discarded += predator.production.stock[kind] + predator.load[kind];
            }
        }
    }
}

impl World {
    /// Issue #76 (opt-in): whether herbivore `id` makes toxin. Always true when the trait is off.
    /// Founders are drawn from a stateless stream; offspring inherit with rare flips.
    pub(super) fn herbivore_is_toxic(&self, id: EntityId) -> bool {
        match self.config.animal_toxin.herbivore_toxic_initial_fraction {
            None => true,
            Some(fraction) => self
                .animal_toxins
                .toxic_trait
                .get(&id)
                .copied()
                .unwrap_or_else(|| trait_draw(self.config.seed, id, 0, 95) < fraction),
        }
    }

    pub(super) fn produce_herbivore_toxins(&mut self) -> Result<(), String> {
        let types = type_count(&self.config.toxin);
        let decay = self.config.animal_toxin.herbivore_toxin_decay_per_tick;
        let mut decayed = [0.; TYPES];
        for animal in &self.animals.items {
            if animal.hp <= 0. || animal.age >= animal.max_age {
                continue;
            }
            let toxic = self.herbivore_is_toxic(animal.id);
            let production = self
                .animal_toxins
                .herbivores
                .entry(animal.id)
                .or_insert_with(|| Production::founder(self.config.seed, animal.id, 71));
            // Issue #74 (opt-in): stored toxin breaks down, so holding it needs synthesis.
            if decay > 0. {
                for (stock, lost) in production.stock.iter_mut().zip(&mut decayed).take(types) {
                    *lost += *stock * decay;
                    *stock *= 1. - decay;
                }
            }
            let available = self
                .resources
                .energy_balance(animal.reserve)
                .ok_or("missing animal reserve")?;
            // Issue #76 (opt-in): non-toxic herbivores make nothing and pay nothing.
            let units = if toxic {
                production.generate(types, available)
            } else {
                [0.; TYPES]
            };
            let cost = (units.iter().sum::<f64>() * ENERGY_PER_UNIT).min(available);
            self.ledger
                .dissipate_heat(&mut self.resources, animal.reserve, cost)
                .map_err(|e| format!("herbivore toxin production: {e:?}"))?;
            self.animal_toxins.production_energy += cost;
            for (total, amount) in self.animal_toxins.produced.iter_mut().zip(units) {
                *total += amount;
            }
        }
        // Broken-down toxin is accounted as cleared, keeping the toxin balance closed.
        for (cleared, lost) in self.animal_toxins.cleared.iter_mut().zip(decayed) {
            *cleared += lost;
        }
        Ok(())
    }

    /// Read-only: strongest toxin production ability over active types (observation only;
    /// falls back to the stateless founder draw the lazy initialisation would use).
    pub(super) fn herbivore_toxin_ability(&self, id: EntityId) -> f64 {
        let types = type_count(&self.config.toxin);
        let founder;
        let production = match self.animal_toxins.herbivores.get(&id) {
            Some(p) => p,
            None => {
                founder = Production::founder(self.config.seed, id, 71);
                &founder
            }
        };
        production.ability[..types]
            .iter()
            .copied()
            .fold(0., f64::max)
    }

    /// Read-only: current toxin stock summed over active types (0 if never produced).
    pub(super) fn herbivore_toxin_stock(&self, id: EntityId) -> f64 {
        let types = type_count(&self.config.toxin);
        self.animal_toxins
            .herbivores
            .get(&id)
            .map_or(0., |p| p.stock[..types].iter().sum())
    }

    /// Read-only: mean toxin resistance over active types for a carnivore (display only).
    pub(super) fn carnivore_resistance_mean(&self, id: EntityId) -> f32 {
        let types = type_count(&self.config.toxin);
        match self.animal_toxins.predators.get(&id) {
            Some(p) => p.resistance[..types].iter().sum::<f32>() / types as f32,
            None => self.config.toxin.initial_resistance as f32,
        }
    }

    /// Read-only counterpart of `herbivore_toxin_ability` for carnivores.
    pub(super) fn carnivore_toxin_ability(&self, id: EntityId) -> f64 {
        let types = type_count(&self.config.toxin);
        let founder;
        let production = match self.animal_toxins.predators.get(&id) {
            Some(p) => &p.production,
            None => {
                founder = Production::founder(self.config.seed, id, 83);
                &founder
            }
        };
        production.ability[..types]
            .iter()
            .copied()
            .fold(0., f64::max)
    }

    pub(super) fn inherit_herbivore_production(&mut self, parent: EntityId, child: EntityId) {
        let seed = self.config.seed;
        if self
            .config
            .animal_toxin
            .herbivore_toxic_initial_fraction
            .is_some()
        {
            let flip = trait_draw(seed, child, 0, 97)
                < self
                    .config
                    .animal_toxin
                    .herbivore_toxic_mutation_probability;
            let toxic = self.herbivore_is_toxic(parent) != flip;
            self.animal_toxins.toxic_trait.insert(child, toxic);
        }
        let states = &mut self.animal_toxins.herbivores;
        let production = states
            .entry(parent)
            .or_insert_with(|| Production::founder(seed, parent, 71))
            .offspring(seed, child, 73, &self.config.toxin);
        states.insert(child, production);
    }

    pub(super) fn inherit_carnivore_toxin(&mut self, parent: EntityId, child: EntityId) {
        let seed = self.config.seed;
        let t = &self.config.toxin;
        let states = &mut self.animal_toxins.predators;
        let parent = &states[&parent];
        let child_state = PredatorToxin {
            production: parent.production.offspring(seed, child, 85, t),
            resistance: std::array::from_fn(|kind| {
                inherited(f64::from(parent.resistance[kind]), seed, child, kind, 87, t) as f32
            }),
            ..PredatorToxin::founder(seed, child, t)
        };
        states.insert(child, child_state);
    }

    pub(super) fn transfer_animal_toxin(
        &mut self,
        prey_index: usize,
        predator_index: usize,
        body_n: f64,
        eaten_n: f64,
    ) {
        if body_n <= 0. || eaten_n <= 0. {
            return;
        }
        let fraction = (eaten_n / body_n).clamp(0., 1.);
        let plant_threshold = self.config.animal_toxin.plant_toxin_transfer_threshold;
        // Issue #76 diagnosis (observation only): group of the prey for dose accounting.
        let dose_group = self
            .mimicry
            .choice_observation
            .then(|| self.mimicry_attack_group(prey_index));
        let prey = &mut self.animals.items[prey_index];
        let id = self.carnivores[predator_index].id;
        let t = &self.config.toxin;
        let book = &mut self.animal_toxins;
        let predator = book
            .predators
            .entry(id)
            .or_insert_with(|| PredatorToxin::founder(self.config.seed, id, t));
        let mut production = book.herbivores.get_mut(&prey.id);
        let learning = self.config.animal_toxin.color_learning_enabled;
        let color = visible_color(prey.genes.body_color);
        for kind in 0..type_count(t) {
            let stock = if kind == 0 {
                &mut prey.toxin_load
            } else {
                &mut prey.typed_load[kind]
            };
            let before = *stock;
            // Issue #76 (opt-in): a small plant-derived load stays in the herbivore.
            let residual = if plant_threshold <= 0. || before >= plant_threshold {
                remove_fraction(stock, fraction)
            } else {
                0.
            };
            if residual > 0. {
                if let Some(memory) = self.plant_color_learning.get_mut(&prey.id) {
                    memory.observe_damage(kind, before, *stock, 0.);
                }
            }
            let produced = production
                .as_deref_mut()
                .map_or(0., |p| remove_fraction(&mut p.stock[kind], fraction));
            let dose = produced + residual;
            if let Some(g) = dose_group {
                self.mimicry.dose_produced[g] += produced;
                self.mimicry.dose_plant[g] += residual;
            }
            let previous_load = predator.load[kind];
            predator.load[kind] += dose;
            if dose > 0. {
                if let Some(trace) = &mut book.trace {
                    if trace.opportunities {
                        trace
                            .first_prey_by_color
                            .entry((id, color))
                            .or_insert(prey.id);
                    }
                    trace
                        .events
                        .push(serde_json::json!({"event":"transfer","tick":self.tick,
                        "id":trace_id(id),"prey":trace_id(prey.id),"rgb":color,"type":kind,
                        "body_n":body_n,"eaten_n":eaten_n,"produced":produced,"residual":residual,
                        "load_before":previous_load,"load_after":predator.load[kind]}));
                }
            }
            if learning && dose > 0. {
                // Issue #76 diagnosis (observation only): memory turnover.
                let (created, evicts_harm) = predator.memory.ingest_effect(color);
                self.mimicry.memory_entries_created += u64::from(created);
                self.mimicry.harmful_evictions += u64::from(evicts_harm);
                predator.memory.ingest(color, kind, dose);
            }
            book.transferred_produced[kind] += produced;
            book.transferred_residual[kind] += residual;
        }
    }

    pub(super) fn advance_carnivore_toxin(&mut self, index: usize) -> Result<bool, String> {
        let animal = &self.carnivores[index];
        let mut settings = self.config.toxin.clone();
        if let Some(threshold) = self.config.animal_toxin.carnivore_damage_threshold {
            settings.damage_threshold = threshold;
        }
        if let Some(coefficient) = self.config.animal_toxin.carnivore_hp_damage_per_excess_unit {
            settings.hp_damage_per_excess_unit = coefficient;
        }
        let t = &settings;
        let types = type_count(t);
        let book = &mut self.animal_toxins;
        let predator = book
            .predators
            .entry(animal.id)
            .or_insert_with(|| PredatorToxin::founder(self.config.seed, animal.id, t));
        let mut damage = 0.;
        for kind in 0..types {
            let before = predator.load[kind];
            let step = advance_burden(before, predator.resistance[kind], t);
            predator.load[kind] = step.remaining_load;
            book.cleared[kind] += step.cleared;
            book.damage[kind] += f64::from(step.damage);
            book.prevented_damage[kind] += step.prevented_damage;
            book.peak_cleared_load[kind] = book.peak_cleared_load[kind].max(step.remaining_load);
            book.damaging_type_ticks[kind] += u64::from(step.damage > 0.);
            book.threshold_exceedance_ticks[kind] +=
                u64::from(step.remaining_load > t.damage_threshold);
            let trace_damage = step.damage > 0. && book.trace.is_some();
            let memory_before = trace_damage.then(|| predator.memory.harm_scores());
            if self.config.animal_toxin.color_learning_enabled {
                match self.config.animal_toxin.carnivore_learning_gain {
                    None => predator.memory.observe_damage(
                        kind,
                        before,
                        step.remaining_load,
                        step.damage,
                    ),
                    // Issue #76 (opt-in): learn more strongly from the same poisoning.
                    Some(gain) => predator.memory.observe_damage_with_gain(
                        kind,
                        before,
                        step.remaining_load,
                        step.damage,
                        gain,
                    ),
                }
            }
            if let Some(memory_before) = memory_before {
                let memory_after = predator.memory.harm_scores();
                let trace = book.trace.as_mut().unwrap();
                if memory_before != memory_after {
                    trace.next_decision.insert(animal.id);
                }
                trace
                    .events
                    .push(serde_json::json!({"event":"damage","tick":self.tick,
                    "id":trace_id(animal.id),"type":kind,"load_before_clearance":before,
                    "load_after_clearance":step.remaining_load,"damage":step.damage,
                    "prevented":step.prevented_damage,"hp_before":(predator.hp - damage).max(0.),
                    "hp_after":(predator.hp - damage - f64::from(step.damage)).max(0.),
                    "memory_before":memory_before,"memory_after":memory_after}));
            }
            if let Some(trace) = book.trace.as_mut().filter(|trace| trace.opportunities) {
                if before > 0. {
                    trace.events.push(serde_json::json!({"event":"clearance","tick":self.tick,
                        "id":trace_id(animal.id),"type":kind,"before":before,"after":step.remaining_load}));
                }
            }
            damage += f64::from(step.damage);
        }
        predator.hp = (predator.hp - damage).max(0.);
        if predator.hp == 0. {
            return Ok(true);
        }
        if animal.age >= animal.max_age {
            return Ok(false);
        }
        let available = self
            .resources
            .energy_balance(animal.reserve)
            .ok_or("missing animal reserve")?;
        let upkeep = if t.resistance_enabled {
            let first = f64::from(predator.resistance[0]);
            let extra = predator.resistance[1..types]
                .iter()
                .map(|&r| f64::from(r))
                .sum::<f64>();
            let multiplier = if t.herbivore_multi_toxin_enabled {
                t.multi_type_resistance_upkeep_multiplier
            } else {
                1.
            };
            ((first + extra * t.additional_type_resistance_upkeep_multiplier)
                * t.resistance_energy_cost_per_tick
                * multiplier)
                .min(available)
        } else {
            0.
        };
        let units = predator.production.generate(types, available - upkeep);
        let production_cost = (units.iter().sum::<f64>() * ENERGY_PER_UNIT).min(available - upkeep);
        self.ledger
            .dissipate_heat(
                &mut self.resources,
                animal.reserve,
                (upkeep + production_cost).min(available),
            )
            .map_err(|e| format!("carnivore toxin costs: {e:?}"))?;
        book.resistance_energy += upkeep;
        book.production_energy += production_cost;
        for (total, amount) in book.produced.iter_mut().zip(units) {
            *total += amount;
        }
        Ok(false)
    }

    pub(super) fn carnivore_allows_color(
        &self,
        id: EntityId,
        color: crate::animals::color::BodyColor,
        reserve_e: f64,
    ) -> bool {
        !self.config.animal_toxin.color_learning_enabled
            || self
                .animal_toxins
                .predators
                .get(&id)
                .is_none_or(|p| p.memory.allows(visible_color(color), reserve_e / 60.))
    }

    /// Issue #76 diagnosis (read-only): color-memory summary over living carnivores.
    /// `avoids_own_harm_color` = the largest learned harm exceeds the current risk tolerance,
    /// i.e. the carnivore would refuse at least the exact color it learned from.
    pub(super) fn carnivore_memory_summary(&self) -> serde_json::Value {
        let mut n = 0u64;
        let (mut entries, mut harmful, mut max_harm, mut tolerance, mut hp, mut load) =
            (0., 0., 0., 0., 0., 0.);
        let mut avoiding = 0u64;
        let types = type_count(&self.config.toxin);
        for c in &self.carnivores {
            let Some(p) = self.animal_toxins.predators.get(&c.id) else {
                continue;
            };
            let scores = p.memory.harm_scores();
            let top = scores.iter().map(|&(_, s)| s).fold(0., f64::max);
            let reserve = self.resources.energy_balance(c.reserve).unwrap_or(0.);
            let tol = super::plant_color_learning::risk_tolerance(reserve / 60.);
            n += 1;
            entries += p.memory.entry_count() as f64;
            harmful += scores.len() as f64;
            max_harm += top;
            tolerance += tol;
            hp += p.hp;
            load += p.load[..types].iter().sum::<f64>();
            avoiding += u64::from(top * 10. > tol);
        }
        let mean = |v: f64| (n > 0).then(|| v / n as f64);
        serde_json::json!({
            "carnivores_with_state": n,
            "mean_entries": mean(entries),
            "mean_harmful_entries": mean(harmful),
            "mean_max_harm": mean(max_harm),
            "mean_tolerance": mean(tolerance),
            "mean_hp": mean(hp),
            "mean_load": mean(load),
            "avoids_own_harm_color": mean(avoiding as f64),
        })
    }

    /// Issue #76 diagnosis (read-only): the risk a carnivore's color memory assigns to a color
    /// (0 without learning or memory). Compared with the risk tolerance in prey choice.
    pub(super) fn carnivore_color_risk(
        &self,
        id: EntityId,
        color: crate::animals::color::BodyColor,
    ) -> f64 {
        if !self.config.animal_toxin.color_learning_enabled {
            return 0.;
        }
        self.animal_toxins
            .predators
            .get(&id)
            .map_or(0., |p| p.memory.color_risk(visible_color(color)))
    }

    /// A counterattack teaches the carnivore that this prey color is harmful.
    pub(super) fn learn_counterattack(
        &mut self,
        predator_index: usize,
        prey_index: usize,
        harm: f64,
    ) {
        let id = self.carnivores[predator_index].id;
        let color = visible_color(self.animals.items[prey_index].genes.body_color);
        let t = &self.config.toxin;
        self.animal_toxins
            .predators
            .entry(id)
            .or_insert_with(|| PredatorToxin::founder(self.config.seed, id, t))
            .memory
            .observe_injury(color, harm);
    }

    /// Opt-in forgetting of a carnivore's learned color harm.
    pub(super) fn decay_carnivore_color_memory(&mut self, predator_index: usize, rate: f64) {
        let id = self.carnivores[predator_index].id;
        if let Some(predator) = self.animal_toxins.predators.get_mut(&id) {
            predator.memory.decay(rate);
        }
    }

    pub(super) fn record_carnivore_color_skips(
        &mut self,
        predator: EntityId,
        prey: EntityId,
        count: u64,
    ) {
        self.animal_toxins.color_skips += count;
        self.animal_toxins.last_color_skip.insert(predator, prey);
    }

    pub(super) fn observe_carnivore_color_target(
        &mut self,
        predator: EntityId,
        prey: EntityId,
        color: crate::animals::color::BodyColor,
        reserve_e: f64,
        moved: f32,
    ) -> bool {
        let book = &mut self.animal_toxins;
        let color = visible_color(color);
        let Some(state) = book.predators.get(&predator) else {
            return false;
        };
        if !state.memory.has_harm_for_color(color) {
            return false;
        }
        book.learned_target_ticks += 1;
        let hunger_opportunity = book.last_color_skip.get(&predator) == Some(&prey)
            && !state.memory.allows(color, 1.)
            && state.memory.allows(color, reserve_e / 60.);
        book.hunger_reapproach_opportunities += u64::from(hunger_opportunity);
        if moved > 0. {
            book.learned_approach_ticks += 1;
            if hunger_opportunity {
                book.hunger_reapproaches += 1;
                book.last_color_skip.remove(&predator);
            }
        }
        if let Some(trace) = &mut book.trace {
            trace.events.push(
                serde_json::json!({"event":"learned_target","tick":self.tick,
                "id":trace_id(predator),"prey":trace_id(prey),"rgb":color,
                "reserve_e":reserve_e,"moved":moved,"hunger_opportunity":hunger_opportunity}),
            );
        }
        true
    }

    pub(super) fn observe_carnivore_learned_attack(&mut self, fed: bool) {
        self.animal_toxins.learned_attacks += 1;
        self.animal_toxins.learned_feeds += u64::from(fed);
    }

    pub fn set_carnivore_toxin_trace(&mut self, enabled: bool) {
        self.animal_toxins.trace = enabled.then(CarnivoreToxinTrace::default);
    }

    #[cfg(feature = "state-audit")]
    pub fn audit_without_carnivore_trace(&mut self) -> Vec<u8> {
        let trace = self.animal_toxins.trace.take();
        let bytes = self.audit_state(false, None);
        self.animal_toxins.trace = trace;
        bytes
    }

    pub fn enable_carnivore_opportunity_trace(&mut self) {
        self.set_carnivore_toxin_trace(true);
        self.animal_toxins.trace.as_mut().unwrap().opportunities = true;
    }

    pub(super) fn observes_carnivore_opportunities(&self) -> bool {
        self.animal_toxins
            .trace
            .as_ref()
            .is_some_and(|trace| trace.opportunities)
    }

    pub(super) fn carnivore_opportunity_snapshot(
        &self,
        index: usize,
        selected: Option<usize>,
        satiated: bool,
        hunger_override: bool,
    ) -> Option<serde_json::Value> {
        if !self.observes_carnivore_opportunities() {
            return None;
        }
        let pred = &self.carnivores[index];
        let c = &self.config.carnivores;
        let e = self.resources.energy_balance(pred.reserve).unwrap_or(0.);
        let stomach_n = self.resources.nutrient_balance(pred.stomach).unwrap_or(0.);
        let stomach_e = self.resources.energy_balance(pred.stomach).unwrap_or(0.);
        let full = c.all_bites_to_stomach
            && (stomach_e >= c.stomach_capacity_energy - 1e-9
                || stomach_n >= c.stomach_capacity_nutrients - 1e-9);
        let refuge =
            c.prey_refuge_enabled && self.animals.items.len() <= c.prey_refuge_threshold as usize;
        let ready = !satiated && !full && !refuge && pred.attack_cooldown == 0;
        let scores = self
            .animal_toxins
            .predators
            .get(&pred.id)
            .map_or_else(Vec::new, |p| p.memory.harm_scores());
        let mut visible = [0_u64; 3]; // plant-derived, produced, either
        let mut same_colors = BTreeMap::<u32, [u64; 5]>::new();
        let mut same_color_distance = BTreeMap::<u32, f32>::new();
        let mut learned_ready = Vec::new();
        let tolerance = super::plant_color_learning::risk_tolerance(e / 60.);
        let mut closest = None;
        let mut closest_d2 = pred.vision * pred.vision;
        for prey in &self.animals.items {
            let d2 = (prey.x - pred.x).powi(2) + (prey.y - pred.y).powi(2);
            if prey.hp <= 0.
                || d2 >= pred.vision * pred.vision
                || self.resources.nutrient_balance(prey.body).unwrap_or(0.) <= 0.
            {
                continue;
            }
            let plant = prey.toxin_load > 0. || prey.typed_load.iter().any(|v| *v > 0.);
            let produced = self
                .animal_toxins
                .herbivores
                .get(&prey.id)
                .is_some_and(|p| p.stock.iter().any(|v| *v > 0.));
            visible[0] += u64::from(plant);
            visible[1] += u64::from(produced);
            visible[2] += u64::from(plant || produced);
            if d2 < closest_d2 {
                closest_d2 = d2;
                closest = Some(prey.id);
            }
            let color = visible_color(prey.genes.body_color);
            if ready && d2 <= c.attack_range * c.attack_range {
                let risk = scores
                    .iter()
                    .filter_map(|&(key, score)| {
                        let similarity = (1.
                            - super::plant_color_learning::color_distance(color, key) / 0.25)
                            .max(0.);
                        (similarity > 0.).then_some((key, score * 10. * similarity))
                    })
                    .max_by(|a, b| a.1.total_cmp(&b.1));
                if let Some((memory_color, risk)) = risk {
                    let original = self
                        .animal_toxins
                        .trace
                        .as_ref()
                        .unwrap()
                        .first_prey_by_color
                        .get(&(pred.id, memory_color))
                        == Some(&prey.id);
                    learned_ready.push(serde_json::json!({
                        "prey":trace_id(prey.id), "color":color, "memory_color":memory_color,
                        "original":original, "distance":d2.sqrt(), "risk":risk,
                        "rejected":!self.carnivore_allows_color(pred.id, prey.genes.body_color, e),
                    }));
                }
            }
            if scores.iter().any(|&(key, _)| key == color) {
                same_color_distance
                    .entry(color)
                    .and_modify(|d| *d = d.min(d2.sqrt()))
                    .or_insert_with(|| d2.sqrt());
                let row = same_colors.entry(color).or_default();
                row[0] += 1;
                row[4] += u64::from(
                    self.animal_toxins
                        .trace
                        .as_ref()
                        .unwrap()
                        .first_prey_by_color
                        .get(&(pred.id, color))
                        == Some(&prey.id),
                );
                let attackable = ready && d2 <= c.attack_range * c.attack_range;
                row[1] += u64::from(attackable);
                let rejected = !self.carnivore_allows_color(pred.id, prey.genes.body_color, e);
                row[2] += u64::from(rejected);
                row[3] += u64::from(attackable && rejected);
            }
        }
        let target = selected.map(|j| {
            let p = &self.animals.items[j];
            serde_json::json!({"id":trace_id(p.id),"color":visible_color(p.genes.body_color),
                "distance_before":(p.x-pred.x).hypot(p.y-pred.y)})
        });
        Some(
            serde_json::json!({"event":"opportunity","tick":self.tick,"id":trace_id(pred.id),
            "visible":visible,"same_colors":same_colors,"same_color_distance":same_color_distance,"target":target,
            "closest_edible_without_color":closest.map(trace_id),"reserve_e":e,"vision":pred.vision,
            "learned_ready":learned_ready,"tolerance":tolerance,
            "attack_range":c.attack_range,"cooldown":pred.attack_cooldown,"satiated":satiated,
            "hunger_override":hunger_override,"stomach_full":full,"refuge":refuge}),
        )
    }

    pub(super) fn record_carnivore_opportunity(&mut self, row: serde_json::Value) {
        self.animal_toxins.trace.as_mut().unwrap().events.push(row);
    }

    pub(super) fn trace_carnivore_toxin_removal(&mut self, id: EntityId, reason: &'static str) {
        let book = &mut self.animal_toxins;
        if let Some(trace) = book.trace.as_mut().filter(|t| t.opportunities) {
            let load = book
                .predators
                .get(&id)
                .map(|p| p.load)
                .unwrap_or([0.; TYPES]);
            trace
                .events
                .push(serde_json::json!({"event":"removal","tick":self.tick,
                "id":trace_id(id),"load":load,"reason":reason}));
        }
    }

    pub fn take_carnivore_toxin_trace(&mut self) -> Vec<serde_json::Value> {
        self.animal_toxins
            .trace
            .as_mut()
            .map_or_else(Vec::new, |trace| std::mem::take(&mut trace.events))
    }

    pub(super) fn trace_carnivore_toxin_decision(
        &mut self,
        predator: EntityId,
        selected: Option<EntityId>,
        skipped: Option<EntityId>,
        reserve_e: f64,
        satiated: bool,
        attacked: bool,
    ) {
        if let Some(trace) = &mut self.animal_toxins.trace {
            let next = trace.next_decision.remove(&predator);
            if next || skipped.is_some() {
                trace.events.push(serde_json::json!({"event":"decision","tick":self.tick,
                    "id":trace_id(predator),"selected":selected.map(trace_id),"skipped":skipped.map(trace_id),
                    "reserve_e":reserve_e,"satiated":satiated,"attacked":attacked,"after_memory_update":next}));
            }
        }
    }

    /// Read-only individual state for the Web inspector.
    pub fn inspect_entity(&self, index: u32, generation: u32) -> serde_json::Value {
        use serde_json::json;
        let id = EntityId { index, generation };
        let n = |account| self.resources.nutrient_balance(account);
        let e = |account| self.resources.energy_balance(account);
        let rgb = |color: u32| [24, 16, 8].map(|shift| ((color >> shift) & 255) as f64 / 255.);
        let types = type_count(&self.config.toxin);
        if let Some(p) = self.plants.items.iter().find(|p| p.id == id) {
            return json!({"kind":"plant","id":[index,generation],"position":[p.x,p.y],"rgb":rgb(p.color),
                "age":p.age,"max_age":p.max_age,"body_n":n(p.body),"reserve_n":n(p.reserve),
                "reserve_e":e(p.reserve),"growth":p.growth,"seed_cooldown":p.seed_cooldown,
                "toxic":self.config.toxin.enabled && p.produces_toxin,"toxin_type":p.toxin_type.0,
                "toxin_stock":null});
        }
        if let Some(a) = self.animals.items.iter().find(|a| a.id == id) {
            let h = &self.config.herbivores;
            let maximum = f64::from(a.genes.adult_max_hp) * h.max_energy_per_hp;
            let reserve = e(a.reserve);
            let mut load = a.typed_load;
            load[0] = a.toxin_load;
            let mut resistance = a.genes.typed_resistance;
            resistance[0] = a.genes.toxin_resistance;
            let memory = self.plant_color_learning.get(&id);
            return json!({"kind":"herbivore","id":[index,generation],"position":[a.x,a.y],
                "rgb":a.genes.body_color.rgb(),"age":a.age,"max_age":a.max_age,
                "hp":a.hp,"adult_hp_max":a.genes.adult_max_hp,"body_n":n(a.body),
                "reserve_n":n(a.reserve),"reserve_e":reserve,"reserve_e_max":maximum,
                "stomach_n":n(a.stomach),"stomach_n_max":h.satiety_enabled.then_some(h.stomach_capacity_n),
                "load":load[..types],"resistance":resistance[..types],
                "toxin_stock":self.animal_toxins.herbivores.get(&id).map(|p| &p.stock[..types]),
                "target":a.target.map(trace_id),"feeding_active":h.satiety_enabled.then_some(a.feeding_active),
                "reproduction_cooldown":a.reproduction_cooldown,"well_fed_ticks":a.well_fed_ticks,
                "births":a.successful_births,"dose":a.lifetime_toxin_dose,"damage":a.lifetime_toxin_damage,
                "tolerance":reserve.filter(|_| h.color_learning_enabled).map(|v|
                    super::plant_color_learning::risk_tolerance(v / maximum)),
                "memory":memory.map(|m| m.harm_scores().into_iter().map(|(c, harm)|
                    json!({"rgb":rgb(c),"harm":harm,"risk_same_color":harm*10.})).collect::<Vec<_>>())});
        }
        if let Some(a) = self.carnivores.iter().find(|a| a.id == id) {
            let c = &self.config.carnivores;
            let toxin = self.animal_toxins.predators.get(&id);
            let reserve = e(a.reserve);
            return json!({"kind":"carnivore","id":[index,generation],"position":[a.x,a.y],"rgb":a.color.rgb(),
                "age":a.age,"max_age":a.max_age,"parent":a.parent_id.map(trace_id),
                "hp":toxin.map(|p|p.hp),"adult_hp_max":self.config.animal_toxin.enabled.then_some(INITIAL_HP),
                "body_n":n(a.body),"reserve_n":n(a.reserve),"reserve_e":reserve,
                "stomach_n":n(a.stomach),"stomach_e":e(a.stomach),
                "stomach_n_max":c.digestion_enabled.then_some(c.stomach_capacity_nutrients),
                "stomach_e_max":c.digestion_enabled.then_some(c.stomach_capacity_energy),
                "load":toxin.map(|p|&p.load[..types]),"resistance":toxin.map(|p|&p.resistance[..types]),
                "toxin_stock":toxin.map(|p|&p.production.stock[..types]),
                "last_target":a.p42.previous_target.map(trace_id),"feeding_active":c.digestion_enabled.then_some(a.feeding_active),
                "reproduction_cooldown":a.cooldown,"attack_cooldown":a.attack_cooldown,
                "births":a.p42.births_as_parent,
                "tolerance":reserve.filter(|_|self.config.animal_toxin.color_learning_enabled).map(|v|
                    super::plant_color_learning::risk_tolerance(v / 60.)),
                "memory":toxin.map(|p| p.memory.harm_scores().into_iter().map(|(c,harm)|
                    json!({"rgb":rgb(c),"harm":harm,"risk_same_color":harm*10.})).collect::<Vec<_>>())});
        }
        serde_json::Value::Null
    }

    pub fn animal_toxin_observation_state(&self) -> String {
        format!("{:?}", self.animal_toxins)
    }

    /// Read-only accounting for the opt-in animal toxin pathway.
    pub fn animal_toxin_snapshot_json(&self) -> String {
        let book = &self.animal_toxins;
        let mut stock = [0.; TYPES];
        for prey in &self.animals.items {
            stock[0] += prey.toxin_load;
            for (kind, total) in stock.iter_mut().enumerate().skip(1) {
                *total += prey.typed_load[kind];
            }
        }
        for p in book.herbivores.values() {
            for (total, amount) in stock.iter_mut().zip(p.stock) {
                *total += amount;
            }
        }
        for p in book.predators.values() {
            for (kind, total) in stock.iter_mut().enumerate() {
                *total += p.production.stock[kind] + p.load[kind];
            }
        }
        let error: [f64; TYPES] = std::array::from_fn(|kind| {
            book.plant_intake[kind] + book.produced[kind]
                - book.cleared[kind]
                - book.discarded[kind]
                - stock[kind]
        });
        serde_json::json!({
            "stock": stock, "balance_error": error, "produced": book.produced,
            "plant_intake": book.plant_intake, "cleared": book.cleared, "discarded": book.discarded,
            "transferred_produced": book.transferred_produced,
            "transferred_residual": book.transferred_residual,
            "damage": book.damage, "prevented_damage": book.prevented_damage,
            "peak_cleared_load": book.peak_cleared_load,
            "damaging_type_ticks": book.damaging_type_ticks,
            "production_energy": book.production_energy,
            "resistance_energy": book.resistance_energy, "toxin_deaths": book.toxin_deaths,
            "color_skips": book.color_skips,
            "learned_target_ticks": book.learned_target_ticks,
            "learned_approach_ticks": book.learned_approach_ticks,
            "learned_attacks": book.learned_attacks,
            "learned_feeds": book.learned_feeds,
            "hunger_reapproaches": book.hunger_reapproaches,
            "hunger_reapproach_opportunities": book.hunger_reapproach_opportunities,
            "threshold_exceedance_ticks": book.threshold_exceedance_ticks,
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SimulationConfig;

    fn world() -> World {
        let mut cfg = SimulationConfig::default();
        cfg.initial_population.plants = 0;
        cfg.initial_population.herbivores = 1;
        cfg.initial_population.carnivores = 1;
        cfg.carnivores.enabled = true;
        cfg.animal_toxin.enabled = true;
        cfg.toxin.clearance_fraction_per_tick = 0.;
        cfg.toxin.damage_threshold = 0.;
        cfg.toxin.hp_damage_per_excess_unit = 1.;
        World::new(cfg).unwrap()
    }

    fn close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
    }

    #[test]
    fn generation_limits_stock_and_prorates_energy_without_n() {
        let mut w = world();
        let n = w.resources.total_nutrients();
        let e = w.resources.total_energy();
        w.produce_herbivore_toxins().unwrap();
        assert!(w.animal_toxins.produced[0] > 0.);
        close(w.resources.total_nutrients(), n);
        close(e - w.resources.total_energy(), w.ledger.heat_total());
        close(w.ledger.heat_total(), w.animal_toxins.production_energy);
        let mut production = Production {
            ability: [1.; TYPES],
            stock: [0.; TYPES],
        };
        let units = production.generate(2, 0.008);
        close(units[0], 0.004);
        close(units[1], 0.004);
        production.stock[0] = CAPACITY;
        close(production.generate(1, 1.)[0], 0.);
        close(production.generate(2, 0.)[1], 0.);
    }

    #[test]
    fn moves_both_stocks_using_each_transactions_remaining_body() {
        let mut w = world();
        w.produce_herbivore_toxins().unwrap();
        let prey = w.animals.items[0].id;
        let predator = w.carnivores[0].id;
        w.animal_toxins.herbivores.get_mut(&prey).unwrap().stock[0] = 4.;
        w.animals.items[0].toxin_load = 2.;
        w.transfer_animal_toxin(0, 0, 10., 0.);
        assert!(w.animal_toxins.predators.is_empty());
        w.transfer_animal_toxin(0, 0, 10., 2.);
        close(w.animal_toxins.herbivores[&prey].stock[0], 3.2);
        close(w.animals.items[0].toxin_load, 1.6);
        close(w.animal_toxins.predators[&predator].load[0], 1.2);
        w.transfer_animal_toxin(0, 0, 8., 2.);
        close(w.animal_toxins.predators[&predator].load[0], 2.4);
        w.transfer_animal_toxin(0, 0, 6., 6.);
        close(w.animal_toxins.predators[&predator].load[0], 6.);
        close(w.animal_toxins.herbivores[&prey].stock[0], 0.);
        close(w.animals.items[0].toxin_load, 0.);
    }

    #[test]
    fn carnivore_damage_coefficient_is_an_optional_species_override() {
        let measure = |override_coefficient| {
            let mut w = world();
            w.config.toxin.hp_damage_per_excess_unit = 0.01;
            w.config.toxin.resistance_enabled = false;
            w.config.toxin.damage_threshold = 0.;
            w.config.toxin.clearance_fraction_per_tick = 0.;
            w.config.animal_toxin.carnivore_hp_damage_per_excess_unit = override_coefficient;
            let id = w.carnivores[0].id;
            let mut state = PredatorToxin::founder(w.config.seed, id, &w.config.toxin);
            state.load[0] = 2.;
            w.animal_toxins.predators.insert(id, state);
            assert!(!w.advance_carnivore_toxin(0).unwrap());
            assert_eq!(w.config.toxin.hp_damage_per_excess_unit, 0.01);
            assert_eq!(w.animal_toxins.predators[&id].load[0], 2.);
            w.animal_toxins.damage[0]
        };
        assert_eq!(measure(None), measure(Some(0.01)));
        assert_eq!(measure(None), f64::from(0.02_f32));
        assert_eq!(measure(Some(0.50)), 1.);
    }

    #[test]
    fn typed_resistance_and_poison_death_use_existing_removal() {
        let mut w = world();
        w.config.toxin.herbivore_multi_toxin_enabled = true;
        w.config.toxin.plant_toxin_type_count = 2;
        w.config.toxin.resistance_enabled = true;
        w.config.toxin.resistance_effectiveness = 1.;
        let id = w.carnivores[0].id;
        let mut state = PredatorToxin::founder(w.config.seed, id, &w.config.toxin);
        state.resistance[0] = 1.;
        state.resistance[1] = 0.;
        state.production.stock[1] = 0.5;
        state.load[0] = 20.;
        state.load[1] = 101.;
        w.animal_toxins.predators.insert(id, state);
        let n = w.resources.total_nutrients();
        let e = w.resources.total_energy();
        w.update_carnivores().unwrap();
        assert!(w.carnivores.is_empty());
        assert_eq!(w.carnivore_births, 0);
        assert_eq!(w.animal_toxins.toxin_deaths, 1);
        close(w.animal_toxins.damage[0], 0.);
        close(w.animal_toxins.damage[1], 101.);
        close(w.animal_toxins.discarded[1], 101.5);
        close(w.resources.total_nutrients(), n);
        close(e - w.resources.total_energy(), w.ledger.heat_total());
    }

    #[test]
    fn learned_color_changes_real_attack_even_when_next_prey_is_safe() {
        use crate::animals::color::BodyColor;
        let red = BodyColor::new(1., 0., 0.);
        let near = BodyColor::new(0.95, 0., 0.);
        let blue = BodyColor::new(0., 0., 1.);
        for learning in [false, true] {
            let mut w = world();
            w.config.animal_toxin.color_learning_enabled = learning;
            w.config.carnivores.reproduction_enabled = false;
            w.config.carnivores.bite_nutrients = 0.1;
            w.config.carnivores.bite_energy = 0.;
            w.config.carnivores.attack_range = 10.;
            w.config.carnivores.basal_energy_per_tick = 0.;
            w.config.carnivores.movement_energy_per_unit = 0.;
            w.config.carnivores.attack_cooldown_ticks = 0;
            w.carnivores[0].vision = 20.;
            w.animals.items[0].x = w.carnivores[0].x;
            w.animals.items[0].y = w.carnivores[0].y;
            w.animals.items[0].genes.body_color = red;
            let body = w.animals.items[0].body;
            let body_n = w.resources.nutrient_balance(body).unwrap();
            assert!(body_n > 0.2);
            w.animals.items[0].toxin_load = body_n;
            let id = w.carnivores[0].id;
            assert!(w.carnivore_allows_color(id, red, 60.));
            w.update_carnivores().unwrap();
            assert_eq!(w.carnivore_debug.hunt_attempts, 1);
            assert!(w.animal_toxins.predators[&id].hp < INITIAL_HP);
            assert_eq!(w.carnivore_allows_color(id, near, 60.), !learning);
            assert!(w.carnivore_allows_color(id, blue, 60.));
            assert!(w.carnivore_allows_color(id, red, 0.));
            let reserve = w.carnivores[0].reserve;
            let fixture_energy = w.resources.open(0., 60.).unwrap();
            w.resources
                .transfer_energy(fixture_energy, reserve, 60.)
                .unwrap();
            w.animals.items[0].toxin_load = 0.;
            w.animals.items[0].hp = 100.;
            w.update_carnivores().unwrap();
            assert_eq!(
                w.carnivore_debug.hunt_attempts,
                if learning { 1 } else { 2 }
            );
            assert_eq!(w.animal_toxins.color_skips, u64::from(learning));
        }
    }

    #[test]
    fn descendants_inherit_traits_without_toxin_health_or_memory_state() {
        let mut w = world();
        w.config.toxin.resistance_mutation_probability = 0.;
        let id = w.carnivores[0].id;
        let mut parent = PredatorToxin::founder(w.config.seed, id, &w.config.toxin);
        parent.hp = 1.;
        parent.load[0] = 10.;
        parent.production.stock[0] = 0.5;
        parent.resistance[1] = 0.8;
        parent.memory.ingest(0xff000000, 0, 10.);
        parent.memory.observe_damage(0, 10., 10., 0.1);
        let expected_ability = parent.production.ability;
        w.animal_toxins.predators.insert(id, parent);
        let child = w.entities.spawn().unwrap();
        w.inherit_carnivore_toxin(id, child);
        let state = &w.animal_toxins.predators[&child];
        assert_eq!(state.production.ability, expected_ability);
        assert_eq!(state.resistance[1], 0.8);
        assert_eq!(state.hp, INITIAL_HP);
        assert_eq!(state.production.stock, [0.; TYPES]);
        assert_eq!(state.load, [0.; TYPES]);
        assert!(state.memory.allows(0xff000000, 1.));
    }

    #[test]
    fn lethal_bonus_uses_post_bite_body_and_both_stocks() {
        let mut w = world();
        w.config.carnivores.reproduction_enabled = false;
        w.config.carnivores.satiety_enabled = true;
        w.config.carnivores.bite_nutrients = 2.;
        w.config.carnivores.kill_bonus_nutrients = 2.;
        w.config.carnivores.attack_range = 10.;
        w.carnivores[0].vision = 20.;
        w.animals.items[0].x = w.carnivores[0].x;
        w.animals.items[0].y = w.carnivores[0].y;
        w.animals.items[0].hp = 1.;
        let prey = w.animals.items[0].id;
        let body_n = w
            .resources
            .nutrient_balance(w.animals.items[0].body)
            .unwrap();
        assert!(body_n > 4.);
        w.produce_herbivore_toxins().unwrap();
        w.animal_toxins.herbivores.get_mut(&prey).unwrap().stock[0] = 1.;
        w.animals.items[0].toxin_load = 2.;
        w.update_carnivores().unwrap();
        assert_eq!(w.carnivore_debug.kills_with_meal, 1);
        close(w.animal_toxins.transferred_produced[0], 4. / body_n);
        close(w.animal_toxins.transferred_residual[0], 8. / body_n);
        close(w.animal_toxins.herbivores[&prey].stock[0], 1. - 4. / body_n);
        close(w.animals.items[0].toxin_load, 2. - 8. / body_n);
    }

    #[test]
    fn damage_after_a_later_meal_is_assigned_to_the_earlier_color_too() {
        use crate::animals::color::BodyColor;
        let mut w = world();
        w.config.animal_toxin.color_learning_enabled = true;
        w.config.toxin.damage_threshold = 1.;
        let id = w.carnivores[0].id;
        let red = BodyColor::new(1., 0., 0.);
        let blue = BodyColor::new(0., 0., 1.);
        w.animals.items[0].genes.body_color = red;
        w.animals.items[0].toxin_load = 0.8;
        w.transfer_animal_toxin(0, 0, 1., 1.);
        w.advance_carnivore_toxin(0).unwrap();
        close(w.animal_toxins.predators[&id].hp, INITIAL_HP);
        assert!(w.carnivore_allows_color(id, red, 60.));

        w.animals.items[0].genes.body_color = blue;
        w.animals.items[0].toxin_load = 0.4;
        w.transfer_animal_toxin(0, 0, 1., 1.);
        w.advance_carnivore_toxin(0).unwrap();
        assert!(!w.carnivore_allows_color(id, red, 60.));
        assert!(!w.carnivore_allows_color(id, blue, 60.));
    }
}

#[cfg(all(test, feature = "state-audit"))]
#[path = "animal_toxin_encounter_test.rs"]
mod encounter_test;
