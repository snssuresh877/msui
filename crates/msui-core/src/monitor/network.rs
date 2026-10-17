use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkInfo {
    pub is_connected: bool,
    pub ssid: Option<String>,
    pub signal_percent: u8,
    pub is_ethernet: bool,
}

impl Default for NetworkInfo {
    fn default() -> Self {
        Self {
            is_connected: false,
            ssid: None,
            signal_percent: 0,
            is_ethernet: false,
        }
    }
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

pub struct NetworkMonitor;

impl NetworkMonitor {
    pub fn read() -> NetworkInfo {
        // Try nmcli for active wifi first
        if let Ok(output) = Command::new("nmcli")
            .args(["-t", "-f", "active,ssid,signal", "dev", "wifi"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 3 && parts[0].trim() == "yes" {
                    let ssid = parts[1].trim().to_string();
                    let signal = parts[2].trim().parse::<u8>().unwrap_or(0);
                    return NetworkInfo {
                        is_connected: true,
                        ssid: Some(ssid),
                        signal_percent: signal,
                        is_ethernet: false,
                    };
                }
            }
        }

        // Check for ethernet connection
        if let Ok(output) = Command::new("nmcli")
            .args(["-t", "-f", "TYPE,STATE", "dev"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 && parts[0] == "ethernet" && parts[1] == "connected" {
                    return NetworkInfo {
                        is_connected: true,
                        ssid: Some("Ethernet".into()),
                        signal_percent: 100,
                        is_ethernet: true,
                    };
                }
            }
        }

        NetworkInfo::default()
    }

    pub fn scan_wifi() -> Vec<WifiNetwork> {
        let mut networks = Vec::new();
        if let Ok(output) = Command::new("nmcli")
            .args(["-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY,BARS", "device", "wifi", "list"])
            .output()
        {
            if output.status.success() {
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
        if let Ok(output) = Command::new("nmcli").args(["radio", "wifi"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            return text.trim() == "enabled";
        }
        false
    }

    pub fn toggle_wifi_radio() {
        let enabled = Self::is_wifi_enabled();
        let target = if enabled { "off" } else { "on" };
        let _ = Command::new("nmcli").args(["radio", "wifi", target]).status();
    }

    pub fn rescan_wifi() {
        let _ = Command::new("nmcli").args(["device", "wifi", "rescan"]).spawn();
    }

    pub fn connect_wifi(ssid: &str, password: Option<&str>) -> Result<(), String> {
        let mut cmd = Command::new("nmcli");
        cmd.args(["device", "wifi", "connect", ssid]);
        if let Some(pwd) = password {
            if !pwd.trim().is_empty() {
                cmd.args(["password", pwd.trim()]);
            }
        }

        match cmd.output() {
            Ok(out) => {
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
        if let Ok(status) = Command::new("nmcli").args(["device", "disconnect", &dev]).status() {
            status.success()
        } else {
            false
        }
    }

    fn get_wifi_device() -> Option<String> {
        if let Ok(out) = Command::new("nmcli").args(["-t", "-f", "DEVICE,TYPE", "device"]).output() {
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
