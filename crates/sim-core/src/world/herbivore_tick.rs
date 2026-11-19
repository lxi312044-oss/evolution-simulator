use super::{
    behavior::{Action, Perception, Policy},
    World,
};
use crate::animals::{Herbivore, HerbivoreGenes};
impl World {
    pub(super) fn update_herbivores(&mut self) -> Result<(), String> {
        let h = self.config.herbivores.clone();
        self.plant_grid.rebuild(&self.plants);
        let plants_protected = h.plant_protection_enabled
            && self.plants.len() <= h.plant_protection_threshold as usize;
        // Global conservation safeguard: prohibit feeding at or below the configured living-plant threshold.
        // The threshold is evaluated once per tick; no N or E is created or destroyed.
        // One deterministic search and movement step per animal. The grid prevents O(animals * plants) scans.
        for i in 0..self.animals.items.len() {
            let animal = &self.animals.items[i];
            let foraging_observed = self
                .foraging_trace
                .as_ref()
                .is_some_and(|t| t.observes(animal.id, self.tick));
            let before_feeding = foraging_observed.then(|| {
                (
                    self.resources
                        .nutrient_balance(animal.stomach)
                        .unwrap_or(0.),
                    self.resources.energy_balance(animal.reserve).unwrap_or(0.),
                    animal.feeding_active,
                )
            });
            let mut digested_n = 0.;
            let health_observed = self
                .health_trace
                .as_ref()
                .is_some_and(|t| t.watched.contains(&self.animals.items[i].id));
            if health_observed {
                self.health_trace.as_mut().unwrap().record(
                    self.tick,
                    &self.animals.items[i],
                    &self.resources,
                    "begin_herbivore",
                    None,
                    serde_json::Value::Null,
                );
            }

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
                digested_n = digest;
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
            let basal = h.basal_energy_per_tick
                * (hp_max as f64 / 60.).powf(0.75)
                * crate::animals::speed_upkeep_factor(
                    h.speed_upkeep_slope,
                    speed,
                    (h.speed_min + h.speed_max) / 2.,
                );
            let current = self.resources.energy_balance(reserve).unwrap_or(0.);
            self.ledger
                .dissipate_heat(&mut self.resources, reserve, basal.min(current))
                .map_err(|e| format!("herbivore metabolism: {e:?}"))?;
            self.record_energy(
                self.animals.items[i].id,
                "basal",
                current,
                self.resources.energy_balance(reserve).unwrap_or(0.),
                -basal.min(current),
            );
            if health_observed {
                self.health_trace.as_mut().unwrap().record(self.tick,&self.animals.items[i],&self.resources,
                    "basal",None,serde_json::json!({"requested_e":basal,"paid_e":basal.min(current),"reserve_before":current}));
            }
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
            let color_memory = if h.color_learning_enabled {
                self.plant_color_learning.get(&self.animals.items[i].id)
            } else {
                None
            };
            let observed_reserve_fraction = foraging_observed.then(|| {
                self.resources.energy_balance(reserve).unwrap_or(0.)
                    / (f64::from(hp_max) * h.max_energy_per_hp)
            });
            let mut target = if !may_feed {
                None
            } else if let Some(memory) = color_memory {
                let reserve_fraction = self.resources.energy_balance(reserve).unwrap_or(0.)
                    / (f64::from(hp_max) * h.max_energy_per_hp);
                self.plant_grid
                    .nearest_allowed(&self.plants, x, y, vision, |color| {
                        memory.allows(color, reserve_fraction)
                    })
            } else {
                self.plant_grid.nearest(&self.plants, x, y, vision)
            };
            let mut hungry_color_recheck = false;
            let recovery_enabled = h.hungry_color_recheck_enabled
                && h.hungry_color_recovery_enabled
                && h.color_learning_enabled;
            let id = self.animals.items[i].id;
            let mut recovery_active = recovery_enabled && self.hungry_color_recovery.contains(&id);
            let mut recovery_released = false;
            if recovery_active {
                let fraction = self.resources.energy_balance(reserve).unwrap_or(0.)
                    / (f64::from(hp_max) * h.max_energy_per_hp);
                if fraction >= 0.30 {
                    self.hungry_color_recovery.remove(&id);
                    recovery_active = false;
                    recovery_released = true;
                }
            }
            if h.hungry_color_recheck_enabled && may_feed {
                if let Some(memory) = color_memory {
                    let fraction = self.resources.energy_balance(reserve).unwrap_or(0.)
                        / (f64::from(hp_max) * h.max_energy_per_hp);
                    let edible = |j: usize| {
                        self.resources
                            .nutrient_balance(self.plants.items[j].body)
                            .unwrap_or(0.)
                            > 0.
                    };
                    if (fraction < 0.25 || recovery_active) && !target.is_some_and(edible) {
                        let nearest_edible =
                            self.plant_grid
                                .nearest_where(&self.plants, x, y, vision, edible);
                        if nearest_edible.is_some()
                            && self
                                .plant_grid
                                .nearest_where(&self.plants, x, y, vision, |j| {
                                    edible(j) && memory.allows(self.plants.items[j].color, fraction)
                                })
                                .is_none()
                        {
                            // Tolerance 1 for the selected edible target in this tick only.
                            target = nearest_edible;
                            hungry_color_recheck = true;
                            if recovery_enabled && !recovery_active {
                                self.hungry_color_recovery.insert(id);
                                recovery_active = true;
                            }
                        }
                    }
                }
            }
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
                self.record_energy(
                    self.animals.items[i].id,
                    "movement",
                    energy,
                    self.resources.energy_balance(reserve).unwrap_or(0.),
                    -charge,
                );
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
            let mut feeding_outcome = None;
            let decision = if let Some((stomach_before, reserve_before, feeding_before)) =
                before_feeding
            {
                let mut visible = 0_u64;
                let mut visible_n = 0_u64;
                let mut visible_ne = 0_u64;
                let mut color_rejected_n = 0_u64;
                let mixed_observation = self.config.toxin.experimental_plant_toxicity_colors;
                let inherited = self.config.toxin.experimental_inherited_plant_colors;
                let mut color_visible = [0_u64; 2];
                let mut color_in_range = [0_u64; 2];
                let mut color_rejected = [0_u64; 2];
                let stomach_n = self
                    .resources
                    .nutrient_balance(self.animals.items[i].stomach)
                    .unwrap_or(0.);
                let can_ingest = h.feeding_nutrients_per_tick > 0.
                    && (!h.satiety_enabled
                        || h.feeding_nutrient_absorption == 0.
                        || stomach_n < h.stomach_capacity_n);
                let mut visible_by_toxicity = [0_u64; 2];
                let mut rejected_by_toxicity = [0_u64; 2];
                let mut in_range_by_toxicity = [0_u64; 2];
                let memory = h
                    .color_learning_enabled
                    .then(|| self.plant_color_learning.get(&self.animals.items[i].id))
                    .flatten();
                for p in &self.plants.items {
                    let in_vision = (p.x - x).powi(2) + (p.y - y).powi(2) <= vision * vision;
                    if mixed_observation
                        && (p.x - ax).powi(2) + (p.y - ay).powi(2) <= range * range
                        && self.resources.nutrient_balance(p.body).unwrap_or(0.) > 0.
                    {
                        in_range_by_toxicity[usize::from(p.produces_toxin)] += 1;
                    }
                    if in_vision {
                        visible += 1;
                        let n = self.resources.nutrient_balance(p.body).unwrap_or(0.);
                        let e = self.resources.energy_balance(p.reserve).unwrap_or(0.);
                        if n > 0. {
                            visible_n += 1;
                            if inherited {
                                let color = usize::from(p.color == 0xb86be3ff);
                                color_visible[color] += 1;
                                if can_ingest
                                    && may_feed
                                    && (p.x - x).powi(2) + (p.y - y).powi(2) <= range * range
                                {
                                    color_in_range[color] += 1;
                                }
                                if memory.is_some_and(|m| {
                                    !m.allows(p.color, observed_reserve_fraction.unwrap())
                                }) {
                                    color_rejected[color] += 1;
                                }
                            }
                            if mixed_observation {
                                visible_by_toxicity[usize::from(p.produces_toxin)] += 1;
                            }
                            if memory.is_some_and(|m| {
                                !m.allows(p.color, observed_reserve_fraction.unwrap())
                            }) {
                                color_rejected_n += 1;
                                if mixed_observation {
                                    rejected_by_toxicity[usize::from(p.produces_toxin)] += 1;
                                }
                            }
                            if e > 0. {
                                visible_ne += 1;
                            }
                        }
                    }
                }
                let observed_target = target.map(|j| {let p=&self.plants.items[j];serde_json::json!({"id":[p.id.index,p.id.generation],
                        "distance_after":((ax-p.x).powi(2)+(ay-p.y).powi(2)).sqrt(),
                        "in_range":(ax-p.x).powi(2)+(ay-p.y).powi(2) <= range*range,
                        "n":self.resources.nutrient_balance(p.body).unwrap_or(0.),"e":self.resources.energy_balance(p.reserve).unwrap_or(0.)})});
                let mixed = serde_json::json!({"visible":visible_by_toxicity,"rejected":rejected_by_toxicity,
                    "in_range":in_range_by_toxicity,"target_toxic":target.map(|j| self.plants.items[j].produces_toxin)});
                let inherited_choice = inherited.then(|| {
                    let mut remembered = [false; 2];
                    if let Some(memory) = memory {
                        for (color, _) in memory.harm_scores() {
                            if color == 0x4ecb7000 { remembered[0] = true; }
                            if color == 0xb86be300 { remembered[1] = true; }
                        }
                    }
                    serde_json::json!({"visible":color_visible,"in_range":color_in_range,
                        "rejected":color_rejected,"both_remembered":remembered.iter().all(|&v| v),
                        "target":target.map(|j| usize::from(self.plants.items[j].color == 0xb86be3ff)),
                        "target_toxic":target.map(|j| self.plants.items[j].produces_toxin)})
                });
                let mut event = serde_json::json!({"id":[self.animals.items[i].id.index,self.animals.items[i].id.generation],"tick":self.tick,
                    "plants":self.plants.len(),"plant_protected":plants_protected,"satiety_blocked":h.satiety_enabled && !self.animals.items[i].feeding_active,
                    "may_feed":may_feed,"action":format!("{action:?}"),"visible":visible,"visible_n":visible_n,"visible_ne":visible_ne,"color_rejected_visible_n":color_rejected_n,
                    "position_before":[x,y],"position_after":[ax,ay],"vision":vision,"range":range,
                    "desired_step":desired_distance,"affordable_step":step,"movement_energy":energy,
                    "reserve_e":self.resources.energy_balance(reserve).unwrap_or(0.),
                    "mixed":mixed,
                    "stomach_n_before_digest":stomach_before,"digested_n":digested_n,
                    "stomach_n_before_feeding":self.resources.nutrient_balance(self.animals.items[i].stomach).unwrap_or(0.),
                    "reserve_e_before_tick":reserve_before,"hp_before_feeding":self.animals.items[i].hp,
                    "reserve_fraction_at_choice":observed_reserve_fraction,
                    "max_reserve_e":f64::from(hp_max)*h.max_energy_per_hp,
                    "hungry_color_recheck":hungry_color_recheck,
                    "recovery_active":recovery_active,"recovery_released":recovery_released,
                    "energy_threshold":f64::from(hp_max)*h.max_energy_per_hp*h.reproduction_min_energy_fraction,
                    "feeding_active_before":feeding_before,"feeding_active":self.animals.items[i].feeding_active,
                    "streak_before":prior_streak,"streak_after":self.animals.items[i].well_fed_ticks,
                    "target": observed_target});
                if let Some(choice) = inherited_choice {
                    event["inherited_choice"] = choice;
                }
                Some(event)
            } else {
                None
            };
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
                        if self.config.animal_toxin.enabled {
                            self.animal_toxins.plant_intake[exposure_type] += dose;
                        }
                        if h.color_learning_enabled && dose > 0. {
                            self.plant_color_learning
                                .entry(self.animals.items[i].id)
                                .or_default()
                                .ingest(self.plants.items[pi].color, exposure_type, dose);
                        }
                        self.diagnostics.toxin_exposure_events += 1;
                        self.diagnostics.toxin_exposure_total += dose;
                    }
                    // Opt-in: eating plant matter without ingesting toxin is safe evidence
                    // for the visible color. Only the herbivore's own experience is used.
                    if h.color_learning_enabled
                        && h.color_memory_safe_meal_relief > 0.
                        && eaten > 0.
                        && !(self.config.toxin.enabled && self.plants.items[pi].produces_toxin)
                    {
                        let color = self.plants.items[pi].color;
                        if let Some(memory) =
                            self.plant_color_learning.get_mut(&self.animals.items[i].id)
                        {
                            memory.relieve_safe_meal(color, h.color_memory_safe_meal_relief);
                        }
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
                    let feeding_before = self.resources.energy_balance(ares).unwrap_or(0.);
                    if gained > 0. {
                        self.resources
                            .transfer_energy(pres, ares, gained)
                            .map_err(|e| format!("feeding energy: {e:?}"))?;
                    }
                    self.record_energy(
                        self.animals.items[i].id,
                        "feeding",
                        feeding_before,
                        self.resources.energy_balance(ares).unwrap_or(0.),
                        gained,
                    );
                    if health_observed {
                        self.health_trace.as_mut().unwrap().record(self.tick,&self.animals.items[i],&self.resources,
                            "feeding",None,serde_json::json!({"eaten_n":eaten,"gained_e":gained,
                            "plant":[self.plants.items[pi].id.index,self.plants.items[pi].id.generation],
                            "available_plant_e":plant_e,"wanted_e":desired}));
                    }
                    if foraging_observed {
                        feeding_outcome = Some(
                            serde_json::json!({"attempted":true,"eaten_n":eaten,"gained_e":gained,
                            "free_stomach_n":if free_space.is_finite(){Some(free_space)}else{None},"energy_capacity":capacity,
                                "available_plant_n":available,"available_plant_e":plant_e,
                                "absorbed_n":absorbed,"waste_n":waste}),
                        );
                    }
                    let dissipated = available_e - gained;
                    if dissipated > 0. {
                        self.ledger
                            .dissipate_heat(&mut self.resources, pres, dissipated)
                            .map_err(|e| format!("digestion heat: {e:?}"))?;
                    }
                }
            }
            if let Some(mut event) = decision {
                event["feeding"] = feeding_outcome.unwrap_or_else(
                    || serde_json::json!({"attempted":false,"eaten_n":0.,"gained_e":0.}),
                );
                event["reserve_e_after_feeding"] =
                    serde_json::json!(self.resources.energy_balance(reserve).unwrap_or(0.));
                self.foraging_trace.as_mut().unwrap().events.push(event);
            }
            // Opt-in forgetting of learned plant-color harm, before this tick's damage.
            if h.color_learning_enabled && h.color_memory_decay_per_tick > 0. {
                if let Some(memory) = self.plant_color_learning.get_mut(&self.animals.items[i].id) {
                    memory.decay(h.color_memory_decay_per_tick);
                }
            }
            // Clearance and physiological damage; HP is a health state, not a resource balance.
            if self.config.toxin.enabled {
                let t = &self.config.toxin;
                let animal = &mut self.animals.items[i];
                let mut memory = if h.color_learning_enabled {
                    self.plant_color_learning.get_mut(&animal.id)
                } else {
                    None
                };
                let load_zero = animal.toxin_load;
                let result = crate::toxin::advance_burden(
                    animal.toxin_load,
                    animal.genes.toxin_resistance,
                    t,
                );
                animal.toxin_load = result.remaining_load;
                if self.config.animal_toxin.enabled {
                    self.animal_toxins.cleared[0] += result.cleared;
                }
                if t.herbivore_multi_toxin_enabled {
                    for toxin_type in 1..t.plant_toxin_type_count as usize {
                        let load = animal.typed_load[toxin_type];
                        let resistance = animal.genes.typed_resistance[toxin_type];
                        let step = crate::toxin::advance_burden(load, resistance, t);
                        animal.typed_load[toxin_type] = step.remaining_load;
                        if self.config.animal_toxin.enabled {
                            self.animal_toxins.cleared[toxin_type] += step.cleared;
                        }
                        if let Some(memory) = memory.as_deref_mut() {
                            memory.observe_damage(
                                toxin_type,
                                load,
                                step.remaining_load,
                                step.damage,
                            );
                        }
                        self.diagnostics.toxin_clearance_total += step.cleared;
                        self.diagnostics.resistance_prevented_damage_total += step.prevented_damage;
                        animal.lifetime_prevented_damage += step.prevented_damage;
                        animal.lifetime_toxin_damage += step.damage as f64;
                        if step.damage > 0. {
                            self.diagnostics.toxin_damage_ticks += 1;
                            self.diagnostics.toxin_hp_damage_total += step.damage as f64;
                        }
                        let hp_before = animal.hp;
                        animal.hp -= step.damage;
                        if health_observed {
                            self.health_trace.as_mut().unwrap().record(self.tick,animal,&self.resources,
                                "toxin",Some(hp_before),serde_json::json!({"type":toxin_type,"damage":step.damage,"load_before":load}));
                        }
                    }
                }
                if let Some(memory) = memory {
                    memory.observe_damage(0, load_zero, result.remaining_load, result.damage);
                }
                self.diagnostics.toxin_clearance_total += result.cleared;
                self.diagnostics.resistance_prevented_damage_total += result.prevented_damage;
                animal.lifetime_prevented_damage += result.prevented_damage;
                animal.lifetime_toxin_damage += result.damage as f64;
                if result.damage > 0. {
                    self.diagnostics.toxin_damage_ticks += 1;
                    self.diagnostics.toxin_hp_damage_total += result.damage as f64;
                }
                let hp_before = animal.hp;
                animal.hp -= result.damage;
                if health_observed {
                    self.health_trace.as_mut().unwrap().record(
                        self.tick,
                        animal,
                        &self.resources,
                        "toxin",
                        Some(hp_before),
                        serde_json::json!({"type":0,"damage":result.damage}),
                    );
                }
            }
            let upkeep_before =
                health_observed.then(|| self.resources.energy_balance(reserve).unwrap_or(0.));
            let paid_before =
                health_observed.then_some(self.animals.items[i].lifetime_resistance_cost);
            if self.config.toxin.resistance_enabled
                && self.resistance_upkeep_exempt != Some(self.animals.items[i].id)
            {
                let animal = &self.animals.items[i];
                let resistance = self
                    .focal_upkeep_override
                    .filter(|(id, _)| *id == animal.id)
                    .map_or(animal.genes.toxin_resistance, |(_, value)| value)
                    as f64;
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
                    let energy_before = self.resources.energy_balance(reserve).unwrap_or(0.);
                    self.ledger
                        .dissipate_heat(&mut self.resources, reserve, cost)
                        .map_err(|e| format!("resistance upkeep: {e:?}"))?;
                    self.record_energy(
                        self.animals.items[i].id,
                        "resistance_upkeep",
                        energy_before,
                        self.resources.energy_balance(reserve).unwrap_or(0.),
                        -cost,
                    );
                    self.diagnostics.resistance_energy_cost_total += cost;
                    self.animals.items[i].lifetime_resistance_cost += cost;
                }
            }
            if health_observed {
                self.health_trace.as_mut().unwrap().record(
                    self.tick,
                    &self.animals.items[i],
                    &self.resources,
                    "resistance_upkeep",
                    None,
                    serde_json::json!({"reserve_before":upkeep_before,
                    "paid_e":self.animals.items[i].lifetime_resistance_cost-paid_before.unwrap()}),
                );
            }
            let energy = self.resources.energy_balance(reserve).unwrap_or(0.);
            if energy <= 1e-9 {
                self.diagnostics.herbivore_zero_energy_ticks += 1;
                self.diagnostics.phase21_starvation_damage_ticks += 1;
                self.diagnostics.phase21_starvation_hp_damage +=
                    (hp_max * h.starvation_hp_fraction) as f64;
                let hp_before = self.animals.items[i].hp;
                self.animals.items[i].hp -= hp_max * h.starvation_hp_fraction;
                if health_observed {
                    self.health_trace.as_mut().unwrap().record(self.tick,&self.animals.items[i],&self.resources,
                        "starvation",Some(hp_before),serde_json::json!({"damage":hp_max*h.starvation_hp_fraction,"reserve_e":energy}));
                }
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
            if self.config.toxin.experimental_inherited_plant_colors {
                if let Some(trace) = &mut self.foraging_trace {
                    trace.events.push(serde_json::json!({"plant_event":"consumed","color":p.color,"toxic":p.produces_toxin}));
                }
            }
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
            if let Some(trace) = &mut self.health_trace {
                trace.record(self.tick,&self.animals.items[i],&self.resources,
                    "pre_removal",None,serde_json::json!({"trigger":if self.animals.items[i].hp<=0. {"hp_nonpositive"} else {"age_limit"}}));
            }
            let last_index = self.animals.items.len() - 1;
            if i != last_index {
                let moved = self.animals.items[last_index].id;
                if let Some(trace) = &mut self.health_trace {
                    if trace.watches_reproduction(self.tick, moved) {
                        trace.record_reproduction(
                            self.tick,
                            moved,
                            "swap_remove_move",
                            serde_json::json!({"from":last_index,"to":i}),
                        );
                    }
                }
            }
            let a = self.animals.items.swap_remove(i);
            if self.config.animal_toxin.enabled {
                self.animal_toxins.remove_herbivore(&a);
            }
            self.plant_color_learning.remove(&a.id);
            self.hungry_color_recovery.remove(&a.id);
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
                    if account == a.reserve {
                        self.record_energy(
                            a.id,
                            "removal_heat",
                            e,
                            self.resources.energy_balance(account).unwrap_or(0.),
                            -e,
                        );
                    }
                }
            }
            let remaining = self.resources.energy_balance(a.reserve).unwrap_or(0.);
            self.record_energy(a.id, "removal", 0., remaining, 0.);
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
        let capacity = self.config.world.max_living_entities as usize;
        let plants = self.plants.items.len();
        let mut cutoff = None;
        for i in 0..parents {
            if cutoff.is_none()
                && plants + self.animals.items.len() + self.carnivores.len() >= capacity
            {
                self.diagnostics.herbivore_reproduction_capacity_blocked += 1;
                if !self.health_trace.as_ref().is_some_and(|trace| {
                    trace.observe_reproduction
                        && (trace.window.0..=trace.window.1).contains(&self.tick)
                }) {
                    break;
                }
                cutoff = Some(i);
            }
            let a = &self.animals.items[i];
            let watched = self
                .health_trace
                .as_ref()
                .is_some_and(|trace| trace.watches_reproduction(self.tick, a.id));
            if cutoff.is_some() && !watched {
                continue;
            }
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
            if let Some(first_unprocessed) = cutoff {
                self.health_trace.as_mut().unwrap().record_reproduction(
                    self.tick, a.id, "capacity_blocked", serde_json::json!({
                        "age_ok":age_ok,"cooldown_ok":cooldown_ok,"hp_ok":hp_ok,
                        "energy_ok":energy_ok,"satiety_ok":satiety_ok,"body_n_ok":body_n_ok,
                        "reserve_n_ok":reserve_n_ok,"transfer_e_ok":transfer_e_ok,
                        "hp":a.hp,"hp_threshold":a.genes.adult_max_hp*h.reproduction_min_hp_fraction,
                        "age":a.age,"cooldown":a.reproduction_cooldown,"well_fed_ticks":a.well_fed_ticks,
                        "reserve_e":e,"energy_threshold":emax*h.reproduction_min_energy_fraction,
                        "body_n":body_n_now,"reserve_n":reserve_n_now,"stomach_n":stomach_n_now,
                        "index":i,"first_unprocessed":first_unprocessed,
                        "plants":plants,"herbivores":self.animals.items.len(),
                        "carnivores":self.carnivores.len(),"capacity":capacity,
                        "entry_herbivores":parents,"earlier_births":self.animals.items.len()-parents,
                        "food_evaluated":false
                    }),
                );
                continue;
            }
            if self.focal_trace_id == Some(a.id) || watched {
                let gates = serde_json::json!({
                    "phase": "reproduction_gates", "tick": self.tick,
                    "hp": a.hp, "hp_threshold": a.genes.adult_max_hp * h.reproduction_min_hp_fraction,
                    "reserve_e": e, "energy_threshold": emax * h.reproduction_min_energy_fraction,
                    "body_n": body_n_now, "reserve_n": reserve_n_now, "stomach_n": stomach_n_now,
                    "age_ok": age_ok, "cooldown_ok": cooldown_ok, "hp_ok": hp_ok,
                    "energy_ok": energy_ok, "satiety_ok": satiety_ok, "body_n_ok": body_n_ok,
                    "reserve_n_ok": reserve_n_ok, "transfer_e_ok": transfer_e_ok,
                    "births": a.successful_births, "well_fed_ticks": a.well_fed_ticks,
                    "age":a.age,"cooldown":a.reproduction_cooldown,
                    "index":i, "entry_herbivores":parents,
                });
                if self.focal_trace_id == Some(a.id) {
                    self.focal_trace.push(gates.clone());
                }
                if watched {
                    self.health_trace.as_mut().unwrap().record_reproduction(
                        self.tick,
                        a.id,
                        "reproduction_gates",
                        gates,
                    );
                }
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
                let failed = self.rng.unit() >= chance;
                if watched {
                    self.health_trace.as_mut().unwrap().record_reproduction(
                        self.tick, a.id, "food_gate",
                        serde_json::json!({"passed":!failed,"local_plants":local_plants,"chance":chance}),
                    );
                }
                if failed {
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
            let mutated_speed = mutate(genes.speed, 0.5, 8., &mut self.rng, &mut self.diagnostics);
            // Experimental control: discard the drawn speed so the RNG sequence is unchanged.
            let speed = if h.speed_mutation_enabled {
                mutated_speed
            } else {
                genes.speed
            };
            let vision = mutate(
                genes.vision,
                20.,
                150.,
                &mut self.rng,
                &mut self.diagnostics,
            );
            // Color has its own deterministic stream and does not consume ecology RNG.
            let color_seed = self.config.seed ^ self.tick.rotate_left(17);
            // Issue #76 (opt-in): body color may mutate separately from the other genes.
            let (color, color_flags) = crate::animals::color::inherit_with_flags(
                genes.body_color,
                color_seed,
                self.births.wrapping_add(1),
                h.body_color_mutation_probability
                    .unwrap_or(h.mutation_probability),
                h.body_color_mutation_scale.unwrap_or(h.mutation_scale),
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
            if let Some(trace) = &mut self.health_trace {
                if trace.watches_reproduction(self.tick, parent_id) {
                    trace.record(
                        self.tick,
                        &self.animals.items[i],
                        &self.resources,
                        "parent_before_birth",
                        None,
                        serde_json::Value::Null,
                    );
                }
            }
            self.resources
                .transfer_nutrients(body_src, child_body, h.offspring_body_nutrients)
                .map_err(|e| format!("offspring body N: {e:?}"))?;
            self.resources
                .transfer_nutrients(reserve_src, child_reserve, h.offspring_reserve_nutrients)
                .map_err(|e| format!("offspring reserve N: {e:?}"))?;
            self.diagnostics
                .phase21_reproduction_offspring_e_transferred += h.offspring_energy;
            self.diagnostics.phase21_reproduction_work_e += h.reproduction_work_energy;
            let parent_energy_before = self.resources.energy_balance(reserve_src).unwrap_or(0.);
            self.resources
                .transfer_energy(reserve_src, child_reserve, h.offspring_energy)
                .map_err(|e| format!("offspring E: {e:?}"))?;
            let before_work = self.resources.energy_balance(reserve_src).unwrap_or(0.);
            self.record_energy(
                parent_id,
                "offspring_transfer",
                parent_energy_before,
                before_work,
                -h.offspring_energy,
            );
            self.ledger
                .dissipate_heat(&mut self.resources, reserve_src, h.reproduction_work_energy)
                .map_err(|e| format!("reproduction work: {e:?}"))?;
            self.record_energy(
                parent_id,
                "reproduction_work",
                before_work,
                self.resources.energy_balance(reserve_src).unwrap_or(0.),
                -h.reproduction_work_energy,
            );
            self.animals.items[i].reproduction_cooldown = h.reproduction_cooldown_ticks;
            let id = self.entities.spawn()?;
            let child_max_age = ((max_age as f64) * (0.9 + 0.2 * self.rng.unit() as f64))
                .round()
                .max(1.) as u32;
            let exploration_heading = self.rng.unit() * std::f32::consts::TAU;
            // Opt-in: the offspring is born as a carnivore with the herbivore child's
            // resources and inherited body color (Issue 11, simple version).
            if self.config.carnivores.enabled
                && crate::animals::carnivore::offspring_is_carnivore(
                    h.carnivore_offspring_probability,
                    self.config.seed,
                    self.tick,
                    id.index as u64,
                )
            {
                let c = &self.config.carnivores;
                self.carnivores.push(crate::animals::carnivore::Carnivore {
                    id,
                    x,
                    y,
                    age: 0,
                    max_age: c.max_age_ticks,
                    cooldown: c.reproduction_cooldown_ticks,
                    attack_cooldown: 0,
                    satiety_ticks: 0,
                    stomach: child_stomach,
                    feeding_active: true,
                    body: child_body,
                    reserve: child_reserve,
                    color: child_genes.body_color,
                    speed: if c.speed_evolution_enabled {
                        crate::animals::carnivore::initial_speed(
                            c.speed,
                            c.initial_speed_spread,
                            self.config.seed,
                            id.index as u64,
                        )
                    } else {
                        c.speed
                    },
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
                self.species_carnivore_birth(self.carnivores.len() - 1, None, Some(i));
                self.animals.items[i].successful_births += 1;
                self.carnivore_births += 1;
                self.defense.carnivores_born_from_herbivores += 1;
                continue;
            }
            if self.config.animal_toxin.enabled {
                self.inherit_herbivore_production(parent_id, id);
            }
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
            self.species_herbivore_birth(self.animals.items.len() - 1, Some(i));
            self.record_energy(
                id,
                "birth",
                0.,
                self.resources.energy_balance(child_reserve).unwrap_or(0.),
                h.offspring_energy,
            );
            if let Some(trace) = &mut self.health_trace {
                trace.record(
                    self.tick,
                    self.animals.items.last().unwrap(),
                    &self.resources,
                    "birth",
                    None,
                    serde_json::json!({"parent":[parent_id.index,parent_id.generation]}),
                );
            }
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
            if let Some(trace) = &mut self.health_trace {
                if trace.watches_reproduction(self.tick, parent_id) {
                    trace.record(
                        self.tick,
                        &self.animals.items[i],
                        &self.resources,
                        "parent_after_birth",
                        None,
                        serde_json::Value::Null,
                    );
                }
            }
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
