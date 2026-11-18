use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

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

impl Default for BluetoothInfo {
    fn default() -> Self {
        Self {
            is_available: true,
            is_powered: false,
            connected_devices: Vec::new(),
            paired_devices: Vec::new(),
        }
    }
}

static BLUETOOTH_CACHE: Mutex<Option<(Instant, BluetoothInfo)>> = Mutex::new(None);
static IS_FETCHING_BLUETOOTH: AtomicBool = AtomicBool::new(false);

pub struct BluetoothManager;

impl BluetoothManager {
    /// Ultra-fast non-blocking bluetooth query for the GUI event loop.
    /// Returns cached state in ~0.001ms. Never blocks the UI thread on subprocesses.
    pub fn read() -> BluetoothInfo {
        // 1. Check in-memory cache (2-second TTL)
        if let Ok(guard) = BLUETOOTH_CACHE.lock()
            && let Some((instant, ref info)) = *guard
            && instant.elapsed() < Duration::from_secs(2)
        {
            return info.clone();
        }

        // 2. Ultra-fast sysfs rfkill check (< 0.05ms, 0 subprocesses)
        if let Ok(entries) = std::fs::read_dir("/sys/class/rfkill") {
            let mut found_bt = false;
            let mut is_soft_blocked = false;

            for entry in entries.flatten() {
                let p = entry.path();
                if let Ok(t) = std::fs::read_to_string(p.join("type"))
                    && t.trim() == "bluetooth" {
                        found_bt = true;
                        if let Ok(s) = std::fs::read_to_string(p.join("soft"))
                            && s.trim() == "1" {
                                is_soft_blocked = true;
                            }
                    }
            }

            if found_bt && is_soft_blocked {
                let info = BluetoothInfo {
                    is_available: true,
                    is_powered: false,
                    connected_devices: Vec::new(),
                    paired_devices: Vec::new(),
                };
                if let Ok(mut guard) = BLUETOOTH_CACHE.lock() {
                    *guard = Some((Instant::now(), info.clone()));
                }
                return info;
            }
        }

        // 3. Bluetooth is unblocked / powered on: trigger background fetch if not already in flight
        if !IS_FETCHING_BLUETOOTH.swap(true, Ordering::SeqCst) {
            std::thread::Builder::new()
                .name("msui-bt-mon".into())
                .spawn(move || {
                    let info = Self::fetch_bluetooth_info();
                    if let Ok(mut guard) = BLUETOOTH_CACHE.lock() {
                        *guard = Some((Instant::now(), info));
                    }
                    IS_FETCHING_BLUETOOTH.store(false, Ordering::SeqCst);
                })
                .ok();
        }

        // Return existing cached value (or default if very first frame)
        if let Ok(guard) = BLUETOOTH_CACHE.lock()
            && let Some((_, ref info)) = *guard
        {
            return info.clone();
        }

        BluetoothInfo {
            is_available: true,
            is_powered: true,
            connected_devices: Vec::new(),
            paired_devices: Vec::new(),
        }
    }

    pub fn invalidate_cache() {
        if let Ok(mut guard) = BLUETOOTH_CACHE.lock() {
            *guard = None;
        }
    }

    fn fetch_bluetooth_info() -> BluetoothInfo {
        let mut connected_devices = Vec::new();
        let mut paired_devices = Vec::new();
        let mut is_powered = false;

        // Query status
        if let Ok(out) = Command::new("bluetoothctl")
            .arg("show")
            .stdin(Stdio::null())
            .output()
            && out.status.success() {
                let show_text = String::from_utf8_lossy(&out.stdout);
                is_powered = show_text.lines().any(|l| l.trim().starts_with("Powered: yes"));
            }

        if is_powered {
            // Connected devices
            if let Ok(out) = Command::new("bluetoothctl")
                .args(["devices", "Connected"])
                .stdin(Stdio::null())
                .output()
                && out.status.success() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    for line in text.lines() {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 3 && parts[0] == "Device" {
                            let mac = parts[1].to_string();
                            let name = parts[2..].join(" ");
                            let icon = Self::guess_icon(&name);
                            connected_devices.push(BluetoothDevice {
                                mac,
                                name,
                                connected: true,
                                paired: true,
                                icon,
                            });
                        }
                    }
                }

            // Paired devices
            if let Ok(out) = Command::new("bluetoothctl")
                .args(["devices", "Paired"])
                .stdin(Stdio::null())
                .output()
                && out.status.success() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    for line in text.lines() {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 3 && parts[0] == "Device" {
                            let mac = parts[1].to_string();
                            let name = parts[2..].join(" ");
                            let is_conn = connected_devices.iter().any(|d| d.mac == mac);
                            let icon = Self::guess_icon(&name);
                            paired_devices.push(BluetoothDevice {
                                mac,
                                name,
                                connected: is_conn,
                                paired: true,
                                icon,
                            });
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
        Self::invalidate_cache();
        let info = Self::read();
        if info.is_powered {
            let _ = Command::new("bluetoothctl").arg("power").arg("off").status();
        } else {
            let _ = Command::new("rfkill").arg("unblock").arg("bluetooth").status();
            let _ = Command::new("bluetoothctl").arg("power").arg("on").status();
        }
    }

    pub fn set_power(on: bool) {
        Self::invalidate_cache();
        if on {
            let _ = Command::new("rfkill").arg("unblock").arg("bluetooth").status();
            let _ = Command::new("bluetoothctl").arg("power").arg("on").status();
        } else {
            let _ = Command::new("bluetoothctl").arg("power").arg("off").status();
        }
    }

    pub fn connect(mac: &str) -> bool {
        Self::invalidate_cache();
        let _ = Command::new("rfkill").arg("unblock").arg("bluetooth").status();
        let _ = Command::new("bluetoothctl").arg("power").arg("on").status();
        if let Ok(status) = Command::new("bluetoothctl").arg("connect").arg(mac).status() {
            status.success()
        } else {
            false
        }
    }

    pub fn disconnect(mac: &str) -> bool {
        Self::invalidate_cache();
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
