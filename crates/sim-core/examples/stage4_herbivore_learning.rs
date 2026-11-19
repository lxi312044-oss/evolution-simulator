//! Fixed-window Stage 4 plant-color learning contrast with a read-only shadow.
use serde::Serialize;
use serde_json::{json, Value};
use sim_core::{
    world::{ForagingTrace, ForagingWindow},
    SimulationConfig, World,
};
use std::{
    collections::BTreeMap,
    collections::BTreeSet,
    time::{Duration, Instant},
};

#[derive(Default, Serialize)]
struct Choice {
    opportunities: u64,
    safe_selected: u64,
    no_target: u64,
    safe_rejected: u64,
    toxic_rejected: u64,
    safe_n: f64,
    toxic_n: f64,
}

impl Choice {
    fn record(&mut self, row: &Value) {
        self.opportunities += 1;
        self.safe_selected += u64::from(row["mixed"]["target_toxic"] == false);
        self.no_target += u64::from(row["mixed"]["target_toxic"].is_null());
        self.safe_rejected += row["mixed"]["rejected"][0].as_u64().unwrap();
        self.toxic_rejected += row["mixed"]["rejected"][1].as_u64().unwrap();
        let n = row["feeding"]["eaten_n"].as_f64().unwrap();
        match row["mixed"]["target_toxic"].as_bool() {
            Some(true) => self.toxic_n += n,
            Some(false) => self.safe_n += n,
            None => assert_eq!(n, 0.),
        }
    }
}

#[derive(Default, Serialize)]
struct Founder {
    ticks: u64,
    choice_visible: Choice,
    choice_in_range: Choice,
    gained_e: f64,
    color_excluded: u64,
    first_color_excluded: Option<u64>,
    feed_ticks: u64,
    first_feed: Option<u64>,
    eaten_n: f64,
    absorbed_n: f64,
    feed_streak: u64,
    max_feed_streak: u64,
    stomach_histogram: [u64; 11],
    max_stomach_n: f64,
    first_n10: Option<u64>,
    first_streak60: Option<u64>,
    max_full_streak: u64,
    hp_sum: f64,
    reserve_e_sum: f64,
    dose: f64,
    damage: f64,
    prevented_damage: f64,
    upkeep: f64,
    low_e_all_color_rejected: u64,
    recheck_ticks: u64,
    recheck_fed: u64,
    recheck_positive_net_e: u64,
    recheck_recovered_25: u64,
    recheck_gained_e: f64,
    recheck_net_e: f64,
    choice_e_histogram_5pct: [u64; 21],
    choice_e_fraction_sum: f64,
    recovery_starts: u64,
    recovery_releases: u64,
    releases_at_30: u64,
    durations_completed: Vec<u64>,
    durations_removed: Vec<u64>,
    durations_censored: Vec<u64>,
    return_below_25: Vec<u64>,
    return_removed: Vec<u64>,
    return_censored: Vec<u64>,
    #[serde(skip)]
    recovery_start: Option<u64>,
    #[serde(skip)]
    release_wait: Option<u64>,
    removal_tick: Option<u64>,
    removal_reason: Option<&'static str>,
}

