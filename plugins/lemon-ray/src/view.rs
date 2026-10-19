use os_plugkit::vals::Vals;
use os_theme::design::lemon as brand;
use os_ui::{Canvas, Font, Rect};

use crate::{Scope, dsp, params};

pub const W: u32 = 720;
pub const H: u32 = 500;
pub const PREVIOUS: Rect = Rect::new(460.0, 57.0, 32.0, 30.0);
pub const NEXT: Rect = Rect::new(660.0, 57.0, 32.0, 30.0);
pub const PRESET: Rect = Rect::new(496.0, 57.0, 160.0, 30.0);

pub fn track(index: usize) -> Rect {
    Rect::new(28.0 + index as f32 * 138.0, 332.0, 112.0, 24.0)
}

pub fn preset(vals: &Vals) -> Option<usize> {
    params::PRESETS.iter().position(|(_, values)| params::IDS.iter().zip(values).all(|(id, value)| (vals.plain(id) - value).abs() < 0.05))
}

pub fn settings(vals: &Vals) -> dsp::Settings {
    dsp::Settings::from_values(std::array::from_fn(|index| vals.plain(params::IDS[index])))
}

pub fn paint(canvas: &mut Canvas, vals: &Vals, scope: &Scope, focus: Option<usize>) {
    let palette = os_plugkit::palette();
    let text = palette.text;
    let secondary = text.mix(palette.toolbar, 0.35);
    let accent = brand::MAIN;
    let small = Font::new(13);
    let normal = Font::new(16);
    let settings = settings(vals);
    canvas.fill_rect(Rect::new(0.0, 0.0, W as f32, H as f32), palette.toolbar);
    canvas.text(Font::new(27), 26.0, 12.0, text, "LEMON RAY");
    canvas.text(small, 200.0, 25.0, secondary, "Sound Canvas  /  Laser");
    canvas.text(normal, 28.0, 62.0, text, "Pitch-envelope + FM voice");
    for (rect, label) in [(PREVIOUS, "<"), (NEXT, ">"), (PRESET, params::PRESETS.get(preset(vals).unwrap_or(usize::MAX)).map_or("Custom", |preset| preset.0))] {
        canvas.fill_rrect(rect, 5.0, palette.button);
        canvas.text_center(normal, rect, text, label);
    }
    canvas.fill_rrect(Rect::new(24.0, 104.0, 672.0, 191.0), 8.0, palette.sunken_dark);
    canvas.text(normal, 38.0, 113.0, text, "Pitch journey");
    canvas.text_right(small, 681.0, 116.0, text, &format!("+{:.0} st -> root  /  {:.0} ms", settings.sweep, settings.length * 1000.0));
    let plot = Rect::new(80.0, 157.0, 598.0, 100.0);
    for value in [0.0, 24.0, 48.0] {
        let vertical = plot.bottom() - value / 48.0 * plot.h;
        canvas.line(plot.x, vertical, plot.right(), vertical, 1.0, palette.toolbar_edge);
        canvas.text_right(small, 67.0, vertical - 8.0, secondary, &if value == 0.0 { "Root".into() } else { format!("+{value:.0}") });
    }
    canvas.text(small, 38.0, 137.0, secondary, "Pitch (st)");
    let curve: Vec<_> = (0..=240)
        .map(|index| {
            let fraction = f64::from(index) / 240.0;
            (plot.x + fraction as f32 * plot.w, plot.bottom() - (dsp::pitch_drop(fraction * settings.length, settings) / 48.0) as f32 * plot.h)
        })
        .collect();
    canvas.polyline(&curve, 2.0, accent);
    if scope.voices > 0 && scope.progress >= 0.0 {
        let fraction = scope.progress.clamp(0.0, 1.0);
        let horizontal = plot.x + fraction * plot.w;
        canvas.line(horizontal, plot.y, horizontal, plot.bottom(), 1.0, secondary);
        canvas.fill_circle(horizontal, plot.bottom() - (dsp::pitch_drop(f64::from(fraction) * settings.length, settings) / 48.0) as f32 * plot.h, 4.0, accent);
    }
    canvas.text(small, plot.x, 267.0, secondary, "0");
    canvas.text_center(small, Rect::new(270.0, 264.0, 220.0, 22.0), secondary, "Time (ms)");
    canvas.text_right(small, plot.right(), 267.0, secondary, &format!("{:.0}", settings.length * 1000.0));
    for index in 0..5 {
        let rect = track(index);
        let id = params::IDS[index];
        canvas.text(normal, rect.x, 306.0, text, params::NAMES[index]);
        canvas.text_right(small, rect.right(), 310.0, secondary, &vals.text(id));
        let position = rect.x + vals.norm(id) * rect.w;
        canvas.fill_rrect(Rect::new(rect.x, rect.cy() - 2.0, rect.w, 4.0), 2.0, palette.sunken_dark);
        canvas.fill_rrect(Rect::new(rect.x, rect.cy() - 2.0, (position - rect.x).max(1.0), 4.0), 2.0, accent);
        canvas.fill_circle(position, rect.cy(), 5.0, accent);
        if focus == Some(index) {
            canvas.stroke_rrect(rect.outset(3.0), 5.0, 1.0, text);
        }
        canvas.text(small, rect.x, 364.0, secondary, params::HINTS[index]);
    }
    canvas.text(normal, 28.0, 400.0, text, "Live output (auto-scaled)");
    let peak = scope.waveform.iter().map(|value| value.abs()).fold(0.0, f32::max);
    let level = if peak > 0.00001 { format!("Peak {:.0} dBFS", 20.0 * peak.log10()) } else { "Silent".into() };
    canvas.text_right(small, 691.0, 403.0, secondary, &level);
    let points: Vec<_> = scope
        .waveform
        .iter()
        .enumerate()
        .map(|(index, value)| (28.0 + index as f32 / (scope.waveform.len() - 1) as f32 * 664.0, 445.0 - value / peak.max(0.00001) * 16.0))
        .collect();
    canvas.push_clip(Rect::new(26.0, 424.0, 668.0, 38.0));
    canvas.line(28.0, 445.0, 692.0, 445.0, 1.0, palette.toolbar_edge);
    canvas.polyline(&points, 1.5, accent);
    canvas.pop_clip();
    canvas.text(small, 28.0, 476.0, secondary, "Drag or scroll  /  Double-click resets  /  Right-click for value or automation");
    canvas.text_right(small, 692.0, 476.0, text, &format!("{} / 32 voices", scope.voices));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn sound_canvas_responds_and_fits_at_hyprland_scales() {
        let params = Arc::new(params::RayParams::default());
        let mut vals = Vals::new(&*params, params.live.clone(), None);
        for scale in [1.0, 1.5, 2.0] {
            let mut canvas = Canvas::new((W as f32 * scale) as u32, (H as f32 * scale) as u32, scale).unwrap();
            paint(&mut canvas, &vals, &Scope::default(), None);
            let before = canvas.pixmap.data().to_vec();
            vals.edit_plain("sweep", 12.0);
            paint(&mut canvas, &vals, &Scope::default(), Some(0));
            assert_ne!(before, canvas.pixmap.data());
            assert!(params.settings().sweep == 12.0);
            vals.edit_plain("sweep", 36.0);
            if let Some(directory) = std::env::var_os("OS_SHOT_DIR") {
                std::fs::create_dir_all(&directory).unwrap();
                canvas.pixmap.save_png(std::path::Path::new(&directory).join(format!("lemon-ray-{scale}.png"))).unwrap();
            }
        }
        for index in 0..5 {
            assert!(track(index).right() < W as f32 && track(index).bottom() < H as f32);
        }
        if let Some(directory) = std::env::var_os("OS_SHOT_DIR") {
            let mut engine = dsp::Engine::new(48000.0);
            engine.note_on(0, 60, 0.8, None);
            let mut scope = Scope::default();
            let mut right = [0.0; crate::SCOPE_SAMPLES];
            for _ in 0..6 { engine.render(&mut scope.waveform, &mut right, params.settings(), |_, _| {}); }
            scope.progress = engine.progress(params.settings());
            scope.voices = engine.active() as u32;
            let mut canvas = Canvas::new(W, H, 1.0).unwrap();
            paint(&mut canvas, &vals, &scope, None);
            canvas.pixmap.save_png(std::path::Path::new(&directory).join("lemon-ray-live.png")).unwrap();
        }
    }
}
