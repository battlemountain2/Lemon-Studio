use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use nih_plug::prelude::*;
use os_hostlink::{Cmd, PageIcon, Peer};
use os_plugkit::gui::{self, Button, Input};
use os_plugkit::vals::Vals;
use os_ui::Canvas;
use os_ui::kit::menu::{MenuItem, MenuStyle};

use crate::{
    Scope, Shared,
    params::{self, RayParams},
    view,
};

pub struct RayEditor {
    params: Arc<RayParams>,
    shared: Arc<Shared>,
    scale: AtomicU32,
    slot: gui::Slot,
}

impl RayEditor {
    pub fn new(params: Arc<RayParams>, shared: Arc<Shared>) -> Self {
        Self { params, shared, scale: AtomicU32::new(1f32.to_bits()), slot: Default::default() }
    }
}

impl Editor for RayEditor {
    fn spawn(&self, parent: ParentWindowHandle, context: Arc<dyn GuiContext>) -> Box<dyn Any + Send> {
        let scale = f32::from_bits(self.scale.load(Ordering::Relaxed));
        let (parent, scale) = match parent {
            #[cfg(target_os = "linux")]
            ParentWindowHandle::X11Window(window) => (window, scale),
            #[cfg(target_os = "macos")]
            ParentWindowHandle::AppKitNsView(view) => (view as usize, gui::backing_scale(view as usize)),
            _ => return Box::new(()),
        };
        let params = self.params.clone();
        let shared = self.shared.clone();
        let wake = self.slot.clone();
        let link = Peer::register(parent as u64, vec![PageIcon::Plugin], (view::W, view::H), move || gui::notify(&wake));
        let peer = link.clone();
        let size = ((view::W as f32 * scale).round() as u32, (view::H as f32 * scale).round() as u32);
        let thread = gui::spawn(parent, size, &self.slot, move |window| {
            let Ok(canvas) = Canvas::new(size.0, size.1, scale) else {
                peer.unregister();
                return;
            };
            let logic = Logic::new(params, Some(context));
            Window {
                window,
                canvas,
                buffer: vec![0; (size.0 * size.1) as usize],
                size,
                scale,
                logic,
                shared,
                scope: Scope::default(),
                sequence: u64::MAX,
                link: peer,
            }
            .run();
        });
        if thread.is_none() {
            link.unregister();
        }
        Box::new(thread)
    }

    fn size(&self) -> (u32, u32) {
        (view::W, view::H)
    }

    fn set_scale_factor(&self, factor: f32) -> bool {
        if !factor.is_finite() || !(0.5..=4.0).contains(&factor) {
            return false;
        }
        self.scale.store(factor.to_bits(), Ordering::Relaxed);
        true
    }

    fn param_value_changed(&self, _id: &str, _value: f32) {
        gui::notify(&self.slot);
    }
    fn param_modulation_changed(&self, _id: &str, _offset: f32) {
        gui::notify(&self.slot);
    }
    fn param_values_changed(&self) {
        gui::notify(&self.slot);
    }
}

pub struct Logic {
    pub vals: Vals,
    pub focus: Option<usize>,
    _params: Arc<RayParams>,
    dragging: bool,
}

impl Logic {
    pub fn new(params: Arc<RayParams>, context: Option<Arc<dyn GuiContext>>) -> Self {
        Self { vals: Vals::new(&*params, params.live.clone(), context), _params: params, focus: None, dragging: false }
    }

    pub fn hit(horizontal: f32, vertical: f32) -> Option<usize> {
        (0..5).find(|index| view::track(*index).outset(8.0).contains(horizontal, vertical))
    }

    fn end(&mut self) {
        if self.dragging {
            if let Some(index) = self.focus {
                self.vals.end(params::IDS[index]);
            }
        }
        self.dragging = false;
    }

    pub fn down(&mut self, horizontal: f32, vertical: f32, reset: bool) {
        self.end();
        self.focus = Self::hit(horizontal, vertical);
        if let Some(index) = self.focus {
            let id = params::IDS[index];
            if reset {
                self.vals.edit(id, self.vals.default_norm(id));
            } else {
                self.vals.begin(id);
                self.dragging = true;
                self.moved(horizontal);
            }
        } else if view::PREVIOUS.contains(horizontal, vertical) {
            self.load_preset((view::preset(&self.vals).unwrap_or(0) + params::PRESETS.len() - 1) % params::PRESETS.len());
        } else if view::NEXT.contains(horizontal, vertical) || view::PRESET.contains(horizontal, vertical) {
            self.load_preset((view::preset(&self.vals).unwrap_or(params::PRESETS.len() - 1) + 1) % params::PRESETS.len());
        }
    }

    pub fn moved(&mut self, horizontal: f32) {
        if self.dragging {
            if let Some(index) = self.focus {
                let rect = view::track(index);
                self.vals.set(params::IDS[index], (horizontal - rect.x) / rect.w);
            }
        }
    }

    pub fn wheel(&mut self, horizontal: f32, vertical: f32, notches: f32) {
        if let Some(index) = Self::hit(horizontal, vertical) {
            self.focus = Some(index);
            self.vals.edit(params::IDS[index], self.vals.norm(params::IDS[index]) + notches.signum() * 0.01);
        }
    }

