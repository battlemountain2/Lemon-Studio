use nih_plug::prelude::*;
use os_plugkit::live::LiveEdits;
use std::sync::Arc;

use crate::dsp::Settings;

pub const IDS: [&str; 5] = ["sweep", "length", "bite", "body", "output"];
pub const NAMES: [&str; 5] = ["Sweep", "Length", "Bite", "Body", "Output"];
pub const HINTS: [&str; 5] = ["Pitch drop", "Tail duration", "Metallic edge", "Low-end weight", "Final level"];
pub const PRESETS: [(&str, [f32; 5]); 3] =
    [("Arc zap", [36.0, 280.0, 65.0, 30.0, -12.0]), ("Soft ray", [12.0, 650.0, 14.0, 60.0, -12.0]), ("Glass ping", [0.0, 900.0, 45.0, 15.0, -12.0])];

#[derive(Params)]
pub struct RayParams {
    #[id = "sweep"]
    pub sweep: FloatParam,
    #[id = "length"]
    pub length: FloatParam,
    #[id = "bite"]
    pub bite: FloatParam,
    #[id = "body"]
    pub body: FloatParam,
    #[id = "output"]
    pub output: FloatParam,
    pub live: Arc<LiveEdits>,
}

impl Default for RayParams {
    fn default() -> Self {
        let parameter = |name, default, min, max, unit| {
            FloatParam::new(name, default, FloatRange::Linear { min, max }).with_unit(unit).with_value_to_string(Arc::new(|value| format!("{value:.0}")))
        };
        Self {
            sweep: parameter("Sweep", 36.0, 0.0, 48.0, " st"),
            length: parameter("Length", 280.0, 60.0, 1200.0, " ms"),
            bite: parameter("Bite", 65.0, 0.0, 100.0, " %"),
            body: parameter("Body", 30.0, 0.0, 100.0, " %"),
            output: parameter("Output", -12.0, -24.0, 0.0, " dB"),
            live: Arc::new(LiveEdits::default()),
        }
    }
}

impl RayParams {
    pub fn settings(&self) -> Settings {
        Settings::from_values([
            self.live.value(&self.sweep),
            self.live.value(&self.length),
            self.live.value(&self.bite),
            self.live.value(&self.body),
            self.live.value(&self.output),
        ])
    }
}
