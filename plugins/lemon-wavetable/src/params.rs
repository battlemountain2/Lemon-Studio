use nih_plug::prelude::*;
use os_plugkit::live::LiveEdits;
use std::sync::Arc;

use crate::dsp::Settings;
use crate::warp::WarpMode;

pub const IDS: [&str; 14] = [
    "wt_pos", "table", "attack", "decay", "sustain", "release", "output",
    "unison", "detune", "blend", "spread", "phase_rand", "warp_mode", "warp_amt",
];

#[derive(Params)]
pub struct WavetableParams {
    #[id = "wt_pos"]
    pub wt_pos: FloatParam,

    #[id = "table"]
    pub table: IntParam,

    #[id = "attack"]
    pub attack: FloatParam,

    #[id = "decay"]
    pub decay: FloatParam,

    #[id = "sustain"]
    pub sustain: FloatParam,

    #[id = "release"]
    pub release: FloatParam,

    #[id = "output"]
    pub output: FloatParam,

    // --- Milestone 2: Unison & Warp ---
    #[id = "unison"]
    pub unison: IntParam,

    #[id = "detune"]
    pub detune: FloatParam,

    #[id = "blend"]
    pub blend: FloatParam,

    #[id = "spread"]
    pub spread: FloatParam,

    #[id = "phase_rand"]
    pub phase_rand: FloatParam,

    #[id = "warp_mode"]
    pub warp_mode: IntParam,

    #[id = "warp_amt"]
    pub warp_amt: FloatParam,

    pub live: Arc<LiveEdits>,
}

impl Default for WavetableParams {
    fn default() -> Self {
        Self {
            wt_pos: FloatParam::new("WT Pos", 32.0, FloatRange::Linear { min: 0.0, max: 100.0 })
                .with_unit(" %")
                .with_value_to_string(Arc::new(|v| format!("{v:.0}"))),

            table: IntParam::new("Wavetable", 0, IntRange::Linear { min: 0, max: 2 })
                .with_value_to_string(Arc::new(|v| match v {
                    0 => "Basic Shapes".to_string(),
                    1 => "Analog Saw".to_string(),
                    2 => "Digital Bell".to_string(),
                    _ => format!("{v}"),
                })),

            attack: FloatParam::new("Attack", 12.0, FloatRange::Skewed { min: 1.0, max: 2000.0, factor: 0.25 })
                .with_unit(" ms")
                .with_value_to_string(Arc::new(|v| format!("{v:.0}"))),

            decay: FloatParam::new("Decay", 450.0, FloatRange::Skewed { min: 10.0, max: 3000.0, factor: 0.35 })
                .with_unit(" ms")
                .with_value_to_string(Arc::new(|v| format!("{v:.0}"))),

            sustain: FloatParam::new("Sustain", 70.0, FloatRange::Linear { min: 0.0, max: 100.0 })
                .with_unit(" %")
                .with_value_to_string(Arc::new(|v| format!("{v:.0}"))),

            release: FloatParam::new("Release", 250.0, FloatRange::Skewed { min: 10.0, max: 5000.0, factor: 0.35 })
                .with_unit(" ms")
                .with_value_to_string(Arc::new(|v| format!("{v:.0}"))),

            output: FloatParam::new("Output", -12.0, FloatRange::Linear { min: -24.0, max: 0.0 })
                .with_unit(" dB")
                .with_value_to_string(Arc::new(|v| format!("{v:.1}"))),

            unison: IntParam::new("Unison Voices", 7, IntRange::Linear { min: 1, max: 16 }),

            detune: FloatParam::new("Detune", 18.0, FloatRange::Linear { min: 0.0, max: 100.0 })
                .with_unit(" %")
                .with_value_to_string(Arc::new(|v| format!("{v:.0}"))),

            blend: FloatParam::new("Unison Blend", 80.0, FloatRange::Linear { min: 0.0, max: 100.0 })
                .with_unit(" %")
                .with_value_to_string(Arc::new(|v| format!("{v:.0}"))),

            spread: FloatParam::new("Stereo Spread", 100.0, FloatRange::Linear { min: 0.0, max: 100.0 })
                .with_unit(" %")
                .with_value_to_string(Arc::new(|v| format!("{v:.0}"))),

            phase_rand: FloatParam::new("Phase Random", 100.0, FloatRange::Linear { min: 0.0, max: 100.0 })
                .with_unit(" %")
                .with_value_to_string(Arc::new(|v| format!("{v:.0}"))),

            warp_mode: IntParam::new("Warp Mode", 0, IntRange::Linear { min: 0, max: 6 })
                .with_value_to_string(Arc::new(|v| match v {
                    1 => "Sync".to_string(),
                    2 => "Bend +".to_string(),
                    3 => "Bend -".to_string(),
                    4 => "PWM".to_string(),
                    5 => "Asym +".to_string(),
                    6 => "Flip".to_string(),
                    _ => "Off".to_string(),
                })),

            warp_amt: FloatParam::new("Warp Amount", 0.0, FloatRange::Linear { min: 0.0, max: 100.0 })
                .with_unit(" %")
                .with_value_to_string(Arc::new(|v| format!("{v:.0}"))),

            live: Arc::new(LiveEdits::default()),
        }
    }
}

impl WavetableParams {
    pub fn settings(&self) -> Settings {
        let db = self.live.value(&self.output);
        let linear_gain = 10.0f32.powf(db / 20.0);

        Settings {
            wt_pos: (self.live.value(&self.wt_pos) / 100.0).clamp(0.0, 1.0),
            table_idx: self.table.value() as usize,
            level: linear_gain,
            attack: (self.live.value(&self.attack) / 1000.0).max(0.001),
            decay: (self.live.value(&self.decay) / 1000.0).max(0.001),
            sustain: (self.live.value(&self.sustain) / 100.0).clamp(0.0, 1.0),
            release: (self.live.value(&self.release) / 1000.0).max(0.001),
            unison: self.unison.value() as usize,
            detune: (self.live.value(&self.detune) / 100.0).clamp(0.0, 1.0),
            blend: (self.live.value(&self.blend) / 100.0).clamp(0.0, 1.0),
            spread: (self.live.value(&self.spread) / 100.0).clamp(0.0, 1.0),
            phase_rand: (self.live.value(&self.phase_rand) / 100.0).clamp(0.0, 1.0),
            warp_mode: WarpMode::from(self.warp_mode.value()),
            warp_amt: (self.live.value(&self.warp_amt) / 100.0).clamp(0.0, 1.0),
        }
    }
}
