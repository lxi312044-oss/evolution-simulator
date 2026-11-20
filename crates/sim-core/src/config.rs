use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SimulationConfig {
    pub version: String,
    pub seed: u64,
    pub world: WorldConfig,
    pub initial_population: InitialPopulation,
    pub simulation: SimulationSettings,
    pub plants: PlantSettings,
    pub herbivores: HerbivoreSettings,
    pub carnivores: CarnivoreSettings,
    pub toxin: ToxinSettings,
    #[serde(skip_serializing_if = "AnimalToxinSettings::is_off")]
    pub animal_toxin: AnimalToxinSettings,
    /// Issue #78 (opt-in): a heritable diet value on every animal. Absent = no diet value
    /// (previous behavior).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diet_gene: Option<DietGeneSettings>,
    /// Issue #80 (opt-in): herbivores weigh carnivore danger against hunger. Absent = the
    /// previous flee rule.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub herbivore_risk: Option<HerbivoreRiskSettings>,
}

/// Issue #80: a herbivore accepts a danger d (0 at the edge of the danger radius, 1 at the
/// carnivore) when `hunger_weight x hunger x (1 - fear) >= d x fear`, where hunger is
/// 1 - reserve energy / reserve capacity and fear is the inherited fear gene plus learned fear.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HerbivoreRiskSettings {
    pub hunger_weight: f64,
    /// A herbivore remembers where it last saw a carnivore for this many Ticks and keeps
    /// treating it as danger until it is this many detection radii away.
    pub flee_until_factor: f64,
    pub memory_ticks: u32,
    /// Plants within this many detection radii of the danger are targets only when their
    /// danger is accepted.
    pub plant_danger_factor: f64,
}

impl Default for HerbivoreRiskSettings {
    fn default() -> Self {
        Self {
            hunger_weight: 1.,
            flee_until_factor: 1.5,
            memory_ticks: 30,
            plant_danger_factor: 1.,
        }
    }
}

/// Issue #78: every animal carries a diet value from 0 (herbivore) to 1 (carnivore). Offspring
/// inherit it with rare normal steps drawn from a stateless stream (no ecology RNG).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DietGeneSettings {
    /// false: the value is inherited and observed but changes nothing (stage 0). true: an
    /// offspring with a value below 0.5 is born a herbivore and at or above 0.5 a carnivore,
    /// and feeding gains scale with the value (stage 1).
    pub effects: bool,
    /// Founder herbivores draw their value uniformly from this centre +/- `initial_spread`,
    /// kept below 0.5.
    pub initial_herbivore: f64,
    /// Founder carnivores draw their value uniformly from this centre +/- `initial_spread`,
    /// kept at or above 0.5.
    pub initial_carnivore: f64,
    pub initial_spread: f64,
    /// Probability that an offspring's value moves, and the standard deviation of the move.
    pub mutation_probability: f64,
    pub mutation_scale: f64,
    /// Feeding-gain exponent k: a herbivore gains x (1 - value)^k from plants, a carnivore
    /// x value^k from bites.
    pub efficiency_exponent: f64,
}

