// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Omni Command Palette (Phase 4): Global searchable action runner, plugin inserter, and navigation hub.
//!
//! Toggled globally via `Ctrl+K` (or `Cmd+K` on macOS) and View ▸ Command palette.
//! Provides instant fuzzy filtering over DAW actions, native & scanned plugins, open windows,
//! channels, and mixer tracks. Supports full keyboard navigation (Up/Down, Enter, Esc).

use os_ui::Color;
use crate::app::{Action, App, CloseAll, State, Win};

/// Categories for command palette entries, rendered with distinct badge colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Action,
    Window,
    Plugin,
    Effect,
    Channel,
    Mixer,
    Automation,
    Groove,
    Arrangement,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::Action => "ACTION",
            Category::Window => "WINDOW",
            Category::Plugin => "PLUGIN",
            Category::Effect => "EFFECT",
            Category::Channel => "CHANNEL",
            Category::Mixer => "MIXER",
            Category::Automation => "AUTO",
            Category::Groove => "GROOVE",
            Category::Arrangement => "ARRANGE",
        }
    }

    /// Returns (background, text) color for the category badge.
    pub fn colors(&self) -> (Color, Color) {
        match self {
            Category::Action => (Color::rgb(0x183050), Color::rgb(0x60a5fa)),
            Category::Window => (Color::rgb(0x14402a), Color::rgb(0x34d399)),
            Category::Plugin => (Color::rgb(0x483010), Color::rgb(0xfbbf24)),
            Category::Effect => (Color::rgb(0x3b1d54), Color::rgb(0xc084fc)),
            Category::Channel => (Color::rgb(0x4a1834), Color::rgb(0xf472b6)),
            Category::Mixer => (Color::rgb(0x133848), Color::rgb(0x38bdf8)),
            Category::Automation => (Color::rgb(0x402208), Color::rgb(0xfb923c)),
            Category::Groove => (Color::rgb(0x421236), Color::rgb(0xf43f5e)),
            Category::Arrangement => (Color::rgb(0x0e3a3a), Color::rgb(0x2dd4bf)),
        }
    }
}

/// An executable action triggered by choosing a palette entry.
#[derive(Clone, Debug, PartialEq)]
pub enum PaletteAction {
    DawAction(Action),
    AddGenerator(usize),
    AddEffect(usize),
    FocusWindow(Win),
    FocusChannel(usize),
    FocusMixer(u16),
    OpenSettings(usize),
    ChopTransients,
    ChopGrid(crate::playlist::ChopGrid),
    ChopStutter(usize),
    SeparateStems(crate::stem_separator::StemMode),
    CloseAll,
    CloseUnfocused,
    ClosePlugins,
    ToggleScaleHighlight,
    CycleNoteDisplay,
    CycleKeyLabels,
    StampChord(usize),
    ClearStamp,
    ColorGradientTracks,
    TransposeChannel(i32),
    LastTweakedAutomation,
    LastTweakedLink,
    AddAutomationClip,
    AutoSidechainDucking,
    ApplyDrumGroove(crate::grooves::DrumGroove),
    ApplyChordProgression(crate::grooves::ChordProgression),
    Generate808,
    ToggleSlide,
    AddOctaveSlide,
    HumanizeNotes,
    SnapNotesToScale,
    TransposeClips(i16),
    GenerateArrangement(crate::grooves::ArrangementStyle),
}

/// A single item searchable within the command palette.
#[derive(Clone, Debug)]
pub struct PaletteItem {
    pub title: String,
    pub subtitle: String,
    pub category: Category,
    pub shortcut: Option<&'static str>,
    pub action: PaletteAction,
}

/// Runtime state for the command palette.
#[derive(Clone, Debug, Default)]
pub struct Palette {
    pub open: bool,
    pub query: String,
    pub selected: usize,
    pub scroll: usize,
}

/// A matched item with calculated fuzzy score.
#[derive(Clone, Debug)]
pub struct ScoredItem<'a> {
    pub item: &'a PaletteItem,
    pub original_index: usize,
    pub score: i32,
}

pub const PAGE_SIZE: usize = 9;

