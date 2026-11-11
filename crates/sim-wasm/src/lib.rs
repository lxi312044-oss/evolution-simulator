use sim_core::world::SpontaneousGermination;
use sim_core::{SimulationConfig, World};
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub struct Simulation {
    world: World,
    lineage: Option<sim_core::lineage::LineageObservation>,
    focal_intervention_applied: bool,
}
#[wasm_bindgen]
impl Simulation {
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: &str) -> Result<Simulation, JsValue> {
        let config: SimulationConfig =
            serde_json::from_str(config_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let world = World::new(config).map_err(|e| JsValue::from_str(&e))?;
        Ok(Self {
            world,
            lineage: None,
            focal_intervention_applied: false,
        })
    }
    /// Same soil-driven germination as the Phase 07.2.25 batch (p50, threshold 20, p=0.005, interval 20, max 20).
    pub fn enable_standard_germination(&mut self) -> Result<(), JsValue> {
        if self.lineage.is_some() {
            return Err(JsValue::from_str(
                "configure germination before lineage observation",
            ));
        }
        let settings = SpontaneousGermination {
            soil_threshold: 20.0,
            probability_per_cell: 0.005,
            check_interval: 20,
            max_births_per_check: 20,
        };
        settings.validate().map_err(|e| JsValue::from_str(&e))?;
        self.world.spontaneous_germination = Some(settings);
        Ok(())
    }
    /// Stable founder IDs, including generation to reject recycled IDs.
    pub fn intervention_candidates_json(&self) -> String {
        let mut ids: Vec<_> = self.world.animals.items.iter().map(|a| a.id).collect();
        ids.sort();
        serde_json::to_string(
            &ids.iter()
                .map(|id| [id.index, id.generation])
                .collect::<Vec<_>>(),
        )
        .unwrap()
    }
    /// Initial founder attributes for selection based only on pre-intervention state.
    pub fn founder_metadata_json(&self) -> Result<String, JsValue> {
        if self.world.tick != 0 {
            return Err(JsValue::from_str("founder metadata requires Tick 0"));
        }
        let resources = &self.world.resources;
        let rows: Vec<_> = self
            .world
            .animals
            .items
            .iter()
            .map(|a| {
                let account = |id| {
                    serde_json::json!({
                        "n":resources.nutrient_balance(id), "e":resources.energy_balance(id)
                    })
                };
                serde_json::json!({"id":[a.id.index,a.id.generation],"age":a.age,"hp":a.hp,
                "x":a.x,"y":a.y,"genes":format!("{:?}",a.genes),
                "body":account(a.body),"reserve":account(a.reserve),"stomach":account(a.stomach)})
            })
            .collect();
        Ok(serde_json::to_string(&rows).unwrap())
    }

    /// Research-only, once at Tick 0, before observation. Changes one scalar gene.
    pub fn intervene_focal_resistance(
        &mut self,
        index: u32,
        generation: u32,
        value: f32,
    ) -> Result<String, JsValue> {
        if self.world.tick != 0 || self.lineage.is_some() || self.focal_intervention_applied {
            return Err(JsValue::from_str(
                "intervention requires Tick 0 before observation, once",
            ));
        }
        if !value.is_finite()
            || !(0.0..=1.0).contains(&value)
            || self.world.config.toxin.herbivore_multi_toxin_enabled
        {
            return Err(JsValue::from_str(
                "intervention requires finite [0,1] resistance and single-toxin mode",
            ));
        }
        let a = self
            .world
            .animals
            .items
            .iter_mut()
            .find(|a| a.id.index == index && a.id.generation == generation)
            .ok_or_else(|| JsValue::from_str("focal herbivore ID not found"))?;
        let before = format!("{a:?}");
        let old = a.genes.toxin_resistance;
        a.genes.toxin_resistance = value;
        let after = format!("{a:?}");
        self.focal_intervention_applied = true;
        Ok(serde_json::json!({"id":[index,generation],"old":old,"value":value,"before":before,"after":after,"age":a.age,"max_age":a.max_age}).to_string())
    }
    /// Research-only override of one founder's upkeep input, independent of defense.
    pub fn override_focal_upkeep(
        &mut self,
        index: u32,
        generation: u32,
        value: f32,
    ) -> Result<(), JsValue> {
        let id = sim_core::entity::EntityId { index, generation };
        if self.world.tick != 0
            || self.lineage.is_some()
            || self.world.energy_trace.is_some()
            || self.world.health_trace.is_some()
            || self.world.focal_upkeep_override.is_some()
            || self.world.config.toxin.herbivore_multi_toxin_enabled
            || !self.world.animals.items.iter().any(|a| a.id == id)
            || !(0.0..=1.0).contains(&value)
        {
            return Err(JsValue::from_str("upkeep override requires a live founder at Tick 0 before observation, once, with a finite single-toxin resistance in [0,1]"));
        }
        self.world.focal_upkeep_override = Some((id, value));
        Ok(())
    }

    /// Configure the focal reserve trace and optional upkeep exemption at Tick 0.
    pub fn configure_upkeep_probe(
        &mut self,
        index: u32,
        generation: u32,
        exempt: bool,
    ) -> Result<(), JsValue> {
        let id = sim_core::entity::EntityId { index, generation };
        if self.world.tick != 0
            || self.lineage.is_some()
            || self.world.energy_trace.is_some()
            || !self.world.animals.items.iter().any(|a| a.id == id)
        {
            return Err(JsValue::from_str(
                "upkeep probe requires a live founder at Tick 0 before observation, once",
            ));
        }
        self.world.energy_trace = Some(sim_core::world::EnergyTrace::new([id]));
        self.world.resistance_upkeep_exempt = exempt.then_some(id);
        Ok(())
    }

    /// Start a single recovery-phase probe on an existing high-resistance herbivore.
    pub fn configure_recovery_upkeep_probe(
        &mut self,
        index: u32,
        generation: u32,
        exempt: bool,
    ) -> Result<(), JsValue> {
        let id = sim_core::entity::EntityId { index, generation };
        if self.world.tick == 0
            || self.world.energy_trace.is_some()
            || self.world.focal_trace_id.is_some()
            || !self
                .world
                .animals
                .items
                .iter()
                .any(|a| a.id == id && a.hp > 0.0 && a.genes.toxin_resistance >= 0.6)
        {
            return Err(JsValue::from_str("recovery probe requires an untraced living high-resistance individual after Tick 0"));
        }
        self.world.energy_trace = Some(sim_core::world::EnergyTrace::new([id]));
        self.world.focal_trace_id = Some(id);
        self.world.resistance_upkeep_exempt = exempt.then_some(id);
        Ok(())
    }

    pub fn take_upkeep_probe_json(&mut self) -> String {
        let Some(trace) = self.world.energy_trace.as_mut() else {
            return "null".into();
        };
        let events = std::mem::take(&mut trace.events);
        let focal = self.world.animals.items.iter().find(|a| trace.watched.contains(&a.id)).map(|a| {
            serde_json::json!({"hp":a.hp,"reserve_e":self.world.resources.energy_balance(a.reserve).unwrap_or(0.),
                "dose":a.lifetime_typed_dose[0],"damage":a.lifetime_toxin_damage,
                "upkeep":a.lifetime_resistance_cost,"births":a.successful_births})
        });
        let focal_trace = std::mem::take(&mut self.world.focal_trace);
        serde_json::json!({"tick":self.world.tick,"focal":focal,"energy":events,"trace":focal_trace,
            "plants":self.world.plants.items.len(),
            "toxic_plants":self.world.plants.items.iter().filter(|p| p.produces_toxin).count()})
        .to_string()
    }

    /// Observe fixed IDs, including future births, without changing their state.
    pub fn configure_health_observation(
        &mut self,
        ids_json: &str,
        start: u64,
        end: u64,
    ) -> Result<(), JsValue> {
        if self.world.tick != 0 || self.world.health_trace.is_some() || start > end {
            return Err(JsValue::from_str(
                "health observation requires Tick 0, once, and a valid window",
            ));
        }
        let ids: Vec<[u32; 2]> =
            serde_json::from_str(ids_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let mut trace = sim_core::world::HealthTrace::new(
            ids.into_iter()
                .map(|[index, generation]| sim_core::entity::EntityId { index, generation }),
            (start, end),
        );
        trace.observe_reproduction = true;
        self.world.health_trace = Some(trace);
        Ok(())
    }

    pub fn take_health_observation_json(&mut self) -> String {
        let events = self
            .world
            .health_trace
            .as_mut()
            .map(|trace| std::mem::take(&mut trace.events));
        serde_json::to_string(&events.unwrap_or_default()).unwrap()
    }

    /// Observe intake and reserve transactions for the existing health cohort after the trough.
    pub fn enable_cohort_foraging_observation(&mut self) -> Result<(), JsValue> {
        let Some(trace) = &self.world.health_trace else {
            return Err(JsValue::from_str("health cohort is required"));
        };
        if self.world.tick != trace.window.0
            || self.world.foraging_trace.is_some()
            || self.world.energy_trace.is_some()
        {
            return Err(JsValue::from_str(
                "foraging observation must start once at the cohort boundary",
            ));
        }
        self.world.foraging_trace = Some(sim_core::world::ForagingTrace::new(
            trace
                .watched
                .iter()
                .map(|id| sim_core::world::ForagingWindow {
                    id: [id.index, id.generation],
                    start: trace.window.0 + 1,
                    end: trace.window.1,
                })
                .collect(),
        ));
        self.world.energy_trace = Some(sim_core::world::EnergyTrace::new(
            trace.watched.iter().copied(),
        ));
        Ok(())
    }

    pub fn take_cohort_foraging_json(&mut self) -> String {
        let decisions = self
            .world
            .foraging_trace
            .as_mut()
            .map(|t| std::mem::take(&mut t.events));
        let energy = self
            .world
            .energy_trace
            .as_mut()
            .map(|t| std::mem::take(&mut t.events));
        serde_json::json!({"decisions":decisions.unwrap_or_default(),"energy":energy.unwrap_or_default()}).to_string()
    }

    pub fn health_observation_status_json(&self) -> String {
        let mut rows = Vec::new();
        if let Some(trace) = &self.world.health_trace {
            for animal in &self.world.animals.items {
                if trace.watched.contains(&animal.id) {
                    rows.push(serde_json::json!({"id":[animal.id.index,animal.id.generation],
                        "hp":animal.hp,"age":animal.age,"resistance":animal.genes.toxin_resistance,
                        "hp_threshold":animal.genes.adult_max_hp * self.world.config.herbivores.reproduction_min_hp_fraction}));
                }
            }
        }
        serde_json::to_string(&rows).unwrap()
    }

    /// Resistance distribution among herbivores with positive HP.
    pub fn living_resistance_distribution_json(&self) -> String {
        let mut counts = [0u32; 3];
        let mut sum = 0.0;
        for animal in self.world.animals.items.iter().filter(|a| a.hp > 0.0) {
            let resistance = animal.genes.toxin_resistance;
            let bin = if resistance <= 0.4 {
                0
            } else if resistance >= 0.6 {
                2
            } else {
                1
            };
            counts[bin] += 1;
            sum += f64::from(resistance);
        }
        let living = counts.iter().sum::<u32>();
        serde_json::json!({
            "living": living,
            "counts": counts,
            "mean": (living > 0).then(|| sum / f64::from(living)),
            "plants": self.world.plants.items.len()
        })
        .to_string()
    }

    pub fn resistance_mean(&self) -> f64 {
        let a = &self.world.animals.items;
        if a.is_empty() {
            return f64::NAN;
        }
        a.iter()
            .map(|x| x.genes.toxin_resistance as f64)
            .sum::<f64>()
            / a.len() as f64
    }
    pub fn resistance_high_count(&self) -> u32 {
        self.world
            .animals
            .items
            .iter()
            .filter(|a| a.genes.toxin_resistance >= 0.6)
            .count() as u32
    }
    pub fn spontaneous_plant_births(&self) -> f64 {
        self.world.spontaneous_births as f64
    }
    pub fn step(&mut self, count: u32) -> Result<(), JsValue> {
        if count > self.world.config.simulation.max_steps_per_call {
            return Err(JsValue::from_str("step count exceeds max_steps_per_call"));
        }
        if let Some(o) = &mut self.lineage {
            for _ in 0..count {
                o.before_step(&self.world)
                    .map_err(|e| JsValue::from_str(&e))?;
                self.world.step(1).map_err(|e| JsValue::from_str(&e))?;
                if self.world.foraging_trace.is_some() {
                    self.world.checkpoint_energy();
                }
                o.after_step(&mut self.world)
                    .map_err(|e| JsValue::from_str(&e))?;
            }
            Ok(())
        } else {
            self.world.step(count).map_err(|e| JsValue::from_str(&e))
        }
    }
    /// Opt-in at Tick 0. Does not change ecological calculations or RNG.
    pub fn enable_lineage_observation(&mut self, ticks: u64) -> Result<(), JsValue> {
        if self.lineage.is_some() {
            return Err(JsValue::from_str("lineage observation already enabled"));
        }
        self.lineage = Some(
            sim_core::lineage::LineageObservation::new(&mut self.world, ticks)
                .map_err(|e| JsValue::from_str(&e))?,
        );
        Ok(())
    }
    pub fn take_lineage_events_json(&mut self) -> String {
        serde_json::to_string(
            &self
                .lineage
                .as_mut()
                .map(|o| o.take_events())
                .unwrap_or_default(),
        )
        .unwrap()
    }
    pub fn finish_lineage_observation(&mut self) -> Result<(), JsValue> {
        self.lineage
            .as_mut()
            .ok_or_else(|| JsValue::from_str("lineage observation is disabled"))?
            .finish(&self.world)
            .map_err(|e| JsValue::from_str(&e))
    }
    /// Exact state on the same target for observation ON/OFF verification.
    pub fn observation_state_json(&self) -> String {
        let mut state = serde_json::json!({"tick":self.world.tick,"plants":format!("{:?}",self.world.plants),"animals":format!("{:?}",self.world.animals),"carnivores":format!("{:?}",self.world.carnivores),"resources":format!("{:?}",self.world.resources),"ledger":format!("{:?}",self.world.ledger),"entities":format!("{:?}",self.world.entities),"land":format!("{:?}",self.world.land),"diagnostics":self.world.diagnostics,"carnivore_debug":self.world.carnivore_debug,"rng":format!("{:?}",self.world.rng)});
        if self.world.config.herbivores.color_learning_enabled {
            state["plant_color_learning"] =
                serde_json::json!(format!("{:?}", self.world.plant_color_learning));
        }
        if self.world.config.animal_toxin.enabled {
            state["animal_toxin"] = serde_json::json!(self.world.animal_toxin_observation_state());
        }
        state.to_string()
    }
    pub fn animal_toxin_snapshot_json(&self) -> String {
        self.world.animal_toxin_snapshot_json()
    }
    /// Test-only snapshot. The observer's output queues are not simulation inputs.
    #[cfg(feature = "state-audit")]
    pub fn audit_state(&self, legacy: bool, section: Option<u32>) -> Vec<u8> {
        let mut state = self.world.audit_state(legacy, section);
        if section.is_none() || section == Some(30) {
            state.push(u8::from(self.focal_intervention_applied));
        }
        state
    }
    pub fn tick_count(&self) -> u64 {
        self.world.tick
    }
    pub fn animal_count(&self) -> u32 {
        self.world.animal_count()
    }
    pub fn herbivore_count(&self) -> u32 {
        self.world.herbivore_count()
    }
    pub fn herbivore_energy(&self) -> f64 {
        self.world.herbivore_energy()
    }
    pub fn carnivore_count(&self) -> u32 {
        self.world.carnivores.len() as u32
    }
    pub fn carnivore_hunts(&self) -> u64 {
        self.world.carnivore_hunts
    }
    pub fn carnivore_births(&self) -> u64 {
        self.world.carnivore_births
    }
    pub fn carnivore_deaths(&self) -> u64 {
        self.world.carnivore_deaths
    }
    pub fn carnivore_debug_json(&self) -> String {
        serde_json::to_string(&self.world.carnivore_debug).unwrap_or_else(|_| "{}".into())
    }
    pub fn carnivore_snapshot_json(&self) -> String {
        serde_json::to_string(&self.world.carnivore_snapshot()).unwrap_or_else(|_| "[]".into())
    }
    pub fn herbivore_color_snapshot_json(&self) -> String {
        serde_json::to_string(&self.world.herbivore_color_snapshot())
            .unwrap_or_else(|_| "[]".into())
    }
    pub fn herbivore_snapshot_json(&self) -> String {
        serde_json::to_string(&self.world.herbivore_snapshot()).unwrap_or_else(|_| "[]".into())
    }
    pub fn plant_count(&self) -> u32 {
        self.world.plant_count()
    }
    pub fn soil_nutrients(&self) -> f64 {
        self.world.total_soil()
    }
    pub fn detritus_nutrients(&self) -> f64 {
        self.world.total_detritus()
    }
    pub fn total_nutrients(&self) -> f64 {
        self.world.resources.total_nutrients()
    }
    pub fn total_energy(&self) -> f64 {
        self.world.resources.total_energy()
    }
    pub fn ecology_snapshot_json(&self) -> String {
        self.world.ecology_snapshot().to_string()
    }
    pub fn diagnostics_json(&self) -> String {
        serde_json::to_string(&self.world.diagnostics).unwrap_or_else(|_| "{}".into())
    }
    pub fn sunlight_in(&self) -> f64 {
        self.world.ledger.sunlight_in
    }
    pub fn heat_out(&self) -> f64 {
        self.world.ledger.heat_out
    }
    pub fn births(&self) -> u64 {
        self.world.births
    }
    pub fn deaths(&self) -> u64 {
        self.world.deaths
    }
    pub fn plant_snapshot_json(&self) -> String {
        serde_json::to_string(&self.world.render_snapshot()).unwrap_or_else(|_| "[]".into())
    }
}
