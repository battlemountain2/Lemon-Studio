// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Preset Grooves & Progressions: Digicore, Hyperpop, EDM, Trap, and Dembow drum patterns and chord progressions.
//!
//! Provides instant musical building blocks:
//! 1. Drum Grooves:
//!    - Trap 2-step hi-hats with velocity dynamics & roll variations.
//!    - Digicore/Hyperpop frantic glitch hats and stutter snares.
//!    - UK/NY Drill syncopated triplet hi-hat bounce.
//!    - EDM 4-on-the-floor kick, off-beat open hats, and build-up rolls.
//!    - Dembow / Reggaeton syncopated rhythms.
//! 2. Harmonic Chord Progressions:
//!    - Digicore/Hyperpop Royal "Anime" progression (IVmaj7 - V - iii7 - vi).
//!    - High-energy Hyperpop anthem chords (I - V - vi - IV).
//!    - Melancholic Trap / Emo minor (i - VI - III - VII).
//!    - EDM / Future Bass lush 7th/9th stabs.
//!    - Neo-Soul / Lo-Fi jazz turnaround (ii9 - V13 - Imaj9 - vi7).
//!    - Cyber Trap Phrygian dark bounce (i - bII - i - bVII).

use os_model::Note;
use crate::app::State;
use crate::piano;

/// Preset drum grooves for step sequencer and channel rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrumGroove {
    Trap2Step,
    TrapRolls,
    DigicoreGlitchHats,
    DrillBounce,
    Edm4OnFloor,
    EdmOffbeatHat,
    TrapSnare,
    DigicoreGlitchSnare,
    DembowSnare,
    EdmBuildUpSnare,
}

impl DrumGroove {
    pub fn title(&self) -> &'static str {
        match self {
            DrumGroove::Trap2Step => "Drum Groove: Trap 2-Step Hi-Hat",
            DrumGroove::TrapRolls => "Drum Groove: Trap Hi-Hat (8th & 16th Rolls)",
            DrumGroove::DigicoreGlitchHats => "Drum Groove: Digicore / Hyperpop Glitch Hats",
            DrumGroove::DrillBounce => "Drum Groove: UK / NY Drill Bounce Hi-Hat",
            DrumGroove::Edm4OnFloor => "Drum Groove: EDM 4-on-the-Floor Kick",
            DrumGroove::EdmOffbeatHat => "Drum Groove: EDM Off-Beat Open Hi-Hat",
            DrumGroove::TrapSnare => "Drum Groove: Trap / Boom-Bap Snare & Clap",
            DrumGroove::DigicoreGlitchSnare => "Drum Groove: Digicore Glitch Snare (Flam & Double-Tap)",
            DrumGroove::DembowSnare => "Drum Groove: Dembow / Reggaeton Snare",
            DrumGroove::EdmBuildUpSnare => "Drum Groove: EDM Build-Up Snare Roll (1 Bar)",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            DrumGroove::Trap2Step => "Classic 8th-note hats with downbeat velocity accents".into(),
            DrumGroove::TrapRolls => "2-step hats with 16th roll and 32nd turnaround bursts".into(),
            DrumGroove::DigicoreGlitchHats => "Fast 16th hats with 32nd triplet rolls & velocity jumps".into(),
            DrumGroove::DrillBounce => "Syncopated 3-step dotted bounce with ghost notes".into(),
            DrumGroove::Edm4OnFloor => "Driving kicks on beats 1, 2, 3, 4 (steps 0, 4, 8, 12)".into(),
            DrumGroove::EdmOffbeatHat => "Club driving open hats on the off-beats (steps 2, 6, 10, 14)".into(),
            DrumGroove::TrapSnare => "Punchy backbeat snare on beats 2 and 4 (steps 4, 12)".into(),
            DrumGroove::DigicoreGlitchSnare => "Backbeat with step 14 double-tap and ghost flam".into(),
            DrumGroove::DembowSnare => "Syncopated Latin / Urban Dembow rhythm (steps 0, 3, 6, 10, 12)".into(),
            DrumGroove::EdmBuildUpSnare => "Accelerating roll: 1/4 to 1/8 to 1/16 with volume swell".into(),
        }
    }
}