/// Gathers all searchable items given the current DAW state.
pub fn items(s: &State) -> Vec<PaletteItem> {
    let mut list = Vec::with_capacity(128);

    // 1. Windows & Primary Workspaces
    list.push(PaletteItem {
        title: "Focus Playlist".into(),
        subtitle: "Main arrangement & timeline workspace".into(),
        category: Category::Window,
        shortcut: Some("F5"),
        action: PaletteAction::FocusWindow(Win::Playlist),
    });
    list.push(PaletteItem {
        title: "Focus Piano Roll".into(),
        subtitle: "Note sequencer & melody editor".into(),
        category: Category::Window,
        shortcut: Some("F7"),
        action: PaletteAction::FocusWindow(Win::PianoRoll),
    });
    list.push(PaletteItem {
        title: "Focus Channel Rack".into(),
        subtitle: "Step sequencer & instrument rack".into(),
        category: Category::Window,
        shortcut: Some("F6"),
        action: PaletteAction::FocusWindow(Win::ChannelRack),
    });
    list.push(PaletteItem {
        title: "Focus Mixer".into(),
        subtitle: "Audio routing, inserts & effects rack".into(),
        category: Category::Window,
        shortcut: Some("F9"),
        action: PaletteAction::FocusWindow(Win::Mixer),
    });
    list.push(PaletteItem {
        title: "Toggle Browser".into(),
        subtitle: "Samples, presets, and project tree sidebar".into(),
        category: Category::Window,
        shortcut: Some("Alt+F8"),
        action: PaletteAction::DawAction(Action::ToggleWin(Win::Browser)),
    });
    list.push(PaletteItem {
        title: "Toggle Bottom Device Strip".into(),
        subtitle: "Docked signal chain, macro knobs & quick slot controls".into(),
        category: Category::Window,
        shortcut: Some("Alt+D"),
        action: PaletteAction::DawAction(Action::ToggleDeviceStrip),
    });
    list.push(PaletteItem {
        title: "Focus Plugin Performance Monitor".into(),
        subtitle: "CPU usage & processing latency per plugin".into(),
        category: Category::Window,
        shortcut: None,
        action: PaletteAction::FocusWindow(Win::Perf),
    });
    list.push(PaletteItem {
        title: "Focus Tempo Tapper".into(),
        subtitle: "Tap tempo BPM calculation tool".into(),
        category: Category::Window,
        shortcut: Some("Ctrl+F5"),
        action: PaletteAction::FocusWindow(Win::Tapper),
    });
    list.push(PaletteItem {
        title: "Focus Touch Controller".into(),
        subtitle: "Touch pads and on-screen keyboard".into(),
        category: Category::Window,
        shortcut: Some("Alt+F7"),
        action: PaletteAction::FocusWindow(Win::Touch),
    });
    list.push(PaletteItem {
        title: "Open Project Picker".into(),
        subtitle: "Visual grid of patterns and audio clips".into(),
        category: Category::Window,
        shortcut: Some("Ctrl+F8"),
        action: PaletteAction::DawAction(Action::ProjectPicker),
    });
    list.push(PaletteItem {
        title: "Open Plugin Picker".into(),
        subtitle: "Visual grid of installed instruments and effects".into(),
        category: Category::Window,
        shortcut: Some("F8"),
        action: PaletteAction::DawAction(Action::PickerOpen),
    });

    // 2. Core DAW Actions & Transport
    list.push(PaletteItem {
        title: "Play / Pause".into(),
        subtitle: "Start or pause playback from current position".into(),
        category: Category::Action,
        shortcut: Some("Space"),
        action: PaletteAction::DawAction(Action::Play),
    });
    list.push(PaletteItem {
        title: "Stop Playback".into(),
        subtitle: "Stop playback and rewind to song/pattern start".into(),
        category: Category::Action,
        shortcut: Some("Space"),
        action: PaletteAction::DawAction(Action::Stop),
    });
    list.push(PaletteItem {
        title: "Toggle Song / Pattern Mode".into(),
        subtitle: "Switch between Pattern and Song arrangement playback".into(),
        category: Category::Action,
        shortcut: Some("L"),
        action: PaletteAction::DawAction(Action::PatSong(!s.song_mode)),
    });
    list.push(PaletteItem {
        title: "Toggle Recording".into(),
        subtitle: "Arm audio and MIDI recording".into(),
        category: Category::Action,
        shortcut: Some("R"),
        action: PaletteAction::DawAction(Action::Record),
    });
    list.push(PaletteItem {
        title: "Toggle Metronome".into(),
        subtitle: "Audible click track during playback and recording".into(),
        category: Category::Action,
        shortcut: Some("Ctrl+M"),
        action: PaletteAction::DawAction(Action::Metronome),
    });
    list.push(PaletteItem {
        title: "Toggle Typing Keyboard to Piano".into(),
        subtitle: "Use QWERTY keyboard as MIDI note controller".into(),
        category: Category::Action,
        shortcut: Some("Ctrl+T"),
        action: PaletteAction::DawAction(Action::Typing),
    });
    list.push(PaletteItem {
        title: "Toggle Step Edit Mode".into(),
        subtitle: "Advance note cursor step-by-step on input".into(),
        category: Category::Action,
        shortcut: Some("Ctrl+E"),
        action: PaletteAction::DawAction(Action::StepEdit),
    });
    list.push(PaletteItem {
        title: "Toggle Auto-Scrolling".into(),
        subtitle: "Keep playback cursor in view continuously".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::DawAction(Action::AutoScroll),
    });
    list.push(PaletteItem {
        title: "Toggle Enable Groups".into(),
        subtitle: "Control grouped channel parameters simultaneously".into(),
        category: Category::Action,
        shortcut: Some("Shift+Alt+G"),
        action: PaletteAction::DawAction(Action::EnableGroups),
    });
    list.push(PaletteItem {
        title: "Quantize Notes (Tools Menu)".into(),
        subtitle: "Align selected notes to current snap grid".into(),
        category: Category::Action,
        shortcut: Some("Alt+Q"),
        action: PaletteAction::DawAction(Action::PianoToolsMenu),
    });
    list.push(PaletteItem {
        title: "Toggle Scale Highlighting".into(),
        subtitle: "Highlight in-key notes and root tone on piano roll".into(),
        category: Category::Action,
        shortcut: Some("Alt+H"),
        action: PaletteAction::ToggleScaleHighlight,
    });
    list.push(PaletteItem {
        title: "Cycle Note Pitch Display".into(),
        subtitle: "Switch between note names (C5), MIDI numbers (60), or none".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::CycleNoteDisplay,
    });
    list.push(PaletteItem {
        title: "Cycle Piano Key Labels".into(),
        subtitle: "Switch between all white notes, root notes only, or none".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::CycleKeyLabels,
    });
    list.push(PaletteItem {
        title: "Color Selected: Gradient".into(),
        subtitle: "Interpolate colors across selected playlist tracks, mixer tracks, or channels".into(),
        category: Category::Action,
        shortcut: Some("Shift+Alt+C"),
        action: PaletteAction::ColorGradientTracks,
    });
    list.push(PaletteItem {
        title: "Transpose Selected +1 Octave (+12)".into(),
        subtitle: "Shift selected channel notes up by 12 semitones".into(),
        category: Category::Action,
        shortcut: Some("Ctrl+Up"),
        action: PaletteAction::TransposeChannel(12),
    });
    list.push(PaletteItem {
        title: "Transpose Selected -1 Octave (-12)".into(),
        subtitle: "Shift selected channel notes down by 12 semitones".into(),
        category: Category::Action,
        shortcut: Some("Ctrl+Down"),
        action: PaletteAction::TransposeChannel(-12),
    });
    list.push(PaletteItem {
        title: "Transpose Selected +1 Semitone".into(),
        subtitle: "Shift selected channel notes up by 1 semitone".into(),
        category: Category::Action,
        shortcut: Some("Shift+Up"),
        action: PaletteAction::TransposeChannel(1),
    });
    list.push(PaletteItem {
        title: "Transpose Selected -1 Semitone".into(),
        subtitle: "Shift selected channel notes down by 1 semitone".into(),
        category: Category::Action,
        shortcut: Some("Shift+Down"),
        action: PaletteAction::TransposeChannel(-1),
    });
    list.push(PaletteItem {
        title: "Stamp Tool: Major Chord".into(),
        subtitle: "Left-click piano roll grid to place a Major chord".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::StampChord(1),
    });
    list.push(PaletteItem {
        title: "Stamp Tool: Minor Chord".into(),
        subtitle: "Left-click piano roll grid to place a minor chord".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::StampChord(5),
    });
    list.push(PaletteItem {
        title: "Stamp Tool: 7th Chord".into(),
        subtitle: "Left-click piano roll grid to place a dominant 7th chord".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::StampChord(15),
    });
    list.push(PaletteItem {
        title: "Stamp Tool: Minor 7th Chord".into(),
        subtitle: "Left-click piano roll grid to place a minor 7th chord".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::StampChord(32),
    });
    list.push(PaletteItem {
        title: "Stamp Tool: Major 7th Chord".into(),
        subtitle: "Left-click piano roll grid to place a Major 7th chord".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::StampChord(27),
    });
    list.push(PaletteItem {
        title: "Clear Stamp Tool".into(),
        subtitle: "Return to normal single-note draw tool".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::ClearStamp,
    });
    list.push(PaletteItem {
        title: "Reverse Audio Clips".into(),
        subtitle: "Flip selected playlist audio clips backward in time".into(),
        category: Category::Action,
        shortcut: Some("Shift+R"),
        action: PaletteAction::DawAction(Action::PlTool(crate::playlist::PlTool::Select)),
    });
    list.push(PaletteItem {
        title: "Slice at Transients (Drum Chopper)".into(),
        subtitle: "Cut selected audio clips along acoustic onsets at zero-crossings".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::ChopTransients,
    });
    list.push(PaletteItem {
        title: "Chop to Grid: Beat (1/4)".into(),
        subtitle: "Slice selected clips every quarter-note beat".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::ChopGrid(crate::playlist::ChopGrid::Beat),
    });
    list.push(PaletteItem {
        title: "Chop to Grid: Half-Beat (1/8)".into(),
        subtitle: "Slice selected clips every eighth-note".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::ChopGrid(crate::playlist::ChopGrid::HalfBeat),
    });
    list.push(PaletteItem {
        title: "Chop to Grid: Step (1/16)".into(),
        subtitle: "Slice selected clips every sixteenth-note step".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::ChopGrid(crate::playlist::ChopGrid::Step),
    });
    list.push(PaletteItem {
        title: "Chop to Grid: Bar (1/1)".into(),
        subtitle: "Slice selected clips every bar".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::ChopGrid(crate::playlist::ChopGrid::Bar),
    });
    list.push(PaletteItem {
        title: "Stutter Clips (4x Repeat)".into(),
        subtitle: "Split and repeat first slice 4 times for quick glitch rolls".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::ChopStutter(4),
    });
    list.push(PaletteItem {
        title: "Stutter Clips (8x Roll)".into(),
        subtitle: "Split and repeat first slice 8 times for rapid drum rolls".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::ChopStutter(8),
    });
    list.push(PaletteItem {
        title: "Stutter Clips (16x Glitch Roll)".into(),
        subtitle: "Split and repeat first slice 16 times for hyper-fast digicore glitch stutters".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::ChopStutter(16),
    });
    list.push(PaletteItem {
        title: "Chop to Grid: 32nd Note (1/32 Glitch)".into(),
        subtitle: "Micro-slice selected clips every 32nd note for digicore granular chops".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::ChopGrid(crate::playlist::ChopGrid::ThirtySecond),
    });
    list.push(PaletteItem {
        title: "Chop to Grid: Triplet (1/6)".into(),
        subtitle: "Slice selected clips to 1/6 triplet grid divisions".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::ChopGrid(crate::playlist::ChopGrid::TripletSixth),
    });
    // Audio Stem Separation (Phase 6)
    use crate::stem_separator::StemMode;
    list.push(PaletteItem {
        title: "Spectral split: 8 layers (experimental)".into(),
        subtitle: "Frequency/stereo estimates; not AI isolation. Short source files only.".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::SeparateStems(StemMode::Stems8),
    });
    list.push(PaletteItem {
        title: "Spectral split: 6 layers (experimental)".into(),
        subtitle: "Experimental frequency/stereo estimates; short source files only.".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::SeparateStems(StemMode::Stems6),
    });
    list.push(PaletteItem {
        title: "Spectral split: 4 layers (experimental)".into(),
        subtitle: "Experimental frequency/stereo estimates; short source files only.".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::SeparateStems(StemMode::All4),
    });
    list.push(PaletteItem {
        title: "Spectral split: vocal-range & music (experimental)".into(),
        subtitle: "Frequency/stereo estimates, not isolated vocals. Short source files only.".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::SeparateStems(StemMode::VocalsAndMusic),
    });
    list.push(PaletteItem {
        title: "Spectral split: transients & melodic (experimental)".into(),
        subtitle: "Transient/frequency estimates, not isolated drums. Short source files only.".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::SeparateStems(StemMode::DrumsAndMelody),
    });
    list.push(PaletteItem {
        title: "Undo".into(),
        subtitle: "Revert the last edit or parameter change".into(),
        category: Category::Action,
        shortcut: Some("Ctrl+Z"),
        action: PaletteAction::DawAction(Action::PianoUndo),
    });
    list.push(PaletteItem {
        title: "New Project".into(),
        subtitle: "Start a fresh project from the default template".into(),
        category: Category::Action,
        shortcut: Some("Ctrl+N"),
        action: PaletteAction::DawAction(Action::MenuItem(0, crate::views::menus::FILE_NEW)),
    });
    list.push(PaletteItem {
        title: "Open Project...".into(),
        subtitle: "Open an existing Lemon Studio or FLP project".into(),
        category: Category::Action,
        shortcut: Some("Ctrl+O"),
        action: PaletteAction::DawAction(Action::MenuItem(0, crate::views::menus::FILE_OPEN)),
    });
    list.push(PaletteItem {
        title: "Save Project".into(),
        subtitle: "Save changes to current project file".into(),
        category: Category::Action,
        shortcut: Some("Ctrl+S"),
        action: PaletteAction::DawAction(Action::MenuItem(0, crate::views::menus::FILE_SAVE)),
    });
    list.push(PaletteItem {
        title: "Save Project As...".into(),
        subtitle: "Save project to a new filename or folder".into(),
        category: Category::Action,
        shortcut: Some("Ctrl+Shift+S"),
        action: PaletteAction::DawAction(Action::MenuItem(0, crate::views::menus::FILE_SAVE_AS)),
    });
    list.push(PaletteItem {
        title: "Export as WAV Audio...".into(),
        subtitle: "Render project or loop to uncompressed 16/24/32-bit WAV".into(),
        category: Category::Action,
        shortcut: Some("Ctrl+R"),
        action: PaletteAction::DawAction(Action::MenuItem(0, crate::views::menus::FILE_EXPORT)),
    });
    list.push(PaletteItem {
        title: "Export as MP3 Audio...".into(),
        subtitle: "Render project to compressed MP3".into(),
        category: Category::Action,
        shortcut: Some("Ctrl+Shift+R"),
        action: PaletteAction::DawAction(Action::MenuItem(0, crate::views::menus::FILE_EXPORT_MP3)),
    });
    list.push(PaletteItem {
        title: "Export as MIDI File...".into(),
        subtitle: "Export all pattern notes to standard MIDI file".into(),
        category: Category::Action,
        shortcut: Some("Ctrl+Shift+M"),
        action: PaletteAction::DawAction(Action::MenuItem(0, crate::views::menus::FILE_EXPORT_MIDI)),
    });
    list.push(PaletteItem {
        title: "Close All Windows".into(),
        subtitle: "Hide all floating sub-windows".into(),
        category: Category::Action,
        shortcut: Some("F12"),
        action: PaletteAction::CloseAll,
    });
    list.push(PaletteItem {
        title: "Close All Unfocused Windows".into(),
        subtitle: "Hide all background windows, keeping active focused".into(),
        category: Category::Action,
        shortcut: Some("Ctrl+F12"),
        action: PaletteAction::CloseUnfocused,
    });
    list.push(PaletteItem {
        title: "Close All Plugin Windows".into(),
        subtitle: "Dismiss all hosted and native plugin GUI windows".into(),
        category: Category::Action,
        shortcut: Some("Alt+F12"),
        action: PaletteAction::ClosePlugins,
    });
    list.push(PaletteItem {
        title: "Add Pattern".into(),
        subtitle: "Create a new musical pattern".into(),
        category: Category::Action,
        shortcut: Some("F4"),
        action: PaletteAction::DawAction(Action::PatternAdd),
    });
    list.push(PaletteItem {
        title: "Next Pattern".into(),
        subtitle: "Select the next pattern index".into(),
        category: Category::Action,
        shortcut: Some("+"),
        action: PaletteAction::DawAction(Action::PatternNext),
    });
    list.push(PaletteItem {
        title: "Previous Pattern".into(),
        subtitle: "Select the previous pattern index".into(),
        category: Category::Action,
        shortcut: Some("-"),
        action: PaletteAction::DawAction(Action::PatternPrev),
    });
    list.push(PaletteItem {
        title: "Add Instrument / Channel (+)".into(),
        subtitle: "Append a new generator channel to the Channel Rack".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::DawAction(Action::RackAdd),
    });
    list.push(PaletteItem {
        title: "MIDI Settings".into(),
        subtitle: "Configure MIDI input controllers and outputs".into(),
        category: Category::Action,
        shortcut: Some("F10"),
        action: PaletteAction::OpenSettings(crate::views::settings::MIDI),
    });
    list.push(PaletteItem {
        title: "Audio Settings".into(),
        subtitle: "Buffer size, sample rate, and audio driver device".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::OpenSettings(crate::views::settings::AUDIO),
    });
    list.push(PaletteItem {
        title: "General Settings".into(),
        subtitle: "GUI scaling, animations, and system preferences".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::OpenSettings(crate::views::settings::GENERAL),
    });
    list.push(PaletteItem {
        title: "File Settings".into(),
        subtitle: "Extra search folders, VST search paths, and backups".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::OpenSettings(crate::views::settings::FILE),
    });
    list.push(PaletteItem {
        title: "Theme Settings".into(),
        subtitle: "Customize DAW colors, accents, and visual theme".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::OpenSettings(crate::views::settings::THEME),
    });
    list.push(PaletteItem {
        title: "Manage Plugins".into(),
        subtitle: "Scan, verify, and organize VST3 and CLAP plugins".into(),
        category: Category::Action,
        shortcut: None,
        action: PaletteAction::OpenSettings(crate::views::settings::PLUGINS),
    });

    // 3. Automation Actions
    list.push(PaletteItem {
        title: "Create Automation Clip for Last Tweaked Parameter".into(),
        subtitle: "Create automation clip linked to the most recently adjusted control".into(),
        category: Category::Automation,
        shortcut: None,
        action: PaletteAction::LastTweakedAutomation,
    });
    list.push(PaletteItem {
        title: "Link Last Tweaked Parameter to Controller...".into(),
        subtitle: "Connect last adjusted knob or slider to an external MIDI controller".into(),
        category: Category::Automation,
        shortcut: None,
        action: PaletteAction::LastTweakedLink,
    });
    list.push(PaletteItem {
        title: "Add Empty Automation Clip".into(),
        subtitle: "Append a blank automation clip channel to arrangement".into(),
        category: Category::Automation,
        shortcut: None,
        action: PaletteAction::AddAutomationClip,
    });
    list.push(PaletteItem {
        title: "Setup Auto-Sidechain Ducking (Selected Track -> Target Track)".into(),
        subtitle: "Route selected mixer track with 0 dB sidechain send and create ducking automation".into(),
        category: Category::Automation,
        shortcut: None,
        action: PaletteAction::AutoSidechainDucking,
    });
    list.push(PaletteItem {
        title: "Focus Event Editor (Pattern Automation)".into(),
        subtitle: "Open event editor window for fine-grained controller automation".into(),
        category: Category::Automation,
        shortcut: None,
        action: PaletteAction::FocusWindow(Win::Events),
    });

    // 4. Digicore, EDM & Trap Preset Grooves & Progressions
    use crate::grooves::{DrumGroove, ChordProgression};
    for groove in [
        DrumGroove::Trap2Step,
        DrumGroove::TrapRolls,
        DrumGroove::DigicoreGlitchHats,
        DrumGroove::DrillBounce,
        DrumGroove::Edm4OnFloor,
        DrumGroove::EdmOffbeatHat,
        DrumGroove::TrapSnare,
        DrumGroove::DigicoreGlitchSnare,
        DrumGroove::DembowSnare,
        DrumGroove::EdmBuildUpSnare,
    ] {
        list.push(PaletteItem {
            title: groove.title().into(),
            subtitle: groove.subtitle().into(),
            category: Category::Groove,
            shortcut: None,
            action: PaletteAction::ApplyDrumGroove(groove),
        });
    }

    for prog in [
        ChordProgression::DigicoreRoyal,
        ChordProgression::HyperpopAnthem,
        ChordProgression::TrapEmotionalMinor,
        ChordProgression::EdmFutureBassStabs,
        ChordProgression::NeoSoulJazz251,
        ChordProgression::DigicoreCyberPhrygian,
    ] {
        list.push(PaletteItem {
            title: prog.title().into(),
            subtitle: prog.subtitle().into(),
            category: Category::Groove,
            shortcut: None,
            action: PaletteAction::ApplyChordProgression(prog),
        });
    }

    // Smart Pattern & 808 MIDI Tools
    list.push(PaletteItem {
        title: "Auto-Generate 808 Bassline from Chords".into(),
        subtitle: "Extracts chord roots into syncopated sub-bass with octave slides".into(),
        category: Category::Groove,
        shortcut: None,
        action: PaletteAction::Generate808,
    });
    list.push(PaletteItem {
        title: "Toggle Slide Notes (808 Glides)".into(),
        subtitle: "Flips 0x0008 slide note flag on selected notes in piano roll".into(),
        category: Category::Groove,
        shortcut: None,
        action: PaletteAction::ToggleSlide,
    });
    list.push(PaletteItem {
        title: "Add Octave Slide-Up (+12 st) to Selected Notes".into(),
        subtitle: "Appends gliding slide note to note tails for 808 pitch bends".into(),
        category: Category::Groove,
        shortcut: None,
        action: PaletteAction::AddOctaveSlide,
    });
    list.push(PaletteItem {
        title: "Humanize Notes (Micro-Timing & Velocity)".into(),
        subtitle: "Injects organic swing (±6 velocity, ±3 ticks) to notes".into(),
        category: Category::Groove,
        shortcut: Some("Alt+H"),
        action: PaletteAction::HumanizeNotes,
    });
    list.push(PaletteItem {
        title: "Snap Notes to Scale (Key Quantize)".into(),
        subtitle: "Snaps out-of-key note pitches to current piano roll scale highlight".into(),
        category: Category::Groove,
        shortcut: None,
        action: PaletteAction::SnapNotesToScale,
    });

    // Song Arrangement & Structure Generators
    for style in [
        crate::grooves::ArrangementStyle::TrapStructure,
        crate::grooves::ArrangementStyle::DigicoreStructure,
        crate::grooves::ArrangementStyle::EdmStructure,
    ] {
        list.push(PaletteItem {
            title: style.title().into(),
            subtitle: style.subtitle().into(),
            category: Category::Arrangement,
            shortcut: None,
            action: PaletteAction::GenerateArrangement(style),
        });
    }

    // Clip Pitch Manipulation & Transposition
    list.push(PaletteItem {
        title: "Transpose Selected Clips +1 Semitone".into(),
        subtitle: "Pitch shift audio clips (+100 cents) or pattern notes (+1 st)".into(),
        category: Category::Arrangement,
        shortcut: Some("Shift+Alt+Up"),
        action: PaletteAction::TransposeClips(1),
    });
    list.push(PaletteItem {
        title: "Transpose Selected Clips -1 Semitone".into(),
        subtitle: "Pitch shift audio clips (-100 cents) or pattern notes (-1 st)".into(),
        category: Category::Arrangement,
        shortcut: Some("Shift+Alt+Down"),
        action: PaletteAction::TransposeClips(-1),
    });
    list.push(PaletteItem {
        title: "Transpose Selected Clips +1 Octave (+12 st)".into(),
        subtitle: "Pitch shift audio clips (+1200 cents) or pattern notes (+12 st)".into(),
        category: Category::Arrangement,
        shortcut: Some("Ctrl+Shift+Up"),
        action: PaletteAction::TransposeClips(12),
    });
    list.push(PaletteItem {
        title: "Transpose Selected Clips -1 Octave (-12 st)".into(),
        subtitle: "Pitch shift audio clips (-1200 cents) or pattern notes (-12 st)".into(),
        category: Category::Arrangement,
        shortcut: Some("Ctrl+Shift+Down"),
        action: PaletteAction::TransposeClips(-12),
    });

    // 5. Channels & Automation in Current Project
    for (i, ch) in s.project.channels.iter().enumerate() {
        let name = ch.name.as_deref().unwrap_or("Channel");
        if ch.kind == os_model::ChannelKind::Automation {
            let points = s.project.automation(ch.index).map_or(0, |a| a.points.len());
            list.push(PaletteItem {
                title: format!("Automation Clip: {name}"),
                subtitle: format!("{points} point(s) · Channel Row {}", i + 1),
                category: Category::Automation,
                shortcut: None,
                action: PaletteAction::FocusChannel(i),
            });
        } else {
            let kind = match ch.kind {
                os_model::ChannelKind::Sampler => "Sampler",
                os_model::ChannelKind::Plugin => "Plugin",
                os_model::ChannelKind::AudioClip => "Audio Clip",
                os_model::ChannelKind::Layer => "Layer",
                _ => "Channel",
            };
            list.push(PaletteItem {
                title: format!("Channel: {name}"),
                subtitle: format!("{kind} · Row {}, Mixer Track {}", i + 1, ch.mixer_track),
                category: Category::Channel,
                shortcut: None,
                action: PaletteAction::FocusChannel(i),
            });
        }
    }

    // 5. Mixer Tracks
    for m in &s.project.mixer {
        let name = if m.index == 0 {
            "Master".to_string()
        } else if let Some(n) = &m.name {
            n.clone()
        } else {
            format!("Insert {}", m.index)
        };
        let vol = m.volume.unwrap_or(0.8) * 100.0;
        list.push(PaletteItem {
            title: format!("Mixer Track {}: {name}", m.index),
            subtitle: format!("Fader {:.0}%", vol),
            category: Category::Mixer,
            shortcut: None,
            action: PaletteAction::FocusMixer(m.index),
        });
    }

    // 5. Scanned & Built-in Generator Plugins
    for entry in &s.plugdb.lists.gens {
        list.push(PaletteItem {
            title: format!("Insert Instrument: {}", entry.name),
            subtitle: format!("{} · {}", entry.vendor, entry.format.as_str().to_uppercase()),
            category: Category::Plugin,
            shortcut: None,
            action: PaletteAction::AddGenerator(entry.index),
        });
    }

    // 6. Scanned & Built-in Effect Plugins
    for entry in &s.plugdb.lists.fx {
        list.push(PaletteItem {
            title: format!("Insert Effect: {}", entry.name),
            subtitle: format!("{} · {} (to selected track {})", entry.vendor, entry.format.as_str().to_uppercase(), s.selected_mixer),
            category: Category::Effect,
            shortcut: None,
            action: PaletteAction::AddEffect(entry.index),
        });
    }

    list
}

