use std::fs::{DirBuilder, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{Value, json};

use super::Controller;
use crate::app::App;

const MAX_REQUEST: u64 = 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    name: String,
    arguments: Value,
}

struct Pending {
    request: Request,
    deadline: Instant,
    canceled: Arc<AtomicBool>,
    reply: mpsc::SyncSender<Value>,
}

pub struct Bridge {
    controller: Controller,
    requests: mpsc::Receiver<Pending>,
    quit: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    socket: PathBuf,
    inode: u64,
}

fn private_directory(path: &Path) -> Result<(), String> {
    match DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.to_string()),
    }
    let metadata = path.symlink_metadata().map_err(|error| error.to_string())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o077 != 0 {
        return Err(format!("{} must be a real directory owned by you with permissions 0700.", path.display()));
    }
    Ok(())
}

impl Bridge {
    pub fn open(root: &Path, headless: bool) -> Result<Self, String> {
        if !root.is_absolute() {
            return Err("Assistant workspace must be absolute.".into());
        }
        private_directory(root)?;
        let root = root.canonicalize().map_err(|error| error.to_string())?;
        for directory in ["assets", "exports", "checkpoints"] {
            private_directory(&root.join(directory))?;
        }
        let socket = root.join("control.sock");
        let listener = UnixListener::bind(&socket).map_err(|error| format!("{}: {error}; another session may own this socket.", socket.display()))?;
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600)).map_err(|error| error.to_string())?;
        listener.set_nonblocking(true).map_err(|error| error.to_string())?;
        let inode = socket.symlink_metadata().map_err(|error| error.to_string())?.ino();
        let mut random = [0u8; 16];
        std::fs::File::open("/dev/urandom").and_then(|mut file| file.read_exact(&mut random)).map_err(|error| error.to_string())?;
        let session_id = random.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
        let (sender, requests) = mpsc::sync_channel::<Pending>(4);
        let quit = Arc::new(AtomicBool::new(false));
        let closing = quit.clone();
        let audit = root.join("assistant-audit.jsonl");
        let thread = std::thread::Builder::new().name("lemon-assistant-io".into()).spawn(move || {
            while !closing.load(Ordering::Acquire) {
                let (mut connection, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => { std::thread::sleep(Duration::from_millis(20)); continue; }
                    Err(_) => break,
                };
                let _ = connection.set_read_timeout(Some(Duration::from_secs(2)));
                let _ = connection.set_write_timeout(Some(Duration::from_secs(2)));
                let mut bytes = Vec::new();
                let read = BufReader::new((&mut connection).take(MAX_REQUEST + 1)).read_until(b'\n', &mut bytes);
                let response = if read.is_err() || bytes.len() as u64 > MAX_REQUEST || bytes.last() != Some(&b'\n') {
                    json!({"ok":false,"error":"Request must be one JSON line, at most 1 MiB."})
                } else {
                    match serde_json::from_slice::<Request>(&bytes) {
                        Err(error) => json!({"ok":false,"error":error.to_string()}),
                        Ok(request) => {
                            let name = request.name.clone();
                            let operation = request.arguments.get("operation_id").cloned();
                            let (reply, receiver) = mpsc::sync_channel(1);
                            let canceled = Arc::new(AtomicBool::new(false));
                            let pending = Pending { request, deadline: Instant::now() + Duration::from_secs(8), canceled: canceled.clone(), reply };
                            let response = if sender.try_send(pending).is_err() {
                                json!({"ok":false,"error":"Assistant queue is busy."})
                            } else {
                                crate::wake_ui();
                                match receiver.recv_timeout(Duration::from_secs(8)) {
                                    Ok(response) => response,
                                    Err(_) => {
                                        canceled.store(true, Ordering::Release);
                                        json!({"ok":false,"error":"App did not respond in time. Read session before retrying; preserve operation_id."})
                                    }
                                }
                            };
                            if let Ok(mut file) = OpenOptions::new().append(true).create(true).mode(0o600).custom_flags(libc::O_NOFOLLOW).open(&audit) {
                                let _ = writeln!(file, "{}", json!({"tool":name,"operation_id":operation,"ok":response["ok"],"time":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()}));
                            }
                            response
                        }
                    }
                };
                let _ = writeln!(connection, "{response}");
            }
        }).map_err(|error| error.to_string())?;
        Ok(Self { controller: Controller::new(root, session_id, headless), requests, quit, thread: Some(thread), socket, inode })
    }

    pub fn poll(&mut self, app: &mut App) {
        self.controller.poll_render(app);
        for _ in 0..4 {
            let Ok(pending) = self.requests.try_recv() else { break };
            if pending.canceled.load(Ordering::Acquire) || Instant::now() >= pending.deadline {
                continue;
            }
            let result = self.controller.handle(app, &pending.request.name, pending.request.arguments);
            let response = match result {
                Ok(result) => json!({"ok":true,"result":result}),
                Err(error) => json!({"ok":false,"error":error}),
            };
            let _ = pending.reply.send(response);
        }
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.quit.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        if self.socket.symlink_metadata().is_ok_and(|metadata| metadata.file_type().is_socket() && metadata.ino() == self.inode) {
            let _ = std::fs::remove_file(&self.socket);
        }
    }
}