/// Preset harmonic chord progressions voiced for synths and keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChordProgression {
    DigicoreRoyal,
    HyperpopAnthem,
    TrapEmotionalMinor,
    EdmFutureBassStabs,
    NeoSoulJazz251,
    DigicoreCyberPhrygian,
}

impl ChordProgression {
    pub fn title(&self) -> &'static str {
        match self {
            ChordProgression::DigicoreRoyal => "Chord Progression: Digicore Royal (IVmaj7 - V - iii7 - vi)",
            ChordProgression::HyperpopAnthem => "Chord Progression: Hyperpop Anthem (I - V - vi - IV)",
            ChordProgression::TrapEmotionalMinor => "Chord Progression: Trap / Emo Minor (i - VI - III - VII)",
            ChordProgression::EdmFutureBassStabs => "Chord Progression: EDM / Future Bass 7th & 9th Stabs",
            ChordProgression::NeoSoulJazz251 => "Chord Progression: Neo-Soul / Lo-Fi 2-5-1-6 (ii9 - V13 - Imaj9)",
            ChordProgression::DigicoreCyberPhrygian => "Chord Progression: Cyber Trap Phrygian (i - bII - i - bVII)",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            ChordProgression::DigicoreRoyal => "The iconic Japanese / Digicore progression (Fmaj7 - G - Em7 - Am)".into(),
            ChordProgression::HyperpopAnthem => "Soaring, uplifting supersaw progression (C - G - Am - F)".into(),
            ChordProgression::TrapEmotionalMinor => "Dark melancholic minor chords (Am - F - C - G) for 808s and leads".into(),
            ChordProgression::EdmFutureBassStabs => "Syncopated Future Bass stabs (Am9 - Fmaj7 - Cmaj7 - G)".into(),
            ChordProgression::NeoSoulJazz251 => "Smooth jazzy extensions (Dm9 - G13 - Cmaj9 - Am7)".into(),
            ChordProgression::DigicoreCyberPhrygian => "Dark, aggressive cyber Phrygian tension (Am - Bb - Am - G)".into(),
        }
    }
}

