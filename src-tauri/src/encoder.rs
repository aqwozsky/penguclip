//! FFmpeg subprocess management for hardware-accelerated video encoding.
//!
//! ## Architecture
//!
//! Penguclip runs an FFmpeg subprocess in the background that continuously
//! encodes the screen capture into small segment files (2 seconds each).
//! These segments form a **ring buffer** — old segments are automatically
//! deleted to keep a fixed window of history (e.g., last 60 seconds).
//!
//! When the user presses the hotkey, we concatenate the relevant segments
//! into a single MP4 file named `Clip_YYYY-MM-DD_HH-MM.mp4`.
//!
//! ## Encoding Pipeline
//!
//! ```
//! Screen Capture → FFmpeg (encode) → buffer_00001.mp4
//!                                   → buffer_00002.mp4
//!                                   → ...
//!                                   → buffer_00030.mp4
//!                                   → buffer_00001.mp4 (wrap)
//!
//! Hotkey Press:
//!   buffer_00015.mp4 → buffer_00030.mp4 → buffer_00001.mp4 → buffer_00014.mp4
//!   [concat via FFmpeg concat demuxer] → Clip_2026-05-16_14-41.mp4
//! ```
//!
//! ## Hardware Encoder Support
//!
//! - **VA-API** (Intel / AMD): `h264_vaapi` with `-vaapi_device /dev/dri/renderD128`
//! - **NVENC** (NVIDIA): `h264_nvenc` — zero-copy when used with PipeWire DMA-BUF
//! - **Software** fallback: `libx264` with ultrafast/veryfast preset

use crate::capture::EncoderType;
use crate::config::{RecordingFps, VideoQuality};
use anyhow::Context;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Detect the current screen resolution via xrandr.
fn detect_screen_size(display: &str) -> String {
    let output = Command::new("xrandr")
        .args(["-display", display, "--current"])
        .output()
        .ok();
    if let Some(o) = output {
        let stdout = String::from_utf8_lossy(&o.stdout);
        for line in stdout.lines() {
            if line.contains(" connected") || line.contains(" primary") {
                // Parse "1920x1080+0+0" from xrandr output
                if let Some(res) = line.split_whitespace().find(|w| w.contains('x') && w.chars().filter(|c| *c == 'x').count() == 1) {
                    let clean: String = res.chars().take_while(|c| c.is_ascii_digit() || *c == 'x').collect();
                    if clean.contains('x') {
                        return clean;
                    }
                }
            }
        }
    }
    // Default fallback
    "1920x1080".to_string()
}

/// An active FFmpeg encoding session.
pub struct EncoderSession {
    /// The FFmpeg child process handle.
    pub process: Child,
    /// The directory where segment files are written.
    pub buffer_dir: PathBuf,
    /// The encoder type (for logging / diagnostics).
    pub encoder_type: EncoderType,
    /// Current segment index (wraps around).
    pub segment_index: Arc<Mutex<u32>>,
}

/// Build the FFmpeg command arguments for continuous segment encoding.
///
/// The output is a series of `.mp4` segment files in `buffer_dir`,
/// named `seg_00001.mp4`, `seg_00002.mp4`, etc.
///
/// # Arguments
/// - `encoder_type` — Hardware or software encoder
/// - `fps` — Recording frames per second
/// - `quality` — Video quality preset
/// - `buffer_dir` — Directory for segment files
/// - `segment_time` — Duration of each segment in seconds (typically 2)
/// - `max_segments` — Maximum number of segments before cleanup kicks in
pub fn build_ffmpeg_command(
    encoder_type: EncoderType,
    fps: RecordingFps,
    quality: VideoQuality,
    buffer_dir: &PathBuf,
    segment_time: u32,
    _max_segments: u32,
) -> anyhow::Result<Command> {
    let seg_pattern = buffer_dir.join("seg_%05d.mp4");

    let mut cmd = Command::new("ffmpeg");

    // --- Input: x11grab (direct screen capture) ---
    // Uses X11 SHM for zero-copy capture. Works on X11 and XWayland.
    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
    let screen_size = detect_screen_size(&display);
    cmd.args([
        "-f", "x11grab",
        "-video_size", &screen_size,
        "-framerate", &fps.value().to_string(),
        "-i", &display,
        // Draw mouse cursor in recording
        "-draw_mouse", "0",
    ]);

    // --- Video encoding ---
    cmd.arg("-c:v");
    cmd.arg(encoder_type.h264_encoder());

    match encoder_type {
        EncoderType::Vaapi => {
            // VA-API needs the DRM device for HW acceleration
            cmd.args([
                "-vaapi_device", "/dev/dri/renderD128",
                "-vf", "format=nv12,hwupload",
                "-qp", &quality.crf().to_string(),
                "-preset", quality.preset(),
            ]);
        }
        EncoderType::Nvenc => {
            cmd.args([
                "-preset", "p4", // p1=fastest, p7=slowest, p4 is balanced
                "-cq", &quality.crf().to_string(),
                "-rc", "vbr",
            ]);
        }
        EncoderType::Software => {
            cmd.args([
                "-crf", &quality.crf().to_string(),
                "-preset", quality.preset(),
                // x264 tuning for screen content (lots of flat areas, text)
                "-tune", "zerolatency",
            ]);
        }
    }

    // --- Segment muxer configuration ---
    cmd.args([
        "-f", "segment",
        "-segment_time", &segment_time.to_string(),
        "-segment_format", "mp4",
        "-reset_timestamps", "1",
        "-segment_list", "/dev/null", // Don't write a segment list file
        "-segment_list_type", "flat",
        &format!("{}", seg_pattern.display()),
    ]);

    // Mute audio (we only capture video)
    cmd.arg("-an");

    // Overwrite existing segment files
    cmd.arg("-y");

    // Suppress FFmpeg banner (too noisy in logs)
    cmd.arg("-hide_banner");
    cmd.arg("-loglevel");
    cmd.arg("error");

    // Pipe stdin and capture stderr for error reporting
    cmd.stdin(Stdio::piped());
    cmd.stderr(Stdio::piped());

    Ok(cmd)
}

