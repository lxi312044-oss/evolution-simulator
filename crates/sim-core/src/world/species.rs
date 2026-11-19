//! Issue #66 / #67: species and event observation for the Web display.
//!
//! Observation only: nothing here feeds back into ecology, consumes RNG or touches
//! resources, and the state audit does not emit it. Species are assigned at birth from
//! the parent's species and a genetic distance to that species' founder; events are
//! detected at birth and in a census every `CENSUS_TICKS` ticks.
use super::World;
use crate::entity::EntityId;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, VecDeque};

/// Plant, herbivore or carnivore. Species and ranks are kept per kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SpeciesKind {
    Plant,
    Herbivore,
    Carnivore,
}

impl SpeciesKind {
    const ALL: [Self; 3] = [Self::Plant, Self::Herbivore, Self::Carnivore];
    fn key(self) -> &'static str {
        match self {
            Self::Plant => "plant",
            Self::Herbivore => "herbivore",
            Self::Carnivore => "carnivore",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Plant => "植物",
            Self::Herbivore => "草食",
            Self::Carnivore => "肉食",
        }
    }
}

/// Genetic distance from a species founder at or above which a newborn founds a new
/// species ("standard" granularity, Issue #66).
pub const SPECIES_THRESHOLD: f32 = 0.5;
/// Census and global event checks run every this many ticks.
pub const CENSUS_TICKS: u64 = 100;
/// Individual events of the same type are emitted at most once per this many ticks; the
/// rest are counted and reported with the next one.
pub const EVENT_COOLDOWN_TICKS: u64 = 200;
const MAX_EVENTS: usize = 500;
const DOMINANCE_ON: f64 = 0.6;
const DOMINANCE_OFF: f64 = 0.5;
const PLANT_DROP: f64 = 0.3;
const PLANT_DROP_WINDOW: u64 = 1000;
const EXTINCTION_WINDOW: u64 = 2000;
const STRONG_TOXIN: f64 = 0.9;
const HIGH_RESISTANCE: f32 = 0.6;

#[derive(Debug, Clone)]
struct Species {
    kind: SpeciesKind,
    name: String,
    traits: Vec<f32>,
    rgb: [f32; 3],
    feature: String,
    founder: EntityId,
    born_tick: u64,
    count: u64,
    peak: u64,
    extinct_tick: Option<u64>,
}

#[derive(Debug, Clone)]
struct Event {
    tick: u64,
    code: &'static str,
    message: String,
    entity: Option<EntityId>,
    species: Option<u32>,
}

/// Observed traits of one individual at birth.
pub(crate) struct Observed {
    pub kind: SpeciesKind,
    pub traits: Vec<f32>,
    pub rgb: [f32; 3],
    pub toxic: bool,
    pub strong_toxin: bool,
    pub resistant: bool,
}

/// Species registry and event queue (observation only).
#[derive(Debug, Default)]
pub struct SpeciesBook {
    species: Vec<Species>,
    member: HashMap<EntityId, u32>,
    names: BTreeMap<String, u32>,
    events: VecDeque<Event>,
    last_emitted: HashMap<&'static str, u64>,
    suppressed: HashMap<&'static str, u64>,
    dominant: HashMap<SpeciesKind, u32>,
    plant_history: VecDeque<(u64, u64)>,
    plant_drop_until: u64,
    extinctions: Vec<u64>,
    acceleration_until: u64,
    kind_present: HashMap<SpeciesKind, bool>,
}

fn distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

fn mix(mut h: u64) -> u64 {
    h = (h ^ (h >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94d049bb133111eb);
    h ^ (h >> 31)
}

/// Color word from RGB (0..1).
fn color_word(rgb: [f32; 3]) -> &'static str {
    let [r, g, b] = rgb;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    if max < 0.25 {
        return "クロ";
    }
    if max - min < 0.12 {
        return if max > 0.75 { "シロ" } else { "ハイ" };
    }
    let hue = if max == r {
        60. * (((g - b) / (max - min)).rem_euclid(6.))
    } else if max == g {
        60. * ((b - r) / (max - min) + 2.)
    } else {
        60. * ((r - g) / (max - min) + 4.)
    };
    match hue {
        h if !(15. ..345.).contains(&h) => {
            if max > 0.7 && min > 0.45 {
                "モモ"
            } else {
                "アカ"
            }
        }
        h if h < 45. => "ダイダイ",
        h if h < 70. => "キ",
        h if h < 160. => "ミドリ",
        h if h < 200. => "ミズ",
        h if h < 255. => "アオ",
        h if h < 300. => "ムラサキ",
        _ => "モモ",
    }
}