/// Computes a fuzzy match score for ranking search candidates.
pub fn match_score(query: &str, title: &str, subtitle: &str) -> Option<i32> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Some(0);
    }

    let t_low = title.to_lowercase();
    let s_low = subtitle.to_lowercase();

    // Exact prefix match on title has highest priority
    if t_low.starts_with(&q) {
        return Some(10000 + 100 * q.len() as i32 - title.len() as i32);
    }

    // Exact substring match in title
    if let Some(pos) = t_low.find(&q) {
        let is_word_boundary = pos == 0 || title.as_bytes().get(pos - 1).is_some_and(|&b| {
            b == b' ' || b == b':' || b == b'-' || b == b'/' || b == b'('
        });
        let bonus = if is_word_boundary { 5000 } else { 2000 };
        return Some(bonus + 50 * q.len() as i32 - (pos as i32 * 10) - title.len() as i32);
    }

    // Fuzzy subsequence match in title
    let q_chars: Vec<char> = q.chars().collect();
    let t_chars: Vec<char> = t_low.chars().collect();
    let orig_chars: Vec<char> = title.chars().collect();

    let mut t_idx = 0;
    let mut consecutive = 0;
    let mut score = 500;
    let mut matched = 0;

    for &qc in &q_chars {
        let mut found = false;
        while t_idx < t_chars.len() {
            let tc = t_chars[t_idx];
            if tc == qc {
                found = true;
                let is_word_start = t_idx == 0 || {
                    let prev = orig_chars[t_idx - 1];
                    prev == ' ' || prev == ':' || prev == '-' || prev == '/' || prev == '('
                        || (prev.is_ascii_lowercase() && orig_chars[t_idx].is_ascii_uppercase())
                };
                if is_word_start {
                    score += 300;
                    consecutive += 1;
                } else if consecutive > 0 {
                    score += 100 * consecutive;
                    consecutive += 1;
                } else {
                    score += 30;
                    consecutive = 1;
                }
                t_idx += 1;
                matched += 1;
                break;
            } else {
                consecutive = 0;
                t_idx += 1;
            }
        }
        if !found {
            break;
        }
    }

    if matched == q_chars.len() {
        score -= title.len() as i32 * 2;
        return Some(score);
    }

    // Fallback: check match in subtitle
    if let Some(pos) = s_low.find(&q) {
        return Some(100 - pos as i32 - subtitle.len() as i32);
    }

    None
}

