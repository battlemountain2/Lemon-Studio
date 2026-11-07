// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Hyprland as the window manager of the detached frames (XWayland windows).
//! - It lists `_NET_WM_MOVERESIZE` as supported but ignores it, so a title-bar drag would not move the frame. Moving it
//!   ourselves through X11 shakes: XWayland reports the pointer against where the window is drawn, which lags behind
//!   where it was asked to go, and every step overshoots. So the drag asks Hyprland where the pointer is and has it put
//!   the window there, both in its own coordinates (`Drag`).
//! - It draws its border and shadow around every window, the frame's own title bar and edge included, and animates
//!   moves (a dragged window would trail the pointer): once a frame is on screen, its IPC socket takes all three off
//!   that window (`plain`).

use std::io::{Read, Write};
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::{ffi::OsStrExt, net::UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_millis(30);
const REPLY_LIMIT: usize = 1024 * 1024;

pub fn running() -> bool {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

pub fn class(frame: u32) -> String {
    format!("lemonstudio-frame-{frame:x}")
}

fn socket_path() -> Option<PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")?;
    let sig = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    Some(Path::new(&dir).join("hypr").join(sig).join(".socket.sock"))
}

fn target(list: &serde_json::Value, frame: u32, pid: u32) -> Option<&serde_json::Value> {
    let class = class(frame);
    list.as_array()?.iter().find(|c| {
        c["pid"].as_u64() == Some(u64::from(pid))
            && c["xwayland"].as_bool() == Some(true)
            && c["class"].as_str() == Some(&class)
            && c["mapped"].as_bool() != Some(false)
            && address(c).is_some()
    })
}

fn address(client: &serde_json::Value) -> Option<&str> {
    let a = client["address"].as_str()?;
    let hex = a.strip_prefix("0x")?;
    (!hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit())).then_some(a)
}

pub fn plain(frame: u32) {
    let Some(path) = socket_path() else {
        return;
    };
    let _ = std::thread::Builder::new().name("hypr-frame".into()).spawn(move || {
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(1500) {
            std::thread::sleep(Duration::from_millis(40));
            let Some(list) = ask(&path, "j/clients").and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()) else {
                return;
            };
            let Some(client) = target(&list, frame, std::process::id()) else {
                continue;
            };
            let Some(a) = address(client) else {
                return;
            };
            for (prop, value) in [("border_size", "0"), ("no_shadow", "1"), ("no_anim", "1")] {
                let lua = format!("dispatch hl.dsp.window.set_prop({{ prop = '{prop}', value = '{value}', window = 'address:{a}' }})");
                if ask(&path, &lua).as_deref() != Some("ok") {
                    ask(&path, &format!("dispatch setprop address:{a} {prop} {value}"));
                }
            }
            return;
        }
    });
}

struct Control {
    epoch: AtomicUsize,
    released: AtomicBool,
    stopped: AtomicBool,
    failed: AtomicBool,
}

pub struct Drag {
    control: Arc<Control>,
}

impl Drag {
    pub fn start(frame: u32) -> Option<Self> {
        let path = socket_path()?;
        let control = Arc::new(Control {
            epoch: AtomicUsize::new(0),
            released: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            failed: AtomicBool::new(false),
        });
        let c = control.clone();
        std::thread::Builder::new()
            .name("hypr-drag".into())
            .spawn(move || {
                let result = (|| -> Option<()> {
                    let list = serde_json::from_str::<serde_json::Value>(&ask(&path, "j/clients")?).ok()?;
                    let client = target(&list, frame, std::process::id())?;
                    let a = address(client)?.to_string();
                    let at = (client["at"][0].as_f64()?, client["at"][1].as_f64()?);
                    let initial = cursor(&path)?;
                    let hold = (initial.0 - at.0, initial.1 - at.1);
                    let mut seen = 0;
                    while !c.stopped.load(Ordering::Relaxed) {
                        let epoch = c.epoch.load(Ordering::Acquire);
                        let released = c.released.load(Ordering::Acquire);
                        if epoch != seen || released {
                            seen = epoch;
                            let cursor = cursor(&path)?;
                            let (x, y) = ((cursor.0 - hold.0).round(), (cursor.1 - hold.1).round());
                            let lua = format!("dispatch hl.dsp.window.move({{ x = {x}, y = {y}, window = 'address:{a}' }})");
                            if ask(&path, &lua).as_deref() != Some("ok")
                                && ask(&path, &format!("dispatch movewindowpixel exact {x} {y},address:{a}")).as_deref() != Some("ok")
                            {
                                return None;
                            }
                            if released {
                                break;
                            }
                        }
                        std::thread::sleep(Duration::from_millis(8));
                    }
                    Some(())
                })();
                if result.is_none() {
                    c.failed.store(true, Ordering::Release);
                }
            })
            .ok()?;
        Some(Self { control })
    }

