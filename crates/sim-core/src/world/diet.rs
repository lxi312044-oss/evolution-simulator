//! Issue #78 (opt-in): a heritable diet value from 0 (herbivore) to 1 (carnivore) on every
//! animal. Values come from a stateless stream keyed by the entity id, so carrying them never
//! consumes ecology RNG. With `diet_gene.effects` off the value is observed only.
//!
//! With `effects` on (stage 1): an offspring whose value is below 0.5 is born a herbivore and
//! one at or above 0.5 a carnivore, from either parent body. Herbivores gain x (1 - value)^k
//! from plants and carnivores x value^k from bites; the rest goes where uneaten food already
//! goes (nutrients to the soil detritus, energy to heat). Each animal carries the genes of the
//! other body without using them, so a lineage that crosses back gets them again.
use super::World;
use crate::{
    animals::{Herbivore, HerbivoreGenes},
    entity::EntityId,
    resources::AccountId,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const SALT: u64 = 0x6469_6574_5f67_656e;
const FOUNDER_STREAM: u64 = 1;
const MOVE_STREAM: u64 = 2;
const STEP_STREAM: u64 = 4;
const FOUNDER_GENE_STREAM: u64 = 8;
const AGE_STREAM: u64 = 16;
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

/// Carnivore genes a herbivore carries without using them.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub(super) struct DormantCarnivore {
    pub(super) speed: f32,
    pub(super) ambush_gene: f32,
}

/// A carnivore's offspring that is born a herbivore (stage 1).
pub(super) struct HerbivoreFromCarnivore {
    pub(super) id: EntityId,
    pub(super) value: f64,
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) color: crate::animals::color::BodyColor,
    pub(super) carnivore: DormantCarnivore,
    pub(super) body: AccountId,
    pub(super) reserve: AccountId,
    pub(super) stomach: AccountId,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub struct DietBook {
    /// Values of animals born to a parent (founders are drawn from the seed when read).
    value: BTreeMap<EntityId, f64>,
    /// Herbivore genes carried by carnivores (stage 1).
    dormant_herbivore: BTreeMap<EntityId, HerbivoreGenes>,
    /// Carnivore genes carried by herbivores (stage 1).
    dormant_carnivore: BTreeMap<EntityId, DormantCarnivore>,
    /// Births by [parent body][child body] (observation only).
    births: [[u64; 2]; 2],
    /// Feeding gains lost to the diet value by [herbivore, carnivore] x [nutrients, energy]
    /// (observation only).
    lost: [[f64; 2]; 2],
    /// Parent values of births that cross the body boundary, by direction (0 herbivore to
    /// carnivore, 1 carnivore to herbivore): sum and 10-bin histogram (observation only).
    crossing_parent_sum: [f64; 2],
    crossing_parent_hist: [[u64; 10]; 2],
}

impl DietBook {
    pub(super) fn remove(&mut self, id: EntityId) {
        self.value.remove(&id);
        self.dormant_herbivore.remove(&id);
        self.dormant_carnivore.remove(&id);
    }

