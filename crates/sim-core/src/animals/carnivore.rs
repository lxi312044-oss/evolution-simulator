use crate::{animals::color::BodyColor, entity::EntityId, resources::AccountId};
#[derive(Debug)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub struct Carnivore {
    pub id: EntityId,
    pub x: f32,
    pub y: f32,
    pub age: u32,
    pub max_age: u32,
    pub cooldown: u32,
    pub attack_cooldown: u32,
    pub satiety_ticks: u32,
    pub stomach: AccountId,
    pub feeding_active: bool,
    pub body: AccountId,
    pub reserve: AccountId,
    pub color: BodyColor,
    pub speed: f32,
    pub vision: f32,
    /// Heritable propensity to wait when prey is not visible (0=roam, 1=ambush).
    pub ambush_gene: f32,
    // Phase 42: per-individual observational counters; never influence decisions.
    pub p42: CarnivoreTrace,
    /// Phase 43: parent lineage (None for initial individuals).
    pub parent_id: Option<EntityId>,
    pub search_heading: f32,
    pub search_ticks_left: u32,
    pub last_prey_heading: f32,
    pub last_prey_ticks_left: u32,
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub struct CarnivoreTrace {
    pub visible_ticks: u64,
    pub approach_ticks: u64,
    pub retreat_ticks: u64,
    pub attack_range_ticks: u64,
    pub attacks: u64,
    pub positive_energy_attacks: u64,
    pub last_positive_attack_tick: Option<u64>,
    pub births_as_parent: u64,
    pub basal_spent: f64,
    pub movement_spent: f64,
    pub digestion_received: f64,
    pub bite_energy_received: f64,
    pub reproduction_energy_spent: f64,
    /// Same prey across consecutive ticks: includes prey movement between predator updates.
    pub previous_target: Option<EntityId>,
    pub previous_target_distance: f32,
    pub relative_closer: u64,
    pub relative_farther: u64,
    pub relative_unchanged: u64,
    pub longest_no_positive_attack_ticks: u64,
    pub low_energy_visible_ticks: u64,
}
