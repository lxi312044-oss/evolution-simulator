use crate::{
    animals::{AnimalStore, Herbivore, HerbivoreGenes},
    config::SimulationConfig,
    entity::EntityAllocator,
    land::{LandCell, LandGrid},
    plants::{Plant, PlantStore},
    resources::{ConservationBaseline, ResourceBank, ResourceLedger},
    rng::SimulationRng,
    spatial::PlantGrid,
};
#[derive(Debug)]
pub struct World {
    pub tick: u64,
    pub config: SimulationConfig,
    pub spontaneous_germination: Option<SpontaneousGermination>,
    pub spontaneous_births: u64,
    pub entities: EntityAllocator,
    pub resources: ResourceBank,
    pub ledger: ResourceLedger,
    pub baseline: ConservationBaseline,
    pub land: LandGrid,
    pub plants: PlantStore,
    pub animals: AnimalStore,
    pub carnivores: Vec<crate::animals::carnivore::Carnivore>,
    pub plant_grid: PlantGrid,
    pub rng: SimulationRng,
    pub births: u64,
    pub deaths: u64,
    pub carnivore_births: u64,
    pub carnivore_deaths: u64,
    pub carnivore_hunts: u64,
    pub diagnostics: super::Diagnostics,
    pub carnivore_debug: super::CarnivoreDebug,
    pub prey_trend_snapshot: super::PreyTrendSnapshot,
    pub resistance_event_logging: bool,
    pub resistance_birth_events: Vec<super::ResistanceBirthEvent>,
    pub herbivore_terminal_logging: bool,
    pub herbivore_terminal_events: Vec<super::HerbivoreTerminalEvent>,
    /// Optional focal-only diagnostic trace; never consumes random numbers.
    pub focal_trace_id: Option<crate::entity::EntityId>,
    pub focal_trace: Vec<serde_json::Value>,
    pub behavior_trace_window: Option<(u64, u64)>,
    pub behavior_trace_subject: Option<crate::entity::EntityId>,
    pub health_trace: Option<super::HealthTrace>,
    pub energy_trace: Option<super::EnergyTrace>,
    pub color_event_logging: bool,
    pub color_events: Vec<super::ColorBirthEvent>,
    pub p42_life_events: Vec<super::CarnivoreLifeEvent>,
}
/// Phase 07.2.19 opt-in experimental soil-driven germination.
#[derive(Clone, Copy, Debug)]
pub struct SpontaneousGermination {
    pub soil_threshold: f64,
    pub probability_per_cell: f64,
    pub check_interval: u64,
    pub max_births_per_check: usize,
}
impl SpontaneousGermination {
    pub fn validate(&self) -> Result<(), String> {
        if !self.soil_threshold.is_finite()
            || self.soil_threshold < 0.
            || !self.probability_per_cell.is_finite()
            || !(0. ..=1.).contains(&self.probability_per_cell)
            || self.check_interval == 0
            || self.max_births_per_check == 0
        {
            return Err("invalid spontaneous germination parameters".into());
        }
        Ok(())
    }
}
impl World {
    pub fn new(config: SimulationConfig) -> Result<Self, String> {
        config.validate()?;
        let mut resources = ResourceBank::new();
        let mut cells = Vec::new();
        for _ in 0..config.world.land_columns * config.world.land_rows {
            let soil = resources
                .open(config.plants.initial_soil_nutrients_per_cell, 0.)
                .map_err(|e| format!("{e:?}"))?;
            let detritus = resources
                .open(config.plants.initial_detritus_per_cell, 0.)
                .map_err(|e| format!("{e:?}"))?;
            cells.push(LandCell { soil, detritus });
        }
        let mut world = Self {
            tick: 0,
            rng: SimulationRng::new(config.seed),
            config,
            spontaneous_germination: None,
            spontaneous_births: 0,
            entities: EntityAllocator::new(),
            resources,
            ledger: ResourceLedger::default(),
            baseline: ConservationBaseline {
                nutrients: 0.,
                energy: 0.,
            },
            land: LandGrid {
                columns: 0,
                rows: 0,
                cells,
            },
            plants: PlantStore::default(),
            animals: AnimalStore::default(),
            carnivores: Vec::new(),
            plant_grid: PlantGrid::new(1., 1., 50.),
            births: 0,
            deaths: 0,
            carnivore_births: 0,
            carnivore_deaths: 0,
            carnivore_hunts: 0,
            diagnostics: super::Diagnostics::default(),
            carnivore_debug: super::CarnivoreDebug::default(),
            prey_trend_snapshot: super::PreyTrendSnapshot::default(),
            resistance_event_logging: false,
            resistance_birth_events: Vec::new(),
            herbivore_terminal_logging: false,
            herbivore_terminal_events: Vec::new(),
            focal_trace_id: None,
            focal_trace: Vec::new(),
            behavior_trace_window: None,
            behavior_trace_subject: None,
            health_trace: None,
            energy_trace: None,
            color_event_logging: false,
            color_events: Vec::new(),
            p42_life_events: Vec::new(),
        };
        world.land.columns = world.config.world.land_columns;
        world.land.rows = world.config.world.land_rows;
        for _ in 0..world.config.initial_population.plants {
            let x = world.rng.unit() * world.config.world.width;
            let y = world.rng.unit() * world.config.world.height;
            world.create_initial_plant(x, y)?;
        }
        world.plant_grid = PlantGrid::new(world.config.world.width, world.config.world.height, 50.);
        world.plant_grid.rebuild(&world.plants);
        for _ in 0..world.config.initial_population.herbivores {
            let x = world.rng.unit() * world.config.world.width;
            let y = world.rng.unit() * world.config.world.height;
            world.create_initial_herbivore(x, y)?;
        }
        if world.config.carnivores.enabled {
            for _ in 0..world.config.initial_population.carnivores {
                let x = world.rng.unit() * world.config.world.width;
                let y = world.rng.unit() * world.config.world.height;
                world.create_initial_carnivore(x, y)?;
            }
        }
        world.baseline = ConservationBaseline::capture(&world.resources);
        Ok(world)
    }
    fn create_initial_plant(&mut self, x: f32, y: f32) -> Result<(), String> {
        let cell = self
            .land
            .cell_index(x, y, self.config.world.width, self.config.world.height);
        let id = self.entities.spawn()?;
        let body = self
            .resources
            .open(self.config.plants.initial_body_nutrients, 0.)
            .map_err(|e| format!("{e:?}"))?;
        let reserve = self
            .resources
            .open(0., self.config.plants.initial_energy)
            .map_err(|e| format!("{e:?}"))?;
        self.plants.items.push(Plant {
            id,
            x,
            y,
            cell,
            body,
            reserve,
            growth: 0.,
            age: 0,
            seed_cooldown: self.config.plants.seed_cooldown_ticks,
            max_age: self.config.plants.max_age_ticks,
            color: 0x4ecb70ff,
            produces_toxin: crate::toxin::founder_plant_toxic(
                self.config.seed,
                id.index as u64,
                self.config.toxin.plant_variation_enabled,
                self.config.toxin.initial_toxic_plant_fraction,
            ),
            toxin_type: crate::toxin::founder_plant_toxin_type_biased(
                self.config.seed,
                id.index as u64,
                self.config.toxin.plant_toxin_type_variation_enabled,
                self.config.toxin.plant_toxin_type_count,
                self.config.toxin.initial_type0_fraction,
            ),
        });
        Ok(())
    }
    fn create_initial_herbivore(&mut self, x: f32, y: f32) -> Result<(), String> {
        let h = &self.config.herbivores;
        let hp = 45. + 30. * self.rng.unit();
        let speed = h.speed_min + (h.speed_max - h.speed_min) * self.rng.unit();
        let vision = h.vision_min + (h.vision_max - h.vision_min) * self.rng.unit();
        let energy = hp as f64 * h.max_energy_per_hp * h.initial_energy_fraction;
        let id = self.entities.spawn()?;
        let body = self
            .resources
            .open(h.initial_body_nutrients, 0.)
            .map_err(|e| format!("{e:?}"))?;
        let reserve = self
            .resources
            .open(h.initial_reserve_nutrients, energy)
            .map_err(|e| format!("{e:?}"))?;
        // Individual lifetimes and initial ages prevent a synchronized die-off at world tick 6000.
        let max_age = ((h.max_age_ticks as f64) * (0.8 + 0.4 * self.rng.unit() as f64))
            .round()
            .max(1.) as u32;
        let age = (max_age as f64 * 0.15 * self.rng.unit() as f64) as u32;
        let exploration_heading = self.rng.unit() * std::f32::consts::TAU;
        let exploration_ticks = 30 + (self.rng.next_u64() % 71) as u32;
        let stomach = self.resources.open(0., 0.).map_err(|e| format!("{e:?}"))?;
        self.animals.items.push(Herbivore {
            id,
            x,
            y,
            hp,
            growth: 1.,
            genes: HerbivoreGenes {
                adult_max_hp: hp,
                speed,
                vision,
                feeding_range: h.feeding_range,
                body_color: crate::animals::color::initial(self.config.seed, id.index as u64),
                fear: 0.45 + 0.4 * self.rng.unit(),
                aggression: 0.15 + 0.6 * self.rng.unit(),
                toxin_resistance: self.config.toxin.initial_resistance as f32,
                typed_resistance: [self.config.toxin.initial_resistance as f32; 16],
            },
            learned_fear: 0.,
            defense_active: false,
            toxin_load: 0.,
            typed_load: [0.; 16],
            lifetime_toxin_dose: 0.,
            lifetime_typed_dose: [0.; 16],
            lifetime_toxin_damage: 0.,
            lifetime_prevented_damage: 0.,
            lifetime_resistance_cost: 0.,
            reproduction_age_reached: false,
            successful_births: 0,
            body,
            reserve,
            stomach,
            feeding_active: true,
            well_fed_ticks: 0,
            age,
            max_age,
            reproduction_cooldown: h.reproduction_cooldown_ticks,
            exploration_heading,
            exploration_ticks,
            target: None,
        });
        Ok(())
    }
    fn create_initial_carnivore(&mut self, x: f32, y: f32) -> Result<(), String> {
        let c = &self.config.carnivores;
        let id = self.entities.spawn()?;
        let body = self
            .resources
            .open(c.initial_body_nutrients, 0.)
            .map_err(|e| format!("{e:?}"))?;
        let reserve = self
            .resources
            .open(c.initial_reserve_nutrients, c.initial_energy)
            .map_err(|e| format!("{e:?}"))?;
        let stomach = self.resources.open(0., 0.).map_err(|e| format!("{e:?}"))?;
        self.carnivores.push(crate::animals::carnivore::Carnivore {
            id,
            x,
            y,
            age: 0,
            max_age: c.max_age_ticks,
            cooldown: c.reproduction_cooldown_ticks,
            attack_cooldown: 0,
            satiety_ticks: 0,
            stomach,
            feeding_active: true,
            body,
            reserve,
            color: crate::animals::color::initial(self.config.seed ^ 0xabcdef, id.index as u64),
            speed: c.speed,
            vision: c.vision,
            ambush_gene: 0.15
                + 0.70
                    * ((id.index as u64)
                        .wrapping_mul(0x9e3779b97f4a7c15)
                        .rotate_left(17) as u32 as f32
                        / u32::MAX as f32),
            p42: Default::default(),
            parent_id: None,
            search_heading: 0.,
            search_ticks_left: 0,
            last_prey_heading: 0.,
            last_prey_ticks_left: 0,
        });
        Ok(())
    }
    pub fn carnivore_snapshot(&self) -> Vec<[f32; 7]> {
        self.carnivores
            .iter()
            .map(|a| [a.x, a.y, 6., 1., a.color.r, a.color.g, a.color.b])
            .collect()
    }
    /// Enable detailed birth events before stepping; defaults to off for low overhead.
    pub fn set_color_event_logging(&mut self, enabled: bool) {
        self.color_event_logging = enabled;
        if !enabled {
            self.color_events.clear();
        }
    }
    /// Drain events regularly (e.g. each sample interval) to bound memory use.
    pub fn take_color_events(&mut self) -> Vec<super::ColorBirthEvent> {
        std::mem::take(&mut self.color_events)
    }
    /// Read-only population statistics. Zero population produces zero-valued means/variances.
    pub fn color_statistics(&self) -> super::ColorStatistics {
        let n = self.animals.items.len();
        let mut mean = [0f64; 3];
        let mut variance = [0f64; 3];
        if n > 0 {
            for a in &self.animals.items {
                for (i, v) in a.genes.body_color.rgb().iter().enumerate() {
                    mean[i] += *v as f64;
                }
            }
            for v in &mut mean {
                *v /= n as f64;
            }
            for a in &self.animals.items {
                for (i, v) in a.genes.body_color.rgb().iter().enumerate() {
                    variance[i] += (*v as f64 - mean[i]).powi(2);
                }
            }
            for v in &mut variance {
                *v /= n as f64;
            }
        }
        super::ColorStatistics {
            population: n as u64,
            mean,
            variance,
            inheritances: self.diagnostics.color_inheritances,
            mutated_births: self.diagnostics.color_mutated_births,
            mutated_channels: self.diagnostics.color_mutated_channels,
        }
    }
    /// Phase 06.2.34: read-only evolutionary trait snapshot; does not consume RNG.
    pub fn behavior_genetics_snapshot(&self) -> serde_json::Value {
        let count = self.animals.items.len();
        let mut fear = Vec::with_capacity(count);
        let mut aggression = Vec::with_capacity(count);
        let mut learned = Vec::with_capacity(count);
        let mut defensive = 0usize;
        for animal in &self.animals.items {
            fear.push(animal.genes.fear as f64);
            aggression.push(animal.genes.aggression as f64);
            learned.push(animal.learned_fear as f64);
            defensive += usize::from(animal.defense_active);
        }
        fn stats(values: &mut [f64]) -> (f64, f64, f64, f64) {
            if values.is_empty() {
                return (0.0, 0.0, 0.0, 0.0);
            }
            values.sort_by(|a, b| a.total_cmp(b));
            let n = values.len();
            let mean = values.iter().sum::<f64>() / n as f64;
            let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
            let median = if n.is_multiple_of(2) {
                (values[n / 2 - 1] + values[n / 2]) / 2.0
            } else {
                values[n / 2]
            };
            (mean, variance, median, values[n - 1] - values[0])
        }
        let (fear_mean, fear_variance, fear_median, fear_range) = stats(&mut fear);
        let (aggression_mean, aggression_variance, aggression_median, aggression_range) =
            stats(&mut aggression);
        let (learned_mean, learned_variance, _, _) = stats(&mut learned);
        serde_json::json!({
            "herbivores": count, "defense_active": defensive,
            "fear_mean": fear_mean, "fear_variance": fear_variance,
            "fear_median": fear_median, "fear_range": fear_range,
            "aggression_mean": aggression_mean, "aggression_variance": aggression_variance,
            "aggression_median": aggression_median, "aggression_range": aggression_range,
            "learned_fear_mean": learned_mean, "learned_fear_variance": learned_variance,
            "carnivore_ambush_gene_mean": if self.carnivores.is_empty(){0.}else{self.carnivores.iter().map(|c|c.ambush_gene as f64).sum::<f64>()/self.carnivores.len() as f64},
            "carnivore_ambush_gene_variance": if self.carnivores.is_empty(){0.}else{let mean=self.carnivores.iter().map(|c|c.ambush_gene as f64).sum::<f64>()/self.carnivores.len() as f64;self.carnivores.iter().map(|c|(c.ambush_gene as f64-mean).powi(2)).sum::<f64>()/self.carnivores.len() as f64}
        })
    }

