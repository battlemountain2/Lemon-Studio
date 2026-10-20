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

    ui.c.push_clip(body_r);

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
    let hov_route = ui.hit(route_r, Action::Fader(target_mixer), "Select target mixer track");
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
}

fn draw_connector(ui: &mut Ui, cx: f32, cy: f32) {
    let t = ui.t;
    // Subtle horizontal signal line and chevron
    let line_y = cy;
    ui.c.fill_rect(Rect::new(cx + 2.0, line_y, 8.0, 1.0), t.s(0x383e42));
    chevron(ui, cx + 12.0, cy, true, t.tx(0x525a60));
}
