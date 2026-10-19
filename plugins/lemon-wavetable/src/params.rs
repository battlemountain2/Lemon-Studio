use nih_plug::prelude::*;
use os_plugkit::live::LiveEdits;
use std::sync::Arc;

use crate::dsp::Settings;

pub const IDS: [&str; 7] = ["wt_pos", "table", "attack", "decay", "sustain", "release", "output"];

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
        }
    }
}
