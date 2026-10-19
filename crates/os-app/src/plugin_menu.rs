// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Plugin menus (the full-screen plugin picker is retired): F8, the toolbar's plugin button, the channel rack's "+",
//! Add one ▸ / Insert ▸ and the mixer slots open the app's own popup listing the installed plugins, and a pick adds
//! the plugin where that menu says.
// spec: ui/plugin-menu.md

use std::collections::BTreeMap;

use crate::app::{Action, App, State};
use crate::playlist::{Cmd, Mark, PItem, Popup};
use crate::plugin_db::{self as db, Entry, MenuView, Mode};

/// What a plugin menu entry does (`Cmd::Plugin`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlugCmd {
    /// Generator `index` (into `Plugins::available`): a new channel at the bottom of the rack, or right below row
    /// `after`.
    Generator { index: usize, after: Option<usize> },
    /// Effect `index` (into `Plugins::effects`) into the selected mixer track: slot `slot`, else its first empty slot.
    Effect { index: usize, slot: Option<u8> },
    /// Find new plugins (a background scan).
    Find,
    /// Settings > Plugins.
    Manage,
    View(MenuView),
}

/// Which plugins a menu lists and where a pick goes (spec §1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    /// F8, the toolbar button, VIEW / ADD > Plugin picker: generators and effects.
    All,
    /// Generators: a new channel at the bottom (`None`) or right below row `r`.
    Gen(Option<usize>),
    /// Effects into the selected track: slot `k`, else its first empty slot.
    Fx(Option<u8>),
}

fn pick(target: Target, effect: bool, index: usize) -> Cmd {
    Cmd::Plugin(match (target, effect) {
        (Target::Fx(slot), _) => PlugCmd::Effect { index, slot },
        (_, true) => PlugCmd::Effect { index, slot: None },
        (Target::Gen(after), false) => PlugCmd::Generator { index, after },
        (Target::All, false) => PlugCmd::Generator { index, after: None },
    })
}

fn item(e: &Entry, target: Target, effect: bool) -> PItem {
    PItem::new(e.label.clone(), pick(target, effect, e.index))
}

/// A header with a run-time label.
fn head(label: String) -> PItem {
    PItem { head: true, ..PItem::new(label, Cmd::Nothing) }
}

/// One section's entries (spec §2): favourites, a separator, then the rest by display mode. `nested`: the list is a
/// submenu itself, so "By vendor" uses headers instead of submenus (popups open one submenu level).
fn section(s: &State, effect: bool, target: Target, nested: bool) -> Vec<PItem> {
    let list = if effect { &s.plugdb.lists.fx } else { &s.plugdb.lists.gens };
    if list.is_empty() {
        return vec![PItem::new(if effect { "(no effect plugins found)" } else { "(no instruments found)" }, Cmd::Nothing).off()];
    }
    let mut v: Vec<PItem> = list.iter().filter(|e| e.favorite).map(|e| item(e, target, effect)).collect();
    let rest: Vec<&Entry> = list.iter().filter(|e| !e.favorite).collect();
    if !v.is_empty() && !rest.is_empty() {
        v.push(PItem::sep());
    }
    match s.plugdb.prefs.menu_view {
        MenuView::Simple => v.extend(rest.iter().map(|e| item(e, target, effect))),
        MenuView::Categories => {
            for cat in db::categories(effect) {
                let in_cat: Vec<&&Entry> = rest.iter().filter(|e| e.category == cat).collect();
                if !in_cat.is_empty() {
                    v.push(PItem::head(cat));
                    v.extend(in_cat.iter().map(|e| item(e, target, effect)));
                }
            }
        }
        MenuView::Vendor => {
            let mut by: BTreeMap<String, (String, Vec<&Entry>)> = BTreeMap::new();
            for e in &rest {
                by.entry(e.vendor.to_lowercase()).or_insert_with(|| (e.vendor.clone(), Vec::new())).1.push(e);
            }
            for (_, (vendor, es)) in by {
                let items: Vec<PItem> = es.iter().map(|e| item(e, target, effect)).collect();
                if nested {
                    v.push(head(vendor));
                    v.extend(items);
                } else {
                    v.push(PItem { children: items, ..PItem::new(vendor, Cmd::Nothing) });
                }
            }
        }
    }
    v
}

