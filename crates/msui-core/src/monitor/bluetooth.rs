use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BluetoothDevice {
    pub mac: String,
    pub name: String,
    pub connected: bool,
    pub paired: bool,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BluetoothInfo {
    pub is_available: bool,
    pub is_powered: bool,
    pub connected_devices: Vec<BluetoothDevice>,
    pub paired_devices: Vec<BluetoothDevice>,
}

pub struct BluetoothManager;

impl BluetoothManager {
    pub fn read() -> BluetoothInfo {
        let show_out = Command::new("bluetoothctl")
            .arg("show")
            .output();

        let Ok(out) = show_out else {
            return BluetoothInfo {
                is_available: false,
                is_powered: false,
                connected_devices: Vec::new(),
                paired_devices: Vec::new(),
            };
        };

        if !out.status.success() {
            return BluetoothInfo {
                is_available: false,
                is_powered: false,
                connected_devices: Vec::new(),
                paired_devices: Vec::new(),
            };
        }

        let show_text = String::from_utf8_lossy(&out.stdout);
        let is_powered = show_text.lines().any(|l| l.trim().starts_with("Powered: yes"));

        let mut paired_devices = Vec::new();
        let mut connected_devices = Vec::new();

        if let Ok(dev_out) = Command::new("bluetoothctl").arg("devices").output() {
            if dev_out.status.success() {
                let dev_text = String::from_utf8_lossy(&dev_out.stdout);
                for line in dev_text.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 && parts[0] == "Device" {
                        let mac = parts[1].to_string();
                        let name = parts[2..].join(" ");
                        let (connected, paired, icon) = Self::device_info(&mac, &name);

                        let dev = BluetoothDevice {
                            mac,
                            name,
                            connected,
                            paired,
                            icon,
                        };

                        if connected {
                            connected_devices.push(dev.clone());
                        }
                        paired_devices.push(dev);
                    }
                }
            }
        }

        BluetoothInfo {
            is_available: true,
            is_powered,
            connected_devices,
            paired_devices,
        }
    }

    fn device_info(mac: &str, name: &str) -> (bool, bool, String) {
        let mut connected = false;
        let mut paired = true;
        let mut icon = Self::guess_icon(name);

        if let Ok(info_out) = Command::new("bluetoothctl").arg("info").arg(mac).output() {
            if info_out.status.success() {
                let text = String::from_utf8_lossy(&info_out.stdout);
                for line in text.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("Connected: yes") {
                        connected = true;
                    } else if trimmed.starts_with("Paired: no") {
                        paired = false;
                    } else if trimmed.starts_with("Icon:") {
                        let icon_type = trimmed.strip_prefix("Icon:").unwrap_or("").trim();
                        icon = match icon_type {
                            "audio-headphones" | "audio-headset" => "󰋋".to_string(),
                            "audio-card" => "󰓃".to_string(),
                            "input-keyboard" => "󰌌".to_string(),
                            "input-mouse" => "󰍽".to_string(),
                            "phone" => "󰍡".to_string(),
                            _ => Self::guess_icon(name),
                        };
                    }
                }
            }
        }

        (connected, paired, icon)
    }

    fn guess_icon(name: &str) -> String {
        let lower = name.to_lowercase();
        if lower.contains("phone") || lower.contains("s2") || lower.contains("pixel") || lower.contains("iphone") {
            "󰍡".to_string()
        } else if lower.contains("elite") || lower.contains("buds") || lower.contains("airpod") || lower.contains("headset") || lower.contains("headphone") {
            "󰋋".to_string()
        } else if lower.contains("keyboard") || lower.contains("keychron") {
            "󰌌".to_string()
        } else if lower.contains("mouse") || lower.contains("mx") {
            "󰍽".to_string()
        } else if lower.contains("speaker") || lower.contains("sound") {
            "󰓃".to_string()
        } else {
            "󰂯".to_string()
        }
    }

    pub fn toggle_power() {
        let info = Self::read();
        if info.is_powered {
            let _ = Command::new("bluetoothctl").arg("power").arg("off").status();
        } else {
            let _ = Command::new("rfkill").arg("unblock").arg("bluetooth").status();
            let _ = Command::new("bluetoothctl").arg("power").arg("on").status();
        }
    }

    pub fn set_power(on: bool) {
        if on {
            let _ = Command::new("rfkill").arg("unblock").arg("bluetooth").status();
            let _ = Command::new("bluetoothctl").arg("power").arg("on").status();
        } else {
            let _ = Command::new("bluetoothctl").arg("power").arg("off").status();
        }
    }

    pub fn connect(mac: &str) -> bool {
        let _ = Command::new("rfkill").arg("unblock").arg("bluetooth").status();
        let _ = Command::new("bluetoothctl").arg("power").arg("on").status();
        if let Ok(status) = Command::new("bluetoothctl").arg("connect").arg(mac).status() {
            status.success()
        } else {
            false
        }
    }

    pub fn disconnect(mac: &str) -> bool {
        if let Ok(status) = Command::new("bluetoothctl").arg("disconnect").arg(mac).status() {
            status.success()
        } else {
            false
        }
    }

    pub fn start_scan() {
        let _ = Command::new("bluetoothctl").arg("scan").arg("on").spawn();
    }
}
