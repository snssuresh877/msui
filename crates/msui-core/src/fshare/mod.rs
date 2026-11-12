pub mod server;
pub mod web_assets;

pub use server::{FShareServer, FShareServerHandle};

use chrono::Local;
use qrcode::render::svg;
use qrcode::QrCode;
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum FShareEvent {
    Started { url: String, pin: String },
    ClientConnected { peer_ip: String },
    Progress { transferred_bytes: u64, total_bytes: u64, speed_mbps: f32 },
    FileReceived { filename: String, size_bytes: u64, path: PathBuf },
    FileSent { filename: String, size_bytes: u64 },
    Error(String),
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferHistoryEntry {
    pub timestamp: String,
    pub direction: String, // "Sent" or "Received"
    pub filename: String,
    pub size_bytes: u64,
}

/// Detect active local IPv4 LAN address prioritizing physical Wi-Fi/Ethernet adapters over VPNs/tunnels
pub fn get_local_ip() -> Option<String> {
    // 1. Check physical network interfaces with IPv4
    if let Ok(output) = std::process::Command::new("ip")
        .args(["-4", "-o", "addr", "show"])
        .output()
    {
        if let Ok(text) = String::from_utf8(output.stdout) {
            let mut candidates: Vec<(u8, String)> = Vec::new();
            for line in text.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let iface = parts[1];
                    let ip_mask = parts[3];
                    let ip = ip_mask.split('/').next().unwrap_or("");

                    if iface == "lo" || ip.starts_with("127.") {
                        continue;
                    }
                    let iface_lower = iface.to_lowercase();
                    // Ignore VPN tunnels and virtual adapters
                    if iface_lower.starts_with("proton")
                        || iface_lower.starts_with("tun")
                        || iface_lower.starts_with("tap")
                        || iface_lower.starts_with("wg")
                        || iface_lower.starts_with("docker")
                        || iface_lower.starts_with("veth")
                        || iface_lower.starts_with("br-")
                        || iface_lower.starts_with("ipv6leak")
                        || iface_lower.starts_with("tailscale")
                    {
                        continue;
                    }

                    let is_physical = std::path::Path::new(&format!("/sys/class/net/{}/device", iface)).exists();
                    let score: u8 = if (iface_lower.starts_with("wl") || iface_lower.starts_with("wlan")) && is_physical {
                        100 // Physical Wi-Fi
                    } else if (iface_lower.starts_with("en") || iface_lower.starts_with("eth")) && is_physical {
                        90  // Physical Ethernet
                    } else if is_physical {
                        80
                    } else if ip.starts_with("192.168.") {
                        70
                    } else if ip.starts_with("10.") || ip.starts_with("172.") {
                        50
                    } else {
                        10
                    };
                    candidates.push((score, ip.to_string()));
                }
            }
            candidates.sort_by(|a, b| b.0.cmp(&a.0));
            if let Some((_, best_ip)) = candidates.into_iter().next() {
                return Some(best_ip);
            }
        }
    }

    // 2. Fallback: UDP socket routing
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("10.255.255.255:1").or_else(|_| socket.connect("8.8.8.8:80")).ok()?;
    let addr = socket.local_addr().ok()?;
    let ip = addr.ip();
    if ip.is_loopback() {
        None
    } else {
        Some(ip.to_string())
    }
}

/// Generate a 128-bit cryptographically secure random session token (32 hex characters)
pub fn generate_token() -> String {
    let mut bytes = [0u8; 16];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        if f.read_exact(&mut bytes).is_ok() {
            let mut hex = String::with_capacity(32);
            for b in bytes {
                use std::fmt::Write;
                let _ = write!(hex, "{:02x}", b);
            }
            return hex;
        }
    }

    // High-resolution clock & process entropy fallback if /dev/urandom is unavailable
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let pid = std::process::id() as u128;
    let rand_val: u128 = (now ^ (pid << 64)) ^ 0x9e3779b97f4a7c15;
    format!("{:032x}", rand_val)
}

