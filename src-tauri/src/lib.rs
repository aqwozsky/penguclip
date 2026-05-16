//! Penguclip — High-performance background clipping for Linux.

mod capture;
mod config;
mod encoder;
mod hotkey;
pub mod ipc;
mod notify;
mod ring_buffer;

use capture::EncoderType;
use config::{AppConfig, ClipMode, RecordingFps, VideoQuality};
use ring_buffer::RingBuffer;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::{Mutex, RwLock};

// ─── App State ────────────────────────────────────────────────────

pub struct AppState {
    pub config: RwLock<AppConfig>,
    pub ring_buffer: Mutex<Option<RingBuffer>>,
    pub recording: AtomicBool,
    pub encoder_type: Mutex<Option<EncoderType>>,
    pub buffer_dir: Mutex<Option<PathBuf>>,
    pub hotkey_combo: RwLock<Option<hotkey::HotkeyCombo>>,
    /// Handle to the active FFmpeg/pw-cat subprocesses for cleanup.
    pub capture_handles: Mutex<Vec<std::process::Child>>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: RwLock::new(config),
            ring_buffer: Mutex::new(None),
            recording: AtomicBool::new(false),
            encoder_type: Mutex::new(None),
            buffer_dir: Mutex::new(None),
            hotkey_combo: RwLock::new(None),
            capture_handles: Mutex::new(vec![]),
        }
    }
}

// ─── Tauri Commands ────────────────────────────────────────────────

#[tauri::command]
async fn get_config(state: tauri::State<'_, AppState>) -> Result<Option<AppConfig>, String> {
    let config = state.config.read().await;
    Ok(Some(config.clone()))
}

#[tauri::command]
async fn save_config(
    state: tauri::State<'_, AppState>,
    output_folder: String,
    recording_fps: String,
    video_quality: String,
    clip_duration_secs: u32,
    max_recording_secs: u32,
    hotkey: String,
    clip_mode: String,
    app_filters: Vec<String>,
) -> Result<AppConfig, String> {
    let output_folder = AppConfig::expand_tilde(&output_folder);
    std::fs::create_dir_all(&output_folder)
        .map_err(|e| format!("Failed to create output folder: {}", e))?;

    let fps = match recording_fps.as_str() {
        "fps30" => RecordingFps::Fps30,
        "fps60" => RecordingFps::Fps60,
        "fps120" => RecordingFps::Fps120,
        _ => return Err(format!("Invalid FPS: {}", recording_fps)),
    };

    let quality = match video_quality.as_str() {
        "low" => VideoQuality::Low,
        "medium" => VideoQuality::Medium,
        "high" => VideoQuality::High,
        _ => return Err(format!("Invalid quality: {}", video_quality)),
    };

    let mode = match clip_mode.as_str() {
        "anything" => ClipMode::Anything,
        "games" => ClipMode::Games,
        "apps" => ClipMode::Apps,
        _ => return Err(format!("Invalid clip mode: {}", clip_mode)),
    };

    let mut config = AppConfig {
        output_folder,
        recording_fps: fps,
        video_quality: quality,
        clip_duration_secs,
        max_recording_secs,
        hotkey,
        clip_mode: mode,
        app_filters,
        setup_complete: true,
    };

    config.save().map_err(|e| format!("Failed to save config: {}", e))?;

    let mut state_config = state.config.write().await;
    *state_config = config.clone();

    log::info!("Configuration saved");
    Ok(config)
}

#[tauri::command]
async fn detect_encoder(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let encoder_type = EncoderType::detect();
    let label = encoder_type.label().to_string();
    *state.encoder_type.lock().await = Some(encoder_type);
    Ok(label)
}