pub fn headless(root: &Path) -> Result<(), String> {
    let mut bridge = Bridge::open(root, true)?;
    unsafe {
        std::env::set_var("OPENSTUDIO_USER_DIR", bridge.controller.root.join("settings"));
    }
    let mut app = App::new(None);
    app.plugins.available = crate::plugins::builtin();
    app.plugins.effects = crate::plugins::builtin_effects();
    app.state.assistant_enabled = true;
    app.state.song_mode = true;
    eprintln!("Lemon assistant scratch session: {}", root.display());
    loop {
        bridge.poll(&mut app);
        app.tick();
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixStream;
    use std::sync::atomic::AtomicU64;

    static SERIAL: AtomicU64 = AtomicU64::new(0);

    struct Directory(PathBuf);

    impl Directory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("lemon-bridge-{}-{}", std::process::id(), SERIAL.fetch_add(1, Ordering::Relaxed)));
            Self(path)
        }
    }

    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn assistant_bridge_rejects_shared_directories_symlinks_and_existing_socket() {
        let directory = Directory::new();
        std::fs::create_dir(&directory.0).unwrap();
        std::fs::set_permissions(&directory.0, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(Bridge::open(&directory.0, true).is_err());
        std::fs::set_permissions(&directory.0, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::os::unix::fs::symlink(std::env::temp_dir(), directory.0.join("assets")).unwrap();
        assert!(Bridge::open(&directory.0, true).is_err());
        std::fs::remove_file(directory.0.join("assets")).unwrap();
        let bridge = Bridge::open(&directory.0, true).unwrap();
        assert_eq!(bridge.socket.symlink_metadata().unwrap().mode() & 0o777, 0o600);
        assert!(Bridge::open(&directory.0, true).is_err());
        let socket = bridge.socket.clone();
        drop(bridge);
        assert!(!socket.exists());
        assert!(Bridge::open(&directory.0, true).is_ok());
    }

    #[test]
    fn assistant_bridge_real_socket_reaches_app_without_mutation() {
        let directory = Directory::new();
        let mut bridge = Bridge::open(&directory.0, true).unwrap();
        let socket = bridge.socket.clone();
        let client = std::thread::spawn(move || {
            let mut stream = UnixStream::connect(socket).unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            writeln!(stream, "{}", json!({"name":"lemon_get_session","arguments":{}})).unwrap();
            let mut line = String::new();
            BufReader::new(stream).read_line(&mut line).unwrap();
            serde_json::from_str::<Value>(&line).unwrap()
        });
        let mut app = App::new(None);
        let deadline = Instant::now() + Duration::from_secs(4);
        while !client.is_finished() && Instant::now() < deadline {
            bridge.poll(&mut app);
            std::thread::sleep(Duration::from_millis(5));
        }
        let response = client.join().unwrap();
        assert_eq!(response["ok"], true);
        assert_eq!(response["result"]["headless"], true);
        assert_eq!(app.state.history.at(), 0);
    }

    #[test]
    fn assistant_bridge_drops_expired_and_canceled_writes() {
        let directory = Directory::new();
        let mut bridge = Bridge::open(&directory.0, true).unwrap();
        let (sender, requests) = mpsc::sync_channel(4);
        bridge.requests = requests;
        let mut app = App::new(None);
        let tempo = app.state.tempo;
        for (operation, expired, canceled) in [("expired", true, false), ("canceled", false, true)] {
            let arguments = json!({"session_id":bridge.controller.session_id,"expected_revision":bridge.controller.revision(&app),"operation_id":operation,"operations":[{"op":"set_tempo","bpm":160.0}]});
            let (reply, receiver) = mpsc::sync_channel(1);
            sender
                .send(Pending {
                    request: Request { name: "lemon_apply_edit".into(), arguments },
                    deadline: if expired { Instant::now() - Duration::from_secs(1) } else { Instant::now() + Duration::from_secs(8) },
                    canceled: Arc::new(AtomicBool::new(canceled)),
                    reply,
                })
                .unwrap();
            bridge.poll(&mut app);
            assert!(receiver.try_recv().is_err());
        }
        assert_eq!(app.state.tempo, tempo);
        assert_eq!(app.state.history.at(), 0);
        assert_eq!(std::fs::read_dir(directory.0.join("checkpoints")).unwrap().count(), 0);
    }
}