    pub fn follow(&mut self, release: bool) -> bool {
        if self.control.failed.load(Ordering::Acquire) {
            return false;
        }
        self.control.epoch.fetch_add(1, Ordering::Release);
        if release {
            self.control.released.store(true, Ordering::Release);
        }
        true
    }
}

impl Drop for Drag {
    fn drop(&mut self) {
        if !self.control.released.load(Ordering::Acquire) {
            self.control.stopped.store(true, Ordering::Release);
        }
    }
}

fn cursor(path: &Path) -> Option<(f64, f64)> {
    let v: serde_json::Value = serde_json::from_str(&ask(path, "j/cursorpos")?).ok()?;
    let p = (v["x"].as_f64()?, v["y"].as_f64()?);
    (p.0.is_finite() && p.1.is_finite()).then_some(p)
}

fn connect(path: &Path) -> Option<UnixStream> {
    let bytes = path.as_os_str().as_bytes();
    let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    if bytes.len() >= addr.sun_path.len() {
        return None;
    }
    addr.sun_family = libc::AF_UNIX as _;
    for (out, &byte) in addr.sun_path.iter_mut().zip(bytes) {
        *out = byte as _;
    }
    let raw = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC, 0) };
    if raw < 0 {
        return None;
    }
    let fd = unsafe { OwnedFd::from_raw_fd(raw) };
    let len = (std::mem::offset_of!(libc::sockaddr_un, sun_path) + bytes.len() + 1) as libc::socklen_t;
    if unsafe { libc::connect(raw, (&addr as *const libc::sockaddr_un).cast(), len) } < 0 {
        if std::io::Error::last_os_error().raw_os_error() != Some(libc::EINPROGRESS) {
            return None;
        }
        let mut poll = libc::pollfd { fd: raw, events: libc::POLLOUT, revents: 0 };
        if unsafe { libc::poll(&mut poll, 1, TIMEOUT.as_millis() as i32) } <= 0 {
            return None;
        }
        let mut error: libc::c_int = 0;
        let mut size = std::mem::size_of_val(&error) as libc::socklen_t;
        if unsafe { libc::getsockopt(raw, libc::SOL_SOCKET, libc::SO_ERROR, (&mut error as *mut libc::c_int).cast(), &mut size) } < 0
            || error != 0
        {
            return None;
        }
    }
    let stream = UnixStream::from(fd);
    stream.set_nonblocking(false).ok()?;
    Some(stream)
}

fn ask(path: &Path, request: &str) -> Option<String> {
    let start = Instant::now();
    let mut stream = connect(path)?;
    stream.set_read_timeout(Some(TIMEOUT)).ok()?;
    stream.set_write_timeout(Some(TIMEOUT)).ok()?;
    stream.write_all(request.as_bytes()).ok()?;
    let mut out = Vec::new();
    let mut chunk = [0; 4096];
    loop {
        if start.elapsed() > TIMEOUT * 3 {
            return None;
        }
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        if out.len() + n > REPLY_LIMIT {
            return None;
        }
        out.extend_from_slice(&chunk[..n]);
    }
    String::from_utf8(out).ok().map(|s| s.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_only_the_specific_frame_not_the_xwayland_main_or_another_instance() {
        let list = serde_json::json!([
            {"pid": 42, "xwayland": true, "class": "lemonstudio", "address": "0x100"},
            {"pid": 43, "xwayland": true, "class": class(17), "address": "0x200"},
            {"pid": 42, "xwayland": false, "class": class(17), "address": "0x300"},
            {"pid": 42, "xwayland": true, "class": class(18), "address": "0x400"},
            {"pid": 42, "xwayland": true, "class": class(17), "address": "0x500"}
        ]);
        assert_eq!(address(target(&list, 17, 42).unwrap()), Some("0x500"));
        assert!(target(&list, 99, 42).is_none());
        assert!(address(&serde_json::json!({"address": "0x1'; anything"})).is_none());
    }

    #[test]
    fn ipc_times_out_and_limits_replies_without_accessing_the_compositor() {
        use std::os::unix::net::UnixListener;
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!("hypr-ipc-{}-{}.sock", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        let listener = UnixListener::bind(&path).unwrap();
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 16];
            stream.read(&mut buf).unwrap();
            std::thread::sleep(Duration::from_millis(180));
        });
        let start = Instant::now();
        assert!(ask(&path, "j/clients").is_none());
        assert!(start.elapsed() < Duration::from_millis(150));
        worker.join().unwrap();
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 16];
            stream.read(&mut buf).unwrap();
            let _ = stream.write_all(&vec![b'x'; REPLY_LIMIT + 1]);
        });
        assert!(ask(&path, "j/clients").is_none());
        worker.join().unwrap();
        let _ = std::fs::remove_file(path);
    }
}
