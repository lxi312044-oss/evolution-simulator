use super::World;
use crate::plants::Plant;
impl World {
    pub fn step(&mut self, count: u32) -> Result<(), String> {
        if count > self.config.simulation.max_steps_per_call {
            return Err("step count exceeds max_steps_per_call".into());
        }
        for _ in 0..count {
            self.step_one()?;
        }
        Ok(())
    }
    fn step_one(&mut self) -> Result<(), String> {
        self.tick = self.tick.checked_add(1).ok_or("tick overflow")?;
        let p = &self.config.plants;
        // 1. Organic detritus mineralizes into soil, preserving nutrient mass.
        for cell in &self.land.cells {
            let amount = self.resources.nutrient_balance(cell.detritus).unwrap_or(0.)
                * p.mineralization_rate;
            self.resources
                .transfer_nutrients(cell.detritus, cell.soil, amount)
                .map_err(|e| format!("mineralization: {e:?}"))?;
            self.diagnostics.mineralized_n += amount;
        }
        // 1b. Conservative soil diffusion: calculate all fluxes from a frozen soil snapshot.
        // A cell sends at most soil_diffusion_rate of its nutrients in total.
        if p.soil_diffusion_rate > 0. {
            let cols = self.land.columns as usize;
            let rows = self.land.rows as usize;
            let balances: Vec<f64> = self
                .land
                .cells
                .iter()
                .map(|c| self.resources.nutrient_balance(c.soil).unwrap_or(0.))
                .collect();
            let mut transfers = Vec::new();
            for row in 0..rows {
                for col in 0..cols {
                    let i = row * cols + col;
                    let neighbors = [
                        (col > 0).then(|| i - 1),
                        (col + 1 < cols).then(|| i + 1),
                        (row > 0).then(|| i - cols),
                        (row + 1 < rows).then(|| i + cols),
                    ];
                    let destinations: Vec<usize> = neighbors.into_iter().flatten().collect();
                    if destinations.is_empty() {
                        continue;
                    }
                    let per_neighbor =
                        balances[i] * p.soil_diffusion_rate / destinations.len() as f64;
                    for j in destinations {
                        transfers.push((i, j, per_neighbor));
                    }
                }
            }
            // Incoming transfers may arrive before outgoing transfers; outgoing totals
            // are bounded by the initial balance, avoiding negative soil accounts.
            for (i, j, amount) in transfers {
                if amount > 0. {
                    let source = self.land.cells[i].soil;
                    let destination = self.land.cells[j].soil;
                    self.resources
                        .transfer_nutrients(source, destination, amount)
                        .map_err(|e| format!("soil diffusion: {e:?}"))?;
                    self.diagnostics.soil_diffused_n += amount;
                }
            }
        }
        // 2. Photosynthesis and basal metabolism. Never exceed energy capacity.
        for plant in &self.plants.items {
            let current = self.resources.energy_balance(plant.reserve).unwrap_or(0.);
            let photo = (p.photosynthesis_per_tick * (0.3 + 0.7 * plant.growth as f64))
                .min((p.energy_capacity - current).max(0.));
            self.ledger
                .capture_sunlight(&mut self.resources, plant.reserve, photo)
                .map_err(|e| format!("photosynthesis: {e:?}"))?;
            self.diagnostics.plant_photosynthesis_e += photo;
            // Synthesis costs existing plant chemical energy, dissipated as heat.
            // No RNG or ecological changes when toxin.enabled=false.
            if self.config.toxin.enabled && plant.produces_toxin {
                let cost = self
                    .config
                    .toxin
                    .plant_energy_cost_per_tick
                    .min(self.resources.energy_balance(plant.reserve).unwrap_or(0.));
                if cost > 0. {
                    self.diagnostics.toxin_plant_energy_cost_total += cost;
                    self.ledger
                        .dissipate_heat(&mut self.resources, plant.reserve, cost)
                        .map_err(|e| format!("toxin synthesis: {e:?}"))?;
                }
            }
            let burn = p
                .basal_energy_per_tick
                .min(self.resources.energy_balance(plant.reserve).unwrap_or(0.));
            self.ledger
                .dissipate_heat(&mut self.resources, plant.reserve, burn)
                .map_err(|e| format!("metabolism: {e:?}"))?;
        }
        // 3. Soil uptake: per-cell requests are scaled to available nutrients, independent of plant ordering.
        let mut requests = vec![0.; self.plants.len()];
        let mut totals = vec![0.; self.land.cells.len()];
        for (i, plant) in self.plants.items.iter().enumerate() {
            let soil = self
                .resources
                .nutrient_balance(self.land.cells[plant.cell].soil)
                .unwrap_or(0.);
            let demand = p.nutrient_uptake_max * soil / (p.nutrient_half_saturation + soil);
            requests[i] = demand;
            totals[plant.cell] += demand;
        }
        for (i, plant) in self.plants.items.iter().enumerate() {
            let cell = &self.land.cells[plant.cell];
            let available = self.resources.nutrient_balance(cell.soil).unwrap_or(0.);
            let scale = if totals[plant.cell] > available {
                available / totals[plant.cell]
            } else {
                1.
            };
            let amount =
                (requests[i] * scale).min(self.resources.nutrient_balance(cell.soil).unwrap_or(0.));
            self.resources
                .transfer_nutrients(cell.soil, plant.reserve, amount)
                .map_err(|e| format!("uptake: {e:?}"))?;
            self.diagnostics.plant_uptake_n += amount;
        }
        // 4. Growth transfers nutrient matter from reserve to body and dissipates work energy.
        for plant in &mut self.plants.items {
            plant.age = plant.age.saturating_add(1);
            plant.seed_cooldown = plant.seed_cooldown.saturating_sub(1);
            let remaining = (1. - plant.growth).max(0.);
            let target_n =
                (p.adult_body_nutrients * 0.7 * p.growth_per_tick.min(remaining) as f64).max(0.);
            let stored_n = self.resources.nutrient_balance(plant.reserve).unwrap_or(0.);
            let stored_e = self.resources.energy_balance(plant.reserve).unwrap_or(0.);
            let grown = target_n
                .min(stored_n)
                .min(if p.growth_energy_per_nutrient > 0. {
                    stored_e / p.growth_energy_per_nutrient
                } else {
                    f64::MAX
                });
            if target_n > 1e-12 {
                if stored_n + 1e-10 < target_n {
                    self.diagnostics.plant_growth_nutrient_limited += 1;
                }
                if p.growth_energy_per_nutrient > 0.
                    && stored_e / p.growth_energy_per_nutrient + 1e-10 < target_n
                {
                    self.diagnostics.plant_growth_energy_limited += 1;
                }
                if grown <= 1e-12 {
                    self.diagnostics.plant_growth_zero += 1;
                }
            }
            if grown > 0. {
                self.resources
                    .transfer_nutrients(plant.reserve, plant.body, grown)
                    .map_err(|e| format!("growth: {e:?}"))?;
                self.ledger
                    .dissipate_heat(
                        &mut self.resources,
                        plant.reserve,
                        grown * p.growth_energy_per_nutrient,
                    )
                    .map_err(|e| format!("growth energy: {e:?}"))?;
                self.diagnostics.plant_growth_n += grown;
                plant.growth =
                    (plant.growth + (grown / (p.adult_body_nutrients * 0.7)) as f32).min(1.);
            }
        }
        // 5. Seed dispersal: asexual, immediate germination, all matter/energy from parent.
        let mut offspring = Vec::new();
        for i in 0..self.plants.len() {
            if self.plants.len()
                + self.animals.items.len()
                + self.carnivores.len()
                + offspring.len()
                >= self.config.world.max_living_entities as usize
            {
                self.diagnostics.seed_capacity_blocked += 1;
                break;
            }
            let parent = &self.plants.items[i];
            if parent.growth < 1. {
                self.diagnostics.seed_immature += 1;
                continue;
            }
            if parent.seed_cooldown > 0 {
                self.diagnostics.seed_cooldown_blocked += 1;
                continue;
            }
            self.diagnostics.seed_attempts += 1;
            if parent.produces_toxin {
                self.diagnostics.toxic_seed_attempts += 1;
            } else {
                self.diagnostics.nontoxic_seed_attempts += 1;
            }
            if self.rng.unit() > p.seed_probability {
                self.diagnostics.seed_probability_rejected += 1;
                continue;
            }
            let body_n = self.resources.nutrient_balance(parent.body).unwrap_or(0.);
            let reserve_n = self
                .resources
                .nutrient_balance(parent.reserve)
                .unwrap_or(0.);
            let energy = self.resources.energy_balance(parent.reserve).unwrap_or(0.);
            if body_n < p.adult_body_nutrients * 0.9
                || reserve_n < p.seed_body_nutrients
                || energy < p.seed_energy + p.seed_work_energy
            {
                self.diagnostics.seed_resource_blocked += 1;
                if parent.produces_toxin {
                    self.diagnostics.toxic_seed_resource_blocked += 1;
                } else {
                    self.diagnostics.nontoxic_seed_resource_blocked += 1;
                }
                continue;
            }
            let angle = self.rng.unit() * std::f32::consts::TAU;
            let radius = self.rng.unit().sqrt() * p.seed_dispersal_radius;
            let x = (parent.x + angle.cos() * radius).clamp(0., self.config.world.width);
            let y = (parent.y + angle.sin() * radius).clamp(0., self.config.world.height);
            offspring.push((i, x, y));
        }
        for (i, x, y) in offspring {
            let parent_reserve = self.plants.items[i].reserve;
            if self.plants.items[i].produces_toxin {
                self.diagnostics.toxic_seed_success += 1;
            } else {
                self.diagnostics.nontoxic_seed_success += 1;
            }
            let id = self.entities.spawn()?;
            let body = self.resources.open(0., 0.).map_err(|e| format!("{e:?}"))?;
            let reserve = self.resources.open(0., 0.).map_err(|e| format!("{e:?}"))?;
            self.resources
                .transfer_nutrients(parent_reserve, body, p.seed_body_nutrients)
                .map_err(|e| format!("seed N: {e:?}"))?;
            self.resources
                .transfer_energy(parent_reserve, reserve, p.seed_energy)
                .map_err(|e| format!("seed E: {e:?}"))?;
            self.ledger
                .dissipate_heat(&mut self.resources, parent_reserve, p.seed_work_energy)
                .map_err(|e| format!("seed work: {e:?}"))?;
            self.plants.items[i].seed_cooldown = p.seed_cooldown_ticks;
            let cell =
                self.land
                    .cell_index(x, y, self.config.world.width, self.config.world.height);
            let produces_toxin = crate::toxin::offspring_plant_toxic(
                self.config.seed,
                id.index as u64,
                self.plants.items[i].produces_toxin,
                self.config.toxin.plant_variation_enabled,
                self.config.toxin.plant_toxic_mutation_probability,
            );
            if produces_toxin {
                self.diagnostics.toxic_plant_births += 1;
            } else {
                self.diagnostics.nontoxic_plant_births += 1;
            }
            if produces_toxin != self.plants.items[i].produces_toxin {
                if produces_toxin {
                    self.diagnostics.toxic_plant_mutated_births += 1;
                } else {
                    self.diagnostics.nontoxic_plant_mutated_births += 1;
                }
            }
            let toxin_type = crate::toxin::offspring_plant_toxin_type(
                self.config.seed,
                id.index as u64,
                self.plants.items[i].toxin_type,
                self.config.toxin.plant_toxin_type_variation_enabled,
                self.config.toxin.plant_toxin_type_count,
                self.config.toxin.plant_toxin_type_mutation_probability,
            );
            if self.config.toxin.experimental_inherited_plant_colors {
                if let Some(trace) = &mut self.foraging_trace {
                    let parent = &self.plants.items[i];
                    trace.events.push(serde_json::json!({"plant_event":"birth","color":parent.color,
                        "toxic":produces_toxin,"type":toxin_type.0,"parent_toxic":parent.produces_toxin,
                        "parent_type":parent.toxin_type.0}));
                }
            }
            self.plants.items.push(Plant {
                id,
                x,
                y,
                cell,
                body,
                reserve,
                growth: 0.,
                age: 0,
                seed_cooldown: p.seed_cooldown_ticks,
                max_age: p.max_age_ticks,
                color: if self.config.toxin.experimental_inherited_plant_colors {
                    // Issue #70: optional color mutation for inherited experimental colors
                    // (stateless hash; probability or scale 0 keeps the parent color).
                    crate::plants::seed_color(
                        self.plants.items[i].color,
                        self.config.seed,
                        id,
                        p.color_mutation_probability,
                        p.color_mutation_scale,
                    )
                } else if p.color_inheritance_enabled {
                    crate::plants::seed_color(
                        self.plants.items[i].color,
                        self.config.seed,
                        id,
                        p.color_mutation_probability,
                        p.color_mutation_scale,
                    )
                } else {
                    crate::plants::visible_color(
                        self.config.toxin.experimental_plant_toxicity_colors,
                        produces_toxin,
                    )
                },
                produces_toxin,
                toxin_type,
            });
            super::species::plant_birth(
                &mut self.species,
                &self.plants.items,
                self.config.seed,
                self.tick,
                self.plants.items.len() - 1,
                Some(i),
            );
            self.births += 1;
            self.diagnostics.plant_births += 1;
        }
        // Phase 07.2.19: optional soil-funded spontaneous germination.
        // Use a separate deterministic hash, leaving the baseline RNG stream unchanged.
        if let Some(g) = self
            .spontaneous_germination
            .filter(|_| !self.config.toxin.experimental_inherited_plant_colors)
        {
            if self.tick.is_multiple_of(g.check_interval) {
                let needed = self.config.plants.initial_body_nutrients;
                let initial_energy = self.config.plants.initial_energy;
                let mut created = 0usize;
                for cell in 0..self.land.cells.len() {
                    if created >= g.max_births_per_check
                        || self.plants.len() + self.animals.items.len() + self.carnivores.len()
                            >= self.config.world.max_living_entities as usize
                    {
                        break;
                    }
                    let soil_account = self.land.cells[cell].soil;
                    let soil = self.resources.nutrient_balance(soil_account).unwrap_or(0.);
                    if soil < g.soil_threshold.max(needed) {
                        continue;
                    }
                    let mut h = self.config.seed
                        ^ self.tick.wrapping_mul(0x9e3779b97f4a7c15)
                        ^ (cell as u64).wrapping_mul(0xbf58476d1ce4e5b9);
                    h = (h ^ (h >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
                    h = (h ^ (h >> 27)).wrapping_mul(0x94d049bb133111eb);
                    h ^= h >> 31;
                    let chance = ((h >> 11) as f64) / (1u64 << 53) as f64;
                    if chance >= g.probability_per_cell {
                        continue;
                    }
                    let col = cell % self.land.columns as usize;
                    let row = cell / self.land.columns as usize;
                    let x = (col as f32 + 0.5) * self.config.world.width / self.land.columns as f32;
                    let y = (row as f32 + 0.5) * self.config.world.height / self.land.rows as f32;
                    let id = self.entities.spawn()?;
                    let body = self
                        .resources
                        .open(0., 0.)
                        .map_err(|e| format!("germination body: {e:?}"))?;
                    let reserve = self
                        .resources
                        .open(0., 0.)
                        .map_err(|e| format!("germination reserve: {e:?}"))?;
                    self.resources
                        .transfer_nutrients(soil_account, body, needed)
                        .map_err(|e| format!("germination nutrient: {e:?}"))?;
                    self.ledger
                        .capture_sunlight(&mut self.resources, reserve, initial_energy)
                        .map_err(|e| format!("germination sunlight: {e:?}"))?;
                    let produces_toxin = crate::toxin::founder_plant_toxic(
                        self.config.seed,
                        id.index as u64,
                        self.config.toxin.plant_variation_enabled,
                        self.config.toxin.initial_toxic_plant_fraction,
                    );
                    self.plants.items.push(Plant {
                        id,
                        x,
                        y,
                        cell,
                        body,
                        reserve,
                        growth: 0.,
                        age: 0,
                        seed_cooldown: p.seed_cooldown_ticks,
                        max_age: p.max_age_ticks,
                        color: crate::plants::visible_color(
                            self.config.toxin.experimental_plant_toxicity_colors,
                            produces_toxin,
                        ),
                        produces_toxin,
                        toxin_type: crate::toxin::founder_plant_toxin_type_biased(
                            self.config.seed,
                            id.index as u64,
                            self.config.toxin.plant_toxin_type_variation_enabled,
                            self.config.toxin.plant_toxin_type_count,
                            self.config.toxin.initial_type0_fraction,
                        ),
                    });
                    super::species::plant_birth(
                        &mut self.species,
                        &self.plants.items,
                        self.config.seed,
                        self.tick,
                        self.plants.items.len() - 1,
                        None,
                    );
                    if self.plants.items.last().unwrap().produces_toxin {
                        self.diagnostics.toxic_plant_births += 1;
                    } else {
                        self.diagnostics.nontoxic_plant_births += 1;
                    }
                    self.births += 1;
                    self.diagnostics.plant_births += 1;
                    self.spontaneous_births += 1;
                    created += 1;
                }
            }
        }
        // 6. Death: transfer all remaining matter to local detritus and energy to heat.
        let mut i = 0;
        while i < self.plants.len() {
            if self.plants.items[i].age < self.plants.items[i].max_age {
                i += 1;
                continue;
            }
            let plant = self.plants.items.swap_remove(i);
            if self.config.toxin.experimental_inherited_plant_colors {
                if let Some(trace) = &mut self.foraging_trace {
                    trace.events.push(serde_json::json!({"plant_event":"age","color":plant.color,"toxic":plant.produces_toxin}));
                }
            }
            let detritus = self.land.cells[plant.cell].detritus;
            for account in [plant.body, plant.reserve] {
                let n = self.resources.nutrient_balance(account).unwrap_or(0.);
                let e = self.resources.energy_balance(account).unwrap_or(0.);
                self.resources
                    .transfer_nutrients(account, detritus, n)
                    .map_err(|e| format!("death N: {e:?}"))?;
                self.ledger
                    .dissipate_heat(&mut self.resources, account, e)
                    .map_err(|e| format!("death E: {e:?}"))?;
            }
            self.entities.despawn(plant.id);
            if plant.produces_toxin {
                self.diagnostics.toxic_plant_age_deaths += 1;
            } else {
                self.diagnostics.nontoxic_plant_age_deaths += 1;
            }
            self.deaths += 1;
            self.diagnostics.plant_age_deaths += 1;
        }
        // Phase 04: herbivore movement, feeding, starvation and death.
        self.update_herbivores()?;
        if self.config.animal_toxin.enabled {
            self.produce_herbivore_toxins()?;
        }
        if self.config.carnivores.enabled {
            self.update_carnivores()?;
        }
        self.species_census_if_due();
        self.mimicry_census_if_due();
        self.baseline.check(&self.resources,&self.ledger,1e-3,1e-9).map_err(|e|format!(
            "conservation at tick {}: {e:?}; energy_current={:.12}, energy_initial={:.12}, sunlight_in={:.12}, heat_out={:.12}, energy_tolerance={:.12}",
            self.tick, self.resources.total_energy(), self.baseline.energy,
            self.ledger.sunlight_total(), self.ledger.heat_total(),
            1e-3_f64.max(1e-9 * (self.baseline.energy.abs() + self.ledger.sunlight_total().abs() + self.ledger.heat_total().abs()))
        ))?;
        Ok(())
    }
}
