use crate::app::{App, State};
use crate::icons::Icon;
use crate::views::browser::{self as view, Item, Row};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum Kind {
    Sample,
    Preset,
    Folder,
}
pub type Favorites = BTreeMap<String, Kind>;
const KEY: &str = "browser_favorites_v1";
pub const WAVE_H: f32 = 76.0;
const POINTS: usize = 512;

pub fn load_favorites() -> Favorites {
    crate::settings_file::get(KEY).and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default()
}

fn file_target(row: &Row) -> Option<(String, Kind)> {
    match &row.item {
        Item::Sample(p) => Some((p.clone(), Kind::Sample)),
        Item::Preset(p) => Some((p.clone(), Kind::Preset)),
        Item::None if row.open.is_some() => row.path.as_ref().map(|p| (p.to_string_lossy().into_owned(), Kind::Folder)),
        _ => None,
    }
}

fn plugin_target(row: &Row) -> Option<(bool, usize)> {
    match row.item {
        Item::Generator(i) => Some((false, i)),
        Item::Effect(i) => Some((true, i)),
        _ => None,
    }
}

pub fn favoritable(row: &Row) -> bool {
    file_target(row).is_some() || plugin_target(row).is_some()
}

pub fn is_favorite(s: &State, row: &Row) -> bool {
    if let Some((path, kind)) = file_target(row) {
        return s.browser_favorites.get(&path) == Some(&kind);
    }
    let Some((effect, index)) = plugin_target(row) else {
        return false;
    };
    let list = if effect { &s.plugdb.lists.fx } else { &s.plugdb.lists.gens };
    list.iter().find(|e| e.index == index).is_some_and(|e| s.plugdb.prefs.is_favorite(&e.uid))
}

pub fn toggle(app: &mut App, row: Row) {
    if let Some((effect, index)) = plugin_target(&row) {
        crate::plugin_menu::click(app, crate::plugin_menu::Hit::ToggleFavorite(effect, index));
    } else if let Some((path, kind)) = file_target(&row) {
        if app.state.browser_favorites.remove(&path).is_none() {
            app.state.browser_favorites.insert(path, kind);
        }
        let value = serde_json::to_string(&app.state.browser_favorites).unwrap();
        crate::settings_file::write(KEY, &value);
        app.state.message = Some(if is_favorite(&app.state, &row) {
            format!("Added '{}' to Favorites", row.label)
        } else {
            format!("Removed '{}' from Favorites", row.label)
        });
        app.state.browser_scroll = app.state.browser_scroll.min(view::max_scroll(&app.state));
        app.invalidate();
    }
}

pub fn favorite_rows(s: &State) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut entries: Vec<_> = s.browser_favorites.iter().collect();
    entries.sort_by_cached_key(|(p, _)| Path::new(p).file_name().unwrap_or_default().to_string_lossy().to_lowercase());
    for (path, kind) in entries {
        let p = PathBuf::from(path);
        let key = format!("3:@{path}");
        let open = *kind == Kind::Folder && s.browser_open.contains(&key);
        let (item, icon) = match kind {
            Kind::Sample => (Item::Sample(path.clone()), Icon::FileAudio),
            Kind::Preset => (Item::Preset(path.clone()), Icon::FilePreset),
            Kind::Folder => (Item::None, if open { Icon::FolderOpen } else { Icon::Folder }),
        };
        let label =
            if *kind == Kind::Folder { view::root_label(s, &p) } else { p.file_stem().unwrap_or_default().to_string_lossy().into_owned() };
        rows.push(Row {
            key: key.clone(),
            depth: 0,
            label,
            icon,
            cat: 254,
            open: (*kind == Kind::Folder).then_some(open),
            bright: false,
            item,
            path: Some(p.clone()),
        });
        if *kind == Kind::Folder && (open || !s.browser_search.is_empty()) {
            view::disk_rows(s, &p, &key, 1, &mut rows);
        }
    }
    for (effect, list) in [(false, &s.plugdb.lists.gens), (true, &s.plugdb.lists.fx)] {
        for e in list.iter().filter(|e| s.plugdb.prefs.is_favorite(&e.uid)) {
            rows.push(Row {
                key: format!("3:plugin:{}:{}", effect, e.uid),
                depth: 0,
                label: e.label.clone(),
                icon: if effect { Icon::Speaker } else { Icon::Plug },
                cat: 1,
                open: None,
                bright: false,
                item: if effect { Item::Effect(e.index) } else { Item::Generator(e.index) },
                path: None,
            });
        }
    }
    rows
}