/// Generate a 4-digit human verification PIN
pub fn generate_pin() -> String {
    let mut bytes = [0u8; 2];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        if f.read_exact(&mut bytes).is_ok() {
            let val = u16::from_ne_bytes(bytes) as u32;
            let pin = (val % 9000) + 1000;
            return format!("{}", pin);
        }
    }

    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let pin = (now % 9000) + 1000;
    format!("{}", pin)
}

/// Detect active connected Wi-Fi SSID
pub fn get_active_wifi_ssid() -> Option<String> {
    if let Ok(output) = std::process::Command::new("nmcli")
        .args(["-t", "-f", "ACTIVE,SSID", "device", "wifi"])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("yes:") {
                    let ssid = rest.trim();
                    if !ssid.is_empty() && ssid != "--" {
                        return Some(ssid.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Generate vector SVG QR code in memory with high-contrast mobile-scanner-friendly styling
pub fn generate_qr_svg(content: &str) -> Result<String, anyhow::Error> {
    let code = QrCode::new(content.as_bytes())?;
    let image = code
        .render::<svg::Color>()
        .min_dimensions(240, 240)
        .dark_color(svg::Color("#000000"))  // Dark modules for universal camera scanning
        .light_color(svg::Color("#ffffff")) // Crisp white background & quiet zone (ISO/IEC standard)
        .build();
    Ok(image)
}

/// Generate ANSI / Unicode QR code string for terminal output
pub fn generate_qr_terminal(content: &str) -> Result<String, anyhow::Error> {
    let code = QrCode::new(content.as_bytes())?;
    let image = code
        .render::<qrcode::render::unicode::Dense1x2>()
        .build();
    Ok(image)
}

/// Sanitize filename: prevents directory traversal, null bytes, and hidden files
pub fn sanitize_filename(name: &str) -> String {
    let base = std::path::Path::new(name)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "uploaded_file".to_string());

    let mut clean = base
        .replace(['/', '\\', '\0'], "_")
        .trim()
        .to_string();

    // Prevent hidden files (leading dots)
    while clean.starts_with('.') {
        clean.remove(0);
    }

    if clean.is_empty() {
        clean = "uploaded_file".to_string();
    }

    clean
}

/// Default save directory for incoming FShare transfers (~/Downloads/msui-fshare)
pub fn default_save_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join("Downloads").join("msui-fshare")
    } else {
        PathBuf::from("/tmp/msui-fshare")
    }
}

fn history_file_path() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".config").join("msui").join("fshare_history.json")
    } else {
        PathBuf::from("/tmp/msui_fshare_history.json")
    }
}

pub fn load_transfer_history() -> Vec<TransferHistoryEntry> {
    let path = history_file_path();
    if path.exists() {
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(entries) = serde_json::from_str(&data) {
                return entries;
            }
        }
    }
    Vec::new()
}

pub fn log_transfer(direction: &str, filename: &str, size_bytes: u64) {
    let mut history = load_transfer_history();
    let entry = TransferHistoryEntry {
        timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        direction: direction.to_string(),
        filename: filename.to_string(),
        size_bytes,
    };
    history.insert(0, entry);
    history.truncate(50); // Keep last 50 transfers

    let path = history_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&history) {
        let _ = std::fs::write(path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("../../etc/passwd"), "passwd");
        assert_eq!(sanitize_filename(".bashrc"), "bashrc");
        assert_eq!(sanitize_filename("...secret.key"), "secret.key");
        assert_eq!(sanitize_filename("normal_file.txt"), "normal_file.txt");
        assert_eq!(sanitize_filename(""), "uploaded_file");
    }

    #[test]
    fn test_generate_token_and_pin() {
        let token = generate_token();
        assert_eq!(token.len(), 32);
        let pin = generate_pin();
        assert_eq!(pin.len(), 4);
    }

    #[test]
    fn test_generate_qr_svg() {
        let svg = generate_qr_svg("http://192.168.1.15:5233/d/test").expect("Valid QR");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_get_local_ip() {
        let ip = get_local_ip();
        println!("DETECTED LOCAL IP: {:?}", ip);
        assert!(ip.is_some());
    }
}