/// Short sound-based word (e.g. トロロ, ララ, ソール) from a stateless hash.
fn sound_word(seed: u64, number: u32) -> String {
    const HEAD: [&str; 20] = [
        "ト", "ラ", "ア", "ミ", "ポ", "ル", "ソ", "ナ", "ク", "ピ", "モ", "ヌ", "リ", "カ", "ウ",
        "テ", "ホ", "セ", "フ", "ニ",
    ];
    const TAIL: [&str; 16] = [
        "ロ", "ラ", "ル", "ー", "ン", "リ", "ノ", "ム", "ッタ", "ール", "ポ", "ナ", "ミ", "ソ",
        "ネ", "ロロ",
    ];
    let h = mix(seed ^ 0x5bec_1e5a_17ed ^ u64::from(number).wrapping_mul(0x9e3779b97f4a7c15));
    let head = HEAD[(h % 20) as usize];
    let tail = TAIL[((h >> 8) % 16) as usize];
    match (h >> 16) % 4 {
        // Repeat the head sound: ララ, トトロ style.
        0 => format!("{head}{head}{tail}"),
        1 => format!("{head}{tail}{tail}"),
        2 => format!("{head}{}{tail}", HEAD[((h >> 24) % 20) as usize]),
        _ => format!("{head}{tail}"),
    }
}

impl SpeciesBook {
    fn new_name(&mut self, seed: u64, number: u32, feature: &str) -> String {
        let base = format!("{feature}・{}", sound_word(seed, number));
        let n = self.names.entry(base.clone()).or_insert(0);
        *n += 1;
        if *n == 1 {
            base
        } else {
            format!("{base}{n}")
        }
    }

    fn emit(
        &mut self,
        tick: u64,
        code: &'static str,
        message: String,
        entity: Option<EntityId>,
        species: Option<u32>,
    ) {
        if self.events.len() == MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(Event {
            tick,
            code,
            message,
            entity,
            species,
        });
    }

    /// Individual events are rate limited per type; suppressed ones are counted.
    fn emit_limited(
        &mut self,
        tick: u64,
        code: &'static str,
        message: String,
        entity: Option<EntityId>,
        species: Option<u32>,
    ) {
        if tick == 0 {
            return;
        }
        let last = self.last_emitted.get(code).copied();
        if last.is_some_and(|t| tick < t + EVENT_COOLDOWN_TICKS) {
            *self.suppressed.entry(code).or_insert(0) += 1;
            return;
        }
        let skipped = self.suppressed.remove(code).unwrap_or(0);
        let message = if skipped > 0 {
            format!("{message}（前回からほか{skipped}件）")
        } else {
            message
        };
        self.last_emitted.insert(code, tick);
        self.emit(tick, code, message, entity, species);
    }

