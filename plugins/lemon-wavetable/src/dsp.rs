//! Polyphonic DSP engine and voice management for Lemon Wavetable.

use crate::table::Wavetable;

pub const MAX_VOICES: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EnvStage {
    Off,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Copy, Debug)]
pub struct Voice {
    pub id: Option<i32>,
    pub channel: u8,
    pub note: u8,
    pub velocity: f32,
    pub phase: f64,
    pub freq: f64,
    pub active: bool,
    pub stage: EnvStage,
    pub env_level: f32,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            id: None,
            channel: 0,
            note: 60,
            velocity: 0.0,
            phase: 0.0,
            freq: 440.0,
            active: false,
            stage: EnvStage::Off,
            env_level: 0.0,
        }
    }
}

impl Voice {
    pub fn note_on(&mut self, id: Option<i32>, channel: u8, note: u8, velocity: f32, bend_ratio: f64) {
        self.id = id;
        self.channel = channel;
        self.note = note;
        self.velocity = velocity.clamp(0.0, 1.0);
        self.phase = 0.0;
        self.freq = midi_note_to_freq(note) * bend_ratio;
        self.active = true;
        self.stage = EnvStage::Attack;
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
        self.phase = 0.0;
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
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            wt_pos: 0.32,
            table_idx: 0,
            level: 0.25, // -12 dB default
            attack: 0.012,
            decay: 0.450,
            sustain: 0.70,
            release: 0.250,
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
        // -1.0 to +1.0 mapped to ±2 semitones
        self.bend_semitones = (bend_norm as f64).clamp(-1.0, 1.0) * 2.0;
        let bend_ratio = 2.0f64.powf(self.bend_semitones / 12.0);
        for v in &mut self.voices {
            if v.active {
                v.freq = midi_note_to_freq(v.note) * bend_ratio;
            }
        }
    }

    pub fn note_on(&mut self, id: Option<i32>, channel: u8, note: u8, velocity: f32) {
        let bend_ratio = 2.0f64.powf(self.bend_semitones / 12.0);

        // Find inactive voice, or steal oldest/quietest
        let slot = self.voices.iter().position(|v| !v.active).unwrap_or_else(|| {
            // Find voice in release with lowest level, or slot 0
            self.voices
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.env_level.partial_cmp(&b.env_level).unwrap())
                .map(|(i, _)| i)
                .unwrap_or(0)
        });

        self.voices[slot].note_on(id, channel, note, velocity, bend_ratio);
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

    /// Renders a block of stereo samples and returns any voice terminations that finished releasing.
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

                // Sample anti-aliased wavetable
                let freq_ratio = voice.freq / self.sample_rate;
                let osc_sample = table.sample(settings.wt_pos, voice.phase, freq_ratio);

                // Advance phase
                voice.phase = (voice.phase + freq_ratio).fract();

                let gain = voice.env_level * voice.velocity * settings.level;
                let voice_out = osc_sample * gain;

                sum_l += voice_out;
                sum_r += voice_out;
            }

            // Soft saturation limiter to avoid hard clipping
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_note_on_renders_sound() {
        let mut engine = Engine::new(48000.0);
        let settings = Settings::default();

        assert_eq!(engine.active_voice_count(), 0);
        engine.note_on(Some(1), 0, 60, 0.8); // C4
        assert_eq!(engine.active_voice_count(), 1);

        let mut left = vec![0.0; 512];
        let mut right = vec![0.0; 512];
        engine.render_block(&settings, &mut left, &mut right);

        // Sound must be non-zero and finite
        assert!(left.iter().any(|&s| s.abs() > 0.001));
        assert!(left.iter().chain(right.iter()).all(|&s| s.is_finite()));
    }

    #[test]
    fn engine_note_off_releases_cleanly() {
        let mut engine = Engine::new(48000.0);
        let mut settings = Settings::default();
        settings.release = 0.01; // 10ms fast release

        engine.note_on(Some(1), 0, 60, 0.8);
        let mut left = vec![0.0; 512];
        let mut right = vec![0.0; 512];
        engine.render_block(&settings, &mut left, &mut right);

        engine.note_off(Some(1), 0, 60);

        // Render until voice terminates
        let mut ended = Vec::new();
        for _ in 0..10 {
            ended.extend(engine.render_block(&settings, &mut left, &mut right));
        }

        assert_eq!(ended.len(), 1);
        assert_eq!(engine.active_voice_count(), 0);
    }
}