/// Applies a drum groove to the active pattern on the currently selected channel.
/// Replaces the channel's notes in the pattern with the newly generated rhythm.
pub fn apply_drum_groove(s: &mut State, groove: DrumGroove) -> usize {
    let ch = s.selected_channel as u16;
    let ppq = u32::from(s.project.ppq.max(4));
    let ps = ppq / 4; // Ticks per 16th-note step

    piano::snapshot_as(s, "Apply drum groove");

    let mut notes = Vec::new();
    let default_key = 60u16; // C5

    let make_step = |step_idx: u32, vel: u8, len: u32| -> Note {
        Note {
            position: step_idx * ps,
            length: len,
            channel: ch,
            key: default_key,
            fine_pitch: 120,
            release: 0,
            pan: 64,
            velocity: vel,
            mod_x: 0,
            mod_y: 0,
            flags: 0x4000,
            group: 0,
            unk14: 0,
            repeat: 0,
        }
    };

    match groove {
        DrumGroove::Trap2Step => {
            // Every 2 steps (0, 2, 4, 6, 8, 10, 12, 14)
            for step in (0..16).step_by(2) {
                let vel = if step % 4 == 0 { 108 } else { 82 };
                notes.push(make_step(step, vel, ps));
            }
        }
        DrumGroove::TrapRolls => {
            // 2-step base
            for step in [0, 2, 4, 8, 10, 12] {
                let vel = if step % 4 == 0 { 108 } else { 85 };
                notes.push(make_step(step, vel, ps));
            }
            // 16th roll at step 6: two quick hits
            notes.push(make_step(6, 92, ps / 2));
            let mut n_sub = make_step(6, 96, ps / 2);
            n_sub.position += ps / 2;
            notes.push(n_sub);

            // 32nd roll at step 14: four rapid hits with velocity ramp
            for (sub_i, &vel) in [75, 88, 100, 114].iter().enumerate() {
                let mut n = make_step(14, vel, ps / 4);
                n.position += (sub_i as u32) * (ps / 4);
                notes.push(n);
            }
        }
        DrumGroove::DigicoreGlitchHats => {
            // High-speed 16th hats with 32nd stutter bursts
            for step in 0..16 {
                if step == 7 || step == 15 {
                    // 32nd stutter roll
                    for sub in 0..4 {
                        let vel = 80 + sub * 10;
                        let mut n = make_step(step, vel, ps / 4);
                        n.position += (sub as u32) * (ps / 4);
                        notes.push(n);
                    }
                } else {
                    let vel = if step % 4 == 0 { 110 } else if step % 2 == 0 { 92 } else { 78 };
                    notes.push(make_step(step, vel, ps));
                }
            }
        }
        DrumGroove::DrillBounce => {
            // UK / NY Drill dotted 3-step bounce: 0, 3, 8, 11, 14 + ghost note on 5
            notes.push(make_step(0, 110, ps));
            notes.push(make_step(3, 98, ps));
            notes.push(make_step(5, 55, ps)); // Ghost note
            notes.push(make_step(8, 105, ps));
            notes.push(make_step(11, 95, ps));
            notes.push(make_step(14, 100, ps));
        }
        DrumGroove::Edm4OnFloor => {
            // Kicks on steps 0, 4, 8, 12
            for step in (0..16).step_by(4) {
                notes.push(make_step(step, 118, ps * 2));
            }
        }
        DrumGroove::EdmOffbeatHat => {
            // Open hats on steps 2, 6, 10, 14
            for step in [2, 6, 10, 14] {
                notes.push(make_step(step, 96, ps * 2));
            }
        }
        DrumGroove::TrapSnare => {
            // Snare on steps 4 and 12
            notes.push(make_step(4, 112, ps * 2));
            notes.push(make_step(12, 112, ps * 2));
        }
        DrumGroove::DigicoreGlitchSnare => {
            // Ghost flam at step 3.5
            let mut flam = make_step(3, 62, ps / 2);
            flam.position += ps / 2;
            notes.push(flam);
            // Main hits on 4 and 12
            notes.push(make_step(4, 112, ps * 2));
            notes.push(make_step(12, 112, ps * 2));
            // Double-tap hit on 14
            notes.push(make_step(14, 98, ps));
        }
        DrumGroove::DembowSnare => {
            // Dembow rhythm: 0, 3, 6, 10, 12
            for &step in &[0, 3, 6, 10, 12] {
                let vel = if step == 0 || step == 10 { 110 } else { 96 };
                notes.push(make_step(step, vel, ps));
            }
        }
        DrumGroove::EdmBuildUpSnare => {
            // 1 Bar build-up:
            // Quarter notes on 0, 4
            notes.push(make_step(0, 60, ps * 2));
            notes.push(make_step(4, 75, ps * 2));
            // Eighth notes on 8, 10
            notes.push(make_step(8, 88, ps));
            notes.push(make_step(10, 98, ps));
            // Sixteenth notes on 12, 13, 14, 15
            notes.push(make_step(12, 108, ps));
            notes.push(make_step(13, 114, ps));
            notes.push(make_step(14, 120, ps));
            notes.push(make_step(15, 126, ps));
        }
    }

    let p = piano::pattern_mut(s);
    p.notes.retain(|n| n.channel != ch);
    let count = notes.len();
    p.notes.extend(notes);
    p.notes.sort_by_key(|n| (n.position, n.channel, n.key));
    s.piano.dirty = true;
    s.message = Some(format!("Applied {}: {count} notes placed", groove.title()));
    count
}