#[tauri::command]
async fn start_recording(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    if state.recording.load(Ordering::Relaxed) {
        return Err("Recording is already active".to_string());
    }

    let config = state.config.read().await.clone();
    let encoder_type = state.encoder_type.lock().await.unwrap_or(EncoderType::Software);

    // Create buffer directory inside ~/.penguclip/buffer/
    let app_dir = AppConfig::app_dir();
    let buffer_dir = app_dir.join("buffer").join(std::process::id().to_string());
    std::fs::create_dir_all(&buffer_dir)
        .map_err(|e| format!("Failed to create buffer dir: {}", e))?;

    // Initialize ring buffer
    let rb = RingBuffer::new(buffer_dir.clone(), 2, config.clip_duration_secs * 2);

    // Start FFmpeg encoder with x11grab (captures screen directly)
    let _encoder_session = encoder::start_encoding(
        encoder_type,
        config.recording_fps,
        config.video_quality,
        buffer_dir.clone(),
    )
    .map_err(|e| format!("Failed to start encoder: {}", e))?;

    log::info!(
        "Recording started — encoder: {}, screen capture via x11grab",
        encoder_type.label(),
    );

    // Store state
    *state.ring_buffer.lock().await = Some(rb);
    *state.buffer_dir.lock().await = Some(buffer_dir.clone());
    state.recording.store(true, Ordering::Relaxed);

    // Auto-stop timer if max_recording_secs is set
    if config.max_recording_secs > 0 {
        let app_handle_stop = app_handle.clone();
        let max_secs = config.max_recording_secs;
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(max_secs as u64)).await;
            log::info!("Max recording time reached ({}s) — auto-stopping", max_secs);
            let _ = app_handle_stop.emit("auto-stop-recording", ());
        });
    }

    app_handle
        .emit("recording-status", serde_json::json!({
            "recording": true,
            "encoder": encoder_type.label(),
            "buffer_dir": buffer_dir.to_string_lossy(),
        }))
        .map_err(|e| e.to_string())?;

    Ok(format!("Recording started with {} encoder", encoder_type.label()))
}

#[tauri::command]
async fn stop_recording(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    if !state.recording.load(Ordering::Relaxed) {
        return Err("Recording is not active".to_string());
    }

    state.recording.store(false, Ordering::Relaxed);

    // Kill capture subprocesses
    let mut handles = state.capture_handles.lock().await;
    for mut child in handles.drain(..) {
        let _ = child.kill();
        let _ = child.wait();
    }

    if let Some(rb) = state.ring_buffer.lock().await.take() {
        log::info!("Ring buffer stopped ({} segments)", rb.len());
    }

    if let Some(dir) = state.buffer_dir.lock().await.take() {
        log::info!("Cleaning up buffer directory: {}", dir.display());
        let _ = std::fs::remove_dir_all(&dir);
    }

    app_handle
        .emit("recording-status", serde_json::json!({ "recording": false }))
        .map_err(|e| e.to_string())?;

    Ok("Recording stopped".to_string())
}

