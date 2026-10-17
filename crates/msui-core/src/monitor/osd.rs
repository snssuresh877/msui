use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::audio::AudioMonitor;
use super::brightness::BrightnessMonitor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OsdKind {
    Volume,
    Brightness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsdState {
    pub kind: OsdKind,
    pub percent: u8,
    pub is_muted: bool,
    pub timestamp_ms: u64,
}

pub struct OsdManager;

impl OsdManager {
    fn cache_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("XDG_CACHE_HOME") {
            PathBuf::from(dir).join("msui")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".cache").join("msui")
        } else {
            PathBuf::from("/tmp/msui")
        }
    }

    pub fn state_path() -> PathBuf {
        Self::cache_dir().join("osd_state.json")
    }

    pub fn pid_path() -> PathBuf {
        Self::cache_dir().join("osd.pid")
    }

    pub fn current_time_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn read_state() -> Option<OsdState> {
        let content = fs::read_to_string(Self::state_path()).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn write_state(state: &OsdState) {
        let dir = Self::cache_dir();
        let _ = fs::create_dir_all(&dir);
        if let Ok(json) = serde_json::to_string(state) {
            let _ = fs::write(Self::state_path(), json);
        }
    }

    pub fn is_osd_running() -> bool {
        let pid_path = Self::pid_path();
        if pid_path.exists() {
            if let Ok(content) = fs::read_to_string(&pid_path) {
                if let Ok(pid) = content.trim().parse::<i32>() {
                    let proc_path = format!("/proc/{}", pid);
                    if Path::new(&proc_path).exists() {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn record_osd_pid(pid: u32) {
        let dir = Self::cache_dir();
        let _ = fs::create_dir_all(&dir);
        let _ = fs::write(Self::pid_path(), pid.to_string());
    }

    pub fn remove_osd_pid() {
        let _ = fs::remove_file(Self::pid_path());
    }

    fn ensure_osd_window() {
        if !Self::is_osd_running() {
            if let Ok(exe) = std::env::current_exe() {
                if let Ok(mut child) = std::process::Command::new(exe).arg("osd-window").spawn() {
                    std::thread::spawn(move || {
                        let _ = child.wait();
                    });
                }
            }
        }
    }

    pub fn trigger_volume(action: &str) {
        match action.to_lowercase().as_str() {
            "up" | "+" => {
                AudioMonitor::increase_volume(5);
            }
            "down" | "-" => {
                AudioMonitor::decrease_volume(5);
            }
            "mute" | "toggle-mute" => {
                AudioMonitor::toggle_mute();
            }
            val if val.parse::<u8>().is_ok() => {
                let v = val.parse::<u8>().unwrap();
                AudioMonitor::set_volume(v);
            }
            _ => {
                // If just querying or show current
            }
        }

        let audio = AudioMonitor::read();
        let state = OsdState {
            kind: OsdKind::Volume,
            percent: audio.volume_percent.min(100),
            is_muted: audio.is_muted,
            timestamp_ms: Self::current_time_ms(),
        };

        Self::write_state(&state);
        Self::ensure_osd_window();
    }

    pub fn trigger_brightness(action: &str) {
        match action.to_lowercase().as_str() {
            "up" | "+" => {
                BrightnessMonitor::increase(5);
            }
            "down" | "-" => {
                BrightnessMonitor::decrease(5);
            }
            val if val.parse::<u8>().is_ok() => {
                let v = val.parse::<u8>().unwrap();
                BrightnessMonitor::set(v);
            }
            _ => {
                // If just querying or show current
            }
        }

        let bright_mon = BrightnessMonitor::new();
        let bright = bright_mon.read().map(|b| b.percent).unwrap_or(50);
        let state = OsdState {
            kind: OsdKind::Brightness,
            percent: bright,
            is_muted: false,
            timestamp_ms: Self::current_time_ms(),
        };

        Self::write_state(&state);
        Self::ensure_osd_window();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osd_state_serde() {
        let state = OsdState {
            kind: OsdKind::Volume,
            percent: 75,
            is_muted: false,
            timestamp_ms: 1700000000,
        };

        let json = serde_json::to_string(&state).unwrap();
        let decoded: OsdState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, decoded);
    }
}