/// Applies a 2-bar or 4-bar rich harmonic chord progression to the currently selected channel.
/// Replaces the channel's notes in the pattern with voiced chords.
pub fn apply_chord_progression(s: &mut State, prog: ChordProgression) -> usize {
    let ch = s.selected_channel as u16;
    let ppq = u32::from(s.project.ppq.max(4));
    let chord_len = 2 * ppq; // Half-bar per chord (2 beats)

    piano::snapshot_as(s, "Apply chord progression");

    let make_note = |tick: u32, key: u16, len: u32, vel: u8| -> Note {
        Note {
            position: tick,
            length: len,
            channel: ch,
            key,
            fine_pitch: 120,
            release: 0,
            pan: 64,
            velocity: vel,
            mod_x: 0,
            mod_y: 0,
            flags: 0x4000,
            group: 0,
            unk14: 0,
            repeat: 0,
        }
    };

    let mut notes = Vec::new();

    // Helper to add a voiced chord with a bass root note and upper chord voices
    let mut add_voiced_chord = |tick: u32, bass: u16, chord_keys: &[u16]| {
        // Bass root note (lower octave)
        notes.push(make_note(tick, bass, chord_len, 105));
        // Voiced upper notes
        for &k in chord_keys {
            notes.push(make_note(tick, k, chord_len, 95));
        }
    };

    match prog {
        ChordProgression::DigicoreRoyal => {
            // IVmaj7 - V - iii7 - vi (Key of C / Am)
            // 1. Fmaj7: Bass F2(41), Voicing: F3(53), C4(60), E4(64), A4(69)
            add_voiced_chord(0, 41, &[53, 60, 64, 69]);
            // 2. G: Bass G2(43), Voicing: G3(55), D4(62), G4(67), B4(71)
            add_voiced_chord(chord_len, 43, &[55, 62, 67, 71]);
            // 3. Em7: Bass E2(40), Voicing: E3(52), B3(59), D4(62), G4(67)
            add_voiced_chord(chord_len * 2, 40, &[52, 59, 62, 67]);
            // 4. Am: Bass A2(45), Voicing: A3(57), C4(60), E4(64), A4(69)
            add_voiced_chord(chord_len * 3, 45, &[57, 60, 64, 69]);
        }
        ChordProgression::HyperpopAnthem => {
            // I - V - vi - IV (Key of C)
            // 1. C: Bass C2(36), Voicing: C3(48), G3(55), C4(60), E4(64)
            add_voiced_chord(0, 36, &[48, 55, 60, 64]);
            // 2. G: Bass G2(43), Voicing: G3(55), B3(59), D4(62), G4(67)
            add_voiced_chord(chord_len, 43, &[55, 59, 62, 67]);
            // 3. Am: Bass A2(45), Voicing: A3(57), C4(60), E4(64), A4(69)
            add_voiced_chord(chord_len * 2, 45, &[57, 60, 64, 69]);
            // 4. F: Bass F2(41), Voicing: F3(53), C4(60), F4(65), A4(69)
            add_voiced_chord(chord_len * 3, 41, &[53, 60, 65, 69]);
        }
        ChordProgression::TrapEmotionalMinor => {
            // i - VI - III - VII (Key of Am)
            // 1. Am: Bass A2(45), Voicing: A3(57), C4(60), E4(64)
            add_voiced_chord(0, 45, &[57, 60, 64]);
            // 2. F: Bass F2(41), Voicing: F3(53), C4(60), A4(69)
            add_voiced_chord(chord_len, 41, &[53, 60, 69]);
            // 3. C: Bass C2(36), Voicing: C3(48), G3(55), E4(64)
            add_voiced_chord(chord_len * 2, 36, &[48, 55, 64]);
            // 4. G: Bass G2(43), Voicing: G3(55), D4(62), B4(71)
            add_voiced_chord(chord_len * 3, 43, &[55, 62, 71]);
        }
        ChordProgression::EdmFutureBassStabs => {
            // vi7 - IVmaj7 - I - V with syncopated Future Bass rhythm stabs
            let stab_len = ppq / 2; // 8th note stabs
            let chords = [
                (45, [57, 60, 64, 71]), // Am9
                (41, [53, 60, 64, 69]), // Fmaj7
                (36, [48, 55, 59, 64]), // Cmaj7
                (43, [55, 60, 62, 67]), // Gsus4
            ];
            for (chord_i, (bass, keys)) in chords.iter().enumerate() {
                let base_tick = (chord_i as u32) * chord_len;
                // Two syncopated stabs per chord: at tick 0 and at tick + 3/4 beat
                for &offset in &[0u32, ppq * 3 / 4] {
                    let tick = base_tick + offset;
                    notes.push(make_note(tick, *bass, stab_len, 108));
                    for &k in keys {
                        notes.push(make_note(tick, k, stab_len, 100));
                    }
                }
            }
        }
        ChordProgression::NeoSoulJazz251 => {
            // ii9 - V13 - Imaj9 - vi7
            // 1. Dm9: Bass D2(38), Voicing: D3(50), F3(53), A3(57), C4(60), E4(64)
            add_voiced_chord(0, 38, &[50, 53, 57, 60, 64]);
            // 2. G13: Bass G2(43), Voicing: F3(53), B3(59), E4(64), A4(69)
            add_voiced_chord(chord_len, 43, &[53, 59, 64, 69]);
            // 3. Cmaj9: Bass C2(36), Voicing: E3(52), G3(55), B3(59), D4(62)
            add_voiced_chord(chord_len * 2, 36, &[52, 55, 59, 62]);
            // 4. Am7: Bass A2(45), Voicing: G3(55), C4(60), E4(64), A4(69)
            add_voiced_chord(chord_len * 3, 45, &[55, 60, 64, 69]);
        }
        ChordProgression::DigicoreCyberPhrygian => {
            // i - bII - i - bVII (Key of Am Phrygian)
            // 1. Am: Bass A2(45), Voicing: A3(57), C4(60), E4(64)
            add_voiced_chord(0, 45, &[57, 60, 64]);
            // 2. Bb: Bass Bb2(46), Voicing: Bb3(58), D4(62), F4(65)
            add_voiced_chord(chord_len, 46, &[58, 62, 65]);
            // 3. Am: Bass A2(45), Voicing: A3(57), C4(60), E4(64)
            add_voiced_chord(chord_len * 2, 45, &[57, 60, 64]);
            // 4. G: Bass G2(43), Voicing: G3(55), B3(59), D4(62)
            add_voiced_chord(chord_len * 3, 43, &[55, 59, 62]);
        }
    }

    let p = piano::pattern_mut(s);
    p.notes.retain(|n| n.channel != ch);
    let count = notes.len();
    p.notes.extend(notes);
    p.notes.sort_by_key(|n| (n.position, n.channel, n.key));
    s.piano.dirty = true;
    s.message = Some(format!("Applied {}: {count} notes placed", prog.title()));
    count
}