#[tauri::command]
async fn save_clip(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let config = state.config.read().await.clone();

    let mut ring_buffer = state.ring_buffer.lock().await;
    let rb = ring_buffer.as_mut().ok_or("No active recording — start recording first. Press ● Start Recording then try again.")?;

    // Scan for segments
    rb.scan().map_err(|e| format!("{}", e))?;

    if rb.len() == 0 {
        return Err("Buffer is empty. Recording needs to run for at least a few seconds before clips can be saved. Wait and try again.".to_string());
    }

    let output_path = ring_buffer::save_clip(
        rb,
        &config.output_folder,
        config.clip_duration_secs,
    )
    .map_err(|e| format!("Failed to save clip: {}", e))?;

    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
async fn get_status(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let config = state.config.read().await;
    let recording = state.recording.load(Ordering::Relaxed);
    let encoder_type = state.encoder_type.lock().await;

    Ok(serde_json::json!({
        "recording": recording,
        "encoder": encoder_type.map(|e| e.label()),
        "hotkey": config.hotkey,
        "output_folder": config.output_folder.to_string_lossy(),
        "clip_duration_secs": config.clip_duration_secs,
        "clip_mode": config.clip_mode,
        "max_recording_secs": config.max_recording_secs,
        "setup_complete": config.setup_complete,
    }))
}

#[tauri::command]
async fn open_output_folder(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let config = state.config.read().await;
    let path = config.output_folder.to_string_lossy().to_string();
    std::process::Command::new("xdg-open").arg(&path).spawn()
        .map_err(|e| format!("Failed to open folder: {}", e))?;
    Ok(())
}

// ─── Clip Management ─────────────────────────────────────────────

#[derive(serde::Serialize, Clone)]
struct ClipEntry {
    name: String,
    path: String,
    size_bytes: u64,
    modified: String,
}

#[tauri::command]
async fn list_clips(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ClipEntry>, String> {
    let config = state.config.read().await;
    let folder = &config.output_folder;
    if !folder.exists() {
        return Ok(vec![]);
    }

    let mut clips: Vec<ClipEntry> = std::fs::read_dir(folder)
        .map_err(|e| format!("Failed to read output folder: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map(|ext| ext == "mp4").unwrap_or(false))
        .filter_map(|entry| {
            let path = entry.path();
            let metadata = entry.metadata().ok()?;
            let modified = metadata.modified().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| {
                    chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                })
                .unwrap_or_else(|| "unknown".to_string());
            Some(ClipEntry {
                name: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                path: path.to_string_lossy().to_string(),
                size_bytes: metadata.len(),
                modified,
            })
        })
        .collect();

    clips.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(clips)
}

#[tauri::command]
async fn delete_clip(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err("File not found".to_string());
    }
    std::fs::remove_file(&p).map_err(|e| format!("Failed to delete clip: {}", e))?;
    log::info!("Deleted clip: {}", path);
    Ok(())
}

#[tauri::command]
async fn trim_clip(
    path: String,
    start_secs: f64,
    end_secs: f64,
) -> Result<String, String> {
    let input = PathBuf::from(&path);
    if !input.exists() {
        return Err("Source clip not found".to_string());
    }
    let stem = input.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let parent = input.parent().unwrap_or(std::path::Path::new("."));
    let output = parent.join(format!("{}_trimmed.mp4", stem));

    let result = std::process::Command::new("ffmpeg")
        .args(["-ss", &start_secs.to_string(), "-to", &end_secs.to_string(),
               "-i", &path, "-c", "copy", "-avoid_negative_ts", "make_zero", "-y",
               &format!("{}", output.display())])
        .args(["-hide_banner", "-loglevel", "error"])
        .output()
        .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

    if !result.status.success() {
        return Err(format!("FFmpeg trim failed: {}", String::from_utf8_lossy(&result.stderr)));
    }
    Ok(output.to_string_lossy().to_string())
}

#[tauri::command]
async fn open_file(path: String) -> Result<(), String> {
    std::process::Command::new("xdg-open").arg(&path).spawn()
        .map_err(|e| format!("Failed to open file: {}", e))?;
    Ok(())
}

// ─── Settings ────────────────────────────────────────────────────

#[tauri::command]
async fn update_settings(
    state: tauri::State<'_, AppState>,
    output_folder: Option<String>,
    recording_fps: Option<String>,
    video_quality: Option<String>,
    clip_duration_secs: Option<u32>,
    max_recording_secs: Option<u32>,
    hotkey: Option<String>,
    clip_mode: Option<String>,
    app_filters: Option<Vec<String>>,
) -> Result<AppConfig, String> {
    let mut config = state.config.read().await.clone();

    if let Some(folder) = output_folder {
        config.output_folder = AppConfig::expand_tilde(&folder);
        std::fs::create_dir_all(&config.output_folder)
            .map_err(|e| format!("Failed to create output folder: {}", e))?;
    }
    if let Some(s) = recording_fps {
        config.recording_fps = match s.as_str() {
            "fps30" => RecordingFps::Fps30,
            "fps60" => RecordingFps::Fps60,
            "fps120" => RecordingFps::Fps120,
            _ => return Err(format!("Invalid FPS: {}", s)),
        };
    }
    if let Some(s) = video_quality {
        config.video_quality = match s.as_str() {
            "low" => VideoQuality::Low, "medium" => VideoQuality::Medium, "high" => VideoQuality::High,
            _ => return Err(format!("Invalid quality: {}", s)),
        };
    }
    if let Some(d) = clip_duration_secs { config.clip_duration_secs = d; }
    if let Some(d) = max_recording_secs { config.max_recording_secs = d; }
    if let Some(h) = hotkey { config.hotkey = h; }
    if let Some(m) = clip_mode {
        config.clip_mode = match m.as_str() {
            "anything" => ClipMode::Anything,
            "games" => ClipMode::Games,
            "apps" => ClipMode::Apps,
            _ => return Err(format!("Invalid clip mode: {}", m)),
        };
    }
    if let Some(f) = app_filters { config.app_filters = f; }

    config.save().map_err(|e| format!("Failed to save settings: {}", e))?;
    let mut state_config = state.config.write().await;
    *state_config = config.clone();
    log::info!("Settings updated");
    Ok(config)
}

// ─── Window / Game Detection ──────────────────────────────────────

/// A running window entry for app selection.
#[derive(serde::Serialize, Clone)]
struct WindowEntry {
    id: String,
    title: String,
    class: String,
}

/// List currently visible windows (uses xdotool or wmctrl).
#[tauri::command]
async fn list_windows() -> Result<Vec<WindowEntry>, String> {
    // Try xdotool first
    let output = std::process::Command::new("xdotool")
        .args(["search", "--onlyvisible", "--name", ""])
        .output();

    let ids = match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
        }
        _ => vec![],
    };

    if ids.is_empty() {
        // Try wmctrl as fallback
        let output = std::process::Command::new("wmctrl")
            .args(["-l"])
            .output()
            .map_err(|_| "Neither xdotool nor wmctrl found. Install one: sudo pacman -S xdotool".to_string())?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut windows = vec![];
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                windows.push(WindowEntry {
                    id: parts[0].to_string(),
                    title: parts[3..].join(" "),
                    class: parts.get(2).unwrap_or(&"?").to_string(),
                });
            }
        }
        return Ok(windows);
    }

    let mut windows = vec![];
    for id in &ids {
        let title = std::process::Command::new("xdotool")
            .args(["getwindowname", id])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();

        let class = std::process::Command::new("xprop")
            .args(["-id", id, "WM_CLASS"])
            .output()
            .ok()
            .and_then(|o| {
                let s = String::from_utf8_lossy(&o.stdout);
                s.split('"').nth(3).map(|c| c.to_string())
            })
            .unwrap_or_default();

        if !title.is_empty() && title != "Penguclip" {
            windows.push(WindowEntry { id: id.clone(), title, class });
        }
    }
    Ok(windows)
}

