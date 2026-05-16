//! Ring buffer for continuous background recording.
//!
//! ## How it works
//!
//! FFmpeg writes 2-second MP4 segments to a buffer directory:
//! `seg_00001.mp4`, `seg_00002.mp4`, ..., up to `seg_NNNNN.mp4`.
//!
//! The `RingBuffer` tracks which segments exist, their order, and
//! provides methods to:
//! - Get the last N seconds worth of segments
//! - Clean up old segments to stay within the buffer window
//! - Extract and concatenate segments into a saved clip
//!
//! ## Segment File Naming
//!
//! FFmpeg's segment muxer uses 1-indexed sequential numbers that
//! never wrap (we set a very high `segment_list_size`). To avoid
//! filling the disk, `cleanup_old_segments()` removes files older
//! than the configured buffer window.

use crate::encoder;
use anyhow::Context;
use chrono::Local;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// Manages the rotating buffer of video segments.
pub struct RingBuffer {
    /// Directory where segment files are stored.
    buffer_dir: PathBuf,
    /// Queue of segment file paths, oldest first.
    segments: VecDeque<PathBuf>,
    /// Duration of each segment in seconds.
    segment_duration_secs: u32,
    /// Maximum number of segments to keep (buffer window).
    max_segments: u32,
}

impl RingBuffer {
    /// Create a new ring buffer manager.
    pub fn new(
        buffer_dir: PathBuf,
        segment_duration_secs: u32,
        buffer_window_secs: u32,
    ) -> Self {
        let max_segments = buffer_window_secs / segment_duration_secs;

        // Ensure the directory exists
        if let Err(e) = std::fs::create_dir_all(&buffer_dir) {
            log::error!(
                "Failed to create buffer directory {}: {}",
                buffer_dir.display(),
                e
            );
        }

        log::info!(
            "Ring buffer initialized: {} ({}s window, {}s segments, max {} segments)",
            buffer_dir.display(),
            buffer_window_secs,
            segment_duration_secs,
            max_segments
        );

        Self {
            buffer_dir,
            segments: VecDeque::new(),
            segment_duration_secs,
            max_segments,
        }
    }

    /// Scan the buffer directory and update the internal segment list.
    /// Call this when you need to sync with what's actually on disk.
    pub fn scan(&mut self) -> anyhow::Result<()> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(&self.buffer_dir)
            .with_context(|| {
                format!(
                    "Failed to read buffer directory: {}",
                    self.buffer_dir.display()
                )
            })?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .extension()
                        .map(|ext| ext == "mp4")
                        .unwrap_or(false)
            })
            .collect();

        // Sort by filename (which includes sequential numbers)
        files.sort();

        self.segments = VecDeque::from(files);

        log::debug!(
            "Ring buffer scan: {} segments found",
            self.segments.len()
        );

        Ok(())
    }

    /// Add a new segment file path to the buffer.
    pub fn push(&mut self, path: PathBuf) {
        self.segments.push_back(path);

        // Trim if over capacity
        while self.segments.len() > self.max_segments as usize {
            if let Some(old) = self.segments.pop_front() {
                log::debug!("Removing old segment: {}", old.display());
                let _ = std::fs::remove_file(&old);
            }
        }
    }

    /// Remove segment files beyond the buffer window from disk.
    pub fn cleanup_old_segments(&mut self) {
        while self.segments.len() > self.max_segments as usize {
            if let Some(old) = self.segments.pop_front() {
                if old.exists() {
                    log::debug!("Cleanup: removing {}", old.display());
                    let _ = std::fs::remove_file(&old);
                }
            }
        }
    }

    /// Get the last `duration_secs` worth of segment file paths,
    /// ordered oldest-to-newest for concatenation.
    ///
    /// Returns the list of segment paths that cover approximately
    /// `duration_secs` of recent footage.
    ///
    /// Skips the last segment (currently being written by FFmpeg —
    /// no moov atom yet, would fail concat).
    pub fn get_recent_segments(
        &self,
        duration_secs: u32,
    ) -> Vec<PathBuf> {
        let needed = (duration_secs / self.segment_duration_secs) as usize;
        let available = self.segments.len();

        // Need at least 2 segments — one complete + one being written
        if available < 2 {
            return vec![];
        }

        // Skip the last segment (still being written, no moov atom)
        let complete = available - 1;
        let take = needed.min(complete);
        let start = complete - take;

        self.segments
            .iter()
            .skip(start)
            .take(take)
            .cloned()
            .collect()
    }

    /// Get the total number of segments currently tracked.
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Get the buffer directory path.
    pub fn dir(&self) -> &Path {
        &self.buffer_dir
    }
}

/// Extract and save the last `duration_secs` of footage from the ring
/// buffer to the configured output folder.
///
/// The clip is named `Clip_YYYY-MM-DD_HH-MM.mp4` in the output folder.
///
/// Returns the full path to the saved clip on success.
pub fn save_clip(
    ring_buffer: &mut RingBuffer,
    output_folder: &Path,
    duration_secs: u32,
) -> anyhow::Result<PathBuf> {
    // Refresh segment list from disk
    ring_buffer.scan()?;

    let segments = ring_buffer.get_recent_segments(duration_secs);

    if segments.is_empty() {
        anyhow::bail!(
            "No segments available in buffer. Has recording been running for at least {} seconds?",
            duration_secs
        );
    }

    let now = Local::now();
    let filename = format!("Clip_{}.mp4", now.format("%Y-%m-%d_%H-%M"));
    let output_path = output_folder.join(&filename);

    // Ensure output directory exists
    std::fs::create_dir_all(output_folder).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            output_folder.display()
        )
    })?;

    log::info!(
        "Saving clip: {} ({} segments, ~{}s)",
        output_path.display(),
        segments.len(),
        duration_secs
    );

    // Concatenate segments using FFmpeg stream copy (instant, no re-encode)
    encoder::concat_segments(&segments, &output_path)?;

    // Verify the file was created
    if !output_path.exists() {
        anyhow::bail!(
            "Clip file was not created at expected path: {}",
            output_path.display()
        );
    }

    let file_size = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    log::info!(
        "Clip saved successfully: {} ({} bytes)",
        output_path.display(),
        file_size
    );

    Ok(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_get_recent() {
        let tmp = std::env::temp_dir().join("penguclip_test_ring");
        let _ = std::fs::create_dir_all(&tmp);

        let mut rb = RingBuffer::new(tmp.clone(), 2, 60);

        // Push 30 segments (60 seconds of buffer)
        for i in 1..=30 {
            rb.push(tmp.join(format!("seg_{:05}.mp4", i)));
        }

        // Get last 10 seconds = 5 segments
        let recent = rb.get_recent_segments(10);
        assert_eq!(recent.len(), 5);
        assert!(recent[0].to_str().unwrap().contains("seg_00026"));
        assert!(recent[4].to_str().unwrap().contains("seg_00030"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_ring_buffer_cleanup() {
        let tmp = std::env::temp_dir().join("penguclip_test_cleanup");
        let _ = std::fs::create_dir_all(&tmp);

        let mut rb = RingBuffer::new(tmp.clone(), 2, 20); // 10 max segments

        for i in 1..=15 {
            rb.push(tmp.join(format!("seg_{:05}.mp4", i)));
        }

        rb.cleanup_old_segments();
        assert_eq!(rb.len(), 10);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
