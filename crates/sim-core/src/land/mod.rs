use crate::resources::AccountId;
#[derive(Debug)]
pub struct LandCell {
    pub soil: AccountId,
    pub detritus: AccountId,
}
#[derive(Debug)]
pub struct LandGrid {
    pub columns: u32,
    pub rows: u32,
    pub cells: Vec<LandCell>,
}
impl LandGrid {
    pub fn cell_index(&self, x: f32, y: f32, width: f32, height: f32) -> usize {
        let col =
            ((x / width * self.columns as f32).floor() as i64).clamp(0, self.columns as i64 - 1);
        let row = ((y / height * self.rows as f32).floor() as i64).clamp(0, self.rows as i64 - 1);
        (row as usize) * self.columns as usize + col as usize
    }
}
