use std::f64::consts::{FRAC_PI_4, TAU};

pub const MAX_VOICES: usize = 32;
pub const RELEASE: f64 = 0.012;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub sweep: f64,
    pub length: f64,
    pub bite: f64,
    pub body: f64,
    pub gain: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self::from_values([36.0, 280.0, 65.0, 30.0, -12.0])
    }
}

impl Settings {
    pub fn from_values(values: [f32; 5]) -> Self {
        let safe =
            |index: usize, min: f32, max: f32, fallback: f32| f64::from(if values[index].is_finite() { values[index].clamp(min, max) } else { fallback });
        Self {
            sweep: safe(0, 0.0, 48.0, 36.0),
            length: safe(1, 60.0, 1200.0, 280.0) / 1000.0,
            bite: safe(2, 0.0, 100.0, 65.0) / 100.0,
            body: safe(3, 0.0, 100.0, 30.0) / 100.0,
            gain: 10f64.powf(safe(4, -24.0, 0.0, -12.0) / 20.0),
        }
    }

    fn approach(&mut self, target: Self, coefficient: f64) {
        self.sweep += (target.sweep - self.sweep) * coefficient;
        self.length += (target.length - self.length) * coefficient;
        self.bite += (target.bite - self.bite) * coefficient;
        self.body += (target.body - self.body) * coefficient;
        self.gain += (target.gain - self.gain) * coefficient;
    }
}

pub fn pitch_drop(seconds: f64, settings: Settings) -> f64 {
    let progress = (seconds / settings.length).clamp(0.0, 1.0);
    settings.sweep * ((-5.0 * progress).exp() - (-5f64).exp()) / (1.0 - (-5f64).exp())
}

