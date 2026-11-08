use sim_core::world::SpontaneousGermination;
use sim_core::{SimulationConfig, World};
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub struct Simulation {
    world: World,
    lineage: Option<sim_core::lineage::LineageObservation>,
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
        serde_json::json!({"tick":self.world.tick,"plants":format!("{:?}",self.world.plants),"animals":format!("{:?}",self.world.animals),"carnivores":format!("{:?}",self.world.carnivores),"resources":format!("{:?}",self.world.resources),"ledger":format!("{:?}",self.world.ledger),"entities":format!("{:?}",self.world.entities),"land":format!("{:?}",self.world.land),"diagnostics":self.world.diagnostics,"carnivore_debug":self.world.carnivore_debug,"rng":format!("{:?}",self.world.rng)}).to_string()
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
