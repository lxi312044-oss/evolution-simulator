use crate::{entity::EntityId, resources::AccountId};
#[derive(Debug)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub struct Plant {
    pub id: EntityId,
    pub x: f32,
    pub y: f32,
    pub cell: usize,
    pub body: AccountId,
    pub reserve: AccountId,
    pub growth: f32,
    pub age: u32,
    pub seed_cooldown: u32,
    pub max_age: u32,
    pub color: u32,
    /// Inherited binary toxin-production phenotype; default configuration is all toxic.
    pub produces_toxin: bool,
    /// Heritable toxin-type identity, independent of whether toxin is produced.
    pub toxin_type: crate::toxin::ToxinTypeId,
}
#[derive(Debug, Default)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub struct PlantStore {
    pub items: Vec<Plant>,
}
impl PlantStore {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

pub(crate) fn visible_color(experimental: bool, toxic: bool) -> u32 {
    if experimental && toxic {
        0xb86be3ff
    } else {
        0x4ecb70ff
    }
}

/// Mutate the stored visible RGB using the animal per-channel rule and a
/// plant-specific seed domain. The complete child ID distinguishes reused slots.
pub(crate) fn seed_color(
    parent: u32,
    seed: u64,
    id: EntityId,
    probability: f32,
    scale: f32,
) -> u32 {
    if probability == 0. || scale == 0. {
        return parent;
    }
    let [r, g, b] = [24, 16, 8].map(|shift| ((parent >> shift) & 255) as f32 / 255.);
    let child_key = (u64::from(id.generation) << 32) | u64::from(id.index);
    let color = crate::animals::color::inherit(
        crate::animals::color::BodyColor::new(r, g, b),
        seed ^ 0x42e7_9d03_6b8a_15cf,
        child_key,
        probability,
        scale,
    );
    let [r, g, b] = color.rgb().map(|value| (value * 255.).round() as u32);
    (r << 24) | (g << 16) | (b << 8) | 255
}
