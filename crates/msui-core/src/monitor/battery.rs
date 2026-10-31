use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BatteryStatus {
    Charging,
    Discharging,
    Full,
    NotCharging,
    Unknown,
}

impl BatteryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BatteryStatus::Charging => "Charging",
            BatteryStatus::Discharging => "Discharging",
            BatteryStatus::Full => "Full",
            BatteryStatus::NotCharging => "Not Charging",
            BatteryStatus::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PowerProfile {
    PowerSaver,
    Balanced,
    Performance,
}

impl PowerProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            PowerProfile::PowerSaver => "power-saver",
            PowerProfile::Balanced => "balanced",
            PowerProfile::Performance => "performance",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            PowerProfile::PowerSaver => "Power Saver",
            PowerProfile::Balanced => "Balanced",
            PowerProfile::Performance => "Performance",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            PowerProfile::PowerSaver => "🍃",
            PowerProfile::Balanced => "⚖",
            PowerProfile::Performance => "⚡",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            PowerProfile::PowerSaver => "Limits CPU clock & throttles power draw to maximize battery runtime.",
            PowerProfile::Balanced => "Dynamic power scaling for balanced daily performance and battery life.",
            PowerProfile::Performance => "Unlocks maximum CPU frequencies and turbo responsiveness.",
        }
    }

    pub fn from_str_loose(s: &str) -> Option<Self> {
        let lower = s.to_lowercase();
        if lower.contains("saver") || lower.contains("save") || lower.contains("low") {
            Some(PowerProfile::PowerSaver)
        } else if lower.contains("perf") || lower.contains("turbo") || lower.contains("high") {
            Some(PowerProfile::Performance)
        } else if lower.contains("bal") || lower.contains("sched") || lower.contains("def") {
            Some(PowerProfile::Balanced)
        } else {
            None
        }
    }
}

pub struct PowerProfileManager;

impl PowerProfileManager {
    pub fn get_profile() -> PowerProfile {
        // Fast ACPI platform_profile sysfs check (< 0.05ms)
        if let Ok(text) = std::fs::read_to_string("/sys/firmware/acpi/platform_profile") {
            if let Some(prof) = PowerProfile::from_str_loose(&text) {
                return prof;
            }
        }

        if let Ok(output) = std::process::Command::new("tlpctl").arg("get").output() {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                if let Some(prof) = PowerProfile::from_str_loose(&text) {
                    return prof;
                }
            }
        }

