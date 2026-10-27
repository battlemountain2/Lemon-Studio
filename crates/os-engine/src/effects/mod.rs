// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Mixer-slot effects: Lemon Studio's own recreations, configured from a project's saved
//! plugin state.

pub mod balance;
pub mod chorus;
pub mod delay;
pub mod distortion;
pub mod dynamics;
pub mod eq;
pub mod fast_dist;
mod legacy_reverb;
pub mod limiter;
pub mod love_philter;
pub mod maximus;
pub mod reverb;
pub mod soft_clipper;
pub mod stereo_enhancer;
pub mod stereo_shaper;
pub mod transient_processor;

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
    /// The song's tempo in BPM (effects with tempo-based times).
    fn set_tempo(&mut self, _bpm: f64) {}
    /// The delay the effect puts its output out late by, in samples (a look-ahead; spec: audio/delay-compensation.md).
    /// The engine reads it after every `process` and delays the other paths to match. Default: none.
    fn latency(&self) -> usize {
        0
    }
    /// An extra output (Fruity Stereo Shaper's difference output): the number of the track's output it goes to,
    /// counted from 1, and what the block processed last put out on it, interleaved like the block (spec:
    /// audio/sidechain.md "Extra output").
    fn extra(&self) -> Option<(usize, &[f32])> {
        None
    }
    /// A sidechain input (Fruity Limiter's): the number of the track's input it listens to, counted from 1, 0 = none
    /// (spec: audio/sidechain.md "Sidechain input").
    fn key_input(&self) -> usize {
        0
    }
    /// Processes one block with its sidechain input `key`, interleaved like the block. Called instead of `process`
    /// when `key_input` names an input and the track has one.
    fn process_keyed(&mut self, buf: &mut [f32], _key: &[f32]) {
        self.process(buf);
    }
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
        "Lemon Reverb" | "Reverb" if state.starts_with(b"LMR1") => {
            legacy_reverb::LegacyReverb::from_state(state, sample_rate).map(|effect| Box::new(effect) as Box<dyn Effect>)
        }
        "Fruity Reeverb 2" | "Lemon Reverb" | "Reverb" => {
            if state.is_empty() {
                Some(Box::new(reverb::Reverb::new(sample_rate, reverb::Settings::default(), 120.0)) as Box<dyn Effect>)
            } else {
                reverb::Reverb::from_state(state, sample_rate).map(|e| Box::new(e) as Box<dyn Effect>)
            }
        }
        "Fruity Delay 3" | "Lemon Delay" | "Delay" => {
            delay::Delay::from_state(state, sample_rate).map(|e| Box::new(e) as Box<dyn Effect>)
        }
        "Fruity Soft Clipper" => soft_clipper::SoftClipper::from_state(state).map(|e| Box::new(e) as Box<dyn Effect>),
        "Fruity Fast Dist" => fast_dist::FastDist::from_state(state).map(|e| Box::new(e) as Box<dyn Effect>),
        "Fruity Balance" => balance::Balance::from_state(state, sample_rate).map(|e| Box::new(e) as Box<dyn Effect>),
        "Fruity Stereo Enhancer" => stereo_enhancer::StereoEnhancer::from_state(state, sample_rate).map(|e| Box::new(e) as Box<dyn Effect>),
        "Fruity Chorus" => chorus::Chorus::from_state(state, sample_rate).map(|e| Box::new(e) as Box<dyn Effect>),
        "Fruity Stereo Shaper" => stereo_shaper::StereoShaper::from_state(state, sample_rate).map(|e| Box::new(e) as Box<dyn Effect>),
        "Fruity Love Philter" => love_philter::LovePhilter::from_state(state, sample_rate).map(|e| Box::new(e) as Box<dyn Effect>),
        "Maximus" => maximus::Maximus::from_state(state, sample_rate).map(|e| Box::new(e) as Box<dyn Effect>),
        "Transient Processor" => {
            transient_processor::TransientProcessor::from_state(state, sample_rate).map(|e| Box::new(e) as Box<dyn Effect>)
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

/// `N` little-endian int32 fields of a saved state from field `first` on; `None` when the state is shorter.
pub(crate) fn ints<const N: usize>(state: &[u8], first: usize) -> Option<[i32; N]> {
    let mut v = [0; N];
    for (k, x) in v.iter_mut().enumerate() {
        *x = field(state, first + k)?;
    }
    Some(v)
}

/// Values as a saved state of little-endian int32 fields.
pub(crate) fn state_of(v: &[i32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_fl_effects_return_none_without_fake_substitution() {
        assert!(from_slot("Fruity Delay", &[], 44100).is_none());
        assert!(from_slot("Fruity Delay 2", &[], 44100).is_none());
        assert!(from_slot("Fruity Reeverb", &[], 44100).is_none());
        assert!(from_slot("Fruity Blood Overdrive", &[], 44100).is_none());
        assert!(from_slot("Fruity Squeeze", &[], 44100).is_none());
    }

    #[test]
    fn legacy_reverb_matches_the_original_impulse_at_both_rates() {
        for (rate, expected) in [(44100, 0x1589a91a6ca74fc6_u64), (48000, 0xc395ee7689154e1c_u64)] {
            let mut state = b"LMR1".to_vec();
            for value in [0.4_f32, 0.58, 0.88, 0.15] {
                state.extend_from_slice(&value.to_le_bytes());
            }
            let mut effect = from_slot("Lemon Reverb", &state, rate).unwrap();
            let mut audio = vec![0.0; rate as usize * 2];
            audio[0] = 1.0;
            audio[1] = 0.5;
            effect.process(&mut audio);
            let hash = audio.iter().flat_map(|sample| sample.to_le_bytes()).fold(0xcbf29ce484222325_u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
            });
            assert_eq!(hash, expected);
            let mut invalid = state.clone();
            invalid[4..8].copy_from_slice(&f32::NAN.to_le_bytes());
            assert!(from_slot("Lemon Reverb", &invalid, rate).is_none());
            assert!(from_slot("Lemon Reverb", &state[..8], rate).is_none());
        }
    }

    #[test]
    fn legacy_reverb_states_remain_playable() {
        let mut state = b"LMR1".to_vec();
        for value in [0.4_f32, 0.58, 0.88, 0.15] {
            state.extend_from_slice(&value.to_le_bytes());
        }
        let mut effect = from_slot("Lemon Reverb", &state, 48000).expect("legacy reverb");
        let mut impulse = vec![0.0; 48000 * 2];
        impulse[0] = 1.0;
        effect.process(&mut impulse);
        assert_eq!(impulse[0], 0.85);
        assert!(impulse[4000..].iter().any(|sample| sample.abs() > 0.00001));
        effect.reset();
        impulse.fill(0.0);
        effect.process(&mut impulse);
        assert!(impulse.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn lemon_effects_load_and_round_trip_versioned_state() {
        assert!(from_slot("Lemon Delay", &[], 44100).is_some());
        assert!(from_slot("Lemon Reverb", &[], 44100).is_some());
        assert!(from_slot("Lemon Distortion", &[], 44100).is_some());

        let dist = distortion::Distortion::new(DistMode::Bitcrush, 2.5, 6.0, 8, 0.8, 0.9);
        let dist_state = dist.to_state();
        let loaded_dist = distortion::Distortion::from_state("Lemon Bitcrusher", &dist_state, 44100);
        assert_eq!(loaded_dist.mode, DistMode::Bitcrush);
        assert_eq!(loaded_dist.bits, 6.0);
        assert_eq!(loaded_dist.downsample, 8);
    }
}
