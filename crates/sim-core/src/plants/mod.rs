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
