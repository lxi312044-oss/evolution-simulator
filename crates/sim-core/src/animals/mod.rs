use crate::{entity::EntityId, resources::AccountId};
pub mod carnivore;
pub mod color;
use color::BodyColor;
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub struct HerbivoreGenes {
    pub adult_max_hp: f32,
    pub speed: f32,
    pub vision: f32,
    pub feeding_range: f32,
    pub body_color: BodyColor,
    pub fear: f32,
    pub aggression: f32,
    pub toxin_resistance: f32,
    pub typed_resistance: [f32; 16],
}
#[derive(Debug)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub struct Herbivore {
    pub id: EntityId,
    pub x: f32,
    pub y: f32,
    pub hp: f32,
    pub growth: f32,
    pub genes: HerbivoreGenes,
    pub lifetime_toxin_dose: f64,
    pub lifetime_typed_dose: [f64; 16],
    pub lifetime_toxin_damage: f64,
    pub lifetime_prevented_damage: f64,
    pub lifetime_resistance_cost: f64,
    pub reproduction_age_reached: bool,
    pub successful_births: u64,
    pub learned_fear: f32,
    pub defense_active: bool,
    pub toxin_load: f64,
    pub typed_load: [f64; 16],
    pub body: AccountId,
    pub reserve: AccountId,
    pub stomach: AccountId,
    pub feeding_active: bool,
    pub well_fed_ticks: u32,
    pub age: u32,
    pub max_age: u32,
    pub reproduction_cooldown: u32,
    pub exploration_heading: f32,
    pub exploration_ticks: u32,
    pub target: Option<EntityId>,
}
#[derive(Debug, Default)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub struct AnimalStore {
    pub items: Vec<Herbivore>,
}

// Phase 07.3.3: compatibility views; no per-animal allocations or new fields.
impl Herbivore {
    pub fn toxin_burden(&self) -> crate::toxin::ToxinBurden {
        crate::toxin::legacy_burden(self.toxin_load)
    }
    pub fn toxin_resistance(&self) -> crate::toxin::ToxinResistance {
        crate::toxin::legacy_resistance(self.genes.toxin_resistance)
    }
}

/// Basal-energy multiplier for speed upkeep: 1 + slope x (speed / reference - 1), floored
/// at 0. Exactly 1 when `slope` is 0, so the opt-in leaves basal energy unchanged.
pub fn speed_upkeep_factor(slope: f64, speed: f32, reference: f32) -> f64 {
    if slope == 0. {
        return 1.;
    }
    (1. + slope * (f64::from(speed) / f64::from(reference) - 1.)).max(0.)
}
