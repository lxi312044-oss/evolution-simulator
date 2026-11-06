mod state;
mod tick;
pub use state::{World,SpontaneousGermination};

mod behavior;
mod herbivore_tick;
mod carnivore_tick;

mod diagnostics;
pub use diagnostics::Diagnostics;

/// Observation-only genealogy event for herbivore resistance.
#[derive(Debug,Clone)]
pub struct ResistanceBirthEvent {
 pub tick:u64, pub parent_id:crate::entity::EntityId, pub child_id:crate::entity::EntityId,
 pub parent_resistance:f32, pub child_resistance:f32, pub mutation_attempted:bool,
}

/// A successful birth. Mutation flags record RNG decisions, even if clamping
/// makes the resulting channel equal to the parent value.
#[derive(Debug,Clone,PartialEq)]
pub struct ColorBirthEvent {
    pub tick:u64, pub parent_id:crate::entity::EntityId, pub child_id:crate::entity::EntityId,
    pub parent_rgb:[f32;3], pub child_rgb:[f32;3], pub mutated:[bool;3],
}
#[derive(Debug,Clone,PartialEq)]
pub struct ColorStatistics {
    pub population:u64, pub mean:[f64;3], pub variance:[f64;3],
    pub inheritances:u64, pub mutated_births:u64, pub mutated_channels:u64,
}

/// Phase 42: immutable snapshot of a carnivore at death or final tick.
#[derive(Debug,Clone,serde::Serialize)]
pub struct CarnivoreLifeEvent {
 pub tick:u64, pub event:&'static str, pub id_index:u32, pub id_generation:u32,
 pub age:u32, pub ambush_gene:f32, pub reserve_energy:f64, pub stomach_energy:f64,
 pub parent_id_index:Option<u32>, pub parent_id_generation:Option<u32>,
 pub relative_closer:u64, pub relative_farther:u64, pub relative_unchanged:u64,
 pub longest_no_positive_attack_ticks:u64, pub low_energy_visible_ticks:u64,
 pub nearest_prey_distance:Option<f32>, pub visible_prey_count:usize,
 pub attack_range_prey_count:usize, pub feeding_active:bool,
 pub ticks_since_last_positive_attack:Option<u64>,
 pub visible_ticks:u64, pub approach_ticks:u64, pub retreat_ticks:u64,
 pub attack_range_ticks:u64, pub attacks:u64, pub positive_energy_attacks:u64,
 pub births_as_parent:u64, pub basal_spent:f64, pub movement_spent:f64,
 pub digestion_received:f64, pub bite_energy_received:f64, pub reproduction_energy_spent:f64,
}
/// Rolling herbivore population and demographic sample; no resource accounts are modified.
#[derive(Debug,Default,Clone)]
pub struct PreyTrendSnapshot {
 pub tick:u64, pub population:usize, pub births:u64, pub deaths:u64,
 pub decline_fraction:f64, pub births_in_window:u64, pub deaths_in_window:u64,
}

/// Cumulative, read-only diagnostics for the predator/prey balance investigation.
#[derive(Debug,Default,Clone,serde::Serialize)]
pub struct CarnivoreDebug {
 // Phase 06.2.38: cumulative counts, bins [0,.2), ... [.8,1].
 pub gene_births:[u64;5], pub gene_parent_reproductions:[u64;5], pub gene_deaths:[u64;5],
 pub gene_ambush_choices:[u64;5], pub gene_search_choices:[u64;5],
 pub gene_mutations:[u64;5],
 // Phase 39: read-only diagnostics (cumulative individual-ticks).
 pub prey_near_but_outside_vision:u64, pub prey_visible_while_low_energy:u64,
 pub prey_near_while_satiated:u64, pub predator_low_energy_no_visible_prey:u64,
 pub herbivore_flee_with_plant_visible:u64, pub herbivore_threat_near_detection_boundary:u64,
 pub gene_mutation_trials:u64,

