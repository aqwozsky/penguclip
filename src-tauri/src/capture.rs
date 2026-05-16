//! Screen capture via XDG Desktop Portal ScreenCast interface.
//!
//! Uses ashpd 0.10 async API. The portal returns PipeWire node IDs
//! after session start — these are consumed by `pw-cat` which pipes
//! raw video to stdout → FFmpeg stdin for encoding.
//!
//! ## Compatibility
//!
//! Works on all Linux DEs implementing XDG Desktop Portal:
//! GNOME, KDE, Hyprland, Sway, wlroots, X11.

use anyhow::Context;
use ashpd::desktop::{
    screencast::{CursorMode, Screencast, SourceType},
    PersistMode, Session,
};
use std::process::{Child, Command, Stdio};

/// Result of starting a screen capture session.
#[derive(Debug, Clone)]
pub struct CaptureSession {
    /// PipeWire stream node IDs for frame capture.
    pub pipewire_node_ids: Vec<u32>,
}

/// Hardware encoder type detected on this system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderType {
    Vaapi,
    Nvenc,
    Software,
}

impl EncoderType {
    pub fn detect() -> Self {
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .arg("--query-gpu=name")
            .arg("--format=csv,noheader")
            .output()
        {
            if output.status.success() && !output.stdout.is_empty() {
                log::info!("NVIDIA GPU detected — using NVENC");
                return EncoderType::Nvenc;
            }
        }

        if let Ok(output) = std::process::Command::new("vainfo").output() {
            if output.status.success() {
                log::info!("VA-API detected — using VA-API encoding");
                return EncoderType::Vaapi;
            }
        }

        log::warn!("No hardware encoder detected — falling back to libx264");
        EncoderType::Software
    }

    pub fn h264_encoder(&self) -> &'static str {
        match self {
            EncoderType::Vaapi => "h264_vaapi",
            EncoderType::Nvenc => "h264_nvenc",
            EncoderType::Software => "libx264",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            EncoderType::Vaapi => "VA-API (Intel/AMD)",
            EncoderType::Nvenc => "NVENC (NVIDIA)",
            EncoderType::Software => "Software (libx264)",
        }
    }
}

/// Start a screen capture session via the XDG Desktop Portal.
///
/// Shows a system dialog for source selection (monitor/window).
/// Returns PipeWire node IDs for frame capture.
///
/// The returned `Session` must be kept alive for the duration of
/// recording. Call `session.close()` to clean up.
pub async fn start_capture_session() -> anyhow::Result<(CaptureSession, Session<'static, Screencast<'static>>)> {
    log::info!("Starting screen capture session via XDG Desktop Portal");

    let screencast = Screencast::new()
        .await
        .context("Failed to create Screencast portal proxy — is xdg-desktop-portal running?")?;

    // Step 1: Create the session. The Session IS the handle.
    let session = screencast
        .create_session()
        .await
        .context("Failed to create ScreenCast session")?;

    log::info!("ScreenCast session created");

    // Step 2: Select sources (monitor/window).
    screencast
        .select_sources(
            &session,
            CursorMode::Hidden,
            SourceType::Monitor | SourceType::Window,
            false, // single source (not multiple)
            None,  // no restore token
            PersistMode::DoNot,
        )
        .await
        .context("Failed to select capture sources (user may have cancelled)")?;

    log::info!("Source selection confirmed");

    // Step 3: Start the session — returns PipeWire stream info.
    let start_response = screencast
        .start(&session, None)
        .await
        .context("Failed to start ScreenCast session")?;

    let streams = start_response.response()?;
    let pipewire_node_ids: Vec<u32> = streams
        .streams()
        .iter()
        .map(|s| s.pipe_wire_node_id())
        .collect();

    log::info!(
        "ScreenCast started — {} PipeWire node(s): {:?}",
        pipewire_node_ids.len(),
        pipewire_node_ids
    );

    Ok((
        CaptureSession {
            pipewire_node_ids,
        },
        session,
    ))
}

/// Start frame capture via `pw-cat` subprocess.
///
/// Pipes raw RGBA video frames to stdout, to be connected to
/// FFmpeg's stdin for encoding.
pub fn start_frame_capture(
    node_id: u32,
    fps: u32,
) -> anyhow::Result<Child> {
    log::info!(
        "Starting pw-cat frame capture: node={}, @ {}fps",
        node_id,
        fps
    );

    Command::new("pw-cat")
        .args([
            "--record",
            "--target",
            &node_id.to_string(),
            "--media-type",
            "Video",
            "--format",
            "RGBA",
            "--rate",
            &fps.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start pw-cat. Is pipewire installed?")
}