/// Preset song arrangement structures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrangementStyle {
    TrapStructure,
    DigicoreStructure,
    EdmStructure,
}

impl ArrangementStyle {
    pub fn title(&self) -> &'static str {
        match self {
            ArrangementStyle::TrapStructure => "Generate Arrangement: Trap / Hip-Hop (Intro, Drop, Verse, Outro)",
            ArrangementStyle::DigicoreStructure => "Generate Arrangement: Digicore / Hyperpop (Build, Drop, Glitch, Outro)",
            ArrangementStyle::EdmStructure => "Generate Arrangement: EDM / Dance (Intro, Build, Drop, Breakdown, Drop 2)",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            ArrangementStyle::TrapStructure => "28-bar structure with Intro, Hook, Verse, and Outro markers & pattern layout".into(),
            ArrangementStyle::DigicoreStructure => "24-bar high-energy structure with fast build, frantic drops, and breakdown".into(),
            ArrangementStyle::EdmStructure => "32-bar festival layout with driving build, drop stabs, and euphoric breakdown".into(),
        }
    }
}

/// Generates a complete song arrangement structure with time markers and pattern clip layout.
pub fn generate_arrangement(s: &mut State, style: ArrangementStyle) -> String {
    crate::playlist::snapshot_as(s, "Generate arrangement");
    let ppq = u32::from(s.project.ppq.max(4));
    let bar = ppq * 4;
    let pat = s.pattern;

    let arr = crate::playlist::arr_mut(s);
    arr.markers.clear();

    match style {
        ArrangementStyle::TrapStructure => {
            arr.markers.push(os_model::TimeMarker::new(0, "Intro (4 bars)"));
            arr.markers.push(os_model::TimeMarker::new(4 * bar, "Hook / Drop (8 bars)"));
            arr.markers.push(os_model::TimeMarker::new(12 * bar, "Verse (8 bars)"));
            arr.markers.push(os_model::TimeMarker::new(20 * bar, "Breakdown (4 bars)"));
            arr.markers.push(os_model::TimeMarker::new(24 * bar, "Outro (4 bars)"));

            let src = os_model::ClipSource::Pattern(pat);
            for i in 0..7 {
                arr.clips.push(os_model::Clip::new(i * 4 * bar, 4 * bar, 0, src));
            }
            arr.clips.push(os_model::Clip::new(4 * bar, 8 * bar, 1, src));
            arr.clips.push(os_model::Clip::new(16 * bar, 4 * bar, 1, src));
            arr.clips.push(os_model::Clip::new(4 * bar, 8 * bar, 2, src));
            arr.clips.push(os_model::Clip::new(20 * bar, 4 * bar, 2, src));
        }
        ArrangementStyle::DigicoreStructure => {
            arr.markers.push(os_model::TimeMarker::new(0, "Buildup (2 bars)"));
            arr.markers.push(os_model::TimeMarker::new(2 * bar, "Drop 1 (8 bars)"));
            arr.markers.push(os_model::TimeMarker::new(10 * bar, "Glitch Breakdown (4 bars)"));
            arr.markers.push(os_model::TimeMarker::new(14 * bar, "Drop 2 - Full Energy (8 bars)"));
            arr.markers.push(os_model::TimeMarker::new(22 * bar, "Outro (2 bars)"));

            let src = os_model::ClipSource::Pattern(pat);
            for i in 0..6 {
                arr.clips.push(os_model::Clip::new(i * 4 * bar, 4 * bar, 0, src));
            }
            arr.clips.push(os_model::Clip::new(2 * bar, 8 * bar, 1, src));
            arr.clips.push(os_model::Clip::new(14 * bar, 8 * bar, 1, src));
            arr.clips.push(os_model::Clip::new(2 * bar, 8 * bar, 2, src));
            arr.clips.push(os_model::Clip::new(14 * bar, 8 * bar, 2, src));
        }
        ArrangementStyle::EdmStructure => {
            arr.markers.push(os_model::TimeMarker::new(0, "Intro (8 bars)"));
            arr.markers.push(os_model::TimeMarker::new(8 * bar, "Buildup (4 bars)"));
            arr.markers.push(os_model::TimeMarker::new(12 * bar, "Drop (8 bars)"));
            arr.markers.push(os_model::TimeMarker::new(20 * bar, "Breakdown (8 bars)"));
            arr.markers.push(os_model::TimeMarker::new(28 * bar, "Outro (4 bars)"));

            let src = os_model::ClipSource::Pattern(pat);
            for i in 0..8 {
                arr.clips.push(os_model::Clip::new(i * 4 * bar, 4 * bar, 0, src));
            }
            arr.clips.push(os_model::Clip::new(12 * bar, 8 * bar, 1, src));
            arr.clips.push(os_model::Clip::new(12 * bar, 8 * bar, 2, src));
        }
    }
    s.piano.dirty = true;
    format!("Playlist: generated {} with markers & track layout", style.title())
}

