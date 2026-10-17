use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoragePartition {
    pub name: String,
    pub device_path: String,
    pub label: Option<String>,
    pub fs_type: Option<String>,
    pub mount_point: Option<String>,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub use_percent: u8,
    pub is_removable: bool,
    pub is_mounted: bool,
    pub is_read_only: bool,
    pub drive_model: Option<String>,
    pub drive_type: String,
}

impl StoragePartition {
    pub fn format_bytes(bytes: u64) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;
        const TB: f64 = GB * 1024.0;

        let b = bytes as f64;
        if b >= TB {
            format!("{:.1} TB", b / TB)
        } else if b >= GB {
            format!("{:.1} GB", b / GB)
        } else if b >= MB {
            format!("{:.1} MB", b / MB)
        } else if b >= KB {
            format!("{:.1} KB", b / KB)
        } else {
            format!("{} B", bytes)
        }
    }

    pub fn total_formatted(&self) -> String {
        Self::format_bytes(self.total_bytes)
    }

    pub fn used_formatted(&self) -> String {
        Self::format_bytes(self.used_bytes)
    }

    pub fn available_formatted(&self) -> String {
        Self::format_bytes(self.available_bytes)
    }

    pub fn display_title(&self) -> String {
        if let Some(lbl) = &self.label {
            if !lbl.trim().is_empty() {
                return lbl.clone();
            }
        }
        if let Some(mp) = &self.mount_point {
            if mp == "/" {
                return "Root (/)".to_string();
            } else if mp == "/home" {
                return "Home (/home)".to_string();
            } else if mp == "/boot/efi" {
                return "EFI System (/boot/efi)".to_string();
            } else if mp == "/boot" {
                return "Boot (/boot)".to_string();
            } else if mp.starts_with("/run/media/") {
                let name = mp.split('/').last().unwrap_or("Removable Drive");
                return format!("Removable ({})", name);
            }
            return mp.clone();
        }
        if self.is_removable {
            if let Some(model) = &self.drive_model {
                return format!("USB Drive ({})", model);
            }
            return format!("USB Drive ({})", self.name);
        }
        self.device_path.clone()
    }

    pub fn icon(&self) -> &'static str {
        if self.is_removable {
            "󱊞" // USB drive icon
        } else if let Some(mp) = &self.mount_point {
            if mp == "/" {
                "󰋊" // Root disk
            } else if mp == "/home" {
                "󱂵" // Home folder/partition
            } else if mp.contains("boot") {
                "󰌢" // Boot
            } else {
                "󰋊"
            }
        } else if let Some(fs) = &self.fs_type {
            if fs.contains("ntfs") || fs.contains("fat") {
                "󰨡" // Windows/external partition
            } else {
                "󰋊"
            }
        } else {
            "󰋊"
        }
    }
}

#[derive(Debug, Deserialize)]
struct LsblkResponse {
    #[serde(default)]
    blockdevices: Vec<LsblkDevice>,
}

#[derive(Debug, Deserialize)]
struct LsblkDevice {
    name: String,
    path: Option<String>,
    model: Option<String>,
    size: Option<serde_json::Value>,
    fstype: Option<String>,
    mountpoint: Option<String>,
    label: Option<String>,
    rm: Option<serde_json::Value>,
    ro: Option<serde_json::Value>,
    #[serde(rename = "type")]
    device_type: Option<String>,
    children: Option<Vec<LsblkDevice>>,
}

fn parse_json_bool(val: Option<&serde_json::Value>) -> bool {
    match val {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::Number(n)) => n.as_i64().map(|v| v != 0).unwrap_or(false),
        Some(serde_json::Value::String(s)) => {
            s == "1" || s.eq_ignore_ascii_case("true") || s.eq_ignore_ascii_case("yes")
        }
        _ => false,
    }
}

fn parse_json_u64(val: Option<&serde_json::Value>) -> u64 {
    match val {
        Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0),
        Some(serde_json::Value::String(s)) => s.parse::<u64>().unwrap_or(0),
        _ => 0,
    }
}

#[derive(Debug, Default, Clone)]
struct DfEntry {
    total_bytes: u64,
    used_bytes: u64,
    available_bytes: u64,
    use_percent: u8,
    mount_point: String,
}

pub struct StorageManager;

impl StorageManager {
    pub fn list_partitions() -> Vec<StoragePartition> {
        let lsblk_output = Command::new("lsblk")
            .args(["-J", "-b", "-o", "NAME,PATH,MODEL,SIZE,FSTYPE,MOUNTPOINT,LABEL,RM,RO,TYPE"])
            .output();

        let lsblk_json = match lsblk_output {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
            _ => return Vec::new(),
        };

        let df_output = Command::new("df")
            .args(["-B1"])
            .output();

        let df_text = match df_output {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
            _ => String::new(),
        };

        Self::parse_partitions_from_data(&lsblk_json, &df_text)
    }

