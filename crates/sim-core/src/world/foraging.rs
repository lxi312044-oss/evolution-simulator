//! Read-only foraging decisions at explicitly selected individual/tick windows.
use crate::entity::EntityId;
use serde_json::Value;
#[derive(Debug, serde::Deserialize)]
pub struct ForagingWindow {
    pub id: [u32; 2],
    pub start: u64,
    pub end: u64,
}
#[derive(Debug)]
pub struct ForagingTrace {
    pub windows: Vec<ForagingWindow>,
    pub events: Vec<Value>,
}
impl ForagingTrace {
    pub fn new(windows: Vec<ForagingWindow>) -> Self {
        Self {
            windows,
            events: Vec::new(),
        }
    }
    pub fn observes(&self, id: EntityId, tick: u64) -> bool {
        self.windows
            .iter()
            .any(|w| w.id == [id.index, id.generation] && (w.start..=w.end).contains(&tick))
    }
}
