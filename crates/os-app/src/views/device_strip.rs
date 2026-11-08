// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Bottom Device Strip (Bitwig & Ableton Style): horizontal signal chain dock
//! showing the selected channel's generator macros and mixer insert effect slots.

use os_model::ChannelKind;
use os_ui::kit::lemon as lk;
use os_ui::{Color, Font, Rect};

use crate::app::{Action, State, Ui};
use crate::icons::{self, Icon};
use crate::knobs::KnobId;
use crate::mixer as mx;
use crate::views::{ball_knob, led, mixer::chevron, rack, rgb, FrameInfo};

pub const HEADER_H: f32 = 24.0;
pub const CARD_MARGIN: f32 = 6.0;
pub const ARROW_W: f32 = 18.0;

/// Paint the bottom device strip dock.
pub fn paint(ui: &mut Ui, s: &State, r: Rect, _f: &FrameInfo) {
    let t = ui.t;

    // Background panel fill & top dock divider line
    ui.c.fill_rect(r, t.s(0x202325));
    ui.c.fill_rect(Rect::new(r.x, r.y, r.w, 1.0), t.s(0x161819));

    // Top Header Ribbon (Title bar & channel navigation)
    let header_r = Rect::new(r.x, r.y, r.w, HEADER_H);
    ui.c.fill_rect(header_r, t.s(0x1a1c1e));
    ui.c.fill_rect(Rect::new(r.x, header_r.bottom() - 1.0, r.w, 1.0), t.s(0x141617));

    // Left: Plug icon + Title
    icons::draw(ui.c, Icon::Plug, Rect::new(r.x + 8.0, r.y + 4.0, 16.0, 16.0), t.tx(0xffb020));
    ui.c.text(Font::new(13), r.x + 28.0, r.y + 4.5, t.text_dim, "DEVICE STRIP");
    ui.c.text(Font::new(11), r.x + 104.0, r.y + 6.0, t.tx(0x606870), "SIGNAL CHAIN");

    // View Switcher Tabs: [ Chain ] [ Piano Roll ] [ Split ]
    let tab_x = r.x + 185.0;
    let tabs = [("Chain", 0), ("Piano Roll", 1), ("Split", 2)];
    for (idx, &(label, tab_val)) in tabs.iter().enumerate() {
        let tab_r = Rect::new(tab_x + (idx as f32) * 62.0, r.y + 3.0, 58.0, 18.0);
        let is_active = s.device_strip_tab == tab_val;
        let hov_tab = ui.hit(tab_r, Action::DeviceStripTab(tab_val), format!("Switch dock view to {label}"));
        let bg_c = if is_active { t.s(0x353a3e) } else if hov_tab { t.s(0x282c2f) } else { t.s(0x1c1e20) };
        let border_c = if is_active { t.s(0x485056) } else { t.s(0x222527) };
        lk::outlined(ui.c, tab_r, 3.0, border_c, bg_c);
        let text_c = if is_active { t.tx(0xffffff) } else if hov_tab { t.text } else { t.text_dim };
        ui.c.text_center(Font::new(10), tab_r, text_c, label);
    }

    let num_chans = s.project.channels.len();
    if num_chans > 0 {
        let ch_idx = s.selected_channel.min(num_chans - 1);
        let ch = &s.project.channels[ch_idx];
        let ch_name = ch.name.as_deref().unwrap_or("Untitled");

        // Center: Channel switcher [<] [Ch Name → Insert] [>]
        let nav_cx = r.cx();
        let prev_btn = Rect::new(nav_cx - 130.0, r.y + 3.0, 20.0, 18.0);
        let hov_prev = ui.hit(prev_btn, Action::DeviceStripPrevChan, "Previous channel");
        lk::outlined(ui.c, prev_btn, 3.0, t.s(0x1c1e20), if hov_prev { t.s(0x323639) } else { t.s(0x242729) });
        ui.c.text_center(Font::new(12), prev_btn, if hov_prev { t.tx(0xffffff) } else { t.tx(0xb0b6bc) }, "‹");

        let pill_w = 200.0;
        let pill_r = Rect::new(nav_cx - pill_w / 2.0, r.y + 3.0, pill_w, 18.0);
        let hov_pill = ui.hit(pill_r, Action::DeviceStripFocusChan(ch_idx), "Cycle / focus channel (scroll wheel cycles)");
        lk::outlined(ui.c, pill_r, 3.0, t.s(0x1c1e20), if hov_pill { t.s(0x323639) } else { t.s(0x232628) });

        // Channel custom color indicator dot
        let dot_c = ch.color.map_or(Color::rgb(0xffb020), rgb);
        ui.c.fill_circle(pill_r.x + 10.0, pill_r.cy(), 4.0, dot_c);

        let disp_name = format!("{}: {}", ch_idx + 1, ch_name);
        ui.c.text_clip(Font::new(12), pill_r.x + 18.0, pill_r.y + 2.0, pill_w - 75.0, if hov_pill { t.tx(0xffffff) } else { t.text }, &disp_name);

        let insert_badge = if ch.mixer_track == 0 { "Master".to_string() } else { format!("→ Ins {}", ch.mixer_track) };
        ui.c.text_right(Font::new(11), pill_r.right() - 6.0, pill_r.y + 2.5, t.tx(0x8e969d), &insert_badge);

        let next_btn = Rect::new(nav_cx + 110.0, r.y + 3.0, 20.0, 18.0);
        let hov_next = ui.hit(next_btn, Action::DeviceStripNextChan, "Next channel");
        lk::outlined(ui.c, next_btn, 3.0, t.s(0x1c1e20), if hov_next { t.s(0x323639) } else { t.s(0x242729) });
        ui.c.text_center(Font::new(12), next_btn, if hov_next { t.tx(0xffffff) } else { t.tx(0xb0b6bc) }, "›");
    }

    // Right: Close button
    let close_r = Rect::new(r.right() - 22.0, r.y + 4.0, 16.0, 16.0);
    let hov_close = ui.hit(close_r, Action::ToggleDeviceStrip, "Close device strip (Alt+D)");
    icons::draw(ui.c, Icon::Close, close_r.inset(1.5), if hov_close { t.tx(0xffffff) } else { t.tx(0x80878d) });

    // Body Area
    let body_r = Rect::new(r.x, r.y + HEADER_H, r.w, (r.h - HEADER_H).max(0.0));
    ui.hit(body_r, Action::DeviceStripArea, "Device strip");

    if num_chans == 0 {
        ui.c.text_center(Font::new(13), body_r, t.text_dim, "No channels in project. Add an instrument to the Channel Rack to view signal chain.");
        return;
    }

    let ch_idx = s.selected_channel.min(num_chans - 1);
    let ch = &s.project.channels[ch_idx];
    let target_mixer = ch.mixer_track;

    if s.device_strip_tab == 1 {
        paint_mini_piano_roll(ui, s, body_r.inset(4.0), ch_idx, _f);
        return;
    }

    let (devices_area, mini_area) = if s.device_strip_tab == 2 {
        let mini_w = (body_r.w * 0.46).clamp(240.0, 560.0);
        let mini_r = Rect::new(body_r.right() - mini_w - 4.0, body_r.y + 4.0, mini_w, (body_r.h - 8.0).max(0.0));
        let dev_r = Rect::new(body_r.x, body_r.y, (mini_r.x - body_r.x - 4.0).max(10.0), body_r.h);
        (dev_r, Some(mini_r))
    } else {
        (body_r, None)
    };

    ui.c.push_clip(devices_area);

    let card_y = body_r.y + CARD_MARGIN;
    let card_h = (body_r.h - 2.0 * CARD_MARGIN).max(0.0);
    let mut cx = r.x + 8.0 - s.device_strip_scroll_x;

    // -------------------------------------------------------------
    // CARD 1: Channel Overview & Routing Card
    // -------------------------------------------------------------
    let c1_w = 148.0;
    let c1_r = Rect::new(cx, card_y, c1_w, card_h);
    lk::outlined(ui.c, c1_r, 4.0, t.s(0x181a1b), t.s(0x26292b));

    // Card 1 Banner
    let c1_banner = Rect::new(c1_r.x, c1_r.y, c1_r.w, 20.0);
    ui.c.fill_rrect(c1_banner, 4.0, t.s(0x1d1f21));
    ui.c.fill_rect(Rect::new(c1_r.x, c1_banner.bottom() - 4.0, c1_r.w, 4.0), t.s(0x1d1f21));
    ui.c.fill_rect(Rect::new(c1_r.x, c1_banner.bottom() - 1.0, c1_r.w, 1.0), t.s(0x161718));

    // Card 1 LED
    led(ui, c1_r.x + 12.0, c1_r.y + 10.0, ch.enabled);
    ui.hit(Rect::new(c1_r.x + 4.0, c1_r.y + 2.0, 16.0, 16.0), Action::ChannelEnable(ch_idx), "Mute / unmute channel");
    ui.c.text(Font::new(11), c1_r.x + 24.0, c1_r.y + 3.5, t.text_dim, "CHANNEL");

    // Channel name text
    let name_str = ch.name.as_deref().unwrap_or("Untitled");
    ui.c.text_clip(Font::new(13), c1_r.x + 10.0, c1_r.y + 25.0, c1_r.w - 20.0, t.text, name_str);

    // Vol & Pan Knobs
    let vol_id = KnobId::ChanVol(ch_idx);
    let pan_id = KnobId::ChanPan(ch_idx);

    let k_vol_r = Rect::new(c1_r.x + 14.0, c1_r.y + 44.0, 24.0, 24.0);
    ui.hit(k_vol_r, Action::Knob(vol_id), s.knob_text(vol_id));
    ball_knob(ui, k_vol_r.cx(), k_vol_r.cy(), 9.5, s.knob(vol_id));
    ui.c.text_center(Font::new(10), Rect::new(c1_r.x + 8.0, c1_r.y + 70.0, 36.0, 12.0), t.text_dim, "VOL");

    let k_pan_r = Rect::new(c1_r.x + 64.0, c1_r.y + 44.0, 24.0, 24.0);
    ui.hit(k_pan_r, Action::Knob(pan_id), s.knob_text(pan_id));
    ball_knob(ui, k_pan_r.cx(), k_pan_r.cy(), 9.5, s.knob(pan_id));
    ui.c.text_center(Font::new(10), Rect::new(c1_r.x + 58.0, c1_r.y + 70.0, 36.0, 12.0), t.text_dim, "PAN");

    // Target Mixer Track pill at bottom of Card 1
    let route_r = Rect::new(c1_r.x + 8.0, c1_r.bottom() - 26.0, c1_r.w - 16.0, 18.0);
    let hov_route = ui.hit(route_r, Action::Mix(crate::mixer::MHit::Strip(target_mixer)), "Select target mixer track");
    lk::outlined(ui.c, route_r, 3.0, t.s(0x191b1c), if hov_route { t.s(0x2f3336) } else { t.s(0x222527) });
    let route_label = rack::insert_name(target_mixer);
    ui.c.text_center(Font::new(11), route_r, if hov_route { t.tx(0xffffff) } else { t.tx(0x9da4ab) }, &route_label);

    cx += c1_w;

    // Connector arrow
    draw_connector(ui, cx, card_y + card_h / 2.0);
    cx += ARROW_W;

    // -------------------------------------------------------------
    // CARD 2: Instrument / Generator Device Card
    // -------------------------------------------------------------
    let c2_w = 205.0;
    let c2_r = Rect::new(cx, card_y, c2_w, card_h);
    lk::outlined(ui.c, c2_r, 4.0, t.s(0x181a1b), t.s(0x26292b));

    // Card 2 Banner
    let c2_banner = Rect::new(c2_r.x, c2_r.y, c2_r.w, 20.0);
    ui.c.fill_rrect(c2_banner, 4.0, t.s(0x1d1f21));
    ui.c.fill_rect(Rect::new(c2_r.x, c2_banner.bottom() - 4.0, c2_r.w, 4.0), t.s(0x1d1f21));
    ui.c.fill_rect(Rect::new(c2_r.x, c2_banner.bottom() - 1.0, c2_r.w, 1.0), t.s(0x161718));

    // Instrument Title & Kind
    let inst_name = match ch.kind {
        ChannelKind::Sampler => "Lemon Sampler".to_string(),
        ChannelKind::Plugin => crate::project_plugins::plugin_title(ch).unwrap_or_else(|| "Instrument Plugin".into()),
        ChannelKind::Layer => "Layer".into(),
        ChannelKind::AudioClip => "Audio Clip".into(),
        ChannelKind::Automation => "Automation Clip".into(),
        ChannelKind::Other(n) => format!("Instrument ({n})"),
    };
    ui.c.text_clip(Font::new(12), c2_r.x + 8.0, c2_r.y + 3.0, c2_r.w - 55.0, t.tx(0xffd700), &inst_name);

    // OPEN / EDIT button
    let edit_btn = Rect::new(c2_r.right() - 44.0, c2_r.y + 2.0, 38.0, 16.0);
    let hov_edit = ui.hit(edit_btn, Action::DeviceStripOpenChan(ch_idx), "Open generator / plugin window");
    lk::outlined(ui.c, edit_btn, 2.5, t.s(0x181a1b), if hov_edit { t.s(0x383e42) } else { t.s(0x2a2e31) });
    ui.c.text_center(Font::new(10), edit_btn, if hov_edit { t.tx(0xffffff) } else { t.tx(0xb0b8be) }, "EDIT");

    // Subtitle (Sample path or details)
    let sample_name = match (&ch.sample_path, ch.kind) {
        (Some(p), _) => p.rsplit(['/', '\\']).next().unwrap_or(p).to_string(),
        (None, ChannelKind::Sampler) => "(built-in pluck)".into(),
        (None, ChannelKind::Plugin) => "Hosted Plugin Engine".into(),
        (None, _) => "-".into(),
    };
    ui.c.text_clip(Font::new(11).italic(), c2_r.x + 8.0, c2_r.y + 25.0, c2_r.w - 16.0, t.text_dim, &sample_name);

    // Macro Knobs (Cutoff & Resonance)
    if ch.kind.is_sampler() {
        let cut_id = KnobId::ChanCutoff(ch_idx);
        let res_id = KnobId::ChanRes(ch_idx);

        let k_cut_r = Rect::new(c2_r.x + 18.0, c2_r.y + 44.0, 24.0, 24.0);
        ui.hit(k_cut_r, Action::Knob(cut_id), s.knob_text(cut_id));
        ball_knob(ui, k_cut_r.cx(), k_cut_r.cy(), 9.5, s.knob(cut_id));
        ui.c.text_center(Font::new(10), Rect::new(c2_r.x + 8.0, c2_r.y + 70.0, 44.0, 12.0), t.text_dim, "CUTOFF");

        let k_res_r = Rect::new(c2_r.x + 70.0, c2_r.y + 44.0, 24.0, 24.0);
        ui.hit(k_res_r, Action::Knob(res_id), s.knob_text(res_id));
        ball_knob(ui, k_res_r.cx(), k_res_r.cy(), 9.5, s.knob(res_id));
        ui.c.text_center(Font::new(10), Rect::new(c2_r.x + 64.0, c2_r.y + 70.0, 36.0, 12.0), t.text_dim, "RES");
    } else {
        ui.c.text_clip(Font::new(10), c2_r.x + 8.0, c2_r.y + 49.0, 102.0, t.text_dim, "Filter: plugin editor");
    }

    // Pitch & cents badge
    let pitch_r = Rect::new(c2_r.x + 116.0, c2_r.y + 46.0, 78.0, 36.0);
    lk::outlined(ui.c, pitch_r, 3.0, t.s(0x191b1c), t.s(0x222527));
    ui.c.text_center(Font::new(10), Rect::new(pitch_r.x, pitch_r.y + 2.0, pitch_r.w, 12.0), t.text_dim, "PITCH");
    let pitch_str = format!("{:+} ct", ch.pitch_cents);
    ui.c.text_center(Font::new(11), Rect::new(pitch_r.x, pitch_r.y + 16.0, pitch_r.w, 16.0), t.tx(0x9da4ab), &pitch_str);

    cx += c2_w;

    // Connector arrow
    draw_connector(ui, cx, card_y + card_h / 2.0);
    cx += ARROW_W;

    // -------------------------------------------------------------
    // CARDS 3..N: Insert Effect Slots on Target Mixer Track
    // -------------------------------------------------------------
    let track_data = s.project.mixer.iter().find(|m| m.index == target_mixer);
    for k in 0..10u8 {
        if let Some(fx_name) = mx::slot_name(s, target_mixer, k) {
            let fx_w = 175.0;
            let fx_r = Rect::new(cx, card_y, fx_w, card_h);
            lk::outlined(ui.c, fx_r, 4.0, t.s(0x181a1b), t.s(0x26292b));

            // Banner
            let fx_banner = Rect::new(fx_r.x, fx_r.y, fx_r.w, 20.0);
            ui.c.fill_rrect(fx_banner, 4.0, t.s(0x1d1f21));
            ui.c.fill_rect(Rect::new(fx_r.x, fx_banner.bottom() - 4.0, fx_r.w, 4.0), t.s(0x1d1f21));
            ui.c.fill_rect(Rect::new(fx_r.x, fx_banner.bottom() - 1.0, fx_r.w, 1.0), t.s(0x161718));

            // Slot Bypass LED
            let slot_on = track_data.map_or(true, |m| m.slot_on[k as usize]);
            led(ui, fx_r.x + 10.0, fx_r.y + 10.0, slot_on);
            ui.hit(Rect::new(fx_r.x + 2.0, fx_r.y + 2.0, 16.0, 16.0), Action::DeviceStripSlotLed { track: target_mixer, slot: k }, "Enable / bypass effect");

            // Slot badge: "FX 1"
            let badge = format!("FX {}", k + 1);
            ui.c.text(Font::new(11), fx_r.x + 22.0, fx_r.y + 3.5, t.text_dim, &badge);

            // Options Chevron
            let arrow_r = Rect::new(fx_r.right() - 20.0, fx_r.y + 2.0, 16.0, 16.0);
            let hov_arrow = ui.hit(arrow_r, Action::DeviceStripSlotArrow { track: target_mixer, slot: k }, "FX slot options / presets");
            chevron(ui, arrow_r.cx(), arrow_r.cy(), false, if hov_arrow { t.tx(0xffffff) } else { t.tx(0x9da4ab) });

            // Plugin Name
            ui.c.text_clip(Font::new(13), fx_r.x + 10.0, fx_r.y + 25.0, fx_r.w - 20.0, t.text, &fx_name);

            // OPEN GUI Button
            let open_btn = Rect::new(fx_r.x + 10.0, fx_r.y + 50.0, 52.0, 24.0);
            let hov_open = ui.hit(open_btn, Action::DeviceStripSlotClick { track: target_mixer, slot: k }, "Open plugin window");
            lk::outlined(ui.c, open_btn, 3.0, t.s(0x181a1b), if hov_open { t.s(0x383e42) } else { t.s(0x2a2e31) });
            ui.c.text_center(Font::new(11), open_btn, if hov_open { t.tx(0xffffff) } else { t.tx(0xb0b8be) }, "OPEN");

            // Dry/Wet Mix Knob
            let mix_id = KnobId::SlotMix(target_mixer, k);
            let k_mix_r = Rect::new(fx_r.right() - 44.0, fx_r.y + 44.0, 24.0, 24.0);
            ui.hit(k_mix_r, Action::Knob(mix_id), s.knob_text(mix_id));
            ball_knob(ui, k_mix_r.cx(), k_mix_r.cy(), 9.5, s.knob(mix_id));
            ui.c.text_center(Font::new(10), Rect::new(fx_r.right() - 50.0, fx_r.y + 70.0, 36.0, 12.0), t.text_dim, "MIX");

            let mix_pct = format!("{:.0}%", s.knob(mix_id) * 100.0);
            ui.c.text_center(Font::new(10), Rect::new(fx_r.right() - 54.0, fx_r.y + 82.0, 44.0, 12.0), t.tx(0x7c848a), &mix_pct);

            cx += fx_w;

            // Connector arrow
            draw_connector(ui, cx, card_y + card_h / 2.0);
            cx += ARROW_W;
        }
    }

    // -------------------------------------------------------------
    // CARD END: + Add Effect Card
    // -------------------------------------------------------------
    let add_w = 95.0;
    let add_r = Rect::new(cx, card_y, add_w, card_h);
    let hov_add = ui.hit(add_r, Action::DeviceStripAddEffect { track: target_mixer }, "Add effect to channel chain");
    let border = if hov_add { t.s(0x42494e) } else { t.s(0x2c3033) };
    let bg = if hov_add { t.s(0x282c2f) } else { t.s(0x222426) };
    lk::outlined(ui.c, add_r, 4.0, border, bg);

    let text_c = if hov_add { t.tx(0xffffff) } else { t.tx(0x848c92) };
    ui.c.text_center(Font::new(18), Rect::new(add_r.x, add_r.cy() - 18.0, add_r.w, 20.0), text_c, "+");
    ui.c.text_center(Font::new(11), Rect::new(add_r.x, add_r.cy() + 4.0, add_r.w, 14.0), text_c, "ADD EFFECT");

    ui.c.pop_clip();

    if let Some(mini_r) = mini_area {
        paint_mini_piano_roll(ui, s, mini_r, ch_idx, _f);
    }
}

