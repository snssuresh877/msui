use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatteryStatus {
    Charging,
    Discharging,
    Full,
    NotCharging,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct BatteryInfo {
    pub percentage: u8,
    pub status: BatteryStatus,
    pub is_charging: bool,
}

pub struct BatteryMonitor {
    sys_path: Option<PathBuf>,
}

impl BatteryMonitor {
    pub fn new() -> Self {
        let sys_path = Self::find_battery_path();
        Self { sys_path }
    }

    fn find_battery_path() -> Option<PathBuf> {
        let base = Path::new("/sys/class/power_supply");
        if !base.exists() {
            return None;
        }

        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("BAT") {
                    return Some(entry.path());
                }
            }
        }
        None
    }

    pub fn read(&self) -> Option<BatteryInfo> {
        let path = self.sys_path.as_ref()?;

        let cap_path = path.join("capacity");
        let status_path = path.join("status");

        let percentage = fs::read_to_string(cap_path)
            .ok()?
            .trim()
            .parse::<u8>()
            .ok()?;

        let status_str = fs::read_to_string(status_path)
            .unwrap_or_default()
            .trim()
            .to_string();

        let status = match status_str.as_str() {
            "Charging" => BatteryStatus::Charging,
            "Discharging" => BatteryStatus::Discharging,
            "Full" => BatteryStatus::Full,
            "Not charging" => BatteryStatus::NotCharging,
            _ => BatteryStatus::Unknown,
        };

        let is_charging = matches!(status, BatteryStatus::Charging | BatteryStatus::Full);

        Some(BatteryInfo {
            percentage,
            status,
            is_charging,
        })
    }
}

impl Default for BatteryMonitor {
    fn default() -> Self {
        Self::new()
    }
}
