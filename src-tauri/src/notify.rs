//! Desktop notifications with sound for clip save/failure events.

use std::process::Command;

/// Show a success notification when a clip is saved.
pub fn clip_saved(path: &str) {
    notify(
        "Penguclip",
        &format!("Clip saved!\n{}", path),
        "dialog-information",
    );
}

/// Show a failure notification when clip save fails.
pub fn clip_failed(reason: &str) {
    notify(
        "Penguclip",
        &format!("Failed to save clip\n{}", reason),
        "dialog-error",
    );
}

/// Show a notification with sound.
fn notify(summary: &str, body: &str, icon: &str) {
    // notify-send (libnotify) — works on all Linux DEs
    let result = Command::new("notify-send")
        .args(["-i", icon, "-a", "Penguclip", summary, body])
        .output();

    match result {
        Ok(o) if o.status.success() => {
            log::info!("Notification sent: {}", summary);
        }
        Ok(o) => {
            log::warn!(
                "notify-send failed: {}",
                String::from_utf8_lossy(&o.stderr)
            );
        }
        Err(e) => {
            log::warn!("notify-send not available: {}", e);
        }
    }

    // Play sound via canberra-gtk-play or paplay
    play_sound(icon == "dialog-error");
}

fn play_sound(is_error: bool) {
    let sound = if is_error {
        "dialog-error"
    } else {
        "complete"
    };

    // Try canberra first (GTK sound theme)
    let result = Command::new("canberra-gtk-play")
        .args(["-i", sound])
        .output();

    if result.map_or(true, |o| !o.status.success()) {
        // Fall back to paplay with a simple beep
        let _ = Command::new("paplay")
            .arg(if is_error {
                "/usr/share/sounds/freedesktop/stereo/dialog-error.oga"
            } else {
                "/usr/share/sounds/freedesktop/stereo/complete.oga"
            })
            .output();
    }
}
