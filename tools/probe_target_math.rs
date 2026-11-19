// Standalone numerical-target probe; not linked into the simulator.
#[no_mangle]
pub extern "C" fn sine_bits(v:f32)->u32 {v.sin().to_bits()}
#[no_mangle]
pub extern "C" fn cosine_bits(v:f32)->u32 {v.cos().to_bits()}
#[no_mangle]
pub extern "C" fn angle_bits(v:f32)->u32 {v.atan2(1.).to_bits()}
#[no_mangle]
pub extern "C" fn scale_bits(v:f32)->u64 {((v as f64)/60.).powf(0.75).to_bits()}
#[no_mangle]
pub extern "C" fn reconstructed_heading_bits(v:f32)->u32 {v.sin().atan2(v.cos()).to_bits()}
#[cfg(not(target_arch="wasm32"))]
fn main(){println!("input_bits,sin_bits,cos_bits,atan_bits,scale_bits,reconstructed_heading_bits");for a in std::env::args().skip(1){let v:f32=a.parse().unwrap();println!("{:08x},{:08x},{:08x},{:08x},{:016x},{:08x}",v.to_bits(),sine_bits(v),cosine_bits(v),angle_bits(v),scale_bits(v),reconstructed_heading_bits(v));}}
