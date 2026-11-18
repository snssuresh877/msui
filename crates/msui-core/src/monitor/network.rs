use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
pub struct NetworkInfo {
    pub is_connected: bool,
    pub ssid: Option<String>,
    pub signal_percent: u8,
    pub is_ethernet: bool,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u8,
    pub is_connected: bool,
    pub is_secure: bool,
    pub security: String,
    pub bars: String,
}

// In-memory cache with TTL to protect UI from blocking subprocesses
static NETWORK_CACHE: Mutex<Option<(Instant, NetworkInfo)>> = Mutex::new(None);
static IS_FETCHING: AtomicBool = AtomicBool::new(false);

pub struct NetworkMonitor;

impl NetworkMonitor {
    /// Ultra-fast, non-blocking network query for the GUI event loop.
    /// Returns cached state in ~0.001ms. Never blocks the UI thread on subprocesses or D-Bus.
    pub fn read() -> NetworkInfo {
        // 1. Check in-memory cache (2-second TTL)
        if let Ok(guard) = NETWORK_CACHE.lock()
            && let Some((instant, ref info)) = *guard
                && instant.elapsed() < Duration::from_secs(2) {
                    return info.clone();
                }

        // 2. Kernel sysfs check (< 0.02ms, 0 subprocesses): Is any non-loopback interface UP?
        if !Self::has_active_interface() {
            let disconnected = NetworkInfo::default();
            if let Ok(mut guard) = NETWORK_CACHE.lock() {
                *guard = Some((Instant::now(), disconnected.clone()));
            }
            return disconnected;
        }

        // 3. If cache is stale or missing, trigger a background fetch if not already in flight
        if !IS_FETCHING.swap(true, Ordering::SeqCst) {
            std::thread::Builder::new()
                .name("msui-net-mon".into())
                .spawn(move || {
                    let info = Self::fetch_network_info();
                    if let Ok(mut guard) = NETWORK_CACHE.lock() {
                        *guard = Some((Instant::now(), info));
                    }
                    IS_FETCHING.store(false, Ordering::SeqCst);
                })
                .ok();
        }

        // Return existing cached value (or default if very first frame)
        if let Ok(guard) = NETWORK_CACHE.lock()
            && let Some((_, ref info)) = *guard {
                return info.clone();
            }

        NetworkInfo::default()
    }

    /// Fast kernel check: checks /sys/class/net/*/operstate without any subprocess or D-Bus call.
    fn has_active_interface() -> bool {
        let Ok(entries) = std::fs::read_dir("/sys/class/net") else {
            return false;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str == "lo" {
                continue;
            }
            if let Ok(operstate) = std::fs::read_to_string(entry.path().join("operstate")) {
                let state = operstate.trim();
                if state == "up" || state == "unknown" {
                    return true;
                }
            }
        }
        false
    }

