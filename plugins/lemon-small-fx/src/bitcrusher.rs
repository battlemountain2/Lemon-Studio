// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Lemon Bitcrusher: Lemon Studio's bit depth reduction, sample rate decimator, and saturation overdrive plugin.
//!
//! Features:
//! - Bit depth quantization: 2 to 16 bits (digital lo-fi distortion, retro gaming, hyperpop crunch).
//! - Sample rate decimation (downsampling): 1x to 32x sample-and-hold aliasing.
//! - Input pre-drive: analog-style boost into the quantizer.
//! - Dry / Wet mix: parallel bitcrushing for modern textured production.
//! - Post gain: output level compensation.

use nih_plug::prelude::*;
use os_engine::effects::distortion::{DistMode, Distortion};
use os_ui::{Canvas, Rect};

use crate::params::{db_text, leading_number, nearest, parse_db};
use crate::{view, Card, Control, Display, Kind, Mark, Setting, SmallFx, Sound, Spec};

pub const PARAMS: usize = 5;
pub const BITS: usize = 0;
pub const DOWNSAMPLE: usize = 1;
pub const DRIVE: usize = 2;
pub const MIX: usize = 3;
pub const POST: usize = 4;

pub const RANGES: [(i32, i32); PARAMS] = [
    (2, 16),      // bits: 2..16
    (1, 32),      // downsample factor: 1..32
    (64, 256),    // drive: 0.5x .. 2.0x (128 = 1.0x)
    (0, 128),     // mix: 0..100%
    (0, 192),     // post gain: -inf .. +11dB (128 = 0dB)
];

pub const DEFAULTS: [i32; PARAMS] = [8, 4, 128, 128, 128];

pub const SETTINGS: [Setting; PARAMS] = [
    Setting { id: "bits", name: "Bit depth", range: RANGES[BITS], start: DEFAULTS[BITS], magnet: None },
    Setting { id: "downsample", name: "Downsample", range: RANGES[DOWNSAMPLE], start: DEFAULTS[DOWNSAMPLE], magnet: None },
    Setting { id: "drive", name: "Drive", range: RANGES[DRIVE], start: DEFAULTS[DRIVE], magnet: Some(128) },
    Setting { id: "mix", name: "Mix", range: RANGES[MIX], start: DEFAULTS[MIX], magnet: None },
    Setting { id: "post", name: "Post gain", range: RANGES[POST], start: DEFAULTS[POST], magnet: Some(128) },
];

const CARDS: [Card; 3] = [
    Card { x0: 8.0, x1: 175.0, y0: 28.0, y1: 115.0, heading: "CRUSH" },
    Card { x0: 182.0, x1: 345.0, y0: 28.0, y1: 115.0, heading: "GAIN & MIX" },
    Card { x0: 352.0, x1: 495.0, y0: 28.0, y1: 145.0, heading: "CURVE" },
];

const CONTROLS: [Control; 5] = [
    Control::Knob { id: "bits", cx: 48.0, cy: 64.0, r: 20.0, caption: "BITS", bipolar: false },
    Control::Knob { id: "downsample", cx: 128.0, cy: 64.0, r: 20.0, caption: "DOWNSAMPLE", bipolar: false },
    Control::Knob { id: "drive", cx: 218.0, cy: 64.0, r: 20.0, caption: "DRIVE", bipolar: false },
    Control::Knob { id: "post", cx: 272.0, cy: 64.0, r: 20.0, caption: "POST", bipolar: false },
    Control::Knob { id: "mix", cx: 326.0, cy: 64.0, r: 20.0, caption: "MIX", bipolar: false },
];

pub const SPEC: &Spec = &Spec {
    name: "Lemon Bitcrusher",
    word: "BITCRUSHER",
    about: "Lemon Studio's lo-fi bitcrusher and sample decimator",
    settings: &SETTINGS,
    version: None,
    any_version: false,
    fields: &[],
    tail: &[],
    blank: None,
    locate: None,
    size: (504, 154),
    cards: &CARDS,
    controls: &CONTROLS,
    display: Some(Display { r: Rect::new(360.0, 36.0, 126.0, 102.0), draw: curve }),
    mark: Mark { x: 346.0, y: 135.0, anchor: 1.0, stacked: false },
    hint: None,
    labels: &[],
    text,
    parse,
    sound,
    rings: false,
};

