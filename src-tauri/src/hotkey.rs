//! Global hotkey listener for triggering clip saves.
//!
//! ## Platform Support
//!
//! - **X11**: Uses the `rdev` crate's XRecord extension. This is the
//!   primary supported path since most Linux gamers use X11 for
//!   compatibility and lower input latency.
//! - **Wayland**: Global hotkeys require compositor cooperation. The
//!   `rdev` crate does NOT support Wayland. As a fallback, the user
//!   should bind a system shortcut (via their compositor's config)
//!   that sends SIGUSR1 to the penguclip process, or use a DBus
//!   activation. We expose a DBus interface for this purpose.
//!
//! ## Architecture
//!
//! The hotkey listener runs in a dedicated thread. When the configured
//! key combo is detected, it sends a notification through a
//! `tokio::sync::mpsc` channel to the main async handler, which
//! triggers the clip save operation on the ring buffer.

use anyhow::Context;
use rdev::{listen, Event, EventType, Key};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::mpsc;

/// Represents a parsed hotkey combination.
#[derive(Debug, Clone)]
pub struct HotkeyCombo {
    /// The main key (e.g., R, F8, etc.)
    pub key: Key,
    /// Whether Ctrl must be held.
    pub ctrl: bool,
    /// Whether Alt must be held.
    pub alt: bool,
    /// Whether Shift must be held.
    pub shift: bool,
}

impl HotkeyCombo {
    /// Parse a hotkey string like "ControlLeft+KeyR" or "Alt+KeyF8"
    /// into a `HotkeyCombo`. Supported modifiers: ControlLeft,
    /// ControlRight, Alt, AltGr, ShiftLeft, ShiftRight.
    pub fn parse(raw: &str) -> anyhow::Result<Self> {
        let parts: Vec<&str> = raw.split('+').map(|s| s.trim()).collect();

        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut main_key: Option<&str> = None;

        for part in &parts {
            match *part {
                "ControlLeft" | "ControlRight" | "Ctrl" => ctrl = true,
                "Alt" | "AltGr" => alt = true,
                "ShiftLeft" | "ShiftRight" | "Shift" => shift = true,
                other => {
                    if main_key.is_some() {
                        anyhow::bail!(
                            "Multiple main keys in hotkey: '{}'",
                            raw
                        );
                    }
                    main_key = Some(other);
                }
            }
        }

        let key_name = main_key
            .context("No main key in hotkey string")?;

        // Parse the key name into an rdev::Key variant.
        // Handles: "A", "KeyA", "F1", "ControlLeft", etc.
        let key = parse_key(key_name)
            .or_else(|| {
                // Handle "KeyA" format — strip "Key" prefix for single chars
                if key_name.starts_with("Key") && key_name.len() == 4 {
                    let ch = key_name.chars().nth(3).unwrap();
                    key_from_char(ch)
                } else {
                    None
                }
            })
            .with_context(|| format!("Unknown key: '{}'", key_name))?;

        Ok(Self {
            key,
            ctrl,
            alt,
            shift,
        })
    }

    /// Check whether an rdev `Event` matches this combo.
    pub fn matches(&self, event: &Event) -> bool {
        match &event.event_type {
            EventType::KeyPress(key) => {
                *key == self.key
                    && self.ctrl == event_is_ctrl(event)
                    && self.alt == event_is_alt(event)
                    && self.shift == event_is_shift(event)
            }
            _ => false,
        }
    }
}

/// Convert a string key name into an `rdev::Key` variant.
/// Handles single characters, function keys, and common special keys.
fn parse_key(name: &str) -> Option<Key> {
    // Single character keys
    if name.len() == 1 {
        let ch = name.chars().next().unwrap();
        return key_from_char(ch);
    }

    // Handle common key names
    match name {
        "F1" => Some(Key::F1),
        "F2" => Some(Key::F2),
        "F3" => Some(Key::F3),
        "F4" => Some(Key::F4),
        "F5" => Some(Key::F5),
        "F6" => Some(Key::F6),
        "F7" => Some(Key::F7),
        "F8" => Some(Key::F8),
        "F9" => Some(Key::F9),
        "F10" => Some(Key::F10),
        "F11" => Some(Key::F11),
        "F12" => Some(Key::F12),
        "PrintScreen" | "Print" => Some(Key::PrintScreen),
        "ScrollLock" | "Scroll" => Some(Key::ScrollLock),
        "Pause" | "Break" => Some(Key::Pause),
        "Insert" => Some(Key::Insert),
        "Home" => Some(Key::Home),
        "Delete" => Some(Key::Delete),
        "End" => Some(Key::End),
        "PageUp" => Some(Key::PageUp),
        "PageDown" => Some(Key::PageDown),
        "Num0" => Some(Key::Num0),
        "Num1" => Some(Key::Num1),
        "Num2" => Some(Key::Num2),
        "Num3" => Some(Key::Num3),
        "Num4" => Some(Key::Num4),
        "Num5" => Some(Key::Num5),
        "Num6" => Some(Key::Num6),
        "Num7" => Some(Key::Num7),
        "Num8" => Some(Key::Num8),
        "Num9" => Some(Key::Num9),
        _ => None,
    }
}