    /// Assigns a species to a newborn (or initial) individual and emits birth events.
    pub(crate) fn on_birth(
        &mut self,
        seed: u64,
        tick: u64,
        id: EntityId,
        child: Observed,
        parent: Option<(EntityId, Observed)>,
        feature: String,
    ) {
        let parent_species = parent.as_ref().and_then(|(p, o)| {
            (o.kind == child.kind)
                .then(|| self.member.get(p).copied())
                .flatten()
        });
        let close = |s: &Species| {
            s.kind == child.kind
                && s.extinct_tick.is_none()
                && distance(&s.traits, &child.traits) < SPECIES_THRESHOLD
        };
        let joined = match parent_species {
            Some(sp) if close(&self.species[sp as usize]) => Some(sp),
            _ => self
                .species
                .iter()
                .enumerate()
                .filter(|(_, s)| close(s))
                .min_by(|(_, a), (_, b)| {
                    distance(&a.traits, &child.traits)
                        .total_cmp(&distance(&b.traits, &child.traits))
                })
                .map(|(i, _)| i as u32),
        };
        let sp = match joined {
            Some(sp) => sp,
            None => {
                let number = self.species.len() as u32;
                let name = self.new_name(seed, number, &feature);
                self.species.push(Species {
                    kind: child.kind,
                    name: name.clone(),
                    traits: child.traits.clone(),
                    rgb: child.rgb,
                    feature,
                    founder: id,
                    born_tick: tick,
                    count: 0,
                    peak: 0,
                    extinct_tick: None,
                });
                self.emit_limited(
                    tick,
                    "new_species",
                    format!("新しい{}の種「{name}」が生まれました", child.kind.label()),
                    Some(id),
                    Some(number),
                );
                number
            }
        };
        self.member.insert(id, sp);
        let name = self.species[sp as usize].name.clone();
        let cross_kind = parent.as_ref().is_some_and(|(_, o)| o.kind != child.kind);
        if cross_kind && child.kind == SpeciesKind::Carnivore {
            self.emit_limited(
                tick,
                "carnivore_from_herbivore",
                format!("草食から肉食が生まれました（{name}）"),
                Some(id),
                Some(sp),
            );
        }
        let Some((_, p)) = parent.filter(|(_, o)| o.kind == child.kind) else {
            return;
        };
        if child.kind == SpeciesKind::Plant && child.toxic && !p.toxic {
            self.emit_limited(
                tick,
                "toxic_plant",
                format!("毒のある植物が生まれました（{name}）"),
                Some(id),
                Some(sp),
            );
        }
        if child.kind != SpeciesKind::Plant && child.strong_toxin && !p.strong_toxin {
            self.emit_limited(
                tick,
                "toxic_animal",
                format!("強い毒をもつ{}が生まれました（{name}）", child.kind.label()),
                Some(id),
                Some(sp),
            );
        }
        if child.resistant && !p.resistant {
            self.emit_limited(
                tick,
                "resistant_animal",
                format!(
                    "毒への耐性が高い{}が生まれました（{name}）",
                    child.kind.label()
                ),
                Some(id),
                Some(sp),
            );
        }
    }

