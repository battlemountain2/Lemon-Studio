//! Polyphonic DSP engine with up to 16-voice unison and non-linear phase warp for Lemon Wavetable.

use crate::table::Wavetable;
use crate::warp::{self, WarpMode};
use std::f32::consts::FRAC_PI_2;

pub const MAX_VOICES: usize = 32;
pub const MAX_UNISON: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EnvStage {
    Off,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Copy, Debug)]
pub struct UnisonSubVoice {
    pub phase: f64,
    pub detune_ratio: f64,
    pub pan_l: f32,
    pub pan_r: f32,
    pub gain: f32,
}

impl Default for UnisonSubVoice {
    fn default() -> Self {
        Self {
            phase: 0.0,
            detune_ratio: 1.0,
            pan_l: 0.7071,
            pan_r: 0.7071,
            gain: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Voice {
    pub id: Option<i32>,
    pub channel: u8,
    pub note: u8,
    pub velocity: f32,
    pub base_freq: f64,
    pub active: bool,
    pub stage: EnvStage,
    pub env_level: f32,
    pub unison_count: usize,
    pub unison_voices: [UnisonSubVoice; MAX_UNISON],
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            id: None,
            channel: 0,
            note: 60,
            velocity: 0.0,
            base_freq: 440.0,
            active: false,
            stage: EnvStage::Off,
            env_level: 0.0,
            unison_count: 1,
            unison_voices: [UnisonSubVoice::default(); MAX_UNISON],
        }
    }
}

impl Voice {
    pub fn note_on(
        &mut self,
        id: Option<i32>,
        channel: u8,
        note: u8,
        velocity: f32,
        bend_ratio: f64,
        unison_count: usize,
        detune_depth: f32,
        blend: f32,
        stereo_spread: f32,
        phase_rand: f32,
    ) {
        self.id = id;
        self.channel = channel;
        self.note = note;
        self.velocity = velocity.clamp(0.0, 1.0);
        self.base_freq = midi_note_to_freq(note) * bend_ratio;
        self.active = true;
        self.stage = EnvStage::Attack;

        let num_unison = unison_count.clamp(1, MAX_UNISON);
        self.unison_count = num_unison;

        // Equal-power normalization factor across unison voices
        let norm_power = 1.0 / (num_unison as f32).sqrt();

        for i in 0..num_unison {
            let initial_phase = if phase_rand > 0.001 {
                pseudo_rand(note, i) * phase_rand as f64
            } else {
                0.0
            };

            let (detune_ratio, pan_l, pan_r, gain) = if num_unison == 1 {
                (1.0, 0.7071, 0.7071, 1.0)
            } else {
                // Normalized spread index [-1.0, +1.0]
                let t = (2.0 * i as f32 / (num_unison - 1) as f32) - 1.0;

                // Exponential detune curve (Serum-style scaling up to 150 cents max)
                let detune_cents = t.signum() * t.abs().powf(1.3) * (detune_depth * 150.0);
                let detune_ratio = 2.0f64.powf((detune_cents as f64) / 1200.0);

                // Stereo Panning (equal-power law)
                let pan_pos = (0.5 + 0.5 * t * stereo_spread).clamp(0.0, 1.0);
                let pan_l = (pan_pos * FRAC_PI_2).cos();
                let pan_r = (pan_pos * FRAC_PI_2).sin();

                // Unison Blend: center voice vs side voice attenuation
                let is_center = (num_unison % 2 == 1) && (i == num_unison / 2);
                let voice_gain = if is_center {
                    1.0
                } else {
                    blend.clamp(0.1, 1.0)
                } * norm_power;

                (detune_ratio, pan_l, pan_r, voice_gain)
            };

            self.unison_voices[i] = UnisonSubVoice {
                phase: initial_phase,
                detune_ratio,
                pan_l,
                pan_r,
                gain,
            };
        }
    }