/// Check if a game is likely running (fullscreen + known gaming processes).
#[tauri::command]
async fn detect_game() -> Result<bool, String> {
    // Simple heuristics: check for fullscreen windows
    // and known game process names
    let known_games = [
        "csgo", "cs2", "dota2", "minecraft", "java", "valorant", "overwatch",
        "rainbow", "rocketleague", "fortnite", "apex", "pubg", "rust",
        "steam", "factorio", "terraria", "stardew", "witcher", "cyberpunk",
        "eldenring", "baldur", "skyrim", "fallout", "doom", "quake", "halo",
        "league", "wine", "proton", "gamescope",
    ];

    // Check running processes
    if let Ok(output) = std::process::Command::new("pgrep")
        .args(["-f", &known_games.join("|")])
        .output()
    {
        if output.status.success() && !output.stdout.is_empty() {
            return Ok(true);
        }
    }

    // Check for fullscreen window (X11)
    if let Ok(output) = std::process::Command::new("xdotool")
        .args(["getactivewindow", "windowsize", "--usehints"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        log::debug!("Active window size: {}", stdout.trim());
    }

    Ok(false)
}

// ─── Tauri App Setup ───────────────────────────────────────────────

/// Shared clip-save logic used by both hotkey and IPC trigger.
async fn do_clip_save(app_handle: &tauri::AppHandle, source: &str) {
    log::info!("Clip trigger from: {}", source);
    let state: tauri::State<'_, AppState> = app_handle.state();
    let config = state.config.read().await.clone();

    // Check clip mode
    if config.clip_mode == ClipMode::Games {
        match detect_game().await {
            Ok(false) => {
                log::debug!("No game detected — skipping");
                return;
            }
            Err(_) => {}
            _ => {}
        }
    }

    let mut ring_buffer = state.ring_buffer.lock().await;
    if let Some(rb) = ring_buffer.as_mut() {
        match ring_buffer::save_clip(rb, &config.output_folder, config.clip_duration_secs) {
            Ok(path) => {
                let path_str = path.to_string_lossy().to_string();
                log::info!("Clip saved: {}", path_str);
                notify::clip_saved(&path_str);
                let _ = app_handle.emit("clip-saved", serde_json::json!({
                    "path": path_str,
                }));
            }
            Err(e) => {
                let err_str = e.to_string();
                log::error!("Failed to save clip: {}", err_str);
                notify::clip_failed(&err_str);
                let _ = app_handle.emit("clip-error", serde_json::json!({ "error": err_str }));
            }
        }
    } else {
        log::warn!("Trigger received but no active recording");
        notify::clip_failed("No active recording. Start recording first.");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    log::info!("Penguclip starting...");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize ~/.penguclip/ directory structure
            if let Err(e) = AppConfig::init_app_dir() {
                log::warn!("Failed to create .penguclip directory: {}", e);
            }

            // ── System Tray ──────────────────────────────────────
            use tauri::{
                menu::{MenuBuilder, MenuItemBuilder},
                tray::TrayIconBuilder,
            };

            let start_item = MenuItemBuilder::with_id("start", "● Start Recording").build(app)?;
            let stop_item = MenuItemBuilder::with_id("stop", "■ Stop Recording").build(app)?;
            let save_item = MenuItemBuilder::with_id("save", "Save Clip").build(app)?;
            let folder_item = MenuItemBuilder::with_id("folder", "Open Clips Folder").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&start_item)
                .item(&stop_item)
                .item(&save_item)
                .separator()
                .item(&folder_item)
                .separator()
                .item(&quit_item)
                .build()?;

            // Use a simple 32x32 RGBA icon (filled circle = penguin placeholder)
            let mut icon_pixels = vec![0u8; 32 * 32 * 4];
            let center = 16.0;
            for y in 0..32 {
                for x in 0..32 {
                    let dx = x as f32 - center;
                    let dy = y as f32 - center;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let idx = (y * 32 + x) * 4;
                    if dist < 14.0 {
                        // White body
                        icon_pixels[idx] = 255;
                        icon_pixels[idx + 1] = 255;
                        icon_pixels[idx + 2] = 255;
                        icon_pixels[idx + 3] = 255;
                    } else if dist < 15.0 {
                        // Dark ring
                        icon_pixels[idx] = 10;
                        icon_pixels[idx + 1] = 10;
                        icon_pixels[idx + 2] = 10;
                        icon_pixels[idx + 3] = 255;
                    }
                    // else transparent
                }
            }
            let icon = tauri::image::Image::new(&icon_pixels, 32, 32);

            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .tooltip("Penguclip")
                .on_menu_event(move |app_handle, event| {
                    match event.id().as_ref() {
                        "start" => {
                            let ah = app_handle.clone();
                            let state = ah.state::<AppState>();
                            if !state.recording.load(Ordering::Relaxed) {
                                tauri::async_runtime::spawn(async move {
                                    let result = start_recording(
                                        ah.state::<AppState>(),
                                        ah.clone(),
                                    ).await;
                                    if let Err(e) = result {
                                        notify::clip_failed(&e);
                                    }
                                });
                            }
                        }
                        "stop" => {
                            let ah = app_handle.clone();
                            let state = ah.state::<AppState>();
                            if state.recording.load(Ordering::Relaxed) {
                                tauri::async_runtime::spawn(async move {
                                    let _ = stop_recording(
                                        ah.state::<AppState>(),
                                        ah.clone(),
                                    ).await;
                                });
                            }
                        }
                        "save" => {
                            let ah = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                do_clip_save(&ah, "tray").await;
                            });
                        }
                        "folder" => {
                            let _ = std::process::Command::new("xdg-open")
                                .arg(AppConfig::app_dir().join("clips"))
                                .spawn();
                        }
                        "quit" => {
                            app_handle.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // ── Prevent close = minimize to tray ─────────────────
            let window = app.get_webview_window("main").unwrap();
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    window_clone.hide().ok();
                }
            });

            let config = AppConfig::load_or_default()
                .expect("Failed to load configuration");

            log::info!(
                "Config loaded — setup_complete: {}, output: {}, mode: {:?}",
                config.setup_complete,
                config.output_folder.display(),
                config.clip_mode,
            );

            let state = AppState::new(config);
            let encoder_type = EncoderType::detect();
            log::info!("Detected encoder: {}", encoder_type.label());

            let app_handle = app.handle().clone();

            tauri::async_runtime::block_on(async {
                *state.encoder_type.lock().await = Some(encoder_type);
            });

            let config = tauri::async_runtime::block_on(async {
                state.config.read().await.clone()
            });

            match hotkey::HotkeyCombo::parse(&config.hotkey) {
                Ok(combo) => {
                    log::info!("Hotkey listener configured: {:?}", combo);
                    let running = Arc::new(AtomicBool::new(true));
                    let mut hotkey_rx = hotkey::start_hotkey_listener(combo.clone(), running.clone());

                    tauri::async_runtime::block_on(async {
                        *state.hotkey_combo.write().await = Some(combo);
                    });

                    let app_handle_clone = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        while let Some(trigger) = hotkey_rx.recv().await {
                            do_clip_save(&app_handle_clone, "hotkey").await;
                        }
                    });

                    // Start IPC listener for Wayland (penguclip --trigger-clip)
                    let mut ipc_rx = ipc::start_ipc_listener(running.clone());
                    let app_handle_ipc = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        while ipc_rx.recv().await.is_some() {
                            do_clip_save(&app_handle_ipc, "trigger-clip").await;
                        }
                    });

                    app.manage(running);
                }
                Err(e) => {
                    log::error!("Failed to parse hotkey '{}': {} — hotkey disabled", config.hotkey, e);
                    // Still start IPC even if hotkey parse fails
                    let running = Arc::new(AtomicBool::new(true));
                    let mut ipc_rx = ipc::start_ipc_listener(running.clone());
                    let app_handle_ipc = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        while ipc_rx.recv().await.is_some() {
                            do_clip_save(&app_handle_ipc, "trigger-clip").await;
                        }
                    });
                    app.manage(running);
                }
            }

            app.manage(state);
            log::info!("Penguclip initialized successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config, save_config, detect_encoder, start_recording,
            stop_recording, save_clip, get_status, open_output_folder,
            list_clips, delete_clip, update_settings, trim_clip, open_file,
            list_windows, detect_game,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Penguclip");
}