    // Read-only snapshot: spatial nutrient availability and plant age distribution.
    // No RNG or resource transfers, so diagnostics cannot change simulation dynamics.
    pub fn ecology_snapshot(&self) -> serde_json::Value {
        let n = self.land.cells.len();
        let mut plants_per_cell = vec![0u32; n];
        let mut soil_min = f64::INFINITY;
        let mut soil_max = 0f64;
        let mut soil_zero = 0u64;
        let mut soil_low = 0u64;
        for c in &self.land.cells {
            let v = self.resources.nutrient_balance(c.soil).unwrap_or(0.);
            soil_min = soil_min.min(v);
            soil_max = soil_max.max(v);
            if v <= 1e-9 {
                soil_zero += 1;
            }
            if v < self.config.plants.nutrient_half_saturation {
                soil_low += 1;
            }
        }
        let mut plant_reserve_n = 0f64;
        let mut plant_body_n = 0f64;
        let mut plant_reserve_e = 0f64;
        let mut immature = 0u64;
        let mut mature = 0u64;
        let mut near_age = 0u64;
        let mut growth_sum = 0f64;
        for plant in &self.plants.items {
            plants_per_cell[plant.cell] += 1;
            plant_reserve_n += self.resources.nutrient_balance(plant.reserve).unwrap_or(0.);
            plant_body_n += self.resources.nutrient_balance(plant.body).unwrap_or(0.);
            plant_reserve_e += self.resources.energy_balance(plant.reserve).unwrap_or(0.);
            growth_sum += plant.growth as f64;
            if plant.growth < 1. {
                immature += 1;
            } else {
                mature += 1;
            }
            if plant.max_age.saturating_sub(plant.age) <= 100 {
                near_age += 1;
            }
        }
        let occupied = plants_per_cell.iter().filter(|&&v| v > 0).count();
        let max_density = plants_per_cell.iter().copied().max().unwrap_or(0);
        let mut herbivore_body_n = 0f64;
        let mut herbivore_reserve_n = 0f64;
        let mut herbivore_stomach_n = 0f64;
        let mut herbivore_satiety_sum = 0f64;
        let mut herbivore_satiety_min = 1f64;
        let mut herbivore_satiety_max = 0f64;
        for a in &self.animals.items {
            herbivore_body_n += self.resources.nutrient_balance(a.body).unwrap_or(0.);
            herbivore_reserve_n += self.resources.nutrient_balance(a.reserve).unwrap_or(0.);
            let stomach_n = self.resources.nutrient_balance(a.stomach).unwrap_or(0.);
            herbivore_stomach_n += stomach_n;
            let fraction = (stomach_n / self.config.herbivores.stomach_capacity_n).clamp(0., 1.);
            herbivore_satiety_sum += fraction;
            herbivore_satiety_min = herbivore_satiety_min.min(fraction);
            herbivore_satiety_max = herbivore_satiety_max.max(fraction);
        }
        serde_json::json!({
            "soil_min_cell_n":if n==0 {0.}else{soil_min},"soil_max_cell_n":soil_max,
            "soil_depleted_cells":soil_zero,"soil_below_half_saturation_cells":soil_low,
            "plant_occupied_cells":occupied,"plant_max_per_cell":max_density,
            "plant_immature":immature,"plant_mature":mature,"plant_near_lifespan_end":near_age,
            "plant_mean_growth":if !self.plants.is_empty() {growth_sum/self.plants.len() as f64}else{0.},
            "plant_body_n":plant_body_n,"plant_reserve_n":plant_reserve_n,"plant_reserve_e":plant_reserve_e,
            "herbivore_body_n":herbivore_body_n,"herbivore_reserve_n":herbivore_reserve_n,
            "herbivore_stomach_n":herbivore_stomach_n,
            "herbivore_satiety_mean":if self.animals.items.is_empty(){0.}else{herbivore_satiety_sum/self.animals.items.len() as f64},
            "herbivore_satiety_min":if self.animals.items.is_empty(){0.}else{herbivore_satiety_min},
            "herbivore_satiety_max":herbivore_satiety_max,
        })
    }
    pub fn animal_count(&self) -> u32 {
        self.animals.items.len() as u32
    }
    pub fn herbivore_count(&self) -> u32 {
        self.animal_count()
    }
    pub fn herbivore_snapshot(&self) -> Vec<[f32; 4]> {
        self.animals
            .items
            .iter()
            .map(|a| [a.x, a.y, 4.5, a.hp / a.genes.adult_max_hp])
            .collect()
    }
    pub fn herbivore_color_snapshot(&self) -> Vec<[f32; 7]> {
        self.animals
            .items
            .iter()
            .map(|a| {
                let c = a.genes.body_color;
                [a.x, a.y, 4.5, a.hp / a.genes.adult_max_hp, c.r, c.g, c.b]
            })
            .collect()
    }
    pub fn herbivore_energy(&self) -> f64 {
        self.animals
            .items
            .iter()
            .map(|a| self.resources.energy_balance(a.reserve).unwrap_or(0.))
            .sum()
    }