#[derive(Debug, Clone)]
pub struct Waveform {
    pub peaks: Vec<(f32, f32)>,
    pub rate: u32,
    pub frames: usize,
}
#[derive(Debug, Clone)]
pub struct Preview {
    pub path: String,
    pub waveform: Option<Waveform>,
    pub error: Option<String>,
}

type Stamp = Option<(u64, Option<std::time::SystemTime>)>;
fn stamp(path: &Path) -> Stamp {
    std::fs::metadata(path).ok().map(|m| (m.len(), m.modified().ok()))
}
#[derive(Clone, PartialEq)]
struct Key {
    path: String,
    stamp: Stamp,
}
struct Request {
    key: Key,
    cancel: Arc<AtomicBool>,
}
struct Shared {
    pending: Option<Request>,
    result: Option<(Key, Result<Waveform, String>)>,
    stop: bool,
}
pub struct Media {
    shared: Arc<(Mutex<Shared>, Condvar)>,
    current: Option<(Key, Arc<AtomicBool>)>,
    cache: VecDeque<(Key, Result<Waveform, String>)>,
}

fn decode(path: &str, cancel: &AtomicBool) -> Result<Waveform, String> {
    let resolved = crate::plugins::resolve_sample(path);
    let path = Path::new(&resolved);
    if let Some(layout) = lemon_sampler::stream::Layout::of(path) {
        return layout
            .waveform(path, POINTS, cancel)
            .map(|peaks| Waveform { peaks, rate: layout.rate, frames: layout.frames })
            .ok_or("Waveform cancelled".into());
    }
    let sample = os_engine::sample::load_bounded(path, 32 * 1024 * 1024, 1_000_000, cancel)
        .map_err(|_| "Waveform unavailable for this file; long compressed files are not decoded for browsing".to_string())?;
    let per = sample.frames().div_ceil(POINTS).max(1);
    let peaks = sample
        .left
        .chunks(per)
        .zip(sample.right.chunks(per))
        .map(|(l, r)| {
            l.iter()
                .zip(r)
                .map(|(a, b)| (a + b) * 0.5)
                .filter(|v| v.is_finite())
                .fold((0.0f32, 0.0f32), |(lo, hi), v| (lo.min(v), hi.max(v)))
        })
        .collect();
    Ok(Waveform { peaks, rate: sample.rate, frames: sample.frames() })
}