    pub fn key(&mut self, symbol: u32) {
        if symbol == 0xff09 {
            self.end();
            self.focus = Some(self.focus.map_or(0, |index| (index + 1) % 5));
            return;
        }
        let Some(index) = self.focus else { return };
        let id = params::IDS[index];
        match symbol {
            0xff51 | 0xff54 => self.vals.edit(id, self.vals.norm(id) - 0.01),
            0xff52 | 0xff53 => self.vals.edit(id, self.vals.norm(id) + 0.01),
            0xff50 => self.vals.edit(id, 0.0),
            0xff57 => self.vals.edit(id, 1.0),
            0xff08 => self.vals.edit(id, self.vals.default_norm(id)),
            _ => {}
        }
    }

    pub fn load_preset(&mut self, index: usize) {
        self.end();
        if let Some((_, values)) = params::PRESETS.get(index) {
            for (id, value) in params::IDS.iter().zip(values) {
                self.vals.edit_plain(id, *value);
            }
        }
    }
}

impl Drop for Logic {
    fn drop(&mut self) {
        self.end();
    }
}

struct Window {
    window: gui::Window,
    canvas: Canvas,
    buffer: Vec<u32>,
    size: (u32, u32),
    scale: f32,
    logic: Logic,
    shared: Arc<Shared>,
    scope: Scope,
    sequence: u64,
    link: Arc<Peer>,
}

impl Drop for Window {
    fn drop(&mut self) {
        self.link.unregister();
    }
}

impl Window {
    fn run(mut self) {
        let mut dirty = true;
        let mut signature = 0;
        while let Some(input) = self.window.next() {
            match input {
                Input::Down { x, y, btn, dbl } => {
                    let (horizontal, vertical) = (x / self.scale, y / self.scale);
                    match btn {
                        Button::Left | Button::Middle => self.logic.down(horizontal, vertical, dbl || btn == Button::Middle || self.window.mods().2),
                        Button::Right => self.menu(horizontal, vertical),
                    }
                    dirty = true;
                }
                Input::Move { x, .. } if self.logic.dragging => {
                    self.logic.moved(x / self.scale);
                    dirty = true;
                }
                Input::Up { .. } => {
                    self.logic.end();
                    dirty = true;
                }
                Input::Wheel { x, y, notches } => {
                    self.logic.wheel(x / self.scale, y / self.scale, notches);
                    dirty = true;
                }
                Input::Key { sym, .. } => {
                    self.logic.key(sym);
                    dirty = true;
                }
                Input::Expose => dirty = true,
                Input::Tick => {
                    dirty |= self.logic.vals.expire();
                    if let Some((sequence, scope)) = self.shared.snapshot() {
                        if sequence != self.sequence {
                            dirty |= scope.voices > 0 || self.scope.voices > 0 || scope.waveform != self.scope.waveform;
                            self.sequence = sequence;
                            self.scope = scope;
                        }
                    }
                }
                _ => {}
            }
            let current = self.logic.vals.signature();
            dirty |= signature != current;
            if dirty && self.window.idle() {
                view::paint(&mut self.canvas, &self.logic.vals, &self.scope, self.logic.focus);
                self.canvas.copy_to_xrgb(&mut self.buffer);
                self.window.blit(&self.buffer, self.size.0, self.size.1);
                signature = current;
                dirty = false;
            }
        }
    }

    fn menu(&mut self, horizontal: f32, vertical: f32) {
        self.logic.end();
        let Some(index) = Logic::hit(horizontal, vertical) else { return };
        self.logic.focus = Some(index);
        let id = params::IDS[index];
        let mut menu = os_plugkit::MenuDef::new(MenuStyle::Context);
        menu.head(params::NAMES[index], 0);
        menu.add(MenuItem::item("Reset"), 1);
        menu.add(MenuItem::item("Type in value..."), 2);
        menu.add(MenuItem::item("Create automation clip").enabled(self.link.hosted()), 3);
        let (horizontal, vertical) = self.window.to_root((horizontal * self.scale) as i32, (vertical * self.scale) as i32);
        match self.window.popup(horizontal, vertical, &menu, &os_plugkit::palette(), self.scale) {
            Some(1) => self.logic.vals.edit(id, self.logic.vals.default_norm(id)),
            Some(2) => {
                if let Some(text) = self.window.type_in(horizontal, vertical, &self.logic.vals.text(id), self.scale) {
                    if let Some(value) = self.logic.vals.parse(id, &text) {
                        self.logic.vals.edit(id, value);
                    }
                }
            }
            Some(3) => {
                self.link.to_host(Cmd::Automate, index as isize);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sliders_keyboard_reset_and_presets_reach_audio_immediately() {
        let params = Arc::new(RayParams::default());
        let mut logic = Logic::new(params.clone(), None);
        let rect = view::track(0);
        logic.down(rect.cx(), rect.cy(), false);
        assert_eq!(params.settings().sweep, 24.0);
        logic.moved(rect.right() + 100.0);
        logic.end();
        assert_eq!(params.settings().sweep, 48.0);
        logic.down(rect.cx(), rect.cy(), true);
        assert_eq!(params.settings().sweep, 36.0);
        logic.key(0xff50);
        assert_eq!(params.settings().sweep, 0.0);
        logic.wheel(rect.cx(), rect.cy(), 1.0);
        assert!(params.settings().sweep > 0.0);
        logic.key(0xff09);
        assert_eq!(logic.focus, Some(1));
        logic.load_preset(1);
        assert_eq!(view::preset(&logic.vals), Some(1));
        assert_eq!(params.settings().sweep, 12.0);
    }
}
