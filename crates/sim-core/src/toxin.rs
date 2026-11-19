//! Phase 07.3.2: species-neutral single-toxin physiological calculations.
//! These pure functions intentionally preserve the original operation order,
//! f64 -> f32 rounding, and independent resistance upkeep semantics.
//! Species-specific resource accounting and HP mutation remain in the caller.
use crate::config::ToxinSettings;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToxinStep {
    pub remaining_load: f64,
    pub cleared: f64,
    pub damage: f32,
    pub prevented_damage: f64,
}

/// The caller checks toxin.enabled and eaten > 0, as in the legacy code.
pub fn exposure_dose(eaten: f64, settings: &ToxinSettings) -> f64 {
    eaten * settings.dose_per_nutrient
}

/// One physiological tick, with the original clearance-before-damage order.
/// The caller checks toxin.enabled; no RNG or resource ledger is touched.
pub fn advance_burden(load: f64, resistance: f32, settings: &ToxinSettings) -> ToxinStep {
    let before = load;
    let remaining_load = load * (1. - settings.clearance_fraction_per_tick);
    let cleared = before - remaining_load;
    let excess = (remaining_load - settings.damage_threshold).max(0.);
    let raw_damage = excess * settings.hp_damage_per_excess_unit;
    let protection = if settings.resistance_enabled {
        (resistance as f64 * settings.resistance_effectiveness).clamp(0., 1.)
    } else {
        0.
    };
    let damage = (raw_damage * (1. - protection)).min(f32::MAX as f64) as f32;
    let prevented_damage = raw_damage - (damage as f64);
    ToxinStep {
        remaining_load,
        cleared,
        damage,
        prevented_damage,
    }
}

/// Upkeep applies whenever resistance_enabled, even if toxin.enabled is false.
/// Caller retains the original resource-ledger heat dissipation.
pub fn resistance_upkeep(resistance: f64, available_energy: f64, settings: &ToxinSettings) -> f64 {
    (settings.resistance_energy_cost_per_tick * resistance).min(available_energy)
}

// Phase 07.3.3: compact, species-independent toxin data representation.
// Only type 0 is active in the current simulation. Other IDs are reserved for
// Phase 07.5; these definitions do not enable new toxin pathways.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub struct ToxinTypeId(pub u16);

impl ToxinTypeId {
    pub const LEGACY: Self = Self(0);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToxinProfile {
    pub toxin_type: ToxinTypeId,
    pub dose_per_nutrient: f64,
    pub clearance_fraction_per_tick: f64,
    pub damage_threshold: f64,
    pub hp_damage_per_excess_unit: f64,
}

impl ToxinProfile {
    pub fn from_legacy(settings: &ToxinSettings) -> Self {
        Self {
            toxin_type: ToxinTypeId::LEGACY,
            dose_per_nutrient: settings.dose_per_nutrient,
            clearance_fraction_per_tick: settings.clearance_fraction_per_tick,
            damage_threshold: settings.damage_threshold,
            hp_damage_per_excess_unit: settings.hp_damage_per_excess_unit,
        }
    }
}

/// One toxin burden. A future multi-toxin container can hold multiple entries;
/// the current model keeps its existing f64 field without allocation.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ToxinBurden {
    pub toxin_type: ToxinTypeId,
    pub load: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ToxinResistance {
    pub toxin_type: ToxinTypeId,
    pub level: f32,
}

/// Production is independent of resistance and of the burden acquired by eating.
/// Not attached to plants or animals until Phase 07.4/08.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ToxinProduction {
    pub toxin_type: ToxinTypeId,
    pub output_per_tick: f64,
    pub energy_cost_per_tick: f64,
    pub enabled: bool,
}

/// Zero-allocation adapter for existing herbivore toxin state.
/// The old f64/f32 fields remain authoritative, preserving memory layout,
/// serialization and deterministic simulation behavior.
pub fn legacy_burden(load: f64) -> ToxinBurden {
    ToxinBurden {
        toxin_type: ToxinTypeId::LEGACY,
        load,
    }
}

