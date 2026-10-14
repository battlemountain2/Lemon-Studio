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
        "Lemon Delay" | "Delay" => {
            Some(Box::new(delay::Delay::from_state(state, sample_rate)))
        }
        "Lemon Reverb" | "Reverb" => {
            Some(Box::new(reverb::Reverb::from_state(state, sample_rate)))
        }
        "Lemon Distortion" | "Lemon Bitcrusher" | "Lemon Overdrive" | "Distortion" | "Bitcrusher"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_fl_effects_return_none_without_fake_substitution() {
        assert!(from_slot("Fruity Delay", &[], 44100).is_none());
        assert!(from_slot("Fruity Delay 2", &[], 44100).is_none());
        assert!(from_slot("Fruity Delay 3", &[], 44100).is_none());
        assert!(from_slot("Fruity Reeverb", &[], 44100).is_none());
        assert!(from_slot("Fruity Reeverb 2", &[], 44100).is_none());
        assert!(from_slot("Fruity Fast Dist", &[], 44100).is_none());
        assert!(from_slot("Fruity Blood Overdrive", &[], 44100).is_none());
        assert!(from_slot("Fruity Squeeze", &[], 44100).is_none());
    }

    #[test]
    fn lemon_effects_load_and_round_trip_versioned_state() {
        assert!(from_slot("Lemon Delay", &[], 44100).is_some());
        assert!(from_slot("Lemon Reverb", &[], 44100).is_some());
        assert!(from_slot("Lemon Distortion", &[], 44100).is_some());

        let delay = delay::Delay::new(44100, 250.0, 0.6, 0.2, true, 0.5);
        let state = delay.to_state();
        let loaded = delay::Delay::from_state(&state, 44100);
        assert_eq!(loaded.delay_ms, 250.0);
        assert_eq!(loaded.feedback, 0.6);
        assert_eq!(loaded.mix, 0.5);

        let dist = distortion::Distortion::new(DistMode::Bitcrush, 2.5, 6.0, 8, 0.8, 0.9);
        let dist_state = dist.to_state();
        let loaded_dist = distortion::Distortion::from_state("Lemon Bitcrusher", &dist_state, 44100);
        assert_eq!(loaded_dist.mode, DistMode::Bitcrush);
        assert_eq!(loaded_dist.bits, 6.0);
        assert_eq!(loaded_dist.downsample, 8);
    }
}
