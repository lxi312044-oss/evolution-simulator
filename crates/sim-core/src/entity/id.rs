#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub struct EntityId {
    pub index: u32,
    pub generation: u32,
}