pub fn legacy_resistance(level: f32) -> ToxinResistance {
    ToxinResistance {
        toxin_type: ToxinTypeId::LEGACY,
        level,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn common_types_preserve_legacy_precision_and_identity() {
        let settings = ToxinSettings {
            dose_per_nutrient: 0.125,
            ..Default::default()
        };
        let profile = ToxinProfile::from_legacy(&settings);
        assert_eq!(profile.toxin_type, ToxinTypeId::LEGACY);
        assert_eq!(
            profile.dose_per_nutrient.to_bits(),
            settings.dose_per_nutrient.to_bits()
        );
        assert_eq!(
            profile.clearance_fraction_per_tick.to_bits(),
            settings.clearance_fraction_per_tick.to_bits()
        );
        assert_eq!(
            profile.damage_threshold.to_bits(),
            settings.damage_threshold.to_bits()
        );
        assert_eq!(
            profile.hp_damage_per_excess_unit.to_bits(),
            settings.hp_damage_per_excess_unit.to_bits()
        );
        assert_eq!(
            legacy_burden(0.123456789).load.to_bits(),
            0.123456789_f64.to_bits()
        );
        assert_eq!(
            legacy_resistance(0.35_f32).level.to_bits(),
            0.35_f32.to_bits()
        );
        assert_ne!(ToxinTypeId::LEGACY, ToxinTypeId(1));
        assert!(!ToxinProduction::default().enabled);
    }
    #[test]
    fn legacy_order_and_rounding() {
        let t = ToxinSettings {
            enabled: true,
            resistance_enabled: true,
            damage_threshold: 0.5,
            hp_damage_per_excess_unit: 0.010,
            ..Default::default()
        };
        for load in [0.0, 0.5, 1.0, 20.0, 1000.0] {
            for resistance in [0.0_f32, 0.2, 0.5, 1.0] {
                let before = load;
                let remaining = load * (1. - t.clearance_fraction_per_tick);
                let cleared = before - remaining;
                let excess = (remaining - t.damage_threshold).max(0.);
                let raw = excess * t.hp_damage_per_excess_unit;
                let protection = (resistance as f64 * t.resistance_effectiveness).clamp(0., 1.);
                let damage = (raw * (1. - protection)).min(f32::MAX as f64) as f32;
                let result = advance_burden(load, resistance, &t);
                assert_eq!(result.remaining_load.to_bits(), remaining.to_bits());
                assert_eq!(result.cleared.to_bits(), cleared.to_bits());
                assert_eq!(result.damage.to_bits(), damage.to_bits());
                assert_eq!(
                    result.prevented_damage.to_bits(),
                    (raw - damage as f64).to_bits()
                );
            }
        }
    }
    #[test]
    fn resistance_off_and_upkeep_independent() {
        let mut t = ToxinSettings::default();
        let result = advance_burden(10.0, 1.0, &t);
        assert!(result.damage > 0.0);
        assert_eq!(exposure_dose(2.0, &t), 0.4);
        assert_eq!(resistance_upkeep(0.5, 0.0001, &t), 0.0001);
        t.resistance_enabled = true;
        assert!(advance_burden(10.0, 1.0, &t).damage < result.damage);
    }
}

/// Independent hash-based Bernoulli draw, avoiding any change to the ecology RNG stream.
/// A stable discriminator distinguishes founders, spontaneous births, and mutations.
pub fn plant_trait_draw(seed: u64, entity_index: u64, discriminator: u64) -> f64 {
    let mut x = seed ^ entity_index.wrapping_mul(0x9e3779b97f4a7c15) ^ discriminator;
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^= x >> 31;
    ((x >> 11) as f64) / ((1u64 << 53) as f64)
}

pub fn founder_plant_toxic(seed: u64, entity_index: u64, variation: bool, fraction: f64) -> bool {
    !variation || plant_trait_draw(seed, entity_index, 0x74100001) < fraction
}

pub fn offspring_plant_toxic(
    seed: u64,
    entity_index: u64,
    parent_toxic: bool,
    variation: bool,
    mutation: f64,
) -> bool {
    if !variation {
        return true;
    }
    parent_toxic ^ (plant_trait_draw(seed, entity_index, 0x74100002) < mutation)
}

#[cfg(test)]
mod plant_trait_tests {
    use super::*;
    #[test]
    fn disabled_is_legacy_all_toxic() {
        for i in 0..200 {
            assert!(founder_plant_toxic(12345, i, true, 1.));
            assert!(founder_plant_toxic(12345, i, false, 0.));
        }
    }
    #[test]
    fn phenotype_inheritance_and_mutation_extremes() {
        assert!(offspring_plant_toxic(123, 45, true, true, 0.));
        assert!(!offspring_plant_toxic(123, 45, false, true, 0.));
        assert!(!offspring_plant_toxic(123, 45, true, true, 1.));
        assert!(offspring_plant_toxic(123, 45, false, true, 1.));
    }
    #[test]
    fn hash_is_repeatable_and_in_unit_interval() {
        for i in 0..1000 {
            let a = plant_trait_draw(12345, i, 17);
            assert_eq!(a, plant_trait_draw(12345, i, 17));
            assert!((0. ..1.).contains(&a));
        }
    }
}

/// Phase 07.5: deterministic, type-indexed multi-toxin physiology kernel.
/// The legacy type-0 simulation path is deliberately not replaced here.
/// Each entry has its own dose, clearance, threshold and resistance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypedToxinState {
    pub toxin_type: ToxinTypeId,
    pub load: f64,
    pub resistance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypedToxinParameters {
    pub profile: ToxinProfile,
    pub resistance_effectiveness: f64,
    pub resistance_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypedToxinResult {
    pub toxin_type: ToxinTypeId,
    pub remaining_load: f64,
    pub cleared: f64,
    pub damage: f32,
    pub prevented_damage: f64,
}

/// Calculates one toxin without accessing RNG, entity state or resource ledgers.
/// This uses the same operation sequence as advance_burden, including f32 rounding.
pub fn advance_typed_toxin(
    state: TypedToxinState,
    params: TypedToxinParameters,
) -> Result<TypedToxinResult, &'static str> {
    if state.toxin_type != params.profile.toxin_type {
        return Err("toxin type mismatch");
    }
    let p = params.profile;
    if !state.load.is_finite()
        || state.load < 0.
        || !state.resistance.is_finite()
        || !(0.0..=1.0).contains(&state.resistance)
        || !p.clearance_fraction_per_tick.is_finite()
        || !(0.0..=1.0).contains(&p.clearance_fraction_per_tick)
        || !p.damage_threshold.is_finite()
        || p.damage_threshold < 0.
        || !p.hp_damage_per_excess_unit.is_finite()
        || p.hp_damage_per_excess_unit < 0.
        || !params.resistance_effectiveness.is_finite()
        || !(0.0..=1.0).contains(&params.resistance_effectiveness)
    {
        return Err("invalid toxin parameters");
    }
    let remaining_load = state.load * (1. - p.clearance_fraction_per_tick);
    let cleared = state.load - remaining_load;
    let excess = (remaining_load - p.damage_threshold).max(0.);
    let raw_damage = excess * p.hp_damage_per_excess_unit;
    let protection = if params.resistance_enabled {
        (state.resistance as f64 * params.resistance_effectiveness).clamp(0., 1.)
    } else {
        0.
    };
    let damage = (raw_damage * (1. - protection)).min(f32::MAX as f64) as f32;
    Ok(TypedToxinResult {
        toxin_type: state.toxin_type,
        remaining_load,
        cleared,
        damage,
        prevented_damage: raw_damage - damage as f64,
    })
}

/// Updates independent toxin loads in fixed type-ID order. Never sums loads
/// before thresholding; cross-resistance is intentionally absent.
pub fn advance_toxin_set(
    states: &[TypedToxinState],
    parameters: &[TypedToxinParameters],
) -> Result<Vec<TypedToxinResult>, &'static str> {
    if states.len() != parameters.len() {
        return Err("toxin state/parameter length mismatch");
    }
    let mut result = Vec::with_capacity(states.len());
    let mut last = None;
    for (state, param) in states.iter().zip(parameters) {
        if last.is_some_and(|previous: u16| state.toxin_type.0 <= previous) {
            return Err("toxin types must be unique and sorted");
        }
        last = Some(state.toxin_type.0);
        result.push(advance_typed_toxin(*state, *param)?);
    }
    Ok(result)
}

