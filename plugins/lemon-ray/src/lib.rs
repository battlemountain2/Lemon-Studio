#![allow(semicolon_in_expressions_from_non_local_macros)]

pub mod dsp;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod editor;
pub mod params;
pub mod view;

use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering, fence};

use nih_plug::prelude::*;
use params::RayParams;

pub const CLAP_ID: &str = "org.lemonstudio.lemon-ray";
pub const SCOPE_SAMPLES: usize = 512;

pub struct Shared {
    sequence: AtomicU64,
    waveform: [AtomicU32; SCOPE_SAMPLES],
    progress: AtomicU32,
    voices: AtomicU32,
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            sequence: AtomicU64::new(0),
            waveform: std::array::from_fn(|_| AtomicU32::new(0)),
            progress: AtomicU32::new((-1.0f32).to_bits()),
            voices: AtomicU32::new(0),
        }
    }
}

#[derive(Clone)]
pub struct Scope {
    pub waveform: [f32; SCOPE_SAMPLES],
    pub progress: f32,
    pub voices: u32,
}

impl Default for Scope {
    fn default() -> Self {
        Self { waveform: [0.0; SCOPE_SAMPLES], progress: -1.0, voices: 0 }
    }
}

impl Shared {
    fn publish(&self, waveform: &[f32; SCOPE_SAMPLES], offset: usize, progress: f32, voices: usize) {
        self.sequence.fetch_add(1, Ordering::AcqRel);
        for (index, value) in self.waveform.iter().enumerate() {
            value.store(waveform[(offset + index) % SCOPE_SAMPLES].to_bits(), Ordering::Relaxed);
        }
        self.progress.store(progress.to_bits(), Ordering::Relaxed);
        self.voices.store(voices as u32, Ordering::Relaxed);
        self.sequence.fetch_add(1, Ordering::Release);
    }

    pub fn snapshot(&self) -> Option<(u64, Scope)> {
        let sequence = self.sequence.load(Ordering::Acquire);
        if sequence & 1 != 0 {
            return None;
        }
        let scope = Scope {
            waveform: std::array::from_fn(|index| f32::from_bits(self.waveform[index].load(Ordering::Relaxed))),
            progress: f32::from_bits(self.progress.load(Ordering::Relaxed)),
            voices: self.voices.load(Ordering::Relaxed),
        };
        fence(Ordering::Acquire);
        (sequence == self.sequence.load(Ordering::Relaxed)).then_some((sequence, scope))
    }
}

pub struct LemonRay {
    params: Arc<RayParams>,
    shared: Arc<Shared>,
    engine: dsp::Engine,
    waveform: [f32; SCOPE_SAMPLES],
    offset: usize,
}

impl Default for LemonRay {
    fn default() -> Self {
        Self {
            params: Arc::new(RayParams::default()),
            shared: Arc::new(Shared::default()),
            engine: dsp::Engine::new(48000.0),
            waveform: [0.0; SCOPE_SAMPLES],
            offset: 0,
        }
    }
}

fn terminated(timing: usize, identity: dsp::Ended) -> NoteEvent<()> {
    NoteEvent::VoiceTerminated { timing: timing as u32, voice_id: identity.id, channel: identity.channel, note: identity.note }
}