    /// Counts living members, records extinctions and emits global events.
    fn census(&mut self, tick: u64, living: &[(SpeciesKind, EntityId)]) {
        let mut counts = vec![0u64; self.species.len()];
        let mut member = HashMap::with_capacity(living.len());
        for (_, id) in living {
            if let Some(&sp) = self.member.get(id) {
                counts[sp as usize] += 1;
                member.insert(*id, sp);
            }
        }
        self.member = member;
        let mut extinct_now = Vec::new();
        for (i, s) in self.species.iter_mut().enumerate() {
            s.count = counts[i];
            s.peak = s.peak.max(s.count);
            if s.count == 0 && s.extinct_tick.is_none() {
                s.extinct_tick = Some(tick);
                extinct_now.push(i as u32);
            }
        }
        for sp in extinct_now {
            let s = &self.species[sp as usize];
            let message = format!("{}の種「{}」が絶滅しました", s.kind.label(), s.name);
            self.extinctions.push(tick);
            self.emit_limited(tick, "species_extinct", message, None, Some(sp));
        }
        // Acceleration: >= 3 extinctions in the last window and >= 2x the window before.
        let recent = self
            .extinctions
            .iter()
            .filter(|&&t| t + EXTINCTION_WINDOW > tick)
            .count();
        let before = self
            .extinctions
            .iter()
            .filter(|&&t| t + EXTINCTION_WINDOW <= tick && t + 2 * EXTINCTION_WINDOW > tick)
            .count();
        if tick >= self.acceleration_until && recent >= 3 && recent >= 2 * before.max(1) {
            self.acceleration_until = tick + EXTINCTION_WINDOW;
            self.emit(
                tick,
                "extinction_accelerating",
                format!("種の絶滅が加速しています（直近{EXTINCTION_WINDOW} Tickで{recent}種）"),
                None,
                None,
            );
        }
        for kind in SpeciesKind::ALL {
            let total: u64 = self
                .species
                .iter()
                .filter(|s| s.kind == kind)
                .map(|s| s.count)
                .sum();
            let present = total > 0;
            let was = self.kind_present.insert(kind, present).unwrap_or(present);
            if was && !present {
                self.emit(
                    tick,
                    "kind_gone",
                    format!("{}がいなくなりました", kind.label()),
                    None,
                    None,
                );
            } else if !was && present && kind == SpeciesKind::Carnivore {
                let again = self
                    .species
                    .iter()
                    .any(|s| s.kind == kind && s.extinct_tick.is_some());
                let message = if again {
                    "が再び現れました"
                } else {
                    "が初めて現れました"
                };
                self.emit(
                    tick,
                    "kind_back",
                    format!("{}{message}", kind.label()),
                    None,
                    None,
                );
            }
            if total == 0 {
                self.dominant.remove(&kind);
                continue;
            }
            let (top, count) = self
                .species
                .iter()
                .enumerate()
                .filter(|(_, s)| s.kind == kind)
                .map(|(i, s)| (i as u32, s.count))
                .max_by_key(|&(i, c)| (c, std::cmp::Reverse(i)))
                .unwrap();
            let share = count as f64 / total as f64;
            let current = self.dominant.get(&kind).copied();
            if share >= DOMINANCE_ON && total >= 10 && current != Some(top) {
                self.dominant.insert(kind, top);
                let name = self.species[top as usize].name.clone();
                self.emit(
                    tick,
                    "dominance",
                    format!(
                        "{}の種「{name}」の支配率が高くなっています（{:.0}%）",
                        kind.label(),
                        share * 100.
                    ),
                    None,
                    Some(top),
                );
            } else if share < DOMINANCE_OFF && current.is_some() {
                self.dominant.remove(&kind);
            }
        }
        let plants: u64 = self
            .species
            .iter()
            .filter(|s| s.kind == SpeciesKind::Plant)
            .map(|s| s.count)
            .sum();
        self.plant_history.push_back((tick, plants));
        while self
            .plant_history
            .front()
            .is_some_and(|&(t, _)| t + PLANT_DROP_WINDOW < tick)
        {
            self.plant_history.pop_front();
        }
        if let Some(&(t0, before)) = self.plant_history.front() {
            if t0 + PLANT_DROP_WINDOW == tick
                && tick >= self.plant_drop_until
                && before > 0
                && (plants as f64) <= before as f64 * (1. - PLANT_DROP)
            {
                self.plant_drop_until = tick + PLANT_DROP_WINDOW;
                self.emit(
                    tick,
                    "plants_declining",
                    format!("植物が減っています（{PLANT_DROP_WINDOW} Tickで{before}→{plants}株）"),
                    None,
                    None,
                );
            }
        }
    }

    pub fn species_of(&self, id: EntityId) -> Option<u32> {
        self.member.get(&id).copied()
    }

    pub fn name_of(&self, number: u32) -> Option<&str> {
        self.species.get(number as usize).map(|s| s.name.as_str())
    }