/// The display modes as their radio items name them (spec §2-3); ADD's radios are the same.
pub const VIEWS: &[(&str, MenuView)] = &[("Categories", MenuView::Categories), ("Simple", MenuView::Simple), ("By vendor", MenuView::Vendor)];

/// After the lists (spec §2): Find new plugins, Manage plugins..., the display modes.
fn footer(s: &State) -> Vec<PItem> {
    let view = s.plugdb.prefs.menu_view;
    let scanning = s.plugdb.progress.is_some();
    let find = PItem::new(if scanning { "Find new plugins (scanning...)" } else { "Find new plugins" }, Cmd::Plugin(PlugCmd::Find));
    let mut v = vec![PItem::sep(), if scanning { find.off() } else { find }, PItem::new("Manage plugins...", Cmd::Plugin(PlugCmd::Manage)), PItem::sep()];
    v.extend(VIEWS.iter().map(|&(label, m)| PItem::new(label, Cmd::Plugin(PlugCmd::View(m))).mark(Mark::Radio(view == m))));
    v
}

/// The entries of a plugin menu for `target`; `nested` when they fill a submenu.
pub fn items(s: &State, target: Target, nested: bool) -> Vec<PItem> {
    let mut v = Vec::new();
    match target {
        Target::All => {
            v.push(PItem::head("Generators"));
            v.extend(section(s, false, target, nested));
            v.push(PItem::col());
            v.push(PItem::head("Effects"));
            v.extend(section(s, true, target, nested));
        }
        Target::Gen(_) => v.extend(section(s, false, target, nested)),
        Target::Fx(_) => v.extend(section(s, true, target, nested)),
    }
    v.extend(footer(s));
    v
}

/// A plugin menu for `target` at (x, y).
pub fn menu(s: &State, target: Target, x: f32, y: f32) -> Popup {
    Popup { x, y, items: items(s, target, false), sub: None }
}

/// Whether the open popup is a plugin menu (F8 closes it).
pub fn is_open(s: &State) -> bool {
    s.playlist.popup.as_ref().is_some_and(|p| p.items.iter().any(|i| i.cmd == Cmd::Plugin(PlugCmd::Manage)))
}

impl App {
    /// Opens the plugin menu for `target` at (x, y); F8 while one is open closes it.
    pub fn open_plugin_menu(&mut self, target: Target, at: (f32, f32)) {
        self.state.plugin_search = Search::default();
        self.state.menu_open = None;
        self.state.playlist.popup = Some(menu(&self.state, target, at.0, at.1));
        self.invalidate();
    }

    /// F8 (spec §1): the all-plugins menu at the mouse, or closes an open plugin menu.
    pub fn toggle_plugin_menu(&mut self) {
        if is_open(&self.state) {
            self.state.playlist.popup = None;
            self.invalidate();
        } else {
            self.open_plugin_menu(Target::All, self.mouse);
        }
    }
}

/// Runs a plugin menu command. Returns a status message.
pub fn run(app: &mut App, c: PlugCmd) -> Option<String> {
    let m = match c {
        PlugCmd::Generator { index, after } => add_generator(app, index, after),
        PlugCmd::Effect { index, slot } => add_effect(app, index, slot),
        PlugCmd::Find => {
            app.start_scan(Mode::Changed);
            None
        }
        PlugCmd::Manage => {
            app.open_settings(crate::views::settings::PLUGINS);
            None
        }
        PlugCmd::View(v) => {
            app.state.plugdb.prefs.menu_view = v;
            app.prefs_changed();
            None
        }
    };
    app.invalidate();
    m
}