/// Paint an inline mini Piano Roll / Clip Editor inside the device strip dock.
/// Supports Fold mode (only active pitches displayed), 16-step pad sequencer mode,
/// top Ableton-style time ruler, and velocity stems lane.
pub fn paint_mini_piano_roll(ui: &mut Ui, s: &State, r: Rect, ch_idx: usize, f: &FrameInfo) {
    let t = ui.t;
    lk::outlined(ui.c, r, 4.0, t.s(0x1a1c1e), t.s(0x141617));

    let Some(ch) = s.project.channels.get(ch_idx) else { return };
    let pat_idx = s.pattern.max(1);
    let pat = s.project.patterns.iter().find(|p| p.number == pat_idx);

    // Mini Title Bar
    let title_h = 22.0;
    let title_r = Rect::new(r.x, r.y, r.w, title_h);
    ui.c.fill_rrect(title_r, 4.0, t.s(0x1a1c1e));
    ui.c.fill_rect(Rect::new(r.x, title_r.bottom() - 3.0, r.w, 3.0), t.s(0x1a1c1e));
    ui.c.fill_rect(Rect::new(r.x, title_r.bottom() - 1.0, r.w, 1.0), t.s(0x121314));

    icons::draw(ui.c, Icon::PianoRoll, Rect::new(r.x + 6.0, r.y + 4.0, 14.0, 14.0), t.tx(0xffb020));
    let pat_name = pat.and_then(|p| p.name.as_deref()).unwrap_or("Pattern 1");
    let title_str = format!("CLIP • {pat_name} • {}", ch.name.as_deref().unwrap_or("Untitled"));
    ui.c.text(Font::new(11), r.x + 24.0, r.y + 4.0, t.text, &title_str);

    // Middle toggle buttons: [ Fold: On/Off ] [ Mode: Roll / 16 Pads ]
    let fold_lbl = if s.device_strip_fold { "Fold: ON" } else { "Fold: OFF" };
    let fold_btn_r = Rect::new(r.right() - 250.0, r.y + 2.0, 68.0, 18.0);
    let hov_fold = ui.hit(fold_btn_r, Action::DeviceStripToggleFold, "Fold mode: only show used notes");
    let fold_bg = if s.device_strip_fold { t.s(0x383e44) } else if hov_fold { t.s(0x282c2f) } else { t.s(0x1e2124) };
    lk::outlined(ui.c, fold_btn_r, 3.0, t.s(0x2a2e32), fold_bg);
    ui.c.text_center(Font::new(10), fold_btn_r, if s.device_strip_fold { t.tx(0xffffff) } else { t.text_dim }, fold_lbl);

    let mode_lbl = if s.device_strip_pads { "16 Pads" } else { "Piano Roll" };
    let mode_btn_r = Rect::new(r.right() - 176.0, r.y + 2.0, 74.0, 18.0);
    let hov_mode = ui.hit(mode_btn_r, Action::DeviceStripTogglePads, "Toggle between Piano Roll and 16-Step Pad Sequencer");
    let mode_bg = if s.device_strip_pads { t.brand_lit(os_theme::design::lemon::MAIN).mix(t.s(0x1a1c1e), 0.7) } else if hov_mode { t.s(0x282c2f) } else { t.s(0x1e2124) };
    lk::outlined(ui.c, mode_btn_r, 3.0, t.s(0x2a2e32), mode_bg);
    ui.c.text_center(Font::new(10), mode_btn_r, if s.device_strip_pads { t.tx(0xffffff) } else { t.text_dim }, mode_lbl);

    let double_hint = "Open full Piano Roll window";
    let open_btn_r = Rect::new(r.right() - 96.0, r.y + 2.0, 92.0, 18.0);
    let hov_open = ui.hit(open_btn_r, Action::ShowMax(crate::app::Win::PianoRoll), double_hint);
    lk::outlined(ui.c, open_btn_r, 3.0, t.s(0x2a2e32), if hov_open { t.s(0x32373c) } else { t.s(0x1e2124) });
    ui.c.text_center(Font::new(10), open_btn_r, if hov_open { t.tx(0xffffff) } else { t.tx(0x8e969d) }, "Open Window ↗");

    // Body Area
    let body_r = Rect::new(r.x + 1.0, r.y + title_h, r.w - 2.0, (r.h - title_h - 1.0).max(0.0));
    ui.c.push_clip(body_r);

    let ppq = s.project.ppq.max(4) as f32;
    let ticks_per_bar = ppq * 4.0;
    let total_ticks = (ticks_per_bar * 4.0).max(1.0);
    let step_ticks = ppq / 4.0;

    let notes: Vec<&os_model::Note> = pat.map_or(Vec::new(), |p| {
        p.notes.iter().filter(|n| n.channel == ch.index).collect()
    });
    let key_names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let note_c = ch.color.map_or(Color::rgb(0x42a5f5), rgb);

    // MODE 1: 16-Step Pad Sequencer Mode
    if s.device_strip_pads {
        let pad_margin = 4.0;
        let pad_y = body_r.y + 12.0;
        let pad_h = (body_r.h - 24.0).max(20.0);
        let avail_w = body_r.w - 16.0;
        let pad_w = ((avail_w - (15.0 * pad_margin)) / 16.0).max(8.0);

        // Header for Step Sequencer
        ui.c.text(Font::new(10), body_r.x + 8.0, body_r.y + 2.0, t.text_dim, "16-STEP SEQUENCER (click pad to toggle step)");

        for step in 0..16 {
            let px = body_r.x + 8.0 + (step as f32) * (pad_w + pad_margin);
            let pad_r = Rect::new(px, pad_y, pad_w, pad_h);

            // Check if note is active at this step
            let is_active = notes.iter().any(|n| (n.position as f32 / step_ticks).floor() as usize == step);
            let beat_group = step / 4;
            let is_beat_start = step % 4 == 0;
            let group_base = if beat_group % 2 == 0 { t.s(0x232629) } else { t.s(0x191b1c) };

            let hov = ui.hit(pad_r, Action::DeviceStripStepClick(step), format!("Step {}: toggle note", step + 1));
            let fill = if is_active {
                if hov { note_c.shade(0.15) } else { note_c }
            } else if hov {
                t.s(0x353a3f)
            } else {
                group_base
            };

            lk::outlined(ui.c, pad_r, 4.0, if is_active { note_c.shade(-0.2) } else if is_beat_start { t.s(0x383e44) } else { t.s(0x1c1e20) }, fill);

            // Step number label
            let num_lbl = format!("{}", step + 1);
            let num_c = if is_active { Color::rgb(0x10151a) } else if is_beat_start { t.tx(0xffb020) } else { t.tx(0x656c73) };
            ui.c.text_center(Font::new(10), Rect::new(pad_r.x, pad_r.y + 4.0, pad_r.w, 12.0), num_c, &num_lbl);

            // LED indicator inside pad
            let led_r = Rect::new(pad_r.cx() - 4.0, pad_r.bottom() - 10.0, 8.0, 4.0);
            ui.c.fill_rrect(led_r, 1.0, if is_active { Color::rgb(0xffffff) } else { Color::rgb(0x000000).with_alpha(80) });
        }

        // Playhead for Pads
        if f.playing {
            let cur_step = ((f.tick as f32 % total_ticks) / step_ticks).floor() as usize;
            if cur_step < 16 {
                let px = body_r.x + 8.0 + (cur_step as f32) * (pad_w + pad_margin);
                let head_r = Rect::new(px - 1.0, pad_y - 2.0, pad_w + 2.0, pad_h + 4.0);
                ui.c.stroke_rrect(head_r, 5.0, 1.5, Color::rgb(0xffffff).with_alpha(200));
            }
        }

        ui.c.pop_clip();
        return;
    }

    // MODE 2: Piano Roll / Clip View (Folded or Semitones)
    let ruler_h = 16.0;
    let vel_h = 22.0;

    let key_col_w = 40.0;
    let grid_x = body_r.x + key_col_w;
    let grid_w = (body_r.w - key_col_w).max(10.0);
    let grid_y = body_r.y + ruler_h;
    let grid_h = (body_r.h - ruler_h - vel_h).max(20.0);
    let vel_y = grid_y + grid_h;

    // Time Ruler
    let ruler_r = Rect::new(grid_x, body_r.y, grid_w, ruler_h);
    ui.c.fill_rect(ruler_r, t.s(0x181a1c));
    ui.c.fill_rect(Rect::new(ruler_r.x, ruler_r.bottom() - 1.0, ruler_r.w, 1.0), t.s(0x202428));

    for bar in 0..4 {
        let bx = grid_x + ((bar as f32) * ticks_per_bar / total_ticks) * grid_w;
        let bar_num = bar + 1;
        ui.c.text(Font::new(9), bx + 4.0, body_r.y + 2.0, t.tx(0x9098a0), &format!("{bar_num}"));
        for beat in 1..4 {
            let bex = grid_x + (((bar as f32) * ticks_per_bar + (beat as f32) * ppq) / total_ticks) * grid_w;
            ui.c.fill_rect(Rect::new(bex, body_r.y + 8.0, 1.0, 6.0), t.s(0x30353a));
        }
    }

    // Loop Brace indicator bar across top
    let loop_bar_h = 2.0;
    ui.c.fill_rect(Rect::new(grid_x, body_r.y, grid_w, loop_bar_h), t.brand_lit(os_theme::design::lemon::MAIN));

    // Determine Pitches / Rows to show
    let pitches_to_show: Vec<u8> = if s.device_strip_fold && !notes.is_empty() {
        let mut uniq: Vec<u8> = notes.iter().map(|n| n.key as u8).collect();
        uniq.sort_unstable();
        uniq.dedup();
        uniq.reverse(); // Highest pitch on top
        if uniq.len() < 4 {
            let base = uniq[0];
            let mut padded = Vec::new();
            for offset in 0..6 {
                padded.push(base.saturating_sub(offset));
            }
            padded
        } else {
            uniq
        }
    } else {
        let avg_pitch = if notes.is_empty() {
            60
        } else {
            (notes.iter().map(|n| n.key as usize).sum::<usize>() / notes.len()).clamp(30, 90) as u8
        };
        let num_rows = 16usize;
        let base_key = avg_pitch.saturating_sub(8).clamp(12, 110);
        (0..num_rows).map(|row| (base_key as usize + (num_rows - 1 - row)) as u8).collect()
    };

    let num_rows = pitches_to_show.len().max(1);
    let rh = grid_h / (num_rows as f32);

    // Draw Keys & Grid Rows
    for (row_idx, &k) in pitches_to_show.iter().enumerate() {
        let y = grid_y + (row_idx as f32) * rh;
        let is_black = [1, 3, 6, 8, 10].contains(&(k % 12));
        let is_c = (k % 12) == 0;

        // Piano Key
        let key_r = Rect::new(body_r.x, y, key_col_w, rh);
        let key_bg = if is_black { t.s(0x181a1b) } else { t.s(0x2a2d30) };
        let hov_key = ui.hit(key_r, Action::DeviceStripKey(k), format!("Play {}", key_names[(k % 12) as usize]));
        let fill = if hov_key { t.s(0x404449) } else { key_bg };
        ui.c.fill_rect(key_r, fill);
        ui.c.fill_rect(Rect::new(key_r.x, key_r.bottom() - 1.0, key_r.w, 1.0), t.s(0x141617));

        let oct = (k / 12).saturating_sub(1);
        let lbl = format!("{}{oct}", key_names[(k % 12) as usize]);
        let lbl_col = if is_c { t.tx(0xffb020) } else if is_black { t.tx(0x808890) } else { t.tx(0xb0b6be) };
        ui.c.text(Font::new(10), key_r.x + 4.0, y + (rh - 12.0) / 2.0, lbl_col, &lbl);

        // Grid Row
        let row_r = Rect::new(grid_x, y, grid_w, rh);
        let row_bg = if is_black { t.s(0x121415) } else { t.s(0x16181a) };
        ui.c.fill_rect(row_r, row_bg);
        ui.c.fill_rect(Rect::new(grid_x, y + rh - 1.0, grid_w, 1.0), t.s(0x101112));
    }

    // Grid Columns (Bars & Beats)
    for bar in 0..=4 {
        let bx = grid_x + ((bar as f32) * ticks_per_bar / total_ticks) * grid_w;
        ui.c.fill_rect(Rect::new(bx, grid_y, 1.0, grid_h + vel_h), t.s(0x282c30));
        if bar < 4 {
            for beat in 1..4 {
                let bex = grid_x + (((bar as f32) * ticks_per_bar + (beat as f32) * ppq) / total_ticks) * grid_w;
                ui.c.fill_rect(Rect::new(bex, grid_y, 1.0, grid_h + vel_h), t.s(0x1c1e21));
            }
        }
    }

    // Velocity Lane Background
    let vel_r = Rect::new(grid_x, vel_y, grid_w, vel_h);
    ui.c.fill_rect(vel_r, t.s(0x111314));
    ui.c.fill_rect(Rect::new(vel_r.x, vel_r.y, vel_r.w, 1.0), t.s(0x24282c));
    ui.c.text(Font::new(8), body_r.x + 4.0, vel_y + 4.0, t.tx(0x606870), "VEL");

    // Draw Notes and Velocity Stems
    for n in &notes {
        let Some(row_pos) = pitches_to_show.iter().position(|&p| p == n.key as u8) else { continue };
        let ny = grid_y + (row_pos as f32) * rh + 1.0;
        let nx = grid_x + ((n.position as f32) / total_ticks) * grid_w;
        let nw = (((n.length as f32) / total_ticks) * grid_w).max(4.0);

        let nr = Rect::new(nx, ny, nw, (rh - 2.0).max(2.0));
        ui.c.fill_rrect(nr, 2.5, note_c);
        ui.c.fill_rect(Rect::new(nr.x, nr.bottom() - 1.0, nr.w, 1.0), note_c.shade(-0.3));

        if nw > 20.0 {
            let kname = key_names[(n.key % 12) as usize];
            ui.c.text(Font::new(9), nr.x + 3.0, nr.y + (rh - 12.0) / 2.0, Color::rgb(0x10151a), kname);
        }

        // Velocity stem in bottom velocity lane
        let vel_frac = (n.velocity as f32 / 127.0).clamp(0.05, 1.0);
        let stem_h = (vel_h - 4.0) * vel_frac;
        let stem_x = nx + 2.0;
        let stem_y = vel_r.bottom() - stem_h;
        let stem_col = if vel_frac > 0.8 {
            Color::rgb(0xff6060)
        } else if vel_frac > 0.5 {
            note_c
        } else {
            Color::rgb(0x40c080)
        };
        ui.c.fill_rect(Rect::new(stem_x, stem_y, 1.5, stem_h), stem_col.with_alpha(190));
        ui.c.fill_circle(stem_x + 0.75, stem_y, 2.0, stem_col);
    }

    // Playhead line
    if f.playing {
        let cur_tick = f.tick as f32 % total_ticks;
        let px = grid_x + (cur_tick / total_ticks) * grid_w;
        ui.c.fill_rect(Rect::new(px, body_r.y, 2.0, body_r.h), Color::rgb(0xffffff));
    }

    // Grid Hit to open / focus
    ui.hit(Rect::new(grid_x, grid_y, grid_w, grid_h), Action::ShowMax(crate::app::Win::PianoRoll), "Double-click to open full Piano Roll");

    ui.c.pop_clip();
}

fn draw_connector(ui: &mut Ui, cx: f32, cy: f32) {
    let t = ui.t;
    // Subtle horizontal signal line and chevron
    let line_y = cy;
    ui.c.fill_rect(Rect::new(cx + 2.0, line_y, 8.0, 1.0), t.s(0x383e42));
    chevron(ui, cx + 12.0, cy, true, t.tx(0x525a60));
}