/// Generates a syncopated 808 sub-bassline tailored to active chord notes with slides and octave jumps.
pub fn generate_808_bassline(s: &mut State) -> String {
    let ch = s.selected_channel as u16;
    let ppq = u32::from(s.project.ppq.max(4));
    let bar = ppq * 4;
    let step = ppq / 4;

    piano::snapshot_as(s, "Generate 808 bassline");

    let p = piano::pattern_mut(s);
    let mut root_notes: Vec<(u32, u16)> = Vec::new();

    let window_size = ppq * 2;
    let max_pos = p.notes.iter().map(|n| n.position + n.length).max().unwrap_or(4 * bar);
    let total_windows = (max_pos / window_size).max(8);

    for w in 0..total_windows {
        let t_start = w * window_size;
        let t_end = t_start + window_size;
        let notes_in_win: Vec<&Note> = p.notes.iter()
            .filter(|n| n.position >= t_start && n.position < t_end)
            .collect();

        if let Some(lowest) = notes_in_win.iter().min_by_key(|n| n.key) {
            let pitch_class = (lowest.key % 12) as u16;
            let sub_key = 36 + pitch_class;
            root_notes.push((t_start, sub_key));
        }
    }

    if root_notes.is_empty() {
        let roots = [41, 43, 40, 45]; // F2, G2, E2, A2
        for (i, &r) in roots.iter().cycle().take(8).enumerate() {
            root_notes.push(((i as u32) * window_size, r));
        }
    }

    let make_808 = |tick: u32, key: u16, len: u32, vel: u8, slide: bool| -> Note {
        let mut n = Note {
            position: tick,
            length: len,
            channel: ch,
            key,
            fine_pitch: 120,
            release: 0,
            pan: 64,
            velocity: vel,
            mod_x: 0,
            mod_y: 0,
            flags: 0x4000,
            group: 0,
            unk14: 0,
            repeat: 0,
        };
        if slide {
            n.set_slide(true);
        }
        n
    };

    let mut bass_notes = Vec::new();

    for (i, &(t0, root)) in root_notes.iter().enumerate() {
        bass_notes.push(make_808(t0, root, ppq + ppq / 2, 115, false));

        let push_tick = t0 + step * 6;
        bass_notes.push(make_808(push_tick, root, ppq, 102, false));

        if i % 2 == 1 {
            let slide_start = t0 + window_size - step * 2;
            let slide_key = (root + 12).min(127);
            bass_notes.push(make_808(slide_start, slide_key, step * 2, 110, true));
        } else if i % 4 == 2 {
            let bounce_tick = t0 + step * 10;
            let fifth_key = (root + 7).min(127);
            bass_notes.push(make_808(bounce_tick, fifth_key, step * 2, 95, false));
        }
    }

    p.notes.retain(|n| n.channel != ch);
    let count = bass_notes.len();
    p.notes.extend(bass_notes);
    p.notes.sort_by_key(|n| (n.position, n.channel, n.key));
    s.piano.dirty = true;
    format!("Generated syncopated 808 bassline: {count} notes placed (with slides & octave jumps)")
}

