// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use os_model::{ClipSource, Rgb};
use crate::app::State;
use crate::playlist::{arr_mut, clips};

mod job;
pub use job::{Job, start, tick, cancel, paint};
#[cfg(test)]
use crate::playlist::add_audio_clip;

/// Musical stem kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StemKind {
    Kick,
    Snare,
    HiHats,
    Drums,
    Bass,
    LeadVocals,
    BackingVocals,
    Vocals,
    Instruments,
    Ambient,
    Other,
    Music,
    Melodic,
}

impl StemKind {
    /// Human-readable label for track naming.
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            StemKind::Kick => "Kick",
            StemKind::Snare => "Snare & Claps",
            StemKind::HiHats => "Hi-Hats & Cymbals",
            StemKind::Drums => "Drums",
            StemKind::Bass => "Bass",
            StemKind::LeadVocals => "Lead Vocals",
            StemKind::BackingVocals => "Backing Vocals",
            StemKind::Vocals => "Vocals",
            StemKind::Instruments => "Guitars & Keys",
            StemKind::Ambient => "Ambient FX",
            StemKind::Other => "Other",
            StemKind::Music => "Music",
            StemKind::Melodic => "Melodic",
        }
    }

    /// Distinctive theme color for stem clips and channels.
    pub fn color(self) -> Rgb {
        match self {
            // Punchy crimson red for kick fundamentals
            StemKind::Kick => Rgb::new(230, 57, 70),
            // Vibrant tangerine orange for snare and claps
            StemKind::Snare => Rgb::new(247, 127, 0),
            // Warm golden amber for hi-hats, rides, and cymbals
            StemKind::HiHats => Rgb::new(252, 191, 73),
            // Punchy drum orange for combined percussion
            StemKind::Drums => Rgb::new(235, 94, 40),
            // Deep resonant electric cyan for bass
            StemKind::Bass => Rgb::new(0, 180, 216),
            // Bright radiant gold for lead vocal formants
            StemKind::LeadVocals => Rgb::new(255, 230, 0),
            // Warm rose coral for stereo backing vocals
            StemKind::BackingVocals => Rgb::new(255, 117, 143),
            // Warm gold for combined vocals
            StemKind::Vocals => Rgb::new(245, 197, 24),
            // Electric violet for guitars, keyboards, and synths
            StemKind::Instruments => Rgb::new(157, 78, 221),
            // Atmospheric teal for reverb tails, delay, and ambient sheen
            StemKind::Ambient => Rgb::new(46, 196, 182),
            // Lush violet for combined other
            StemKind::Other => Rgb::new(173, 76, 219),
            // Slate blue for combined instrumental music
            StemKind::Music => Rgb::new(90, 120, 220),
            // Melodic purple
            StemKind::Melodic => Rgb::new(160, 90, 230),
        }
    }
}

/// Stem separation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StemMode {
    /// 8 Stems: Kick, Snare & Claps, Hi-Hats, Bass, Lead Vocals, Backing Vocals, Instruments, Ambient FX.
    Stems8,
    /// 6 Stems: Kick, Snare & Perc, Bass, Vocals, Instruments, Ambient FX.
    Stems6,
    /// 4 Stems: Drums, Bass, Vocals, Other.
    All4,
    /// 2 Stems: Vocals & Instrumental Music.
    VocalsAndMusic,
    /// 2 Stems: Drums & Melodic.
    DrumsAndMelody,
}

impl StemMode {
    pub fn label(self) -> &'static str {
        match self {
            StemMode::Stems8 => "8 spectral layers (experimental)",
            StemMode::Stems6 => "6 spectral layers (experimental)",
            StemMode::All4 => "4 spectral layers (experimental)",
            StemMode::VocalsAndMusic => "Vocal-range & music layers (experimental)",
            StemMode::DrumsAndMelody => "Transient & melodic layers (experimental)",
        }
    }
}