    pub fn plant_count(&self) -> u32 {
        self.plants.len() as u32
    }
    pub fn total_soil(&self) -> f64 {
        self.land
            .cells
            .iter()
            .map(|c| self.resources.nutrient_balance(c.soil).unwrap_or(0.))
            .sum()
    }
    pub fn total_detritus(&self) -> f64 {
        self.land
            .cells
            .iter()
            .map(|c| self.resources.nutrient_balance(c.detritus).unwrap_or(0.))
            .sum()
    }
    pub fn render_snapshot(&self) -> Vec<[f32; 4]> {
        self.plants
            .items
            .iter()
            .map(|p| [p.x, p.y, 2.5 + 4.5 * p.growth, p.growth])
            .collect()
    }
}

impl World {
    pub fn p42_snapshot(
        &self,
        a: &crate::animals::carnivore::Carnivore,
        event: &'static str,
    ) -> super::CarnivoreLifeEvent {
        let c = &self.config.carnivores;
        let distances: Vec<f32> = self
            .animals
            .items
            .iter()
            .map(|h| ((h.x - a.x).powi(2) + (h.y - a.y).powi(2)).sqrt())
            .collect();
        let p = &a.p42;
        super::CarnivoreLifeEvent {
            tick: self.tick,
            event,
            id_index: a.id.index,
            id_generation: a.id.generation,
            age: a.age,
            ambush_gene: a.ambush_gene,
            parent_id_index: a.parent_id.map(|v| v.index),
            parent_id_generation: a.parent_id.map(|v| v.generation),
            relative_closer: p.relative_closer,
            relative_farther: p.relative_farther,
            relative_unchanged: p.relative_unchanged,
            longest_no_positive_attack_ticks: p.longest_no_positive_attack_ticks,
            low_energy_visible_ticks: p.low_energy_visible_ticks,
            reserve_energy: self.resources.energy_balance(a.reserve).unwrap_or(0.),
            stomach_energy: self.resources.energy_balance(a.stomach).unwrap_or(0.),
            nearest_prey_distance: distances.iter().copied().reduce(f32::min),
            visible_prey_count: distances.iter().filter(|d| **d <= a.vision).count(),
            attack_range_prey_count: distances.iter().filter(|d| **d <= c.attack_range).count(),
            feeding_active: a.feeding_active,
            ticks_since_last_positive_attack: p
                .last_positive_attack_tick
                .map(|t| self.tick.saturating_sub(t)),
            visible_ticks: p.visible_ticks,
            approach_ticks: p.approach_ticks,
            retreat_ticks: p.retreat_ticks,
            attack_range_ticks: p.attack_range_ticks,
            attacks: p.attacks,
            positive_energy_attacks: p.positive_energy_attacks,
            births_as_parent: p.births_as_parent,
            basal_spent: p.basal_spent,
            movement_spent: p.movement_spent,
            digestion_received: p.digestion_received,
            bite_energy_received: p.bite_energy_received,
            reproduction_energy_spent: p.reproduction_energy_spent,
        }
    }
}