    #[cfg(feature = "state-audit")]
    pub(super) fn is_empty(&self) -> bool {
        self.value.is_empty()
            && self.dormant_herbivore.is_empty()
            && self.dormant_carnivore.is_empty()
            && self.births == [[0; 2]; 2]
            && self.lost == [[0.; 2]; 2]
            && self.crossing_parent_sum == [0.; 2]
            && self.crossing_parent_hist == [[0; 10]; 2]
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

    /// Record a birth (no-op without a diet value). Births that cross the body boundary also
    /// record the parent's value.
    pub(super) fn diet_record_birth(
        &mut self,
        parent: EntityId,
        parent_carnivore: bool,
        child: EntityId,
        value: Option<f64>,
        child_carnivore: bool,
    ) {
        let Some(v) = value else {
            return;
        };
        if parent_carnivore != child_carnivore {
            if let Some(p) = self.diet_value(parent, parent_carnivore) {
                let direction = body(parent_carnivore);
                self.diet.crossing_parent_sum[direction] += p;
                self.diet.crossing_parent_hist[direction][((p * 10.) as usize).min(9)] += 1;
            }
        }
        self.diet.value.insert(child, v);
        self.diet.births[body(parent_carnivore)][body(child_carnivore)] += 1;
    }

    /// Whether the diet value decides bodies and feeding gains (stage 1).
    pub(super) fn diet_effects(&self) -> bool {
        self.config.diet_gene.as_ref().is_some_and(|d| d.effects)
    }

    /// Feeding-gain factor: (1 - value)^k for a herbivore, value^k for a carnivore; exactly 1
    /// when the value has no effects.
    pub(super) fn diet_factor(&self, id: EntityId, carnivore: bool) -> f64 {
        let (Some(d), Some(v)) = (&self.config.diet_gene, self.diet_value(id, carnivore)) else {
            return 1.;
        };
        if !d.effects {
            return 1.;
        }
        if carnivore {
            v.powf(d.efficiency_exponent)
        } else {
            (1. - v).powf(d.efficiency_exponent)
        }
    }

    /// Record plant gains a herbivore lost to its diet value (observation only).
    pub(super) fn diet_record_herbivore_loss(&mut self, nutrients: f64, energy: f64) {
        self.diet.lost[0][0] += nutrients;
        self.diet.lost[0][1] += energy;
    }

    /// After carnivore `index` moved `n` nutrients and `e` energy into `target`, give up the
    /// share its diet value does not gain: nutrients to the detritus of its cell, energy to heat.
    pub(super) fn diet_lose_meal(
        &mut self,
        index: usize,
        target: AccountId,
        n: f64,
        e: f64,
    ) -> Result<(), String> {
        let factor = self.diet_factor(self.carnivores[index].id, true);
        if factor >= 1. {
            return Ok(());
        }
        let (lost_n, lost_e) = (n * (1. - factor), e * (1. - factor));
        let (x, y) = (self.carnivores[index].x, self.carnivores[index].y);
        let cell = self
            .land
            .cell_index(x, y, self.config.world.width, self.config.world.height);
        let detritus = self.land.cells[cell].detritus;
        if lost_n > 0. {
            self.resources
                .transfer_nutrients(target, detritus, lost_n)
                .map_err(|err| format!("diet meal N: {err:?}"))?;
        }
        if lost_e > 0. {
            self.ledger
                .dissipate_heat(&mut self.resources, target, lost_e)
                .map_err(|err| format!("diet meal E: {err:?}"))?;
        }
        self.diet.lost[1][0] += lost_n;
        self.diet.lost[1][1] += lost_e;
        Ok(())
    }

    pub(super) fn diet_dormant_herbivore(&self, id: EntityId) -> Option<HerbivoreGenes> {
        self.diet.dormant_herbivore.get(&id).copied()
    }

    pub(super) fn diet_set_dormant_herbivore(&mut self, id: EntityId, genes: HerbivoreGenes) {
        self.diet.dormant_herbivore.insert(id, genes);
    }

    pub(super) fn diet_dormant_carnivore(&self, id: EntityId) -> Option<DormantCarnivore> {
        self.diet.dormant_carnivore.get(&id).copied()
    }

    pub(super) fn diet_set_dormant_carnivore(&mut self, id: EntityId, genes: DormantCarnivore) {
        self.diet.dormant_carnivore.insert(id, genes);
    }

    /// Herbivore genes for a lineage without carried herbivore genes (founder carnivores):
    /// the founder herbivore ranges, drawn from the stateless stream.
    fn diet_founder_herbivore_genes(&self, id: EntityId) -> HerbivoreGenes {
        let h = &self.config.herbivores;
        let u = |k: u64| draw(self.config.seed, id, FOUNDER_GENE_STREAM + k) as f32;
        let resistance = self.config.toxin.initial_resistance as f32;
        HerbivoreGenes {
            adult_max_hp: 45. + 30. * u(0),
            speed: h.speed_min + (h.speed_max - h.speed_min) * u(1),
            vision: h.vision_min + (h.vision_max - h.vision_min) * u(2),
            feeding_range: h.feeding_range,
            body_color: crate::animals::color::initial(self.config.seed, u64::from(id.index)),
            fear: 0.45 + 0.4 * u(3),
            aggression: 0.15 + 0.6 * u(4),
            toxin_resistance: resistance,
            typed_resistance: [resistance; 16],
        }
    }

    /// Stage 1: carnivore `parent` has an offspring whose value is below 0.5; it is born a
    /// herbivore with the herbivore genes the parent carried and the parent's resources for
    /// the offspring.
    pub(super) fn diet_birth_herbivore_from_carnivore(
        &mut self,
        parent: usize,
        child: HerbivoreFromCarnivore,
    ) {
        let parent_id = self.carnivores[parent].id;
        let mut genes = self
            .diet_dormant_herbivore(parent_id)
            .unwrap_or_else(|| self.diet_founder_herbivore_genes(child.id));
        genes.body_color = child.color;
        let h = &self.config.herbivores;
        let max_age = ((h.max_age_ticks as f64)
            * (0.9 + 0.2 * draw(self.config.seed, child.id, AGE_STREAM)))
        .round()
        .max(1.) as u32;
        let exploration_heading =
            (std::f64::consts::TAU * draw(self.config.seed, child.id, AGE_STREAM + 1)) as f32;
        let reproduction_cooldown = h.reproduction_cooldown_ticks;
        self.animals.items.push(Herbivore {
            id: child.id,
            x: child.x,
            y: child.y,
            hp: genes.adult_max_hp * 0.75,
            growth: 0.,
            genes,
            lifetime_toxin_dose: 0.,
            lifetime_typed_dose: [0.; 16],
            lifetime_toxin_damage: 0.,
            lifetime_prevented_damage: 0.,
            lifetime_resistance_cost: 0.,
            reproduction_age_reached: false,
            successful_births: 0,
            learned_fear: 0.,
            defense_active: false,
            toxin_load: 0.,
            typed_load: [0.; 16],
            body: child.body,
            reserve: child.reserve,
            stomach: child.stomach,
            feeding_active: true,
            well_fed_ticks: 0,
            age: 0,
            max_age,
            reproduction_cooldown,
            exploration_heading,
            exploration_ticks: 30,
            target: None,
        });
        let index = self.animals.items.len() - 1;
        if self.config.animal_toxin.enabled {
            self.inherit_herbivore_production(parent_id, child.id);
            if self.has_predator_toxin(parent_id) {
                self.inherit_carnivore_toxin(parent_id, child.id);
            }
        }
        self.diet_set_dormant_carnivore(child.id, child.carnivore);
        self.species_herbivore_birth_from_carnivore(index, parent);
        self.diet_record_birth(parent_id, true, child.id, Some(child.value), false);
        self.apex_record_birth(parent_id, child.id);
        self.births += 1;
        self.diagnostics.herbivore_births += 1;
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
        let lost = &self.diet.lost;
        let crossing = |direction: usize| {
            let hist = self.diet.crossing_parent_hist[direction];
            let n: u64 = hist.iter().sum();
            let mean = (n > 0).then(|| self.diet.crossing_parent_sum[direction] / n as f64);
            json!({"n": n, "mean": mean, "hist": hist})
        };
        json!({
            "crossing_parents": {
                "herbivore_to_carnivore": crossing(0),
                "carnivore_to_herbivore": crossing(1),
            },
            "lost": {
                "herbivore_nutrients": lost[0][0],
                "herbivore_energy": lost[0][1],
                "carnivore_nutrients": lost[1][0],
                "carnivore_energy": lost[1][1],
            },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::DietGeneSettings, SimulationConfig};

    /// A quarter of the current standard world with the diet value acting.
    fn acting(change: impl Fn(&mut SimulationConfig, &mut DietGeneSettings)) -> World {
        let mut c: SimulationConfig = serde_json::from_str(include_str!(
            "../../../../config/stage4-standard-20261120.json"
        ))
        .unwrap();
        c.world.width = 1000.;
        c.world.height = 1000.;
        c.world.land_columns = 20;
        c.world.land_rows = 20;
        c.initial_population.plants = 300;
        c.initial_population.herbivores = 80;
        c.initial_population.carnivores = 1;
        c.herbivores.carnivore_offspring_probability = 0.;
        let mut d = DietGeneSettings {
            effects: true,
            ..Default::default()
        };
        change(&mut c, &mut d);
        c.diet_gene = Some(d);
        World::new(c).unwrap()
    }

    /// Herbivore genes a carnivore carried come back when its offspring is born a herbivore;
    /// without carried genes the founder herbivore ranges are used.
    #[test]
    fn carried_herbivore_genes_return() {
        let mut w = acting(|_, _| {});
        let parent = w.carnivores[0].id;
        let carried = HerbivoreGenes {
            adult_max_hp: 199.,
            speed: 7.75,
            vision: 149.,
            feeding_range: 9.,
            body_color: crate::animals::color::BodyColor::new(0., 0., 0.),
            fear: 0.01,
            aggression: 0.99,
            toxin_resistance: 0.5,
            typed_resistance: [0.5; 16],
        };
        w.diet_set_dormant_herbivore(parent, carried);
        let color = crate::animals::color::BodyColor::new(0.9, 0.1, 0.2);
        let child = |w: &mut World| {
            let id = w.entities.spawn().unwrap();
            let [body, reserve, stomach] = [(); 3].map(|_| w.resources.open(0., 0.).unwrap());
            HerbivoreFromCarnivore {
                id,
                value: 0.3,
                x: 10.,
                y: 20.,
                color,
                carnivore: DormantCarnivore {
                    speed: 6.5,
                    ambush_gene: 0.25,
                },
                body,
                reserve,
                stomach,
            }
        };
        let first = child(&mut w);
        let id = first.id;
        let before = w.animals.items.len();
        w.diet_birth_herbivore_from_carnivore(0, first);
        assert_eq!(w.animals.items.len(), before + 1);
        let h = w.animals.items.last().unwrap();
        assert_eq!(h.id, id);
        assert_eq!(h.genes.adult_max_hp, 199.);
        assert_eq!(h.genes.speed, 7.75);
        assert_eq!(h.genes.aggression, 0.99);
        assert_eq!(h.genes.body_color.rgb(), color.rgb());
        assert_eq!(h.hp, 199. * 0.75);
        assert_eq!(w.diet_value(id, false), Some(0.3));
        let back = w.diet_dormant_carnivore(id).unwrap();
        assert_eq!((back.speed, back.ambush_gene), (6.5, 0.25));
        assert_eq!(w.diet.births[1][0], 1);
        // A carnivore without carried genes: founder herbivore ranges.
        w.diet.dormant_herbivore.clear();
        let second = child(&mut w);
        w.diet_birth_herbivore_from_carnivore(0, second);
        let g = w.animals.items.last().unwrap().genes;
        assert!((45. ..=75.).contains(&g.adult_max_hp));
        assert!((0.45..=0.85).contains(&g.fear));
    }

    /// Carnivore genes the herbivores carried come back when their offspring are born
    /// carnivores (speed evolution is off, so the speed is otherwise the configured one).
    #[test]
    fn carried_carnivore_genes_return() {
        let mut w = acting(|c, d| {
            c.initial_population.carnivores = 0;
            d.initial_herbivore = 0.45;
            d.initial_spread = 0.04;
            d.mutation_probability = 1.;
            d.mutation_scale = 0.1;
        });
        let ids: Vec<EntityId> = w.animals.items.iter().map(|h| h.id).collect();
        for id in ids {
            w.diet_set_dormant_carnivore(
                id,
                DormantCarnivore {
                    speed: 7.5,
                    ambush_gene: 0.99,
                },
            );
        }
        for _ in 0..10 {
            w.step(100).unwrap();
        }
        assert!(w.diet.births[0][1] > 0, "the test needs crossings");
        assert!(!w.carnivores.is_empty());
        for c in &w.carnivores {
            assert_eq!(c.speed, 7.5);
            if c.parent_id.is_none() {
                assert_eq!(c.ambush_gene, 0.99);
            }
            assert!(w.diet_value(c.id, true).unwrap() >= 0.5);
            assert!(w.diet_dormant_herbivore(c.id).is_some());
        }
        for h in &w.animals.items {
            assert!(w.diet_value(h.id, false).unwrap() < 0.5);
        }
    }
}