    /// Read wireless signal quality (0-100) directly from /proc/net/wireless without subprocess
    fn read_sys_wifi_signal() -> Option<u8> {
        let Ok(content) = std::fs::read_to_string("/proc/net/wireless") else {
            return None;
        };
        for line in content.lines() {
            if line.contains(':') && !line.starts_with("Inter") && !line.starts_with(" face") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let link_str = parts[2].trim_end_matches('.');
                    if let Ok(val) = link_str.parse::<u8>() {
                        let percent = if val <= 70 {
                            ((val as f32 / 70.0) * 100.0) as u8
                        } else {
                            val
                        };
                        return Some(percent.min(100));
                    }
                }
            }
        }
        None
    }

    /// Runs in background thread with a strict 2-second timeout (-w 2) on nmcli
    fn fetch_network_info() -> NetworkInfo {
        // Query nmcli device status: single command with -w 2 timeout
        if let Ok(output) = Command::new("nmcli")
            .args(["-w", "2", "-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "dev"])
            .output()
            && output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 4 {
                        let dev_type = parts[1].trim();
                        let state = parts[2].trim();
                        let connection = parts[3].trim();

                        if state == "connected" {
                            if dev_type == "wifi" {
                                let signal = Self::read_sys_wifi_signal().unwrap_or(75);
                                return NetworkInfo {
                                    is_connected: true,
                                    ssid: Some(connection.to_string()),
                                    signal_percent: signal,
                                    is_ethernet: false,
                                };
                            } else if dev_type == "ethernet" {
                                return NetworkInfo {
                                    is_connected: true,
                                    ssid: Some("Ethernet".into()),
                                    signal_percent: 100,
                                    is_ethernet: true,
                                };
                            }
                        }
                    }
                }
            }

        NetworkInfo::default()
    }

    pub fn scan_wifi() -> Vec<WifiNetwork> {
        let mut networks = Vec::new();
        if let Ok(output) = Command::new("nmcli")
            .args(["-w", "3", "-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY,BARS", "device", "wifi", "list"])
            .output()
            && output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 5 {
                        let is_connected = parts[0].trim() == "*";
                        let ssid = parts[1].trim().to_string();
                        if ssid.is_empty() || ssid == "--" {
                            continue;
                        }
                        let signal = parts[2].trim().parse::<u8>().unwrap_or(0);
                        let security = parts[3].trim().to_string();
                        let is_secure = !security.is_empty() && security != "--";
                        let bars = parts[4].trim().to_string();

                        // Deduplicate: if already present, keep the one with higher signal or connected
                        if let Some(existing) = networks.iter_mut().find(|n: &&mut WifiNetwork| n.ssid == ssid) {
                            if is_connected {
                                existing.is_connected = true;
                            }
                            if signal > existing.signal {
                                existing.signal = signal;
                                existing.bars = bars;
                            }
                        } else {
                            networks.push(WifiNetwork {
                                ssid,
                                signal,
                                is_connected,
                                is_secure,
                                security,
                                bars,
                            });
                        }
                    }
                }
            }

        // Sort: connected first, then by signal strength descending
        networks.sort_by(|a, b| {
            b.is_connected
                .cmp(&a.is_connected)
                .then_with(|| b.signal.cmp(&a.signal))
        });

        networks
    }

    pub fn is_wifi_enabled() -> bool {
        // Fast sysfs rfkill check first (< 0.02ms, 0 subprocesses)
        if let Ok(entries) = std::fs::read_dir("/sys/class/rfkill") {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Ok(t) = std::fs::read_to_string(p.join("type"))
                    && t.trim() == "wlan" {
                        if let Ok(s) = std::fs::read_to_string(p.join("soft"))
                            && s.trim() == "1" {
                                return false; // Soft-blocked (airplane mode / wifi off)
                            }
                        if let Ok(h) = std::fs::read_to_string(p.join("hard"))
                            && h.trim() == "1" {
                                return false; // Hardware killswitch
                            }
                    }
            }
        }

        // Fallback to nmcli with -w 2 timeout
        if let Ok(output) = Command::new("nmcli").args(["-w", "2", "radio", "wifi"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            return text.trim() == "enabled";
        }
        false
    }

    pub fn toggle_wifi_radio() {
        let enabled = Self::is_wifi_enabled();
        let target = if enabled { "off" } else { "on" };
        let _ = Command::new("nmcli").args(["-w", "2", "radio", "wifi", target]).status();
        // Invalidate cache immediately on user toggle
        if let Ok(mut guard) = NETWORK_CACHE.lock() {
            *guard = None;
        }
    }

    pub fn rescan_wifi() {
        let _ = Command::new("nmcli").args(["-w", "2", "device", "wifi", "rescan"]).spawn();
    }

    pub fn connect_wifi(ssid: &str, password: Option<&str>) -> Result<(), String> {
        let mut cmd = Command::new("nmcli");
        let has_pwd = password.map(|p| !p.trim().is_empty()).unwrap_or(false);

        if has_pwd {
            // --ask tells nmcli to read passwords from stdin instead of process arguments
            cmd.arg("--ask");
        }
        cmd.args(["-w", "15", "device", "wifi", "connect", ssid]);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| e.to_string())?;

        if let Some(pwd) = password
            && has_pwd
                && let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    let _ = stdin.write_all(format!("{}\n", pwd.trim()).as_bytes());
                    let _ = stdin.flush();
                }

        match child.wait_with_output() {
            Ok(out) => {
                // Invalidate cache on connect
                if let Ok(mut guard) = NETWORK_CACHE.lock() {
                    *guard = None;
                }
                if out.status.success() {
                    Ok(())
                } else {
                    let err = String::from_utf8_lossy(&out.stderr).to_string();
                    let msg = if err.trim().is_empty() {
                        String::from_utf8_lossy(&out.stdout).to_string()
                    } else {
                        err
                    };
                    Err(msg.trim().to_string())
                }
            }
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn disconnect_wifi() -> bool {
        let dev = Self::get_wifi_device().unwrap_or_else(|| "wlan0".to_string());
        let success = if let Ok(status) = Command::new("nmcli").args(["-w", "2", "device", "disconnect", &dev]).status() {
            status.success()
        } else {
            false
        };
        if success
            && let Ok(mut guard) = NETWORK_CACHE.lock() {
                *guard = None;
            }
        success
    }

    fn get_wifi_device() -> Option<String> {
        if let Ok(out) = Command::new("nmcli").args(["-w", "2", "-t", "-f", "DEVICE,TYPE", "device"]).output() {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 && parts[1] == "wifi" {
                    return Some(parts[0].to_string());
                }
            }
        }
        None
    }
}
