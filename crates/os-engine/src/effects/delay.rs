//! Stereo feedback delay effect with ping-pong, lowpass damping, and wet/dry mix.

use super::Effect;

pub struct Delay {
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    pos: usize,
    delay_len: usize,
    pub delay_ms: f32,
    pub feedback: f32,
    pub damping: f32,
    filter_l: f32,
    filter_r: f32,
    pub ping_pong: bool,
    pub mix: f32,
}

impl Delay {
    pub fn new(sample_rate: u32, delay_ms: f32, feedback: f32, damping: f32, ping_pong: bool, mix: f32) -> Self {
        let max_samples = (sample_rate as usize) * 2; // up to 2 seconds
        let delay_len = ((delay_ms / 1000.0 * sample_rate as f32) as usize).clamp(1, max_samples - 1);
        Delay {
            buf_l: vec![0.0; max_samples],
            buf_r: vec![0.0; max_samples],
            pos: 0,
            delay_len,
            delay_ms,
            feedback: feedback.clamp(0.0, 0.98),
            damping: damping.clamp(0.0, 1.0),
            filter_l: 0.0,
            filter_r: 0.0,
            ping_pong,
            mix: mix.clamp(0.0, 1.0),
        }
    }

    pub fn default_fx(sample_rate: u32) -> Self {
        // Default: 375ms (approx 3/16 at 120-130 BPM), 0.45 feedback, 0.3 damping, 0.4 mix
        Self::new(sample_rate, 375.0, 0.45, 0.3, true, 0.4)
    }

    pub fn to_state(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(24);
        b.extend_from_slice(b"LMD1");
        b.extend_from_slice(&self.delay_ms.to_le_bytes());
        b.extend_from_slice(&self.feedback.to_le_bytes());
        b.extend_from_slice(&self.damping.to_le_bytes());
        b.extend_from_slice(&(if self.ping_pong { 1.0f32 } else { 0.0f32 }).to_le_bytes());
        b.extend_from_slice(&self.mix.to_le_bytes());
        b
    }

    pub fn from_state(state: &[u8], sample_rate: u32) -> Self {
        if state.len() >= 24 && &state[0..4] == b"LMD1" {
            let f = |start: usize| f32::from_le_bytes(state[start..start + 4].try_into().unwrap());
            let time_ms = f(4);
            let feedback = f(8);
            let damping = f(12);
            let ping_pong = f(16) != 0.0;
            let mix = f(20);
            return Self::new(sample_rate, time_ms, feedback, damping, ping_pong, mix);
        }
        Self::default_fx(sample_rate)
    }
}

impl Effect for Delay {
    fn reset(&mut self) {
        self.buf_l.fill(0.0);
        self.buf_r.fill(0.0);
        self.pos = 0;
        self.filter_l = 0.0;
        self.filter_r = 0.0;
    }

    fn process(&mut self, buf: &mut [f32]) {
        let cap = self.buf_l.len();
        let damp = self.damping;
        let one_minus_damp = 1.0 - damp;
        let fb = self.feedback;
        let wet = self.mix;
        let dry = 1.0 - wet;

        for f in buf.chunks_exact_mut(2) {
            let in_l = f[0];
            let in_r = f[1];

            let read_pos = if self.pos >= self.delay_len {
                self.pos - self.delay_len
            } else {
                cap + self.pos - self.delay_len
            };

            let delayed_l = self.buf_l[read_pos];
            let delayed_r = self.buf_r[read_pos];

            // Lowpass damping filter on feedback
            self.filter_l = self.filter_l * damp + delayed_l * one_minus_damp;
            self.filter_r = self.filter_r * damp + delayed_r * one_minus_damp;

            if self.ping_pong {
                // Ping-pong cross feeds L into R and R into L
                self.buf_l[self.pos] = in_l + self.filter_r * fb;
                self.buf_r[self.pos] = in_r + self.filter_l * fb;
            } else {
                self.buf_l[self.pos] = in_l + self.filter_l * fb;
                self.buf_r[self.pos] = in_r + self.filter_r * fb;
            }

            self.pos = (self.pos + 1) % cap;

            f[0] = in_l * dry + delayed_l * wet;
            f[1] = in_r * dry + delayed_r * wet;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_repeats_audio_with_feedback() {
        let mut d = Delay::new(44100, 10.0, 0.5, 0.0, false, 0.5); // 10ms = 441 samples
        let mut impulse = vec![0.0f32; 1000 * 2];
        impulse[0] = 1.0;
        impulse[1] = 1.0;
        d.process(&mut impulse);
        let echo_l = impulse[441 * 2];
        let echo_r = impulse[441 * 2 + 1];
        assert!(echo_l > 0.1, "echo should be present, got {echo_l}");
        assert!(echo_r > 0.1, "echo should be present, got {echo_r}");
    }
}