/// Phase 07.5.1: separate hash stream, no additional ecology RNG draws.
/// The gene is retained even when the plant does not produce toxin.
pub fn founder_plant_toxin_type(
    seed: u64,
    entity_index: u64,
    enabled: bool,
    count: u16,
) -> ToxinTypeId {
    if !enabled || count <= 1 {
        return ToxinTypeId::LEGACY;
    }
    let choice = (plant_trait_draw(seed, entity_index, 0x75100001) * count as f64) as u16;
    ToxinTypeId(choice.min(count - 1))
}

/// Opt-in biased founder assignment for exactly two types; the legacy path is identical at 0.5.
pub fn founder_plant_toxin_type_biased(
    seed: u64,
    entity_index: u64,
    enabled: bool,
    count: u16,
    type0_fraction: f64,
) -> ToxinTypeId {
    if !enabled || count <= 1 {
        return ToxinTypeId::LEGACY;
    }
    if count != 2 || type0_fraction == 0.5 {
        return founder_plant_toxin_type(seed, entity_index, enabled, count);
    }
    let draw = plant_trait_draw(seed, entity_index, 0x75100001);
    ToxinTypeId(if draw < type0_fraction { 0 } else { 1 })
}

pub fn offspring_plant_toxin_type(
    seed: u64,
    entity_index: u64,
    parent: ToxinTypeId,
    enabled: bool,
    count: u16,
    mutation: f64,
) -> ToxinTypeId {
    if !enabled {
        return ToxinTypeId::LEGACY;
    }
    if count <= 1 || plant_trait_draw(seed, entity_index, 0x75100002) >= mutation {
        return parent;
    }
    // Uniform choice among all *other* types, never a silent mutation.
    let offset = (plant_trait_draw(seed, entity_index, 0x75100003) * (count - 1) as f64) as u16;
    let candidate = if offset >= parent.0 {
        offset + 1
    } else {
        offset
    };
    ToxinTypeId(candidate)
}

/// Independent deterministic mutation stream for additional resistance loci.
/// Does not consume the ecology RNG and does not alter type-0 inheritance.
pub fn herbivore_type_draw(seed: u64, birth_index: u64, toxin_type: u16, stream: u64) -> f64 {
    let mut x = seed
        ^ birth_index.wrapping_mul(0x9e3779b97f4a7c15)
        ^ (toxin_type as u64).wrapping_mul(0xbf58476d1ce4e5b9)
        ^ stream.wrapping_mul(0x94d049bb133111eb);
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58476d1ce4e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d049bb133111eb);
    x ^= x >> 31;
    (x >> 11) as f64 * (1.0 / ((1u64 << 53) as f64))
}
