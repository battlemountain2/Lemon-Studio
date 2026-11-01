use super::*;
use crate::app::{Action, App, Ui};
use crate::playlist::{TRACKS, Wave};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::sync::{Arc, mpsc};

pub(super) const MAX_FRAMES: usize = 1_000_000;
const MAX_BYTES: usize = 32 * 1024 * 1024;
type Stamp = Option<(u64, Option<std::time::SystemTime>)>;

fn stamp(path: &Path) -> Stamp {
    std::fs::metadata(path).ok().map(|m| (m.len(), m.modified().ok()))
}

pub(super) struct Input {
    project: os_model::Project,
    project_path: Option<PathBuf>,
    tempo: f64,
    clip_idx: usize,
    mode: StemMode,
    source: PathBuf,
    source_stamp: Stamp,
    out_dir: PathBuf,
}

impl Input {
    pub(super) fn capture(s: &State, clip_idx: usize, mode: StemMode) -> Result<Self, String> {
        let clip = clips(s).get(clip_idx).ok_or("Select an audio clip first")?;
        let ClipSource::Channel(index) = clip.source else {
            return Err("Select an audio clip first".into());
        };
        let channel = s.project.channel(index).ok_or("Audio channel not found")?;
        if channel.kind != os_model::ChannelKind::AudioClip {
            return Err("Select an audio clip first".into());
        }
        if s.project.channels.iter().enumerate().any(|(i, c)| usize::from(c.index) != i) {
            return Err("The project's channel indices must be contiguous before splitting".into());
        }
        let path = channel.sample_path.as_deref().ok_or("Channel has no sample")?;
        let source = PathBuf::from(crate::plugins::resolve_sample(path));
        Ok(Self {
            project: s.project.clone(),
            project_path: s.path.clone(),
            tempo: s.tempo,
            clip_idx,
            mode,
            source_stamp: stamp(&source),
            source,
            out_dir: stems_dir(s),
        })
    }

    pub(super) fn prepare(self, cancel: &AtomicBool, progress: &AtomicUsize) -> Result<Prepared, String> {
        self.prepare_with(cancel, progress, |file, rate, samples| {
            let mut writer = BufWriter::new(file);
            os_engine::wav::write_f32_stereo(&mut writer, rate, samples).map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
            writer.get_ref().sync_all().map_err(|e| e.to_string())
        })
    }

    fn prepare_with(
        self,
        cancel: &AtomicBool,
        progress: &AtomicUsize,
        mut write: impl FnMut(&mut File, u32, &[f32]) -> Result<(), String>,
    ) -> Result<Prepared, String> {
        let sample = os_engine::sample::load_bounded(&self.source, MAX_BYTES, MAX_FRAMES, cancel)?;
        progress.store(10, Ordering::Relaxed);
        let stems = split_controlled(&sample.left, &sample.right, sample.rate, self.mode, cancel, progress)?;
        if stamp(&self.source) != self.source_stamp {
            return Err("Source changed during the split; try again".into());
        }
        let mut files = Artifacts::create(&self.out_dir)?;
        let mut outputs = Vec::with_capacity(stems.len());
        let count = stems.len();
        for (i, stem) in stems.into_iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                return Err("Spectral split cancelled".into());
            }
            let path = files.dir.join(format!("{:02} - {}.wav", i + 1, stem.label));
            let mut file =
                OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| format!("Could not create split output: {e}"))?;
            files.paths.push(path.clone());
            write(&mut file, sample.rate, &stem.interleaved_audio)?;
            let peaks = stem
                .interleaved_audio
                .chunks(128)
                .map(|chunk| {
                    chunk.chunks_exact(2).map(|f| (f[0] + f[1]) * 0.5).fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)))
                })
                .collect();
            outputs.push(Output {
                path,
                label: stem.label,
                color: stem.color,
                wave: Wave { rate: sample.rate, frames: sample.frames(), per: 64, peaks },
            });
            progress.store(80 + (i + 1) * 20 / count, Ordering::Relaxed);
        }
        if cancel.load(Ordering::Relaxed) {
            return Err("Spectral split cancelled".into());
        }
        Ok(Prepared { input: self, files, outputs })
    }
}