impl Founder {
    fn observe(&mut self, row: &Value, hysteresis: bool) {
        self.ticks += 1;
        self.gained_e += row["feeding"]["gained_e"].as_f64().unwrap();
        if row["may_feed"] == true
            && row["mixed"]["visible"]
                .as_array()
                .is_some_and(|types| types.iter().all(|n| n.as_u64().unwrap() > 0))
        {
            self.choice_visible.record(row);
            if row["stomach_n_before_feeding"].as_f64().unwrap() < 20.
                && row["mixed"]["in_range"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|n| n.as_u64().unwrap() > 0)
            {
                self.choice_in_range.record(row);
            }
        }
        let tick = row["tick"].as_u64().unwrap();
        let visible_n = row["visible_n"].as_u64().unwrap();
        if row["may_feed"] == true
            && visible_n > 0
            && row["color_rejected_visible_n"] == visible_n
            && row["target"].is_null()
        {
            self.color_excluded += 1;
            self.first_color_excluded.get_or_insert(tick);
        }
        let fraction = row["reserve_fraction_at_choice"].as_f64().unwrap();
        self.choice_e_fraction_sum += fraction;
        self.choice_e_histogram_5pct[((fraction * 20.) as usize).min(20)] += 1;
        let low_e = fraction < 0.25;
        if low_e {
            if let Some(release) = self.release_wait.take() {
                self.return_below_25.push(tick - release);
            }
        }
        let active = if hysteresis {
            row["recovery_active"] == true
        } else {
            row["hungry_color_recheck"] == true
        };
        if active && self.recovery_start.is_none() {
            self.recovery_starts += 1;
            self.recovery_start = Some(tick);
        } else if !active {
            if let Some(start) = self.recovery_start.take() {
                assert!(!hysteresis || row["recovery_released"] == true);
                self.recovery_releases += 1;
                self.releases_at_30 += u64::from(fraction >= 0.30);
                self.durations_completed.push(tick - start);
                if low_e {
                    self.return_below_25.push(0);
                } else {
                    self.release_wait = Some(tick);
                }
            }
        }
        if low_e
            && row["may_feed"] == true
            && visible_n > 0
            && row["color_rejected_visible_n"] == visible_n
        {
            self.low_e_all_color_rejected += 1;
        }
        let eaten = row["feeding"]["eaten_n"].as_f64().unwrap();
        if row["hungry_color_recheck"] == true {
            self.recheck_ticks += 1;
            if eaten > 0. {
                self.recheck_fed += 1;
                let gained = row["feeding"]["gained_e"].as_f64().unwrap();
                let before = row["reserve_e_before_tick"].as_f64().unwrap();
                let after = row["reserve_e_after_feeding"].as_f64().unwrap();
                self.recheck_gained_e += gained;
                self.recheck_net_e += after - before;
                self.recheck_positive_net_e += u64::from(after > before);
                let cutoff = 0.25 * row["max_reserve_e"].as_f64().unwrap();
                self.recheck_recovered_25 += u64::from(before < cutoff && after >= cutoff);
            }
        }
        if eaten > 0. {
            self.feed_ticks += 1;
            self.feed_streak += 1;
            self.max_feed_streak = self.max_feed_streak.max(self.feed_streak);
            self.first_feed.get_or_insert(tick);
            self.eaten_n += eaten;
            self.absorbed_n += row["feeding"]["absorbed_n"].as_f64().unwrap();
        } else {
            self.feed_streak = 0;
        }
        let n = row["stomach_n_before_feeding"].as_f64().unwrap();
        self.stomach_histogram[((n / 2.) as usize).min(10)] += 1;
        self.max_stomach_n = self.max_stomach_n.max(n);
        if n >= 10. {
            self.first_n10.get_or_insert(tick);
        }
        let streak = row["streak_after"].as_u64().unwrap();
        self.max_full_streak = self.max_full_streak.max(streak);
        if streak >= 60 {
            self.first_streak60.get_or_insert(tick);
        }
        self.hp_sum += row["hp_before_feeding"].as_f64().unwrap();
        self.reserve_e_sum += row["reserve_e_before_tick"].as_f64().unwrap();
    }
}

impl Founder {
    fn finish_recovery(&mut self, tick: u64, removed: bool) {
        if let Some(start) = self.recovery_start.take() {
            let durations = if removed {
                &mut self.durations_removed
            } else {
                &mut self.durations_censored
            };
            durations.push(tick - start + 1);
        }
        if let Some(release) = self.release_wait.take() {
            let returns = if removed {
                &mut self.return_removed
            } else {
                &mut self.return_censored
            };
            returns.push(tick - release);
        }
    }
}

fn audit(world: &mut World, shadow: &World) {
    let observer = world.foraging_trace.take();
    world.herbivore_terminal_logging = false;
    assert_eq!(
        world.audit_without_carnivore_trace(),
        shadow.audit_state(false, None)
    );
    world.herbivore_terminal_logging = true;
    world.foraging_trace = observer;
}

#[derive(Default, Serialize)]
struct InheritedChoice {
    ticks: u64,
    both_visible: u64,
    both_in_range: u64,
    no_plant_in_range: u64,
    all_visible_rejected: u64,
    both_visible_all_rejected: u64,
    both_in_range_all_rejected: u64,
    selected: [u64; 2],
    approached: [u64; 2],
    selected_in_range: [u64; 2],
    fed_visible: [u64; 4],
    fed_in_range: [u64; 4],
    remembered_ticks: u64,
    remembered_both_visible: u64,
    remembered_both_in_range: u64,
    remembered_selected: [u64; 2],
    remembered_fed: [u64; 4],
    remembered_all_rejected: u64,
    remembered_both_visible_all_rejected: u64,
    remembered_both_in_range_all_rejected: u64,
    remembered_individuals: BTreeSet<[u32; 2]>,
    remembered_feeding_individuals: BTreeSet<[u32; 2]>,
    remembered_excluded_individuals: BTreeSet<[u32; 2]>,
}