/// Filters and ranks items given the query string.
pub fn filter_items<'a>(items: &'a [PaletteItem], query: &str) -> Vec<ScoredItem<'a>> {
    let q = query.trim();
    if q.is_empty() {
        return items.iter().enumerate().map(|(i, item)| ScoredItem {
            item,
            original_index: i,
            score: 0,
        }).collect();
    }

    let mut scored = Vec::new();
    for (i, item) in items.iter().enumerate() {
        if let Some(s) = match_score(q, &item.title, &item.subtitle) {
            scored.push(ScoredItem {
                item,
                original_index: i,
                score: s,
            });
        }
    }

    scored.sort_by(|a, b| b.score.cmp(&a.score));
    scored
}

/// Executes the given item and closes the command palette.
pub fn execute(app: &mut App, item: &PaletteItem) {
    app.state.cmd_palette.open = false;
    match &item.action {
        PaletteAction::DawAction(act) => {
            app.act(act.clone());
        }
        PaletteAction::AddGenerator(idx) => {
            crate::plugin_menu::add_generator(app, *idx, None);
        }
        PaletteAction::AddEffect(idx) => {
            crate::plugin_menu::add_effect(app, *idx, None);
        }
        PaletteAction::FocusWindow(win) => {
            app.state.raise(*win);
            if !app.state.shown(*win) {
                app.act(Action::ToggleWin(*win));
            }
        }
        PaletteAction::FocusChannel(ch) => {
            let ch = *ch;
            if ch < app.state.project.channels.len() {
                app.state.selected_channel = ch;
                app.state.rack.select_only(ch);
                app.state.raise(Win::ChannelRack);
                app.open_channel_window(ch);
            }
        }
        PaletteAction::FocusMixer(track) => {
            app.state.selected_mixer = *track;
            app.state.raise(Win::Mixer);
        }
        PaletteAction::OpenSettings(tab) => {
            app.open_settings(*tab);
        }
        PaletteAction::ChopTransients => {
            let n = crate::drum_chop::chop_selected_or_target(&mut app.state, None, |s, idx| {
                crate::drum_chop::chop_clip_transients(s, idx)
            });
            app.say(if n > 1 {
                format!("Chopped into {n} slices (transients)")
            } else {
                "Select an audio clip to slice at transients".into()
            });
        }
        PaletteAction::ChopGrid(grid) => {
            let n = crate::drum_chop::chop_selected_or_target(&mut app.state, None, |s, idx| {
                crate::drum_chop::chop_clip_grid(s, idx, *grid)
            });
            app.say(if n > 1 {
                format!("Chopped into {n} slices ({})", grid.label())
            } else {
                "Select a clip to chop to grid".into()
            });
        }
        PaletteAction::ChopStutter(repeats) => {
            let n = crate::drum_chop::chop_selected_or_target(&mut app.state, None, |s, idx| {
                crate::drum_chop::chop_clip_stutter(s, idx, *repeats)
            });
            app.say(if n > 1 {
                format!("Stutter chopped into {n} slices ({repeats}x)")
            } else {
                "Select a clip to stutter chop".into()
            });
        }
        PaletteAction::SeparateStems(mode) => {
            let res = crate::stem_separator::start(app, None, *mode);
            app.say(match res {
                Ok(()) => format!("Started {}", mode.label()),
                Err(e) => format!("Spectral split: {e}"),
            });
        }
        PaletteAction::CloseAll => {
            app.close_windows(CloseAll::All);
        }
        PaletteAction::CloseUnfocused => {
            app.close_windows(CloseAll::Unfocused);
        }
        PaletteAction::ClosePlugins => {
            app.close_windows(CloseAll::Plugins);
        }
        PaletteAction::ToggleScaleHighlight => {
            let sh = &mut app.state.piano.scale_highlight;
            sh.enabled = !sh.enabled;
            let root_name = crate::views::piano::key_name(sh.root as i32);
            let scale_name = crate::chords::label(sh.scale_idx);
            let status = if sh.enabled { "on" } else { "off" };
            app.state.message = Some(format!("Scale highlighting: {status} ({root_name} {scale_name})"));
            app.damage_piano();
        }
        PaletteAction::CycleNoteDisplay => {
            app.state.piano.note_display = match app.state.piano.note_display {
                crate::piano::NoteDisplay::Name => crate::piano::NoteDisplay::Number,
                crate::piano::NoteDisplay::Number => crate::piano::NoteDisplay::None,
                crate::piano::NoteDisplay::None => crate::piano::NoteDisplay::Name,
            };
            app.state.message = Some(format!("Note pitch display: {:?}", app.state.piano.note_display));
            app.damage_piano();
        }
        PaletteAction::CycleKeyLabels => {
            app.state.piano.key_labels = match app.state.piano.key_labels {
                crate::piano::KeyLabels::AllWhite => crate::piano::KeyLabels::RootOnly,
                crate::piano::KeyLabels::RootOnly => crate::piano::KeyLabels::None,
                crate::piano::KeyLabels::None => crate::piano::KeyLabels::AllWhite,
            };
            app.state.message = Some(format!("Piano key labels: {:?}", app.state.piano.key_labels));
            app.damage_piano();
        }
        PaletteAction::StampChord(k) => {
            app.state.piano.stamp = Some(*k);
            let name = crate::chords::label(*k);
            app.state.message = Some(format!("Stamp tool active: {name} chord (click on grid to stamp)"));
        }
        PaletteAction::ClearStamp => {
            app.state.piano.stamp = None;
            app.state.message = Some("Stamp tool cleared".into());
        }
        PaletteAction::ColorGradientTracks => {
            if app.state.shown(Win::Mixer) {
                crate::mixer::gradient(&mut app.state);
                app.state.message = Some("Applied color gradient across mixer tracks".into());
            } else if app.state.shown(Win::Playlist) {
                crate::playlist::gradient(&mut app.state);
                app.state.message = Some("Applied color gradient across playlist tracks".into());
            } else {
                crate::rack::gradient(&mut app.state);
                app.state.message = Some("Applied color gradient across channels".into());
            }
        }
        PaletteAction::TransposeChannel(semi) => {
            let semi = *semi;
            let sel = app.state.rack.sel.clone();
            let pat = app.state.pattern;
            let mut count = 0;
            if let Some(p) = app.state.project.patterns.iter_mut().find(|p| p.number == pat) {
                for n in &mut p.notes {
                    if sel.contains(&(n.channel as usize)) || (sel.is_empty() && n.channel as usize == app.state.selected_channel) {
                        n.key = (n.key as i32 + semi).clamp(0, 127) as u16;
                        count += 1;
                    }
                }
            }
            app.state.piano.dirty = true;
            app.state.message = Some(format!("Transposed {count} note(s) by {semi:+} semitones"));
        }
        PaletteAction::LastTweakedAutomation => {
            if let Some(msg) = app.params_menu(crate::params::MenuEntry::Automation) {
                app.state.message = Some(msg);
            }
        }
        PaletteAction::LastTweakedLink => {
            if let Some(msg) = app.params_menu(crate::params::MenuEntry::Last) {
                app.state.message = Some(msg);
            }
        }
        PaletteAction::AddAutomationClip => {
            app.add_clip_channel(os_model::ChannelKind::Automation);
        }
        PaletteAction::AutoSidechainDucking => {
            let dst = app.state.selected_mixer;
            if let Some(msg) = crate::mixer::auto_sidechain_ducking(&mut app.state, dst) {
                app.state.message = Some(msg);
            }
        }
        PaletteAction::ApplyDrumGroove(groove) => {
            crate::grooves::apply_drum_groove(&mut app.state, *groove);
        }
        PaletteAction::ApplyChordProgression(prog) => {
            crate::grooves::apply_chord_progression(&mut app.state, *prog);
        }
        PaletteAction::Generate808 => {
            let msg = crate::grooves::generate_808_bassline(&mut app.state);
            app.state.message = Some(msg);
        }
        PaletteAction::ToggleSlide => {
            let msg = crate::grooves::toggle_slide_notes(&mut app.state);
            app.state.message = Some(msg);
        }
        PaletteAction::AddOctaveSlide => {
            let msg = crate::grooves::add_octave_slide(&mut app.state);
            app.state.message = Some(msg);
        }
        PaletteAction::HumanizeNotes => {
            let msg = crate::grooves::humanize_notes(&mut app.state, 6, 3);
            app.state.message = Some(msg);
        }
        PaletteAction::SnapNotesToScale => {
            let msg = crate::grooves::snap_notes_to_scale(&mut app.state);
            app.state.message = Some(msg);
        }
        PaletteAction::TransposeClips(semi) => {
            if let Some(msg) = crate::playlist::run(app, crate::playlist::Cmd::Transpose(*semi)) {
                app.state.message = Some(msg);
            }
        }
        PaletteAction::GenerateArrangement(style) => {
            if let Some(msg) = crate::playlist::run(app, crate::playlist::Cmd::Arrangement(*style)) {
                app.state.message = Some(msg);
            }
        }
    }
    app.invalidate();
}