impl Default for DietGeneSettings {
    fn default() -> Self {
        Self {
            effects: false,
            initial_herbivore: 0.1,
            initial_carnivore: 0.9,
            initial_spread: 0.1,
            mutation_probability: 0.1,
            mutation_scale: 0.1,
            efficiency_exponent: 1.,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AnimalToxinSettings {
    pub enabled: bool,
    pub color_learning_enabled: bool,
    /// Override the shared poison damage threshold for carnivores only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carnivore_damage_threshold: Option<f64>,
    /// Override HP damage per excess toxin unit per Tick for carnivores only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carnivore_hp_damage_per_excess_unit: Option<f64>,
    /// Issue #74: fraction of each herbivore toxin stock that breaks down every Tick
    /// (0 = never). Keeping toxin then needs continuous synthesis.
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    pub herbivore_toxin_decay_per_tick: f64,
    /// Issue #76 (opt-in): multiplier on the harm a carnivore writes into its color memory
    /// from toxin damage, and on the per-color upper limit of that harm. The HP damage itself
    /// is unchanged. Absent = 1 (previous behavior).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carnivore_learning_gain: Option<f64>,
    /// Issue #76 (opt-in): herbivores carry a heritable toxic / non-toxic trait. Founders are
    /// toxic with this probability; non-toxic herbivores make no toxin and pay no synthesis
    /// cost (their production ability is still inherited). Absent = every herbivore makes
    /// toxin by its ability (previous behavior).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub herbivore_toxic_initial_fraction: Option<f64>,
    /// Issue #76 (opt-in): probability that an offspring has the opposite toxic trait.
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    pub herbivore_toxic_mutation_probability: f64,
    /// Issue #76 (opt-in): plant-derived toxin of a type moves from a bitten herbivore to the
    /// carnivore only when the herbivore's load of that type is at least this much; below it
    /// the toxin stays in the herbivore. 0 = always moves (previous behavior).
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    pub plant_toxin_transfer_threshold: f64,
}

impl AnimalToxinSettings {
    fn is_off(&self) -> bool {
        !self.enabled
            && !self.color_learning_enabled
            && self.carnivore_damage_threshold.is_none()
            && self.carnivore_hp_damage_per_excess_unit.is_none()
            && self.herbivore_toxin_decay_per_tick == 0.
            && self.carnivore_learning_gain.is_none()
            && self.herbivore_toxic_initial_fraction.is_none()
            && self.herbivore_toxic_mutation_probability == 0.
            && self.plant_toxin_transfer_threshold == 0.
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ToxinSettings {
    /// Phase 07.1: fixed single toxin, not yet heritable.
    pub enabled: bool,
    /// Toxic exposure units per unit of plant nutrients eaten.
    pub dose_per_nutrient: f64,
    /// Fraction of existing internal toxin cleared per tick (0..=1).
    pub clearance_fraction_per_tick: f64,
    /// No HP damage at or below this toxin burden.
    pub damage_threshold: f64,
    /// HP damage per excess burden per tick.
    pub hp_damage_per_excess_unit: f64,
    /// Plant energy spent per tick to synthesize defensive compounds.
    pub plant_energy_cost_per_tick: f64,
    /// Phase 07.4.1 opt-in heritable toxic/non-toxic plant phenotype.
    pub plant_variation_enabled: bool,
    /// Experiment only: pair founders 50/50 and display the resolved toxicity as color.
    #[serde(skip_serializing_if = "is_false")]
    pub experimental_plant_toxicity_colors: bool,
    /// Fixed Stage 4 founder mix with independently inherited visible color.
    #[serde(skip_serializing_if = "is_false")]
    pub experimental_inherited_plant_colors: bool,
    /// Fraction of initially created / spontaneously germinated plants that are toxic.
    pub initial_toxic_plant_fraction: f64,
    /// Probability that a seed flips its parent's toxic phenotype.
    pub plant_toxic_mutation_probability: f64,
    /// Phase 07.5.1 opt-in plant toxin-type inheritance (legacy type 0 when off).
    pub plant_toxin_type_variation_enabled: bool,
    /// Number of available toxin types (1..=16).
    pub plant_toxin_type_count: u16,
    /// Phase 07.5.7: founder and spontaneous-germination type-0 fraction for two types; default 0.5.
    pub initial_type0_fraction: f64,
    /// Chance that an offspring switches to another toxin type.
    pub plant_toxin_type_mutation_probability: f64,
    /// Phase 07.5.2: opt-in per-type herbivore exposure and resistance.
    pub herbivore_multi_toxin_enabled: bool,
    /// Phase 07.2: enable inherited resistance and its upkeep cost.
    pub resistance_enabled: bool,
    /// Maximum toxin burden reduction at resistance=1 (0..=1).
    pub resistance_effectiveness: f64,
    /// Energy dissipated per tick at resistance=1.
    pub resistance_energy_cost_per_tick: f64,
    /// Phase 07.5.5: multiplier for upkeep of additional toxin types (type 1+). Default 1 preserves Phase 07.5.2 behavior.
    pub additional_type_resistance_upkeep_multiplier: f64,
    /// Phase 07.5.6: equal multiplier for all toxin-type upkeep in multi-type mode. Default 1 preserves legacy.
    pub multi_type_resistance_upkeep_multiplier: f64,
    /// Deterministic starting resistance, 0..=1.
    pub initial_resistance: f64,
    /// Mutation probability per birth, 0..=1.
    pub resistance_mutation_probability: f64,
    /// Maximum absolute mutation step, 0..=1.
    pub resistance_mutation_scale: f64,
}
impl Default for ToxinSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            dose_per_nutrient: 0.2,
            clearance_fraction_per_tick: 0.03,
            damage_threshold: 2.0,
            hp_damage_per_excess_unit: 0.001,
            plant_energy_cost_per_tick: 0.002,
            plant_variation_enabled: false,
            experimental_plant_toxicity_colors: false,
            experimental_inherited_plant_colors: false,
            initial_toxic_plant_fraction: 1.0,
            plant_toxic_mutation_probability: 0.0,
            plant_toxin_type_variation_enabled: false,
            plant_toxin_type_count: 1,
            initial_type0_fraction: 0.5,
            plant_toxin_type_mutation_probability: 0.0,
            herbivore_multi_toxin_enabled: false,
            resistance_enabled: false,
            resistance_effectiveness: 0.8,
            resistance_energy_cost_per_tick: 0.002,
            additional_type_resistance_upkeep_multiplier: 1.0,
            multi_type_resistance_upkeep_multiplier: 1.0,
            initial_resistance: 0.5,
            resistance_mutation_probability: 0.05,
            resistance_mutation_scale: 0.1,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorldConfig {
    pub width: f32,
    pub height: f32,
    pub land_columns: u32,
    pub land_rows: u32,
    /// Shared cap across registered plants, herbivores, and carnivores.
    pub max_living_entities: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InitialPopulation {
    pub plants: u32,
    pub herbivores: u32,
    pub carnivores: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SimulationSettings {
    pub decision_interval_ticks: u32,
    pub max_steps_per_call: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlantSettings {
    /// Seed offspring inherit and can mutate the parent's visible RGB.
    #[serde(skip_serializing_if = "is_false")]
    pub color_inheritance_enabled: bool,
    /// Mutation probability per RGB channel at seed birth.
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub color_mutation_probability: f32,
    /// Maximum absolute mutation step per channel in normalized RGB.
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub color_mutation_scale: f32,
    pub initial_soil_nutrients_per_cell: f64,
    pub initial_detritus_per_cell: f64,
    pub initial_body_nutrients: f64,
    pub initial_energy: f64,
    pub photosynthesis_per_tick: f64,
    pub basal_energy_per_tick: f64,
    pub nutrient_uptake_max: f64,
    pub nutrient_half_saturation: f64,
    pub growth_per_tick: f32,
    pub growth_energy_per_nutrient: f64,
    pub adult_body_nutrients: f64,
    pub energy_capacity: f64,
    pub seed_cooldown_ticks: u32,
    pub seed_body_nutrients: f64,
    pub seed_energy: f64,
    pub seed_work_energy: f64,
    pub seed_dispersal_radius: f32,
    pub seed_probability: f32,
    pub max_age_ticks: u32,
    pub mineralization_rate: f64,
    pub soil_diffusion_rate: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HerbivoreSettings {
    /// Let herbivores use visible plant colors and learned adverse experiences.
    pub color_learning_enabled: bool,
    /// Experimental one-tick rescue when low reserve and all edible plants are rejected.
    #[serde(skip_serializing_if = "is_false")]
    pub hungry_color_recheck_enabled: bool,
    /// Keep the opt-in recovery active until reserve reaches 30 percent.
    #[serde(skip_serializing_if = "is_false")]
    pub hungry_color_recovery_enabled: bool,
    /// Opt-in: fraction of learned plant-color harm removed, scaled by color
    /// similarity, on each tick a herbivore eats plant matter without ingesting toxin.
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub color_memory_safe_meal_relief: f64,
    /// Opt-in: fraction of learned plant-color harm that fades each tick.
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub color_memory_decay_per_tick: f64,
    /// Opt-in: basal energy x (1 + slope x (speed / reference - 1)), floored at 0.
    /// Reference = (speed_min + speed_max) / 2. Zero keeps basal energy independent of speed.
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub speed_upkeep_slope: f64,
    /// Opt-in: a bitten herbivore (standing ground or not) drains this x aggression x
    /// (adult_max_hp / 60) energy from the carnivore (dissipated as heat). Zero disables.
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub counterattack_energy: f64,
    /// When carnivore color learning is on, a counterattack also records harm for the
    /// prey's color (counterattack energy / 60). Off only to separate learning in experiments.
    #[serde(skip_serializing_if = "is_true")]
    pub counterattack_learning_enabled: bool,
    /// Opt-in: only herbivores with aggression x (adult_max_hp / 60) at or above this
    /// strength fight back. Zero lets every bitten herbivore fight back.
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub counterattack_strength_threshold: f64,
    /// Opt-in: after a counterattack, the carnivore does not target that herbivore for
    /// this many ticks (it breaks off and the prey can escape). Zero disables.
    #[serde(skip_serializing_if = "is_zero_u32")]
    pub counterattack_retreat_ticks: u32,
    /// Experimental control: false keeps a child's speed equal to its parent's. The mutation
    /// draw still happens and is discarded, so the RNG sequence is unchanged.
    #[serde(skip_serializing_if = "is_true")]
    pub speed_mutation_enabled: bool,
    /// Opt-in: probability that a herbivore's offspring is born as a carnivore (simple
    /// version of predators arising by mutation). The draw uses a stateless stream, so it
    /// never shifts ecology RNG and does not depend on whether carnivores exist. Zero disables.
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub carnivore_offspring_probability: f64,
    /// Issue #76 (opt-in): body-color mutation probability per channel, separate from the
    /// other genes. Absent = `mutation_probability` (previous behavior). Body color uses its
    /// own stateless stream, so this never shifts ecology RNG.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_color_mutation_probability: Option<f32>,
    /// Issue #76 (opt-in): body-color mutation scale. Absent = `mutation_scale`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_color_mutation_scale: Option<f32>,
    pub initial_body_nutrients: f64,
    pub initial_reserve_nutrients: f64,
    pub initial_energy_fraction: f64,
    pub basal_energy_per_tick: f64,
    pub movement_energy_per_unit: f64,
    pub feeding_energy_gain_fraction: f64,
    pub feeding_nutrient_absorption: f64,
    pub feeding_nutrients_per_tick: f64,
    pub feeding_range: f32,
    pub starvation_hp_fraction: f32,
    pub plant_protection_enabled: bool,
    pub plant_protection_threshold: u32,
    pub predator_evasion_enabled: bool,
    pub predator_detection_radius: f32,
    pub predator_flee_speed_multiplier: f32,
    pub satiety_enabled: bool,
    pub stomach_capacity_n: f64,
    pub feeding_stop_fraction: f64,
    pub feeding_resume_fraction: f64,
    pub digestion_n_per_tick: f64,
    pub body_n_repair_per_tick: f64,
    pub max_age_ticks: u32,
    pub max_energy_per_hp: f64,
    pub vision_min: f32,
    pub vision_max: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub satiety_reproduction_enabled: bool,
    pub reproduction_satiety_threshold: f64,
    pub reproduction_well_fed_ticks: u32,
    pub resource_dependent_reproduction: bool,
    pub reproduction_food_radius: f32,
    pub reproduction_food_target: u32,
    pub reproduction_enabled: bool,
    pub reproduction_cooldown_ticks: u32,
    pub reproduction_min_age: u32,
    pub reproduction_min_hp_fraction: f32,
    pub reproduction_min_energy_fraction: f64,
    pub offspring_body_nutrients: f64,
    pub offspring_reserve_nutrients: f64,
    pub offspring_energy: f64,
    pub reproduction_work_energy: f64,
    pub mutation_probability: f32,
    pub mutation_scale: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CarnivoreSettings {
    pub enabled: bool,
    pub speed: f32,
    pub vision: f32,
    pub attack_range: f32,
    pub basal_energy_per_tick: f64,
    pub movement_energy_per_unit: f64,
    pub initial_body_nutrients: f64,
    pub initial_reserve_nutrients: f64,
    pub initial_energy: f64,
    pub body_n_repair_per_tick: f64,
    pub bite_nutrients: f64,
    pub bite_energy: f64,
    pub max_age_ticks: u32,
    pub attack_cooldown_ticks: u32,
    pub prey_refuge_threshold: u32,
    pub satiety_enabled: bool,
    pub satiety_ticks_on_kill: u32,
    pub kill_bonus_energy: f64,
    pub kill_bonus_nutrients: f64,
    pub digestion_enabled: bool,
    pub all_bites_to_stomach: bool,
    pub stomach_capacity_energy: f64,
    pub stomach_capacity_nutrients: f64,
    pub digestion_energy_per_tick: f64,
    pub digestion_nutrients_per_tick: f64,
    pub adaptive_digestion_enabled: bool,
    pub emergency_reserve_energy: f64,
    pub emergency_digestion_multiplier: f64,
    pub feeding_stop_fraction: f64,
    pub feeding_resume_fraction: f64,
    pub reproduction_stomach_max_fraction: f64,
    pub hunting_hunger_mode: u8,
    pub hunting_low_reserve_energy: f64,
    pub hunting_forecast_ticks: u32,
    pub movement_strategy: u8,
    pub ambush_rest_ticks: u32,
    pub ambush_move_ticks: u32,
    pub hybrid_low_energy: f64,
    pub persistent_search_enabled: bool,
    pub search_heading_ticks: u32,
    pub lost_prey_memory_ticks: u32,
    pub prey_refuge_enabled: bool,
    pub food_limited_reproduction_enabled: bool,
    pub prey_per_predator_target: f64,
    pub sustainable_reproduction_enabled: bool,
    pub minimum_prey_per_predator: f64,
    pub prey_trend_reproduction_enabled: bool,
    pub prey_trend_window_ticks: u64,
    pub prey_decline_penalty: f64,
    pub reproduction_enabled: bool,
    pub reproduction_cooldown_ticks: u32,
    pub offspring_body_nutrients: f64,
    pub offspring_reserve_nutrients: f64,
    pub offspring_energy: f64,
    pub reproduction_work_energy: f64,
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub reproduction_parent_reserve_energy: f64,
    pub mutation_probability: f32,
    pub mutation_scale: f32,
    /// Opt-in: each carnivore carries its own heritable speed. OFF keeps every
    /// carnivore at `speed`.
    #[serde(skip_serializing_if = "is_false")]
    pub speed_evolution_enabled: bool,
    /// Initial speed = `speed` x (1 +/- spread), uniform. Used only when speed evolution is on.
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub initial_speed_spread: f32,
    /// Probability that a child's speed mutates. Used only when speed evolution is on.
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub speed_mutation_probability: f32,
    /// Child speed = parent speed x (1 +/- scale), uniform, clamped to 0.5..=8 like herbivores.
    #[serde(skip_serializing_if = "is_zero_f32")]
    pub speed_mutation_scale: f32,
    /// Opt-in: basal energy x (1 + slope x (speed / `speed` - 1)), floored at 0.
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub speed_upkeep_slope: f64,
    /// Opt-in: fraction of learned color harm (toxin or counterattack) that fades per tick.
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub color_memory_decay_per_tick: f64,
    /// Opt-in: after this many hunting ticks, a carnivore compares the energy it gained from
    /// bites with the energy it spent (basal + movement); if it lost energy it rests for the
    /// same number of ticks. Zero disables.
    #[serde(skip_serializing_if = "is_zero_u32")]
    pub hunt_bout_ticks: u32,
}
impl Default for CarnivoreSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            speed: 3.2,
            vision: 120.,
            attack_range: 7.,
            basal_energy_per_tick: 0.025,
            movement_energy_per_unit: 0.008,
            initial_body_nutrients: 24.,
            initial_reserve_nutrients: 8.,
            initial_energy: 65.,
            body_n_repair_per_tick: 0.,
            bite_nutrients: 0.8,
            bite_energy: 1.2,
            max_age_ticks: 9000,
            attack_cooldown_ticks: 12,
            prey_refuge_threshold: 0,
            satiety_enabled: false,
            satiety_ticks_on_kill: 250,
            kill_bonus_energy: 10.,
            kill_bonus_nutrients: 1.5,
            digestion_enabled: false,
            all_bites_to_stomach: false,
            stomach_capacity_energy: 12.,
            stomach_capacity_nutrients: 2.,
            digestion_energy_per_tick: 0.04,
            digestion_nutrients_per_tick: 0.007,
            adaptive_digestion_enabled: false,
            emergency_reserve_energy: 2.0,
            emergency_digestion_multiplier: 4.0,
            feeding_stop_fraction: 0.8,
            feeding_resume_fraction: 0.25,
            reproduction_stomach_max_fraction: 0.25,
            hunting_hunger_mode: 0,
            hunting_low_reserve_energy: 3.0,
            hunting_forecast_ticks: 300,
            movement_strategy: 0,
            ambush_rest_ticks: 80,
            ambush_move_ticks: 40,
            hybrid_low_energy: 15.,
            persistent_search_enabled: false,
            search_heading_ticks: 90,
            lost_prey_memory_ticks: 45,
            prey_refuge_enabled: false,
            food_limited_reproduction_enabled: false,
            prey_per_predator_target: 20.,
            sustainable_reproduction_enabled: false,
            minimum_prey_per_predator: 12.,
            prey_trend_reproduction_enabled: false,
            prey_trend_window_ticks: 500,
            prey_decline_penalty: 0.85,
            reproduction_enabled: true,
            reproduction_cooldown_ticks: 800,
            offspring_body_nutrients: 9.,
            offspring_reserve_nutrients: 3.,
            offspring_energy: 22.,
            reproduction_work_energy: 2.,
            reproduction_parent_reserve_energy: 0.,
            mutation_probability: 0.05,
            mutation_scale: 0.1,
            speed_evolution_enabled: false,
            initial_speed_spread: 0.,
            speed_mutation_probability: 0.,
            speed_mutation_scale: 0.,
            speed_upkeep_slope: 0.,
            color_memory_decay_per_tick: 0.,
            hunt_bout_ticks: 0,
        }
    }
}
impl Default for HerbivoreSettings {
    fn default() -> Self {
        Self {
            initial_body_nutrients: 30.,
            initial_reserve_nutrients: 9.,
            initial_energy_fraction: 0.8,
            basal_energy_per_tick: 0.03,
            movement_energy_per_unit: 0.01,
            feeding_energy_gain_fraction: 0.8,
            feeding_nutrient_absorption: 0.8,
            feeding_nutrients_per_tick: 1.5,
            feeding_range: 5.,
            starvation_hp_fraction: 0.01,
            plant_protection_enabled: false,
            plant_protection_threshold: 300,
            predator_evasion_enabled: false,
            predator_detection_radius: 55.,
            predator_flee_speed_multiplier: 1.4,
            satiety_enabled: false,
            stomach_capacity_n: 20.,
            feeding_stop_fraction: 0.8,
            feeding_resume_fraction: 0.4,
            digestion_n_per_tick: 0.05,
            body_n_repair_per_tick: 0.,
            max_age_ticks: 6000,
            max_energy_per_hp: 2.,
            vision_min: 35.,
            vision_max: 110.,
            speed_min: 1.,
            speed_max: 4.,
            satiety_reproduction_enabled: false,
            reproduction_satiety_threshold: 0.5,
            reproduction_well_fed_ticks: 100,
            resource_dependent_reproduction: false,
            reproduction_food_radius: 100.,
            reproduction_food_target: 8,
            reproduction_enabled: true,
            reproduction_cooldown_ticks: 600,
            color_learning_enabled: false,
            hungry_color_recheck_enabled: false,
            hungry_color_recovery_enabled: false,
            color_memory_safe_meal_relief: 0.,
            color_memory_decay_per_tick: 0.,
            speed_upkeep_slope: 0.,
            counterattack_energy: 0.,
            counterattack_learning_enabled: true,
            counterattack_strength_threshold: 0.,
            counterattack_retreat_ticks: 0,
            speed_mutation_enabled: true,
            carnivore_offspring_probability: 0.,
            body_color_mutation_probability: None,
            body_color_mutation_scale: None,
            reproduction_min_age: 400,
            reproduction_min_hp_fraction: 0.7,
            reproduction_min_energy_fraction: 0.8,
            offspring_body_nutrients: 12.,
            offspring_reserve_nutrients: 3.,
            offspring_energy: 22.,
            reproduction_work_energy: 2.,
            mutation_probability: 0.05,
            mutation_scale: 0.1,
        }
    }
}
impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            width: 1000.,
            height: 1000.,
            land_columns: 20,
            land_rows: 20,
            max_living_entities: 5000,
        }
    }
}
impl Default for InitialPopulation {
    fn default() -> Self {
        Self {
            plants: 300,
            herbivores: 80,
            carnivores: 0,
        }
    }
}
impl Default for SimulationSettings {
    fn default() -> Self {
        Self {
            decision_interval_ticks: 5,
            max_steps_per_call: 1000,
        }
    }
}
impl Default for PlantSettings {
    fn default() -> Self {
        Self {
            color_inheritance_enabled: false,
            color_mutation_probability: 0.,
            color_mutation_scale: 0.,
            initial_soil_nutrients_per_cell: 200.,
            initial_detritus_per_cell: 30.,
            initial_body_nutrients: 9.,
            initial_energy: 12.,
            photosynthesis_per_tick: 0.5,
            basal_energy_per_tick: 0.02,
            nutrient_uptake_max: 0.1,
            nutrient_half_saturation: 10.,
            growth_per_tick: 0.003,
            growth_energy_per_nutrient: 2.,
            adult_body_nutrients: 30.,
            energy_capacity: 60.,
            seed_cooldown_ticks: 100,
            seed_body_nutrients: 9.,
            seed_energy: 8.,
            seed_work_energy: 1.8,
            seed_dispersal_radius: 70.,
            seed_probability: 0.35,
            max_age_ticks: 2000,
            mineralization_rate: 0.002,
            soil_diffusion_rate: 0.0,
        }
    }
}
impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            version: "1.2".into(),
            seed: 12345,
            world: WorldConfig::default(),
            initial_population: InitialPopulation::default(),
            simulation: SimulationSettings::default(),
            plants: PlantSettings::default(),
            herbivores: HerbivoreSettings::default(),
            carnivores: CarnivoreSettings::default(),
            toxin: ToxinSettings::default(),
            animal_toxin: AnimalToxinSettings::default(),
            diet_gene: None,
            herbivore_risk: None,
        }
    }
}
impl SimulationConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != "1.2" {
            return Err("unsupported config version".into());
        }
        let w = &self.world;
        if !w.width.is_finite() || w.width <= 0. || !w.height.is_finite() || w.height <= 0. {
            return Err("world dimensions must be finite positive".into());
        }
        if w.land_columns == 0
            || w.land_rows == 0
            || w.land_columns.checked_mul(w.land_rows).is_none()
        {
            return Err("invalid land grid".into());
        }
        let n = self
            .initial_population
            .plants
            .checked_add(self.initial_population.herbivores)
            .and_then(|x| x.checked_add(self.initial_population.carnivores))
            .ok_or("population overflow")?;
        if n > w.max_living_entities {
            return Err("initial population exceeds maximum".into());
        }
        if self.simulation.decision_interval_ticks == 0 || self.simulation.max_steps_per_call == 0 {
            return Err("invalid tick settings".into());
        }
        if self.animal_toxin.color_learning_enabled && !self.animal_toxin.enabled {
            return Err("carnivore color learning requires animal toxin".into());
        }
        if !(0.0..1.0).contains(&self.animal_toxin.herbivore_toxin_decay_per_tick) {
            return Err(
                "animal_toxin.herbivore_toxin_decay_per_tick は0以上1未満で指定してください".into(),
            );
        }
        if self
            .animal_toxin
            .carnivore_learning_gain
            .is_some_and(|value| !value.is_finite() || value <= 0. || value > 10000.)
        {
            return Err(
                "animal_toxin.carnivore_learning_gain は0より大きく10000以下で指定してください"
                    .into(),
            );
        }
        {
            let a = &self.animal_toxin;
            if a.herbivore_toxic_initial_fraction
                .is_some_and(|v| !(0.0..=1.0).contains(&v))
                || !(0.0..=1.0).contains(&a.herbivore_toxic_mutation_probability)
                || (a.herbivore_toxic_mutation_probability > 0.
                    && a.herbivore_toxic_initial_fraction.is_none())
                || !a.plant_toxin_transfer_threshold.is_finite()
                || a.plant_toxin_transfer_threshold < 0.
            {
                return Err(concat!(
                    "草食の有毒・無毒の性質（animal_toxin.herbivore_toxic_initial_fraction と ",
                    "herbivore_toxic_mutation_probability、0〜1。変異は初期の割合を指定したときだけ）、",
                    "または植物由来の毒が肉食に移る下限（animal_toxin.plant_toxin_transfer_threshold、",
                    "0以上）の値が正しくありません。"
                )
                .into());
            }
        }
        if self
            .animal_toxin
            .carnivore_damage_threshold
            .is_some_and(|value| !value.is_finite() || value < 0.)
        {
            return Err("invalid carnivore toxin damage threshold".into());
        }
        if self
            .animal_toxin
            .carnivore_hp_damage_per_excess_unit
            .is_some_and(|value| !value.is_finite() || value < 0.)
        {
            return Err("invalid carnivore toxin damage coefficient".into());
        }
        let t = &self.toxin;
        if t.experimental_inherited_plant_colors && t.experimental_plant_toxicity_colors {
            return Err("配色設定 toxin.experimental_inherited_plant_colors と toxin.experimental_plant_toxicity_colors は同時に使えません。片方をOFFにしてください。".into());
        }
        // Founder colors and inherited traits are independent of active plant toxin effects.
        if t.experimental_inherited_plant_colors
            && (!t.plant_variation_enabled
                || !t.plant_toxin_type_variation_enabled
                || t.plant_toxin_type_count != 2
                // Issue #74: toxicity may mutate (opt-in, plant_toxic_mutation_probability);
                // toxin types stay fixed.
                || t.plant_toxin_type_mutation_probability != 0.
                // Founders are assigned in blocks of 10 plants (4/1/1/4); 300 is the
                // 20x20 stage 4 setting and its multiples keep the same density.
                || self.initial_population.plants == 0
                || !self.initial_population.plants.is_multiple_of(10))
        {
            return Err(concat!(
                "継承配色（toxin.experimental_inherited_plant_colors）の設定が不整合です。",
                " 継承配色を使うには initial_population.plants を10の倍数（20×20の段階4では300）、",
                "toxin.plant_variation_enabled と toxin.plant_toxin_type_variation_enabled をON、",
                "toxin.plant_toxin_type_count=2、",
                "toxin.plant_toxin_type_mutation_probability を0にしてください。",
                " この条件を使わない場合は継承配色をOFFにしてください。"
            )
            .into());
        }
        if t.experimental_plant_toxicity_colors
            && (!t.plant_variation_enabled || !self.initial_population.plants.is_multiple_of(2))
        {
            return Err("毒性連動配色（toxin.experimental_plant_toxicity_colors）には toxin.plant_variation_enabled=ON と偶数の initial_population.plants が必要です。これらを確認するか、毒性連動配色をOFFにしてください。".into());
        }
        if [
            t.dose_per_nutrient,
            t.clearance_fraction_per_tick,
            t.damage_threshold,
            t.hp_damage_per_excess_unit,
            t.plant_energy_cost_per_tick,
        ]
        .iter()
        .any(|x| !x.is_finite() || *x < 0.)
            || t.clearance_fraction_per_tick > 1.
            || [
                t.resistance_effectiveness,
                t.initial_resistance,
                t.resistance_mutation_probability,
                t.resistance_mutation_scale,
            ]
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || !t.resistance_energy_cost_per_tick.is_finite()
            || t.resistance_energy_cost_per_tick < 0.
            || !t.additional_type_resistance_upkeep_multiplier.is_finite()
            || !(0.0..=1.0).contains(&t.additional_type_resistance_upkeep_multiplier)
            || !t.multi_type_resistance_upkeep_multiplier.is_finite()
            || !(0.0..=1.0).contains(&t.multi_type_resistance_upkeep_multiplier)
        {
            return Err("invalid toxin settings".into());
        }
        if !t.initial_toxic_plant_fraction.is_finite()
            || !(0.0..=1.0).contains(&t.initial_toxic_plant_fraction)
            || !t.plant_toxic_mutation_probability.is_finite()
            || !(0.0..=1.0).contains(&t.plant_toxic_mutation_probability)
        {
            return Err("invalid plant toxin variation settings".into());
        }
        if !t.initial_type0_fraction.is_finite() || !(0.0..=1.0).contains(&t.initial_type0_fraction)
        {
            return Err("invalid initial_type0_fraction".into());
        }
        if t.plant_toxin_type_count == 0
            || t.plant_toxin_type_count > 16
            || !t.plant_toxin_type_mutation_probability.is_finite()
            || !(0.0..=1.0).contains(&t.plant_toxin_type_mutation_probability)
        {
            return Err("invalid plant toxin type settings".into());
        }
        let p = &self.plants;
        if p.color_inheritance_enabled
            && (t.experimental_plant_toxicity_colors || t.experimental_inherited_plant_colors)
        {
            return Err(concat!(
                "plants.color_inheritance_enabled は実験配色と同時に使えません。",
                " 通常の色継承を使う場合は toxin.experimental_plant_toxicity_colors と ",
                "toxin.experimental_inherited_plant_colors を両方OFFにしてください。",
                " 実験配色を使う場合は plants.color_inheritance_enabled をOFFにしてください。"
            )
            .into());
        }
        if !(0.0..=1.0).contains(&p.color_mutation_probability) {
            return Err("plants.color_mutation_probability は0〜1で指定してください。".into());
        }
        if !(0.0..=1.0).contains(&p.color_mutation_scale) {
            return Err("plants.color_mutation_scale は0〜1で指定してください。".into());
        }
        let nums = [
            p.initial_soil_nutrients_per_cell,
            p.initial_detritus_per_cell,
            p.initial_body_nutrients,
            p.initial_energy,
            p.photosynthesis_per_tick,
            p.basal_energy_per_tick,
            p.nutrient_uptake_max,
            p.nutrient_half_saturation,
            p.growth_energy_per_nutrient,
            p.adult_body_nutrients,
            p.energy_capacity,
            p.seed_body_nutrients,
            p.seed_energy,
            p.seed_work_energy,
            p.mineralization_rate,
            p.soil_diffusion_rate,
        ];
        if nums.iter().any(|v| !v.is_finite() || *v < 0.)
            || p.nutrient_half_saturation <= 0.
            || p.adult_body_nutrients <= 0.
            || p.energy_capacity <= 0.
            || p.seed_body_nutrients <= 0.
            || p.initial_body_nutrients > p.adult_body_nutrients
            || p.initial_energy > p.energy_capacity
            || p.seed_energy > p.energy_capacity
            || p.mineralization_rate > 1.
            || p.soil_diffusion_rate > 1.
        {
            return Err("invalid plant resource parameters".into());
        }
        if !p.growth_per_tick.is_finite()
            || p.growth_per_tick < 0.
            || !p.seed_probability.is_finite()
            || !(0.0..=1.0).contains(&p.seed_probability)
            || !p.seed_dispersal_radius.is_finite()
            || p.seed_dispersal_radius < 0.
            || p.max_age_ticks == 0
        {
            return Err("invalid plant lifecycle parameters".into());
        }
        let h = &self.herbivores;
        if !h.stomach_capacity_n.is_finite()
            || h.stomach_capacity_n <= 0.
            || !h.digestion_n_per_tick.is_finite()
            || h.digestion_n_per_tick < 0.
            || !h.feeding_stop_fraction.is_finite()
            || !h.feeding_resume_fraction.is_finite()
            || !(0.0..=1.0).contains(&h.feeding_resume_fraction)
            || !(0.0..=1.0).contains(&h.feeding_stop_fraction)
            || h.feeding_resume_fraction >= h.feeding_stop_fraction
        {
            return Err("invalid satiety settings".into());
        }
        let hs = [
            h.body_n_repair_per_tick,
            h.initial_body_nutrients,
            h.initial_reserve_nutrients,
            h.initial_energy_fraction,
            h.basal_energy_per_tick,
            h.movement_energy_per_unit,
            h.feeding_energy_gain_fraction,
            h.feeding_nutrient_absorption,
            h.feeding_nutrients_per_tick,
            h.max_energy_per_hp,
        ];
        if [
            h.offspring_body_nutrients,
            h.offspring_reserve_nutrients,
            h.offspring_energy,
            h.reproduction_work_energy,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.)
            || h.offspring_body_nutrients <= 0.
            || h.reproduction_cooldown_ticks == 0
            || !h.reproduction_min_hp_fraction.is_finite()
            || !(0.0..=1.0).contains(&h.reproduction_min_hp_fraction)
            || !h.reproduction_min_energy_fraction.is_finite()
            || !(0.0..=1.0).contains(&h.reproduction_min_energy_fraction)
            || !h.mutation_probability.is_finite()
            || !(0.0..=1.0).contains(&h.mutation_probability)
            || !h.mutation_scale.is_finite()
            || !(0.0..=1.0).contains(&h.mutation_scale)
            || h.offspring_energy > 200. * h.max_energy_per_hp
        {
            return Err("invalid reproduction settings".into());
        }
        if !h.reproduction_satiety_threshold.is_finite()
            || !(0.0..=1.0).contains(&h.reproduction_satiety_threshold)
            || h.reproduction_well_fed_ticks == 0
        {
            return Err("invalid satiety reproduction settings".into());
        }
        if !h.reproduction_food_radius.is_finite()
            || h.reproduction_food_radius <= 0.
            || h.reproduction_food_target == 0
        {
            return Err("invalid resource dependent reproduction settings".into());
        }
        if hs.iter().any(|v| !v.is_finite() || *v < 0.)
            || h.initial_energy_fraction > 1.
            || h.feeding_energy_gain_fraction > 1.
            || h.feeding_nutrient_absorption > 1.
            || h.max_energy_per_hp <= 0.
            || h.max_age_ticks == 0
            || !h.feeding_range.is_finite()
            || h.feeding_range < 0.
            || !h.starvation_hp_fraction.is_finite()
            || h.starvation_hp_fraction < 0.
            || !h.vision_min.is_finite()
            || !h.vision_max.is_finite()
            || h.vision_min <= 0.
            || h.vision_min > h.vision_max
            || !h.speed_min.is_finite()
            || !h.speed_max.is_finite()
            || h.speed_min <= 0.
            || h.speed_min > h.speed_max
        {
            return Err("invalid herbivore settings".into());
        }
        if [
            h.body_color_mutation_probability,
            h.body_color_mutation_scale,
        ]
        .iter()
        .flatten()
        .any(|v| !(0.0..=1.0).contains(v))
        {
            return Err(concat!(
                "草食の体色の突然変異（herbivores.body_color_mutation_probability と ",
                "herbivores.body_color_mutation_scale）は0から1の範囲で指定してください。"
            )
            .into());
        }
        if [
            h.color_memory_safe_meal_relief,
            h.color_memory_decay_per_tick,
        ]
        .iter()
        .any(|v| !(0.0..=1.0).contains(v))
        {
            return Err(concat!(
                "草食の色記憶の更新設定（herbivores.color_memory_safe_meal_relief と ",
                "herbivores.color_memory_decay_per_tick）は0から1の範囲で指定してください。",
                " 使わない場合は0にしてください。"
            )
            .into());
        }
        if !(0.0..=10.0).contains(&h.counterattack_strength_threshold)
            || !(0.0..=1.0).contains(&self.carnivores.color_memory_decay_per_tick)
        {
            return Err(concat!(
                "反撃のしきい値（herbivores.counterattack_strength_threshold）は0から10、",
                "肉食の色の記憶の減衰（carnivores.color_memory_decay_per_tick）は0から1の範囲で",
                "指定してください。使わない場合は0にしてください。"
            )
            .into());
        }
        if !h.counterattack_energy.is_finite() || !(0.0..=100.0).contains(&h.counterattack_energy) {
            return Err(concat!(
                "草食の反撃量（herbivores.counterattack_energy）は0から100の範囲で指定してください。",
                "使わない場合は0にしてください。"
            )
            .into());
        }
        if [h.speed_upkeep_slope, self.carnivores.speed_upkeep_slope]
            .iter()
            .any(|v| !(0.0..=5.0).contains(v))
        {
            return Err(concat!(
                "速さの維持コスト（herbivores.speed_upkeep_slope と ",
                "carnivores.speed_upkeep_slope）は0から5の範囲で指定してください。",
                "使わない場合は0にしてください。"
            )
            .into());
        }
        let c = &self.carnivores;
        let amounts = [
            c.body_n_repair_per_tick,
            c.basal_energy_per_tick,
            c.movement_energy_per_unit,
            c.initial_body_nutrients,
            c.initial_reserve_nutrients,
            c.initial_energy,
            c.bite_nutrients,
            c.bite_energy,
            c.offspring_body_nutrients,
            c.offspring_reserve_nutrients,
            c.offspring_energy,
            c.reproduction_work_energy,
            c.reproduction_parent_reserve_energy,
        ];
        if amounts.iter().any(|v| !v.is_finite() || *v < 0.)
            || c.speed <= 0.
            || !c.speed.is_finite()
            || c.vision <= 0.
            || !c.vision.is_finite()
            || c.attack_range <= 0.
            || !c.attack_range.is_finite()
            || c.max_age_ticks == 0
            || c.reproduction_cooldown_ticks == 0
            || !(0.0..=1.0).contains(&c.mutation_probability)
            || !(0.0..=1.0).contains(&c.mutation_scale)
        {
            return Err("invalid carnivore settings".into());
        }
        if [
            c.initial_speed_spread,
            c.speed_mutation_probability,
            c.speed_mutation_scale,
        ]
        .iter()
        .any(|v| !(0.0..1.0).contains(v))
        {
            return Err(concat!(
                "肉食の速さの遺伝の設定（carnivores.initial_speed_spread、",
                "carnivores.speed_mutation_probability、carnivores.speed_mutation_scale）は",
                "0以上1未満で指定してください。使わない場合は0にしてください。"
            )
            .into());
        }
        if !c.prey_per_predator_target.is_finite() || c.prey_per_predator_target <= 0. {
            return Err("invalid prey_per_predator_target".into());
        }
        if c.prey_trend_window_ticks == 0
            || !c.prey_decline_penalty.is_finite()
            || !(0.0..=1.0).contains(&c.prey_decline_penalty)
        {
            return Err("invalid carnivore prey trend settings".into());
        }
        if c.sustainable_reproduction_enabled
            && (!c.minimum_prey_per_predator.is_finite() || c.minimum_prey_per_predator <= 0.)
        {
            return Err("invalid minimum prey per predator".into());
        }
        if c.adaptive_digestion_enabled
            && (!c.digestion_enabled
                || !c.emergency_reserve_energy.is_finite()
                || c.emergency_reserve_energy < 0.
                || !c.emergency_digestion_multiplier.is_finite()
                || c.emergency_digestion_multiplier < 1.)
        {
            return Err("invalid adaptive carnivore digestion settings".into());
        }
        if c.all_bites_to_stomach && !c.digestion_enabled {
            return Err("all_bites_to_stomach requires digestion_enabled".into());
        }
        if c.persistent_search_enabled
            && (c.search_heading_ticks == 0 || c.lost_prey_memory_ticks == 0)
        {
            return Err("invalid carnivore persistent search settings".into());
        }
        if c.movement_strategy > 4
            || c.ambush_rest_ticks == 0
            || c.ambush_move_ticks == 0
            || !c.hybrid_low_energy.is_finite()
            || c.hybrid_low_energy < 0.
        {
            return Err("invalid carnivore movement strategy".into());
        }
        if self.herbivores.predator_evasion_enabled
            && (!self.herbivores.predator_detection_radius.is_finite()
                || self.herbivores.predator_detection_radius <= 0.
                || !self.herbivores.predator_flee_speed_multiplier.is_finite()
                || self.herbivores.predator_flee_speed_multiplier < 1.
                || self.herbivores.predator_flee_speed_multiplier > 3.)
        {
            return Err("invalid herbivore predator evasion".into());
        }
        if c.hunting_hunger_mode > 2
            || !c.hunting_low_reserve_energy.is_finite()
            || c.hunting_low_reserve_energy < 0.
            || c.hunting_forecast_ticks == 0
        {
            return Err("invalid carnivore hunger policy".into());
        }
        if c.digestion_enabled
            && (c.stomach_capacity_energy <= 0.
                || c.stomach_capacity_nutrients <= 0.
                || c.digestion_energy_per_tick <= 0.
                || c.digestion_nutrients_per_tick <= 0.
                || !c.stomach_capacity_energy.is_finite()
                || !c.stomach_capacity_nutrients.is_finite()
                || !c.digestion_energy_per_tick.is_finite()
                || !c.digestion_nutrients_per_tick.is_finite()
                || !(0.0..=1.0).contains(&c.feeding_stop_fraction)
                || !(0.0..=1.0).contains(&c.feeding_resume_fraction)
                || c.feeding_resume_fraction >= c.feeding_stop_fraction
                || !(0.0..=1.0).contains(&c.reproduction_stomach_max_fraction))
        {
            return Err("invalid carnivore digestion settings".into());
        }
        if !(0.0..=1.0).contains(&self.herbivores.carnivore_offspring_probability) {
            return Err(concat!(
                "草食から肉食が生まれる確率（herbivores.carnivore_offspring_probability）は",
                "0から1の範囲で指定してください。使わない場合は0にしてください。"
            )
            .into());
        }
        let diet_effects = self.diet_gene.as_ref().is_some_and(|d| d.effects);
        if c.enabled
            && self.initial_population.carnivores == 0
            && self.herbivores.carnivore_offspring_probability == 0.
            && !diet_effects
        {
            return Err("enabled carnivores require nonzero initial population".into());
        }
        if let Some(r) = &self.herbivore_risk {
            if !(0.0..=100.0).contains(&r.hunger_weight)
                || !(1.0..=10.0).contains(&r.flee_until_factor)
                || r.memory_ticks > 100_000
                || !(0.0..=10.0).contains(&r.plant_danger_factor)
            {
                return Err(concat!(
                    "草食の危険の判断（herbivore_risk）：空腹の重みは0〜100、逃げ続ける距離は",
                    "1〜10倍、記憶のTick数は100000以下、植物の危険の半径は0〜10倍で指定してください。"
                )
                .into());
            }
        }
        if let Some(d) = &self.diet_gene {
            let unit = |x: f64| (0.0..=1.0).contains(&x);
            if !(0.0..0.5).contains(&d.initial_herbivore)
                || !(0.5..=1.0).contains(&d.initial_carnivore)
                || !(0.0..=0.5).contains(&d.initial_spread)
                || !unit(d.mutation_probability)
                || !(0.0..=1.0).contains(&d.mutation_scale)
                || !(0.0..=10.0).contains(&d.efficiency_exponent)
            {
                return Err(concat!(
                    "食性の遺伝子（diet_gene）：最初の草食の値は0以上0.5未満、最初の肉食の値は",
                    "0.5以上1以下、ばらつき・変異の幅は0〜0.5／0〜1、変異の確率は0〜1、",
                    "効率の指数は0〜10で指定してください。"
                )
                .into());
            }
            if d.effects && (!c.enabled || self.herbivores.carnivore_offspring_probability != 0.) {
                return Err(concat!(
                    "食性の遺伝子を効かせる（diet_gene.effects）ときは、肉食を有効にし、",
                    "草食から肉食が生まれる確率（herbivores.carnivore_offspring_probability）を",
                    "0にしてください。"
                )
                .into());
            }
        }
        Ok(())
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_true(value: &bool) -> bool {
    *value
}

fn is_zero_u32(value: &u32) -> bool {
    *value == 0
}

fn is_zero_f32(value: &f32) -> bool {
    *value == 0.
}

fn is_zero_f64(value: &f64) -> bool {
    *value == 0.
}
