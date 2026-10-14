//! Mixer-slot effects: Lemon Studio's own recreations, configured from a project's saved
//! plugin state.

mod biquad;
pub mod delay;
pub mod distortion;
pub mod eq;
pub mod limiter;
pub mod reverb;

pub use biquad::Biquad;
pub use delay::Delay;
pub use distortion::{DistMode, Distortion};
pub use limiter::Limiter;
pub use reverb::Reverb;

/// An insert effect processing interleaved stereo in place.
pub trait Effect: Send {
    fn process(&mut self, buf: &mut [f32]);
    /// Empties delay lines and filter states (a slot switched on during the song, spec: audio/mixer.md "Changes while
    /// the song plays").
    fn reset(&mut self) {}
}

/// Creates a Lemon Studio effect for a slot plugin, if one exists for that plugin.
/// The FLP plugin name only identifies the saved-state format.
pub fn from_slot(name: &str, state: &[u8], sample_rate: u32) -> Option<Box<dyn Effect>> {
    match name {
        "Fruity Parametric EQ 2" | "Lemon Parametric EQ 2" => {
            eq::ParametricEq::from_state(state, sample_rate).map(|e| Box::new(e) as Box<dyn Effect>)
        }
        "Fruity Limiter" | "Lemon Limiter" => {
            limiter::Limiter::from_state(state, sample_rate).map(|e| Box::new(e) as Box<dyn Effect>)
        }
        "Fruity Delay" | "Fruity Delay 2" | "Fruity Delay 3" | "Lemon Delay" | "Delay" => {
            Some(Box::new(delay::Delay::from_state(state, sample_rate)))
        }
        "Fruity Reeverb" | "Fruity Reeverb 2" | "Lemon Reverb" | "Reverb" => {
            Some(Box::new(reverb::Reverb::from_state(state, sample_rate)))
        }
        "Fruity Fast Dist" | "Fruity Blood Overdrive" | "Fruity Waveshaper" | "Fruity Squeeze"
        | "Lemon Distortion" | "Lemon Bitcrusher" | "Lemon Overdrive" | "Distortion" | "Bitcrusher"
        | "Overdrive" | "Waveshaper" => {
            Some(Box::new(distortion::Distortion::from_state(name, state, sample_rate)))
        }
        _ => None,
    }
}

/// Little-endian int32 field `k` of a saved state.
pub(crate) fn field(state: &[u8], k: usize) -> Option<i32> {
    state.get(4 * k..4 * k + 4).map(|b| i32::from_le_bytes(b.try_into().unwrap()))
}
