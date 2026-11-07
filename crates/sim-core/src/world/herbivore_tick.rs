use super::{
    behavior::{Action, Perception, Policy},
    World,
};
use crate::animals::{Herbivore, HerbivoreGenes};
impl World {
    pub(super) fn update_herbivores(&mut self) -> Result<(), String> {
        let h = &self.config.herbivores;
        self.plant_grid.rebuild(&self.plants);
        let plants_protected = h.plant_protection_enabled
            && self.plants.len() <= h.plant_protection_threshold as usize;
        // Global conservation safeguard: prohibit feeding at or below the configured living-plant threshold.
        // The threshold is evaluated once per tick; no N or E is created or destroyed.
        // One deterministic search and movement step per animal. The grid prevents O(animals * plants) scans.
        for i in 0..self.animals.items.len() {
            let trace_window = self
                .behavior_trace_window
                .is_some_and(|(start, end)| (start..=end).contains(&self.tick));
            let rng_before = trace_window.then(|| format!("{:?}", self.rng));
            let exploration_before = (
                self.animals.items[i].exploration_heading,
                self.animals.items[i].exploration_ticks,
            );

            let (x, y, vision, speed, range, reserve, hp_max) = {
                let a = &self.animals.items[i];
                (
                    a.x,
                    a.y,
                    a.genes.vision,
                    a.genes.speed,
                    a.genes.feeding_range,
                    a.reserve,
                    a.genes.adult_max_hp,
                )
            };
            // Digest existing stomach nutrients before feeding. This is an N transfer, not creation.
            if h.satiety_enabled {
                let stomach = self.animals.items[i].stomach;
                let n = self.resources.nutrient_balance(stomach).unwrap_or(0.);
                let digest = n.min(h.digestion_n_per_tick);
                if digest > 0. {
                    self.resources
                        .transfer_nutrients(stomach, reserve, digest)
                        .map_err(|e| format!("digestion N: {e:?}"))?;
                    self.diagnostics.herbivore_digested_n += digest;
                }
                let fraction = (n - digest) / h.stomach_capacity_n;
                if self.animals.items[i].feeding_active && fraction >= h.feeding_stop_fraction {
                    self.animals.items[i].feeding_active = false;
                } else if !self.animals.items[i].feeding_active
                    && fraction <= h.feeding_resume_fraction
                {
                    self.animals.items[i].feeding_active = true;
                }
            }
            // Phase 25: conservation-preserving repair of structural body N from digested reserve N.
            // No nutrients are created: reserve -> body, capped at the original body-N target.
            if h.body_n_repair_per_tick > 0. {
                let body = self.animals.items[i].body;
                let current_body = self.resources.nutrient_balance(body).unwrap_or(0.);
                let available = self.resources.nutrient_balance(reserve).unwrap_or(0.);
                let cap = h.initial_body_nutrients.max(h.offspring_body_nutrients);
                let repair = (cap - current_body)
                    .max(0.)
                    .min(h.body_n_repair_per_tick)
                    .min(available);
                if repair > 0. {
                    self.resources
                        .transfer_nutrients(reserve, body, repair)
                        .map_err(|e| format!("body N repair: {e:?}"))?;
                    self.diagnostics.phase25_body_n_repaired += repair;
                    self.diagnostics.phase25_body_n_repair_ticks += 1;
                }
            }
            // A consecutive well-fed period is required for reproduction when enabled.
            // This is behavioral state, not a new N/E account.
            let fraction = (self
                .resources
                .nutrient_balance(self.animals.items[i].stomach)
                .unwrap_or(0.)
                / h.stomach_capacity_n)
                .clamp(0., 1.);
            let a = &mut self.animals.items[i];
            let prior_streak = a.well_fed_ticks;
            let satiety_qualified =
                h.satiety_enabled && fraction >= h.reproduction_satiety_threshold;
            if satiety_qualified {
                a.well_fed_ticks = a.well_fed_ticks.saturating_add(1);
            } else {
                a.well_fed_ticks = 0;
            }
            self.diagnostics.phase26_stomach_above_threshold_samples += satiety_qualified as u64;
            self.diagnostics.phase26_stomach_below_threshold_samples += (!satiety_qualified) as u64;
            self.diagnostics.phase26_feeding_active_samples += a.feeding_active as u64;
            self.diagnostics.phase26_feeding_paused_samples += (!a.feeding_active) as u64;
            if prior_streak > 0 && !satiety_qualified {
                self.diagnostics.phase26_satiety_streak_resets += 1;
            }
            if a.well_fed_ticks == h.reproduction_well_fed_ticks {
                self.diagnostics.phase26_satiety_streak_completions += 1;
            }
            // Phase 21: per-animal-tick reserve and stomach snapshots, before metabolism.
            let reserve_e_before = self.resources.energy_balance(reserve).unwrap_or(0.);
            let reserve_n_before = self.resources.nutrient_balance(reserve).unwrap_or(0.);
            let stomach_n_before = self
                .resources
                .nutrient_balance(self.animals.items[i].stomach)
                .unwrap_or(0.);
            self.diagnostics.phase21_live_samples += 1;
            self.diagnostics.phase21_live_reserve_e_total += reserve_e_before;
            self.diagnostics.phase21_live_reserve_n_total += reserve_n_before;
            self.diagnostics.phase21_live_stomach_n_total += stomach_n_before;
            if reserve_e_before <= 1e-9 {
                self.diagnostics.phase21_live_zero_e_samples += 1;
            }
            if reserve_e_before
                >= hp_max as f64 * h.max_energy_per_hp * h.reproduction_min_energy_fraction
            {
                self.diagnostics.phase21_live_energy_gate_ready_samples += 1;
            }
            let basal = h.basal_energy_per_tick * (hp_max as f64 / 60.).powf(0.75);
            let current = self.resources.energy_balance(reserve).unwrap_or(0.);
            self.ledger
                .dissipate_heat(&mut self.resources, reserve, basal.min(current))
                .map_err(|e| format!("herbivore metabolism: {e:?}"))?;
            self.diagnostics.herbivore_basal_e += basal.min(current);
            self.diagnostics.phase21_basal_e_requested += basal;
            self.diagnostics.phase21_basal_e_unpaid += (basal - current).max(0.);
            // Vision is strictly local: the grid only returns plants inside the gene-defined radius.
            let may_feed =
                (!h.satiety_enabled || self.animals.items[i].feeding_active) && !plants_protected;
            if plants_protected {
                self.diagnostics.herbivore_plant_protection_blocked_ticks += 1;
            }
            if h.satiety_enabled && !may_feed {
                self.diagnostics.herbivore_satiety_blocked_ticks += 1;
            }
            let target = if may_feed {
                self.plant_grid.nearest(&self.plants, x, y, vision)
            } else {
                None
            };
            if target.is_some() {
                self.diagnostics.herbivore_target_ticks += 1;
            } else {
                self.diagnostics.herbivore_search_without_target += 1;
                if may_feed {
                    self.diagnostics.phase20_feeding_no_target_ticks += 1;
                }
            }
            if target.is_some() && self.animals.items[i].target.is_none() {
                self.diagnostics.herbivore_target_acquisitions += 1;
            }
            // Phase 32: nearest predator in detection radius triggers flight before plant pursuit.
            // No free movement: the existing energy-limited movement step pays for fleeing.
            let threat = if h.predator_evasion_enabled {
                self.carnivores
                    .iter()
                    .filter_map(|p| {
                        let dx = x - p.x;
                        let dy = y - p.y;
                        let d2 = dx * dx + dy * dy;
                        if d2 < h.predator_detection_radius * h.predator_detection_radius {
                            Some((d2, dx, dy))
                        } else {
                            None
                        }
                    })
                    .min_by(|a, b| a.0.total_cmp(&b.0))
            } else {
                None
            };
            let action = Policy {
                fear: (self.animals.items[i].genes.fear + self.animals.items[i].learned_fear)
                    .clamp(0., 1.),
                aggression: self.animals.items[i].genes.aggression,
                detection_radius: h.predator_detection_radius,
                flee_threshold: 0.,
            }
            .choose(Perception {
                threat_distance: threat.map(|(d2, _, _)| d2.sqrt()),
                food_visible: target.is_some(),
                satiated: !may_feed,
                energy_fraction: (reserve_e_before / 60.).clamp(0., 1.) as f32,
            });
            if action == Action::Flee && target.is_some() {
                self.carnivore_debug.herbivore_flee_with_plant_visible += 1;
            }
            if let Some((d2, _, _)) = threat {
                if d2.sqrt() > h.predator_detection_radius * 0.85 {
                    self.carnivore_debug
                        .herbivore_threat_near_detection_boundary += 1;
                }
            }
            self.animals.items[i].defense_active =
                action == Action::StandGround && threat.is_some();
            if threat.is_some() {
                self.animals.items[i].learned_fear =
                    (self.animals.items[i].learned_fear + 0.0001).min(0.15);
            } else {
                self.animals.items[i].learned_fear =
                    (self.animals.items[i].learned_fear - 0.00002).max(-0.10);
            }
            let (tx, ty, plant_id) = if action == Action::Flee {
                let (d2, dx, dy) = threat.unwrap();
                self.carnivore_debug.herbivore_predator_detected_ticks += 1;
                let norm = d2.sqrt().max(0.001);
                let flee_speed = speed * h.predator_flee_speed_multiplier;
                (x + dx / norm * flee_speed, y + dy / norm * flee_speed, None)
            } else if action == Action::StandGround {
                (x, y, None)
            } else if let Some(pi) = target {
                let p = &self.plants.items[pi];
                (p.x, p.y, Some(p.id))
            } else {
                // Persist a direction for 30..100 ticks instead of drawing a new random
                // direction every tick. This produces long-range correlated exploration.
                let a = &mut self.animals.items[i];
                if a.exploration_ticks == 0 {
                    a.exploration_heading = self.rng.unit() * std::f32::consts::TAU;
                    a.exploration_ticks = 30 + (self.rng.next_u64() % 71) as u32;
                }
                a.exploration_ticks -= 1;
                // Reflect from world edges, without getting stuck against a wall.
                let mut vx = a.exploration_heading.cos();
                let mut vy = a.exploration_heading.sin();
                if (x <= 0. && vx < 0.) || (x >= self.config.world.width && vx > 0.) {
                    vx = -vx;
                }
                if (y <= 0. && vy < 0.) || (y >= self.config.world.height && vy > 0.) {
                    vy = -vy;
                }
                a.exploration_heading = vy.atan2(vx);
                (x + vx * speed, y + vy * speed, None)
            };
            let dx = tx - x;
            let dy = ty - y;
            let dist = (dx * dx + dy * dy).sqrt();
            let energy = self.resources.energy_balance(reserve).unwrap_or(0.).max(0.);
            let movement_cost = h.movement_energy_per_unit * (hp_max as f64 / 60.).powf(0.75);
            let desired_distance = dist.min(if action == Action::Flee {
                speed * h.predator_flee_speed_multiplier
            } else {
                speed
            });
            self.diagnostics.phase21_movement_e_requested +=
                desired_distance as f64 * movement_cost;
            // Compute the actual in-world destination before charging movement energy.
            // Use f64 for cost arithmetic and never debit more than the account balance.
            let affordable = if movement_cost > 0.0 {
                (energy / movement_cost).min(desired_distance as f64) as f32
            } else {
                desired_distance
            };
            let step = affordable.min(desired_distance);
            self.diagnostics.phase21_movement_e_unpaid +=
                (desired_distance - step) as f64 * movement_cost;
            if step > 0. && dist > 0. {
                let nx = (x + dx / dist * step).clamp(0., self.config.world.width);
                let ny = (y + dy / dist * step).clamp(0., self.config.world.height);
                let traveled = (((nx - x) as f64).powi(2) + ((ny - y) as f64).powi(2)).sqrt();
                let charge = (traveled * movement_cost).min(energy);
                if charge > 0. {
                    self.ledger.dissipate_heat(&mut self.resources,reserve,charge).map_err(|e|
      format!("movement tick={} animal={:?} reserve={:?} balance={:.17} charge={:.17} cost_per_unit={:.17} distance={:.9}: {e:?}",
       self.tick,self.animals.items[i].id,reserve,energy,charge,movement_cost,traveled)
     )?;
                }
                self.diagnostics.herbivore_movement_e += charge;
                if action == Action::Flee {
                    self.carnivore_debug.herbivore_flee_distance += traveled;
                }
                if target.is_some() {
                    self.diagnostics.herbivore_target_distance += traveled;
                } else {
                    self.diagnostics.herbivore_exploration_steps += 1;
                    self.diagnostics.herbivore_exploration_distance += traveled;
                }
                let a = &mut self.animals.items[i];
                a.x = nx;
                a.y = ny;
            }
            if trace_window
                && (self.focal_trace_id == Some(self.animals.items[i].id)
                    || self.behavior_trace_subject == Some(self.animals.items[i].id))
            {
                self.focal_trace.push(serde_json::json!({
                    "phase": "behavior", "tick": self.tick,
                    "id":[self.animals.items[i].id.index,self.animals.items[i].id.generation],
                    "position_before": [x,y], "position_after": [self.animals.items[i].x,self.animals.items[i].y],
                    "action": format!("{action:?}"), "may_feed": may_feed,
                    "target": target.map(|j| {
                        let p = &self.plants.items[j];
                        serde_json::json!({"id":[p.id.index,p.id.generation],"x":p.x,"y":p.y,
                            "body_n":self.resources.nutrient_balance(p.body).unwrap_or(0.)})
                    }),
                    "threat": threat, "predators": self.carnivores.iter().map(|p|
                        serde_json::json!({"id":[p.id.index,p.id.generation],"x":p.x,"y":p.y})).collect::<Vec<_>>(),
                    "learned_fear": self.animals.items[i].learned_fear,
                    "energy_before": reserve_e_before, "movement_energy": energy,
                    "desired_distance": desired_distance, "actual_step": step,
                    "exploration_before": exploration_before,
                    "exploration_after": [self.animals.items[i].exploration_heading,
                        self.animals.items[i].exploration_ticks as f32],
                    "rng_before": rng_before, "rng_after": format!("{:?}",self.rng),
                }));
            }
            self.animals.items[i].target = plant_id;
            self.animals.items[i].age = self.animals.items[i].age.saturating_add(1);
            self.animals.items[i].reproduction_cooldown = self.animals.items[i]
                .reproduction_cooldown
                .saturating_sub(1);
            let (ax, ay) = (self.animals.items[i].x, self.animals.items[i].y);
            if let Some(pi) = target {
                let (px, py) = (self.plants.items[pi].x, self.plants.items[pi].y);
                let d2 = (ax - px).powi(2) + (ay - py).powi(2);
                if d2 > range * range && may_feed {
                    self.diagnostics.phase20_feeding_target_out_of_range_ticks += 1;
                }
                if d2 <= range * range {
                    let pbody = self.plants.items[pi].body;
                    let pres = self.plants.items[pi].reserve;
                    let ares = self.animals.items[i].reserve;
                    let cell = self.plants.items[pi].cell;
                    let detritus = self.land.cells[cell].detritus;
                    let available = self.resources.nutrient_balance(pbody).unwrap_or(0.);
                    let stomach = self.animals.items[i].stomach;
                    let free_space = if h.satiety_enabled {
                        (h.stomach_capacity_n
                            - self.resources.nutrient_balance(stomach).unwrap_or(0.))
                        .max(0.)
                    } else {
                        f64::INFINITY
                    };
                    let eaten = available.min(h.feeding_nutrients_per_tick).min(
                        if h.satiety_enabled && h.feeding_nutrient_absorption > 0. {
                            free_space / h.feeding_nutrient_absorption
                        } else {
                            f64::INFINITY
                        },
                    );
                    // Phase 07.1: chemical exposure is proportional to actual consumed plant matter.
                    // The toxin is a dimensionless concentration, not a newly created energy or nutrient account.
                    if self.config.toxin.enabled
                        && self.plants.items[pi].produces_toxin
                        && eaten > 0.
                    {
                        let dose = crate::toxin::exposure_dose(eaten, &self.config.toxin);
                        if self.config.toxin.herbivore_multi_toxin_enabled {
                            let toxin_type = self.plants.items[pi].toxin_type.0 as usize;
                            if toxin_type == 0 {
                                self.animals.items[i].toxin_load += dose;
                            } else {
                                self.animals.items[i].typed_load[toxin_type] += dose;
                            }
                        } else {
                            self.animals.items[i].toxin_load += dose;
                        }
                        self.animals.items[i].lifetime_toxin_dose += dose;
                        let exposure_type = if self.config.toxin.herbivore_multi_toxin_enabled {
                            self.plants.items[pi].toxin_type.0 as usize
                        } else {
                            0
                        };
                        self.animals.items[i].lifetime_typed_dose[exposure_type] += dose;
                        self.diagnostics.toxin_exposure_events += 1;
                        self.diagnostics.toxin_exposure_total += dose;
                    }
                    if eaten > 0. {
                        self.diagnostics.herbivore_feeding_events += 1;
                        self.diagnostics.plant_consumed_n += eaten;
                        self.diagnostics.phase20_feeding_in_range_success_ticks += 1;
                    } else {
                        self.diagnostics.phase20_feeding_in_range_zero_n_ticks += 1;
                    }
                    let absorbed = eaten * h.feeding_nutrient_absorption;
                    self.diagnostics.herbivore_absorbed_n += absorbed;
                    let waste = eaten - absorbed;
                    if absorbed > 0. {
                        self.resources
                            .transfer_nutrients(
                                pbody,
                                if h.satiety_enabled { stomach } else { ares },
                                absorbed,
                            )
                            .map_err(|e| format!("feeding nutrients: {e:?}"))?;
                    }
                    if waste > 0. {
                        self.resources
                            .transfer_nutrients(pbody, detritus, waste)
                            .map_err(|e| format!("feeding waste: {e:?}"))?;
                    }
                    let capacity = (hp_max as f64 * h.max_energy_per_hp
                        - self.resources.energy_balance(ares).unwrap_or(0.))
                    .max(0.);
                    let plant_e = self.resources.energy_balance(pres).unwrap_or(0.);
                    // Transfer chemical energy, do not manufacture it from consumed nutrients.
                    let desired = eaten * 2.;
                    let available_e = plant_e.min(desired);
                    let gained = (available_e * h.feeding_energy_gain_fraction).min(capacity);
                    self.diagnostics.phase21_feeding_plant_energy_available += available_e;
                    self.diagnostics.phase21_feeding_energy_capacity_blocked +=
                        (available_e * h.feeding_energy_gain_fraction - capacity).max(0.);
                    self.diagnostics.herbivore_feeding_e += gained;
                    if gained > 0. {
                        self.resources
                            .transfer_energy(pres, ares, gained)
                            .map_err(|e| format!("feeding energy: {e:?}"))?;
                    }
                    let dissipated = available_e - gained;
                    if dissipated > 0. {
                        self.ledger
                            .dissipate_heat(&mut self.resources, pres, dissipated)
                            .map_err(|e| format!("digestion heat: {e:?}"))?;
                    }
                }
            }
            // Clearance and physiological damage; HP is a health state, not a resource balance.
            if self.config.toxin.enabled {
                let t = &self.config.toxin;
                let animal = &mut self.animals.items[i];
                let result = crate::toxin::advance_burden(
                    animal.toxin_load,
                    animal.genes.toxin_resistance,
                    t,
                );
                animal.toxin_load = result.remaining_load;
                if t.herbivore_multi_toxin_enabled {
                    for toxin_type in 1..t.plant_toxin_type_count as usize {
                        let load = animal.typed_load[toxin_type];
                        let resistance = animal.genes.typed_resistance[toxin_type];
                        let step = crate::toxin::advance_burden(load, resistance, t);
                        animal.typed_load[toxin_type] = step.remaining_load;
                        self.diagnostics.toxin_clearance_total += step.cleared;
                        self.diagnostics.resistance_prevented_damage_total += step.prevented_damage;
                        animal.lifetime_prevented_damage += step.prevented_damage;
                        animal.lifetime_toxin_damage += step.damage as f64;
                        if step.damage > 0. {
                            self.diagnostics.toxin_damage_ticks += 1;
                            self.diagnostics.toxin_hp_damage_total += step.damage as f64;
                        }
                        animal.hp -= step.damage;
                    }
                }
                self.diagnostics.toxin_clearance_total += result.cleared;
                self.diagnostics.resistance_prevented_damage_total += result.prevented_damage;
                animal.lifetime_prevented_damage += result.prevented_damage;
                animal.lifetime_toxin_damage += result.damage as f64;
                if result.damage > 0. {
                    self.diagnostics.toxin_damage_ticks += 1;
                    self.diagnostics.toxin_hp_damage_total += result.damage as f64;
                }
                animal.hp -= result.damage;
            }
            if self.config.toxin.resistance_enabled {
                let resistance = self.animals.items[i].genes.toxin_resistance as f64;
                let mut cost = crate::toxin::resistance_upkeep(
                    resistance,
                    self.resources.energy_balance(reserve).unwrap_or(0.),
                    &self.config.toxin,
                );
                if self.config.toxin.herbivore_multi_toxin_enabled {
                    let extra = self.animals.items[i].genes.typed_resistance
                        [1..self.config.toxin.plant_toxin_type_count as usize]
                        .iter()
                        .map(|&r| r as f64)
                        .sum::<f64>();
                    cost = ((cost
                        + self.config.toxin.resistance_energy_cost_per_tick
                            * extra
                            * self
                                .config
                                .toxin
                                .additional_type_resistance_upkeep_multiplier)
                        * self.config.toxin.multi_type_resistance_upkeep_multiplier)
                        .min(self.resources.energy_balance(reserve).unwrap_or(0.));
                }
                if cost > 0. {
                    self.ledger
                        .dissipate_heat(&mut self.resources, reserve, cost)
                        .map_err(|e| format!("resistance upkeep: {e:?}"))?;
                    self.diagnostics.resistance_energy_cost_total += cost;
                    self.animals.items[i].lifetime_resistance_cost += cost;
                }
            }
            let energy = self.resources.energy_balance(reserve).unwrap_or(0.);
            if energy <= 1e-9 {
                self.diagnostics.herbivore_zero_energy_ticks += 1;
                self.diagnostics.phase21_starvation_damage_ticks += 1;
                self.diagnostics.phase21_starvation_hp_damage +=
                    (hp_max * h.starvation_hp_fraction) as f64;
                self.animals.items[i].hp -= hp_max * h.starvation_hp_fraction;
            }
            if let Some(before) = rng_before {
                let after = format!("{:?}", self.rng);
                if before != after {
                    let a = &self.animals.items[i];
                    self.focal_trace.push(serde_json::json!({
                        "phase":"animal_rng_advance","tick":self.tick,"id":[a.id.index,a.id.generation],
                        "rng_before":before,"rng_after":after,
                        "exploration_before":exploration_before,
                        "exploration_after":[a.exploration_heading,a.exploration_ticks as f32],
                        "target":a.target.map(|id|[id.index,id.generation])
                    }));
                }
            }
        }
        if self
            .behavior_trace_window
            .is_some_and(|(start, end)| (start..=end).contains(&self.tick))
        {
            self.focal_trace.push(serde_json::json!({
                "phase": "rng_post_herbivores", "tick": self.tick, "rng": format!("{:?}",self.rng)
            }));
        }
        if let Some(id) = self.focal_trace_id {
            self.record_focal_state(id, "post_physiology");
        }
        // Remove fully consumed plants only after all feeding requests; return remaining matter and energy.
        let mut j = 0;
        while j < self.plants.items.len() {
            let body = self.plants.items[j].body;
            if self.resources.nutrient_balance(body).unwrap_or(0.) > 0.001 {
                j += 1;
                continue;
            }
            let p = self.plants.items.swap_remove(j);
            let detritus = self.land.cells[p.cell].detritus;
            for a in [p.body, p.reserve] {
                let n = self.resources.nutrient_balance(a).unwrap_or(0.);
                let e = self.resources.energy_balance(a).unwrap_or(0.);
                if n > 0. {
                    self.resources
                        .transfer_nutrients(a, detritus, n)
                        .map_err(|e| format!("plant eaten N: {e:?}"))?;
                }
                if e > 0. {
                    self.ledger
                        .dissipate_heat(&mut self.resources, a, e)
                        .map_err(|e| format!("plant eaten E: {e:?}"))?;
                }
            }
            if p.produces_toxin {
                self.diagnostics.toxic_plant_consumed_deaths += 1;
            } else {
                self.diagnostics.nontoxic_plant_consumed_deaths += 1;
            }
            self.entities.despawn(p.id);
            self.deaths += 1;
            self.diagnostics.plant_consumed_deaths += 1;
        }
        // Plant removals above invalidate grid indices (swap_remove). Rebuild before death diagnostics.
        self.plant_grid.rebuild(&self.plants);
        let mut i = 0;
        while i < self.animals.items.len() {
            if self.animals.items[i].hp > 0.
                && self.animals.items[i].age < self.animals.items[i].max_age
            {
                i += 1;
                continue;
            }
            let a = self.animals.items.swap_remove(i);
            if self.herbivore_terminal_logging {
                self.herbivore_terminal_events
                    .push(super::HerbivoreTerminalEvent::capture(self.tick, &a));
            }
            if self.config.toxin.enabled && a.toxin_load > 0. {
                self.diagnostics.toxin_deaths_with_burden += 1;
                if a.hp <= 0. {
                    self.diagnostics.toxin_deaths_with_burden_and_hp_zero += 1;
                }
            }
            self.diagnostics.phase20_herbivore_death_samples += 1;
            let death_e = self.resources.energy_balance(a.reserve).unwrap_or(0.);
            self.diagnostics.phase21_death_reserve_e_total += death_e;
            self.diagnostics.phase21_death_stomach_n_total +=
                self.resources.nutrient_balance(a.stomach).unwrap_or(0.);
            if death_e <= 1e-9 {
                self.diagnostics.phase21_death_zero_e_count += 1;
            } else {
                self.diagnostics.phase21_death_positive_e_count += 1;
            }
            self.diagnostics.phase20_herbivore_death_reserve_e_total +=
                self.resources.energy_balance(a.reserve).unwrap_or(0.);
            self.diagnostics.phase20_herbivore_death_stomach_n_total +=
                self.resources.nutrient_balance(a.stomach).unwrap_or(0.);
            self.diagnostics.phase20_herbivore_death_nearby_plants_total +=
                self.plant_grid
                    .count_within(&self.plants, a.x, a.y, a.genes.vision) as u64;
            if a.hp <= 0. {
                self.diagnostics.herbivore_hp_deaths += 1;
                if self.resources.energy_balance(a.reserve).unwrap_or(0.) <= 1e-9 {
                    self.diagnostics.phase19_hp_zero_energy_deaths += 1;
                } else {
                    self.diagnostics.phase19_hp_positive_energy_deaths += 1;
                }
            } else {
                self.diagnostics.herbivore_age_deaths += 1;
            }
            let cell =
                self.land
                    .cell_index(a.x, a.y, self.config.world.width, self.config.world.height);
            let detritus = self.land.cells[cell].detritus;
            for account in [a.body, a.reserve, a.stomach] {
                let n = self.resources.nutrient_balance(account).unwrap_or(0.);
                let e = self.resources.energy_balance(account).unwrap_or(0.);
                if n > 0. {
                    self.resources
                        .transfer_nutrients(account, detritus, n)
                        .map_err(|e| format!("herbivore death N: {e:?}"))?;
                }
                if e > 0. {
                    self.ledger
                        .dissipate_heat(&mut self.resources, account, e)
                        .map_err(|e| format!("herbivore death E: {e:?}"))?;
                }
            }
            self.entities.despawn(a.id);
            self.deaths += 1;
        }
        self.reproduce_herbivores()?;
        Ok(())
    }

