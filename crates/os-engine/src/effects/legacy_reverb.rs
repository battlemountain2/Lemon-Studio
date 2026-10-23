use super::Effect;

const COMB_TUNINGS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_TUNINGS: [usize; 4] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;

struct Comb {
    buf: Vec<f32>,
    pos: usize,
    filter: f32,
}

impl Comb {
    fn new(size: usize) -> Self {
        Comb {
            buf: vec![0.0; size],
            pos: 0,
            filter: 0.0,
        }
    }

    fn process(&mut self, input: f32, feedback: f32, damping: f32) -> f32 {
        let output = self.buf[self.pos];
        self.filter = output * (1.0 - damping) + self.filter * damping;
        self.buf[self.pos] = input + self.filter * feedback;
        self.pos = (self.pos + 1) % self.buf.len();
        output
    }

    fn reset(&mut self) {
        self.buf.fill(0.0);
        self.filter = 0.0;
        self.pos = 0;
    }
}

struct Allpass {
    buf: Vec<f32>,
    pos: usize,
}

impl Allpass {
    fn new(size: usize) -> Self {
        Allpass {
            buf: vec![0.0; size],
            pos: 0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let buf_out = self.buf[self.pos];
        let output = -input + buf_out;
        self.buf[self.pos] = input + buf_out * 0.5;
        self.pos = (self.pos + 1) % self.buf.len();
        output
    }

    fn reset(&mut self) {
        self.buf.fill(0.0);
        self.pos = 0;
    }
}

pub struct LegacyReverb {
    combs_l: Vec<Comb>,
    combs_r: Vec<Comb>,
    allpass_l: Vec<Allpass>,
    allpass_r: Vec<Allpass>,
    room_size: f32,
    damping: f32,
    wet: f32,
    dry: f32,
    width: f32,
}

impl LegacyReverb {
    pub fn new(sample_rate: u32, room_size: f32, damping: f32, width: f32, mix: f32) -> Self {
        let scale = sample_rate as f32 / 44100.0;
        let combs_l = COMB_TUNINGS.iter().map(|&tuning| Comb::new(((tuning as f32 * scale) as usize).max(1))).collect();
        let combs_r = COMB_TUNINGS.iter().map(|&tuning| Comb::new((((tuning + STEREO_SPREAD) as f32 * scale) as usize).max(1))).collect();
        let allpass_l = ALLPASS_TUNINGS.iter().map(|&tuning| Allpass::new(((tuning as f32 * scale) as usize).max(1))).collect();
        let allpass_r = ALLPASS_TUNINGS.iter().map(|&tuning| Allpass::new((((tuning + STEREO_SPREAD) as f32 * scale) as usize).max(1))).collect();

        LegacyReverb {
            combs_l,
            combs_r,
            allpass_l,
            allpass_r,
            room_size: room_size.clamp(0.0, 1.0) * 0.28 + 0.7,
            damping: damping.clamp(0.0, 1.0) * 0.4,
            wet: mix.clamp(0.0, 1.0),
            dry: 1.0 - mix.clamp(0.0, 1.0),
            width: width.clamp(0.0, 1.0),
        }
    }

    #[cfg(test)]
    pub fn default_fx(sample_rate: u32) -> Self {
        Self::new(sample_rate, 0.6, 0.25, 1.0, 0.35)
    }

    #[cfg(test)]
    pub fn to_state(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(20);
        bytes.extend_from_slice(b"LMR1");
        let room = ((self.room_size - 0.7) / 0.28).clamp(0.0, 1.0);
        let damp = (self.damping / 0.4).clamp(0.0, 1.0);
        bytes.extend_from_slice(&room.to_le_bytes());
        bytes.extend_from_slice(&damp.to_le_bytes());
        bytes.extend_from_slice(&self.width.to_le_bytes());
        bytes.extend_from_slice(&self.wet.to_le_bytes());
        bytes
    }

    pub fn from_state(state: &[u8], sample_rate: u32) -> Option<Self> {
        if state.len() == 20 && &state[0..4] == b"LMR1" {
            let read_float = |start: usize| f32::from_le_bytes(state[start..start + 4].try_into().unwrap());
            let room_size = read_float(4);
            let damping = read_float(8);
            let width = read_float(12);
            let mix = read_float(16);
            if [room_size, damping, width, mix].iter().all(|value| value.is_finite()) {
                return Some(Self::new(sample_rate, room_size, damping, width, mix));
            }
        }
        None
    }
}

impl Effect for LegacyReverb {
    fn reset(&mut self) {
        for comb in &mut self.combs_l { comb.reset(); }
        for comb in &mut self.combs_r { comb.reset(); }
        for allpass in &mut self.allpass_l { allpass.reset(); }
        for allpass in &mut self.allpass_r { allpass.reset(); }
    }

    fn process(&mut self, buf: &mut [f32]) {
        let wet1 = self.wet * (self.width * 0.5 + 0.5);
        let wet2 = self.wet * ((1.0 - self.width) * 0.5);
        let dry = self.dry;
        let room = self.room_size;
        let damp = self.damping;

        for frame in buf.chunks_exact_mut(2) {
            let input = (frame[0] + frame[1]) * 0.015;

            let mut out_l = 0.0f32;
            let mut out_r = 0.0f32;

            for comb in &mut self.combs_l {
                out_l += comb.process(input, room, damp);
            }
            for comb in &mut self.combs_r {
                out_r += comb.process(input, room, damp);
            }

            for ap in &mut self.allpass_l {
                out_l = ap.process(out_l);
            }
            for ap in &mut self.allpass_r {
                out_r = ap.process(out_r);
            }

            frame[0] = frame[0] * dry + out_l * wet1 + out_r * wet2;
            frame[1] = frame[1] * dry + out_r * wet1 + out_l * wet2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_state_round_trip_preserves_settings() {
        let original = LegacyReverb::default_fx(44100);
        let state = original.to_state();
        let restored = LegacyReverb::from_state(&state, 44100).unwrap();
        assert_eq!(restored.to_state(), state);
    }

    #[test]
    fn reverb_tail_rings_out() {
        let mut effect = LegacyReverb::default_fx(44100);
        let mut buf = vec![0.0f32; 2000 * 2];
        buf[0] = 1.0;
        buf[1] = 1.0;
        effect.process(&mut buf);
        let tail_energy: f32 = buf[200..].iter().map(|sample| sample.abs()).sum();
        assert!(tail_energy > 0.05, "reverb tail should ring, energy {tail_energy}");
    }
}
