use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AudioDevice {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub volume_percent: u8,
    pub is_muted: bool,
    pub is_default: bool,
    pub is_sink: bool,
    pub icon: String,
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
        run_pactl(&["set-sink-volume", "@DEFAULT_SINK@", &pct_str]);
    }

    pub fn increase_volume(delta: u8) {
        let delta_str = format!("+{delta}%");
        run_pactl(&["set-sink-volume", "@DEFAULT_SINK@", &delta_str]);
    }

    pub fn decrease_volume(delta: u8) {
        let delta_str = format!("-{delta}%");
        run_pactl(&["set-sink-volume", "@DEFAULT_SINK@", &delta_str]);
    }

    pub fn toggle_mute() {
        run_pactl(&["set-sink-mute", "@DEFAULT_SINK@", "toggle"]);
    }

    pub fn get_default_sink() -> Option<String> {
        let output = Command::new("pactl").arg("get-default-sink").output().ok()?;
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    pub fn get_default_source() -> Option<String> {
        let output = Command::new("pactl").arg("get-default-source").output().ok()?;
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    pub fn list_sinks() -> Vec<AudioDevice> {
        let default_sink = Self::get_default_sink();
        let output = Command::new("pactl")
            .args(["-f", "json", "list", "sinks"])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let json_str = String::from_utf8_lossy(&out.stdout);
                Self::parse_sinks_from_json(&json_str, default_sink.as_deref())
            }
            _ => Vec::new(),
        }
    }

    pub fn list_sources() -> Vec<AudioDevice> {
        let default_source = Self::get_default_source();
        let output = Command::new("pactl")
            .args(["-f", "json", "list", "sources"])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let json_str = String::from_utf8_lossy(&out.stdout);
                Self::parse_sources_from_json(&json_str, default_source.as_deref())
            }
            _ => Vec::new(),
        }
    }

    pub fn parse_sinks_from_json(json_str: &str, default_sink: Option<&str>) -> Vec<AudioDevice> {
        let mut devices = Vec::new();
        let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) else {
            return devices;
        };

        let Some(items) = val.as_array() else {
            return devices;
        };

        for item in items {
            let index = item.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
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
            let is_muted = item
                .get("mute")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let volume_percent = Self::extract_volume(item.get("volume"));
            let is_default = default_sink.map(|d| d == name).unwrap_or(false);
            let icon = Self::determine_icon(&name, &description, true).to_string();

            devices.push(AudioDevice {
                index,
                name,
                description,
                volume_percent,
                is_muted,
                is_default,
                is_sink: true,
                icon,
            });
        }

        devices
    }

    pub fn parse_sources_from_json(
        json_str: &str,
        default_source: Option<&str>,
    ) -> Vec<AudioDevice> {
        let mut devices = Vec::new();
        let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) else {
            return devices;
        };

        let Some(items) = val.as_array() else {
            return devices;
        };

        for item in items {
            let index = item.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let name = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }

            // Filter out monitor sources
            if name.ends_with(".monitor") {
                continue;
            }
            if let Some(props) = item.get("properties") {
                if props
                    .get("device.class")
                    .and_then(|v| v.as_str())
                    == Some("monitor")
                {
                    continue;
                }
            }

            let description = item
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or(&name)
                .to_string();
            let is_muted = item
                .get("mute")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let volume_percent = Self::extract_volume(item.get("volume"));
            let is_default = default_source.map(|d| d == name).unwrap_or(false);
            let icon = Self::determine_icon(&name, &description, false).to_string();

            devices.push(AudioDevice {
                index,
                name,
                description,
                volume_percent,
                is_muted,
                is_default,
                is_sink: false,
                icon,
            });
        }

        devices
    }

    fn extract_volume(vol_val: Option<&serde_json::Value>) -> u8 {
        if let Some(obj) = vol_val.and_then(|v| v.as_object()) {
            for (_channel, info) in obj {
                if let Some(pct_str) = info.get("value_percent").and_then(|v| v.as_str()) {
                    let trimmed = pct_str.trim_end_matches('%').trim();
                    if let Ok(pct) = trimmed.parse::<u8>() {
                        return pct;
                    }
                } else if let Some(val) = info.get("value").and_then(|v| v.as_u64()) {
                    let pct = ((val as f64 / 65536.0) * 100.0).round().min(150.0) as u8;
                    return pct;
                }
            }
        }
        0
    }

    fn determine_icon(name: &str, description: &str, is_sink: bool) -> &'static str {
        let combined = format!("{name} {description}").to_lowercase();
        if is_sink {
            if combined.contains("bluez") || combined.contains("bluetooth") {
                "󰂯"
            } else if combined.contains("headphone") || combined.contains("headset") {
                "󰋋"
            } else if combined.contains("hdmi")
                || combined.contains("displayport")
                || combined.contains("monitor")
            {
                "󰡁"
            } else if combined.contains("speaker") {
                "󰓃"
            } else {
                "󰕾"
            }
        } else if combined.contains("bluez") || combined.contains("bluetooth") {
            "󰂯"
        } else if combined.contains("headset") || combined.contains("headphone") {
            "󰋎"
        } else {
            "󰍬"
        }
    }

    pub fn set_default_sink(name: &str) {
        run_pactl(&["set-default-sink", name]);
    }

    pub fn set_default_source(name: &str) {
        run_pactl(&["set-default-source", name]);
    }

    pub fn set_sink_volume(name: &str, percent: u8) {
        let pct_str = format!("{percent}%");
        run_pactl(&["set-sink-volume", name, &pct_str]);
    }

    pub fn set_source_volume(name: &str, percent: u8) {
        let pct_str = format!("{percent}%");
        run_pactl(&["set-source-volume", name, &pct_str]);
    }

    pub fn toggle_sink_mute(name: &str) {
        run_pactl(&["set-sink-mute", name, "toggle"]);
    }

    pub fn toggle_source_mute(name: &str) {
        run_pactl(&["set-source-mute", name, "toggle"]);
    }
}

fn run_pactl(args: &[&str]) {
    if let Ok(mut child) = Command::new("pactl").args(args).spawn() {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}