    pub fn parse_partitions_from_data(lsblk_json: &str, df_output: &str) -> Vec<StoragePartition> {
        let df_map = Self::parse_df_output(df_output);

        let lsblk: LsblkResponse = match serde_json::from_str(lsblk_json) {
            Ok(resp) => resp,
            Err(_) => return Vec::new(),
        };

        let mut partitions = Vec::new();

        for dev in lsblk.blockdevices {
            let parent_model = dev.model.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
            let parent_rm = parse_json_bool(dev.rm.as_ref());

            if let Some(ref children) = dev.children {
                if !children.is_empty() {
                    for child in children {
                        if let Some(part) = Self::convert_device(child, parent_model.as_deref(), parent_rm, &df_map) {
                            partitions.push(part);
                        }
                    }
                    continue;
                }
            }

            // Top-level device without children (e.g. unpartitioned USB or raw disk)
            if let Some(part) = Self::convert_device(&dev, parent_model.as_deref(), parent_rm, &df_map) {
                partitions.push(part);
            }
        }

        // Sort: removable devices first, then root /, then /home, then others
        partitions.sort_by(|a, b| {
            let rank = |p: &StoragePartition| -> i32 {
                if p.is_removable {
                    0
                } else if p.mount_point.as_deref() == Some("/") {
                    1
                } else if p.mount_point.as_deref() == Some("/home") {
                    2
                } else if p.mount_point.is_some() {
                    3
                } else {
                    4
                }
            };
            rank(a).cmp(&rank(b)).then_with(|| a.device_path.cmp(&b.device_path))
        });

        partitions
    }

    fn convert_device(
        dev: &LsblkDevice,
        parent_model: Option<&str>,
        parent_rm: bool,
        df_map: &HashMap<String, DfEntry>,
    ) -> Option<StoragePartition> {
        let fs_type = dev.fstype.clone().filter(|s| !s.trim().is_empty());

        // Skip swap and empty non-removable system partitions
        if fs_type.as_deref() == Some("swap") {
            return None;
        }

        let is_removable = parent_rm || parse_json_bool(dev.rm.as_ref());
        let device_path = dev.path.clone().unwrap_or_else(|| format!("/dev/{}", dev.name));
        let total_size = parse_json_u64(dev.size.as_ref());

        // If size is 0 and not removable, skip
        if total_size == 0 && !is_removable {
            return None;
        }

        let mut mount_point = dev.mountpoint.clone().filter(|s| !s.trim().is_empty());
        let mut total_bytes = total_size;
        let mut used_bytes = 0;
        let mut available_bytes = total_size;
        let mut use_percent = 0;

        // Try to find df telemetry by device path or by mount point
        let mut df_entry = df_map.get(&device_path);
        if df_entry.is_none() {
            if let Some(mp) = &mount_point {
                df_entry = df_map.get(mp);
            }
        }

        if let Some(entry) = df_entry {
            total_bytes = entry.total_bytes;
            used_bytes = entry.used_bytes;
            available_bytes = entry.available_bytes;
            use_percent = entry.use_percent;
            if mount_point.is_none() {
                mount_point = Some(entry.mount_point.clone());
            }
        }

        let is_mounted = mount_point.is_some();

        let drive_model = dev
            .model
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| parent_model.map(|s| s.to_string()));