    pub fn note_off(&mut self) {
        if self.active && self.stage != EnvStage::Off {
            self.stage = EnvStage::Release;
        }
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.stage = EnvStage::Off;
        self.env_level = 0.0;
    }
}

pub struct Settings {
    pub wt_pos: f32,
    pub table_idx: usize,
    pub level: f32,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    // Milestone 2 Parameters:
    pub unison: usize,
    pub detune: f32,
    pub blend: f32,
    pub spread: f32,
    pub phase_rand: f32,
    pub warp_mode: WarpMode,
    pub warp_amt: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            wt_pos: 0.32,
            table_idx: 0,
            level: 0.25,
            attack: 0.012,
            decay: 0.450,
            sustain: 0.70,
            release: 0.250,
            unison: 7,
            detune: 0.18,
            blend: 0.80,
            spread: 1.0,
            phase_rand: 1.0,
            warp_mode: WarpMode::None,
            warp_amt: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Ended {
    pub id: Option<i32>,
    pub channel: u8,
    pub note: u8,
}

pub struct Engine {
    pub sample_rate: f64,
    pub voices: [Voice; MAX_VOICES],
    pub tables: Vec<Wavetable>,
    pub bend_semitones: f64,
}

impl Engine {
    pub fn new(sample_rate: f64) -> Self {
        let tables = vec![
            Wavetable::basic_shapes(),
            Wavetable::analog_saw(),
            Wavetable::digital_bell(),
        ];
        Self {
            sample_rate: sample_rate.max(8000.0),
            voices: std::array::from_fn(|_| Voice::default()),
            tables,
            bend_semitones: 0.0,
        }
    }

    pub fn reset(&mut self) {
        for v in &mut self.voices {
            v.stop();
        }
        self.bend_semitones = 0.0;
    }

    pub fn set_pitch_bend(&mut self, bend_norm: f32) {
        self.bend_semitones = (bend_norm as f64).clamp(-1.0, 1.0) * 2.0;
        let bend_ratio = 2.0f64.powf(self.bend_semitones / 12.0);
        for v in &mut self.voices {
            if v.active {
                v.base_freq = midi_note_to_freq(v.note) * bend_ratio;
            }
        }
    }

    pub fn note_on(
        &mut self,
        id: Option<i32>,
        channel: u8,
        note: u8,
        velocity: f32,
        settings: &Settings,
    ) {
        let bend_ratio = 2.0f64.powf(self.bend_semitones / 12.0);

        // Find inactive voice, or steal oldest/quietest
        let slot = self.voices.iter().position(|v| !v.active).unwrap_or_else(|| {
            self.voices
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.env_level.partial_cmp(&b.env_level).unwrap())
                .map(|(i, _)| i)
                .unwrap_or(0)
        });

        self.voices[slot].note_on(
            id,
            channel,
            note,
            velocity,
            bend_ratio,
            settings.unison,
            settings.detune,
            settings.blend,
            settings.spread,
            settings.phase_rand,
        );
    }

