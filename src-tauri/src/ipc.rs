//! Unix domain socket IPC for Wayland hotkey support.
//!
//! On Wayland, global hotkeys via rdev/XRecord don't work.
//! Instead, the user binds a compositor shortcut that runs
//! `penguclip --trigger-clip`, which sends a message to the
//! running instance via a Unix socket.
//!
//! Hyprland config example:
//!   bind = $mainMod, R, exec, penguclip --trigger-clip

use anyhow::Context;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Path to the IPC socket.
pub fn socket_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    home.join(".penguclip").join("socket")
}

/// Start the IPC listener thread. Returns a channel that receives
/// "trigger" messages when `penguclip --trigger-clip` is called.
pub fn start_ipc_listener(
    running: Arc<AtomicBool>,
) -> mpsc::UnboundedReceiver<()> {
    let (tx, rx) = mpsc::unbounded_channel();
    let path = socket_path();

    // Remove stale socket
    let _ = std::fs::remove_file(&path);

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            log::error!("Failed to bind IPC socket at {}: {}", path.display(), e);
            return rx;
        }
    };

    log::info!("IPC socket listening at {}", path.display());

    std::thread::spawn(move || {
        listener.set_nonblocking(true).ok();

        while running.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _)) => {
                    handle_connection(stream, &tx);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(e) => {
                    log::error!("IPC accept error: {}", e);
                    break;
                }
            }
        }

        let _ = std::fs::remove_file(&path);
        log::info!("IPC socket closed");
    });

    rx
}

fn handle_connection(stream: UnixStream, tx: &mpsc::UnboundedSender<()>) {
    let reader = BufReader::new(&stream);
    for line in reader.lines() {
        match line {
            Ok(msg) if msg.trim() == "trigger" => {
                log::info!("IPC: trigger received");
                let _ = tx.send(());
            }
            Ok(msg) => {
                log::debug!("IPC: unknown message: {}", msg);
            }
            Err(e) => {
                log::error!("IPC read error: {}", e);
                break;
            }
        }
    }
}

/// Send a trigger message to the running penguclip instance.
/// Called from `penguclip --trigger-clip`.
pub fn send_trigger() -> anyhow::Result<()> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path)
        .with_context(|| format!("Failed to connect to penguclip at {}. Is penguclip running?", path.display()))?;
    stream.write_all(b"trigger\n")?;
    stream.flush()?;
    log::info!("IPC: trigger sent to running instance");
    Ok(())
}