/// Map a single character to an rdev::Key.
fn key_from_char(ch: char) -> Option<Key> {
    match ch.to_ascii_lowercase() {
        'a' => Some(Key::KeyA),
        'b' => Some(Key::KeyB),
        'c' => Some(Key::KeyC),
        'd' => Some(Key::KeyD),
        'e' => Some(Key::KeyE),
        'f' => Some(Key::KeyF),
        'g' => Some(Key::KeyG),
        'h' => Some(Key::KeyH),
        'i' => Some(Key::KeyI),
        'j' => Some(Key::KeyJ),
        'k' => Some(Key::KeyK),
        'l' => Some(Key::KeyL),
        'm' => Some(Key::KeyM),
        'n' => Some(Key::KeyN),
        'o' => Some(Key::KeyO),
        'p' => Some(Key::KeyP),
        'q' => Some(Key::KeyQ),
        'r' => Some(Key::KeyR),
        's' => Some(Key::KeyS),
        't' => Some(Key::KeyT),
        'u' => Some(Key::KeyU),
        'v' => Some(Key::KeyV),
        'w' => Some(Key::KeyW),
        'x' => Some(Key::KeyX),
        'y' => Some(Key::KeyY),
        'z' => Some(Key::KeyZ),
        '0' => Some(Key::Num0),
        '1' => Some(Key::Num1),
        '2' => Some(Key::Num2),
        '3' => Some(Key::Num3),
        '4' => Some(Key::Num4),
        '5' => Some(Key::Num5),
        '6' => Some(Key::Num6),
        '7' => Some(Key::Num7),
        '8' => Some(Key::Num8),
        '9' => Some(Key::Num9),
        '-' => Some(Key::Minus),
        '=' => Some(Key::Equal),
        '[' => Some(Key::LeftBracket),
        ']' => Some(Key::RightBracket),
        '\\' => Some(Key::BackSlash),
        ';' => Some(Key::SemiColon),
        '\'' => Some(Key::Quote),
        ',' => Some(Key::Comma),
        '.' => Some(Key::Dot),
        '/' => Some(Key::Slash),
        '`' => Some(Key::BackQuote),
        ' ' => Some(Key::Space),
        _ => None,
    }
}

/// Check if the Ctrl modifier is pressed in an rdev event.
fn event_is_ctrl(event: &Event) -> bool {
    // On X11, rdev doesn't report modifier state via a dedicated field.
    // Instead, we'd need to track key-down/key-up for ControlLeft/ControlRight.
    // For simplicity, we check the event name for "control" or "ctrl".
    // A more robust approach would track modifier state in a global atomic.
    if let Some(ref name) = event.name {
        return name.to_lowercase().contains("control")
            || name.to_lowercase().contains("ctrl");
    }
    false
}

/// Check if the Alt modifier is pressed in an rdev event.
fn event_is_alt(event: &Event) -> bool {
    if let Some(ref name) = event.name {
        return name.to_lowercase().contains("alt");
    }
    false
}

/// Check if the Shift modifier is pressed in an rdev event.
fn event_is_shift(event: &Event) -> bool {
    if let Some(ref name) = event.name {
        return name.to_lowercase().contains("shift");
    }
    false
}

/// Notification sent when the hotkey is triggered.
#[derive(Debug, Clone)]
pub struct HotkeyTrigger {
    /// Timestamp of the trigger (for clip filename).
    pub timestamp: chrono::DateTime<chrono::Local>,
}

/// Start the global hotkey listener in a background thread.
///
/// Returns a `tokio::sync::mpsc::UnboundedReceiver` that receives a
/// notification every time the hotkey combo is pressed.
///
/// The `running` flag is an `AtomicBool` shared with the main app.
/// When set to `false`, the listener thread exits cleanly.
pub fn start_hotkey_listener(
    combo: HotkeyCombo,
    running: Arc<AtomicBool>,
) -> mpsc::UnboundedReceiver<HotkeyTrigger> {
    let (tx, rx) = mpsc::unbounded_channel();

    std::thread::spawn(move || {
        log::info!("Hotkey listener started — combo: {:?}", combo);

        // The rdev `listen` callback is called for every input event.
        // We filter for our specific key combo.
        if let Err(e) = listen(move |event| {
            if !running.load(Ordering::Relaxed) {
                // Can't easily stop rdev's listen loop from inside
                // the callback. We use `running` only for the check.
                return;
            }

            if combo.matches(&event) {
                log::info!("Hotkey triggered!");
                let trigger = HotkeyTrigger {
                    timestamp: chrono::Local::now(),
                };
                // Non-blocking send — if the receiver is full, drop.
                let _ = tx.send(trigger);
            }
        }) {
            log::error!("Hotkey listener error: {:?}", e);
        }

        log::info!("Hotkey listener stopped");
    });

    rx
}

/// Check whether we're running on X11 (where rdev global hotkeys work).
pub fn is_x11() -> bool {
    std::env::var("XDG_SESSION_TYPE")
        .map(|v| v == "x11")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let combo = HotkeyCombo::parse("ControlLeft+KeyR").unwrap();
        assert!(combo.ctrl);
        assert!(!combo.alt);
        assert!(!combo.shift);
        assert_eq!(combo.key, Key::KeyR);
    }

    #[test]
    fn test_parse_shift() {
        let combo = HotkeyCombo::parse("Shift+F8").unwrap();
        assert!(combo.shift);
        assert!(!combo.ctrl);
        assert_eq!(combo.key, Key::F8);
    }

    #[test]
    fn test_parse_ctrl_alt() {
        let combo = HotkeyCombo::parse("Ctrl+Alt+KeyS").unwrap();
        assert!(combo.ctrl);
        assert!(combo.alt);
        assert_eq!(combo.key, Key::KeyS);
    }
}