    pub fn note_off(&mut self, id: Option<i32>, channel: u8, note: u8) {
        for v in &mut self.voices {
            if v.active && (id.is_none() || v.id == id) && v.channel == channel && v.note == note {
                v.note_off();
            }
        }
    }

    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    /// Renders a block of stereo samples with multi-voice unison and phase warp.
    pub fn render_block(
        &mut self,
        settings: &Settings,
        left: &mut [f32],
        right: &mut [f32],
    ) -> Vec<Ended> {
        let mut terminated = Vec::new();
        let table_idx = settings.table_idx.min(self.tables.len().saturating_sub(1));
        let table = &self.tables[table_idx];

        let inv_fs = 1.0 / self.sample_rate;
        let attack_step = (inv_fs / settings.attack.max(0.001) as f64) as f32;
        let decay_step = (inv_fs / settings.decay.max(0.001) as f64) as f32;
        let release_step = (inv_fs / settings.release.max(0.001) as f64) as f32;
        let sustain_target = settings.sustain.clamp(0.0, 1.0);

        let num_samples = left.len().min(right.len());

        for sample_idx in 0..num_samples {
            let mut sum_l = 0.0f32;
            let mut sum_r = 0.0f32;

            for voice in &mut self.voices {
                if !voice.active {
                    continue;
                }

                // Envelope state machine
                match voice.stage {
                    EnvStage::Attack => {
                        voice.env_level += attack_step;
                        if voice.env_level >= 1.0 {
                            voice.env_level = 1.0;
                            voice.stage = EnvStage::Decay;
                        }
                    }
                    EnvStage::Decay => {
                        voice.env_level -= decay_step;
                        if voice.env_level <= sustain_target {
                            voice.env_level = sustain_target;
                            voice.stage = EnvStage::Sustain;
                        }
                    }
                    EnvStage::Sustain => {
                        voice.env_level = sustain_target;
                    }
                    EnvStage::Release => {
                        voice.env_level -= release_step;
                        if voice.env_level <= 0.0 {
                            voice.env_level = 0.0;
                            voice.active = false;
                            voice.stage = EnvStage::Off;
                            terminated.push(Ended {
                                id: voice.id,
                                channel: voice.channel,
                                note: voice.note,
                            });
                            continue;
                        }
                    }
                    EnvStage::Off => continue,
                }

                let voice_gain = voice.env_level * voice.velocity * settings.level;

                // Render all unison subvoices
                for u in 0..voice.unison_count {
                    let sub = &mut voice.unison_voices[u];
                    let freq_ratio = (voice.base_freq * sub.detune_ratio) / self.sample_rate;

                    // Apply non-linear phase warp (Sync, Bend, PWM, etc.)
                    let warped_phase = warp::warp_phase(sub.phase, settings.warp_mode, settings.warp_amt);

                    let osc_sample = table.sample(settings.wt_pos, warped_phase, freq_ratio);

                    sum_l += osc_sample * sub.pan_l * sub.gain * voice_gain;
                    sum_r += osc_sample * sub.pan_r * sub.gain * voice_gain;

                    // Advance un-warped phase by frequency ratio
                    sub.phase = (sub.phase + freq_ratio).fract();
                }
            }

            // Soft saturation limiter to protect dynamic range and prevent hard clipping
            left[sample_idx] += sum_l.tanh();
            right[sample_idx] += sum_r.tanh();
        }

        terminated
    }
}

#[inline]
pub fn midi_note_to_freq(note: u8) -> f64 {
    440.0 * 2.0f64.powf((note as f64 - 69.0) / 12.0)
}

#[inline]
fn pseudo_rand(seed1: u8, seed2: usize) -> f64 {
    let mut h = (seed1 as u32).wrapping_mul(0x45d9f3b).wrapping_add(seed2 as u32);
    h = ((h >> 16) ^ h).wrapping_mul(0x45d9f3b);
    h = (h >> 16) ^ h;
    (h as f64) / (u32::MAX as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unison_renders_wide_stereo() {
        let mut engine = Engine::new(48000.0);
        let mut settings = Settings::default();
        settings.unison = 7;
        settings.detune = 0.25;
        settings.spread = 1.0;

        engine.note_on(Some(1), 0, 60, 0.8, &settings);
        assert_eq!(engine.active_voice_count(), 1);
        assert_eq!(engine.voices[0].unison_count, 7);

        let mut left = vec![0.0; 512];
        let mut right = vec![0.0; 512];
        engine.render_block(&settings, &mut left, &mut right);

        // In 7-voice unison with stereo spread, left and right channels should be decorrelated
        let diff_sum: f32 = left.iter().zip(right.iter()).map(|(l, r)| (l - r).abs()).sum();
        assert!(diff_sum > 0.5, "unison should produce wide decorrelated stereo difference: {diff_sum}");
    }

    #[test]
    fn warp_sync_alters_audio_timbre() {
        let mut engine = Engine::new(48000.0);
        let mut settings_dry = Settings::default();
        settings_dry.unison = 1;
        settings_dry.warp_mode = WarpMode::None;

        engine.note_on(Some(1), 0, 60, 0.8, &settings_dry);
        let mut left_dry = vec![0.0; 512];
        let mut right_dry = vec![0.0; 512];
        engine.render_block(&settings_dry, &mut left_dry, &mut right_dry);

        engine.reset();

        let mut settings_warped = settings_dry;
        settings_warped.warp_mode = WarpMode::Sync;
        settings_warped.warp_amt = 0.75; // 75% hard sync

        engine.note_on(Some(1), 0, 60, 0.8, &settings_warped);
        let mut left_warped = vec![0.0; 512];
        let mut right_warped = vec![0.0; 512];
        engine.render_block(&settings_warped, &mut left_warped, &mut right_warped);

        // Warped sound must clearly differ from unwarped sound
        let diff: f32 = left_dry.iter().zip(left_warped.iter()).map(|(a, b)| (a - b).abs()).sum();
        assert!(diff > 1.0, "hard sync should substantially alter spectral shape: {diff}");
    }
}