        Some(StoragePartition {
            name: dev.name.clone(),
            device_path,
            label: dev.label.clone().filter(|s| !s.trim().is_empty()),
            fs_type,
            mount_point,
            total_bytes,
            used_bytes,
            available_bytes,
            use_percent,
            is_removable,
            is_mounted,
            is_read_only: parse_json_bool(dev.ro.as_ref()),
            drive_model,
            drive_type: dev.device_type.clone().unwrap_or_else(|| "part".to_string()),
        })
    }

    fn parse_df_output(output: &str) -> HashMap<String, DfEntry> {
        let mut map = HashMap::new();
        for line in output.lines().skip(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 6 {
                let dev_or_fs = cols[0];
                let total = cols[1].parse::<u64>().unwrap_or(0);
                let used = cols[2].parse::<u64>().unwrap_or(0);
                let avail = cols[3].parse::<u64>().unwrap_or(0);
                let pct = cols[4].trim_end_matches('%').parse::<u8>().unwrap_or(0);
                let mp = cols[5..].join(" ");

                let entry = DfEntry {
                    total_bytes: total,
                    used_bytes: used,
                    available_bytes: avail,
                    use_percent: pct,
                    mount_point: mp.clone(),
                };

                map.insert(dev_or_fs.to_string(), entry.clone());
                map.insert(mp, entry);
            }
        }
        map
    }

    pub fn mount_device(device_path: &str) -> Result<String> {
        let output = Command::new("udisksctl")
            .args(["mount", "-b", device_path, "--no-user-interaction"])
            .output()
            .with_context(|| format!("Failed to execute udisksctl mount on {}", device_path))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            bail!("Mount failed: {}", if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() });
        }

        // Output format: "Mounted /dev/sda1 at /run/media/m/disk."
        if let Some(pos) = stdout.find(" at ") {
            let mount_path = stdout[pos + 4..].trim().trim_end_matches('.');
            return Ok(mount_path.to_string());
        }

        Ok(stdout.trim().to_string())
    }

    pub fn unmount_device(device_path: &str) -> Result<()> {
        let output = Command::new("udisksctl")
            .args(["unmount", "-b", device_path, "--no-user-interaction"])
            .output()
            .with_context(|| format!("Failed to execute udisksctl unmount on {}", device_path))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Unmount failed: {}", stderr.trim());
        }

        Ok(())
    }

    pub fn eject_drive(device_path: &str) -> Result<()> {
        let output = Command::new("udisksctl")
            .args(["power-off", "-b", device_path, "--no-user-interaction"])
            .output()
            .with_context(|| format!("Failed to execute udisksctl power-off on {}", device_path))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Safe eject failed: {}", stderr.trim());
        }

        Ok(())
    }

    pub fn open_in_file_manager(mount_point: &str) -> Result<()> {
        Command::new("xdg-open")
            .arg(mount_point)
            .spawn()
            .with_context(|| format!("Failed to launch file manager for {}", mount_point))?;
        Ok(())
    }

    pub fn has_removable_devices() -> bool {
        let partitions = Self::list_partitions();
        partitions.iter().any(|p| p.is_removable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(StoragePartition::format_bytes(500), "500 B");
        assert_eq!(StoragePartition::format_bytes(2048), "2.0 KB");
        assert_eq!(StoragePartition::format_bytes(10485760), "10.0 MB");
        assert_eq!(StoragePartition::format_bytes(31457280000), "29.3 GB");
        assert_eq!(StoragePartition::format_bytes(1099511627776), "1.0 TB");
    }

    #[test]
    fn test_parse_lsblk_and_df() {
        let sample_lsblk = r#"{
           "blockdevices": [
              {
                 "name": "sda",
                 "path": "/dev/sda",
                 "model": "UDisk",
                 "size": 31457280000,
                 "fstype": null,
                 "mountpoint": null,
                 "label": null,
                 "rm": true,
                 "ro": false,
                 "type": "disk",
                 "children": [
                    {
                       "name": "sda1",
                       "path": "/dev/sda1",
                       "model": null,
                       "size": 31457148928,
                       "fstype": "vfat",
                       "mountpoint": null,
                       "label": "USB_DATA",
                       "rm": true,
                       "ro": false,
                       "type": "part"
                    }
                 ]
              },
              {
                 "name": "nvme0n1",
                 "path": "/dev/nvme0n1",
                 "model": "SAMSUNG NVME",
                 "size": 512110190592,
                 "fstype": null,
                 "mountpoint": null,
                 "label": null,
                 "rm": false,
                 "ro": false,
                 "type": "disk",
                 "children": [
                    {
                       "name": "nvme0n1p6",
                       "path": "/dev/nvme0n1p6",
                       "model": null,
                       "size": 144955146240,
                       "fstype": "ext4",
                       "mountpoint": "/",
                       "label": null,
                       "rm": false,
                       "ro": false,
                       "type": "part"
                    }
                 ]
              }
           ]
        }"#;

        let sample_df = r#"Filesystem        1B-blocks         Used   Available Use% Mounted on
/dev/nvme0n1p6 141543612416  36188377088 98090700800  27% /
"#;

        let parts = StorageManager::parse_partitions_from_data(sample_lsblk, sample_df);
        assert_eq!(parts.len(), 2);

        // Removable drive is sorted first
        let usb = &parts[0];
        assert_eq!(usb.name, "sda1");
        assert_eq!(usb.device_path, "/dev/sda1");
        assert!(usb.is_removable);
        assert_eq!(usb.label.as_deref(), Some("USB_DATA"));
        assert_eq!(usb.fs_type.as_deref(), Some("vfat"));
        assert_eq!(usb.drive_model.as_deref(), Some("UDisk"));
        assert!(!usb.is_mounted);

        // Root partition
        let root = &parts[1];
        assert_eq!(root.name, "nvme0n1p6");
        assert_eq!(root.mount_point.as_deref(), Some("/"));
        assert_eq!(root.use_percent, 27);
        assert_eq!(root.used_bytes, 36188377088);
        assert!(!root.is_removable);
        assert!(root.is_mounted);
    }
}
