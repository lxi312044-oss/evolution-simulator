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
    production_energy: f64,
    resistance_energy: f64,
    pub(super) toxin_deaths: u64,
    color_skips: u64,
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

fn visible_color(color: crate::animals::color::BodyColor) -> u32 {
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
        if let Some(predator) = self.predators.remove(&id) {
            for (kind, discarded) in self.discarded.iter_mut().enumerate() {
                *discarded += predator.production.stock[kind] + predator.load[kind];
            }
        }
    }
}

impl World {
    pub(super) fn produce_herbivore_toxins(&mut self) -> Result<(), String> {
        let types = type_count(&self.config.toxin);
        for animal in &self.animals.items {
            if animal.hp <= 0. || animal.age >= animal.max_age {
                continue;
            }
            let production = self
                .animal_toxins
                .herbivores
                .entry(animal.id)
                .or_insert_with(|| Production::founder(self.config.seed, animal.id, 71));
            let available = self
                .resources
                .energy_balance(animal.reserve)
                .ok_or("missing animal reserve")?;
            let units = production.generate(types, available);
            let cost = (units.iter().sum::<f64>() * ENERGY_PER_UNIT).min(available);
            self.ledger
                .dissipate_heat(&mut self.resources, animal.reserve, cost)
                .map_err(|e| format!("herbivore toxin production: {e:?}"))?;
            self.animal_toxins.production_energy += cost;
            for (total, amount) in self.animal_toxins.produced.iter_mut().zip(units) {
                *total += amount;
            }
        }
        Ok(())
    }

    pub(super) fn inherit_herbivore_production(&mut self, parent: EntityId, child: EntityId) {
        let seed = self.config.seed;
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
            let residual = remove_fraction(stock, fraction);
            if residual > 0. {
                if let Some(memory) = self.plant_color_learning.get_mut(&prey.id) {
                    memory.observe_damage(kind, before, *stock, 0.);
                }
            }
            let produced = production
                .as_deref_mut()
                .map_or(0., |p| remove_fraction(&mut p.stock[kind], fraction));
            let dose = produced + residual;
            predator.load[kind] += dose;
            if learning && dose > 0. {
                predator.memory.ingest(color, kind, dose);
            }
            book.transferred_produced[kind] += produced;
            book.transferred_residual[kind] += residual;
        }
    }

    pub(super) fn advance_carnivore_toxin(&mut self, index: usize) -> Result<bool, String> {
        let animal = &self.carnivores[index];
        let t = &self.config.toxin;
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
            if self.config.animal_toxin.color_learning_enabled {
                predator
                    .memory
                    .observe_damage(kind, before, step.remaining_load, step.damage);
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

    pub(super) fn record_carnivore_color_skip(&mut self) {
        self.animal_toxins.color_skips += 1;
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
            "damage": book.damage, "production_energy": book.production_energy,
            "resistance_energy": book.resistance_energy, "toxin_deaths": book.toxin_deaths,
            "color_skips": book.color_skips,
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
