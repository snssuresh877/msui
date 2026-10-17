use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrightnessInfo {
    pub percent: u8,
}

impl Default for BrightnessInfo {
    fn default() -> Self {
        Self { percent: 100 }
    }
}

pub struct BrightnessMonitor {
    backlight_path: Option<PathBuf>,
}

impl BrightnessMonitor {
    pub fn new() -> Self {
        let backlight_path = Self::find_backlight_path();
        Self { backlight_path }
    }

    fn find_backlight_path() -> Option<PathBuf> {
        let base = Path::new("/sys/class/backlight");
        if !base.exists() {
            return None;
        }

        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                return Some(entry.path());
            }
        }
        None
    }

    pub fn read(&self) -> Option<BrightnessInfo> {
        let path = self.backlight_path.as_ref()?;
        let curr_str = fs::read_to_string(path.join("brightness")).ok()?;
        let max_str = fs::read_to_string(path.join("max_brightness")).ok()?;

        let current: u64 = curr_str.trim().parse().ok()?;
        let max: u64 = max_str.trim().parse().ok()?;

        if max == 0 {
            return None;
        }

        let percent = ((current as f64 / max as f64) * 100.0).round() as u8;
        Some(BrightnessInfo { percent })
    }

    pub fn increase(delta: u8) {
        let arg = format!("+{delta}%");
        let _ = Command::new("brightnessctl")
            .args(["set", &arg])
            .spawn();
    }

    pub fn decrease(delta: u8) {
        let arg = format!("{delta}%-");
        let _ = Command::new("brightnessctl")
            .args(["set", &arg])
            .spawn();
    }

    pub fn set(percent: u8) {
        let arg = format!("{percent}%");
        let _ = Command::new("brightnessctl")
            .args(["set", &arg])
            .spawn();
    }
}

impl Default for BrightnessMonitor {
    fn default() -> Self {
        Self::new()
    }
}