    fn reproduce_herbivores(&mut self) -> Result<(), String> {
        let h = self.config.herbivores.clone();
        if !h.reproduction_enabled {
            return Ok(());
        }
        self.plant_grid.rebuild(&self.plants);
        // Global conservation safeguard: prohibit feeding at or below the configured living-plant threshold.
        // The threshold is evaluated once per tick; no N or E is created or destroyed.
        let parents = self.animals.items.len();
        for animal in &mut self.animals.items {
            if animal.age >= h.reproduction_min_age {
                animal.reproduction_age_reached = true;
            }
        }
        for i in 0..parents {
            if self.plants.items.len() + self.animals.items.len()
                >= self.config.world.max_living_entities as usize
            {
                self.diagnostics.herbivore_reproduction_capacity_blocked += 1;
                break;
            }
            let a = &self.animals.items[i];
            let e = self.resources.energy_balance(a.reserve).unwrap_or(0.);
            let emax = a.genes.adult_max_hp as f64 * h.max_energy_per_hp;
            // Phase 24: non-invasive, independent reproduction-gate observations.
            // Each gate is counted independently; cumulative counts can be differenced by time window.
            let body_n_now = self.resources.nutrient_balance(a.body).unwrap_or(0.);
            let reserve_n_now = self.resources.nutrient_balance(a.reserve).unwrap_or(0.);
            let stomach_n_now = self.resources.nutrient_balance(a.stomach).unwrap_or(0.);
            let age_ok = a.age >= h.reproduction_min_age;
            let cooldown_ok = a.reproduction_cooldown == 0;
            let hp_ok = a.hp >= a.genes.adult_max_hp * h.reproduction_min_hp_fraction;
            let energy_ok = e >= emax * h.reproduction_min_energy_fraction;
            let satiety_ok = !h.satiety_reproduction_enabled
                || (h.satiety_enabled && a.well_fed_ticks >= h.reproduction_well_fed_ticks);
            let body_n_ok = body_n_now >= h.offspring_body_nutrients;
            let reserve_n_ok = reserve_n_now >= h.offspring_reserve_nutrients;
            let transfer_e_ok = e >= h.offspring_energy + h.reproduction_work_energy;
            if self.focal_trace_id == Some(a.id) {
                self.focal_trace.push(serde_json::json!({
                    "phase": "reproduction_gates", "tick": self.tick,
                    "hp": a.hp, "hp_threshold": a.genes.adult_max_hp * h.reproduction_min_hp_fraction,
                    "reserve_e": e, "energy_threshold": emax * h.reproduction_min_energy_fraction,
                    "body_n": body_n_now, "reserve_n": reserve_n_now, "stomach_n": stomach_n_now,
                    "age_ok": age_ok, "cooldown_ok": cooldown_ok, "hp_ok": hp_ok,
                    "energy_ok": energy_ok, "satiety_ok": satiety_ok, "body_n_ok": body_n_ok,
                    "reserve_n_ok": reserve_n_ok, "transfer_e_ok": transfer_e_ok,
                    "births": a.successful_births, "well_fed_ticks": a.well_fed_ticks,
                }));
            }
            self.diagnostics.phase25_body_n_total += body_n_now;
            self.diagnostics.phase25_body_n_samples += 1;
            self.diagnostics.phase25_well_fed_ticks_total += a.well_fed_ticks as u64;
            self.diagnostics.phase25_well_fed_samples += 1;
            if !body_n_ok {
                self.diagnostics.phase25_body_n_deficit_samples += 1;
                if reserve_n_now >= h.offspring_body_nutrients {
                    self.diagnostics
                        .phase25_reserve_n_sufficient_when_body_n_low += 1;
                }
            }
            self.diagnostics.phase24_repro_samples += 1;
            self.diagnostics.phase24_repro_age_ready += age_ok as u64;
            self.diagnostics.phase24_repro_cooldown_ready += cooldown_ok as u64;
            self.diagnostics.phase24_repro_hp_ready += hp_ok as u64;
            self.diagnostics.phase24_repro_energy_ready += energy_ok as u64;
            self.diagnostics.phase24_repro_satiety_ready += satiety_ok as u64;
            self.diagnostics.phase24_repro_body_n_ready += body_n_ok as u64;
            self.diagnostics.phase24_repro_reserve_n_ready += reserve_n_ok as u64;
            self.diagnostics.phase24_repro_transfer_e_ready += transfer_e_ok as u64;
            self.diagnostics.phase24_repro_ready_all += (age_ok
                && cooldown_ok
                && hp_ok
                && energy_ok
                && satiety_ok
                && body_n_ok
                && reserve_n_ok
                && transfer_e_ok) as u64;
            self.diagnostics.phase24_repro_reserve_e_total += e;
            self.diagnostics.phase24_repro_reserve_n_total += reserve_n_now;
            self.diagnostics.phase24_repro_stomach_n_total += stomach_n_now;
            // Local food is counted by actual plant accounts, not global plant count.
            let r2 = h.reproduction_food_radius * h.reproduction_food_radius;
            for plant in &self.plants.items {
                let dx = plant.x - a.x;
                let dy = plant.y - a.y;
                if dx * dx + dy * dy <= r2 {
                    self.diagnostics.phase24_repro_local_plant_count_total += 1;
                    self.diagnostics.phase24_repro_local_plant_n_total +=
                        self.resources.nutrient_balance(plant.body).unwrap_or(0.);
                    self.diagnostics.phase24_repro_local_plant_e_total +=
                        self.resources.energy_balance(plant.reserve).unwrap_or(0.);
                }
            }
            let energy_bad = e < emax * h.reproduction_min_energy_fraction;
            let other_bad = a.age < h.reproduction_min_age
                || a.reproduction_cooldown > 0
                || a.hp < a.genes.adult_max_hp * h.reproduction_min_hp_fraction
                || (h.satiety_reproduction_enabled
                    && (!h.satiety_enabled || a.well_fed_ticks < h.reproduction_well_fed_ticks))
                || self.resources.nutrient_balance(a.body).unwrap_or(0.)
                    < h.offspring_body_nutrients
                || self.resources.nutrient_balance(a.reserve).unwrap_or(0.)
                    < h.offspring_reserve_nutrients
                || e < h.offspring_energy + h.reproduction_work_energy;
            if energy_bad && !other_bad {
                self.diagnostics.phase21_repro_only_energy_blocked += 1;
            }
            if energy_bad && other_bad {
                self.diagnostics.phase21_repro_multiple_gates_blocked += 1;
            }
            if a.age < h.reproduction_min_age {
                self.diagnostics.phase20_repro_age_blocked += 1;
                continue;
            }
            if a.reproduction_cooldown > 0 {
                self.diagnostics.phase20_repro_cooldown_blocked += 1;
                continue;
            }
            if a.hp < a.genes.adult_max_hp * h.reproduction_min_hp_fraction {
                self.diagnostics.phase20_repro_hp_blocked += 1;
                continue;
            }
            let energy_max = a.genes.adult_max_hp as f64 * h.max_energy_per_hp;
            let energy = self.resources.energy_balance(a.reserve).unwrap_or(0.);
            if energy < energy_max * h.reproduction_min_energy_fraction {
                self.diagnostics.phase20_repro_energy_gate_blocked += 1;
                continue;
            }
            if h.satiety_reproduction_enabled
                && (!h.satiety_enabled || a.well_fed_ticks < h.reproduction_well_fed_ticks)
            {
                self.diagnostics.herbivore_reproduction_satiety_blocked += 1;
                self.diagnostics.phase20_repro_satiety_blocked += 1;
                continue;
            }
            if h.resource_dependent_reproduction {
                let local_plants = self.plant_grid.count_within(
                    &self.plants,
                    a.x,
                    a.y,
                    h.reproduction_food_radius,
                );
                let chance = (local_plants as f32 / h.reproduction_food_target as f32).min(1.);
                if self.rng.unit() >= chance {
                    if self.focal_trace_id == Some(a.id) {
                        self.focal_trace.push(serde_json::json!({
                            "phase": "food_gate_failed", "tick": self.tick,
                            "local_plants": local_plants, "chance": chance
                        }));
                    }
                    self.diagnostics.herbivore_reproduction_food_blocked += 1;
                    self.diagnostics.phase20_repro_food_blocked += 1;
                    continue;
                }
            }
            self.diagnostics.herbivore_reproduction_attempts += 1;
            let body_n = self.resources.nutrient_balance(a.body).unwrap_or(0.);
            let reserve_n = self.resources.nutrient_balance(a.reserve).unwrap_or(0.);
            if body_n < h.offspring_body_nutrients {
                self.diagnostics.herbivore_reproduction_resource_blocked += 1;
                self.diagnostics.phase20_repro_body_n_blocked += 1;
                continue;
            }
            if reserve_n < h.offspring_reserve_nutrients {
                self.diagnostics.herbivore_reproduction_resource_blocked += 1;
                self.diagnostics.phase20_repro_reserve_n_blocked += 1;
                continue;
            }
            if energy < h.offspring_energy + h.reproduction_work_energy {
                self.diagnostics.herbivore_reproduction_resource_blocked += 1;
                self.diagnostics.phase20_repro_transfer_e_blocked += 1;
                continue;
            }
            let parent_id = self.animals.items[i].id;
            let (px, py, genes, body_src, reserve_src, max_age) = {
                let a = &self.animals.items[i];
                (a.x, a.y, a.genes, a.body, a.reserve, a.max_age)
            };
            let mutate = |value: f32,
                          lo: f32,
                          hi: f32,
                          rng: &mut crate::rng::SimulationRng,
                          diagnostics: &mut super::Diagnostics|
             -> f32 {
                if rng.unit() < h.mutation_probability {
                    diagnostics.herbivore_mutations += 1;
                    (value * (1. + (rng.unit() * 2. - 1.) * h.mutation_scale)).clamp(lo, hi)
                } else {
                    value
                }
            };
            let hp = mutate(
                genes.adult_max_hp,
                20.,
                200.,
                &mut self.rng,
                &mut self.diagnostics,
            );
            let speed = mutate(genes.speed, 0.5, 8., &mut self.rng, &mut self.diagnostics);
            let vision = mutate(
                genes.vision,
                20.,
                150.,
                &mut self.rng,
                &mut self.diagnostics,
            );
            // Color has its own deterministic stream and does not consume ecology RNG.
            let color_seed = self.config.seed ^ self.tick.rotate_left(17);
            let (color, color_flags) = crate::animals::color::inherit_with_flags(
                genes.body_color,
                color_seed,
                self.births.wrapping_add(1),
                h.mutation_probability,
                h.mutation_scale,
            );
            let fear = (genes.fear + (self.rng.unit() - 0.5) * 0.12).clamp(0., 1.);
            let aggression = (genes.aggression + (self.rng.unit() - 0.5) * 0.12).clamp(0., 1.);
            let mut resistance = genes.toxin_resistance;
            let mut resistance_mutation_attempted = false;
            if self.config.toxin.resistance_enabled {
                self.diagnostics.resistance_births += 1;
                if self.rng.unit() < self.config.toxin.resistance_mutation_probability as f32 {
                    resistance_mutation_attempted = true;
                    resistance = (resistance
                        + (self.rng.unit() * 2. - 1.)
                            * self.config.toxin.resistance_mutation_scale as f32)
                        .clamp(0., 1.);
                    self.diagnostics.resistance_mutations += 1;
                }
            }
            let mut typed_resistance = genes.typed_resistance;
            if self.config.toxin.herbivore_multi_toxin_enabled
                && self.config.toxin.resistance_enabled
            {
                for (toxin_type, level) in typed_resistance
                    .iter_mut()
                    .enumerate()
                    .take(self.config.toxin.plant_toxin_type_count as usize)
                    .skip(1)
                {
                    let draw = crate::toxin::herbivore_type_draw(
                        self.config.seed,
                        self.births.wrapping_add(1),
                        toxin_type as u16,
                        0,
                    );
                    if draw < self.config.toxin.resistance_mutation_probability {
                        let direction = crate::toxin::herbivore_type_draw(
                            self.config.seed,
                            self.births.wrapping_add(1),
                            toxin_type as u16,
                            1,
                        ) * 2.
                            - 1.;
                        *level = (*level
                            + (direction * self.config.toxin.resistance_mutation_scale) as f32)
                            .clamp(0., 1.);
                        self.diagnostics.resistance_mutations += 1;
                    }
                }
            }
            typed_resistance[0] = resistance;
            let child_genes = HerbivoreGenes {
                adult_max_hp: hp,
                speed,
                vision,
                feeding_range: genes.feeding_range,
                body_color: color,
                fear,
                aggression,
                toxin_resistance: resistance,
                typed_resistance,
            };
            let angle = self.rng.unit() * std::f32::consts::TAU;
            let x = (px + angle.cos() * 6.).clamp(0., self.config.world.width);
            let y = (py + angle.sin() * 6.).clamp(0., self.config.world.height);
            let child_body = self
                .resources
                .open(0., 0.)
                .map_err(|e| format!("offspring body: {e:?}"))?;
            let child_reserve = self
                .resources
                .open(0., 0.)
                .map_err(|e| format!("offspring reserve: {e:?}"))?;
            let child_stomach = self
                .resources
                .open(0., 0.)
                .map_err(|e| format!("offspring stomach: {e:?}"))?;
            self.resources
                .transfer_nutrients(body_src, child_body, h.offspring_body_nutrients)
                .map_err(|e| format!("offspring body N: {e:?}"))?;
            self.resources
                .transfer_nutrients(reserve_src, child_reserve, h.offspring_reserve_nutrients)
                .map_err(|e| format!("offspring reserve N: {e:?}"))?;
            self.diagnostics
                .phase21_reproduction_offspring_e_transferred += h.offspring_energy;
            self.diagnostics.phase21_reproduction_work_e += h.reproduction_work_energy;
            self.resources
                .transfer_energy(reserve_src, child_reserve, h.offspring_energy)
                .map_err(|e| format!("offspring E: {e:?}"))?;
            self.ledger
                .dissipate_heat(&mut self.resources, reserve_src, h.reproduction_work_energy)
                .map_err(|e| format!("reproduction work: {e:?}"))?;
            self.animals.items[i].reproduction_cooldown = h.reproduction_cooldown_ticks;
            let id = self.entities.spawn()?;
            let child_max_age = ((max_age as f64) * (0.9 + 0.2 * self.rng.unit() as f64))
                .round()
                .max(1.) as u32;
            let exploration_heading = self.rng.unit() * std::f32::consts::TAU;
            self.animals.items.push(Herbivore {
                id,
                x,
                y,
                hp: hp * 0.75,
                growth: 0.,
                genes: child_genes,
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
                body: child_body,
                reserve: child_reserve,
                stomach: child_stomach,
                feeding_active: true,
                well_fed_ticks: 0,
                age: 0,
                max_age: child_max_age,
                reproduction_cooldown: h.reproduction_cooldown_ticks,
                exploration_heading,
                exploration_ticks: 30,
                target: None,
            });
            if self.resistance_event_logging {
                self.resistance_birth_events
                    .push(super::ResistanceBirthEvent {
                        tick: self.tick,
                        parent_id,
                        child_id: id,
                        parent_resistance: genes.toxin_resistance,
                        child_resistance: resistance,
                        mutation_attempted: resistance_mutation_attempted,
                    });
            }
            self.animals.items[i].successful_births += 1;
            self.births += 1;
            self.diagnostics.herbivore_births += 1;
            self.diagnostics.phase20_repro_success += 1;
            self.diagnostics.color_inheritances += 1;
            let changed = color_flags.iter().filter(|&&flag| flag).count() as u64;
            self.diagnostics.color_mutated_channels += changed;
            if changed > 0 {
                self.diagnostics.color_mutated_births += 1;
            }
            if self.color_event_logging {
                self.color_events.push(super::ColorBirthEvent {
                    tick: self.tick,
                    parent_id,
                    child_id: id,
                    parent_rgb: genes.body_color.rgb(),
                    child_rgb: color.rgb(),
                    mutated: color_flags,
                });
            }
        }
        Ok(())
    }
}
