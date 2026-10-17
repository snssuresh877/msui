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
}
