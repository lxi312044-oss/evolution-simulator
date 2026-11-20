//! Feature-gated, bit-exact state projection for cross-version acceptance.
use super::World;
use serde::Serialize;

impl World {
    pub fn audit_state(&self, legacy: bool, section: Option<u32>) -> Vec<u8> {
        // Exhaustive pattern: a new World field requires an explicit audit decision.
        let Self {
            tick,
            animal_toxins,
            plant_color_learning,
            hungry_color_recovery,
            config,
            spontaneous_germination,
            spontaneous_births,
            entities,
            resources,
            ledger,
            baseline,
            land,
            plants,
            animals,
            carnivores,
            plant_grid,
            rng,
            births,
            deaths,
            carnivore_births,
            carnivore_deaths,
            carnivore_hunts,
            diagnostics,
            carnivore_debug,
            defense: _,
            species: _,
            mimicry: _,
            diet,
            carnivore_retreats,
            carnivore_hunt_bouts,
            prey_trend_snapshot,
            resistance_event_logging,
            resistance_birth_events,
            herbivore_terminal_logging,
            herbivore_terminal_events,
            focal_trace_id,
            focal_upkeep_override,
            resistance_upkeep_exempt,
            focal_trace,
            behavior_trace_window,
            behavior_trace_subject,
            health_trace,
            energy_trace,
            foraging_trace,
            color_event_logging,
            color_events,
            p42_life_events,
        } = self;

        // `defense` holds diagnostic counters only (never an ecology input); not emitted so
        // existing audit baselines are unchanged.
        // The lineage observer alone enables these two output queues and drains
        // them within each tick. They are never inputs to the ecology.
        assert_eq!(resistance_event_logging, herbivore_terminal_logging);
        assert!(resistance_birth_events.is_empty() && herbivore_terminal_events.is_empty());

        let mut bytes = Vec::new();
        macro_rules! emit {
            ($index:expr, $value:expr) => {
                if section.is_none() || section == Some($index) {
                    append(&mut bytes, $value);
                }
            };
        }
        emit!(0, tick);
        emit!(1, config);
        emit!(2, &(spontaneous_germination, spontaneous_births));
        emit!(3, entities);
        emit!(4, resources);
        emit!(5, &(ledger, baseline));
        emit!(6, land);
        emit!(7, plants);
        emit!(8, animals);
        emit!(9, carnivores);
        emit!(10, plant_grid);
        emit!(11, rng);
        emit!(
            12,
            &(
                births,
                deaths,
                carnivore_births,
                carnivore_deaths,
                carnivore_hunts
            )
        );
        emit!(13, diagnostics);
        emit!(14, carnivore_debug);
        emit!(15, prey_trend_snapshot);
        emit!(16, plant_color_learning);
        emit!(
            17,
            &(
                focal_trace_id,
                focal_upkeep_override,
                resistance_upkeep_exempt,
                focal_trace
            )
        );
        emit!(18, &(behavior_trace_window, behavior_trace_subject));
        emit!(19, health_trace);
        emit!(20, energy_trace);
        emit!(21, foraging_trace);
        emit!(22, &(color_event_logging, color_events));
        emit!(23, p42_life_events);
        if legacy {
            assert!(hungry_color_recovery.is_empty());
            assert!(!config.animal_toxin.enabled && !config.animal_toxin.color_learning_enabled);
            assert_eq!(
                bincode::serialize(animal_toxins).unwrap(),
                bincode::serialize(&super::animal_toxin::AnimalToxins::default()).unwrap(),
                "stage 3 state must be empty in legacy mode"
            );
        } else {
            emit!(24, animal_toxins);
            emit!(25, hungry_color_recovery);
        }
        // Opt-in counterattack retreat state; empty unless that option is used, so existing
        // audit baselines are unchanged.
        if !carnivore_retreats.is_empty() {
            emit!(26, carnivore_retreats);
        }
        // Opt-in hunting bouts (Issue 11); empty unless that option is used.
        if !carnivore_hunt_bouts.is_empty() {
            emit!(27, carnivore_hunt_bouts);
        }
        // Opt-in diet values (Issue #78); empty unless `diet_gene` is set.
        if !diet.is_empty() {
            emit!(28, diet);
        }
        bytes
    }
}

fn append(bytes: &mut Vec<u8>, value: &impl Serialize) {
    bincode::serialize_into(bytes, value).expect("writing audit bytes to Vec cannot fail");
}

#[cfg(test)]
mod tests {
    use crate::{SimulationConfig, World};

    #[test]
    fn detects_signed_zero_and_rng_changes_without_mutating_world() {
        let mut config = SimulationConfig::default();
        config.initial_population.herbivores = 2;
        let mut left = World::new(config.clone()).unwrap();
        let mut right = World::new(config).unwrap();
        assert_eq!(left.audit_state(true, None), right.audit_state(true, None));
        left.animals.items[0].x = 0.;
        right.animals.items[0].x = -0.;
        assert_ne!(
            left.audit_state(true, Some(8)),
            right.audit_state(true, Some(8))
        );
        right.animals.items[0].x = 0.;
        left.rng.next_u64();
        assert_ne!(
            left.audit_state(true, Some(11)),
            right.audit_state(true, Some(11))
        );
        right.rng.next_u64();
        let baseline = left.audit_state(true, None);
        assert_eq!(baseline, left.audit_state(true, None));
        assert_eq!(baseline, right.audit_state(true, None));
        for _ in 0..8 {
            left.audit_state(true, None);
            left.step(1).unwrap();
            right.step(1).unwrap();
        }
        assert_eq!(left.audit_state(true, None), right.audit_state(true, None));
    }
}