    pub fn species_count(&self) -> usize {
        self.species.len()
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Living species per kind, ranked by count (rank labels アルファ/ベータ/ガンマ for 1-3).
    pub fn summary(&self, tick: u64) -> Value {
        const LABELS: [&str; 3] = ["アルファ", "ベータ", "ガンマ"];
        let mut kinds = serde_json::Map::new();
        for kind in SpeciesKind::ALL {
            let total: u64 = self
                .species
                .iter()
                .filter(|s| s.kind == kind)
                .map(|s| s.count)
                .sum();
            let mut living: Vec<(usize, &Species)> = self
                .species
                .iter()
                .enumerate()
                .filter(|(_, s)| s.kind == kind && s.count > 0)
                .collect();
            living.sort_by_key(|(i, s)| (std::cmp::Reverse(s.count), *i));
            let list: Vec<Value> = living
                .iter()
                .enumerate()
                .map(|(rank, (i, s))| {
                    json!({
                        "id": i,
                        "rank": rank + 1,
                        "label": LABELS.get(rank).copied(),
                        "name": s.name,
                        "feature": s.feature,
                        "count": s.count,
                        "share": if total > 0 { s.count as f64 / total as f64 } else { 0. },
                        "rgb": s.rgb,
                        "born_tick": s.born_tick,
                        "age_ticks": tick.saturating_sub(s.born_tick),
                        "peak": s.peak,
                        "founder": [s.founder.index, s.founder.generation],
                    })
                })
                .collect();
            kinds.insert(
                kind.key().into(),
                json!({
                    "total": total,
                    "living_species": list.len(),
                    "extinct_species": self.species.iter().filter(|s| s.kind == kind && s.extinct_tick.is_some()).count(),
                    "species": list,
                }),
            );
        }
        json!({"tick": tick, "census_ticks": CENSUS_TICKS, "kinds": kinds})
    }

    /// Returns and clears the queued events (oldest first).
    pub fn take_events(&mut self) -> Value {
        let events: Vec<Value> = self
            .events
            .drain(..)
            .map(|e| {
                json!({
                    "tick": e.tick,
                    "code": e.code,
                    "message": e.message,
                    "entity": e.entity.map(|id| [id.index, id.generation]),
                    "species": e.species,
                })
            })
            .collect();
        Value::Array(events)
    }
}

fn plant_rgb(color: u32) -> [f32; 3] {
    [24, 16, 8].map(|s| ((color >> s) & 255) as f32 / 255.)
}

fn observe_plant(p: &crate::plants::Plant) -> Observed {
    let rgb = plant_rgb(p.color);
    let mut traits = rgb.to_vec();
    traits.push(if p.produces_toxin { 0.6 } else { 0. });
    Observed {
        kind: SpeciesKind::Plant,
        traits,
        rgb,
        toxic: p.produces_toxin,
        strong_toxin: false,
        resistant: false,
    }
}

fn feature_word(o: &Observed) -> String {
    let mut word = color_word(o.rgb).to_string();
    let t = &o.traits;
    match o.kind {
        SpeciesKind::Plant => {
            if o.toxic {
                word.insert_str(0, "ドク");
            }
        }
        SpeciesKind::Herbivore => {
            if o.strong_toxin {
                word.insert_str(0, "ドク");
            }
            word.push_str(if t[3] >= 0.8 {
                "ハヤ"
            } else if t[3] <= 0.4 {
                "ノロ"
            } else if t[4] >= 0.58 {
                "オオ"
            } else if t[5] >= 0.6 {
                "アラ"
            } else if t[7] >= 0.6 {
                "タフ"
            } else {
                ""
            });
        }
        SpeciesKind::Carnivore => {
            if o.strong_toxin {
                word.insert_str(0, "ドク");
            }
            word.push_str(if t[3] >= 0.75 {
                "ハヤ"
            } else if t[4] >= 0.6 {
                "マチ"
            } else {
                "キバ"
            });
        }
    }
    word
}

/// Observe a newborn or initial plant. A free function over disjoint fields so it can be
/// called while plant settings are borrowed.
pub(super) fn plant_birth(
    book: &mut SpeciesBook,
    plants: &[crate::plants::Plant],
    seed: u64,
    tick: u64,
    index: usize,
    parent: Option<usize>,
) {
    let child = observe_plant(&plants[index]);
    let parent = parent.map(|p| (plants[p].id, observe_plant(&plants[p])));
    let feature = feature_word(&child);
    book.on_birth(seed, tick, plants[index].id, child, parent, feature);
}

impl World {
    fn observe_herbivore(&self, index: usize) -> Observed {
        let h = &self.animals.items[index];
        let rgb = h.genes.body_color.rgb();
        let ability = self.herbivore_toxin_ability(h.id);
        let mut traits = rgb.to_vec();
        traits.extend([
            h.genes.speed / 4.,
            h.genes.adult_max_hp / 120.,
            h.genes.aggression,
            h.genes.fear,
            h.genes.toxin_resistance,
            0.5 * ability as f32,
        ]);
        Observed {
            kind: SpeciesKind::Herbivore,
            traits,
            rgb,
            toxic: false,
            strong_toxin: ability >= STRONG_TOXIN,
            resistant: h.genes.toxin_resistance >= HIGH_RESISTANCE,
        }
    }

    fn observe_carnivore(&self, index: usize) -> Observed {
        let c = &self.carnivores[index];
        let rgb = c.color.rgb();
        let ability = self.carnivore_toxin_ability(c.id);
        let mut traits = rgb.to_vec();
        traits.extend([c.speed / 8., c.ambush_gene, 0.5 * ability as f32]);
        Observed {
            kind: SpeciesKind::Carnivore,
            traits,
            rgb,
            toxic: false,
            strong_toxin: ability >= STRONG_TOXIN,
            resistant: false,
        }
    }

