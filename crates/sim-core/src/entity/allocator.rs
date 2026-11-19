use super::EntityId;

#[derive(Default, Debug)]
#[cfg_attr(feature = "state-audit", derive(serde::Serialize))]
pub struct EntityAllocator {
    generations: Vec<u32>,
    alive: Vec<bool>,
    free: Vec<u32>,
    count: usize,
}
impl EntityAllocator {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn spawn(&mut self) -> Result<EntityId, &'static str> {
        if let Some(index) = self.free.pop() {
            let i = index as usize;
            self.alive[i] = true;
            self.count += 1;
            return Ok(EntityId {
                index,
                generation: self.generations[i],
            });
        }
        let index =
            u32::try_from(self.generations.len()).map_err(|_| "entity capacity exceeded")?;
        self.generations.push(0);
        self.alive.push(true);
        self.count += 1;
        Ok(EntityId {
            index,
            generation: 0,
        })
    }
    pub fn contains(&self, id: EntityId) -> bool {
        let i = id.index as usize;
        self.alive.get(i).copied().unwrap_or(false)
            && self.generations.get(i).copied() == Some(id.generation)
    }
    pub fn despawn(&mut self, id: EntityId) -> bool {
        if !self.contains(id) {
            return false;
        }
        let i = id.index as usize;
        // Never reuse a slot once its generation would overflow.
        if let Some(next) = self.generations[i].checked_add(1) {
            self.generations[i] = next;
            self.free.push(id.index);
        }
        self.alive[i] = false;
        self.count -= 1;
        true
    }
    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}