/// A new channel for generator `index` (spec §1): at the bottom of the rack, or right below row `after`; selected and
/// in view. Lemon Sampler makes an empty sampler channel (hosted on the next tick); any other generator a plugin
/// channel hosting it. Its plugin window opens once it is hosted.
pub fn add_generator(app: &mut App, index: usize, after: Option<usize>) -> Option<String> {
    let d = app.plugins.available.get(index)?.desc.clone();
    let s = &mut app.state;
    let sampler = d.name == crate::plugins::LEMON_SAMPLER && crate::project_plugins::is_builtin(&d);
    let mut ch = if sampler { crate::app::add_channel(&mut s.project) } else { crate::app::add_plugin_channel(&mut s.project, &d.name) };
    if sampler {
        s.pending_open = Some(ch);
    } else {
        s.pending_plugin = Some((ch, index));
    }
    s.piano.dirty = true;
    if let Some(r) = after.filter(|&r| r + 1 < ch) {
        let order: Vec<usize> = (0..=r).chain([ch]).chain(r + 1..ch).collect();
        crate::rack::reorder(app, &order);
        ch = r + 1;
    }
    let s = &mut app.state;
    s.selected_channel = ch;
    s.rack.select_only(ch);
    crate::rack::autosize(s);
    if after.is_none() {
        // Keep the new row in view.
        let r = crate::views::rack::rect(s);
        let view_h = crate::views::rack::panel(r).h;
        s.rack_scroll = (crate::views::rack::content_h(s) - view_h).max(0.0);
    }
    sampler.then(|| "Added a sampler channel (Lemon Sampler)".into())
}

/// Effect `index` into the selected mixer track (spec §1): slot `slot`, else the first empty one.
pub fn add_effect(app: &mut App, index: usize, slot: Option<u8>) -> Option<String> {
    let s = &app.state;
    let t = s.selected_mixer;
    let slot = slot.or_else(|| s.project.mixer.iter().find(|m| m.index == t).and_then(|m| m.slots.iter().position(Option::is_none)).map(|k| k as u8));
    match slot {
        Some(k) => crate::mixer::run(app, crate::mixer::MCmd::AddEffect(k, index)),
        None => Some("Every slot of this track is in use".into()),
    }
}

#[derive(Debug, Default)]
pub struct Search {
    pub text: String,
    pub top: usize,
    pub cursor: usize,
    pub selected_text: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hit {
    Area,
    Clear,
    Pick(PlugCmd),
    Scroll(i32),
}

pub fn searchable(items: &[PItem]) -> bool {
    items.iter().any(|item| item.cmd == Cmd::Plugin(PlugCmd::Manage))
}

pub fn active(s: &State) -> Option<&[PItem]> {
    let popup = s.playlist.popup.as_ref()?;
    if searchable(&popup.items) {
        return Some(&popup.items);
    }
    let children = &popup.items.get(popup.sub?)?.children;
    searchable(children).then_some(children.as_slice())
}

pub struct Choice<'a> {
    pub item: &'a PItem,
    pub entry: Option<&'a Entry>,
    pub effect: bool,
}

impl Choice<'_> {
    fn group(&self, view: MenuView) -> &str {
        if self.entry.is_some_and(|entry| entry.favorite) {
            return "Favorites";
        }
        match (view, self.entry) {
            (MenuView::Categories, Some(entry)) => entry.category,
            (MenuView::Vendor, Some(entry)) => &entry.vendor,
            _ => "",
        }
    }

    fn detail(&self, view: MenuView) -> String {
        let kind = if self.effect { "Effect" } else { "Instrument" };
        let mut parts = vec![kind.to_string()];
        if let Some(entry) = self.entry {
            let group = self.group(view);
            if !group.is_empty() {
                parts.push(group.into());
            }
            if !entry.vendor.is_empty() && entry.vendor != group {
                parts.push(entry.vendor.clone());
            }
            parts.push(entry.format.as_str().to_uppercase());
        }
        parts.join(" · ")
    }
}

pub fn choices<'a>(s: &'a State, items: &'a [PItem]) -> Vec<Choice<'a>> {
    let lookup: BTreeMap<_, _> = s.plugdb.lists.gens.iter().map(|entry| ((false, entry.index), entry))
        .chain(s.plugdb.lists.fx.iter().map(|entry| ((true, entry.index), entry))).collect();
    fn collect<'a>(items: &'a [PItem], lookup: &BTreeMap<(bool, usize), &'a Entry>, out: &mut Vec<Choice<'a>>) {
        for item in items {
            let key = match item.cmd {
                Cmd::Plugin(PlugCmd::Generator { index, .. }) => Some((false, index)),
                Cmd::Plugin(PlugCmd::Effect { index, .. }) => Some((true, index)),
                _ => None,
            };
            if let Some((effect, index)) = key {
                if item.enabled {
                    out.push(Choice { item, entry: lookup.get(&(effect, index)).copied(), effect });
                }
            } else {
                collect(&item.children, lookup, out);
            }
        }
    }
    let mut out = Vec::new();
    collect(items, &lookup, &mut out);
    let words: Vec<_> = s.plugin_search.text.split_whitespace().map(str::to_lowercase).collect();
    out.retain(|choice| {
        let entry = choice.entry;
        let text = format!("{} {} {} {}", choice.item.label, entry.map_or("", |entry| entry.vendor.as_str()),
            entry.map_or("", |entry| entry.category), entry.map_or("", |entry| entry.format.as_str())).to_lowercase();
        words.iter().all(|word| text.contains(word))
    });
    let view = s.plugdb.prefs.menu_view;
    out.sort_by_cached_key(|choice| (
        !choice.entry.is_some_and(|entry| entry.favorite), choice.effect,
        choice.group(view).to_lowercase(), choice.item.label.to_lowercase(),
    ));
    out
}

