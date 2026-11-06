//! Heritable sRGB body color. Visual genes do not own N/E accounts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}
impl BodyColor {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self {
            r: r.clamp(0., 1.),
            g: g.clamp(0., 1.),
            b: b.clamp(0., 1.),
        }
    }
    pub fn rgb(self) -> [f32; 3] {
        [self.r, self.g, self.b]
    }
}
// Separate stateless PRNG stream: introducing color never shifts ecology RNG draws.
fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}
fn unit(seed: u64) -> f32 {
    ((mix(seed) >> 40) as f32) / 16777216.
}
pub fn initial(seed: u64, id: u64) -> BodyColor {
    let s = seed ^ id.wrapping_mul(0x517cc1b727220a95);
    BodyColor::new(
        0.25 + 0.55 * unit(s),
        0.19 + 0.52 * unit(s ^ 1),
        0.13 + 0.52 * unit(s ^ 2),
    )
}
pub fn inherit_with_flags(
    parent: BodyColor,
    seed: u64,
    id: u64,
    probability: f32,
    scale: f32,
) -> (BodyColor, [bool; 3]) {
    let mut flags = [false; 3];
    let mut rgb = parent.rgb();
    for (channel, value) in rgb.iter_mut().enumerate() {
        let s = seed
            ^ id.wrapping_mul(0x517cc1b727220a95)
            ^ (channel as u64).wrapping_mul(0x9e3779b97f4a7c15);
        if unit(s) < probability {
            flags[channel] = true;
            *value = (*value + (unit(s ^ 0xa5a5a5a5) * 2. - 1.) * scale).clamp(0., 1.);
        }
    }
    (BodyColor::new(rgb[0], rgb[1], rgb[2]), flags)
}
pub fn inherit(parent: BodyColor, seed: u64, id: u64, probability: f32, scale: f32) -> BodyColor {
    inherit_with_flags(parent, seed, id, probability, scale).0
}
