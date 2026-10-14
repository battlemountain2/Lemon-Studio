//! Dariacore-ready distortion, waveshaper, overdrive, and bitcrusher effect.

use super::Effect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistMode {
    /// Smooth saturation / overdrive using tanh-style soft clipping.
    Overdrive,
    /// Hard digital clipping at ceiling.
    HardClip,
    /// Foldback waveshaper: aggressive harmonic folding distortion.
    Foldback,
    /// Digital bit depth reduction and sample rate decimation.
    Bitcrush,
}

pub struct Distortion {
    pub mode: DistMode,
    pub drive: f32,
    pub bits: f32,
    pub downsample: u32,
    pub post_gain: f32,
    pub mix: f32,
    // State for downsampling sample-and-hold
    hold_count: u32,
    hold_l: f32,
    hold_r: f32,
}

impl Distortion {
    pub fn new(mode: DistMode, drive: f32, bits: f32, downsample: u32, post_gain: f32, mix: f32) -> Self {
        Distortion {
            mode,
            drive: drive.max(0.1),
            bits: bits.clamp(2.0, 16.0),
            downsample: downsample.max(1),
            post_gain: post_gain.max(0.0),
            mix: mix.clamp(0.0, 1.0),
            hold_count: 0,
            hold_l: 0.0,
            hold_r: 0.0,
        }
    }

    /// Overdrive configuration: warm to heavy analog-style saturation.
    pub fn overdrive(drive: f32, mix: f32) -> Self {
        Self::new(DistMode::Overdrive, drive, 16.0, 1, 1.0 / (1.0 + (drive - 1.0) * 0.3), mix)
    }

    /// Dariacore foldback waveshaper: intense aggressive folding harmonics.
    pub fn waveshaper(drive: f32, mix: f32) -> Self {
        Self::new(DistMode::Foldback, drive, 16.0, 1, 0.7, mix)
    }

    /// Lo-fi / Dariacore bitcrusher with bit depth reduction and sample rate decimation.
    pub fn bitcrusher(bits: f32, downsample: u32, mix: f32) -> Self {
        Self::new(DistMode::Bitcrush, 1.5, bits, downsample, 0.9, mix)
    }

    pub fn to_state(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(28);
        b.extend_from_slice(b"LMDS");
        let mode_val: u32 = match self.mode {
            DistMode::Overdrive => 0,
            DistMode::HardClip => 1,
            DistMode::Foldback => 2,
            DistMode::Bitcrush => 3,
        };
        b.extend_from_slice(&mode_val.to_le_bytes());
        b.extend_from_slice(&self.drive.to_le_bytes());
        b.extend_from_slice(&self.bits.to_le_bytes());
        b.extend_from_slice(&self.downsample.to_le_bytes());
        b.extend_from_slice(&self.post_gain.to_le_bytes());
        b.extend_from_slice(&self.mix.to_le_bytes());
        b
    }

    pub fn from_state(name: &str, state: &[u8], _sample_rate: u32) -> Self {
        if state.len() >= 28 && &state[0..4] == b"LMDS" {
            let mode_val = u32::from_le_bytes(state[4..8].try_into().unwrap());
            let mode = match mode_val {
                1 => DistMode::HardClip,
                2 => DistMode::Foldback,
                3 => DistMode::Bitcrush,
                _ => DistMode::Overdrive,
            };
            let f = |start: usize| f32::from_le_bytes(state[start..start + 4].try_into().unwrap());
            let drive = f(8);
            let bits = f(12);
            let downsample = u32::from_le_bytes(state[16..20].try_into().unwrap());
            let post_gain = f(20);
            let mix = f(24);
            return Self::new(mode, drive, bits, downsample, post_gain, mix);
        }
        let lower = name.to_lowercase();
        if lower.contains("bitcrush") {
            Self::bitcrusher(8.0, 4, 1.0)
        } else if lower.contains("waveshaper") {
            Self::waveshaper(3.5, 0.8)
        } else {
            Self::overdrive(3.0, 0.75)
        }
    }

    #[inline]
    fn shape_sample(&self, x: f32) -> f32 {
        let driven = x * self.drive;
        match self.mode {
            DistMode::Overdrive => {
                // Rational soft-clip approximation: x / (1 + |x|) scaled
                driven / (1.0 + driven.abs())
            }
            DistMode::HardClip => {
                driven.clamp(-1.0, 1.0)
            }
            DistMode::Foldback => {
                // Triangle / sine wavefolder: folds signal back inside [-1.0, 1.0]
                let s = (driven * (std::f32::consts::PI * 0.5)).sin();
                (s + driven.clamp(-1.0, 1.0) * 0.5) * 0.7
            }
            DistMode::Bitcrush => {
                // Bit depth quantization: 2^bits levels
                let levels = 2.0f32.powf(self.bits);
                let stepped = (driven.clamp(-1.0, 1.0) * levels).round() / levels;
                stepped
            }
        }
    }
}

impl Effect for Distortion {
    fn reset(&mut self) {
        self.hold_count = 0;
        self.hold_l = 0.0;
        self.hold_r = 0.0;
    }

    fn process(&mut self, buf: &mut [f32]) {
        let wet = self.mix;
        let dry = 1.0 - wet;
        let post = self.post_gain;
        let ds = self.downsample;

        for f in buf.chunks_exact_mut(2) {
            let in_l = f[0];
            let in_r = f[1];

            let (shaped_l, shaped_r) = if ds > 1 {
                if self.hold_count == 0 {
                    self.hold_l = self.shape_sample(in_l);
                    self.hold_r = self.shape_sample(in_r);
                }
                self.hold_count = (self.hold_count + 1) % ds;
                (self.hold_l, self.hold_r)
            } else {
                (self.shape_sample(in_l), self.shape_sample(in_r))
            };

            f[0] = in_l * dry + shaped_l * post * wet;
            f[1] = in_r * dry + shaped_r * post * wet;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overdrive_saturates_loud_signals() {
        let mut dist = Distortion::overdrive(4.0, 1.0);
        let mut buf = [1.0, 1.0];
        dist.process(&mut buf);
        assert!(buf[0] > 0.0 && buf[0] <= 1.0, "should saturate cleanly");
    }

    #[test]
    fn bitcrusher_quantizes_and_decimates() {
        let mut bc = Distortion::bitcrusher(4.0, 2, 1.0); // 4 bits = 16 levels
        let mut buf = [0.12345, 0.12345, 0.6789, 0.6789];
        bc.process(&mut buf);
        // With downsampling factor 2, sample 0 and sample 1 should hold the same value
        assert_eq!(buf[0], buf[2]);
        assert_eq!(buf[1], buf[3]);
    }
}