 pub strategy_ambush_rest_ticks:u64, pub strategy_roaming_ticks:u64, pub strategy_hybrid_rest_ticks:u64,
 pub herbivore_predator_detected_ticks:u64, pub herbivore_flee_distance:f64,
 pub phase17_rest_ticks:u64, pub phase17_active_search_ticks:u64, pub phase17_chase_ticks:u64, pub phase17_repro_cooldown_only_ticks:u64, pub phase17_repro_age_only_ticks:u64, pub phase17_repro_resource_ready_ticks:u64,
 pub reproduction_ready:u64, pub reproduction_disabled_ticks:u64, pub reproduction_physical_ready_ticks:u64, pub reproduction_all_gates_passed_ticks:u64, pub reproduction_cooldown_blocked:u64,
 pub reproduction_age_blocked:u64, pub reproduction_body_n_blocked:u64,
 pub reproduction_reserve_n_blocked:u64, pub reproduction_energy_blocked:u64,
 pub reproduction_capacity_blocked:u64, pub hunt_contacts:u64,
 pub hunt_nutrients:f64, pub hunt_energy:f64,
 pub carnivore_age_deaths:u64, pub carnivore_energy_deaths:u64,
 pub hunt_refuge_blocked:u64, pub hunt_cooldown_blocked:u64,
 pub reproduction_reserve_body_blocked:u64,
 pub reproduction_food_blocked:u64, pub reproduction_trend_blocked:u64,
 pub hunt_attempts:u64, pub hunt_positive_energy:u64, pub hunt_zero_energy:u64,
 pub energy_basal_spent:f64, pub energy_movement_spent:f64, pub energy_reproduction_spent:f64,
 pub predator_search_ticks:u64, pub predator_target_ticks:u64,
 pub persistent_search_ticks:u64, pub remembered_prey_ticks:u64, pub search_heading_changes:u64, pub search_wall_turns:u64,
 // Phase 06.2.7: observational counters only, no effects on simulation decisions.
 pub prey_visible_but_out_of_attack_range:u64,
 pub prey_in_attack_range:u64,
 pub predator_movement_distance:f64,
 pub attack_energy_limited:u64,
 pub prey_hp_zero_after_attack:u64,
 pub satiated_predator_ticks:u64, pub kills_with_meal:u64, pub meal_bonus_energy:f64, pub meal_bonus_nutrients:f64,
 pub stomach_energy_ingested:f64,pub stomach_nutrients_ingested:f64,
 pub stomach_energy_digested:f64,pub stomach_nutrients_digested:f64,
 pub stomach_feeding_blocked_ticks:u64,pub stomach_reproduction_blocked:u64,
 pub emergency_digestion_ticks:u64,pub emergency_digestion_energy:f64,
 pub predator_death_stomach_energy:f64,pub predator_death_stomach_nutrients:f64,
 pub predator_deaths_with_stomach_energy:u64,pub hunting_hunger_override_ticks:u64,
 pub hunting_hunger_override_no_capacity_ticks:u64,
 pub hunting_hunger_override_attacks:u64,
 pub predator_low_reserve_with_stomach_ticks:u64,
 pub predator_starvation_with_stomach_energy:u64,
 pub bite_stomach_energy:f64,pub bite_stomach_nutrients:f64,
 pub bite_stomach_capacity_blocked:u64,pub sustainable_reproduction_blocked:u64,
 pub predator_death_energy_remaining:f64,
 pub predator_death_age_total:u64,
 pub predator_death_distance_to_nearest_prey_total:f64,
 pub predator_death_nearest_prey_samples:u64,
 pub p40_death_prey_visible:u64, pub p40_death_prey_attack_range:u64,
 pub p40_death_prey_absent:u64, pub p40_death_feeding_paused:u64,
 pub p40_death_nearby_prey_twice_vision:u64, pub p40_death_energy_starvation:u64,
 pub p41_visible_samples:u64, pub p41_approach_samples:u64, pub p41_retreat_samples:u64,
 pub p41_attack_reach_samples:u64, pub p41_chase_movement_energy:f64,
 pub p41_chase_basal_energy:f64,
 pub p41_death_after_visible_target:u64,
}
