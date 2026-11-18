use super::{
    behavior::{Action, Perception, Policy},
    World,
};
use crate::animals::carnivore::Carnivore;
// Bin values at the moment of the event, not at CSV export time.
fn ambush_bin(gene: f32) -> usize {
    ((gene.clamp(0., 1.) * 5.) as usize).min(4)
}
impl World {
    pub fn update_carnivores(&mut self) -> Result<(), String> {
        let c = self.config.carnivores.clone();
        let required_birth_energy =
            c.offspring_energy + c.reproduction_work_energy + c.reproduction_parent_reserve_energy;
        // Fixed-duration demographic observation; one sample for all predators in this tick.
        // This only affects births when explicitly enabled, and never changes resource totals.
        if c.prey_trend_reproduction_enabled
            && (self.prey_trend_snapshot.tick == 0
                || self.tick.saturating_sub(self.prey_trend_snapshot.tick)
                    >= c.prey_trend_window_ticks)
        {
            let current = self.animals.items.len();
            let snap = &mut self.prey_trend_snapshot;
            if snap.tick > 0 {
                snap.births_in_window = self
                    .diagnostics
                    .herbivore_births
                    .saturating_sub(snap.births);
                let total_deaths =
                    self.diagnostics.herbivore_age_deaths + self.diagnostics.herbivore_hp_deaths;
                snap.deaths_in_window = total_deaths.saturating_sub(snap.deaths);
                snap.decline_fraction = if snap.population > 0 {
                    ((snap.population as f64 - current as f64) / snap.population as f64)
                        .clamp(0., 1.)
                } else {
                    0.
                };
            }
            snap.tick = self.tick;
            snap.population = current;
            snap.births = self.diagnostics.herbivore_births;
            snap.deaths =
                self.diagnostics.herbivore_age_deaths + self.diagnostics.herbivore_hp_deaths;
        }
        let mut i = 0;
        while i < self.carnivores.len() {
            let (x, y, vision, speed, reserve) = {
                let a = &self.carnivores[i];
                (a.x, a.y, a.vision, a.speed, a.reserve)
            };
            self.carnivores[i].age = self.carnivores[i].age.saturating_add(1);
            self.carnivores[i].cooldown = self.carnivores[i].cooldown.saturating_sub(1);
            self.carnivores[i].attack_cooldown =
                self.carnivores[i].attack_cooldown.saturating_sub(1);
            self.carnivores[i].satiety_ticks = self.carnivores[i].satiety_ticks.saturating_sub(1);
            // Phase 06.2.28: conservative reserve -> body nutrient repair.
            // Unlike herbivores, carnivore reproduction currently takes offspring body N
            // directly from the parent RESERVE; therefore this is an isolated diagnostic
            // experiment, not an assumed reproduction fix.
            if c.body_n_repair_per_tick > 0. {
                let body = self.carnivores[i].body;
                let available = self.resources.nutrient_balance(reserve).unwrap_or(0.);
                // Keep the nutrients required for one offspring in the reserve.
                // This prevents body maintenance from starving the existing reproduction path.
                // Mode 0.02 is the phase28 unprotected diagnostic; mode 0.021 is protected.
                let protected = c.body_n_repair_per_tick >= 0.021;
                let floor = if protected {
                    c.offspring_body_nutrients + c.offspring_reserve_nutrients
                } else {
                    0.
                };
                let transfer = c.body_n_repair_per_tick.min((available - floor).max(0.));
                if transfer > 0. {
                    self.resources
                        .transfer_nutrients(reserve, body, transfer)
                        .map_err(|e| format!("carnivore body N repair: {e:?}"))?;
                }
            }
            let observe = self
                .behavior_trace_window
                .is_some_and(|(start, end)| (start..=end).contains(&self.tick));
            let feeding_before = self.carnivores[i].feeding_active;
            let nutrients_before = self.carnivore_debug.hunt_nutrients;
            let energy_before_bite = self.carnivore_debug.hunt_energy;
            if c.digestion_enabled {
                let stomach = self.carnivores[i].stomach;
                let reserve_e = self.resources.energy_balance(reserve).unwrap_or(0.);
                let stomach_e = self.resources.energy_balance(stomach).unwrap_or(0.);
                if reserve_e < c.emergency_reserve_energy && stomach_e > 1e-9 {
                    self.carnivore_debug.predator_low_reserve_with_stomach_ticks += 1;
                }
                let emergency = c.adaptive_digestion_enabled
                    && reserve_e < c.emergency_reserve_energy
                    && stomach_e > 1e-9;
                if emergency {
                    self.carnivore_debug.emergency_digestion_ticks += 1;
                }
                let digest_rate = if emergency {
                    c.digestion_energy_per_tick * c.emergency_digestion_multiplier
                } else {
                    c.digestion_energy_per_tick
                };
                let digest_e = digest_rate.min(stomach_e);
                let digest_n = c
                    .digestion_nutrients_per_tick
                    .min(self.resources.nutrient_balance(stomach).unwrap_or(0.));
                self.resources
                    .transfer_energy(stomach, reserve, digest_e)
                    .map_err(|e| format!("carnivore digestion E: {e:?}"))?;
                self.resources
                    .transfer_nutrients(stomach, reserve, digest_n)
                    .map_err(|e| format!("carnivore digestion N: {e:?}"))?;
                self.carnivores[i].p42.digestion_received += digest_e;
                self.carnivore_debug.stomach_energy_digested += digest_e;
                if emergency {
                    self.carnivore_debug.emergency_digestion_energy += digest_e;
                }
                self.carnivore_debug.stomach_nutrients_digested += digest_n;
                let fullness = (self.resources.energy_balance(stomach).unwrap_or(0.)
                    / c.stomach_capacity_energy)
                    .max(
                        self.resources.nutrient_balance(stomach).unwrap_or(0.)
                            / c.stomach_capacity_nutrients,
                    );
                if self.carnivores[i].feeding_active && fullness >= c.feeding_stop_fraction {
                    self.carnivores[i].feeding_active = false;
                } else if !self.carnivores[i].feeding_active
                    && fullness <= c.feeding_resume_fraction
                {
                    self.carnivores[i].feeding_active = true;
                }
            }
            let resumed = !feeding_before && self.carnivores[i].feeding_active;
            // Optional energy-aware hunting. Never bypass the physical stomach capacity.
            // Mode 0 preserves the phase06.2.11 behavior and RNG sequence.
            let stomach = self.carnivores[i].stomach;
            let reserve_e = self.resources.energy_balance(reserve).unwrap_or(0.);
            let stomach_e = self.resources.energy_balance(stomach).unwrap_or(0.);
            let stomach_n = self.resources.nutrient_balance(stomach).unwrap_or(0.);
            let stomach_has_capacity = stomach_e < c.stomach_capacity_energy - 1e-9
                && stomach_n < c.stomach_capacity_nutrients - 1e-9;
            let predicted_need = (c.basal_energy_per_tick
                + c.movement_energy_per_unit * speed as f64)
                * c.hunting_forecast_ticks as f64;
            let hunger_override = c.digestion_enabled
                && !self.carnivores[i].feeding_active
                && c.hunting_hunger_mode != 0
                && (if c.hunting_hunger_mode == 1 {
                    reserve_e < c.hunting_low_reserve_energy
                } else {
                    reserve_e + stomach_e < predicted_need
                });
            if hunger_override {
                self.carnivore_debug.hunting_hunger_override_ticks += 1;
            }
            if hunger_override && !stomach_has_capacity {
                self.carnivore_debug
                    .hunting_hunger_override_no_capacity_ticks += 1;
            }
            let satiated = if c.digestion_enabled {
                !self.carnivores[i].feeding_active && !(hunger_override && stomach_has_capacity)
            } else {
                c.satiety_enabled && self.carnivores[i].satiety_ticks > 0
            };
            if c.digestion_enabled && satiated {
                self.carnivore_debug.stomach_feeding_blocked_ticks += 1;
            }
            if satiated {
                self.carnivore_debug.satiated_predator_ticks += 1;
                self.carnivore_debug.phase17_rest_ticks += 1;
            }
            let mut nearest = None;
            let mut dist = vision * vision;
            let predator_id = self.carnivores[i].id;
            let mut color_skips = 0;
            let mut last_color_skip = None;
            let mut selection_color_skips = Vec::new();
            let observe_opportunities = self.observes_carnivore_opportunities();
            for (j, h) in self.animals.items.iter().enumerate() {
                if satiated {
                    break;
                }
                let d = (h.x - x).powi(2) + (h.y - y).powi(2);
                if d < dist {
                    if !self.carnivore_allows_color(predator_id, h.genes.body_color, reserve_e) {
                        color_skips += 1;
                        last_color_skip = Some(h.id);
                        if observe_opportunities {
                            selection_color_skips.push([h.id.index, h.id.generation]);
                        }
                        continue;
                    }
                    dist = d;
                    nearest = Some(j);
                }
            }

            let opportunity =
                self.carnivore_opportunity_snapshot(i, nearest, satiated, hunger_override);
            let opportunity_n_before = self.carnivore_debug.hunt_nutrients;
            // Phase 39: count prey within twice the vision radius even when no target is acquired.
            // These are diagnostic samples, not alternate target selection.
            let nearby_prey = self.animals.items.iter().any(|h| {
                let d = (h.x - x).powi(2) + (h.y - y).powi(2);
                d < (vision * 2.).powi(2)
            });
            if nearest.is_none() && nearby_prey && !satiated {
                self.carnivore_debug.prey_near_but_outside_vision += 1;
            }
            if nearest.is_some() && reserve_e < 3. {
                self.carnivore_debug.prey_visible_while_low_energy += 1;
            }
            if nearby_prey && satiated {
                self.carnivore_debug.prey_near_while_satiated += 1;
            }
            if nearest.is_none() && !satiated && reserve_e < 3. {
                self.carnivore_debug.predator_low_energy_no_visible_prey += 1;
            }
            // Phase 43: compare the same prey at consecutive predator updates. Herbivores
            // have already moved this tick, so unlike p41 this includes prey motion.
            // A target switch starts a new comparison and is NOT counted as retreat.
            let target_observation = nearest.map(|j| (self.animals.items[j].id, dist.sqrt()));
            {
                let trace = &mut self.carnivores[i].p42;
                if let Some((target_id, current_distance)) = target_observation {
                    if trace.previous_target == Some(target_id) {
                        let delta = current_distance - trace.previous_target_distance;
                        if delta > 1e-5 {
                            trace.relative_farther += 1;
                        } else if delta < -1e-5 {
                            trace.relative_closer += 1;
                        } else {
                            trace.relative_unchanged += 1;
                        }
                    }
                    trace.previous_target = Some(target_id);
                    // The predator movement later in this tick is excluded; next observation
                    // therefore measures BOTH prey movement and the previous predator movement.
                    trace.previous_target_distance = current_distance;
                } else {
                    trace.previous_target = None;
                }
                if reserve_e < 3. && nearest.is_some() {
                    trace.low_energy_visible_ticks += 1;
                }
                if let Some(last) = trace.last_positive_attack_tick {
                    trace.longest_no_positive_attack_ticks = trace
                        .longest_no_positive_attack_ticks
                        .max(self.tick.saturating_sub(last));
                }
            }
            if nearest.is_some() {
                self.carnivore_debug.p41_visible_samples += 1;
                self.carnivores[i].p42.visible_ticks += 1;
            }
            let decision = Policy {
                fear: 0.,
                aggression: 1.,
                detection_radius: 1.,
                flee_threshold: 0.,
            }
            .choose(Perception {
                threat_distance: None,
                food_visible: nearest.is_some(),
                satiated,
                energy_fraction: (reserve_e / 60.).clamp(0., 1.) as f32,
            });
            debug_assert_eq!(decision == Action::Pursue, nearest.is_some());
            if nearest.is_some() {
                self.carnivore_debug.predator_target_ticks += 1;
                self.carnivore_debug.phase17_chase_ticks += 1;
            } else {
                self.carnivore_debug.predator_search_ticks += 1;
                if !satiated {
                    self.carnivore_debug.phase17_active_search_ticks += 1;
                }
            }
            // Phase 06.2.13: persistent exploration is opt-in. OFF retains the exact
            // legacy random draw pattern and movement for paired-seed comparison.
            let strategic_search = c.persistent_search_enabled
                || c.movement_strategy == 2
                || c.movement_strategy == 3
                || c.movement_strategy == 4;
            let angle = if !strategic_search {
                if let Some(j) = nearest {
                    (self.animals.items[j].y - y).atan2(self.animals.items[j].x - x)
                } else {
                    self.rng.unit() * std::f32::consts::TAU
                }
            } else if let Some(j) = nearest {
                let heading = (self.animals.items[j].y - y).atan2(self.animals.items[j].x - x);
                self.carnivores[i].last_prey_heading = heading;
                self.carnivores[i].last_prey_ticks_left = c.lost_prey_memory_ticks;
                self.carnivores[i].search_heading = heading;
                self.carnivores[i].search_ticks_left = c.search_heading_ticks;
                heading
            } else if satiated {
                // Resting does not consume search-memory time or RNG.
                self.carnivores[i].search_heading
            } else if self.carnivores[i].last_prey_ticks_left > 0 {
                self.carnivores[i].last_prey_ticks_left -= 1;
                self.carnivore_debug.remembered_prey_ticks += 1;
                self.carnivores[i].last_prey_heading
            } else {
                self.carnivore_debug.persistent_search_ticks += 1;
                if self.carnivores[i].search_ticks_left == 0 {
                    self.carnivores[i].search_heading = self.rng.unit() * std::f32::consts::TAU;
                    // Match the herbivore's 30..100-tick exploration interval.
                    self.carnivores[i].search_ticks_left = 30 + (self.rng.next_u64() % 71) as u32;
                    self.carnivore_debug.search_heading_changes += 1;
                }
                self.carnivores[i].search_ticks_left -= 1;
                self.carnivores[i].search_heading
            };
            // Herbivore-style movement: when there is no prey, follow the stored
            // heading rather than sampling a fresh direction on every tick.
            // Phase 32: energy-aware exploration; chasing visible prey is unchanged.
            // Ambush uses deterministic rest/move windows; hybrid rests only when reserve E is low.
            let resting_without_prey = if nearest.is_none() && !satiated {
                match c.movement_strategy {
                    4 => {
                        // Deterministic per-individual ambush schedule. No extra RNG draws.
                        let gene = self.carnivores[i].ambush_gene.clamp(0., 1.);
                        let period =
                            (c.ambush_rest_ticks as u64 + c.ambush_move_ticks as u64).max(1);
                        let rest = (gene * period as f32).round() as u64;
                        ((self.tick + self.carnivores[i].id.index as u64) % period) < rest
                    }
                    1 => {
                        (self.tick % (c.ambush_rest_ticks as u64 + c.ambush_move_ticks as u64))
                            < c.ambush_rest_ticks as u64
                    }
                    3 => {
                        reserve_e < c.hybrid_low_energy
                            && (self.tick
                                % (c.ambush_rest_ticks as u64 + c.ambush_move_ticks as u64))
                                < c.ambush_rest_ticks as u64
                    }
                    _ => false,
                }
            } else {
                false
            };
            if nearest.is_none() && !satiated {
                let bin = ambush_bin(self.carnivores[i].ambush_gene);
                if resting_without_prey {
                    self.carnivore_debug.gene_ambush_choices[bin] += 1;
                } else {
                    self.carnivore_debug.gene_search_choices[bin] += 1;
                }
            }
            if resting_without_prey && c.movement_strategy == 1 {
                self.carnivore_debug.strategy_ambush_rest_ticks += 1;
            }
            if resting_without_prey && c.movement_strategy == 3 {
                self.carnivore_debug.strategy_hybrid_rest_ticks += 1;
            }
            if !resting_without_prey && nearest.is_none() && !satiated && c.movement_strategy == 2 {
                self.carnivore_debug.strategy_roaming_ticks += 1;
            }
            let distance = if satiated || resting_without_prey {
                0.
            } else if nearest.is_some() {
                dist.sqrt().min(speed)
            } else {
                speed
            };
            if self
                .behavior_trace_window
                .is_some_and(|(start, end)| (start..=end).contains(&self.tick))
            {
                let a = &self.carnivores[i];
                self.focal_trace.push(serde_json::json!({
                    "phase":"predator_behavior", "tick":self.tick,
                    "id":[a.id.index,a.id.generation], "position_before":[x,y],
                    "reserve_e":reserve_e,"stomach_e":stomach_e,"stomach_n":stomach_n,
                    "feeding_active":a.feeding_active, "satiated":satiated,
                    "hunger_override":hunger_override,"stomach_has_capacity":stomach_has_capacity,
                    "predicted_need":predicted_need,"resting_without_prey":resting_without_prey,
                    "decision":format!("{decision:?}"),"angle":angle,"distance":distance,
                    "prey":nearest.map(|j| {
                        let prey = &self.animals.items[j];
                        serde_json::json!({"id":[prey.id.index,prey.id.generation],
                            "x":prey.x,"y":prey.y,"hp":prey.hp,
                            "resistance":prey.genes.toxin_resistance})
                    }),
                }));
            }
            let mut nx = (x + angle.cos() * distance).clamp(0., self.config.world.width);
            let mut ny = (y + angle.sin() * distance).clamp(0., self.config.world.height);
            // Reflect the exploration direction at the world boundary rather than
            // repeatedly pushing into the wall. Recompute this tick's displacement.
            if strategic_search && !satiated && nearest.is_none() && distance > 0. {
                let wall_x = (angle.cos() > 0.
                    && x + angle.cos() * distance >= self.config.world.width)
                    || (angle.cos() < 0. && x + angle.cos() * distance <= 0.);
                let wall_y = (angle.sin() > 0.
                    && y + angle.sin() * distance >= self.config.world.height)
                    || (angle.sin() < 0. && y + angle.sin() * distance <= 0.);
                if wall_x || wall_y {
                    let mut vx = angle.cos();
                    let mut vy = angle.sin();
                    if wall_x {
                        vx = -vx;
                    }
                    if wall_y {
                        vy = -vy;
                    }
                    let reflected = vy.atan2(vx);
                    self.carnivores[i].search_heading = reflected;
                    self.carnivores[i].last_prey_heading = reflected;
                    self.carnivores[i].last_prey_ticks_left = 0;
                    // Keep the current exploration duration after bouncing, as herbivores do.
                    self.carnivore_debug.search_wall_turns += 1;
                    nx = (x + vx * distance).clamp(0., self.config.world.width);
                    ny = (y + vy * distance).clamp(0., self.config.world.height);
                }
            }
            // Phase 41: compare distance to the same selected prey before/after this predator move.
            // Prey may move later in its own tick; this measures predator-side approach only.
            if let Some(j) = nearest {
                let h = &self.animals.items[j];
                let after = ((h.x - nx).powi(2) + (h.y - ny).powi(2)).sqrt();
                let before = dist.sqrt();
                if after + 1e-5 < before {
                    self.carnivore_debug.p41_approach_samples += 1;
                    self.carnivores[i].p42.approach_ticks += 1;
                } else if after > before + 1e-5 {
                    self.carnivore_debug.p41_retreat_samples += 1;
                    self.carnivores[i].p42.retreat_ticks += 1;
                }
                if after <= c.attack_range {
                    self.carnivore_debug.p41_attack_reach_samples += 1;
                    self.carnivores[i].p42.attack_range_ticks += 1;
                }
            }
            let moved = ((nx - x).powi(2) + (ny - y).powi(2)).sqrt();
            self.carnivore_debug.predator_movement_distance += moved as f64;
            if nearest.is_some()
                && (self.animals.items[nearest.unwrap()].x - nx).powi(2)
                    + (self.animals.items[nearest.unwrap()].y - ny).powi(2)
                    > c.attack_range * c.attack_range
            {
                self.carnivore_debug.prey_visible_but_out_of_attack_range += 1;
            }
            self.carnivores[i].x = nx;
            self.carnivores[i].y = ny;
            let available = self.resources.energy_balance(reserve).unwrap_or(0.);
            let basal = (c.basal_energy_per_tick
                * crate::animals::speed_upkeep_factor(c.speed_upkeep_slope, speed, c.speed))
            .min(available);
            let movement =
                (c.movement_energy_per_unit * moved as f64).min((available - basal).max(0.));
            // basal + (available - basal) can exceed `available` by one rounding step.
            // Clamping only changes the result in that case, which previously aborted the run.
            let burn = (basal + movement).min(available);
            if nearest.is_some() {
                self.carnivore_debug.p41_chase_basal_energy += basal;
                self.carnivore_debug.p41_chase_movement_energy += movement;
            }
            self.carnivores[i].p42.basal_spent += basal;
            self.carnivores[i].p42.movement_spent += movement;
            self.carnivore_debug.energy_basal_spent += basal;
            self.carnivore_debug.energy_movement_spent += movement;
            self.ledger
                .dissipate_heat(&mut self.resources, reserve, burn)
                .map_err(|e| format!("carnivore metabolism: {e:?}"))?;
            let attacks_before = self.carnivore_debug.hunt_attempts;
            let mut attack_color_rejected = false;
            let attack_reserve_e = opportunity
                .as_ref()
                .map(|_| self.resources.energy_balance(reserve).unwrap_or(0.));
            if let Some(j) = nearest {
                let prey = &self.animals.items[j];
                let learned_target = self.config.animal_toxin.color_learning_enabled
                    && self.observe_carnivore_color_target(
                        predator_id,
                        prey.id,
                        prey.genes.body_color,
                        reserve_e,
                        moved,
                    );
                let h = &self.animals.items[j];
                if (h.x - nx).powi(2) + (h.y - ny).powi(2) <= c.attack_range * c.attack_range {
                    self.carnivore_debug.prey_in_attack_range += 1;
                    if !self.carnivore_allows_color(
                        predator_id,
                        h.genes.body_color,
                        self.resources.energy_balance(reserve).unwrap_or(0.),
                    ) {
                        color_skips += 1;
                        last_color_skip = Some(h.id);
                        attack_color_rejected = true;
                    } else if c.prey_refuge_enabled
                        && self.animals.items.len() <= c.prey_refuge_threshold as usize
                    {
                        self.carnivore_debug.hunt_refuge_blocked += 1;
                    } else if self.carnivores[i].attack_cooldown > 0 {
                        self.carnivore_debug.hunt_cooldown_blocked += 1;
                    } else if c.all_bites_to_stomach
                        && (self
                            .resources
                            .energy_balance(self.carnivores[i].stomach)
                            .unwrap_or(0.)
                            >= c.stomach_capacity_energy - 1e-9
                            || self
                                .resources
                                .nutrient_balance(self.carnivores[i].stomach)
                                .unwrap_or(0.)
                                >= c.stomach_capacity_nutrients - 1e-9)
                    {
                        self.carnivores[i].feeding_active = false;
                        self.carnivore_debug.bite_stomach_capacity_blocked += 1;
                    } else {
                        self.carnivores[i].p42.attacks += 1;
                        self.carnivore_debug.hunt_attempts += 1;
                        if hunger_override {
                            self.carnivore_debug.hunting_hunger_override_attacks += 1;
                        }
                        self.carnivores[i].attack_cooldown = c.attack_cooldown_ticks;
                        let source = h.body;
                        let prey_energy = self.animals.items[j].reserve;
                        let stomach = self.carnivores[i].stomach;
                        let bite_target = if c.all_bites_to_stomach {
                            stomach
                        } else {
                            reserve
                        };
                        let free_n = if c.all_bites_to_stomach {
                            (c.stomach_capacity_nutrients
                                - self.resources.nutrient_balance(stomach).unwrap_or(0.))
                            .max(0.)
                        } else {
                            f64::INFINITY
                        };
                        let free_e = if c.all_bites_to_stomach {
                            (c.stomach_capacity_energy
                                - self.resources.energy_balance(stomach).unwrap_or(0.))
                            .max(0.)
                        } else {
                            f64::INFINITY
                        };
                        let body_n = self.resources.nutrient_balance(source).unwrap_or(0.);
                        let n = c.bite_nutrients.min(free_n).min(body_n);
                        let e = c
                            .bite_energy
                            .min(free_e)
                            .min(self.resources.energy_balance(prey_energy).unwrap_or(0.));
                        if learned_target {
                            self.observe_carnivore_learned_attack(n > 0. || e > 0.);
                        }
                        if c.all_bites_to_stomach
                            && (n + 1e-9 < c.bite_nutrients || e + 1e-9 < c.bite_energy)
                        {
                            self.carnivore_debug.bite_stomach_capacity_blocked += 1;
                        }
                        self.resources
                            .transfer_nutrients(source, bite_target, n)
                            .map_err(|err| format!("carnivore bite N: {err:?}"))?;
                        let prey_before = self.resources.energy_balance(prey_energy).unwrap_or(0.);
                        self.resources
                            .transfer_energy(prey_energy, bite_target, e)
                            .map_err(|err| format!("carnivore bite E: {err:?}"))?;
                        if self.config.animal_toxin.enabled {
                            self.transfer_animal_toxin(j, i, body_n, n);
                        }
                        self.record_energy(
                            self.animals.items[j].id,
                            "predation_bite",
                            prey_before,
                            self.resources.energy_balance(prey_energy).unwrap_or(0.),
                            -e,
                        );
                        if c.all_bites_to_stomach {
                            self.carnivore_debug.bite_stomach_energy += e;
                            self.carnivore_debug.bite_stomach_nutrients += n;
                            self.carnivore_debug.stomach_energy_ingested += e;
                            self.carnivore_debug.stomach_nutrients_ingested += n;
                            let fullness = (self.resources.energy_balance(stomach).unwrap_or(0.)
                                / c.stomach_capacity_energy)
                                .max(
                                    self.resources.nutrient_balance(stomach).unwrap_or(0.)
                                        / c.stomach_capacity_nutrients,
                                );
                            if fullness >= c.feeding_stop_fraction {
                                self.carnivores[i].feeding_active = false;
                            }
                        }
                        if e > 1e-9 {
                            self.carnivores[i].p42.positive_energy_attacks += 1;
                            self.carnivores[i].p42.last_positive_attack_tick = Some(self.tick);
                            self.carnivore_debug.hunt_positive_energy += 1;
                        } else {
                            self.carnivore_debug.hunt_zero_energy += 1;
                        }
                        if e + 1e-9 < c.bite_energy {
                            self.carnivore_debug.attack_energy_limited += 1;
                        }
                        self.carnivores[i].p42.bite_energy_received += e;
                        self.carnivore_hunts += 1;
                        self.carnivore_debug.hunt_contacts += 1;
                        self.carnivore_debug.hunt_nutrients += n;
                        self.carnivore_debug.hunt_energy += e;
                        let hp_before = self.animals.items[j].hp;
                        // Standing ground is a defensive posture: it mitigates injury but does not create E/N.
                        // Bite nutrient and energy transfers remain unchanged in this initial experiment.
                        let damage = if self.animals.items[j].defense_active {
                            2.5
                        } else {
                            5.
                        };
                        self.animals.items[j].hp = (hp_before - damage).max(0.);
                        let h = &self.config.herbivores;
                        if h.counterattack_energy > 0. && self.animals.items[j].defense_active {
                            let genes = &self.animals.items[j].genes;
                            let requested = h.counterattack_energy
                                * f64::from(genes.aggression)
                                * f64::from(genes.adult_max_hp)
                                / 60.;
                            let available = self.resources.energy_balance(reserve).unwrap_or(0.);
                            let injury = requested.min(available);
                            if injury > 0. {
                                self.ledger
                                    .dissipate_heat(&mut self.resources, reserve, injury)
                                    .map_err(|e| format!("carnivore counterattack: {e:?}"))?;
                            }
                            self.defense.counterattacks += 1;
                            self.defense.counterattack_energy += injury;
                            if h.counterattack_learning_enabled
                                && self.config.animal_toxin.color_learning_enabled
                                && requested > 0.
                            {
                                self.learn_counterattack(i, j, requested / 60.);
                                self.defense.counterattack_learning_events += 1;
                            }
                        }
                        if let Some(trace) = &mut self.health_trace {
                            if trace.watched.contains(&self.animals.items[j].id) {
                                trace.record(self.tick,&self.animals.items[j],&self.resources,"predation",Some(hp_before),
                                    serde_json::json!({"damage":damage,"predator":[self.carnivores[i].id.index,self.carnivores[i].id.generation],
                                        "nutrients_taken":n,"energy_taken":e,"defense_active":self.animals.items[j].defense_active}));
                            }
                        }
                        if self.focal_trace_id == Some(self.animals.items[j].id) {
                            self.focal_trace.push(serde_json::json!({
                                "phase": "attack", "tick": self.tick,
                                "predator": [self.carnivores[i].id.index, self.carnivores[i].id.generation],
                                "hp_before": hp_before, "hp_after": self.animals.items[j].hp,
                                "damage": damage, "nutrients_taken": n, "energy_taken": e,
                                "defense_active": self.animals.items[j].defense_active,
                            }));
                        }
                        if hp_before > 0. && self.animals.items[j].hp <= 0. {
                            self.carnivore_debug.prey_hp_zero_after_attack += 1;
                            if c.satiety_enabled || c.digestion_enabled {
                                // A full meal occurs on a lethal attack, not on every bite. All resources
                                // come from the prey accounts; no energy or nutrients are created.
                                let stomach = self.carnivores[i].stomach;
                                let target = if c.digestion_enabled {
                                    stomach
                                } else {
                                    reserve
                                };
                                let free_e = if c.digestion_enabled {
                                    (c.stomach_capacity_energy
                                        - self.resources.energy_balance(stomach).unwrap_or(0.))
                                    .max(0.)
                                } else {
                                    f64::INFINITY
                                };
                                let free_n = if c.digestion_enabled {
                                    (c.stomach_capacity_nutrients
                                        - self.resources.nutrient_balance(stomach).unwrap_or(0.))
                                    .max(0.)
                                } else {
                                    f64::INFINITY
                                };
                                let extra_e = c
                                    .kill_bonus_energy
                                    .min(free_e)
                                    .min(self.resources.energy_balance(prey_energy).unwrap_or(0.));
                                let body_n = self.resources.nutrient_balance(source).unwrap_or(0.);
                                let extra_n = c.kill_bonus_nutrients.min(free_n).min(body_n);
                                let prey_before =
                                    self.resources.energy_balance(prey_energy).unwrap_or(0.);
                                self.resources
                                    .transfer_energy(prey_energy, target, extra_e)
                                    .map_err(|err| format!("carnivore meal E: {err:?}"))?;
                                self.record_energy(
                                    self.animals.items[j].id,
                                    "predation_lethal_meal",
                                    prey_before,
                                    self.resources.energy_balance(prey_energy).unwrap_or(0.),
                                    -extra_e,
                                );
                                self.resources
                                    .transfer_nutrients(source, target, extra_n)
                                    .map_err(|err| format!("carnivore meal N: {err:?}"))?;
                                if self.config.animal_toxin.enabled {
                                    self.transfer_animal_toxin(j, i, body_n, extra_n);
                                }
                                if c.digestion_enabled {
                                    self.carnivore_debug.stomach_energy_ingested += extra_e;
                                    self.carnivore_debug.stomach_nutrients_ingested += extra_n;
                                }
                                self.carnivore_debug.meal_bonus_energy += extra_e;
                                self.carnivore_debug.meal_bonus_nutrients += extra_n;
                                self.carnivores[i].p42.bite_energy_received += extra_e;
                                self.carnivore_debug.hunt_energy += extra_e;
                                self.carnivore_debug.hunt_nutrients += extra_n;
                                self.carnivore_debug.kills_with_meal += 1;
                                if c.digestion_enabled {
                                    let fullness = (self
                                        .resources
                                        .energy_balance(stomach)
                                        .unwrap_or(0.)
                                        / c.stomach_capacity_energy)
                                        .max(
                                            self.resources.nutrient_balance(stomach).unwrap_or(0.)
                                                / c.stomach_capacity_nutrients,
                                        );
                                    if fullness >= c.feeding_stop_fraction {
                                        self.carnivores[i].feeding_active = false;
                                    }
                                } else {
                                    self.carnivores[i].satiety_ticks = c.satiety_ticks_on_kill;
                                }
                            }
                        }
                    }
                }
            }
            if let Some(mut row) = opportunity {
                row["selection_color_skips"] = serde_json::json!(selection_color_skips);
                row["attack_color_rejected"] = serde_json::json!(attack_color_rejected);
                row["attack_reserve_e"] = serde_json::json!(attack_reserve_e);
                row["attacked"] =
                    serde_json::json!(self.carnivore_debug.hunt_attempts != attacks_before);
                row["eaten_n"] =
                    serde_json::json!(self.carnivore_debug.hunt_nutrients - opportunity_n_before);
                row["moved"] = serde_json::json!(moved);
                row["distance_after"] = serde_json::json!(nearest.map(|j| {
                    let p = &self.animals.items[j];
                    (p.x - nx).hypot(p.y - ny)
                }));
                self.record_carnivore_opportunity(row);
            }
            if let Some(prey) = last_color_skip {
                self.record_carnivore_color_skips(predator_id, prey, color_skips);
            }
            if self.config.animal_toxin.enabled {
                self.trace_carnivore_toxin_decision(
                    predator_id,
                    nearest.map(|j| self.animals.items[j].id),
                    last_color_skip,
                    reserve_e,
                    satiated,
                    self.carnivore_debug.hunt_attempts != attacks_before,
                );
            }
            let toxin_dead = self.config.animal_toxin.enabled && self.advance_carnivore_toxin(i)?;
            // Resource-conservative asexual reproduction. No food or energy created.
            let ready = !toxin_dead && {
                let a = &self.carnivores[i];
                c.reproduction_enabled
                    && a.cooldown == 0
                    && a.age > c.reproduction_cooldown_ticks
                    && self.resources.nutrient_balance(a.reserve).unwrap_or(0.)
                        >= c.offspring_body_nutrients + c.offspring_reserve_nutrients
                    && self.resources.nutrient_balance(a.reserve).unwrap_or(0.)
                        >= c.offspring_reserve_nutrients
                    && self.resources.energy_balance(a.reserve).unwrap_or(0.)
                        >= required_birth_energy
                    && self.plants.len() + self.animals.items.len() + self.carnivores.len()
                        < self.config.world.max_living_entities as usize
            };
            // Phase 06.2.17: diagnostic-only independent eligibility stages.
            // Counts are per predator-tick; they do not change RNG or reproduction.
            if c.reproduction_enabled {
                let a = &self.carnivores[i];
                if a.cooldown > 0 && a.age > c.reproduction_cooldown_ticks {
                    self.carnivore_debug.phase17_repro_cooldown_only_ticks += 1;
                }
                if a.cooldown == 0 && a.age <= c.reproduction_cooldown_ticks {
                    self.carnivore_debug.phase17_repro_age_only_ticks += 1;
                }
                if self.resources.nutrient_balance(a.reserve).unwrap_or(0.)
                    >= c.offspring_body_nutrients + c.offspring_reserve_nutrients
                    && self.resources.energy_balance(a.reserve).unwrap_or(0.)
                        >= required_birth_energy
                {
                    self.carnivore_debug.phase17_repro_resource_ready_ticks += 1;
                }
            }
            if !c.reproduction_enabled {
                self.carnivore_debug.reproduction_disabled_ticks += 1;
            }
            if ready {
                self.carnivore_debug.reproduction_physical_ready_ticks += 1;
            }
            if c.reproduction_enabled {
                let a = &self.carnivores[i];
                if a.cooldown > 0 {
                    self.carnivore_debug.reproduction_cooldown_blocked += 1;
                } else if a.age <= c.reproduction_cooldown_ticks {
                    self.carnivore_debug.reproduction_age_blocked += 1;
                } else if self.resources.nutrient_balance(a.reserve).unwrap_or(0.)
                    < c.offspring_body_nutrients + c.offspring_reserve_nutrients
                {
                    self.carnivore_debug.reproduction_body_n_blocked += 1;
                } else if self.resources.nutrient_balance(a.reserve).unwrap_or(0.)
                    < c.offspring_reserve_nutrients
                {
                    self.carnivore_debug.reproduction_reserve_n_blocked += 1;
                } else if self.resources.energy_balance(a.reserve).unwrap_or(0.)
                    < required_birth_energy
                {
                    self.carnivore_debug.reproduction_energy_blocked += 1;
                } else if self.plants.len() + self.animals.items.len() + self.carnivores.len()
                    >= self.config.world.max_living_entities as usize
                {
                    self.carnivore_debug.reproduction_capacity_blocked += 1;
                } else {
                    self.carnivore_debug.reproduction_ready += 1;
                }
            }
            // Density-dependent birth probability. No extra resource creation, and
            // no artificial guarantee that either species survives. Only consume RNG
            // when the option is ON and all physical reproduction conditions pass.
            let food_allows_birth = if ready && c.food_limited_reproduction_enabled {
                let prey = self.animals.items.len() as f64;
                let predator = self.carnivores.len() as f64;
                let probability = prey / (prey + predator * c.prey_per_predator_target);
                if (self.rng.unit() as f64) < probability {
                    true
                } else {
                    self.carnivore_debug.reproduction_food_blocked += 1;
                    false
                }
            } else {
                true
            };
            // Falling prey populations lower reproduction success, without a hard birth ban.
            // Sample RNG only after all earlier gates pass; OFF retains old RNG sequence.
            let trend_allows_birth =
                if ready && food_allows_birth && c.prey_trend_reproduction_enabled {
                    let snap = &self.prey_trend_snapshot;
                    let demographic_pressure = if snap.deaths_in_window > snap.births_in_window {
                        (snap.deaths_in_window - snap.births_in_window) as f64
                            / snap.deaths_in_window.max(1) as f64
                    } else {
                        0.
                    };
                    let pressure = snap.decline_fraction.max(demographic_pressure);
                    let probability = (1. - c.prey_decline_penalty * pressure).clamp(0., 1.);
                    if (self.rng.unit() as f64) < probability {
                        true
                    } else {
                        self.carnivore_debug.reproduction_trend_blocked += 1;
                        false
                    }
                } else {
                    true
                };
            let stomach_allows_birth = if ready && c.digestion_enabled {
                let a = &self.carnivores[i];
                let fullness = (self.resources.energy_balance(a.stomach).unwrap_or(0.)
                    / c.stomach_capacity_energy)
                    .max(
                        self.resources.nutrient_balance(a.stomach).unwrap_or(0.)
                            / c.stomach_capacity_nutrients,
                    );
                if fullness > c.reproduction_stomach_max_fraction {
                    self.carnivore_debug.stomach_reproduction_blocked += 1;
                    false
                } else {
                    true
                }
            } else {
                true
            };
            let sustainable_birth = if ready && c.sustainable_reproduction_enabled {
                let predators = self.carnivores.len().max(1) as f64;
                let prey = self.animals.items.len() as f64;
                if prey / predators < c.minimum_prey_per_predator {
                    self.carnivore_debug.sustainable_reproduction_blocked += 1;
                    false
                } else {
                    true
                }
            } else {
                true
            };
            if observe {
                let a = &self.carnivores[i];
                let reserve_e = self.resources.energy_balance(a.reserve).unwrap_or(0.);
                let reserve_n = self.resources.nutrient_balance(a.reserve).unwrap_or(0.);
                let stomach_e = self.resources.energy_balance(a.stomach).unwrap_or(0.);
                let stomach_n = self.resources.nutrient_balance(a.stomach).unwrap_or(0.);
                self.focal_trace.push(serde_json::json!({
                    "phase":"predator_reproduction","tick":self.tick, "id":[a.id.index,a.id.generation],
                    "parent":a.parent_id.map(|id| [id.index,id.generation]),
                    "age":a.age,"cooldown":a.cooldown,"attack_cd":a.attack_cooldown,
                    "reserve_e":reserve_e,"reserve_n":reserve_n,"stomach_e":stomach_e,"stomach_n":stomach_n,
                    "fullness":(stomach_e/c.stomach_capacity_energy).max(stomach_n/c.stomach_capacity_nutrients),
                    "population":self.plants.len()+self.animals.items.len()+self.carnivores.len(),
                    "resumed":resumed,"bite_e":self.carnivore_debug.hunt_energy-energy_before_bite,"feeding":a.feeding_active,"bite_n":self.carnivore_debug.hunt_nutrients-nutrients_before,
                    "fail":[!c.reproduction_enabled,toxin_dead,a.cooldown!=0,a.age<=c.reproduction_cooldown_ticks,
                        reserve_n<c.offspring_body_nutrients+c.offspring_reserve_nutrients,
                        reserve_e<required_birth_energy,
                        self.plants.len()+self.animals.items.len()+self.carnivores.len()>=self.config.world.max_living_entities as usize],
                    "ready":ready,"food":(ready && c.food_limited_reproduction_enabled).then_some(food_allows_birth),
                    "trend":(ready && food_allows_birth && c.prey_trend_reproduction_enabled).then_some(trend_allows_birth),
                    "stomach":(ready && c.digestion_enabled).then_some(stomach_allows_birth),
                    "sustainable":(ready && c.sustainable_reproduction_enabled).then_some(sustainable_birth)
                }));
            }
            if ready
                && food_allows_birth
                && trend_allows_birth
                && stomach_allows_birth
                && sustainable_birth
            {
                self.carnivore_debug.reproduction_all_gates_passed_ticks += 1;
                let parent = &self.carnivores[i];
                let reserve_src = parent.reserve;
                let (px, py, color, pspeed, pvision, parent_gene, parent_id) = (
                    parent.x,
                    parent.y,
                    parent.color,
                    parent.speed,
                    parent.vision,
                    parent.ambush_gene,
                    parent.id,
                );
                let id = self.entities.spawn()?;
                let body = self.resources.open(0., 0.).map_err(|e| format!("{e:?}"))?;
                let child_reserve = self.resources.open(0., 0.).map_err(|e| format!("{e:?}"))?;
                self.resources
                    .transfer_nutrients(reserve_src, body, c.offspring_body_nutrients)
                    .map_err(|e| format!("carnivore offspring N: {e:?}"))?;
                self.resources
                    .transfer_nutrients(reserve_src, child_reserve, c.offspring_reserve_nutrients)
                    .map_err(|e| format!("carnivore offspring reserve N: {e:?}"))?;
                self.resources
                    .transfer_energy(reserve_src, child_reserve, c.offspring_energy)
                    .map_err(|e| format!("carnivore offspring E: {e:?}"))?;
                self.carnivores[i].p42.reproduction_energy_spent +=
                    c.reproduction_work_energy + c.offspring_energy;
                self.carnivores[i].p42.births_as_parent += 1;
                self.carnivore_debug.energy_reproduction_spent += c.reproduction_work_energy;
                self.ledger
                    .dissipate_heat(&mut self.resources, reserve_src, c.reproduction_work_energy)
                    .map_err(|e| format!("carnivore reproduction heat: {e:?}"))?;
                let gene_mutated =
                    c.movement_strategy == 4 && self.rng.unit() < c.mutation_probability;
                if gene_mutated {
                    self.carnivore_debug.gene_mutation_trials += 1;
                }
                let child_gene = if gene_mutated {
                    (parent_gene + (self.rng.unit() * 2. - 1.) * c.mutation_scale).clamp(0., 1.)
                } else {
                    parent_gene
                };
                let child_color = crate::animals::color::inherit(
                    color,
                    self.config.seed ^ self.tick.rotate_left(13),
                    id.index as u64,
                    c.mutation_probability,
                    c.mutation_scale,
                );
                let child_speed = if c.speed_evolution_enabled {
                    crate::animals::carnivore::inherit_speed(
                        pspeed,
                        c.speed_mutation_probability,
                        c.speed_mutation_scale,
                        self.config.seed ^ self.tick.rotate_left(29),
                        id.index as u64,
                    )
                    .0
                } else {
                    pspeed
                };
                let stomach = self.resources.open(0., 0.).map_err(|e| format!("{e:?}"))?;
                if self.config.animal_toxin.enabled {
                    self.inherit_carnivore_toxin(parent_id, id);
                }
                self.carnivores.push(Carnivore {
                    id,
                    x: (px + 5.).min(self.config.world.width),
                    y: py,
                    age: 0,
                    max_age: c.max_age_ticks,
                    cooldown: c.reproduction_cooldown_ticks,
                    attack_cooldown: 0,
                    satiety_ticks: 0,
                    stomach,
                    feeding_active: true,
                    body,
                    reserve: child_reserve,
                    color: child_color,
                    speed: child_speed,
                    vision: pvision,
                    ambush_gene: child_gene,
                    p42: Default::default(),
                    parent_id: Some(parent_id),
                    search_heading: 0.,
                    search_ticks_left: 0,
                    last_prey_heading: 0.,
                    last_prey_ticks_left: 0,
                });
                self.carnivores[i].cooldown = c.reproduction_cooldown_ticks;
                let parent_bin = ambush_bin(parent_gene);
                let child_bin = ambush_bin(child_gene);
                self.carnivore_debug.gene_parent_reproductions[parent_bin] += 1;
                self.carnivore_debug.gene_births[child_bin] += 1;
                if (child_gene - parent_gene).abs() > 1e-7 {
                    self.carnivore_debug.gene_mutations[child_bin] += 1;
                }
                self.carnivore_births += 1;
            }
            let dead = {
                let a = &self.carnivores[i];
                toxin_dead
                    || a.age >= a.max_age
                    || self.resources.energy_balance(a.reserve).unwrap_or(0.) <= 1e-9
            };
            if dead {
                let death_event = self.p42_snapshot(&self.carnivores[i], "death");
                self.p42_life_events.push(death_event);
                // Phase 06.2.40: observational snapshot at death. No RNG, no changes to resources.
                let dying = &self.carnivores[i];
                let nearest_death_prey = self
                    .animals
                    .items
                    .iter()
                    .map(|h| ((h.x - dying.x).powi(2) + (h.y - dying.y).powi(2)).sqrt())
                    .reduce(f32::min);
                if let Some(distance) = nearest_death_prey {
                    if distance <= dying.vision {
                        self.carnivore_debug.p40_death_prey_visible += 1;
                    }
                    if distance <= dying.vision && dying.age < dying.max_age {
                        self.carnivore_debug.p41_death_after_visible_target += 1;
                    }
                    if distance <= c.attack_range {
                        self.carnivore_debug.p40_death_prey_attack_range += 1;
                    }
                    if distance <= dying.vision * 2. {
                        self.carnivore_debug.p40_death_nearby_prey_twice_vision += 1;
                    }
                } else {
                    self.carnivore_debug.p40_death_prey_absent += 1;
                }
                if !dying.feeding_active {
                    self.carnivore_debug.p40_death_feeding_paused += 1;
                }
                if dying.age < dying.max_age {
                    self.carnivore_debug.p40_death_energy_starvation += 1;
                }
                let death_stomach_e = self
                    .resources
                    .energy_balance(self.carnivores[i].stomach)
                    .unwrap_or(0.);
                let death_stomach_n = self
                    .resources
                    .nutrient_balance(self.carnivores[i].stomach)
                    .unwrap_or(0.);
                self.carnivore_debug.predator_death_stomach_energy += death_stomach_e;
                self.carnivore_debug.predator_death_stomach_nutrients += death_stomach_n;
                if death_stomach_e > 1e-9 {
                    self.carnivore_debug.predator_deaths_with_stomach_energy += 1;
                }
                if self.carnivores[i].age < self.carnivores[i].max_age && death_stomach_e > 1e-9 {
                    self.carnivore_debug.predator_starvation_with_stomach_energy += 1;
                }
                self.carnivore_debug.predator_death_energy_remaining += self
                    .resources
                    .energy_balance(self.carnivores[i].reserve)
                    .unwrap_or(0.);
                self.carnivore_debug.predator_death_age_total += self.carnivores[i].age as u64;
                if let Some(d) = self
                    .animals
                    .items
                    .iter()
                    .map(|h| {
                        ((h.x - self.carnivores[i].x).powi(2)
                            + (h.y - self.carnivores[i].y).powi(2))
                        .sqrt()
                    })
                    .reduce(f32::min)
                {
                    self.carnivore_debug
                        .predator_death_distance_to_nearest_prey_total += d as f64;
                    self.carnivore_debug.predator_death_nearest_prey_samples += 1;
                }
                if toxin_dead {
                    self.animal_toxins.toxin_deaths += 1;
                } else if self.carnivores[i].age >= self.carnivores[i].max_age {
                    self.carnivore_debug.carnivore_age_deaths += 1;
                } else {
                    self.carnivore_debug.carnivore_energy_deaths += 1;
                }
                let bin = ambush_bin(self.carnivores[i].ambush_gene);
                self.carnivore_debug.gene_deaths[bin] += 1;
                let a = self.carnivores.swap_remove(i);
                if self.config.animal_toxin.enabled {
                    let reason = if toxin_dead {
                        "toxin_hp_zero"
                    } else if a.age >= a.max_age {
                        "age"
                    } else {
                        "energy"
                    };
                    self.trace_carnivore_toxin_removal(a.id, reason);
                    self.animal_toxins.remove_carnivore(a.id);
                }
                let cell = self.land.cell_index(
                    a.x,
                    a.y,
                    self.config.world.width,
                    self.config.world.height,
                );
                let detritus = self.land.cells[cell].detritus;
                for account in [a.body, a.reserve, a.stomach] {
                    let n = self.resources.nutrient_balance(account).unwrap_or(0.);
                    let e = self.resources.energy_balance(account).unwrap_or(0.);
                    self.resources
                        .transfer_nutrients(account, detritus, n)
                        .map_err(|e| format!("carnivore death N: {e:?}"))?;
                    self.ledger
                        .dissipate_heat(&mut self.resources, account, e)
                        .map_err(|e| format!("carnivore death E: {e:?}"))?;
                }
                self.entities.despawn(a.id);
                self.carnivore_deaths += 1;
            } else {
                i += 1;
            }
        }
        Ok(())
    }
}