    fn species_birth(
        &mut self,
        child: Observed,
        id: EntityId,
        parent: Option<(EntityId, Observed)>,
    ) {
        let feature = feature_word(&child);
        let (seed, tick) = (self.config.seed, self.tick);
        self.species
            .on_birth(seed, tick, id, child, parent, feature);
    }

    pub(super) fn species_plant_birth(&mut self, index: usize, parent: Option<usize>) {
        let (seed, tick) = (self.config.seed, self.tick);
        plant_birth(
            &mut self.species,
            &self.plants.items,
            seed,
            tick,
            index,
            parent,
        );
    }

    pub(super) fn species_herbivore_birth(&mut self, index: usize, parent: Option<usize>) {
        let child = self.observe_herbivore(index);
        let parent = parent.map(|p| (self.animals.items[p].id, self.observe_herbivore(p)));
        self.species_birth(child, self.animals.items[index].id, parent);
    }

    /// `herbivore_parent` marks a carnivore born from a herbivore.
    pub(super) fn species_carnivore_birth(
        &mut self,
        index: usize,
        parent: Option<usize>,
        herbivore_parent: Option<usize>,
    ) {
        let child = self.observe_carnivore(index);
        let parent = match (parent, herbivore_parent) {
            (Some(p), _) => Some((self.carnivores[p].id, self.observe_carnivore(p))),
            (None, Some(h)) => Some((self.animals.items[h].id, self.observe_herbivore(h))),
            _ => None,
        };
        self.species_birth(child, self.carnivores[index].id, parent);
    }

    /// Census and global events every `CENSUS_TICKS` ticks (call at the end of a tick).
    pub(super) fn species_census_if_due(&mut self) {
        if !self.tick.is_multiple_of(CENSUS_TICKS) {
            return;
        }
        let living: Vec<(SpeciesKind, EntityId)> = self
            .plants
            .items
            .iter()
            .map(|p| (SpeciesKind::Plant, p.id))
            .chain(
                self.animals
                    .items
                    .iter()
                    .map(|h| (SpeciesKind::Herbivore, h.id)),
            )
            .chain(
                self.carnivores
                    .iter()
                    .map(|c| (SpeciesKind::Carnivore, c.id)),
            )
            .collect();
        self.species.census(self.tick, &living);
    }

    /// Species ranking per kind as JSON (Issue #66).
    pub fn species_summary_json(&self) -> String {
        self.species.summary(self.tick).to_string()
    }

    /// Queued events as JSON, oldest first; clears the queue (Issue #67).
    pub fn take_events_json(&mut self) -> String {
        self.species.take_events().to_string()
    }

