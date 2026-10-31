use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SystemServiceInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_active: bool,
    pub is_radio: bool,
    pub radio_state: Option<String>,
    pub ram_mb: u64,
    pub icon: String,
}

pub struct SystemServiceManager;

impl SystemServiceManager {
    /// List all managed system services with their live active status and memory usage.
    pub fn list_services() -> Vec<SystemServiceInfo> {
        vec![
            Self::bluetooth_info(),
            Self::proton_vpn_info(),
            Self::ai_info(),
            Self::wifi_info(),
            Self::tlp_info(),
            Self::flatpak_info(),
        ]
    }

    /// Fast sysfs rfkill check to see if Bluetooth is soft-blocked (< 0.05ms)
    pub fn is_bluetooth_soft_blocked() -> bool {
        if let Ok(entries) = fs::read_dir("/sys/class/rfkill") {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Ok(t) = fs::read_to_string(p.join("type")) {
                    if t.trim() == "bluetooth" {
                        if let Ok(s) = fs::read_to_string(p.join("soft")) {
                            if s.trim() == "1" {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Check if a systemd unit is active via systemctl is-active
    pub fn is_unit_active(unit_name: &str) -> bool {
        Command::new("systemctl")
            .args(["is-active", unit_name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get RSS memory consumption in MB for processes matching name pattern
    pub fn get_process_ram_mb(pattern: &str) -> u64 {
        let pids_out = Command::new("pgrep")
            .args(["-f", pattern])
            .output();

        let Ok(out) = pids_out else { return 0 };
        if !out.status.success() {
            return 0;
        }

        let pids_text = String::from_utf8_lossy(&out.stdout);
        let mut total_kb: u64 = 0;

        for line in pids_text.lines() {
            let pid = line.trim();
            if pid.is_empty() {
                continue;
            }
            let statm_path = format!("/proc/{}/statm", pid);
            if let Ok(statm) = fs::read_to_string(&statm_path) {
                let parts: Vec<&str> = statm.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(rss_pages) = parts[1].parse::<u64>() {
                        total_kb += rss_pages * 4;
                    }
                }
            }
        }

        total_kb / 1024
    }

    /// Bluetooth service info
    pub fn bluetooth_info() -> SystemServiceInfo {
        let soft_blocked = Self::is_bluetooth_soft_blocked();
        let service_active = Self::is_unit_active("bluetooth.service");
        let is_active = !soft_blocked && service_active;
        let ram_mb = if is_active {
            Self::get_process_ram_mb("bluetoothd")
        } else {
            0
        };

        SystemServiceInfo {
            id: "bluetooth".to_string(),
            name: "Bluetooth Radio".to_string(),
            description: "Kernel rfkill hardware radio & bluetooth.service".to_string(),
            is_active,
            is_radio: true,
            radio_state: Some(if soft_blocked {
                "Soft-Blocked (0 mW)".to_string()
            } else {
                "Powered On".to_string()
            }),
            ram_mb,
            icon: "󰂯".to_string(),
        }
    }

    /// Proton VPN service info
    pub fn proton_vpn_info() -> SystemServiceInfo {
        let is_active = Self::is_unit_active("proton.VPN.service");
        let ram_mb = if is_active {
            Self::get_process_ram_mb("proton.vpn.daemon")
        } else {
            0
        };

        SystemServiceInfo {
            id: "protonvpn".to_string(),
            name: "Proton VPN".to_string(),
            description: "proton.VPN.service background split-tunnel daemon".to_string(),
            is_active,
            is_radio: false,
            radio_state: None,
            ram_mb,
            icon: "󰌆".to_string(),
        }
    }

    /// Local AI Engine info
    pub fn ai_info() -> SystemServiceInfo {
        let is_active = Command::new("pgrep")
            .args(["-x", "ollama"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let ram_mb = if is_active {
            Self::get_process_ram_mb("ollama")
        } else {
            0
        };

        SystemServiceInfo {
            id: "ai".to_string(),
            name: "Local AI Engine".to_string(),
            description: "Qwen 2.5 Coder on-demand LLM on port 11434".to_string(),
            is_active,
            is_radio: false,
            radio_state: None,
            ram_mb,
            icon: "󰚩".to_string(),
        }
    }

    /// Wi-Fi Radio info
    pub fn wifi_info() -> SystemServiceInfo {
        let is_active = Command::new("nmcli")
            .args(["radio", "wifi"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "enabled")
            .unwrap_or(true);

        SystemServiceInfo {
            id: "wifi".to_string(),
            name: "Wi-Fi Radio".to_string(),
            description: "NetworkManager 802.11 wireless radio".to_string(),
            is_active,
            is_radio: true,
            radio_state: Some(if is_active {
                "Enabled".to_string()
            } else {
                "Disabled".to_string()
            }),
            ram_mb: 0,
            icon: "󰤨".to_string(),
        }
    }

    /// TLP Power Management info
    pub fn tlp_info() -> SystemServiceInfo {
        let is_active = Self::is_unit_active("tlp.service");
        let ram_mb = if is_active {
            Self::get_process_ram_mb("tlp")
        } else {
            0
        };

        SystemServiceInfo {
            id: "tlp".to_string(),
            name: "TLP Power Optimizer".to_string(),
            description: "tlp.service battery saver & CPU frequency governor".to_string(),
            is_active,
            is_radio: false,
            radio_state: None,
            ram_mb,
            icon: "󰌪".to_string(),
        }
    }

    /// Flatpak Helper info
    pub fn flatpak_info() -> SystemServiceInfo {
        let is_active = Self::is_unit_active("flatpak-system-helper.service");
        let ram_mb = if is_active {
            Self::get_process_ram_mb("flatpak-system-helper")
        } else {
            0
        };

        SystemServiceInfo {
            id: "flatpak".to_string(),
            name: "Flatpak Helper".to_string(),
            description: "flatpak-system-helper.service desktop indexer".to_string(),
            is_active,
            is_radio: false,
            radio_state: None,
            ram_mb,
            icon: "󰏗".to_string(),
        }
    }

    /// Enable or disable a service by ID
    pub fn set_service_state(service_id: &str, enable: bool) -> bool {
        let action = if enable { "enable" } else { "disable" };
        let status = Command::new("systemcontrol")
            .args([action, service_id])
            .status();

        match status {
            Ok(s) => s.success(),
            Err(_) => {
                // Fallback to direct shell execution if systemcontrol binary is not yet in PATH
                let script_path = Path::new(&std::env::var("HOME").unwrap_or_default())
                    .join(".local/bin/systemcontrol");
                if script_path.exists() {
                    Command::new(&script_path)
                        .args([action, service_id])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false)
                } else {
                    false
                }
            }
        }
    }

    /// Toggle a service by ID
    pub fn toggle_service(service_id: &str) -> bool {
        let status = Command::new("systemcontrol")
            .args(["toggle", service_id])
            .status();

        match status {
            Ok(s) => s.success(),
            Err(_) => {
                let script_path = Path::new(&std::env::var("HOME").unwrap_or_default())
                    .join(".local/bin/systemcontrol");
                if script_path.exists() {
                    Command::new(&script_path)
                        .args(["toggle", service_id])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false)
                } else {
                    false
                }
            }
        }
    }

    /// One-click RAM optimization
    pub fn optimize_ram() -> bool {
        Command::new("systemcontrol")
            .arg("optimize")
            .status()
            .map(|s| s.success())
            .unwrap_or_else(|_| {
                let script_path = Path::new(&std::env::var("HOME").unwrap_or_default())
                    .join(".local/bin/systemcontrol");
                Command::new(&script_path)
                    .arg("optimize")
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_services_structure() {
        let services = SystemServiceManager::list_services();
        assert_eq!(services.len(), 6);
        let ids: Vec<&str> = services.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"bluetooth"));
        assert!(ids.contains(&"protonvpn"));
        assert!(ids.contains(&"ai"));
        assert!(ids.contains(&"wifi"));
        assert!(ids.contains(&"tlp"));
        assert!(ids.contains(&"flatpak"));
    }

    #[test]
    fn test_service_info_serde() {
        let info = SystemServiceInfo {
            id: "bluetooth".to_string(),
            name: "Bluetooth Radio".to_string(),
            description: "Test description".to_string(),
            is_active: true,
            is_radio: true,
            radio_state: Some("Powered On".to_string()),
            ram_mb: 5,
            icon: "󰂯".to_string(),
        };

        let json = serde_json::to_string(&info).expect("Failed to serialize");
        let parsed: SystemServiceInfo = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(parsed.id, "bluetooth");
        assert_eq!(parsed.ram_mb, 5);
        assert!(parsed.is_active);
    }
}