/// Start FFmpeg encoding to a directory of segment files.
///
/// Returns an `EncoderSession` that holds the FFmpeg process handle
/// and buffer directory reference. The caller is responsible for
/// feeding raw video frames to `process.stdin`.
pub fn start_encoding(
    encoder_type: EncoderType,
    fps: RecordingFps,
    quality: VideoQuality,
    buffer_dir: PathBuf,
) -> anyhow::Result<EncoderSession> {
    log::info!(
        "Starting FFmpeg encoder: {} @ {} FPS, quality: {:?}, buffer: {}",
        encoder_type.label(),
        fps.value(),
        quality,
        buffer_dir.display()
    );

    // Create the buffer directory
    std::fs::create_dir_all(&buffer_dir)
        .with_context(|| format!("Failed to create buffer dir: {}", buffer_dir.display()))?;

    let mut cmd = build_ffmpeg_command(
        encoder_type,
        fps,
        quality,
        &buffer_dir,
        2,  // 2-second segments
        60, // max 60 segments = 120 seconds of buffer
    )?;

    let process = cmd
        .spawn()
        .context("Failed to start FFmpeg process. Is ffmpeg installed?")?;

    log::info!(
        "FFmpeg encoder started — PID: {}",
        process.id()
    );

    Ok(EncoderSession {
        process,
        buffer_dir,
        encoder_type,
        segment_index: Arc::new(Mutex::new(0)),
    })
}

/// Stop the encoder by sending SIGTERM to the FFmpeg process,
/// then waiting for graceful shutdown.
pub fn stop_encoding(mut session: EncoderSession) -> anyhow::Result<()> {
    log::info!("Stopping FFmpeg encoder (PID: {})", session.process.id());

    // Drop stdin to signal EOF to ffmpeg
    drop(session.process.stdin.take());

    // Give FFmpeg a moment to flush buffers
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Kill the process
    if let Err(e) = session.process.kill() {
        log::warn!("Failed to kill FFmpeg process: {}", e);
    }

    // Wait for the process to exit
    let status = session.process.wait().context("Failed to wait for FFmpeg exit")?;

    log::info!(
        "FFmpeg encoder stopped — exit code: {}",
        status.code().unwrap_or(-1)
    );

    Ok(())
}

/// Concatenate segment files into a single clip using FFmpeg's concat
/// demuxer with stream copy (no re-encoding — instant).
///
/// # Arguments
/// - `segments` — Ordered list of segment file paths (oldest first)
/// - `output_path` — Full path to the output clip file
pub fn concat_segments(
    segments: &[PathBuf],
    output_path: &PathBuf,
) -> anyhow::Result<()> {
    if segments.is_empty() {
        anyhow::bail!("No segments to concatenate");
    }

    log::info!(
        "Concatenating {} segments → {}",
        segments.len(),
        output_path.display()
    );

    // Create a temporary concat list file
    let concat_list = output_path.with_extension("txt");
    let list_content: String = segments
        .iter()
        .map(|p| format!("file '{}'", p.display()))
        .collect::<Vec<_>>()
        .join("\n");

    std::fs::write(&concat_list, &list_content)
        .context("Failed to write concat list")?;

    // Run ffmpeg concat
    let output = Command::new("ffmpeg")
        .args([
            "-f", "concat",
            "-safe", "0",
            "-i", &format!("{}", concat_list.display()),
            "-c", "copy", // Stream copy — no re-encoding
            "-y",          // Overwrite output
            &format!("{}", output_path.display()),
        ])
        .args(["-hide_banner", "-loglevel", "error"])
        .output()
        .context("Failed to run ffmpeg concat")?;

    // Clean up the temp file
    let _ = std::fs::remove_file(&concat_list);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("FFmpeg concat failed: {}", stderr);
    }

    log::info!("Clip saved: {} ({} segments)", output_path.display(), segments.len());
    Ok(())
}