    /// Name of the species an entity belongs to, if any.
    pub fn entity_species_name(&self, id: EntityId) -> Option<String> {
        let sp = self.species.species_of(id)?;
        self.species.name_of(sp).map(str::to_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_words_cover_basic_colors() {
        assert_eq!(color_word([0.9, 0.1, 0.1]), "アカ");
        assert_eq!(color_word([0.1, 0.8, 0.2]), "ミドリ");
        assert_eq!(color_word([0.1, 0.2, 0.9]), "アオ");
        assert_eq!(color_word([0.05, 0.05, 0.05]), "クロ");
        assert_eq!(color_word([0.9, 0.9, 0.9]), "シロ");
        assert_eq!(color_word([0.9, 0.85, 0.1]), "キ");
    }

    #[test]
    fn sound_words_are_deterministic_and_varied() {
        assert_eq!(sound_word(7, 3), sound_word(7, 3));
        let words: std::collections::BTreeSet<String> = (0..50).map(|n| sound_word(7, n)).collect();
        assert!(words.len() > 30);
    }

    fn id(index: u32) -> EntityId {
        EntityId {
            index,
            generation: 0,
        }
    }

    fn herb(traits: [f32; 3], strong: bool, resistant: bool) -> Observed {
        Observed {
            kind: SpeciesKind::Herbivore,
            traits: traits.to_vec(),
            rgb: traits,
            toxic: false,
            strong_toxin: strong,
            resistant,
        }
    }

    #[test]
    fn close_offspring_join_parent_species_and_distant_ones_found_new() {
        let mut book = SpeciesBook::default();
        book.on_birth(
            1,
            0,
            id(1),
            herb([0.1, 0.1, 0.1], false, false),
            None,
            "A".into(),
        );
        book.on_birth(
            1,
            0,
            id(2),
            herb([0.9, 0.9, 0.9], false, false),
            None,
            "B".into(),
        );
        assert_eq!(book.species_count(), 2);
        let parent = || (id(1), herb([0.1, 0.1, 0.1], false, false));
        book.on_birth(
            1,
            10,
            id(3),
            herb([0.2, 0.1, 0.1], false, false),
            Some(parent()),
            "A".into(),
        );
        assert_eq!(book.species_of(id(3)), Some(0));
        // Far from its parent's founder but close to species 1: joins the nearest.
        book.on_birth(
            1,
            20,
            id(4),
            herb([0.8, 0.9, 0.9], false, false),
            Some(parent()),
            "B".into(),
        );
        assert_eq!(book.species_of(id(4)), Some(1));
        // Far from every species: founds a new one with an event.
        book.on_birth(
            1,
            300,
            id(5),
            herb([0.9, 0.1, 0.1], false, false),
            Some(parent()),
            "C".into(),
        );
        assert_eq!(book.species_of(id(5)), Some(2));
        let events = book.take_events();
        assert_eq!(events[0]["code"], "new_species");
        assert_eq!(events[0]["entity"], json!([5, 0]));
    }

    #[test]
    fn names_are_unique_even_with_the_same_feature() {
        let mut book = SpeciesBook::default();
        let names: std::collections::BTreeSet<String> =
            (0..40).map(|n| book.new_name(3, n % 3, "アカ")).collect();
        assert_eq!(names.len(), 40);
    }

    #[test]
    fn ranking_counts_living_members_per_kind() {
        let mut book = SpeciesBook::default();
        let mut living = Vec::new();
        for i in 0..10 {
            let traits = if i < 7 { [0.1; 3] } else { [0.9; 3] };
            book.on_birth(1, 0, id(i), herb(traits, false, false), None, "X".into());
            living.push((SpeciesKind::Herbivore, id(i)));
        }
        living.pop();
        book.census(100, &living);
        let summary = book.summary(100);
        let list = &summary["kinds"]["herbivore"]["species"];
        assert_eq!(list[0]["count"], 7);
        assert_eq!(list[0]["label"], "アルファ");
        assert_eq!(list[1]["count"], 2);
        assert_eq!(list[1]["label"], "ベータ");
        assert_eq!(summary["kinds"]["herbivore"]["total"], 9);
        // 7/9 >= 60% with >= 10 individuals is not reached (9), so no dominance event.
        let codes: Vec<String> = book
            .take_events()
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["code"].as_str().unwrap().to_string())
            .collect();
        assert!(!codes.contains(&"dominance".to_string()));
    }

    #[test]
    fn individual_events_are_rate_limited_and_counted() {
        let mut book = SpeciesBook::default();
        book.on_birth(1, 0, id(0), herb([0.1; 3], false, false), None, "X".into());
        let parent = || (id(0), herb([0.1; 3], false, false));
        for i in 1..=5 {
            book.on_birth(
                1,
                100 + i as u64,
                id(i),
                herb([0.1; 3], false, true),
                Some(parent()),
                "X".into(),
            );
        }
        book.on_birth(
            1,
            400,
            id(9),
            herb([0.1; 3], false, true),
            Some(parent()),
            "X".into(),
        );
        let events = book.take_events();
        let resistant: Vec<&Value> = events
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["code"] == "resistant_animal")
            .collect();
        assert_eq!(resistant.len(), 2);
        assert!(resistant[1]["message"]
            .as_str()
            .unwrap()
            .contains("ほか4件"));
        assert_eq!(book.event_count(), 0);
    }

    #[test]
    fn extinction_and_kind_gone_are_reported() {
        let mut book = SpeciesBook::default();
        book.on_birth(1, 0, id(0), herb([0.1; 3], false, false), None, "X".into());
        book.census(100, &[(SpeciesKind::Herbivore, id(0))]);
        book.census(200, &[]);
        let codes: Vec<String> = book
            .take_events()
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["code"].as_str().unwrap().to_string())
            .collect();
        assert!(codes.contains(&"species_extinct".to_string()));
        assert!(codes.contains(&"kind_gone".to_string()));
    }
}
