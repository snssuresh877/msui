use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioInfo {
    pub volume_percent: u8,
    pub is_muted: bool,
}

impl Default for AudioInfo {
    fn default() -> Self {
        Self {
            volume_percent: 50,
            is_muted: false,
        }
    }
}

pub struct AudioMonitor;

impl AudioMonitor {
    pub fn read() -> AudioInfo {
        let volume_percent = Self::read_volume().unwrap_or(0);
        let is_muted = Self::read_mute().unwrap_or(false);

        AudioInfo {
            volume_percent,
            is_muted,
        }
    }

    fn read_volume() -> Option<u8> {
        let output = Command::new("pactl")
            .args(["get-sink-volume", "@DEFAULT_SINK@"])
            .output()
            .ok()?;

        let text = String::from_utf8_lossy(&output.stdout);
        // Find percentage like "42%"
        for part in text.split('/') {
            let trimmed = part.trim();
            if trimmed.ends_with('%') {
                let num_str = trimmed.trim_end_matches('%').trim();
                if let Ok(vol) = num_str.parse::<u8>() {
                    return Some(vol);
                }
            }
        }
        None
    }

    fn read_mute() -> Option<bool> {
        let output = Command::new("pactl")
            .args(["get-sink-mute", "@DEFAULT_SINK@"])
            .output()
            .ok()?;

        let text = String::from_utf8_lossy(&output.stdout);
        Some(text.to_lowercase().contains("mute: yes"))
    }

    pub fn set_volume(percent: u8) {
        let pct_str = format!("{percent}%");
        let _ = Command::new("pactl")
            .args(["set-sink-volume", "@DEFAULT_SINK@", &pct_str])
            .spawn();
    }

    pub fn increase_volume(delta: u8) {
        let delta_str = format!("+{delta}%");
        let _ = Command::new("pactl")
            .args(["set-sink-volume", "@DEFAULT_SINK@", &delta_str])
            .spawn();
    }

    pub fn decrease_volume(delta: u8) {
        let delta_str = format!("-{delta}%");
        let _ = Command::new("pactl")
            .args(["set-sink-volume", "@DEFAULT_SINK@", &delta_str])
            .spawn();
    }

    pub fn toggle_mute() {
        let _ = Command::new("pactl")
            .args(["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
            .spawn();
    }
}