const ROW_H: f32 = 38.0;
const CHROME_H: f32 = 104.0;

fn rows_for(height: f32) -> usize {
    ((height.min(650.0) - CHROME_H - 8.0) / ROW_H).floor().max(1.0) as usize
}

pub fn scale(s: &State) -> f32 {
    crate::views::popup_scale(s.ui_scale)
        .min(s.screen.0 * s.ui_scale / 280.0)
        .min(s.screen.1 * s.ui_scale / 180.0)
        .max(0.1)
}

fn page_rows(s: &State) -> usize {
    let ratio = scale(s) / s.ui_scale.max(0.01);
    rows_for(s.screen.1 / ratio)
}

pub fn scroll(s: &mut State, delta: f32) {
    let Some(items) = active(s) else { return };
    let max = choices(s, items).len().saturating_sub(page_rows(s));
    s.plugin_search.top = ((s.plugin_search.top as f32 + delta).round().max(0.0) as usize).min(max);
}

pub fn click(app: &mut App, hit: Hit) {
    match hit {
        Hit::Area => {}
        Hit::Clear => app.state.plugin_search = Search::default(),
        Hit::Scroll(direction) => scroll(&mut app.state, direction as f32),
        Hit::Pick(PlugCmd::View(view)) => {
            app.state.plugdb.prefs.menu_view = view;
            app.state.plugin_search.top = 0;
            app.state.plugin_search.cursor = 0;
            app.prefs_changed();
        }
        Hit::Pick(cmd) => {
            if let Some(message) = crate::playlist::run(app, Cmd::Plugin(cmd)) {
                app.state.message = Some(message);
            }
            app.state.menu_open = None;
            app.state.menu_sub = None;
            app.state.plugin_search = Search::default();
        }
    }
    app.invalidate();
}

pub fn key(app: &mut App, key: &winit::keyboard::Key, ctrl: bool, alt: bool) -> bool {
    use winit::keyboard::{Key, NamedKey};
    let Some(items) = active(&app.state) else {
        app.state.plugin_search = Search::default();
        return false;
    };
    let count = choices(&app.state, items).len();
    let page = page_rows(&app.state);
    let search = &mut app.state.plugin_search;
    match key {
        Key::Named(NamedKey::Escape) if !search.text.is_empty() => *search = Search::default(),
        Key::Named(NamedKey::Escape) => {
            app.state.playlist.popup = None;
            app.state.menu_open = None;
            app.state.menu_sub = None;
            *search = Search::default();
        }
        Key::Named(NamedKey::F8) if crate::shortcuts::f8(app.shift, ctrl, alt) == Some(crate::shortcuts::F8::PluginPicker) => {
            app.state.playlist.popup = None;
            *search = Search::default();
        }
        Key::Named(NamedKey::Enter) => {
            let command = active(&app.state).and_then(|items| choices(&app.state, items).get(app.state.plugin_search.cursor).map(|choice| choice.item.cmd.clone()));
            if let Some(Cmd::Plugin(command)) = command {
                click(app, Hit::Pick(command));
            }
        }
        Key::Named(named @ (NamedKey::ArrowDown | NamedKey::ArrowUp | NamedKey::PageDown | NamedKey::PageUp | NamedKey::Home | NamedKey::End)) => {
            search.cursor = match named {
                NamedKey::ArrowDown => search.cursor.saturating_add(1),
                NamedKey::ArrowUp => search.cursor.saturating_sub(1),
                NamedKey::PageDown => search.cursor.saturating_add(page),
                NamedKey::PageUp => search.cursor.saturating_sub(page),
                NamedKey::Home => 0,
                _ => count.saturating_sub(1),
            }.min(count.saturating_sub(1));
            search.top = search.top.min(search.cursor);
            if search.cursor >= search.top + page {
                search.top = search.cursor + 1 - page;
            }
        }
        Key::Character(text) if ctrl && !alt && text.eq_ignore_ascii_case("a") => search.selected_text = true,
        Key::Named(NamedKey::Backspace | NamedKey::Delete) => {
            if search.selected_text {
                search.text.clear();
            } else {
                search.text.pop();
            }
            search.selected_text = false;
            search.cursor = 0;
            search.top = 0;
        }
        Key::Character(text) if !ctrl && !alt => {
            if search.selected_text {
                search.text.clear();
            }
            search.text.extend(text.chars().filter(|character| !character.is_control()).take(128usize.saturating_sub(search.text.chars().count())));
            search.selected_text = false;
            search.cursor = 0;
            search.top = 0;
        }
        Key::Named(NamedKey::Space) if !ctrl && !alt => {
            if search.selected_text {
                search.text.clear();
                search.selected_text = false;
            }
            if search.text.chars().count() < 128 {
                search.text.push(' ');
                search.cursor = 0;
                search.top = 0;
            }
        }
        _ => {}
    }
    app.invalidate();
    true
}

