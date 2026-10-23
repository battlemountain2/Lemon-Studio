#![allow(semicolon_in_expressions_from_non_local_macros)]

pub mod dsp;
pub mod editor;
pub mod env;
pub mod fft;
pub mod filter;
pub mod lfo;
pub mod mod_matrix;
pub mod params;
pub mod table;
pub mod view;
pub mod warp;

use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use nih_plug::prelude::*;
use params::WavetableParams;

pub const CLAP_ID: &str = "org.lemonstudio.lemon-wavetable";

pub struct Shared {
    pub voices: AtomicU32,
}

pub struct LemonWavetable {
    params: Arc<WavetableParams>,
    engine: dsp::Engine,
    shared: Arc<Shared>,
}

impl Default for LemonWavetable {
    fn default() -> Self {
        Self {
            params: Arc::new(WavetableParams::default()),
            engine: dsp::Engine::new(48000.0),
            shared: Arc::new(Shared {
                voices: AtomicU32::new(0),
            }),
        }
    }
}

fn terminated(timing: usize, identity: dsp::Ended) -> NoteEvent<()> {
    NoteEvent::VoiceTerminated {
        timing: timing as u32,
        voice_id: identity.id,
        channel: identity.channel,
        note: identity.note,
    }
}

impl Plugin for LemonWavetable {
    const NAME: &'static str = "Lemon Wavetable";
    const VENDOR: &'static str = "Lemon Studio";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        Some(Box::new(editor::WavetableEditor::new(
            self.params.clone(),
            self.shared.clone(),
        )))
    }

    fn initialize(
        &mut self,
        _layout: &AudioIOLayout,
        config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        if !config.sample_rate.is_finite() || config.sample_rate < 8000.0 {
            return false;
        }
        self.engine = dsp::Engine::new(config.sample_rate as f64);
        self.reset();
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let settings = self.params.settings();
        let num_samples = buffer.samples();
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
            let timing = (event.timing() as usize).clamp(position, num_samples);
            self.engine.render_block(&settings, &mut left[0][position..timing], &mut right[0][position..timing], |sample, identity| {
                context.send_event(terminated(position + sample, identity));
            });
            position = timing;
            match event {
                NoteEvent::NoteOn { voice_id, channel, note, velocity, .. } => {
                    if let Some(identity) = self.engine.note_on(voice_id, channel, note, velocity, &settings) {
                        context.send_event(terminated(timing, identity));
                    }
                }
                NoteEvent::NoteOff { voice_id, channel, note, .. } => self.engine.note_off(voice_id, channel, note),
                NoteEvent::Choke { voice_id, channel, note, .. } => {
                    self.engine.choke(channel, Some(note), voice_id, |identity| context.send_event(terminated(timing, identity)));
                }
                NoteEvent::MidiPitchBend { channel, value, .. } => self.engine.set_pitch_bend(channel, value),
                NoteEvent::MidiCC { channel, cc: 123, .. } => self.engine.all_off(channel),
                NoteEvent::MidiCC { channel, cc: 120, .. } => {
                    self.engine.choke(channel, None, None, |identity| context.send_event(terminated(timing, identity)));
                }
                _ => {}
            }
        }
        self.engine.render_block(&settings, &mut left[0][position..num_samples], &mut right[0][position..num_samples], |sample, identity| {
            context.send_event(terminated(position + sample, identity));
        });

        self.shared
            .voices
            .store(self.engine.active_voice_count() as u32, Ordering::Relaxed);

        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for LemonWavetable {
    const CLAP_ID: &'static str = CLAP_ID;
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Serum-class mipmapped wavetable synthesizer");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for LemonWavetable {
    const VST3_CLASS_ID: [u8; 16] = *b"LemonStudioWave0";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Stereo,
    ];
}

#[cfg(not(feature = "linked"))]
nih_export_clap!(LemonWavetable);
#[cfg(not(feature = "linked"))]
nih_export_vst3!(LemonWavetable);
os_plugkit::linked_clap_entry!(LemonWavetable);