fn knob_gain(v: i32) -> f32 {
    os_dsp::level::curve(v as f32 * 0.0078125)
}

fn text(k: usize, v: i32) -> String {
    match k {
        BITS => format!("{v} bits"),
        DOWNSAMPLE => format!("{v}x"),
        DRIVE => format!("{:.2}x", knob_gain(v)),
        MIX => format!("{:.1}%", (v as f64 / 128.0) * 100.0),
        _ => db_text(knob_gain(v)),
    }
}

fn parse(k: usize, s: &str) -> Option<i32> {
    let range = SETTINGS[k].range;
    match k {
        BITS | DOWNSAMPLE => leading_number(s).map(|x| (x.round() as i32).clamp(range.0, range.1)),
        MIX => {
            let x = leading_number(s)?;
            nearest(range, x, |v| Some((v as f64 / 128.0) * 100.0))
        }
        DRIVE => {
            let x = leading_number(s)?;
            nearest(range, x, |v| Some(f64::from(knob_gain(v))))
        }
        _ => parse_db(range, s, knob_gain),
    }
}

fn curve(c: &mut Canvas, r: Rect, v: &[i32]) {
    let bits = v[BITS] as f32;
    let drive = knob_gain(v[DRIVE]);
    let post = knob_gain(v[POST]);
    let mix = (v[MIX] as f32) / 128.0;
    let dry = 1.0 - mix;
    let levels = 2.0f32.powf(bits);

    view::transfer(c, r, 1.0, 1.0, |x| {
        let driven = x * drive;
        let crushed = (driven.clamp(-1.0, 1.0) * levels).round() / levels;
        (x * dry + crushed * post * mix).clamp(-1.2, 1.2)
    });
}

struct BitcrushSound(Distortion);

impl Sound for BitcrushSound {
    fn set(&mut self, v: &[i32]) {
        self.0.bits = (v[BITS] as f32).clamp(2.0, 16.0);
        self.0.downsample = (v[DOWNSAMPLE] as u32).max(1);
        self.0.drive = knob_gain(v[DRIVE]).max(0.1);
        self.0.mix = (v[MIX] as f32 / 128.0).clamp(0.0, 1.0);
        self.0.post_gain = knob_gain(v[POST]).max(0.0);
    }
    fn set_sample_rate(&mut self, _rate: u32) {}
    fn reset(&mut self) {
        use os_engine::effects::Effect;
        self.0.reset();
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        use os_engine::effects::Effect;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let mut f = [*l, *r];
            self.0.process(&mut f);
            *l = f[0];
            *r = f[1];
        }
    }
}

fn sound(_rate: u32, v: &[i32]) -> Box<dyn Sound> {
    let bits = (v[BITS] as f32).clamp(2.0, 16.0);
    let ds = (v[DOWNSAMPLE] as u32).max(1);
    let drive = knob_gain(v[DRIVE]).max(0.1);
    let mix = (v[MIX] as f32 / 128.0).clamp(0.0, 1.0);
    let post = knob_gain(v[POST]).max(0.0);
    Box::new(BitcrushSound(Distortion::new(DistMode::Bitcrush, drive, bits, ds, post, mix)))
}

pub struct BitcrusherKind;

impl Kind for BitcrusherKind {
    const SPEC: &'static Spec = SPEC;
    const CLAP_ID: &'static str = "org.openstudio.lemon-bitcrusher";
    const CLAP_DESCRIPTION: &'static str = "Lemon Studio's bitcrusher & decimator";
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Distortion, ClapFeature::Stereo];
    const VST3_CLASS_ID: [u8; 16] = *b"LemonStudioBitCr";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Distortion];
}

pub type LemonBitcrusher = SmallFx<BitcrusherKind>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitcrusher_params_and_sound() {
        assert_eq!(text(BITS, 8), "8 bits");
        assert_eq!(text(DOWNSAMPLE, 4), "4x");
        assert_eq!(text(MIX, 128), "100.0%");
        assert_eq!(parse(BITS, "12"), Some(12));
        assert_eq!(parse(DOWNSAMPLE, "8"), Some(8));

        let mut s = sound(44100, &DEFAULTS);
        let mut l = vec![0.12345, 0.6789];
        let mut r = vec![0.12345, 0.6789];
        s.process(&mut l, &mut r);
        assert_eq!(l.len(), 2);
    }
}