/// Toggles the slide flag on all selected notes (or active channel notes).
pub fn toggle_slide_notes(s: &mut State) -> String {
    let targets = piano::targets(s);
    if targets.is_empty() {
        return "No notes selected to toggle slide".into();
    }
    piano::snapshot_as(s, "Toggle slide notes");
    let p = piano::pattern_mut(s);
    let mut toggled = 0;
    for &i in &targets {
        if let Some(n) = p.notes.get_mut(i) {
            let cur = n.slide();
            n.set_slide(!cur);
            toggled += 1;
        }
    }
    s.piano.dirty = true;
    format!("Toggled slide flag on {toggled} note(s)")
}

/// Appends an octave slide-up note (+12 st) to the tail of each target note.
pub fn add_octave_slide(s: &mut State) -> String {
    let targets = piano::targets(s);
    if targets.is_empty() {
        return "No notes selected".into();
    }
    let all = piano::notes(s);
    let mut slides_to_add = Vec::new();
    for &i in &targets {
        if let Some(n) = all.get(i) {
            let slide_len = (n.length / 4).max(12);
            let slide_pos = n.position + n.length.saturating_sub(slide_len);
            let mut slide_note = *n;
            slide_note.position = slide_pos;
            slide_note.length = slide_len;
            slide_note.key = (n.key + 12).min(127);
            slide_note.set_slide(true);
            slides_to_add.push(slide_note);
        }
    }
    if slides_to_add.is_empty() {
        return "Could not add slide note".into();
    }
    piano::snapshot_as(s, "Add octave slide note");
    let count = slides_to_add.len();
    let p = piano::pattern_mut(s);
    p.notes.extend(slides_to_add);
    p.notes.sort_by_key(|n| (n.position, n.channel, n.key));
    s.piano.dirty = true;
    format!("Added {count} octave slide-up glide(s)")
}

/// Humanizes notes by subtly jittering velocity and micro-timing.
pub fn humanize_notes(s: &mut State, vel_spread: i32, tick_spread: i32) -> String {
    let targets = piano::targets(s);
    if targets.is_empty() {
        return "No notes to humanize".into();
    }
    piano::snapshot_as(s, "Humanize notes");
    let mut rng = s.piano.rng;
    let mut next_val = || {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        (rng >> 33) as u32
    };

    let p = piano::pattern_mut(s);
    let mut modified = 0;
    for &i in &targets {
        if let Some(n) = p.notes.get_mut(i) {
            let v_range = vel_spread * 2 + 1;
            let v_offset = (next_val() % (v_range as u32)) as i32 - vel_spread;
            n.velocity = ((n.velocity as i32) + v_offset).clamp(1, 127) as u8;

            let t_range = tick_spread * 2 + 1;
            let t_offset = (next_val() % (t_range as u32)) as i32 - tick_spread;
            n.position = ((n.position as i64) + (t_offset as i64)).max(0) as u32;

            modified += 1;
        }
    }
    s.piano.rng = rng;
    s.piano.dirty = true;
    format!("Humanized {modified} note(s) (velocity \u{00b1}{vel_spread}, timing \u{00b1}{tick_spread} ticks)")
}