impl Plugin for LemonRay {
    const NAME: &'static str = "Lemon Ray";
    const VENDOR: &'static str = "Lemon Studio";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] =
        &[AudioIOLayout { main_input_channels: None, main_output_channels: NonZeroU32::new(2), ..AudioIOLayout::const_default() }];
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            Some(Box::new(editor::RayEditor::new(self.params.clone(), self.shared.clone())))
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            None
        }
    }

    fn initialize(&mut self, _layout: &AudioIOLayout, config: &BufferConfig, _context: &mut impl InitContext<Self>) -> bool {
        if !config.sample_rate.is_finite() || config.sample_rate < 8000.0 {
            return false;
        }
        self.engine = dsp::Engine::new(config.sample_rate);
        self.reset();
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
        self.waveform.fill(0.0);
        self.offset = 0;
        self.shared.publish(&self.waveform, 0, -1.0, 0);
    }

    fn process(&mut self, buffer: &mut Buffer, _aux: &mut AuxiliaryBuffers, context: &mut impl ProcessContext<Self>) -> ProcessStatus {
        let settings = self.params.settings();
        let frames = buffer.samples();
        let output = buffer.as_slice();
        for channel in output.iter_mut() {
            channel.fill(0.0);
        }
        if output.len() < 2 {
            return ProcessStatus::Normal;
        }
        let (left, right) = output.split_at_mut(1);
        let mut position = 0;
        while let Some(event) = context.next_event() {
            let timing = (event.timing() as usize).clamp(position, frames);
            self.engine.render(&mut left[0][position..timing], &mut right[0][position..timing], settings, |frame, identity| {
                context.send_event(terminated(position + frame, identity))
            });
            position = timing;
            match event {
                NoteEvent::NoteOn { channel, note, velocity, voice_id, .. } => {
                    if let Some(identity) = self.engine.note_on(channel, note, velocity, voice_id) {
                        context.send_event(terminated(timing, identity));
                    }
                }
                NoteEvent::NoteOff { channel, note, voice_id, .. } => self.engine.note_off(channel, note, voice_id),
                NoteEvent::Choke { channel, note, voice_id, .. } => {
                    self.engine.choke(channel, Some(note), voice_id, |identity| context.send_event(terminated(timing, identity)))
                }
                NoteEvent::MidiPitchBend { channel, value, .. } => self.engine.bend(channel, value),
                NoteEvent::MidiCC { channel, cc: 123, .. } => self.engine.all_off(channel),
                NoteEvent::MidiCC { channel, cc: 120, .. } => {
                    self.engine.choke(channel, None, None, |identity| context.send_event(terminated(timing, identity)))
                }
                NoteEvent::PolyPan { channel, note, voice_id, pan, .. } => self.engine.expression(channel, note, voice_id, Some(pan), None),
                NoteEvent::PolyTuning { channel, note, voice_id, tuning, .. } => self.engine.expression(channel, note, voice_id, None, Some(tuning)),
                _ => {}
            }
        }
        self.engine.render(&mut left[0][position..frames], &mut right[0][position..frames], settings, |frame, identity| {
            context.send_event(terminated(position + frame, identity))
        });
        for (left, right) in left[0].iter().zip(&*right[0]) {
            self.waveform[self.offset] = (left + right) * 0.5;
            self.offset = (self.offset + 1) % SCOPE_SAMPLES;
        }
        self.shared.publish(&self.waveform, self.offset, self.engine.progress(settings), self.engine.active());
        if self.engine.active() > 0 { ProcessStatus::KeepAlive } else { ProcessStatus::Normal }
    }
}

impl ClapPlugin for LemonRay {
    const CLAP_ID: &'static str = CLAP_ID;
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Visual pitch-envelope and FM laser instrument");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Instrument, ClapFeature::Synthesizer, ClapFeature::Stereo];
}

impl Vst3Plugin for LemonRay {
    const VST3_CLASS_ID: [u8; 16] = *b"LemonStudioRay01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

#[cfg(not(feature = "linked"))]
nih_export_clap!(LemonRay);
#[cfg(not(feature = "linked"))]
nih_export_vst3!(LemonRay);
os_plugkit::linked_clap_entry!(LemonRay);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_is_audio_not_a_generated_decoration_and_reset_clears_it() {
        let mut plugin = LemonRay::default();
        plugin.waveform[5] = 0.25;
        plugin.shared.publish(&plugin.waveform, 3, 0.5, 2);
        let (_, scope) = plugin.shared.snapshot().unwrap();
        assert_eq!(scope.waveform[2], 0.25);
        assert_eq!((scope.progress, scope.voices), (0.5, 2));
        plugin.reset();
        let (_, scope) = plugin.shared.snapshot().unwrap();
        assert_eq!(scope.waveform, [0.0; SCOPE_SAMPLES]);
        assert_eq!((scope.progress, scope.voices), (-1.0, 0));
    }
}