pub fn paint(ui: &mut crate::app::Ui, s: &State, items: &[PItem], at: (f32, f32), screen: (f32, f32)) {
    use os_ui::{Font, Rect};
    let choices = choices(s, items);
    let capacity = rows_for(screen.1);
    let shown = choices.len().min(capacity).max(1);
    let width = 450.0f32.min((screen.0 - 8.0).max(1.0));
    let height = (CHROME_H + shown as f32 * ROW_H).min((screen.1 - 8.0).max(1.0));
    let rect = Rect::new(at.0.clamp(4.0f32.min(screen.0 / 2.0), (screen.0 - width - 4.0).max(4.0)),
        at.1.clamp(4.0f32.min(screen.1 / 2.0), (screen.1 - height - 4.0).max(4.0)), width, height);
    let top = s.plugin_search.top.min(choices.len().saturating_sub(capacity));
    let theme = ui.t;
    os_ui::kit::popup(ui.c, theme, rect);
    ui.hit(rect, Action::PluginMenu(Hit::Area), "Search plugins; scroll or use arrow keys");
    ui.c.push_clip(rect.inset(2.0));
    let field = Rect::new(rect.x + 8.0, rect.y + 7.0, rect.w - 16.0, 29.0);
    ui.c.fill_rrect(field, 4.0, theme.menu_pop_head);
    ui.c.stroke_rrect(field, 4.0, 1.0, theme.accent);
    let text = &s.plugin_search.text;
    if s.plugin_search.selected_text && !text.is_empty() {
        ui.c.fill_rect(field.inset(3.0), theme.menu_pop_hi);
    }
    ui.c.text_clip(Font::new(15), field.x + 8.0, field.y + 5.0, (field.w - 38.0).max(0.0),
        theme.menu_pop_text,
        if text.is_empty() { "Search name, vendor or category…" } else { text });
    let clear = Rect::new(field.right() - 25.0, field.y + 2.0, 23.0, 25.0);
    ui.hit(clear, Action::PluginMenu(Hit::Clear), "Clear search");
    ui.c.text_center(Font::new(17), clear, theme.menu_pop_text, "×");
    let list_y = rect.y + 42.0;
    let list = Rect::new(rect.x + 4.0, list_y, rect.w - 17.0, shown as f32 * ROW_H);
    ui.c.push_clip(list);
    let view = s.plugdb.prefs.menu_view;
    for (position, choice) in choices.iter().enumerate().skip(top).take(capacity) {
        let row = Rect::new(list.x, list.y + (position - top) as f32 * ROW_H, list.w, ROW_H);
        let Cmd::Plugin(command) = choice.item.cmd else { continue };
        let hovered = ui.hit(row, Action::PluginMenu(Hit::Pick(command)), choice.item.label.to_string());
        if hovered || position == s.plugin_search.cursor {
            os_ui::kit::row_highlight(ui.c, theme, row);
        }
        ui.c.text_clip(Font::new(15), row.x + 9.0, row.y + 3.0, row.w - 18.0, theme.menu_pop_text, &choice.item.label);
        ui.c.text_clip(Font::new(12), row.x + 9.0, row.y + 22.0, row.w - 18.0,
            theme.menu_pop_text, &choice.detail(view));
    }
    if choices.is_empty() {
        ui.c.text_clip(Font::new(14), list.x + 9.0, list.y + 10.0, list.w - 18.0, theme.menu_pop_text, "No matching plugins");
    }
    ui.c.pop_clip();
    if choices.len() > capacity {
        let track = Rect::new(rect.right() - 10.0, list.y, 5.0, list.h);
        ui.c.fill_rrect(track, 2.0, theme.menu_pop_head);
        let thumb_h = (track.h * capacity as f32 / choices.len() as f32).max(16.0);
        let thumb_y = track.y + (track.h - thumb_h) * top as f32 / (choices.len() - capacity) as f32;
        ui.c.fill_rrect(Rect::new(track.x, thumb_y, track.w, thumb_h), 2.0, theme.menu_pop_dim);
        ui.hit(Rect::new(track.x - 2.0, track.y, 9.0, thumb_y - track.y),
            Action::PluginMenu(Hit::Scroll(-(capacity as i32))), "Previous page");
        ui.hit(Rect::new(track.x - 2.0, thumb_y + thumb_h, 9.0, track.bottom() - thumb_y - thumb_h),
            Action::PluginMenu(Hit::Scroll(capacity as i32)), "Next page");
    }
    let footer_y = list.bottom() + 4.0;
    let noun = if choices.len() == 1 { "plugin" } else { "plugins" };
    let label = format!("{} {noun} · Up/Down · Enter adds · Esc clears/closes", choices.len());
    ui.c.text_clip(Font::new(12), rect.x + 10.0, footer_y, rect.w - 20.0, theme.menu_pop_text, &label);
    let button_y = footer_y + 20.0;
    let button_w = (rect.w - 16.0) / 4.0;
    for (position, (label, value)) in [("Category", MenuView::Categories), ("Vendor", MenuView::Vendor), ("A–Z", MenuView::Simple)].iter().enumerate() {
        let button = Rect::new(rect.x + 8.0 + position as f32 * button_w, button_y, button_w - 4.0, 26.0);
        if view == *value {
            ui.c.fill_rrect(button, 3.0, theme.menu_pop_hi);
        }
        ui.hit(button, Action::PluginMenu(Hit::Pick(PlugCmd::View(*value))), format!("Group by {label}"));
        ui.c.push_clip(button);
        ui.c.text_center(Font::new(12), button, theme.menu_pop_text, label);
        ui.c.pop_clip();
    }
    let manage = Rect::new(rect.x + 8.0 + 3.0 * button_w, button_y, button_w - 4.0, 26.0);
    ui.hit(manage, Action::PluginMenu(Hit::Pick(PlugCmd::Manage)), "Manage plugins and scan folders");
    ui.c.push_clip(manage);
    ui.c.text_center(Font::new(12), manage, theme.menu_pop_text, "Manage…");
    ui.c.pop_clip();
    ui.c.pop_clip();
}