/// One separated stem ready for export and playlist insertion.
pub struct SeparatedStem {
    #[allow(dead_code)]
    pub kind: StemKind,
    pub label: String,
    pub color: Rgb,
    pub interleaved_audio: Vec<f32>,
}

// ---------------------------------------------------------------------------------------------------------
// DSP Crossover & Biquad Filters
// ---------------------------------------------------------------------------------------------------------

/// Standard 2nd-order Direct Form I biquad filter.
#[derive(Debug, Clone, Copy)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    /// Butterworth 2nd-order lowpass filter.
    pub fn lowpass(cutoff_hz: f32, sample_rate: f32, q: f32) -> Self {
        let omega = 2.0 * std::f32::consts::PI * (cutoff_hz / sample_rate).clamp(0.0001, 0.499);
        let alpha = omega.sin() / (2.0 * q);
        let cos_w = omega.cos();
        let b0 = (1.0 - cos_w) * 0.5;
        let b1 = 1.0 - cos_w;
        let b2 = (1.0 - cos_w) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w;
        let a2 = 1.0 - alpha;
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// Butterworth 2nd-order highpass filter.
    pub fn highpass(cutoff_hz: f32, sample_rate: f32, q: f32) -> Self {
        let omega = 2.0 * std::f32::consts::PI * (cutoff_hz / sample_rate).clamp(0.0001, 0.499);
        let alpha = omega.sin() / (2.0 * q);
        let cos_w = omega.cos();
        let b0 = (1.0 + cos_w) * 0.5;
        let b1 = -(1.0 + cos_w);
        let b2 = (1.0 + cos_w) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w;
        let a2 = 1.0 - alpha;
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }

    #[inline]
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

/// Zero-phase forward-backward filtering (`filtfilt`) applying a biquad stage.
pub fn filtfilt_biquad(filter: &mut Biquad, buf: &mut [f32]) {
    filter.reset();
    for s in buf.iter_mut() {
        *s = filter.process(*s);
    }
    filter.reset();
    for s in buf.iter_mut().rev() {
        *s = filter.process(*s);
    }
}

/// Linkwitz-Riley 4th order (24 dB/oct) crossover stage composed of two cascaded Butterworth biquads.
#[derive(Debug, Clone, Copy)]
pub struct LinkwitzRiley4 {
    stage1: Biquad,
    stage2: Biquad,
}

impl LinkwitzRiley4 {
    pub fn lowpass(cutoff_hz: f32, sample_rate: f32) -> Self {
        let q = std::f32::consts::FRAC_1_SQRT_2; // 0.70710678
        Self {
            stage1: Biquad::lowpass(cutoff_hz, sample_rate, q),
            stage2: Biquad::lowpass(cutoff_hz, sample_rate, q),
        }
    }

    pub fn highpass(cutoff_hz: f32, sample_rate: f32) -> Self {
        let q = std::f32::consts::FRAC_1_SQRT_2;
        Self {
            stage1: Biquad::highpass(cutoff_hz, sample_rate, q),
            stage2: Biquad::highpass(cutoff_hz, sample_rate, q),
        }
    }

    /// Zero-phase forward-backward filtering over an audio buffer.
    pub fn filtfilt(&mut self, buf: &mut [f32]) {
        filtfilt_biquad(&mut self.stage1, buf);
        filtfilt_biquad(&mut self.stage2, buf);
    }
}

// ---------------------------------------------------------------------------------------------------------
// Core Stem Separation Algorithm (2, 4, 6, 8 Stems)
// ---------------------------------------------------------------------------------------------------------

/// Separates stereo audio into musical stems using the spectral Linkwitz-Riley and Mid/Side engine.
pub fn separate_audio_stems(
    left: &[f32],
    right: &[f32],
    rate: u32,
    mode: StemMode,
) -> Result<Vec<SeparatedStem>, String> {
    split_controlled(left, right, rate, mode, &AtomicBool::new(false), &AtomicUsize::new(0))
}

fn split_controlled(left: &[f32], right: &[f32], rate: u32, mode: StemMode, cancel: &AtomicBool, progress: &AtomicUsize) -> Result<Vec<SeparatedStem>, String> {
    let check = || if cancel.load(Ordering::Relaxed) { Err("Spectral split cancelled".to_string()) } else { Ok(()) };
    check()?;
    let len = left.len().min(right.len());
    if len > job::MAX_FRAMES { return Err("Source exceeds the spectral split frame limit; use a shorter audio file".into()); }
    if rate < 12000 || rate > 384000 { return Err("Spectral splitting supports sample rates from 12 to 384 kHz".into()); }
    if len == 0 {
        return Err("Audio buffer is empty".into());
    }

    let r = rate as f32;

    // 1. Zero-phase Linkwitz-Riley band decomposition
    let mut low_l = left[..len].to_vec();
    let mut low_r = right[..len].to_vec();
    let mut lp_150 = LinkwitzRiley4::lowpass(150.0, r);
    lp_150.filtfilt(&mut low_l);
    lp_150.filtfilt(&mut low_r);

    let mut high_l = left[..len].to_vec();
    let mut high_r = right[..len].to_vec();
    let mut hp_4500 = LinkwitzRiley4::highpass(4500.0, r);
    hp_4500.filtfilt(&mut high_l);
    hp_4500.filtfilt(&mut high_r);

    let mut mid_high_l = Vec::with_capacity(len);
    let mut mid_high_r = Vec::with_capacity(len);
    for i in 0..len {
        mid_high_l.push(left[i] - low_l[i] - high_l[i]);
        mid_high_r.push(right[i] - low_r[i] - high_r[i]);
    }
    let mut hp_1800 = LinkwitzRiley4::highpass(1800.0, r);
    hp_1800.filtfilt(&mut mid_high_l);
    hp_1800.filtfilt(&mut mid_high_r);

    let mut low_mid_l = Vec::with_capacity(len);
    let mut low_mid_r = Vec::with_capacity(len);
    for i in 0..len {
        low_mid_l.push(left[i] - low_l[i] - high_l[i] - mid_high_l[i]);
        low_mid_r.push(right[i] - low_r[i] - high_r[i] - mid_high_r[i]);
    }

    check()?;
    let groups: &[(StemKind, &str, &[usize])] = match mode {
        StemMode::Stems8 => &[(StemKind::Kick, "Kick", &[0]), (StemKind::Snare, "Snare & Claps", &[1]), (StemKind::HiHats, "Hi-Hats & Cymbals", &[2]), (StemKind::Bass, "Bass & Sub", &[3]), (StemKind::LeadVocals, "Lead Vocals", &[4]), (StemKind::BackingVocals, "Backing Vocals", &[5]), (StemKind::Instruments, "Guitars & Keys", &[6]), (StemKind::Ambient, "Ambient FX", &[7])],
        StemMode::Stems6 => &[(StemKind::Kick, "Kick", &[0]), (StemKind::Snare, "Snare & Percussion", &[1,2]), (StemKind::Bass, "Bass", &[3]), (StemKind::Vocals, "Vocals", &[4,5]), (StemKind::Instruments, "Instruments", &[6]), (StemKind::Ambient, "Ambient FX", &[7])],
        StemMode::All4 => &[(StemKind::Drums, "Drums", &[0,1,2]), (StemKind::Bass, "Bass", &[3]), (StemKind::Vocals, "Vocals", &[4,5]), (StemKind::Other, "Other", &[6,7])],
        StemMode::VocalsAndMusic => &[(StemKind::Vocals, "Vocals", &[4,5]), (StemKind::Music, "Music", &[0,1,2,3,6,7])],
        StemMode::DrumsAndMelody => &[(StemKind::Drums, "Drums", &[0,1,2]), (StemKind::Melodic, "Melodic", &[3,4,5,6,7])],
    };
    let mut stems: Vec<_> = groups.iter().map(|(kind, label, _)| SeparatedStem { kind: *kind, label: (*label).into(), color: kind.color(), interleaved_audio: Vec::with_capacity(len * 2) }).collect();

    // Transient follower time constants
    let alpha_fast = (-1.0 / (0.003 * r)).exp(); // ~3ms fast attack
    let alpha_slow = (-1.0 / (0.065 * r)).exp(); // ~65ms slow decay

    let init_low = (low_l[0] + low_r[0]) * 0.5;
    let mut low_fast = init_low * init_low;
    let mut low_slow = low_fast;

    let init_lm = (low_mid_l[0] + low_mid_r[0]) * 0.5;
    let mut lowmid_fast = init_lm * init_lm;
    let mut lowmid_slow = lowmid_fast;

    let init_h = (high_l[0] + high_r[0]) * 0.5;
    let mut high_fast = init_h * init_h;
    let mut high_slow = high_fast;

    for i in 0..len {
        let low_l = low_l[i];
        let low_r = low_r[i];
        let lowmid_l = low_mid_l[i];
        let lowmid_r = low_mid_r[i];
        let midhigh_l = mid_high_l[i];
        let midhigh_r = mid_high_r[i];
        let high_l = high_l[i];
        let high_r = high_r[i];

        // =========================================================================
        // 1. Low Band (<150 Hz): Kick Drum vs Sustained Sub / Bass
        // =========================================================================
        let low_mono = (low_l + low_r) * 0.5;
        let low_sq = low_mono * low_mono;
        low_fast = alpha_fast * low_fast + (1.0 - alpha_fast) * low_sq;
        low_slow = alpha_slow * low_slow + (1.0 - alpha_slow) * low_sq;

        let kick_ratio = (low_fast / (low_slow + 1e-9)).clamp(0.0, 10.0);
        let w_kick = ((kick_ratio - 1.15) / 1.5).clamp(0.0, 1.0);

        let k_low_l = low_l * w_kick;
        let k_low_r = low_r * w_kick;
        let b_low_l = low_l * (1.0 - w_kick);
        let b_low_r = low_r * (1.0 - w_kick);

        // =========================================================================
        // 2. Low-Mid Band (150 Hz - 1.8 kHz): Snare/Clap vs Bass harmonics vs Low Instruments
        // =========================================================================
        let m_lowmid = (lowmid_l + lowmid_r) * 0.5;
        let s_lowmid = (lowmid_l - lowmid_r) * 0.5;

        let lm_sq = m_lowmid * m_lowmid;
        lowmid_fast = alpha_fast * lowmid_fast + (1.0 - alpha_fast) * lm_sq;
        lowmid_slow = alpha_slow * lowmid_slow + (1.0 - alpha_slow) * lm_sq;

        let snare_ratio = (lowmid_fast / (lowmid_slow + 1e-9)).clamp(0.0, 10.0);
        let w_snare = ((snare_ratio - 1.25) / 2.0).clamp(0.0, 1.0);

        let sn_l = m_lowmid * w_snare;
        let sn_r = m_lowmid * w_snare;

        let rem_lm = m_lowmid * (1.0 - w_snare);
        // Low vocal warmth (35%) vs instruments (65%)
        let w_voc_lm = 0.35f32;
        let lv_lm_l = rem_lm * w_voc_lm;
        let lv_lm_r = rem_lm * w_voc_lm;

        let inst_lm_l = rem_lm * (1.0 - w_voc_lm) + s_lowmid;
        let inst_lm_r = rem_lm * (1.0 - w_voc_lm) - s_lowmid;

        // =========================================================================
        // 3. Mid-High Band (1.8 kHz - 4.5 kHz): Lead Vocals vs Backing Vocals vs Instruments
        // =========================================================================
        let m_midhigh = (midhigh_l + midhigh_r) * 0.5;
        let s_midhigh = (midhigh_l - midhigh_r) * 0.5;

        // In center Mid-High, lead vocal formants dominate (85%), with remainder to instruments
        let w_lead = 0.85f32;
        let lv_mh_l = m_midhigh * w_lead;
        let lv_mh_r = m_midhigh * w_lead;
        let inst_mh_l = m_midhigh * (1.0 - w_lead);
        let inst_mh_r = m_midhigh * (1.0 - w_lead);

        // In side Mid-High, backing vocals (50%) and wide instruments/stereo guitars (50%)
        let bv_mh_l = s_midhigh * 0.5;
        let bv_mh_r = -s_midhigh * 0.5;
        let inst_side_l = s_midhigh * 0.5;
        let inst_side_r = -s_midhigh * 0.5;

        // =========================================================================
        // 4. High Band (>4.5 kHz): Hi-Hats/Cymbals vs Vocal Air vs Ambient FX/Reverb Sheen
        // =========================================================================
        let high_mono = (high_l + high_r) * 0.5;
        let high_sq = high_mono * high_mono;
        high_fast = alpha_fast * high_fast + (1.0 - alpha_fast) * high_sq;
        high_slow = alpha_slow * high_slow + (1.0 - alpha_slow) * high_sq;

        let hh_ratio = (high_fast / (high_slow + 1e-9)).clamp(0.0, 10.0);
        let w_hh = ((hh_ratio - 1.2) / 1.8).clamp(0.0, 1.0);

        let hh_l = high_l * w_hh;
        let hh_r = high_r * w_hh;

        let rem_high_l = high_l * (1.0 - w_hh);
        let rem_high_r = high_r * (1.0 - w_hh);

        let m_high = (rem_high_l + rem_high_r) * 0.5;
        let s_high = (rem_high_l - rem_high_r) * 0.5;

        // Vocal air / sibilance in center high (60%), ambient reverb in center (40%)
        let lv_hi_l = m_high * 0.60;
        let lv_hi_r = m_high * 0.60;

        let amb_m_l = m_high * 0.40;
        let amb_m_r = m_high * 0.40;

        // Side high consists of wide backing vocal breath (30%) + ambient reverb/delays (70%)
        let bv_hi_l = s_high * 0.30;
        let bv_hi_r = -s_high * 0.30;

        let amb_s_l = s_high * 0.70;
        let amb_s_r = -s_high * 0.70;

        // =========================================================================
        // Final Stem Accumulation
        // =========================================================================
        let stem_kick_l = k_low_l;
        let stem_kick_r = k_low_r;

        let stem_snare_l = sn_l;
        let stem_snare_r = sn_r;

        let stem_hihat_l = hh_l;
        let stem_hihat_r = hh_r;

        let stem_bass_l = b_low_l;
        let stem_bass_r = b_low_r;

        let stem_lead_voc_l = lv_lm_l + lv_mh_l + lv_hi_l;
        let stem_lead_voc_r = lv_lm_r + lv_mh_r + lv_hi_r;

        let stem_back_voc_l = bv_mh_l + bv_hi_l;
        let stem_back_voc_r = bv_mh_r + bv_hi_r;

        let stem_inst_l = inst_lm_l + inst_mh_l + inst_side_l;
        let stem_inst_r = inst_lm_r + inst_mh_r + inst_side_r;

        let stem_amb_l = amb_m_l + amb_s_l;
        let stem_amb_r = amb_m_r + amb_s_r;

        if i % 4096 == 0 {
            check()?;
            progress.store(20 + i * 60 / len, Ordering::Relaxed);
        }
        let values = [[stem_kick_l, stem_kick_r], [stem_snare_l, stem_snare_r], [stem_hihat_l, stem_hihat_r], [stem_bass_l, stem_bass_r], [stem_lead_voc_l, stem_lead_voc_r], [stem_back_voc_l, stem_back_voc_r], [stem_inst_l, stem_inst_r], [stem_amb_l, stem_amb_r]];
        for (stem, (_, _, indices)) in stems.iter_mut().zip(groups) {
            for side in 0..2 {
                let mut value = values[indices[0]][side];
                for &index in &indices[1..] { value += values[index][side]; }
                stem.interleaved_audio.push(value);
            }
        }
    }
    check()?;
    Ok(stems)
}

// ---------------------------------------------------------------------------------------------------------
// Playlist & Project Integration
// ---------------------------------------------------------------------------------------------------------

/// Target directory for exported stem audio files.
pub fn stems_dir(s: &State) -> PathBuf {
    s.path
        .as_ref()
        .and_then(|p| p.parent())
        .map(|d| d.join("Rendered").join("Stems"))
        .unwrap_or_else(|| std::env::temp_dir().join("Lemon Studio").join("Rendered").join("Stems"))
}

#[cfg(test)]
pub fn separate_clip_stems(s: &mut State, clip_idx: usize, mode: StemMode) -> Result<Vec<usize>, String> {
    let input = job::Input::capture(s, clip_idx, mode)?;
    let prepared = input.prepare(&AtomicBool::new(false), &AtomicUsize::new(0))?;
    prepared.commit(s)
}

// ---------------------------------------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stem_8_crossover_reconstruction_exact_sum() {
        let rate = 44100;
        let frames = 4410; // 0.1 second
        let mut left = Vec::with_capacity(frames);
        let mut right = Vec::with_capacity(frames);

        for i in 0..frames {
            let t = i as f32 / rate as f32;
            let low = (2.0 * std::f32::consts::PI * 70.0 * t).sin() * 0.4;
            let lowmid = (2.0 * std::f32::consts::PI * 400.0 * t).sin() * 0.3;
            let midhigh = (2.0 * std::f32::consts::PI * 2500.0 * t).sin() * 0.25;
            let high = (2.0 * std::f32::consts::PI * 9000.0 * t).sin() * 0.15;
            left.push(low + lowmid + midhigh + high);
            right.push(low * 0.95 + lowmid * 1.05 + midhigh * 0.85 + high * 1.1);
        }

        let stems = separate_audio_stems(&left, &right, rate, StemMode::Stems8)
            .expect("8 stem separation succeeds");

        assert_eq!(stems.len(), 8, "produces 8 stems");

        // Verify mathematical reconstruction across all 8 stems
        for i in 0..frames {
            let mut sum_l = 0.0f32;
            let mut sum_r = 0.0f32;
            for stem in &stems {
                sum_l += stem.interleaved_audio[i * 2];
                sum_r += stem.interleaved_audio[i * 2 + 1];
            }

            let err_l = (sum_l - left[i]).abs();
            let err_r = (sum_r - right[i]).abs();

            assert!(
                err_l < 1e-4,
                "8-stem left sample {i} sum deviation exceeds tolerance: sum={sum_l}, orig={}",
                left[i]
            );
            assert!(
                err_r < 1e-4,
                "8-stem right sample {i} sum deviation exceeds tolerance: sum={sum_r}, orig={}",
                right[i]
            );
        }
    }

    #[test]
    fn test_stem_6_crossover_reconstruction_exact_sum() {
        let rate = 44100;
        let frames = 2205;
        let mut left = Vec::with_capacity(frames);
        let mut right = Vec::with_capacity(frames);

        for i in 0..frames {
            let t = i as f32 / rate as f32;
            left.push((2.0 * std::f32::consts::PI * 220.0 * t).sin() * 0.5);
            right.push((2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5);
        }

        let stems = separate_audio_stems(&left, &right, rate, StemMode::Stems6)
            .expect("6 stem separation succeeds");

        assert_eq!(stems.len(), 6, "produces 6 stems");

        for i in 0..frames {
            let mut sum_l = 0.0f32;
            let mut sum_r = 0.0f32;
            for stem in &stems {
                sum_l += stem.interleaved_audio[i * 2];
                sum_r += stem.interleaved_audio[i * 2 + 1];
            }
            assert!((sum_l - left[i]).abs() < 1e-4);
            assert!((sum_r - right[i]).abs() < 1e-4);
        }
    }

    #[test]
    fn test_stem_spectral_band_energy_distribution() {
        let rate = 44100;
        let frames = 4410;
        let mut left = vec![0.0f32; frames];
        let mut right = vec![0.0f32; frames];

        // Pure 60 Hz tone (Sub-bass)
        for i in 0..frames {
            let t = i as f32 / rate as f32;
            let s = (2.0 * std::f32::consts::PI * 60.0 * t).sin() * 0.5;
            left[i] = s;
            right[i] = s;
        }

        let stems = separate_audio_stems(&left, &right, rate, StemMode::All4)
            .expect("stem separation succeeds");

        let bass_energy: f32 = stems[1].interleaved_audio.iter().map(|x| x * x).sum();
        let high_energy: f32 = stems[3].interleaved_audio.iter().map(|x| x * x).sum();

        assert!(
            bass_energy > high_energy * 10.0,
            "Bass stem should dominate 60Hz signal energy (bass={bass_energy}, other={high_energy})"
        );
    }

    #[test]
    fn test_playlist_8_stem_separation_workflow_and_undo() {
        let mut app = crate::app::App::new(None);

        // 1. Create a dummy test audio WAV file
        let temp_dir = std::env::temp_dir().join(format!("lemon-test-8stems-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let test_wav = temp_dir.join("TestSample8.wav");

        let rate = 44100;
        let frames = 4410;
        let mut interleaved = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            let s = (i as f32 * 0.05).sin() * 0.5;
            interleaved.push(s);
            interleaved.push(s);
        }
        {
            let mut f = std::fs::File::create(&test_wav).unwrap();
            os_engine::wav::write_f32_stereo(&mut f, rate, &interleaved).unwrap();
        }

        // 2. Add audio clip to playlist
        let clip_idx = add_audio_clip(&mut app.state, &test_wav.to_string_lossy(), 0, 0)
            .expect("audio clip added");

        assert_eq!(clips(&app.state).len(), 1);
        assert!(!clips(&app.state)[0].muted, "Original clip starts unmuted");

        // 3. Separate into 8 stems
        let new_clips = separate_clip_stems(&mut app.state, clip_idx, StemMode::Stems8)
            .expect("8-stem separation succeeds");

        assert_eq!(new_clips.len(), 8, "Created 8 stem clips");
        assert_eq!(clips(&app.state).len(), 9, "Total 9 clips (1 source + 8 stems)");
        assert!(clips(&app.state)[0].muted, "Original source clip is now muted");

        // Verify stacked track placement and naming
        let stem_names = [
            "Kick",
            "Snare & Claps",
            "Hi-Hats & Cymbals",
            "Bass & Sub",
            "Lead Vocals",
            "Backing Vocals",
            "Guitars & Keys",
            "Ambient FX",
        ];
        for (k, &idx) in new_clips.iter().enumerate() {
            let c = &clips(&app.state)[idx];
            assert_eq!(c.position, 0);
            assert_eq!(c.track, (k as u16) + 1, "Track stacked consecutively below source");

            if let ClipSource::Channel(ch) = c.source {
                let chan = app.state.project.channel(ch).unwrap();
                let name = chan.name.as_deref().unwrap_or_default();
                assert!(
                    name.contains(stem_names[k]),
                    "Channel name '{name}' should contain '{}'",
                    stem_names[k]
                );
                assert!(chan.color.is_some(), "Channel has custom stem color");
            }
        }

        // 4. Test Single-step Ctrl+Z Undo
        crate::history::undo(&mut app.state);

        assert_eq!(clips(&app.state).len(), 1, "Undo restores clip count to 1");
        assert!(!clips(&app.state)[0].muted, "Undo un-mutes original source clip");

        // Clean up temp dir
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
