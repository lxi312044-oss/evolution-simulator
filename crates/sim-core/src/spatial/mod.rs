//! Rebuilt once per tick after plant births/deaths. Deterministic bucket ordering.
use crate::plants::PlantStore;
#[derive(Debug)]
pub struct PlantGrid {
    pub columns: usize,
    pub rows: usize,
    pub cell_width: f32,
    pub cell_height: f32,
    pub buckets: Vec<Vec<usize>>,
}
impl PlantGrid {
    pub fn new(width: f32, height: f32, cell_size: f32) -> Self {
        let columns = (width / cell_size).ceil().max(1.) as usize;
        let rows = (height / cell_size).ceil().max(1.) as usize;
        Self {
            columns,
            rows,
            cell_width: width / columns as f32,
            cell_height: height / rows as f32,
            buckets: vec![Vec::new(); columns * rows],
        }
    }
    pub fn rebuild(&mut self, plants: &PlantStore) {
        for b in &mut self.buckets {
            b.clear();
        }
        for (i, p) in plants.items.iter().enumerate() {
            let col = ((p.x / self.cell_width) as usize).min(self.columns - 1);
            let row = ((p.y / self.cell_height) as usize).min(self.rows - 1);
            self.buckets[row * self.columns + col].push(i);
        }
    }
    pub fn count_within(&self, plants: &PlantStore, x: f32, y: f32, radius: f32) -> usize {
        let min_c = (((x - radius).max(0.) / self.cell_width) as usize).min(self.columns - 1);
        let max_c = (((x + radius).max(0.) / self.cell_width) as usize).min(self.columns - 1);
        let min_r = (((y - radius).max(0.) / self.cell_height) as usize).min(self.rows - 1);
        let max_r = (((y + radius).max(0.) / self.cell_height) as usize).min(self.rows - 1);
        let mut count = 0;
        for r in min_r..=max_r {
            for c in min_c..=max_c {
                for &i in &self.buckets[r * self.columns + c] {
                    let plant = &plants.items[i];
                    let dx = plant.x - x;
                    let dy = plant.y - y;
                    if dx * dx + dy * dy <= radius * radius {
                        count += 1;
                    }
                }
            }
        }
        count
    }
    pub fn nearest(&self, plants: &PlantStore, x: f32, y: f32, vision: f32) -> Option<usize> {
        self.nearest_allowed(plants, x, y, vision, |_| true)
    }

    pub(crate) fn nearest_allowed(
        &self,
        plants: &PlantStore,
        x: f32,
        y: f32,
        vision: f32,
        allowed: impl Fn(u32) -> bool,
    ) -> Option<usize> {
        let min_c = (((x - vision).max(0.) / self.cell_width) as usize).min(self.columns - 1);
        let max_c = (((x + vision).max(0.) / self.cell_width) as usize).min(self.columns - 1);
        let min_r = (((y - vision).max(0.) / self.cell_height) as usize).min(self.rows - 1);
        let max_r = (((y + vision).max(0.) / self.cell_height) as usize).min(self.rows - 1);
        let mut best = None;
        let mut best_d2 = vision * vision;
        for r in min_r..=max_r {
            for c in min_c..=max_c {
                for &i in &self.buckets[r * self.columns + c] {
                    let p = &plants.items[i];
                    let dx = p.x - x;
                    let dy = p.y - y;
                    let d2 = dx * dx + dy * dy;
                    if d2 <= best_d2
                        && allowed(p.color)
                        && (d2 < best_d2
                            || best.is_none_or(|j: usize| p.id.index < plants.items[j].id.index))
                    {
                        best = Some(i);
                        best_d2 = d2;
                    }
                }
            }
        }
        best
    }
}