impl InheritedChoice {
    fn observe(&mut self, row: &Value, id: [u32; 2]) {
        self.ticks += 1;
        let data = &row["inherited_choice"];
        let visible = data["visible"].as_array().unwrap();
        let range = data["in_range"].as_array().unwrap();
        let both_visible = visible.iter().all(|n| n.as_u64().unwrap() > 0);
        let both_in_range = range.iter().all(|n| n.as_u64().unwrap() > 0);
        let may_feed = row["may_feed"] == true;
        let excluded = may_feed
            && visible.iter().any(|n| n.as_u64().unwrap() > 0)
            && data["visible"] == data["rejected"]
            && data["target"].is_null();
        let remembered = data["both_remembered"] == true;
        self.both_visible += u64::from(both_visible);
        self.both_in_range += u64::from(both_in_range);
        self.no_plant_in_range += u64::from(may_feed && range.iter().all(|n| n == &json!(0)));
        self.all_visible_rejected += u64::from(excluded);
        self.both_visible_all_rejected += u64::from(both_visible && excluded);
        self.both_in_range_all_rejected += u64::from(both_in_range && excluded);
        if remembered {
            self.remembered_ticks += 1;
            self.remembered_individuals.insert(id);
            self.remembered_both_visible += u64::from(both_visible);
            self.remembered_both_in_range += u64::from(both_in_range);
            self.remembered_all_rejected += u64::from(excluded);
            self.remembered_both_visible_all_rejected += u64::from(both_visible && excluded);
            self.remembered_both_in_range_all_rejected += u64::from(both_in_range && excluded);
            if excluded {
                self.remembered_excluded_individuals.insert(id);
            }
        }
        if let Some(color) = data["target"].as_u64() {
            let color = color as usize;
            self.selected[color] += u64::from(both_visible);
            self.approached[color] +=
                u64::from(both_visible && row["position_before"] != row["position_after"]);
            self.selected_in_range[color] += u64::from(both_in_range);
            self.remembered_selected[color] += u64::from(remembered);
            if row["feeding"]["eaten_n"].as_f64().unwrap() > 0. {
                let class = color * 2 + usize::from(data["target_toxic"] == true);
                self.fed_visible[class] += u64::from(both_visible);
                self.fed_in_range[class] += u64::from(both_in_range);
                self.remembered_fed[class] += u64::from(remembered);
                if remembered {
                    self.remembered_feeding_individuals.insert(id);
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let args: Vec<_> = std::env::args().collect();
    let seed: u64 = args[1].parse()?;
    assert!((12345..=12347).contains(&seed));
    let inherited = args[2].starts_with("inherited-");
    let candidate = args[2] == "inherited-predator-candidate";
    let combined = candidate || args[2] == "inherited-predator-reference";
    let mut predator_trace = Vec::new();
    let mut first_zero = [None; 3];
    let mut inherited_choices = BTreeMap::<u64, InheritedChoice>::new();
    let mut inherited_total = InheritedChoice::default();
    let mut plant_flows = [[0_u64; 4]; 3];
    let mut predator_events = BTreeMap::<String, u64>::new();
    let learning = match args[2].as_str() {
        "on"
        | "rescue"
        | "recovery"
        | "mixed-on"
        | "inherited-on"
        | "inherited-predator-candidate"
        | "inherited-predator-reference" => true,
        "off" | "mixed-off" | "inherited-off" => false,
        _ => return Err("expected on or off".into()),
    };
    let mixed = args[2].starts_with("mixed-");
    let mut config: SimulationConfig = serde_json::from_str(if candidate {
        include_str!("../../../config/stage4-inherited-predator-candidate.json")
    } else if inherited {
        include_str!("../../../config/stage4-inherited-plant-colors.json")
    } else if mixed {
        include_str!("../../../config/stage4-mixed-plant-colors.json")
    } else {
        include_str!("../../../config/stage4-carnivore-threshold-050.json")
    })?;
    config.seed = seed;
    config.herbivores.color_learning_enabled = learning;
    let hysteresis = args[2] == "recovery";
    config.herbivores.hungry_color_recheck_enabled = args[2] == "rescue" || hysteresis;
    config.herbivores.hungry_color_recovery_enabled = hysteresis;
    let mut shadow = World::new(config.clone())?;
    let mut world = World::new(config.clone())?;
    if combined {
        let mut reference_config: SimulationConfig = serde_json::from_str(include_str!(
            "../../../config/stage4-inherited-plant-colors.json"
        ))?;
        reference_config.seed = seed;
        let mut reference = World::new(reference_config)?;
        reference.config.animal_toxin = config.animal_toxin.clone();
        assert_eq!(
            world.audit_state(false, None),
            reference.audit_state(false, None)
        );
    }
    if mixed || inherited {
        let mut opposite = config.clone();
        opposite.herbivores.color_learning_enabled = !learning;
        let mut other = World::new(opposite)?;
        other.config.herbivores.color_learning_enabled = learning;
        assert_eq!(
            world.audit_state(false, None),
            other.audit_state(false, None)
        );
    }
    if mixed {
        let mut plain = config;
        plain.toxin.experimental_plant_toxicity_colors = false;
        let mut without_colors = World::new(plain)?;
        assert_eq!(
            format!("{:?}", world.rng),
            format!("{:?}", without_colors.rng)
        );
        without_colors
            .config
            .toxin
            .experimental_plant_toxicity_colors = true;
        for (plain, mixed) in without_colors
            .plants
            .items
            .iter_mut()
            .zip(&world.plants.items)
        {
            plain.color = mixed.color;
            plain.produces_toxin = mixed.produces_toxin;
        }
        assert_eq!(
            world.audit_state(false, None),
            without_colors.audit_state(false, None)
        );
        assert_eq!(
            world
                .plants
                .items
                .iter()
                .filter(|p| p.produces_toxin)
                .count(),
            150
        );
    }
    let mut coexistence_ticks = 0;
    let mut first_absent = [None; 2];
    let mut founders: BTreeMap<[u32; 2], Founder> = world
        .animals
        .items
        .iter()
        .map(|a| ([a.id.index, a.id.generation], Founder::default()))
        .collect();
    world.foraging_trace = Some(ForagingTrace::new(
        founders
            .keys()
            .map(|id| ForagingWindow {
                id: *id,
                start: 1,
                end: 6000,
            })
            .collect(),
    ));
    world.herbivore_terminal_logging = true;
    if inherited {
        world.enable_carnivore_opportunity_trace();
    }
    let mut samples = Vec::new();
    let mut combined_population = Vec::new();
    let mut capacity_ticks = 0;
    let mut herbivore_end_tick_population_sum = 0;
    let mut removed_herbivore_damage = 0.;
    let mut removed_herbivore_dose = 0.;
    let mut max_population = 0;
    let mut max_toxin_error: f64 = 0.;
    let mut n_error: f64 = 0.;
    let mut e_error: f64 = 0.;
    let n0 = world.resources.total_nutrients();
    let e0 = world.resources.total_energy();
    for tick in 0..=6000 {
        if start.elapsed() >= Duration::from_secs(120) {
            return Err(format!("time limit at Tick {tick}").into());
        }
        if tick > 0 {
            world.step(1)?;
            shadow.step(1)?;
        }
        if inherited {
            for event in world.take_carnivore_toxin_trace() {
                let kind = event["event"].as_str().unwrap();
                *predator_events.entry(kind.to_owned()).or_default() += 1;
                if kind == "opportunity"
                    && event["same_colors"]
                        .as_object()
                        .is_some_and(|m| !m.is_empty())
                {
                    *predator_events
                        .entry("learned_color_visible".into())
                        .or_default() += 1;
                }
                if kind == "decision" && !event["skipped"].is_null() {
                    *predator_events.entry("color_skip".into()).or_default() += 1;
                }
                if combined {
                    predator_trace.push(event);
                }
            }
        }
        let counts = [
            world.plants.len(),
            world.animals.items.len(),
            world.carnivores.len(),
        ];
        for (first, count) in first_zero.iter_mut().zip(counts) {
            if count == 0 {
                first.get_or_insert(tick);
            }
        }
        let total: usize = counts.iter().sum();
        if tick > 0 {
            herbivore_end_tick_population_sum += counts[1];
        }
        let mut plants_by_toxicity = [0; 2];
        if mixed {
            for p in &world.plants.items {
                assert_eq!(
                    p.color,
                    if p.produces_toxin {
                        0xb86be3ff
                    } else {
                        0x4ecb70ff
                    }
                );
                plants_by_toxicity[usize::from(p.produces_toxin)] += 1;
            }
            if tick > 0 {
                coexistence_ticks += u64::from(plants_by_toxicity.iter().all(|&n| n > 0));
                for (absent, n) in first_absent.iter_mut().zip(plants_by_toxicity) {
                    if n == 0 {
                        absent.get_or_insert(tick);
                    }
                }
            }
        }
        let cap = world.config.world.max_living_entities as usize;
        assert!(total <= cap);
        assert!(shadow.plants.len() + shadow.animals.items.len() + shadow.carnivores.len() <= cap);
        capacity_ticks += u64::from(total == cap);
        max_population = max_population.max(total);
        let dn = world.resources.total_nutrients() - n0;
        let de = world.resources.total_energy() - e0 - world.ledger.sunlight_total()
            + world.ledger.heat_total();
        assert!(dn.is_finite() && de.is_finite());
        if inherited {
            assert!(dn.abs() < 0.001 && de.abs() < 0.001);
            let shadow_n = shadow.resources.total_nutrients() - n0;
            let shadow_e = shadow.resources.total_energy() - e0 - shadow.ledger.sunlight_total()
                + shadow.ledger.heat_total();
            assert!(shadow_n.abs() < 0.001 && shadow_e.abs() < 0.001);
        }
        n_error = n_error.max(dn.abs());
        e_error = e_error.max(de.abs());
        for row in std::mem::take(&mut world.foraging_trace.as_mut().unwrap().events) {
            if let Some(kind) = row["plant_event"].as_str() {
                let color = usize::from(row["color"] == json!(0xb86be3ff_u32));
                let class = color * 2 + usize::from(row["toxic"] == true);
                let flow = match kind {
                    "birth" => {
                        assert_eq!(row["toxic"], row["parent_toxic"]);
                        assert_eq!(row["type"], row["parent_type"]);
                        0
                    }
                    "age" => 1,
                    "consumed" => 2,
                    _ => unreachable!(),
                };
                plant_flows[flow][class] += 1;
                continue;
            }
            let id: [u32; 2] = serde_json::from_value(row["id"].clone())?;
            if inherited {
                inherited_choices
                    .entry((tick - 1) / 500)
                    .or_default()
                    .observe(&row, id);
                inherited_total.observe(&row, id);
            }
            founders.get_mut(&id).unwrap().observe(&row, hysteresis);
        }
        for event in std::mem::take(&mut world.herbivore_terminal_events) {
            removed_herbivore_damage += event.toxin_damage_total;
            removed_herbivore_dose += event.doses.iter().sum::<f64>();
            if let Some(f) = founders.get_mut(&[event.id.index, event.id.generation]) {
                f.dose = event.doses.iter().sum();
                f.damage = event.toxin_damage_total;
                f.prevented_damage = event.prevented_damage_total;
                f.upkeep = event.resistance_cost_total;
                f.finish_recovery(event.tick, true);
                f.removal_tick = Some(event.tick);
                f.removal_reason = Some(event.removal_trigger);
            }
        }
        if tick % 500 == 0 {
            audit(&mut world, &shadow);
            let toxin: Value = serde_json::from_str(&world.animal_toxin_snapshot_json())?;
            for error in toxin["balance_error"].as_array().unwrap() {
                let error = error.as_f64().ok_or("nonfinite toxin balance")?.abs();
                assert!(error < 0.001, "toxin balance at Tick {tick}");
                max_toxin_error = max_toxin_error.max(error);
            }
            let plant_n: f64 = world
                .plants
                .items
                .iter()
                .map(|p| world.resources.nutrient_balance(p.body).unwrap())
                .sum();
            let plant_e: f64 = world
                .plants
                .items
                .iter()
                .map(|p| world.resources.energy_balance(p.reserve).unwrap())
                .sum();
            let living_founders = world
                .animals
                .items
                .iter()
                .filter(|a| founders.contains_key(&[a.id.index, a.id.generation]))
                .count();
            let mut classes = [[0.; 3]; 4];
            let mut plant_types = [[0; 2]; 2];
            if inherited {
                assert_eq!(world.spontaneous_births, 0);
                for p in &world.plants.items {
                    let color = usize::from(p.color == 0xb86be3ff);
                    assert!(p.color == 0xb86be3ff || p.color == 0x4ecb70ff);
                    let group = &mut classes[color * 2 + usize::from(p.produces_toxin)];
                    group[0] += 1.;
                    group[1] += world.resources.nutrient_balance(p.body).unwrap();
                    group[2] += world.resources.energy_balance(p.reserve).unwrap();
                    if p.produces_toxin {
                        plant_types[color][p.toxin_type.0 as usize] += 1;
                    }
                }
                for (class, initial) in [120, 30, 30, 120].into_iter().enumerate() {
                    assert_eq!(
                        classes[class][0] as u64 + plant_flows[1][class] + plant_flows[2][class],
                        initial + plant_flows[0][class]
                    );
                }
                if tick == 0 {
                    assert_eq!(plant_types, [[15, 15], [60, 60]]);
                }
            }
            if combined {
                let reserves = |ids: &[f64]| -> Option<f64> {
                    (!ids.is_empty()).then(|| ids.iter().sum::<f64>() / ids.len() as f64)
                };
                let herbivore_e: Vec<_> = world
                    .animals
                    .items
                    .iter()
                    .map(|a| world.resources.energy_balance(a.reserve).unwrap())
                    .collect();
                let carnivore_e: Vec<_> = world
                    .carnivores
                    .iter()
                    .map(|a| world.resources.energy_balance(a.reserve).unwrap())
                    .collect();
                combined_population.push(json!({
                    "tick":tick,"carnivore_births":world.carnivore_births,
                    "carnivore_deaths":world.carnivore_deaths,
                    "herbivore_reserve_e_mean":reserves(&herbivore_e),
                    "carnivore_reserve_e_mean":reserves(&carnivore_e)
                }));
            }
            samples.push(
                json!({"tick":tick,"plant_classes":classes,"plant_types":plant_types,"plant_flows":plant_flows,
                "toxin": if inherited { Some(&toxin) } else { None },
                "herbivore_end_tick_population_sum":herbivore_end_tick_population_sum,
                "herbivore_damage_total":removed_herbivore_damage + world.animals.items.iter().map(|a| a.lifetime_toxin_damage).sum::<f64>(),
                "herbivore_dose_total":removed_herbivore_dose + world.animals.items.iter().map(|a| a.lifetime_toxin_dose).sum::<f64>(),"counts":counts,"living_founders":living_founders,
                "plants_by_toxicity":plants_by_toxicity,"plant_body_n":plant_n,"plant_reserve_e":plant_e,"diagnostics":world.diagnostics,
                "carnivore":world.carnivore_debug,"rng":format!("{:?}",world.rng)}),
            );
        }
    }
    for a in &world.animals.items {
        if let Some(f) = founders.get_mut(&[a.id.index, a.id.generation]) {
            f.finish_recovery(6000, false);
            f.dose = a.lifetime_toxin_dose;
            f.damage = a.lifetime_toxin_damage;
            f.prevented_damage = a.lifetime_prevented_damage;
            f.upkeep = a.lifetime_resistance_cost;
        }
    }
    let rows: Vec<_> = founders
        .into_iter()
        .map(|(id, f)| json!({"id":id,"metrics":f}))
        .collect();
    println!(
        "{}",
        json!({"seed":seed,"learning":learning,"rescue":args[2]=="rescue" || hysteresis,"recovery":hysteresis,"ticks":6000,
        "seconds":start.elapsed().as_secs_f64(),"coexistence_ticks":coexistence_ticks,"first_absent":first_absent,"capacity_ticks":capacity_ticks,
        "max_population":max_population,"max_toxin_error":max_toxin_error,
        "max_n_error":n_error,"max_e_error":e_error,"samples":samples,"founders":rows,
        "predator_events":predator_events,"predator_trace":predator_trace,"combined_population":combined_population,"first_zero":first_zero,"inherited_choices":inherited_choices,"inherited_total":inherited.then_some(inherited_total)})
    );
    Ok(())
}