struct Artifacts {
    dir: PathBuf,
    paths: Vec<PathBuf>,
    retained: bool,
}

impl Artifacts {
    fn create(parent: &Path) -> Result<Self, String> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        loop {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let dir = parent.join(format!("Spectral-{}-{id}", std::process::id()));
            match std::fs::create_dir(&dir) {
                Ok(()) => return Ok(Self { dir, paths: Vec::new(), retained: false }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
    }
}

impl Drop for Artifacts {
    fn drop(&mut self) {
        if !self.retained {
            for path in &self.paths {
                let _ = std::fs::remove_file(path);
            }
            let _ = std::fs::remove_dir(&self.dir);
        }
    }
}

struct Output {
    path: PathBuf,
    label: String,
    color: Rgb,
    wave: Wave,
}

pub(super) struct Prepared {
    input: Input,
    files: Artifacts,
    outputs: Vec<Output>,
}

impl Prepared {
    pub(super) fn commit(mut self, s: &mut State) -> Result<Vec<usize>, String> {
        let input = &self.input;
        if s.project != input.project
            || s.path != input.project_path
            || s.tempo != input.tempo
            || stamp(&input.source) != input.source_stamp
        {
            return Err("Project or source changed during the split; no edits were applied. Try again".into());
        }
        let clip = clips(s)[input.clip_idx];
        let ClipSource::Channel(index) = clip.source else {
            return Err("Source clip changed".into());
        };
        let channel = s.project.channel(index).ok_or("Source channel missing")?.clone();
        let from = s.project.channels.len();
        if from + self.outputs.len() > usize::from(u16::MAX) {
            return Err("Too many channels for a split".into());
        }
        let end = clip.position.checked_add(clip.length).ok_or("Clip time exceeds project limits")?;
        let mut cursor = clip.track.checked_add(1).ok_or("No playlist tracks available")?;
        let mut plan = Vec::with_capacity(self.outputs.len());
        for (i, output) in self.outputs.iter().enumerate() {
            let track = (cursor..TRACKS)
                .find(|&t| !clips(s).iter().any(|c| c.track == t && c.position < end && c.end() > clip.position))
                .ok_or("No free playlist tracks below the clip")?;
            cursor = track + 1;
            let mut ch = channel.clone();
            ch.index = (from + i) as u16;
            ch.name = Some(format!("{} - {} (spectral)", input.source.file_stem().unwrap_or_default().to_string_lossy(), output.label));
            ch.color = Some(output.color);
            ch.sample_path = Some(output.path.to_string_lossy().into_owned());
            ch.plugin = None;
            ch.plugin_state = None;
            let mut c = clip;
            c.track = track;
            c.source = ClipSource::Channel(ch.index);
            c.muted = false;
            plan.push((ch, c));
        }
        s.project.channels.try_reserve(plan.len()).map_err(|e| e.to_string())?;
        s.project.arrangement_mut().unwrap().clips.try_reserve(plan.len()).map_err(|e| e.to_string())?;
        let tail = crate::history::Snap::Tail(Box::new(crate::history::Tail::now(s, from)));
        let playlist = crate::history::Snap::Playlist(Box::new(crate::playlist::capture(s)));
        crate::history::record(s, "Experimental spectral split", crate::history::Snap::Several(vec![playlist, tail]));
        arr_mut(s).clips[input.clip_idx].muted = true;
        let mut indices = Vec::with_capacity(plan.len());
        for ((ch, c), output) in plan.into_iter().zip(self.outputs.drain(..)) {
            let path = ch.sample_path.as_ref().unwrap().clone();
            crate::rack::push_channel(s, ch);
            indices.push(clips(s).len());
            arr_mut(s).clips.push(c);
            s.playlist.waves.borrow_mut().insert(path, Some(std::rc::Rc::new(output.wave)));
        }
        s.playlist.sel = indices.clone();
        self.files.retained = true;
        Ok(indices)
    }
}

pub struct Job {
    cancel: Arc<AtomicBool>,
    progress: Arc<AtomicUsize>,
    result: mpsc::Receiver<Result<Prepared, String>>,
}

impl Job {
    fn launch(input: Input) -> Result<Self, String> {
        let cancel = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(AtomicUsize::new(0));
        let (tx, result) = mpsc::channel();
        let (c, p) = (cancel.clone(), progress.clone());
        std::thread::Builder::new()
            .name("spectral-split".into())
            .spawn(move || {
                let result = input.prepare(&c, &p);
                let _ = tx.send(result);
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { cancel, progress, result })
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

pub fn start(app: &mut App, target: Option<usize>, mode: StemMode) -> Result<(), String> {
    if app.spectral_job.is_some() {
        return Err("A spectral split is already running".into());
    }
    if app.loading() || app.audio_rec.is_some() || app.state.rec.recording() || app.state.rec.armed {
        return Err("Finish loading, exporting or recording before splitting".into());
    }
    let index = target.or_else(|| app.state.playlist.sel.first().copied()).ok_or("Select an audio clip first")?;
    app.spectral_job = Some(Job::launch(Input::capture(&app.state, index, mode)?)?);
    app.state.spectral_progress = Some((0, false));
    app.invalidate();
    Ok(())
}

pub fn cancel(app: &mut App) {
    if let Some(job) = &app.spectral_job {
        job.cancel.store(true, Ordering::Relaxed);
        app.state.spectral_progress = Some((job.progress.load(Ordering::Relaxed), true));
        app.invalidate();
    }
}

pub fn tick(app: &mut App) {
    if app.loading() || app.audio_rec.is_some() || app.state.rec.recording() || app.state.rec.armed {
        if let Some(job) = &app.spectral_job {
            job.cancel.store(true, Ordering::Relaxed);
        }
    }
    let Some(job) = &app.spectral_job else {
        return;
    };
    let progress = (job.progress.load(Ordering::Relaxed), job.cancel.load(Ordering::Relaxed));
    match job.result.try_recv() {
        Ok(result) => {
            let cancelled = progress.1;
            app.spectral_job = None;
            app.state.spectral_progress = None;
            let message = if cancelled {
                drop(result);
                "Spectral split cancelled".into()
            } else {
                match result.and_then(|prepared| prepared.commit(&mut app.state)) {
                    Ok(indices) => format!("Created {} experimental spectral layers; Undo restores the source", indices.len()),
                    Err(e) => format!("Spectral split: {e}"),
                }
            };
            app.say(message);
            app.invalidate();
        }
        Err(mpsc::TryRecvError::Empty) => {
            if app.state.spectral_progress != Some(progress) {
                app.state.spectral_progress = Some(progress);
                app.invalidate();
            }
        }
        Err(mpsc::TryRecvError::Disconnected) => {
            app.spectral_job = None;
            app.state.spectral_progress = None;
            app.say("Spectral split worker stopped; no project edits were applied".into());
            app.invalidate();
        }
    }
}

pub fn paint(ui: &mut Ui, s: &State) {
    let Some((progress, cancelled)) = s.spectral_progress else {
        return;
    };
    let (w, h) = ui.c.size();
    let r = os_ui::Rect::new((w - 390.0).max(0.0), (h - 100.0).max(0.0), 380.0, 88.0);
    os_ui::kit::popup(ui.c, ui.t, r);
    ui.hit(r, Action::SpectralArea, "Experimental frequency/stereo split; not AI source separation");
    ui.c.text(os_ui::Font::new(14), r.x + 10.0, r.y + 10.0, ui.t.text, "Experimental spectral split");
    ui.c.text(
        os_ui::Font::new(13),
        r.x + 10.0,
        r.y + 33.0,
        ui.t.text_dim,
        &format!("{}  {progress}%", if cancelled { "Cancelling…" } else { "Processing source audio" }),
    );
    let button = os_ui::Rect::new(r.right() - 78.0, r.y + 8.0, 68.0, 24.0);
    if !cancelled {
        ui.hit(button, Action::CancelSpectralSplit, "Cancel spectral split");
    }
    ui.c.text(os_ui::Font::new(13), button.x + 5.0, button.y + 3.0, ui.t.accent, "Cancel");
    ui.c.fill_rect(os_ui::Rect::new(r.x + 10.0, r.y + 62.0, 360.0, 8.0), ui.t.sunken_dark);
    ui.c.fill_rect(os_ui::Rect::new(r.x + 10.0, r.y + 62.0, 360.0 * progress.min(100) as f32 / 100.0, 8.0), ui.t.accent);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        dir: PathBuf,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn fixture() -> (App, Fixture, usize) {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!("spectral-test-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("Input.wav");
        let samples: Vec<_> = (0..4410)
            .flat_map(|i| {
                let v = (i as f32 * 0.1).sin() * 0.2;
                [v, v * 0.7]
            })
            .collect();
        os_engine::wav::write_f32_stereo(&mut File::create(&source).unwrap(), 44100, &samples).unwrap();
        let mut app = App::new(None);
        app.state.path = Some(dir.join("project.flp"));
        let clip = crate::playlist::add_audio_clip(&mut app.state, &source.to_string_lossy(), 0, 96).unwrap();
        (app, Fixture { dir }, clip)
    }

    fn prepare(s: &State, clip: usize) -> Prepared {
        Input::capture(s, clip, StemMode::VocalsAndMusic).unwrap().prepare(&AtomicBool::new(false), &AtomicUsize::new(0)).unwrap()
    }

    #[test]
    fn repeated_splits_preserve_files_and_original_channel_settings() {
        let (mut app, fixture, clip) = fixture();
        let sentinel = stems_dir(&app.state).join("Input - Vocals.wav");
        std::fs::create_dir_all(sentinel.parent().unwrap()).unwrap();
        std::fs::write(&sentinel, b"earlier audio").unwrap();
        let ClipSource::Channel(index) = clips(&app.state)[clip].source else { panic!() };
        let row = usize::from(index);
        app.state.project.channels[row].volume = 0.43;
        app.state.project.channels[row].pan = -0.2;
        app.state.project.channels[row].mixer_track = 4;
        arr_mut(&mut app.state).clips[clip].window = Some((12.0, 65.0));
        let before = app.state.project.clone();
        let indices = prepare(&app.state, clip).commit(&mut app.state).unwrap();
        let after = app.state.project.clone();
        let first_paths: Vec<_> =
            app.state.project.channels[before.channels.len()..].iter().map(|c| c.sample_path.clone().unwrap()).collect();
        let first_bytes: Vec<_> = first_paths.iter().map(|p| std::fs::read(p).unwrap()).collect();
        for &i in &indices {
            let c = clips(&app.state)[i];
            assert_eq!(
                (c.position, c.length, c.window, c.raw),
                (clips_from(&before)[clip].position, clips_from(&before)[clip].length, Some((12.0, 65.0)), clips_from(&before)[clip].raw)
            );
            let ClipSource::Channel(index) = c.source else { panic!() };
            let channel = app.state.project.channel(index).unwrap();
            assert_eq!((channel.volume, channel.pan, channel.mixer_track), (0.43, -0.2, 4));
        }
        crate::history::undo(&mut app.state);
        assert_eq!(app.state.project, before);
        crate::history::redo(&mut app.state);
        assert_eq!(app.state.project, after);
        let saved = os_flp::project::create(&app.state.project).unwrap().to_bytes().unwrap();
        let reopened = os_flp::project::load(&os_flp::FlpFile::parse(&saved).unwrap()).unwrap();
        assert_eq!(reopened.channels.len(), after.channels.len());
        let rendered = &reopened.channels[before.channels.len()..];
        for (channel, expected) in rendered.iter().zip(&after.channels[before.channels.len()..]) {
            assert_eq!(
                (&channel.sample_path, &channel.name, channel.color, channel.mixer_track),
                (&expected.sample_path, &expected.name, expected.color, expected.mixer_track)
            );
        }
        assert_eq!(clips_from(&reopened).len(), clips_from(&after).len());
        assert!(clips_from(&reopened)[clip].muted);
        prepare(&app.state, clip).commit(&mut app.state).unwrap();
        for (path, bytes) in first_paths.iter().zip(first_bytes) {
            assert_eq!(std::fs::read(path).unwrap(), bytes);
        }
        assert_eq!(std::fs::read(sentinel).unwrap(), b"earlier audio");
        assert!(fixture.dir.exists());
    }

    fn clips_from(project: &os_model::Project) -> &[os_model::Clip] {
        &project.arrangement().unwrap().clips
    }

    #[test]
    fn failed_second_write_removes_only_its_outputs_and_never_mutes_source() {
        let (app, _fixture, clip) = fixture();
        let before = app.state.project.clone();
        let mut writes = 0;
        let result = Input::capture(&app.state, clip, StemMode::All4).unwrap().prepare_with(
            &AtomicBool::new(false),
            &AtomicUsize::new(0),
            |file, _, _| {
                writes += 1;
                file.write_all(b"partial").unwrap();
                if writes == 2 { Err("injected disk failure".into()) } else { Ok(()) }
            },
        );
        assert!(result.is_err());
        assert_eq!(writes, 2);
        assert_eq!(app.state.project, before);
        assert_eq!(std::fs::read_dir(stems_dir(&app.state)).unwrap().count(), 0);
    }

    #[test]
    fn cancellation_after_first_write_cleans_up() {
        let (app, _fixture, clip) = fixture();
        let cancel = AtomicBool::new(false);
        let result = Input::capture(&app.state, clip, StemMode::All4).unwrap().prepare_with(&cancel, &AtomicUsize::new(0), |file, _, _| {
            file.write_all(b"partial").unwrap();
            cancel.store(true, Ordering::Relaxed);
            Ok(())
        });
        assert!(result.is_err());
        assert!(!clips(&app.state)[clip].muted);
        assert_eq!(std::fs::read_dir(stems_dir(&app.state)).unwrap().count(), 0);
    }

    #[test]
    fn stale_project_and_source_results_are_discarded() {
        let (mut app, _fixture, clip) = fixture();
        let prepared = prepare(&app.state, clip);
        let directory = prepared.files.dir.clone();
        app.state.project.channels[0].volume = 0.3;
        let edited = app.state.project.clone();
        assert!(prepared.commit(&mut app.state).is_err());
        assert_eq!(app.state.project, edited);
        assert!(!directory.exists());
        let prepared = prepare(&app.state, clip);
        let directory = prepared.files.dir.clone();
        std::fs::write(&prepared.input.source, b"changed").unwrap();
        assert!(prepared.commit(&mut app.state).is_err());
        assert_eq!(app.state.project, edited);
        assert!(!directory.exists());
    }

    #[test]
    fn no_free_tracks_aborts_without_a_partial_edit() {
        let (mut app, _fixture, clip) = fixture();
        arr_mut(&mut app.state).clips[clip].track = TRACKS - 1;
        let before = app.state.project.clone();
        let prepared = prepare(&app.state, clip);
        let directory = prepared.files.dir.clone();
        assert!(prepared.commit(&mut app.state).is_err());
        assert_eq!(app.state.project, before);
        assert!(!directory.exists());
    }

    #[test]
    fn background_job_is_single_and_cancel_remains_pending_until_cleanup() {
        let (mut app, _fixture, clip) = fixture();
        let before = app.state.project.clone();
        start(&mut app, Some(clip), StemMode::All4).unwrap();
        assert!(start(&mut app, Some(clip), StemMode::All4).is_err());
        cancel(&mut app);
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while app.spectral_job.is_some() {
            assert!(std::time::Instant::now() < until);
            tick(&mut app);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(app.state.project, before);
        assert!(app.state.spectral_progress.is_none());
    }
}
