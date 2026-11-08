// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Lemon Bitcrusher: Lemon Studio's bitcrusher and sample decimator as a VST3 / CLAP plugin module.
#![allow(semicolon_in_expressions_from_non_local_macros)]

#[cfg(not(feature = "linked"))]
use nih_plug::prelude::*;

pub use lemon_small_fx::bitcrusher::LemonBitcrusher;

#[cfg(not(feature = "linked"))]
nih_export_clap!(LemonBitcrusher);
#[cfg(not(feature = "linked"))]
nih_export_vst3!(LemonBitcrusher);
os_plugkit::linked_clap_entry!(LemonBitcrusher);
