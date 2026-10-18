use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DisplayMonitorInfo {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub make: String,
    pub model: String,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: f32,
    pub scale: f32,
    pub transform: u32,
    pub is_focused: bool,
    pub is_disabled: bool,
    pub available_modes: Vec<String>,
}

pub struct DisplayManager;

impl DisplayManager {
    pub fn get_primary_monitor_name() -> Option<String> {
        let monitors = Self::list_monitors();
        if let Some(m) = monitors.iter().find(|m| m.is_focused && !m.is_disabled) {
            return Some(m.name.clone());
        }
        if let Some(m) = monitors.iter().find(|m| !m.is_disabled) {
            return Some(m.name.clone());
        }

        // Direct kernel DRM fallback (e.g. before hyprctl is ready)
        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let path = entry.path();
                let status_file = path.join("status");
                if let Ok(status) = std::fs::read_to_string(status_file) {
                    if status.trim() == "connected" {
                        if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
                            // Format: card1-eDP-1 or card0-HDMI-A-1 -> eDP-1 or HDMI-A-1
                            if let Some(idx) = fname.find('-') {
                                return Some(fname[idx + 1..].to_string());
                            }
                        }
                    }
                }
            }
        }

        None
    }

    pub fn list_monitors() -> Vec<DisplayMonitorInfo> {
        let output = Command::new("hyprctl")
            .args(["monitors", "all", "-j"])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let json_str = String::from_utf8_lossy(&out.stdout);
                Self::parse_monitors_from_json(&json_str)
            }
            _ => Vec::new(),
        }
    }

    pub fn parse_monitors_from_json(json_str: &str) -> Vec<DisplayMonitorInfo> {
        let mut monitors = Vec::new();
        let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) else {
            return monitors;
        };

        let Some(items) = val.as_array() else {
            return monitors;
        };

        for item in items {
            let id = item.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let name = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }

            let description = item
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or(&name)
                .to_string();
            let make = item
                .get("make")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let model = item
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let width = item.get("width").and_then(|v| v.as_u64()).unwrap_or(1920) as u32;
            let height = item.get("height").and_then(|v| v.as_u64()).unwrap_or(1080) as u32;
            let refresh_rate = item
                .get("refreshRate")
                .and_then(|v| v.as_f64())
                .unwrap_or(60.0) as f32;
            let scale = item.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
            let transform = item
                .get("transform")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            let is_focused = item
                .get("focused")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let is_disabled = item
                .get("disabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let available_modes = if let Some(arr) = item
                .get("availableModes")
                .and_then(|v| v.as_array())
            {
                arr.iter()
                    .filter_map(|m| m.as_str().map(|s| s.to_string()))
                    .collect()
            } else {
                Vec::new()
            };

            monitors.push(DisplayMonitorInfo {
                id,
                name,
                description,
                make,
                model,
                width,
                height,
                refresh_rate,
                scale,
                transform,
                is_focused,
                is_disabled,
                available_modes,
            });
        }

        monitors
    }

    pub fn apply_monitor_config(name: &str, mode: &str, scale: f32, transform: u32) -> bool {
        let code = format!(
            r#"hl.monitor({{ output = "{}", mode = "{}", position = "auto", scale = "{}", transform = "{}" }})"#,
            name, mode, scale, transform
        );

        if let Ok(output) = Command::new("hyprctl").args(["eval", &code]).output() {
            if output.status.success() {
                return true;
            }
        }

        false
    }

    pub fn set_scale(name: &str, scale: f32) -> bool {
        if let Some(mon) = Self::list_monitors().into_iter().find(|m| m.name == name) {
            let mode = format!("{}x{}@{:.2}Hz", mon.width, mon.height, mon.refresh_rate);
            Self::apply_monitor_config(name, &mode, scale, mon.transform)
        } else {
            Self::apply_monitor_config(name, "preferred", scale, 0)
        }
    }

    pub fn set_mode(name: &str, mode: &str) -> bool {
        if let Some(mon) = Self::list_monitors().into_iter().find(|m| m.name == name) {
            Self::apply_monitor_config(name, mode, mon.scale, mon.transform)
        } else {
            Self::apply_monitor_config(name, mode, 1.0, 0)
        }
    }

    pub fn set_rotation(name: &str, degrees: u32) -> bool {
        let transform = match degrees {
            90 => 1,
            180 => 2,
            270 => 3,
            _ => 0,
        };

        if let Some(mon) = Self::list_monitors().into_iter().find(|m| m.name == name) {
            let mode = format!("{}x{}@{:.2}Hz", mon.width, mon.height, mon.refresh_rate);
            Self::apply_monitor_config(name, &mode, mon.scale, transform)
        } else {
            Self::apply_monitor_config(name, "preferred", 1.0, transform)
        }
    }

    pub fn toggle_dpms(name: &str, on: bool) {
        let state = if on { "on" } else { "off" };
        let _ = Command::new("hyprctl")
            .args(["dispatch", "dpms", state, name])
            .spawn();
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NightLightConfig {
    pub enabled: bool,
    pub temperature: u32,
}

impl Default for NightLightConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            temperature: 4000,
        }
    }
}

pub struct NightLightManager;

impl NightLightManager {
    fn config_path() -> PathBuf {
        let config_dir = std::env::var("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/m".to_string());
            PathBuf::from(home).join(".config")
        });
        config_dir.join("msui").join("nightlight.json")
    }

    pub fn load_config() -> NightLightConfig {
        let path = Self::config_path();
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str(&data) {
                return cfg;
            }
        }
        NightLightConfig::default()
    }

    pub fn save_config(cfg: &NightLightConfig) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_string_pretty(cfg) {
            let _ = fs::write(path, data);
        }
    }

    pub fn is_active() -> bool {
        let output = Command::new("pgrep")
            .args(["-x", "hyprsunset"])
            .output();
        output.map(|o| o.status.success()).unwrap_or(false)
    }

    pub fn get_temperature() -> u32 {
        Self::load_config().temperature
    }

    pub fn enable(temp_k: u32) {
        let mut cfg = Self::load_config();
        cfg.enabled = true;
        cfg.temperature = temp_k;
        Self::save_config(&cfg);

        if Self::is_active() {
            let temp_str = temp_k.to_string();
            let _ = Command::new("hyprctl")
                .args(["hyprsunset", "temperature", &temp_str])
                .output();
        } else {
            let temp_str = temp_k.to_string();
            let _ = Command::new("hyprsunset")
                .args(["-t", &temp_str])
                .spawn();
        }
    }

    pub fn disable() {
        let mut cfg = Self::load_config();
        cfg.enabled = false;
        Self::save_config(&cfg);

        let _ = Command::new("hyprctl")
            .args(["hyprsunset", "identity"])
            .output();
        let _ = Command::new("pkill")
            .args(["-x", "hyprsunset"])
            .output();
    }

    pub fn set_temperature(temp_k: u32) {
        if Self::is_active() {
            Self::enable(temp_k);
        } else {
            let mut cfg = Self::load_config();
            cfg.temperature = temp_k;
            Self::save_config(&cfg);
        }
    }

    pub fn toggle() -> bool {
        if Self::is_active() {
            Self::disable();
            false
        } else {
            let temp = Self::get_temperature();
            Self::enable(temp);
            true
        }
    }
}
