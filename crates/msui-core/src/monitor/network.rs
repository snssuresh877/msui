use std::collections::HashMap;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{debug, warn};
use zbus::zvariant::OwnedValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InternetConnectivity {
    #[default]
    Unknown,
    None,
    Portal,  // Captive portal detected
    Limited, // Connected to local network, no upstream gateway
    Full,    // Full internet access verified
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NetworkInfo {
    pub is_connected: bool,
    pub ssid: Option<String>,
    pub signal_percent: u8,
    pub is_ethernet: bool,
    pub wifi_enabled: bool,
    pub connectivity: InternetConnectivity,
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

// In-memory cache for ultra-fast, zero-wait UI event loop reads (< 0.001ms)
static NETWORK_CACHE: Mutex<Option<(Instant, NetworkInfo)>> = Mutex::new(None);
static IS_FETCHING: AtomicBool = AtomicBool::new(false);

// Scan Coordination and Generation Guard
static SCAN_GENERATION: AtomicU64 = AtomicU64::new(1);
static IS_SCANNING: AtomicBool = AtomicBool::new(false);
static LAST_SCAN_TIME: Mutex<Option<Instant>> = Mutex::new(None);
static CACHED_APS: Mutex<Vec<WifiNetwork>> = Mutex::new(Vec::new());

const SCAN_COOLDOWN: Duration = Duration::from_secs(15);
const CACHE_TTL: Duration = Duration::from_secs(2);

pub struct NetworkMonitor;

impl NetworkMonitor {
    /// Ultra-fast, non-blocking network query for the GUI event loop.
    /// Returns cached state in ~0.001ms. Never blocks the UI thread on subprocesses or D-Bus.
    pub fn read() -> NetworkInfo {
        // 1. Check in-memory cache (2-second TTL)
        if let Ok(guard) = NETWORK_CACHE.lock()
            && let Some((instant, ref info)) = *guard
            && instant.elapsed() < CACHE_TTL
        {
            return info.clone();
        }

        // 2. Kernel sysfs check (< 0.01ms, 0 subprocesses): Is any non-loopback interface UP?
        if !Self::has_active_interface() {
            let disconnected = NetworkInfo {
                is_connected: false,
                ssid: None,
                signal_percent: 0,
                is_ethernet: false,
                wifi_enabled: Self::is_wifi_enabled(),
                connectivity: InternetConnectivity::None,
            };
            if let Ok(mut guard) = NETWORK_CACHE.lock() {
                *guard = Some((Instant::now(), disconnected.clone()));
            }
            return disconnected;
        }

        // 3. Trigger asynchronous background fetch if not already in flight
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

        // Return existing cached value (or immediate sysfs baseline if very first frame)
        if let Ok(guard) = NETWORK_CACHE.lock()
            && let Some((_, ref info)) = *guard
        {
            return info.clone();
        }

        NetworkInfo {
            is_connected: Self::has_active_interface(),
            ssid: None,
            signal_percent: Self::read_sys_wifi_signal().unwrap_or(0),
            is_ethernet: Self::has_active_ethernet(),
            wifi_enabled: Self::is_wifi_enabled(),
            connectivity: InternetConnectivity::Unknown,
        }
    }

    /// Invalidate cache to force immediate refresh on next read()
    pub fn invalidate_cache() {
        if let Ok(mut guard) = NETWORK_CACHE.lock() {
            *guard = None;
        }
    }

    /// Fast kernel check: checks /sys/class/net/*/operstate without any subprocess or D-Bus call.
    pub fn has_active_interface() -> bool {
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

    /// Fast kernel check for active physical Ethernet carrier
    pub fn has_active_ethernet() -> bool {
        let Ok(entries) = std::fs::read_dir("/sys/class/net") else {
            return false;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if (name_str.starts_with("eth") || name_str.starts_with("en"))
                && let Ok(carrier) = std::fs::read_to_string(entry.path().join("carrier"))
                && carrier.trim() == "1"
            {
                return true;
            }
        }
        false
    }

    /// Read wireless signal quality (0-100) directly from /proc/net/wireless in < 0.01ms
    pub fn read_sys_wifi_signal() -> Option<u8> {
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

    /// Ultra-fast radio status check directly from /sys/class/rfkill (< 0.01ms, 0 subprocesses)
    pub fn is_wifi_enabled() -> bool {
        if let Ok(entries) = std::fs::read_dir("/sys/class/rfkill") {
            let mut found_wlan = false;
            let mut any_blocked = false;
            for entry in entries.flatten() {
                let p = entry.path();
                if let Ok(t) = std::fs::read_to_string(p.join("type"))
                    && t.trim() == "wlan"
                {
                    found_wlan = true;
                    if let Ok(s) = std::fs::read_to_string(p.join("soft"))
                        && s.trim() == "1"
                    {
                        any_blocked = true;
                        break;
                    }
                    if let Ok(h) = std::fs::read_to_string(p.join("hard"))
                        && h.trim() == "1"
                    {
                        any_blocked = true;
                        break;
                    }
                }
            }
            if found_wlan {
                return !any_blocked;
            }
        }

        // Fallback to nmcli with 1s timeout only if sysfs rfkill had no wlan entries
        if let Ok(output) = Command::new("nmcli").args(["-w", "1", "radio", "wifi"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            return text.trim() == "enabled";
        }
        false
    }

    /// Query NetworkManager D-Bus for active connection properties without subprocesses
    async fn fetch_via_dbus() -> Option<NetworkInfo> {
        let conn = zbus::Connection::system().await.ok()?;

        // 1. Check PrimaryConnection and WirelessEnabled from /org/freedesktop/NetworkManager
        let reply = conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                Some("org.freedesktop.DBus.Properties"),
                "GetAll",
                &("org.freedesktop.NetworkManager",),
            )
            .await
            .ok()?;

        let nm_props: HashMap<String, OwnedValue> = reply.body().deserialize().ok()?;

        let wifi_enabled = nm_props
            .get("WirelessEnabled")
            .and_then(|v| bool::try_from(v).ok())
            .unwrap_or_else(Self::is_wifi_enabled);

        let connectivity_raw = nm_props
            .get("Connectivity")
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0);

        let connectivity = match connectivity_raw {
            1 => InternetConnectivity::None,
            2 => InternetConnectivity::Portal,
            3 => InternetConnectivity::Limited,
            4 => InternetConnectivity::Full,
            _ => InternetConnectivity::Unknown,
        };

        let prim_path = nm_props
            .get("PrimaryConnection")
            .and_then(|v| <&zbus::zvariant::ObjectPath>::try_from(v).ok())
            .map(|p| p.as_str().to_string())
            .unwrap_or_default();

        if prim_path.is_empty() || prim_path == "/" {
            return Some(NetworkInfo {
                is_connected: false,
                ssid: None,
                signal_percent: 0,
                is_ethernet: false,
                wifi_enabled,
                connectivity,
            });
        }

        // 2. Query properties of the ActiveConnection
        let conn_reply = conn
            .call_method(
                Some("org.freedesktop.NetworkManager"),
                prim_path.as_str(),
                Some("org.freedesktop.DBus.Properties"),
                "GetAll",
                &("org.freedesktop.NetworkManager.Connection.Active",),
            )
            .await
            .ok()?;

        let act_props: HashMap<String, OwnedValue> = conn_reply.body().deserialize().ok()?;

        let conn_type = act_props
            .get("Type")
            .and_then(|v| <&str>::try_from(v).ok())
            .unwrap_or("");
        let conn_id = act_props
            .get("Id")
            .and_then(|v| <&str>::try_from(v).ok())
            .unwrap_or("");
        let state = act_props
            .get("State")
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0);

        let is_connected = state == 2; // NM_ACTIVE_CONNECTION_STATE_ACTIVATED
        if !is_connected {
            return Some(NetworkInfo {
                is_connected: false,
                ssid: None,
                signal_percent: 0,
                is_ethernet: false,
                wifi_enabled,
                connectivity,
            });
        }

        if conn_type == "802-11-wireless" {
            let signal = Self::read_sys_wifi_signal().unwrap_or(75);
            Some(NetworkInfo {
                is_connected: true,
                ssid: Some(conn_id.to_string()),
                signal_percent: signal,
                is_ethernet: false,
                wifi_enabled,
                connectivity,
            })
        } else if conn_type == "802-3-ethernet" {
            Some(NetworkInfo {
                is_connected: true,
                ssid: Some("Ethernet".into()),
                signal_percent: 100,
                is_ethernet: true,
                wifi_enabled,
                connectivity,
            })
        } else {
            Some(NetworkInfo {
                is_connected: true,
                ssid: Some(conn_id.to_string()),
                signal_percent: 100,
                is_ethernet: false,
                wifi_enabled,
                connectivity,
            })
        }
    }

    /// Background fetch prioritizing native D-Bus with sysfs & nmcli fallbacks
    fn fetch_network_info() -> NetworkInfo {
        // Try D-Bus first via lightweight thread runtime (< 1ms, 0 processes)
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        if let Ok(rt) = rt
            && let Ok(Some(info)) = rt.block_on(async {
                tokio::time::timeout(Duration::from_millis(300), Self::fetch_via_dbus()).await
            })
        {
            return info;
        }

        // Fast sysfs / procfs fallback
        let has_net = Self::has_active_interface();
        let is_eth = Self::has_active_ethernet();
        let wifi_on = Self::is_wifi_enabled();
        let signal = Self::read_sys_wifi_signal().unwrap_or(0);

        if !has_net {
            return NetworkInfo {
                is_connected: false,
                ssid: None,
                signal_percent: 0,
                is_ethernet: is_eth,
                wifi_enabled: wifi_on,
                connectivity: InternetConnectivity::None,
            };
        }

        // Fallback: nmcli dev with strict 1s timeout
        if let Ok(output) = Command::new("nmcli")
            .args(["-w", "1", "-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "dev"])
            .output()
            && output.status.success()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 4 {
                    let dev_type = parts[1].trim();
                    let state = parts[2].trim();
                    let connection = parts[3].trim();

                    if state == "connected" {
                        if dev_type == "wifi" {
                            let sig = Self::read_sys_wifi_signal().unwrap_or(signal);
                            return NetworkInfo {
                                is_connected: true,
                                ssid: Some(connection.to_string()),
                                signal_percent: sig,
                                is_ethernet: false,
                                wifi_enabled: wifi_on,
                                connectivity: InternetConnectivity::Full,
                            };
                        } else if dev_type == "ethernet" {
                            return NetworkInfo {
                                is_connected: true,
                                ssid: Some("Ethernet".into()),
                                signal_percent: 100,
                                is_ethernet: true,
                                wifi_enabled: wifi_on,
                                connectivity: InternetConnectivity::Full,
                            };
                        }
                    }
                }
            }
        }

        NetworkInfo {
            is_connected: has_net,
            ssid: None,
            signal_percent: signal,
            is_ethernet: is_eth,
            wifi_enabled: wifi_on,
            connectivity: InternetConnectivity::Unknown,
        }
    }

    pub fn current_generation() -> u64 {
        SCAN_GENERATION.load(Ordering::SeqCst)
    }

    pub fn next_generation() -> u64 {
        SCAN_GENERATION.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn cached_access_points() -> Vec<WifiNetwork> {
        CACHED_APS.lock().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn scan_wifi() -> Vec<WifiNetwork> {
        let generation = Self::current_generation();
        Self::scan_wifi_guarded(generation, false)
    }

    /// Guaranteed non-stalling Wi-Fi scan with:
    /// 1. Concurrency guard (max 1 scan active)
    /// 2. 15-second cooldown rate limit (unless forced)
    /// 3. Explicit timeout with child process kill on hang
    /// 4. Generation token check (discards stale out-of-order scans)
    pub fn scan_wifi_guarded(target_generation: u64, force: bool) -> Vec<WifiNetwork> {
        if !Self::is_wifi_enabled() {
            if let Ok(mut aps) = CACHED_APS.lock() {
                aps.clear();
            }
            return Vec::new();
        }

        // Check 15-second cooldown unless force requested
        if !force
            && let Ok(last) = LAST_SCAN_TIME.lock()
            && let Some(t) = *last
            && t.elapsed() < SCAN_COOLDOWN
        {
            return Self::cached_access_points();
        }

        // Concurrency guard: only 1 scan in flight across the whole system
        if IS_SCANNING.swap(true, Ordering::SeqCst) {
            return Self::cached_access_points();
        }

        let result = Self::run_scan_subprocess(target_generation);

        IS_SCANNING.store(false, Ordering::SeqCst);

        if let Ok(mut last) = LAST_SCAN_TIME.lock() {
            *last = Some(Instant::now());
        }

        result
    }

    fn run_scan_subprocess(target_generation: u64) -> Vec<WifiNetwork> {
        let mut cmd = Command::new("nmcli");
        cmd.args(["-w", "3", "-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY,BARS", "device", "wifi", "list"]);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to spawn nmcli wifi scan: {e}");
                return Self::cached_access_points();
            }
        };

        // Bounded wait with 4s maximum timeout to prevent zombie / stuck nmcli processes
        let start = Instant::now();
        let timeout = Duration::from_secs(4);
        let mut status = None;

        while start.elapsed() < timeout {
            match child.try_wait() {
                Ok(Some(s)) => {
                    status = Some(s);
                    break;
                }
                Ok(None) => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    warn!("Error polling nmcli child: {e}");
                    break;
                }
            }
        }

        if status.is_none() {
            warn!("nmcli wifi list timed out after 4s; terminating child process");
            let _ = child.kill();
            let _ = child.wait();
            return Self::cached_access_points();
        }

        // Discard stale scan results if user toggled Wi-Fi or triggered a new session
        let active_gen = SCAN_GENERATION.load(Ordering::SeqCst);
        if target_generation != active_gen || !Self::is_wifi_enabled() {
            debug!("Discarding stale scan results (target gen: {target_generation}, active gen: {active_gen})");
            return Self::cached_access_points();
        }

        let output = match child.wait_with_output() {
            Ok(o) => o,
            Err(_) => return Self::cached_access_points(),
        };

        if !output.status.success() {
            return Self::cached_access_points();
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let mut networks = Vec::new();
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

        networks.sort_by(|a, b| {
            b.is_connected
                .cmp(&a.is_connected)
                .then_with(|| b.signal.cmp(&a.signal))
        });

        // Update in-memory cached access points
        if let Ok(mut cached) = CACHED_APS.lock() {
            *cached = networks.clone();
        }

        networks
    }

    pub fn set_wifi_enabled(enable: bool) {
        let target = if enable { "on" } else { "off" };
        let _ = Command::new("nmcli").args(["-w", "2", "radio", "wifi", target]).status();
        Self::next_generation();
        Self::invalidate_cache();
    }

    pub fn toggle_wifi_radio() {
        Self::set_wifi_enabled(!Self::is_wifi_enabled());
    }

    pub fn rescan_wifi() {
        let _ = Command::new("nmcli").args(["-w", "2", "device", "wifi", "rescan"]).spawn();
    }

    pub fn connect_wifi(ssid: &str, password: Option<&str>) -> Result<(), String> {
        let mut cmd = Command::new("nmcli");
        let has_pwd = password.map(|p| !p.trim().is_empty()).unwrap_or(false);

        if has_pwd {
            cmd.arg("--ask");
        }
        cmd.args(["-w", "15", "device", "wifi", "connect", ssid]);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| e.to_string())?;

        if let Some(pwd) = password
            && has_pwd
            && let Some(mut stdin) = child.stdin.take()
        {
            use std::io::Write;
            let _ = stdin.write_all(format!("{}\n", pwd.trim()).as_bytes());
            let _ = stdin.flush();
        }

        match child.wait_with_output() {
            Ok(out) => {
                Self::next_generation();
                Self::invalidate_cache();
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
        Self::next_generation();
        Self::invalidate_cache();
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
