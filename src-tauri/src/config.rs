//! Application configuration — persisted to
//! `~/.config/dev.aqwozsky.penguclip/config.json`

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Video quality presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoQuality {
    Low,
    Medium,
    High,
}

impl VideoQuality {
    pub fn crf(&self) -> u8 {
        match self {
            VideoQuality::Low => 30,
            VideoQuality::Medium => 23,
            VideoQuality::High => 18,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            VideoQuality::Low => "Low (smaller files, faster)",
            VideoQuality::Medium => "Medium (balanced)",
            VideoQuality::High => "High (larger files, best quality)",
        }
    }

    pub fn preset(&self) -> &'static str {
        match self {
            VideoQuality::Low => "ultrafast",
            VideoQuality::Medium => "fast",
            VideoQuality::High => "medium",
        }
    }
}

/// Recording FPS options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordingFps {
    Fps30,
    Fps60,
    Fps120,
}

impl RecordingFps {
    pub fn value(&self) -> u32 {
        match self {
            RecordingFps::Fps30 => 30,
            RecordingFps::Fps60 => 60,
            RecordingFps::Fps120 => 120,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            RecordingFps::Fps30 => "30 FPS",
            RecordingFps::Fps60 => "60 FPS",
            RecordingFps::Fps120 => "120 FPS",
        }
    }
}

/// What to capture — everything, only games, or specific apps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipMode {
    /// Capture the entire display — anything on screen.
    Anything,
    /// Only capture when a game is detected (fullscreen + known process).
    Games,
    /// Only capture specific applications (user-selected windows).
    Apps,
}

impl ClipMode {
    pub fn label(&self) -> &'static str {
        match self {
            ClipMode::Anything => "Clip Anything (entire screen)",
            ClipMode::Games => "Clip Games Only",
            ClipMode::Apps => "Clip Specific Apps",
        }
    }
}

/// Main application configuration, persisted as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Absolute path to the folder where saved clips are stored.
    pub output_folder: PathBuf,

    /// Recording frames per second.
    pub recording_fps: RecordingFps,

    /// Video quality preset.
    pub video_quality: VideoQuality,

    /// How many seconds of footage to save from the ring buffer.
    pub clip_duration_secs: u32,

    /// Recording duration limit in seconds (0 = unlimited).
    pub max_recording_secs: u32,

    /// Global hotkey string.
    pub hotkey: String,

    /// What to capture.
    pub clip_mode: ClipMode,

    /// For ClipMode::Apps — list of window class names to capture.
    pub app_filters: Vec<String>,

    /// Whether the setup wizard has been completed.
    pub setup_complete: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let app_dir = home.join(".penguclip");
        let output_folder = app_dir.join("clips");

        Self {
            output_folder,
            recording_fps: RecordingFps::Fps60,
            video_quality: VideoQuality::Medium,
            clip_duration_secs: 30,
            max_recording_secs: 0,
            hotkey: "ControlLeft+KeyR".to_string(),
            clip_mode: ClipMode::Anything,
            app_filters: vec![],
            setup_complete: false,
        }
    }
}

impl AppConfig {
    /// Returns the app directory: `~/.penguclip/`
    pub fn app_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".penguclip")
    }

    /// Initialize .penguclip/ structure on first launch.
    pub fn init_app_dir() -> anyhow::Result<PathBuf> {
        let dir = Self::app_dir();
        std::fs::create_dir_all(dir.join("clips"))?;
        std::fs::create_dir_all(dir.join("buffer"))?;
        Ok(dir)
    }

    pub fn config_path() -> anyhow::Result<PathBuf> {
        Ok(Self::app_dir().join("config.json"))
    }

    pub fn load() -> anyhow::Result<Option<Self>> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(None);
        }
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;
        let config: Self = serde_json::from_str(&raw)
            .with_context(|| format!("Failed to parse config from {}", path.display()))?;
        Ok(Some(config))
    }

    pub fn load_or_default() -> anyhow::Result<Self> {
        Ok(Self::load()?.unwrap_or_default())
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let dir = Self::app_dir();
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create {}", dir.display()))?;
        let path = Self::config_path()?;
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, raw)
            .with_context(|| format!("Failed to write config to {}", path.display()))?;
        log::info!("Config saved to {}", path.display());
        Ok(())
    }
}