/// Handles keyboard input while the command palette modal is open.
pub fn key(app: &mut App, key: &winit::keyboard::Key, ctrl: bool, alt: bool) -> bool {
    use winit::keyboard::{Key, NamedKey};

    if !app.state.cmd_palette.open {
        return false;
    }

    // Toggle closed on Ctrl+K
    if let Key::Character(c) = key && ctrl && !alt && c.eq_ignore_ascii_case("k") {
        app.state.cmd_palette.open = false;
        app.invalidate();
        return true;
    }

    let all_items = items(&app.state);
    let filtered = filter_items(&all_items, &app.state.cmd_palette.query);
    let count = filtered.len();

    match key {
        Key::Named(NamedKey::Escape) => {
            app.state.cmd_palette.open = false;
        }
        Key::Named(NamedKey::ArrowDown) => {
            if count > 0 {
                app.state.cmd_palette.selected = (app.state.cmd_palette.selected + 1).min(count - 1);
                update_scroll(&mut app.state.cmd_palette, count);
            }
        }
        Key::Named(NamedKey::ArrowUp) => {
            if count > 0 {
                app.state.cmd_palette.selected = app.state.cmd_palette.selected.saturating_sub(1);
                update_scroll(&mut app.state.cmd_palette, count);
            }
        }
        Key::Named(NamedKey::PageDown) => {
            if count > 0 {
                app.state.cmd_palette.selected = (app.state.cmd_palette.selected + PAGE_SIZE).min(count - 1);
                update_scroll(&mut app.state.cmd_palette, count);
            }
        }
        Key::Named(NamedKey::PageUp) => {
            if count > 0 {
                app.state.cmd_palette.selected = app.state.cmd_palette.selected.saturating_sub(PAGE_SIZE);
                update_scroll(&mut app.state.cmd_palette, count);
            }
        }
        Key::Named(NamedKey::Home) => {
            app.state.cmd_palette.selected = 0;
            app.state.cmd_palette.scroll = 0;
        }
        Key::Named(NamedKey::End) => {
            if count > 0 {
                app.state.cmd_palette.selected = count - 1;
                update_scroll(&mut app.state.cmd_palette, count);
            }
        }
        Key::Named(NamedKey::Enter) => {
            if let Some(target) = filtered.get(app.state.cmd_palette.selected) {
                let item = target.item.clone();
                execute(app, &item);
                return true;
            }
        }
        Key::Named(NamedKey::Backspace | NamedKey::Delete) => {
            if ctrl {
                // Delete previous word on Ctrl+Backspace
                let trimmed = app.state.cmd_palette.query.trim_end();
                if let Some(idx) = trimmed.rfind(|c: char| c.is_whitespace() || c == ':') {
                    app.state.cmd_palette.query.truncate(idx);
                } else {
                    app.state.cmd_palette.query.clear();
                }
            } else {
                app.state.cmd_palette.query.pop();
            }
            app.state.cmd_palette.selected = 0;
            app.state.cmd_palette.scroll = 0;
        }
        Key::Named(NamedKey::Space) if !alt => {
            if app.state.cmd_palette.query.len() < 128 {
                app.state.cmd_palette.query.push(' ');
                app.state.cmd_palette.selected = 0;
                app.state.cmd_palette.scroll = 0;
            }
        }
        Key::Character(text) if !alt => {
            // Allow typing even if Ctrl was momentarily held, unless it's a reserved shortcut
            if ctrl && (text.eq_ignore_ascii_case("k") || text.eq_ignore_ascii_case("u") || text.eq_ignore_ascii_case("a")) {
                if text.eq_ignore_ascii_case("u") || text.eq_ignore_ascii_case("a") {
                    app.state.cmd_palette.query.clear();
                    app.state.cmd_palette.selected = 0;
                    app.state.cmd_palette.scroll = 0;
                }
            } else {
                let clean: String = text.chars().filter(|c| !c.is_control()).collect();
                if app.state.cmd_palette.query.len() + clean.len() <= 128 {
                    app.state.cmd_palette.query.push_str(&clean);
                    app.state.cmd_palette.selected = 0;
                    app.state.cmd_palette.scroll = 0;
                }
            }
        }
        _ => {}
    }

    app.invalidate();
    true
}