        if let Ok(output) = std::process::Command::new("powerprofilesctl").arg("get").output() {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                if let Some(prof) = PowerProfile::from_str_loose(&text) {
                    return prof;
                }
            }
        }

        if let Ok(output) = std::process::Command::new("busctl")
            .args([
                "get-property",
                "net.hadess.PowerProfiles",
                "/net/hadess/PowerProfiles",
                "net.hadess.PowerProfiles",
                "ActiveProfile",
            ])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                if let Some(prof) = PowerProfile::from_str_loose(&text) {
                    return prof;
                }
            }
        }

        PowerProfile::Balanced
    }

    pub fn set_profile(profile: PowerProfile) -> bool {
        let slug = profile.as_str();

        if let Ok(status) = std::process::Command::new("tlpctl")
            .arg(slug)
            .status()
        {
            if status.success() {
                std::thread::sleep(std::time::Duration::from_millis(300));
                return true;
            }
        }

        if let Ok(status) = std::process::Command::new("tlpctl")
            .args(["set", slug])
            .status()
        {
            if status.success() {
                std::thread::sleep(std::time::Duration::from_millis(300));
                return true;
            }
        }

        if let Ok(status) = std::process::Command::new("powerprofilesctl")
            .args(["set", slug])
            .status()
        {
            if status.success() {
                std::thread::sleep(std::time::Duration::from_millis(300));
                return true;
            }
        }

        false
    }

    pub fn list_profiles() -> Vec<PowerProfile> {
        vec![
            PowerProfile::PowerSaver,
            PowerProfile::Balanced,
            PowerProfile::Performance,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BatteryInfo {
    pub percentage: u8,
    pub status: BatteryStatus,
    pub is_charging: bool,
    pub is_ac_online: bool,
    pub power_watts: f32,
    pub voltage_volts: f32,
    pub energy_now_wh: f32,
    pub energy_full_wh: f32,
    pub energy_full_design_wh: f32,
    pub health_percent: f32,
    pub cycle_count: Option<u32>,
    pub technology: String,
    pub time_remaining_mins: Option<u32>,
}

pub struct BatteryMonitor {
    sys_path: Option<PathBuf>,
    ac_path: Option<PathBuf>,
}

impl BatteryMonitor {
    pub fn new() -> Self {
        let sys_path = Self::find_battery_path();
        let ac_path = Self::find_ac_path();
        Self { sys_path, ac_path }
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

    fn find_ac_path() -> Option<PathBuf> {
        let base = Path::new("/sys/class/power_supply");
        if !base.exists() {
            return None;
        }

        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("AC") || name.starts_with("ADP") || name.starts_with("Mains") {
                    return Some(entry.path());
                }
            }
        }
        None
    }

    pub fn read(&self) -> Option<BatteryInfo> {
        let path = self.sys_path.as_ref()?;

        let percentage = Self::read_number(&path.join("capacity"))
            .map(|n| n as u8)
            .unwrap_or(0);

        let status_str = fs::read_to_string(path.join("status"))
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

        let is_ac_online = if let Some(ac) = &self.ac_path {
            Self::read_number(&ac.join("online")).map(|n| n == 1).unwrap_or(false)
        } else {
            is_charging
        };

        // Voltage in Volts
        let voltage_volts = Self::read_number(&path.join("voltage_now"))
            .map(|u| u as f32 / 1_000_000.0)
            .unwrap_or(12.0);

        // Power draw in Watts
        let power_watts = if let Some(uw) = Self::read_number(&path.join("power_now")) {
            uw as f32 / 1_000_000.0
        } else if let Some(ua) = Self::read_number(&path.join("current_now")) {
            (ua as f32 * voltage_volts) / 1_000_000.0
        } else {
            0.0
        };

        // Energy in Wh
        let energy_now_wh = if let Some(uwh) = Self::read_number(&path.join("energy_now")) {
            uwh as f32 / 1_000_000.0
        } else if let Some(uah) = Self::read_number(&path.join("charge_now")) {
            (uah as f32 * voltage_volts) / 1_000_000.0
        } else {
            0.0
        };

        let energy_full_wh = if let Some(uwh) = Self::read_number(&path.join("energy_full")) {
            uwh as f32 / 1_000_000.0
        } else if let Some(uah) = Self::read_number(&path.join("charge_full")) {
            (uah as f32 * voltage_volts) / 1_000_000.0
        } else {
            energy_now_wh
        };

        let energy_full_design_wh = if let Some(uwh) = Self::read_number(&path.join("energy_full_design")) {
            uwh as f32 / 1_000_000.0
        } else if let Some(uah) = Self::read_number(&path.join("charge_full_design")) {
            (uah as f32 * voltage_volts) / 1_000_000.0
        } else {
            energy_full_wh
        };

        // Health percentage
        let health_percent = if energy_full_design_wh > 0.1 {
            ((energy_full_wh / energy_full_design_wh) * 100.0).clamp(0.0, 100.0)
        } else {
            100.0
        };

        // Cycle count
        let cycle_count = Self::read_number(&path.join("cycle_count")).map(|n| n as u32);

        // Technology
        let technology = fs::read_to_string(path.join("technology"))
            .unwrap_or_else(|_| "Li-ion".to_string())
            .trim()
            .to_string();

        // Estimated remaining runtime
        let time_remaining_mins = if power_watts > 0.5 {
            if status == BatteryStatus::Discharging {
                Some(((energy_now_wh / power_watts) * 60.0) as u32)
            } else if status == BatteryStatus::Charging && energy_full_wh > energy_now_wh {
                Some((((energy_full_wh - energy_now_wh) / power_watts) * 60.0) as u32)
            } else {
                None
            }
        } else {
            None
        };

        Some(BatteryInfo {
            percentage,
            status,
            is_charging,
            is_ac_online,
            power_watts,
            voltage_volts,
            energy_now_wh,
            energy_full_wh,
            energy_full_design_wh,
            health_percent,
            cycle_count,
            technology,
            time_remaining_mins,
        })
    }

    fn read_number(path: &Path) -> Option<u64> {
        fs::read_to_string(path)
            .ok()?
            .trim()
            .parse::<u64>()
            .ok()
    }
}

impl Default for BatteryMonitor {
    fn default() -> Self {
        Self::new()
    }
}
