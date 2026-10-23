use std::collections::{BTreeMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use os_model::{Clip, ClipSource, Note, Pattern, Project};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::app::{App, State};

#[cfg(unix)]
#[path = "assistant_io.rs"]
pub mod io;

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    project: Project,
    tempo: f64,
    pattern: u16,
    song_mode: bool,
    channel: usize,
    mixer: u16,
}

impl Snapshot {
    pub fn now(state: &State) -> Self {
        Self {
            project: state.project.clone(),
            tempo: state.tempo,
            pattern: state.pattern,
            song_mode: state.song_mode,
            channel: state.selected_channel,
            mixer: state.selected_mixer,
        }
    }

    pub fn rescale(&mut self, ppq: u16) {
        self.project.set_ppq(ppq);
    }

    pub fn put(self, state: &mut State) {
        state.project = self.project;
        state.tempo = self.tempo;
        state.pattern = self.pattern;
        state.song_mode = self.song_mode;
        state.selected_channel = self.channel.min(state.project.channels.len().saturating_sub(1));
        state.selected_mixer = self.mixer;
        state.assistant_restore = true;
        state.piano.clear_selection();
    }
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Edit {
    SetTempo { bpm: f64 },
    CreatePattern { number: u16, name: String, bars: u16 },
    AddNotes { pattern: u16, notes: Vec<InputNote> },
    PlacePattern { pattern: u16, start_bar: u16, track: u16 },
    AddChannel { name: String, instrument: String, asset: Option<String>, preset: Option<String>, volume: f64, pan: f64 },
    SetChannelLevel { channel: usize, volume: f64, pan: f64 },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InputNote {
    channel: u16,
    key: u8,
    beat: f64,
    duration: f64,
    velocity: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Apply {
    session_id: String,
    expected_revision: String,
    operation_id: String,
    operations: Vec<Edit>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Write {
    session_id: String,
    expected_revision: String,
    operation_id: String,
    filename: Option<String>,
    action: Option<String>,
    pattern: Option<u16>,
}

pub struct Controller {
    pub root: PathBuf,
    session_id: String,
    receipts: VecDeque<(String, String, Value)>,
    job: Option<Job>,
    serial: u64,
    pub headless: bool,
}

struct Job {
    id: u64,
    temporary: PathBuf,
    destination: PathBuf,
    status: Value,
    running: bool,
}

pub fn capture(app: &mut App) {
    app.state.project = project_state(app);
}

fn project_state(app: &App) -> Project {
    let mut project = app.state.project.clone();
    crate::osc3::store(&mut project, &app.plugins);
    for track in &mut project.mixer {
        for (index, slot) in track.slots.iter_mut().enumerate() {
            if let Some(slot) = slot.as_mut().filter(|slot| slot.name == crate::plugins::EQ_FILE_NAME) {
                if let Some(bytes) = app.plugins.eq_file_state(crate::plugins::fx_key(track.index, index as u8)) {
                    slot.state = bytes;
                }
            }
        }
    }
    for row in &app.state.hosted_rows {
        if let Some(channel) = project.channels.get_mut(*row).filter(|channel| channel.kind.is_sampler()) {
            if let Some(state) = app.plugins.state(*row) {
                crate::plugins::store_sampler(channel, &state);
            }
        }
    }
    crate::project_plugins::store_hosted(&mut project, &app.plugins);
    project
}

impl Controller {
    pub fn new(root: PathBuf, session_id: String, headless: bool) -> Self {
        Self { root, session_id, receipts: VecDeque::new(), job: None, serial: 0, headless }
    }

    fn revision(&self, app: &App) -> String {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        self.session_id.hash(&mut hash);
        format!("{:?}", app.state.project).hash(&mut hash);
        app.state.tempo.to_bits().hash(&mut hash);
        app.state.pattern.hash(&mut hash);
        app.state.path.hash(&mut hash);
        format!("{:016x}", hash.finish())
    }

    fn context(&self, app: &App) -> Value {
        let state = &app.state;
        let project = &state.project;
        json!({
            "session_id": self.session_id, "revision": self.revision(app), "workspace": self.root,
            "headless": self.headless, "connected": true, "playing": app.transport.playing(),
            "bpm": state.tempo, "ppq": project.ppq, "beats_per_bar": f64::from(project.bar_ticks()) / f64::from(project.ppq),
            "current_pattern": state.pattern, "song_mode": state.song_mode, "file": state.path,
            "busy": app.loading(), "undo_position": state.history.at(),
            "channels": project.channels.iter().enumerate().map(|(row, channel)| json!({"row":row,"name":channel.name,"instrument":crate::project_plugins::plugin_title(channel),"mixer_track":channel.mixer_track,"volume":channel.volume,"pan":channel.pan})).collect::<Vec<_>>(),
            "patterns": project.patterns.iter().map(|pattern| json!({"number":pattern.number,"name":pattern.name,"length_ticks":pattern.length,"notes":pattern.notes.iter().map(|note|json!({"channel":note.channel,"key":note.key,"beat":f64::from(note.position)/f64::from(project.ppq),"duration":f64::from(note.length)/f64::from(project.ppq),"velocity":note.velocity})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
            "clips": project.arrangement().map(|arrangement| arrangement.clips.iter().filter_map(|clip| match clip.source { ClipSource::Pattern(number)=>Some(json!({"pattern":number,"track":clip.track,"position_ticks":clip.position,"length_ticks":clip.length})), _=>None }).collect::<Vec<_>>()).unwrap_or_default(),
            "message": state.message,
        })
    }

    fn idle(&self, app: &App) -> Result<(), String> {
        if app.loading() || app.transport.playing() || app.state.rec.armed || app.state.rec.waiting || app.state.rec.count.is_some() || app.modal_open() {
            return Err("Stop playback/recording and finish dialogs or rendering before editing.".into());
        }
        if app.state.path.as_ref().is_some_and(|path| !path.starts_with(&self.root)) {
            return Err("This project is outside the opted-in workspace; open a scratch copy first.".into());
        }
        Ok(())
    }

    fn guard(&self, app: &App, session: &str, revision: &str) -> Result<(), String> {
        if session != self.session_id {
            return Err("Wrong session. Read lemon_get_session again.".into());
        }
        if revision != self.revision(app) {
            return Err("Stale revision. Read lemon_get_session and review changes before retrying.".into());
        }
        Ok(())
    }

    fn output(&self, filename: &str, extension: &str) -> Result<PathBuf, String> {
        let path = Path::new(filename);
        if filename.is_empty()
            || filename.len() > 120
            || path.components().count() != 1
            || path.file_name().and_then(|name| name.to_str()) != Some(filename)
            || !filename.chars().all(|character| character.is_ascii_alphanumeric() || " -_.".contains(character))
            || path.extension().and_then(|extension| extension.to_str()) != Some(extension)
        {
            return Err(format!("Use a simple new .{extension} filename, not a path."));
        }
        let destination = self.root.join("exports").join(filename);
        if destination.symlink_metadata().is_ok() {
            return Err("Output exists. Choose another filename; overwriting is disabled.".into());
        }
        Ok(destination)
    }

    fn asset(&self, asset: &str) -> Result<PathBuf, String> {
        let path = self.root.join("assets").join(asset).canonicalize().map_err(|error| format!("Asset: {error}"))?;
        if !path.starts_with(self.root.join("assets"))
            || path.extension().and_then(|extension| extension.to_str()) != Some("wav")
            || std::fs::metadata(&path).map_err(|error| error.to_string())?.len() > 32 * 1024 * 1024
        {
            return Err("Only WAV assets inside this workspace's assets folder, at most 32 MiB, are accepted.".into());
        }
        os_engine::sample::load(&path).map_err(|error| format!("Cannot decode asset: {error}"))?;
        Ok(path)
    }

    fn checkpoint(&mut self, project: &Project) -> Result<(), String> {
        self.serial += 1;
        let file = os_flp::project::create(project).map_err(|error| error.to_string())?;
        let bytes = file.to_bytes().map_err(|error| error.to_string())?;
        let path = self.root.join("checkpoints").join(format!("{}-{}.flp", self.session_id, self.serial));
        write_new(&path, &bytes)
    }

    fn apply(&mut self, app: &mut App, input: Apply) -> Result<Value, String> {
        self.guard(app, &input.session_id, &input.expected_revision)?;
        self.idle(app)?;
        if input.operations.is_empty() || input.operations.len() > 128 {
            return Err("Supply 1–128 operations.".into());
        }
        let mut before = Snapshot::now(&app.state);
        before.project = project_state(app);
        let mut project = before.project.clone();
        let bar = project.bar_ticks();
        for operation in input.operations {
            match operation {
                Edit::SetTempo { bpm } => {
                    if !bpm.is_finite() || !(40.0..=240.0).contains(&bpm) {
                        return Err("Tempo must be 40–240 BPM.".into());
                    }
                    project.tempo = Some(bpm);
                }
                Edit::CreatePattern { number, name, bars } => {
                    if number == 0 || number > 999 || project.pattern(number).is_some() || !(1..=64).contains(&bars) {
                        return Err("Pattern must be new, numbered 1–999, and 1–64 bars long.".into());
                    }
                    label(&name)?;
                    project.patterns.push(Pattern { number, name: Some(name), length: Some(u32::from(bars) * bar), ..Default::default() });
                }
                Edit::AddNotes { pattern, notes } => {
                    if notes.len() > 8192 {
                        return Err("Too many notes in one operation.".into());
                    }
                    let channels = project.channels.len();
                    let ppq = f64::from(project.ppq);
                    let target = project.patterns.iter_mut().find(|candidate| candidate.number == pattern).ok_or("Pattern not found")?;
                    let length = target.length.ok_or("Set an explicit pattern length before adding notes")?;
                    for note in notes {
                        if usize::from(note.channel) >= channels
                            || note.key > 127
                            || note.velocity == 0
                            || note.velocity > 127
                            || !note.beat.is_finite()
                            || !note.duration.is_finite()
                            || note.beat < 0.0
                            || note.duration <= 0.0
                            || (note.beat + note.duration) * ppq > f64::from(length) + 0.01
                        {
                            return Err("Invalid note or note outside its pattern.".into());
                        }
                        target.notes.push(Note {
                            position: (note.beat * ppq).round() as u32,
                            length: (note.duration * ppq).round().max(1.0) as u32,
                            key: u16::from(note.key),
                            velocity: note.velocity,
                            ..Note::step(note.channel, 0)
                        });
                    }
                    target.notes.sort_by_key(|note| (note.position, note.channel, note.key));
                }
                Edit::PlacePattern { pattern, start_bar, track } => {
                    if start_bar >= 64 || track >= 64 {
                        return Err("Clip bar/track must be 0–63 (zero-based).".into());
                    }
                    let length = project.pattern(pattern).and_then(|pattern| pattern.length).ok_or("Pattern needs explicit length")?;
                    if u32::from(start_bar) * bar + length > 64 * bar {
                        return Err("Arrangement exceeds 64 bars.".into());
                    }
                    if project.arrangements.is_empty() {
                        project.arrangements.push(os_flp::project::default_arrangement());
                    }
                    let arrangement = project.arrangement_mut().ok_or("No arrangement")?;
                    arrangement.clips.push(Clip::new(u32::from(start_bar) * bar, length, track, ClipSource::Pattern(pattern)));
                    arrangement.clips.sort_by_key(|clip| (clip.position, clip.track));
                }
                Edit::AddChannel { name, instrument, asset, preset, volume, pan } => {
                    label(&name)?;
                    if !volume.is_finite() || !(0.0..=1.0).contains(&volume) || !pan.is_finite() || !(-1.0..=1.0).contains(&pan) {
                        return Err("Channel volume/pan out of range.".into());
                    }
                    let row = match instrument.as_str() {
                        "sampler" => {
                            let path = self.asset(asset.as_deref().ok_or("Sampler needs a workspace WAV asset")?)?;
                            let row = crate::app::add_channel(&mut project);
                            project.channels[row].sample_path = Some(path.to_string_lossy().into_owned());
                            let envelope = &mut project.channels[row].envelopes[1];
                            envelope.enabled = true;
                            envelope.delay = 100;
                            envelope.attack = 700;
                            envelope.hold = 100;
                            envelope.decay = 12000;
                            envelope.sustain = 96;
                            envelope.release = 6000;
                            row
                        }
                        "ray" => {
                            if asset.is_some() {
                                return Err("Ray does not take a sample asset.".into());
                            }
                            let values = lemon_ray::params::PRESETS
                                .iter()
                                .find(|(name, _)| Some(*name) == preset.as_deref())
                                .ok_or("Use Arc zap, Soft ray or Glass ping")?
                                .1;
                            let parameters: BTreeMap<_, _> = lemon_ray::params::IDS.iter().zip(values).map(|(id, value)| (*id, json!({"f32":value}))).collect();
                            let state = serde_json::to_vec(&json!({"version":"0.0.1","params":parameters,"fields":{}})).map_err(|error| error.to_string())?;
                            let mut bytes = (state.len() as u64).to_le_bytes().to_vec();
                            bytes.extend_from_slice(&state);
                            let descriptor = crate::plugins::builtin()
                                .into_iter()
                                .find(|plugin| plugin.desc.id == lemon_ray::CLAP_ID)
                                .ok_or("Built-in Ray unavailable")?
                                .desc;
                            let reference = crate::project_plugins::plugin_ref(&descriptor, bytes);
                            let row = crate::app::add_plugin_channel(&mut project, "Lemon Ray");
                            project.channels[row].plugin = Some(os_flp::wrapper::WRAPPER.into());
                            project.channels[row].plugin_state = Some(os_flp::wrapper::encode(None, &reference));
                            row
                        }
                        "wavetable" => {
                            if asset.is_some() {
                                return Err("Wavetable does not take a sample asset.".into());
                            }
                            let descriptor = crate::plugins::builtin()
                                .into_iter()
                                .find(|plugin| plugin.desc.id == lemon_wavetable::CLAP_ID)
                                .ok_or("Built-in Wavetable unavailable")?
                                .desc;
                            let reference = crate::project_plugins::plugin_ref(&descriptor, Vec::new());
                            let row = crate::app::add_plugin_channel(&mut project, "Lemon Wavetable");
                            project.channels[row].plugin = Some(os_flp::wrapper::WRAPPER.into());
                            project.channels[row].plugin_state = Some(os_flp::wrapper::encode(None, &reference));
                            row
                        }
                        _ => return Err("Supported instruments are sampler, ray, and wavetable.".into()),
                    };
                    let channel = &mut project.channels[row];
                    channel.name = Some(name);
                    channel.volume = volume;
                    channel.pan = pan;
                }
                Edit::SetChannelLevel { channel, volume, pan } => {
                    if !volume.is_finite() || !(0.0..=1.0).contains(&volume) || !pan.is_finite() || !(-1.0..=1.0).contains(&pan) {
                        return Err("Channel volume/pan out of range.".into());
                    }
                    let channel = project.channels.get_mut(channel).ok_or("Channel not found")?;
                    channel.volume = volume;
                    channel.pan = pan;
                }
            }
        }
        if project.channels.len() > 64
            || project.patterns.len() > 128
            || project.patterns.iter().map(|pattern| pattern.notes.len()).sum::<usize>() > 16384
            || project.arrangements.iter().map(|arrangement| arrangement.clips.len()).sum::<usize>() > 1024
        {
            return Err("First-version session size limit exceeded.".into());
        }
        self.checkpoint(&before.project)?;
        crate::history::record(&mut app.state, "Assistant edit", crate::history::Snap::Assistant(Box::new(before)));
        app.reset_hosted();
        app.state.tempo = project.tempo.unwrap_or(app.state.tempo);
        app.state.project = project;
        app.state.piano.dirty = true;
        app.state.message = Some("Assistant edit applied — one Undo restores the batch".into());
        app.plugin_tick();
        app.invalidate();
        Ok(self.context(app))
    }

    pub fn poll_render(&mut self, app: &mut App) {
        let Some(job) = self.job.as_mut().filter(|job| job.running) else { return };
        match app.transport.export_status() {
            crate::transport::Export::Running(progress) => {
                app.export = Some(("Assistant preview".into(), progress));
                job.status = json!({"job_id":job.id,"state":"rendering","progress":progress});
            }
            crate::transport::Export::Done(result) => {
                app.export = None;
                app.assistant_export = false;
                app.plugins.set_offline(false);
                app.midi_after_export();
                job.running = false;
                job.status = match result {
                    Ok((seconds, _files)) => match std::fs::hard_link(&job.temporary, &job.destination) {
                        Ok(()) => json!({"job_id":job.id,"state":"done","seconds":seconds,"path":job.destination}),
                        Err(error) => json!({"job_id":job.id,"state":"error","error":error.to_string()}),
                    },
                    Err(error) => json!({"job_id":job.id,"state":"error","error":error}),
                };
                let _ = std::fs::remove_file(&job.temporary);
                app.state.message = Some(format!("Assistant preview: {}", job.status["state"]));
                app.invalidate();
            }
            _ => {}
        }
    }

    pub fn handle(&mut self, app: &mut App, method: &str, arguments: Value) -> Result<Value, String> {
        self.poll_render(app);
        if method == "lemon_get_session" {
            return Ok(self.context(app));
        }
        if method == "lemon_render_status" {
            return Ok(self.job.as_ref().map_or(json!({"state":"idle"}), |job| job.status.clone()));
        }
        if method == "lemon_list_plugins" {
            return Ok(
                json!({"instruments":app.plugins.available.iter().map(|plugin|json!({"name":plugin.desc.name,"id":plugin.desc.id,"format":plugin.desc.format.as_str()})).collect::<Vec<_>>(),"effects":app.plugins.effects.iter().map(|plugin|&plugin.desc.name).collect::<Vec<_>>()}),
            );
        }
        let signature = format!("{method}:{arguments}");
        let operation = arguments["operation_id"].as_str().ok_or("Writes require operation_id")?.to_string();
        if operation.is_empty() || operation.len() > 80 {
            return Err("operation_id must be 1–80 characters.".into());
        }
        if let Some((_, previous, result)) = self.receipts.iter().find(|(id, _, _)| *id == operation) {
            return if *previous == signature { Ok(result.clone()) } else { Err("operation_id was already used with different arguments.".into()) };
        }
        let result = if method == "lemon_apply_edit" {
            let input: Apply = serde_json::from_value(arguments).map_err(|error| error.to_string())?;
            if input.operation_id != operation {
                return Err("Operation mismatch".into());
            }
            self.apply(app, input)?
        } else {
            let input: Write = serde_json::from_value(arguments).map_err(|error| error.to_string())?;
            if input.operation_id != operation {
                return Err("Operation mismatch".into());
            }
            self.guard(app, &input.session_id, &input.expected_revision)?;
            if method != "lemon_transport" || input.action.as_deref() != Some("stop") {
                self.idle(app)?;
            } else if app.loading() {
                return Err("Render/loading is busy.".into());
            }
            match method {
                "lemon_history" => {
                    capture(app);
                    match input.action.as_deref() {
                        Some("undo") => {
                            crate::history::undo(&mut app.state).ok_or("Nothing to undo")?;
                        }
                        Some("redo") => {
                            crate::history::redo(&mut app.state).ok_or("Nothing to redo")?;
                        }
                        _ => return Err("Use undo or redo.".into()),
                    }
                    app.plugin_tick();
                    app.invalidate();
                    self.context(app)
                }
                "lemon_transport" => {
                    if !matches!(input.action.as_deref(), Some("play_song" | "play_pattern" | "stop")) {
                        return Err("Use play_song, play_pattern or stop.".into());
                    }
                    if self.headless && input.action.as_deref() != Some("stop") {
                        return Err("Headless mode only renders previews; live playback needs an opted-in GUI session.".into());
                    }
                    if let Some(pattern) = input.pattern {
                        if app.state.project.pattern(pattern).is_none() {
                            return Err("Pattern not found".into());
                        }
                        app.state.pattern = pattern;
                        app.state.project.current_pattern = Some(pattern);
                    }
                    match input.action.as_deref() {
                        Some("play_song") if !self.headless => app.play_song_from(0),
                        Some("play_pattern") if !self.headless => {
                            app.state.song_mode = false;
                            app.transport.play(&crate::rack::play_project(&app.state), os_engine::PlayMode::Pattern(app.state.pattern), 0.0);
                        }
                        Some("stop") => {
                            app.act(crate::app::Action::Stop);
                        }
                        Some("play_song" | "play_pattern") => {
                            return Err("Headless mode renders previews; live playback requires an opted-in GUI session.".into());
                        }
                        _ => return Err("Use play_song, play_pattern or stop.".into()),
                    }
                    self.context(app)
                }
                "lemon_save_copy" => {
                    let destination = self.output(input.filename.as_deref().ok_or("filename required")?, "flp")?;
                    self.serial += 1;
                    let temporary = self.root.join("exports").join(format!(".save-{}-{}.flp", self.session_id, self.serial));
                    let saved = app.save_copy(temporary.clone()).ok_or("App is busy")?;
                    let result = saved.and_then(|()| std::fs::hard_link(&temporary, &destination).map_err(|error| error.to_string()));
                    let _ = std::fs::remove_file(&temporary);
                    result?;
                    json!({"path":destination,"session":self.context(app)})
                }
                "lemon_render" => {
                    let destination = self.output(input.filename.as_deref().ok_or("filename required")?, "wav")?;
                    let project = crate::rack::play_project(&app.state);
                    if project.arrangement().is_none_or(|arrangement| arrangement.clips.is_empty()) {
                        return Err("Arrange some clips before rendering.".into());
                    }
                    if project
                        .arrangements
                        .iter()
                        .flat_map(|arrangement| &arrangement.clips)
                        .any(|clip| u64::from(clip.position) + u64::from(clip.length) > u64::from(project.bar_ticks()) * 64)
                        || !(40.0..=240.0).contains(&app.state.tempo)
                        || project.channels.len() > 64
                    {
                        return Err("Render is limited to 64 bars/channels at 40–240 BPM.".into());
                    }
                    self.serial += 1;
                    let temporary = self.root.join("exports").join(format!(".render-{}-{}.wav", self.session_id, self.serial));
                    write_new(&temporary, &[])?;
                    app.plugins.set_offline(true);
                    app.assistant_export = true;
                    app.export = Some(("Assistant preview".into(), 0.0));
                    app.transport.export(&project, os_engine::PlayMode::Song, 48000, crate::export::Job::wav(&temporary));
                    let status = json!({"job_id":self.serial,"state":"rendering","progress":0.0});
                    self.job = Some(Job { id: self.serial, temporary, destination, status: status.clone(), running: true });
                    status
                }
                _ => return Err("Unknown Lemon tool.".into()),
            }
        };
        self.receipts.push_back((operation, signature, result.clone()));
        if self.receipts.len() > 64 {
            self.receipts.pop_front();
        }
        Ok(result)
    }
}

fn label(name: &str) -> Result<(), String> {
    if name.trim().is_empty() || name.len() > 80 || name.chars().any(char::is_control) {
        Err("Names must be 1–80 bytes without control characters.".into())
    } else {
        Ok(())
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|error| error.to_string())?;
    file.write_all(bytes).and_then(|()| file.sync_all()).map_err(|error| error.to_string())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SERIAL: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        controller: Controller,
        app: App,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("lemon-assistant-{}-{}", std::process::id(), SERIAL.fetch_add(1, Ordering::Relaxed)));
            std::fs::create_dir(&root).unwrap();
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
            for directory in ["assets", "checkpoints", "exports"] {
                std::fs::create_dir(root.join(directory)).unwrap();
            }
            let mut app = App::new(None);
            app.plugins.available = crate::plugins::builtin();
            app.plugins.effects = crate::plugins::builtin_effects();
            app.state.assistant_enabled = true;
            Self { controller: Controller::new(root, "test-session".into(), true), app }
        }

        fn call(&mut self, name: &str, mut args: Value, operation: &str) -> Result<Value, String> {
            args["session_id"] = json!("test-session");
            args["expected_revision"] = json!(self.controller.revision(&self.app));
            args["operation_id"] = json!(operation);
            self.controller.handle(&mut self.app, name, args)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.controller.root);
        }
    }

    #[test]
    fn assistant_rejects_armed_recording_and_waiting_for_input() {
        let mut fixture = Fixture::new();
        let before = fixture.app.state.project.clone();
        fixture.app.state.rec.armed = true;
        assert!(fixture.call("lemon_apply_edit", json!({"operations":[{"op":"set_tempo","bpm":150.0}]}), "armed").is_err());
        fixture.app.state.rec.armed = false;
        fixture.app.state.rec.waiting = true;
        assert!(fixture.call("lemon_apply_edit", json!({"operations":[{"op":"set_tempo","bpm":150.0}]}), "waiting").is_err());
        assert_eq!(fixture.app.state.project, before);
        assert_eq!(fixture.app.state.history.at(), 0);
    }

    #[test]
    fn assistant_invalid_batch_is_atomic_and_stale_sessions_are_rejected() {
        let mut fixture = Fixture::new();
        let before = fixture.app.state.project.clone();
        let bad = json!({"operations":[{"op":"set_tempo","bpm":160.0},{"op":"add_notes","pattern":999,"notes":[]}]});
        assert!(fixture.call("lemon_apply_edit", bad, "bad").is_err());
        assert_eq!(fixture.app.state.project, before);
        assert_eq!(fixture.app.state.history.at(), 0);
        let arguments = json!({"session_id":"wrong","expected_revision":fixture.controller.revision(&fixture.app),"operation_id":"wrong","operations":[{"op":"set_tempo","bpm":150.0}]});
        assert!(fixture.controller.handle(&mut fixture.app, "lemon_apply_edit", arguments).is_err());
        let arguments = json!({"session_id":"test-session","expected_revision":"stale","operation_id":"stale","operations":[{"op":"set_tempo","bpm":150.0}]});
        assert!(fixture.controller.handle(&mut fixture.app, "lemon_apply_edit", arguments).is_err());
        assert_eq!(std::fs::read_dir(fixture.controller.root.join("checkpoints")).unwrap().count(), 0);
    }

    #[test]
    fn assistant_batch_is_one_normal_undo_and_redo_restores_ray_and_notes() {
        let mut fixture = Fixture::new();
        let rows = fixture.app.state.project.channels.len();
        let before_tempo = fixture.app.state.tempo;
        let operations = json!({"operations":[
            {"op":"set_tempo","bpm":160.0},
            {"op":"add_channel","instrument":"ray","name":"Ray accents","preset":"Soft ray","volume":0.5,"pan":0.0},
            {"op":"create_pattern","number":2,"name":"Laser phrase","bars":4},
            {"op":"add_notes","pattern":2,"notes":[{"channel":rows,"key":72,"beat":0.0,"duration":1.0,"velocity":90}]},
            {"op":"place_pattern","pattern":2,"start_bar":0,"track":0}
        ]});
        fixture.call("lemon_apply_edit", operations, "compose").unwrap();
        assert_eq!(fixture.app.state.history.at(), 1);
        assert_eq!(fixture.app.state.project.channels.len(), rows + 1);
        assert_eq!(fixture.app.state.tempo, 160.0);
        assert_eq!(fixture.app.state.project.pattern(2).unwrap().notes.len(), 1);
        assert!(fixture.app.state.hosted_rows.contains(&rows));
        fixture.call("lemon_history", json!({"action":"undo"}), "undo").unwrap();
        assert_eq!(fixture.app.state.project.channels.len(), rows);
        assert_eq!(fixture.app.state.tempo, before_tempo);
        assert!(fixture.app.state.project.pattern(2).is_none());
        fixture.call("lemon_history", json!({"action":"redo"}), "redo").unwrap();
        assert_eq!(fixture.app.state.project.channels.len(), rows + 1);
        assert!(fixture.app.state.hosted_rows.contains(&rows));
        assert_eq!(fixture.app.state.project.pattern(2).unwrap().notes.len(), 1);
        assert_eq!(fixture.app.state.project.arrangement().unwrap().clips.len(), 1);
        assert_eq!(fixture.app.state.tempo, 160.0);
        let saved = fixture.call("lemon_save_copy", json!({"filename":"Ray.flp"}), "save-ray").unwrap();
        let file = os_flp::FlpFile::parse(&std::fs::read(saved["path"].as_str().unwrap()).unwrap()).unwrap();
        let project = os_flp::project::load(&file).unwrap();
        let reference = crate::project_plugins::wrapped_channel(&project.channels[rows]).unwrap();
        let length = u64::from_le_bytes(reference.state[..8].try_into().unwrap()) as usize;
        assert_eq!(reference.state.len(), length + 8);
        let state: Value = serde_json::from_slice(&reference.state[8..]).unwrap();
        for (id, value) in lemon_ray::params::IDS.iter().zip(lemon_ray::params::PRESETS[1].1) {
            assert_eq!(state["params"][id]["f32"].as_f64().unwrap(), f64::from(value));
        }
        fixture.app.reset_hosted();
        fixture.app.state.project = project;
        fixture.app.plugin_tick();
        assert!(fixture.app.state.hosted_rows.contains(&rows));
        assert_eq!(fixture.app.plugins.state(rows), Some(reference.state));
    }

    #[test]
    fn assistant_retries_do_not_duplicate_and_manual_edits_invalidate_revision() {
        let mut fixture = Fixture::new();
        let request = json!({"session_id":"test-session","expected_revision":fixture.controller.revision(&fixture.app),"operation_id":"once","operations":[{"op":"create_pattern","number":2,"name":"Once","bars":1}]});
        let first = fixture.controller.handle(&mut fixture.app, "lemon_apply_edit", request.clone()).unwrap();
        assert_eq!(first, fixture.controller.handle(&mut fixture.app, "lemon_apply_edit", request.clone()).unwrap());
        assert_eq!(fixture.app.state.history.at(), 1);
        let mut different = request;
        different["operations"][0]["name"] = json!("Different");
        assert!(fixture.controller.handle(&mut fixture.app, "lemon_apply_edit", different).is_err());
        let revision = fixture.controller.revision(&fixture.app);
        fixture.app.state.project.patterns[0].notes.push(Note::step(0, 12));
        assert_ne!(fixture.controller.revision(&fixture.app), revision);
    }

    #[test]
    fn assistant_paths_overwrites_and_headless_audio_are_guarded() {
        let mut fixture = Fixture::new();
        for filename in ["../outside.flp", "/tmp/out.flp", "bad.wav", "a/b.flp"] {
            assert!(fixture.controller.output(filename, "flp").is_err());
        }
        std::fs::write(fixture.controller.root.join("exports/existing.flp"), b"keep").unwrap();
        assert!(fixture.controller.output("existing.flp", "flp").is_err());
        std::os::unix::fs::symlink("/etc/passwd", fixture.controller.root.join("assets/escape.wav")).unwrap();
        assert!(fixture.controller.asset("escape.wav").is_err());
        let pattern = fixture.app.state.pattern;
        assert!(fixture.call("lemon_transport", json!({"action":"play_pattern","pattern":1}), "no-audio").is_err());
        assert_eq!(fixture.app.state.pattern, pattern);
        fixture.app.state.path = Some(PathBuf::from("/outside/someone.flp"));
        assert!(fixture.call("lemon_apply_edit", json!({"operations":[{"op":"set_tempo","bpm":150.0}]}), "foreign").is_err());
    }

    #[test]
    fn assistant_save_copy_reopens_and_never_overwrites() {
        let mut fixture = Fixture::new();
        fixture.call("lemon_apply_edit", json!({"operations":[{"op":"create_pattern","number":2,"name":"Saved phrase","bars":16},{"op":"add_notes","pattern":2,"notes":[{"channel":0,"key":60,"beat":1.0,"duration":0.25,"velocity":90}]},{"op":"place_pattern","pattern":2,"start_bar":0,"track":0}]}), "notes").unwrap();
        let result = fixture.call("lemon_save_copy", json!({"filename":"Sketch.flp"}), "save").unwrap();
        assert!(fixture.app.state.path.is_none());
        let bytes = std::fs::read(result["path"].as_str().unwrap()).unwrap();
        let file = os_flp::FlpFile::parse(&bytes).unwrap();
        let project = os_flp::project::load(&file).unwrap();
        assert_eq!(project.pattern(2).unwrap().notes.len(), 1);
        assert_eq!(project.arrangement().unwrap().clips[0].length, project.bar_ticks() * 16);
        assert!(fixture.call("lemon_save_copy", json!({"filename":"Sketch.flp"}), "save-again").is_err());
        assert_eq!(std::fs::read(result["path"].as_str().unwrap()).unwrap(), bytes);
    }
}