/// Snaps all target notes to the currently highlighted scale in the piano roll.
pub fn snap_notes_to_scale(s: &mut State) -> String {
    let targets = piano::targets(s);
    if targets.is_empty() {
        return "No notes to snap to scale".into();
    }
    let root = s.piano.scale_highlight.root;
    let scale_idx = s.piano.scale_highlight.scale_idx;

    piano::snapshot_as(s, "Snap notes to scale");
    let p = piano::pattern_mut(s);
    let mut snapped = 0;

    for &i in &targets {
        if let Some(n) = p.notes.get_mut(i) {
            let key = n.key as i32;
            if !crate::chords::in_scale(root, scale_idx, key) {
                let mut best = key;
                for delta in 1..=6 {
                    if crate::chords::in_scale(root, scale_idx, key - delta) {
                        best = key - delta;
                        break;
                    }
                    if crate::chords::in_scale(root, scale_idx, key + delta) {
                        best = key + delta;
                        break;
                    }
                }
                n.key = best.clamp(0, 127) as u16;
                snapped += 1;
            }
        }
    }
    s.piano.dirty = true;
    format!("Snapped {snapped} out-of-scale note(s) to scale")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drum_grooves_place_notes() {
        let mut app = crate::app::App::new(None);
        let s = &mut app.state;
        s.project.ppq = 96;

        let n = apply_drum_groove(s, DrumGroove::Trap2Step);
        assert_eq!(n, 8, "8 notes in 2-step 1-bar hi-hat");

        let n_drill = apply_drum_groove(s, DrumGroove::DrillBounce);
        assert_eq!(n_drill, 6, "6 notes in drill bounce");

        let n_edm = apply_drum_groove(s, DrumGroove::Edm4OnFloor);
        assert_eq!(n_edm, 4, "4 kicks in four-on-the-floor");
    }

    #[test]
    fn test_chord_progressions_place_notes() {
        let mut app = crate::app::App::new(None);
        let s = &mut app.state;
        s.project.ppq = 96;

        let n = apply_chord_progression(s, ChordProgression::DigicoreRoyal);
        assert!(n >= 16, "Royal anime progression has rich 4-chord voicings");

        let n_anthem = apply_chord_progression(s, ChordProgression::HyperpopAnthem);
        assert!(n_anthem >= 16);
    }

    #[test]
    fn test_808_bassline_and_slides() {
        let mut app = crate::app::App::new(None);
        let s = &mut app.state;
        s.project.ppq = 96;

        // Apply chords first
        apply_chord_progression(s, ChordProgression::DigicoreRoyal);

        // Generate 808 bassline on channel 0
        let msg = generate_808_bassline(s);
        assert!(msg.contains("Generated syncopated 808 bassline"));
        let p = piano::pattern_mut(s);
        let slides = p.notes.iter().filter(|n| n.slide()).count();
        assert!(slides > 0, "808 generator places slide notes");

        // Toggle slides
        let toggle_msg = toggle_slide_notes(s);
        assert!(toggle_msg.contains("Toggled slide flag"));

        // Octave slide
        let add_msg = add_octave_slide(s);
        assert!(add_msg.contains("Added"));

        // Humanize
        let hum_msg = humanize_notes(s, 6, 3);
        assert!(hum_msg.contains("Humanized"));

        // Snap to scale (e.g. C Major)
        s.piano.scale_highlight.root = 0; // C
        s.piano.scale_highlight.scale_idx = crate::chords::FIRST_SCALE;
        let snap_msg = snap_notes_to_scale(s);
        assert!(snap_msg.contains("Snapped"));
    }

    #[test]
    fn test_generate_arrangement() {
        let mut app = crate::app::App::new(None);
        let s = &mut app.state;
        s.project.ppq = 96;

        let msg = generate_arrangement(s, ArrangementStyle::TrapStructure);
        assert!(msg.contains("Trap / Hip-Hop"));

        let arr = crate::playlist::arr_mut(s);
        assert_eq!(arr.markers.len(), 5, "Trap structure has 5 markers");
        assert!(!arr.clips.is_empty(), "Trap structure placed pattern clips across tracks");
    }
}