#[cfg(test)]
mod tests {
    use super::*;
    use os_ui::Canvas;
    use winit::keyboard::{Key, KeyCode, NamedKey};

    fn app_with_library(count: usize) -> App {
        let mut app = App::new(None);
        app.plugins = crate::plugins::Plugins::builtins();
        app.state.plugdb.rebuild(&app.plugins);
        let template = app.state.plugdb.lists.fx[0].clone();
        for number in 0..count {
            let mut entry = template.clone();
            entry.index = number + 1;
            entry.name = format!("Example Plugin {number:04}");
            entry.label = entry.name.clone();
            entry.vendor = "Example Audio".into();
            entry.category = if number % 2 == 0 { "EQ" } else { "Dynamics" };
            entry.favorite = number == 42;
            app.state.plugdb.lists.fx.push(entry);
        }
        app
    }

    #[test]
    fn search_matches_words_and_metadata_without_losing_the_destination() {
        let mut app = app_with_library(1000);
        app.state.playlist.popup = Some(crate::mixer::effect_menu(&app.state, 4, 1800.0, 800.0));
        app.key(&Key::Character("LEMON eq".into()), false, false);
        let results = choices(&app.state, active(&app.state).unwrap());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].item.cmd, Cmd::Plugin(PlugCmd::Effect { index: 0, slot: Some(4) }));
        app.state.plugin_search.text = "example audio eq".into();
        assert_eq!(choices(&app.state, active(&app.state).unwrap()).len(), 500);
        app.state.plugin_search.text = "Trio".into();
        assert!(choices(&app.state, active(&app.state).unwrap()).is_empty());
        app.open_plugin_menu(Target::Gen(Some(2)), (0.0, 0.0));
        app.state.plugin_search.text = "trio".into();
        let results = choices(&app.state, active(&app.state).unwrap());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].item.cmd, Cmd::Plugin(PlugCmd::Generator {
            index: app.plugins.index_of(crate::osc3::LEMON_TRIO).unwrap(), after: Some(2),
        }));
    }

    #[test]
    fn search_owns_typing_and_shortcuts_and_releases_held_notes() {
        let mut app = app_with_library(0);
        assert!(app.typing_key(KeyCode::KeyQ, true, false, false));
        app.open_plugin_menu(Target::All, (0.0, 0.0));
        assert!(app.typing_key(KeyCode::KeyQ, false, false, false));
        assert!(!app.typing_key(KeyCode::KeyL, true, false, false));
        app.key(&Key::Character("lemon".into()), false, false);
        app.key(&Key::Named(NamedKey::Space), false, false);
        app.key(&Key::Character("s".into()), true, false);
        assert_eq!(app.state.plugin_search.text, "lemon ");
        assert!(!app.transport.playing());
        assert!(active(&app.state).is_some());
        app.key(&Key::Character("a".into()), true, false);
        app.key(&Key::Character("é".into()), false, false);
        assert_eq!(app.state.plugin_search.text, "é");
        app.key(&Key::Named(NamedKey::Backspace), false, false);
        assert!(app.state.plugin_search.text.is_empty());
        app.key(&Key::Character("not a plugin".into()), false, false);
        app.key(&Key::Named(NamedKey::Enter), false, false);
        assert!(active(&app.state).is_some());
        app.key(&Key::Named(NamedKey::Escape), false, false);
        assert!(app.state.plugin_search.text.is_empty() && active(&app.state).is_some());
        app.key(&Key::Named(NamedKey::Escape), false, false);
        assert!(app.state.playlist.popup.is_none());
        app.open_plugin_menu(Target::All, (0.0, 0.0));
        app.key(&Key::Named(NamedKey::F8), false, false);
        assert!(app.state.playlist.popup.is_none());
    }

    #[test]
    fn large_menu_navigation_scrolls_without_editing_the_project() {
        let mut app = app_with_library(1000);
        app.open_plugin_menu(Target::Fx(Some(7)), (0.0, 0.0));
        let before = app.state.project.clone();
        app.key(&Key::Named(NamedKey::End), false, false);
        assert_eq!(app.state.plugin_search.cursor, 1000);
        assert_eq!(app.state.plugin_search.top + page_rows(&app.state), 1001);
        app.key(&Key::Named(NamedKey::PageUp), false, false);
        assert!(app.state.plugin_search.cursor < 1000);
        app.key(&Key::Named(NamedKey::Home), false, false);
        assert_eq!(app.state.plugin_search.cursor, 0);
        app.wheel(0.0, -2.0, false, false);
        assert_eq!(app.state.plugin_search.top, 6);
        assert_eq!(app.state.project, before);
        app.key(&Key::Character("lemon".into()), false, false);
        assert_eq!((app.state.plugin_search.cursor, app.state.plugin_search.top), (0, 0));
        app.key(&Key::Named(NamedKey::Enter), false, false);
        assert!(app.state.playlist.popup.is_none());
        assert!(app.state.mixer.pending.iter().any(|request| matches!(request,
            crate::mixer::FxReq::Create { slot: 7, index: 0, .. })));
    }

    #[test]
    fn grouping_keeps_favorites_first_and_can_find_plugins_inside_vendor_submenus() {
        let mut app = app_with_library(100);
        app.state.plugdb.prefs.menu_view = MenuView::Vendor;
        app.open_plugin_menu(Target::Fx(Some(3)), (0.0, 0.0));
        app.state.plugin_search.text = "lemon".into();
        assert_eq!(choices(&app.state, active(&app.state).unwrap()).len(), 1);
        click(&mut app, Hit::Clear);
        let results = choices(&app.state, active(&app.state).unwrap());
        assert_eq!(results[0].item.label, "Example Plugin 0042");
        click(&mut app, Hit::Pick(PlugCmd::View(MenuView::Categories)));
        assert!(active(&app.state).is_some());
        assert_eq!(app.state.plugdb.prefs.menu_view, MenuView::Categories);
        app.state.playlist.popup = Some(crate::mixer::slot_menu(&app.state, 3, 0.0, 0.0));
        app.state.playlist.popup.as_mut().unwrap().sub = Some(0);
        app.state.plugin_search.text = "lemon eq".into();
        let results = choices(&app.state, active(&app.state).unwrap());
        assert_eq!(results[0].item.cmd, Cmd::Plugin(PlugCmd::Effect { index: 0, slot: Some(3) }));
    }

    #[test]
    fn searchable_menu_fits_small_and_scaled_windows_and_clicks_the_filtered_result() {
        let sizes = [(640, 480, 1.0, 0), (640, 480, 2.0, 0), (800, 600, 1.0, 3), (640, 480, 1.0, 9), (2544, 1384, 1.0, 0)];
        for (design, (width, height, scale, popups)) in os_theme::design::Design::ALL.into_iter()
            .flat_map(|design| sizes.map(|size| (design, size)))
        {
            crate::prefs::set_choice(&crate::prefs::POPUPS_SCALING, popups);
            let mut app = app_with_library(1000);
            app.set_design(design, false);
            let mut canvas = Canvas::new(width, height, scale).unwrap();
            app.paint(&mut canvas, None);
            app.open_plugin_menu(Target::Fx(Some(0)), (10000.0, 10000.0));
            app.paint(&mut canvas, None);
            let (screen_w, screen_h) = app.state.screen;
            for hit in &app.hits {
                if matches!(hit.action, Action::PluginMenu(_)) {
                    assert!(hit.rect.x >= 0.0 && hit.rect.y >= 0.0 && hit.rect.right() <= screen_w && hit.rect.bottom() <= screen_h,
                        "menu hit {:?} outside {screen_w}×{screen_h}", hit.rect);
                }
            }
            app.open_plugin_menu(Target::Gen(None), (10000.0, 10000.0));
            app.key(&Key::Character("sampler".into()), false, false);
            app.paint(&mut canvas, None);
            let choice = app.hits.iter().find(|hit| matches!(hit.action,
                Action::PluginMenu(Hit::Pick(PlugCmd::Generator { .. })))).unwrap().rect;
            let channels = app.state.project.channels.len();
            app.mouse_move(choice.cx(), choice.cy());
            app.mouse_down();
            app.mouse_up();
            assert_eq!(app.state.project.channels.len(), channels + 1);
            assert!(app.state.playlist.popup.is_none());
        }
    }

    #[test]
    fn searchable_menu_screenshot() {
        let Some(directory) = std::env::var_os("OS_SHOT_DIR") else { return };
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        let mut app = app_with_library(1000);
        app.set_design(os_theme::design::Design::Lemon, false);
        let mut canvas = Canvas::new(1280, 800, 1.0).unwrap();
        app.paint(&mut canvas, None);
        app.open_plugin_menu(Target::Fx(Some(0)), (760.0, 120.0));
        app.paint(&mut canvas, None);
        canvas.pixmap.save_png(directory.join("plugin-search-browse.png")).unwrap();
        app.key(&Key::Character("lemon eq".into()), false, false);
        app.paint(&mut canvas, None);
        canvas.pixmap.save_png(directory.join("plugin-search-filtered.png")).unwrap();
        app.key(&Key::Named(NamedKey::Escape), false, false);
        app.key(&Key::Character("nothing matches this".into()), false, false);
        app.paint(&mut canvas, None);
        canvas.pixmap.save_png(directory.join("plugin-search-empty.png")).unwrap();
    }
}