pub fn amplitude(seconds: f64, settings: Settings) -> f64 {
    let progress = (seconds / settings.length).clamp(0.0, 1.0);
    let attack = (seconds / 0.003).clamp(0.0, 1.0);
    attack * attack * (3.0 - 2.0 * attack) * (1.0 - progress).powi(2)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ended {
    pub note: u8,
    pub channel: u8,
    pub id: Option<i32>,
}

#[derive(Clone, Copy)]
struct Voice {
    identity: Ended,
    velocity: f64,
    age: f64,
    phase: f64,
    filter: [f64; 2],
    release: Option<(f64, f64)>,
    last_amplitude: f64,
    pan: f64,
    tuning: f64,
}

impl Voice {
    fn matches(&self, channel: u8, note: u8, id: Option<i32>) -> bool {
        self.identity.channel == channel && self.identity.note == note && id.is_none_or(|id| self.identity.id == Some(id))
    }
}

pub struct Engine {
    rate: f64,
    voices: [Option<Voice>; MAX_VOICES],
    bend: [f64; 16],
    current: Option<Settings>,
}

impl Engine {
    pub fn new(rate: f32) -> Self {
        Self { rate: f64::from(rate.max(8000.0)), voices: [None; MAX_VOICES], bend: [0.0; 16], current: None }
    }

    pub fn reset(&mut self) {
        self.voices.fill(None);
        self.bend.fill(0.0);
        self.current = None;
    }

    pub fn active(&self) -> usize {
        self.voices.iter().flatten().count()
    }

    pub fn progress(&self, settings: Settings) -> f32 {
        self.voices.iter().flatten().map(|voice| voice.age).min_by(f64::total_cmp).map_or(-1.0, |age| (age / settings.length).clamp(0.0, 1.0) as f32)
    }

    pub fn note_on(&mut self, channel: u8, note: u8, velocity: f32, id: Option<i32>) -> Option<Ended> {
        if channel >= 16 || note >= 128 || !velocity.is_finite() || velocity <= 0.0 {
            self.note_off(channel, note, id);
            return None;
        }
        let index = self
            .voices
            .iter()
            .position(Option::is_none)
            .unwrap_or_else(|| self.voices.iter().enumerate().max_by(|(_, left), (_, right)| left.unwrap().age.total_cmp(&right.unwrap().age)).unwrap().0);
        let ended = self.voices[index].map(|voice| voice.identity);
        self.voices[index] = Some(Voice {
            identity: Ended { channel, note, id },
            velocity: f64::from(velocity.min(1.0)),
            age: 0.0,
            phase: 0.0,
            filter: [0.0; 2],
            release: None,
            last_amplitude: 0.0,
            pan: 0.0,
            tuning: 0.0,
        });
        ended
    }

    pub fn note_off(&mut self, channel: u8, note: u8, id: Option<i32>) {
        for voice in self.voices.iter_mut().flatten().filter(|voice| voice.matches(channel, note, id)) {
            voice.release.get_or_insert((0.0, voice.last_amplitude));
        }
    }

    pub fn all_off(&mut self, channel: u8) {
        for voice in self.voices.iter_mut().flatten().filter(|voice| voice.identity.channel == channel) {
            voice.release.get_or_insert((0.0, voice.last_amplitude));
        }
    }

    pub fn choke(&mut self, channel: u8, note: Option<u8>, id: Option<i32>, mut ended: impl FnMut(Ended)) {
        for slot in &mut self.voices {
            if slot.is_some_and(|voice| voice.identity.channel == channel && note.is_none_or(|note| voice.matches(channel, note, id))) {
                ended(slot.take().unwrap().identity);
            }
        }
    }

    pub fn bend(&mut self, channel: u8, normalized: f32) {
        if channel < 16 && normalized.is_finite() {
            self.bend[usize::from(channel)] = f64::from(normalized.clamp(0.0, 1.0) * 4.0 - 2.0);
        }
    }

    pub fn expression(&mut self, channel: u8, note: u8, id: Option<i32>, pan: Option<f32>, tuning: Option<f32>) {
        for voice in self.voices.iter_mut().flatten().filter(|voice| voice.matches(channel, note, id)) {
            if let Some(pan) = pan.filter(|pan| pan.is_finite()) {
                voice.pan = f64::from(pan.clamp(-1.0, 1.0));
            }
            if let Some(tuning) = tuning.filter(|tuning| tuning.is_finite()) {
                voice.tuning = f64::from(tuning.clamp(-48.0, 48.0));
            }
        }
    }

    pub fn render(&mut self, left: &mut [f32], right: &mut [f32], target: Settings, mut ended: impl FnMut(usize, Ended)) {
        let coefficient = 1.0 - (-1.0 / (self.rate * 0.005)).exp();
        let mut settings = self.current.unwrap_or(target);
        let step = 1.0 / self.rate;
        let lowpass = 1.0 - (-TAU * 0.4 / 4.0).exp();
        for (frame, (left, right)) in left.iter_mut().zip(right).enumerate() {
            settings.approach(target, coefficient);
            let mut stereo = [0.0; 2];
            for slot in &mut self.voices {
                let Some(voice) = slot.as_mut() else { continue };
                let level = if let Some((elapsed, start)) = &mut voice.release {
                    let level = *start * (1.0 - *elapsed / RELEASE).clamp(0.0, 1.0).powi(2);
                    *elapsed += step;
                    level
                } else {
                    amplitude(voice.age, settings)
                };
                if voice.age >= settings.length || voice.release.is_some_and(|(elapsed, _)| elapsed >= RELEASE) {
                    ended(frame, slot.take().unwrap().identity);
                    continue;
                }
                voice.last_amplitude = level;
                let semitones =
                    f64::from(voice.identity.note) - 69.0 + voice.tuning + self.bend[usize::from(voice.identity.channel)] + pitch_drop(voice.age, settings);
                let frequency = (440.0 * 2f64.powf(semitones / 12.0)).min(self.rate * 0.42);
                let fm = (settings.bite * 5.0).min((self.rate * 1.5 / frequency - 1.0).max(0.0) / 2.0);
                let mut sample = 0.0;
                for _ in 0..4 {
                    let carrier = (voice.phase + fm * (voice.phase * 2.0).sin()).sin();
                    let sub = (voice.phase * 0.5).sin();
                    let raw = carrier * (1.0 - settings.body * 0.55) + sub * settings.body * 0.55;
                    voice.filter[0] += lowpass * (raw - voice.filter[0]);
                    voice.filter[1] += lowpass * (voice.filter[0] - voice.filter[1]);
                    sample += voice.filter[1] * 0.25;
                    voice.phase = (voice.phase + TAU * frequency * step / 4.0).rem_euclid(TAU * 2.0);
                }
                let sample = sample * level * voice.velocity * settings.gain * 0.35;
                let pan = (voice.pan + 1.0) * FRAC_PI_4;
                stereo[0] += sample * pan.cos();
                stereo[1] += sample * pan.sin();
                voice.age += step;
            }
            *left = stereo[0].tanh() as f32;
            *right = stereo[1].tanh() as f32;
        }
        self.current = Some(settings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(engine: &mut Engine, frames: usize, settings: Settings) -> (Vec<f32>, Vec<Ended>) {
        let mut left = vec![0.0; frames];
        let mut right = vec![0.0; frames];
        let mut ended = Vec::new();
        engine.render(&mut left, &mut right, settings, |_, identity| ended.push(identity));
        assert!(left.iter().chain(&right).all(|value| value.is_finite() && value.abs() <= 1.0));
        (left, ended)
    }

    #[test]
    fn envelope_matches_the_canvas_and_finishes_at_root() {
        let settings = Settings::default();
        assert!((pitch_drop(0.0, settings) - 36.0).abs() < 1e-10);
        assert_eq!(pitch_drop(settings.length, settings), 0.0);
        assert!(pitch_drop(0.05, settings) > pitch_drop(0.1, settings));
        assert_eq!(amplitude(0.0, settings), 0.0);
        assert_eq!(amplitude(settings.length, settings), 0.0);
    }

    #[test]
    fn finite_bounded_audio_and_natural_termination_at_multiple_rates() {
        for rate in [8000.0, 44100.0, 48000.0, 96000.0] {
            let mut engine = Engine::new(rate);
            for note in [0, 36, 60, 100, 127] {
                engine.note_on(0, note, 1.0, Some(i32::from(note)));
            }
            let (audio, ended) = run(&mut engine, rate as usize * 2, Settings::from_values([48.0, 1200.0, 100.0, 100.0, 0.0]));
            assert!(audio.iter().any(|value| value.abs() > 0.01));
            assert_eq!(ended.len(), 5);
            assert_eq!(engine.active(), 0);
            assert!(audio[audio.len() - 100..].iter().all(|value| *value == 0.0));
        }
    }

    #[test]
    fn releases_are_short_channel_and_voice_specific_and_reset_is_silent() {
        let mut engine = Engine::new(48000.0);
        engine.note_on(0, 60, 0.8, Some(1));
        engine.note_on(0, 60, 0.8, Some(2));
        engine.note_on(1, 60, 0.8, Some(3));
        run(&mut engine, 480, Settings::default());
        engine.note_off(0, 60, Some(1));
        let (_, ended) = run(&mut engine, 960, Settings::default());
        assert_eq!(ended, [Ended { channel: 0, note: 60, id: Some(1) }]);
        assert_eq!(engine.active(), 2);
        engine.all_off(0);
        assert_eq!(run(&mut engine, 960, Settings::default()).1.len(), 1);
        assert_eq!(engine.active(), 1);
        engine.reset();
        assert!(run(&mut engine, 480, Settings::default()).0.iter().all(|value| *value == 0.0));
    }

    #[test]
    fn standard_midi_tuning_and_controls_change_the_sound() {
        let render = |values| {
            let mut engine = Engine::new(48000.0);
            engine.note_on(0, 69, 1.0, None);
            run(&mut engine, 4800, Settings::from_values(values)).0
        };
        let sine = render([0.0, 1200.0, 0.0, 0.0, -12.0]);
        let crossings = sine[480..].windows(2).filter(|pair| pair[0] < 0.0 && pair[1] >= 0.0).count();
        assert!((38..=40).contains(&crossings), "A4 = 440 Hz: {crossings}");
        assert_ne!(sine, render([0.0, 1200.0, 80.0, 0.0, -12.0]));
        assert_ne!(sine, render([0.0, 1200.0, 0.0, 80.0, -12.0]));
        let quiet = render([0.0, 1200.0, 0.0, 0.0, -24.0]);
        assert!(quiet.iter().map(|value| value.abs()).fold(0.0, f32::max) < sine.iter().map(|value| value.abs()).fold(0.0, f32::max) * 0.3);
    }

    #[test]
    fn voice_limit_choke_and_invalid_input_are_safe() {
        let mut engine = Engine::new(48000.0);
        for index in 0..MAX_VOICES {
            assert_eq!(engine.note_on(0, 60, 0.8, Some(index as i32)), None);
        }
        assert!(engine.note_on(0, 61, 0.8, Some(99)).is_some());
        assert_eq!(engine.active(), MAX_VOICES);
        let mut ended = Vec::new();
        engine.choke(0, None, None, |identity| ended.push(identity));
        assert_eq!(ended.len(), MAX_VOICES);
        assert_eq!(engine.active(), 0);
        engine.note_on(16, 60, f32::NAN, None);
        engine.note_on(0, 60, 0.0, None);
        assert_eq!(engine.active(), 0);
        assert!(Settings::from_values([f32::NAN; 5]).gain.is_finite());
    }
}