impl Media {
    fn new() -> Result<Self, String> {
        let shared = Arc::new((Mutex::new(Shared { pending: None, result: None, stop: false }), Condvar::new()));
        let worker = shared.clone();
        std::thread::Builder::new()
            .name("browser-waveform".into())
            .spawn(move || {
                loop {
                    let request = {
                        let (mutex, wake) = &*worker;
                        let mut s = mutex.lock().unwrap_or_else(|e| e.into_inner());
                        while s.pending.is_none() && !s.stop {
                            s = wake.wait(s).unwrap_or_else(|e| e.into_inner());
                        }
                        if s.stop {
                            return;
                        }
                        s.pending.take().unwrap()
                    };
                    let result = decode(&request.key.path, &request.cancel);
                    if !request.cancel.load(Ordering::Relaxed) {
                        let mut s = worker.0.lock().unwrap_or_else(|e| e.into_inner());
                        s.result = Some((request.key, result));
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { shared, current: None, cache: VecDeque::new() })
    }

    fn select(&mut self, path: Option<String>, preview: &mut Option<Preview>) {
        let key = path.map(|path| {
            let resolved = crate::plugins::resolve_sample(&path);
            Key { stamp: stamp(Path::new(&resolved)), path }
        });
        if self.current.as_ref().map(|c| &c.0) == key.as_ref() {
            return;
        }
        if let Some((_, cancel)) = self.current.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        *preview = key.as_ref().map(|key| Preview { path: key.path.clone(), waveform: None, error: None });
        let mut s = self.shared.0.lock().unwrap_or_else(|e| e.into_inner());
        s.pending = None;
        let Some(key) = key else {
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        self.current = Some((key.clone(), cancel.clone()));
        if let Some((_, cached)) = self.cache.iter().find(|(k, _)| k == &key) {
            put(preview, &key, cached.clone());
        } else {
            s.pending = Some(Request { key, cancel });
            self.shared.1.notify_one();
        }
    }

    fn poll(&mut self, preview: &mut Option<Preview>) -> bool {
        let result = self.shared.0.lock().unwrap_or_else(|e| e.into_inner()).result.take();
        let Some((key, result)) = result else {
            return false;
        };
        if self.current.as_ref().map(|c| &c.0) != Some(&key) {
            return false;
        }
        let resolved = crate::plugins::resolve_sample(&key.path);
        if stamp(Path::new(&resolved)) != key.stamp {
            self.current = None;
            return true;
        }
        put(preview, &key, result.clone());
        self.cache.retain(|(k, _)| k != &key);
        self.cache.push_front((key, result));
        self.cache.truncate(8);
        true
    }
}

fn put(preview: &mut Option<Preview>, key: &Key, result: Result<Waveform, String>) {
    let (waveform, error) = match result {
        Ok(w) => (Some(w), None),
        Err(e) => (None, Some(e)),
    };
    *preview = Some(Preview { path: key.path.clone(), waveform, error });
}

impl Drop for Media {
    fn drop(&mut self) {
        if let Some((_, cancel)) = &self.current {
            cancel.store(true, Ordering::Relaxed);
        }
        let mut s = self.shared.0.lock().unwrap_or_else(|e| e.into_inner());
        s.stop = true;
        s.pending = None;
        self.shared.1.notify_one();
    }
}

pub fn selected_audio(s: &State) -> Option<String> {
    if !s.show_browser {
        return None;
    }
    let key = s.browser_sel.as_deref()?;
    let row = view::rows(s).into_iter().find(|r| r.key == key)?;
    match row.item {
        Item::Sample(path) => Some(path),
        _ => None,
    }
}

pub fn tick(app: &mut App) {
    let path = selected_audio(&app.state);
    if path.is_some() && app.browser_media.is_none() {
        match Media::new() {
            Ok(media) => app.browser_media = Some(media),
            Err(e) => {
                app.state.message = Some(format!("Could not start waveform preview: {e}"));
                return;
            }
        }
    }
    if let Some(media) = &mut app.browser_media {
        let changed = media.current.as_ref().map(|c| &c.0.path) != path.as_ref();
        media.select(path, &mut app.state.browser_wave);
        if media.poll(&mut app.state.browser_wave) || changed {
            app.invalidate();
        }
    }
}

pub fn pending(s: &State) -> bool {
    s.browser_wave.as_ref().is_some_and(|p| p.waveform.is_none() && p.error.is_none())
}

pub fn paint_wave(ui: &mut crate::app::Ui, s: &State, r: os_ui::Rect) {
    use os_ui::{Font, Rect};
    let t = ui.t;
    ui.c.fill_rect(r, t.browser_tabs);
    let area = Rect::new(r.x + 7.0, r.y + 24.0, (r.w - 14.0).max(1.0), (r.h - 29.0).max(1.0));
    ui.hit(r, crate::app::Action::BrowserPreview, "Click to preview the sample from the start");
    ui.c.fill_rrect(area, 3.0, t.sunken_dark);
    ui.c.icon(&os_ui::tabler::PLAYER_PLAY, Rect::new(r.right() - 24.0, r.y + 4.0, 16.0, 16.0), t.accent);
    let selected = selected_audio(s);
    let preview = s.browser_wave.as_ref().filter(|p| Some(&p.path) == selected.as_ref());
    let wave = preview.and_then(|p| p.waveform.as_ref());
    let header = wave
        .map(|w| format!("{:.2}s  ·  {:.1} kHz", w.frames as f64 / f64::from(w.rate.max(1)), w.rate as f32 / 1000.0))
        .unwrap_or_else(|| "Sample preview".into());
    ui.c.text_clip(Font::new(12), r.x + 8.0, r.y + 5.0, (r.w - 36.0).max(0.0), t.text_dim, &header);
    if let Some(wave) = wave {
        let width = area.w.floor().max(1.0) as usize;
        let scale = wave.peaks.iter().fold(0.05f32, |max, (lo, hi)| max.max(lo.abs()).max(hi.abs()));
        let mid = area.cy();
        ui.c.fill_rect(Rect::new(area.x, mid, area.w, 1.0), t.text_faint.shade(-0.2));
        for x in 0..width {
            let from = x * wave.peaks.len() / width;
            let end = ((x + 1) * wave.peaks.len()).div_ceil(width).min(wave.peaks.len());
            let (lo, hi) =
                wave.peaks.get(from..end).unwrap_or(&[]).iter().fold((0.0f32, 0.0f32), |(lo, hi), (a, b)| (lo.min(*a), hi.max(*b)));
            let top = mid - (hi / scale).clamp(0.0, 1.0) * (area.h * 0.45);
            let bottom = mid - (lo / scale).clamp(-1.0, 0.0) * (area.h * 0.45);
            ui.c.fill_rect(Rect::new(area.x + x as f32, top, 1.0, (bottom - top).max(1.0)), t.accent);
        }
    } else {
        let label = preview.and_then(|p| p.error.as_deref()).unwrap_or("Loading waveform…");
        ui.c.text_clip(Font::new(12), area.x + 6.0, area.y + 13.0, (area.w - 12.0).max(0.0), t.text_dim, label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    struct Fixture(PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn fixture() -> (App, Fixture, String) {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let directory =
            std::env::temp_dir().join(format!("browser-assets-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("808 - DREAMFLASHER.wav").to_string_lossy().into_owned();
        write_audio(&path, 44100);
        let mut app = App::new(None);
        app.plugins = crate::plugins::Plugins::builtins();
        app.state.plugdb.rebuild(&app.plugins);
        app.state.browser_folders = vec![directory.clone()];
        app.state.browser_open.insert(view::disk_key(&directory));
        let row = view::rows(&app.state).into_iter().find(|r| matches!(&r.item, Item::Sample(p) if p == &path)).unwrap();
        app.state.browser_sel = Some(row.key);
        (app, Fixture(directory), path)
    }

    fn write_audio(path: &str, frames: usize) {
        let samples: Vec<_> = (0..frames)
            .flat_map(|i| {
                let envelope = (-6.0 * i as f32 / frames as f32).exp();
                let value = (i as f32 * 0.07).sin() * envelope;
                [value, value]
            })
            .collect();
        os_engine::wav::write_f32_stereo(&mut std::fs::File::create(path).unwrap(), 44100, &samples).unwrap();
    }

    fn wait(app: &mut App) {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            tick(app);
            if app.state.browser_wave.as_ref().is_some_and(|p| p.waveform.is_some() || p.error.is_some()) {
                break;
            }
            assert!(std::time::Instant::now() < until, "waveform completes");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    #[test]
    fn stars_persist_and_favorites_keep_search_and_folder_browsing() {
        let (mut app, fixture, path) = fixture();
        let selected = app.state.browser_sel.clone();
        let before = app.state.project.clone();
        let row = view::rows(&app.state).into_iter().find(|r| matches!(&r.item, Item::Sample(p) if p == &path)).unwrap();
        toggle(&mut app, row.clone());
        assert!(is_favorite(&app.state, &row));
        assert_eq!(app.state.project, before);
        assert_eq!(app.state.browser_sel, selected);
        assert!(app.drag.is_none());
        let folder = view::rows(&app.state).into_iter().find(|r| r.path.as_ref() == Some(&fixture.0)).unwrap();
        toggle(&mut app, folder);
        let mut reopened = App::new(None);
        assert_eq!(reopened.state.browser_favorites, app.state.browser_favorites);
        reopened.state.browser_tab = 3;
        reopened.state.browser_search = "dream".into();
        assert!(view::rows(&reopened.state).iter().any(|r| matches!(&r.item, Item::Sample(p) if p == &path)));
        toggle(&mut app, row);
        assert!(!load_favorites().contains_key(&path));
    }

    #[test]
    fn plugin_browser_stars_use_the_existing_plugin_favorites() {
        let (mut app, _fixture, _) = fixture();
        let entry = app.state.plugdb.lists.fx[0].clone();
        app.state.browser_tab = 2;
        app.state.browser_search = entry.label.clone();
        let row = view::rows(&app.state).into_iter().find(|r| r.item == Item::Effect(entry.index)).unwrap();
        toggle(&mut app, row.clone());
        assert!(app.state.plugdb.prefs.is_favorite(&entry.uid));
        assert!(is_favorite(&app.state, &row));
        app.state.browser_tab = 3;
        assert!(view::rows(&app.state).iter().any(|r| r.item == Item::Effect(entry.index)));
        toggle(&mut app, row);
        assert!(!app.state.plugdb.prefs.is_favorite(&entry.uid));
    }

    #[test]
    fn waveform_tracks_the_latest_selection_and_invalidates_changed_files() {
        let (mut app, fixture, path) = fixture();
        tick(&mut app);
        let second = fixture.0.join("second.wav").to_string_lossy().into_owned();
        write_audio(&second, 22050);
        app.state.browser_dirs.borrow_mut().clear();
        let row = view::rows(&app.state).into_iter().find(|r| matches!(&r.item, Item::Sample(p) if p == &second)).unwrap();
        app.state.browser_sel = Some(row.key);
        wait(&mut app);
        let preview = app.state.browser_wave.as_ref().unwrap();
        assert_eq!(preview.path, second);
        assert_eq!(preview.waveform.as_ref().unwrap().frames, 22050);
        assert!(preview.waveform.as_ref().unwrap().peaks.len() <= POINTS);
        write_audio(&second, 11025);
        tick(&mut app);
        wait(&mut app);
        assert_eq!(app.state.browser_wave.as_ref().unwrap().waveform.as_ref().unwrap().frames, 11025);
        assert!(app.browser_media.as_ref().unwrap().cache.len() <= 8);
        assert_ne!(path, second);
        app.state.show_browser = false;
        tick(&mut app);
        assert!(app.state.browser_wave.is_none());
    }

    #[test]
    fn browser_stars_and_waveform_render_without_starting_audio() {
        let (mut app, _fixture, path) = fixture();
        wait(&mut app);
        let mut canvas = os_ui::Canvas::new(1280, 800, 1.0).expect("offscreen browser canvas");
        app.state.screen = (1280.0, 800.0);
        app.state.browser_w = 340.0;
        app.paint(&mut canvas, None);
        let rows = view::rows(&app.state);
        let index = rows.iter().position(|r| matches!(&r.item, Item::Sample(p) if p == &path)).unwrap();
        let y = view::list_top(&app.state) + view::ROW_H * index as f32 + view::ROW_H * 0.5;
        let x = crate::views::browser_w(&app.state) - 8.0 - 14.0;
        assert_eq!(app.top_hit(x, y).map(|h| &h.action), Some(&crate::app::Action::BrowserFavorite(index)));
        assert!(app.browser_act(&crate::app::Action::BrowserFavorite(index)));
        app.paint(&mut canvas, None);
        if let Some(directory) = std::env::var_os("LEMON_BROWSER_CAPTURE") {
            let directory = PathBuf::from(directory);
            canvas.pixmap.save_png(directory.join("browser-sample-preview.png")).unwrap();
            app.state.browser_tab = 3;
            app.paint(&mut canvas, None);
            canvas.pixmap.save_png(directory.join("browser-favorites.png")).unwrap();
        }
        assert!(!app.transport.playing());
        assert!(app.drag.is_none());
    }
}