fn update_scroll(pal: &mut Palette, count: usize) {
    if count == 0 {
        pal.selected = 0;
        pal.scroll = 0;
        return;
    }
    if pal.selected < pal.scroll {
        pal.scroll = pal.selected;
    } else if pal.selected >= pal.scroll + PAGE_SIZE {
        pal.scroll = pal.selected + 1 - PAGE_SIZE;
    }
}

/// Scrolls the command palette by `notches` (mouse wheel). Positive is up, negative is down.
pub fn wheel(pal: &mut Palette, count: usize, notches: f32) {
    if count <= PAGE_SIZE {
        return;
    }
    let max_scroll = count.saturating_sub(PAGE_SIZE);
    let step = if notches.abs() >= 1.0 { (notches * 2.0).round() as i32 } else { notches.signum() as i32 * 2 };
    let new_scroll = (pal.scroll as i32 - step).clamp(0, max_scroll as i32) as usize;
    pal.scroll = new_scroll;
    if pal.selected < pal.scroll {
        pal.selected = pal.scroll;
    } else if pal.selected >= pal.scroll + PAGE_SIZE {
        pal.selected = pal.scroll + PAGE_SIZE - 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_wheel_scrolls_and_clamps() {
        let mut p = Palette::default();
        wheel(&mut p, 30, -1.0); // scroll down
        assert!(p.scroll > 0);
        assert!(p.selected >= p.scroll);

        wheel(&mut p, 30, 20.0); // scroll up
        assert_eq!(p.scroll, 0);
        assert!(p.selected < PAGE_SIZE);
    }

    #[test]
    fn fuzzy_matching_prefers_prefix_and_word_boundaries() {
        // Prefix match scores higher than substring match
        let s_prefix = match_score("cit", "Citrus", "Synthesizer").unwrap();
        let s_mid = match_score("cit", "Electric City", "Effects").unwrap();
        assert!(s_prefix > s_mid, "Prefix match scores higher than middle match");

        // Subsequence matches
        assert!(match_score("rev", "Reverse Audio Clips", "").is_some());
        assert!(match_score("fpl", "Focus Playlist", "").is_some());
        assert!(match_score("xyz123", "Focus Playlist", "").is_none());

        // Empty query matches everything
        assert_eq!(match_score("", "Any Title", ""), Some(0));
    }

    #[test]
    fn filter_items_sorts_descending_by_score() {
        let dummy = vec![
            PaletteItem {
                title: "Electric Guitar".into(),
                subtitle: "".into(),
                category: Category::Plugin,
                shortcut: None,
                action: PaletteAction::CloseAll,
            },
            PaletteItem {
                title: "Guitar Amp".into(),
                subtitle: "".into(),
                category: Category::Effect,
                shortcut: None,
                action: PaletteAction::CloseAll,
            },
        ];

        let filtered = filter_items(&dummy, "guitar");
        assert_eq!(filtered.len(), 2);
        // "Guitar Amp" starts with "guitar" so it ranks first
        assert_eq!(filtered[0].item.title, "Guitar Amp");
        assert_eq!(filtered[1].item.title, "Electric Guitar");
    }

    #[test]
    fn scroll_pagination_keeps_selected_visible() {
        let mut p = Palette::default();
        p.selected = 5;
        update_scroll(&mut p, 20);
        assert_eq!(p.scroll, 0);

        p.selected = 12;
        update_scroll(&mut p, 20);
        assert_eq!(p.scroll, 12 + 1 - PAGE_SIZE);

        p.selected = 2;
        update_scroll(&mut p, 20);
        assert_eq!(p.scroll, 2);
    }

    #[test]
    fn category_metadata_and_colors() {
        assert_eq!(Category::Action.label(), "ACTION");
        assert_eq!(Category::Window.label(), "WINDOW");
        assert_eq!(Category::Plugin.label(), "PLUGIN");
        assert_eq!(Category::Effect.label(), "EFFECT");
        assert_eq!(Category::Channel.label(), "CHANNEL");
        assert_eq!(Category::Mixer.label(), "MIXER");
        assert_eq!(Category::Automation.label(), "AUTO");
        assert_eq!(Category::Groove.label(), "GROOVE");
        assert_eq!(Category::Arrangement.label(), "ARRANGE");

        let (bg, fg) = Category::Action.colors();
        assert_ne!(bg, fg);
        let (bg_a, fg_a) = Category::Automation.colors();
        assert_ne!(bg_a, fg_a);
        let (bg_g, fg_g) = Category::Groove.colors();
        assert_ne!(bg_g, fg_g);
        let (bg_r, fg_r) = Category::Arrangement.colors();
        assert_ne!(bg_r, fg_r);
    }
}
