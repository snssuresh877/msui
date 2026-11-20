use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tracing::debug;
use zbus::zvariant::{ObjectPath, OwnedValue};

use crate::network::cache::CacheManager;
use crate::network::connection::{
    DbusConnection, DBUS_PROPERTIES_IFACE, NM_ACTIVE_CONN_IFACE, NM_IFACE, NM_PATH, NM_SERVICE,
};
use crate::network::device::DeviceManager;
use crate::network::types::{InternetConnectivity, NetworkInfo};

static IS_REFRESHING: AtomicBool = AtomicBool::new(false);

pub struct NetworkService;

impl NetworkService {
    /// Non-blocking immediate read of current network state (< 0.001ms)
    /// Guarantees UI thread never waits on D-Bus or subprocesses.
    pub fn read_state() -> NetworkInfo {
        // Trigger background refresh if stale (> 3s) and not already in flight
        if CacheManager::is_stale(Duration::from_secs(3))
            && !IS_REFRESHING.swap(true, Ordering::SeqCst)
        {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async {
                    let _ = Self::refresh_async().await;
                    IS_REFRESHING.store(false, Ordering::SeqCst);
                });
            } else {
                std::thread::spawn(|| {
                    if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        rt.block_on(async {
                            let _ = Self::refresh_async().await;
                        });
                    }
                    IS_REFRESHING.store(false, Ordering::SeqCst);
                });
            }
        }

        CacheManager::get_info()
    }

    /// Authoritative D-Bus network state refresh with fallback
    pub async fn refresh_async() -> NetworkInfo {
        match Self::query_dbus_state().await {
            Ok(info) => {
                CacheManager::update_info(info.clone());
                info
            }
            Err(e) => {
                debug!("D-Bus state query failed ({e}); using sysfs fallback");
                let fallback = Self::sysfs_fallback();
                CacheManager::update_info(fallback.clone());
                fallback
            }
        }
    }

    /// Query NetworkManager D-Bus for global networking properties
    async fn query_dbus_state() -> Result<NetworkInfo, String> {
        let dbus = DbusConnection::system().await?;

        // 1. Get global NetworkManager properties
        let call_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            NM_PATH,
            Some(DBUS_PROPERTIES_IFACE),
            "GetAll",
            &(NM_IFACE,),
        );

        let reply = tokio::time::timeout(Duration::from_millis(300), call_fut)
            .await
            .map_err(|_| "D-Bus GetAll timed out".to_string())?
            .map_err(|e| format!("D-Bus GetAll failed: {e}"))?;

        let props: HashMap<String, OwnedValue> = reply
            .body()
            .deserialize()
            .map_err(|e| format!("Deserialize props failed: {e}"))?;

        let wifi_enabled = props
            .get("WirelessEnabled")
            .and_then(|v| bool::try_from(v).ok())
            .unwrap_or_else(Self::is_wifi_rfkill_enabled);

        let connectivity_raw = props
            .get("Connectivity")
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0);

        let connectivity = InternetConnectivity::from(connectivity_raw);

        // Update cached primary wifi device path
        if let Some((path, iface)) = DeviceManager::find_primary_wifi_device(&dbus).await {
            CacheManager::set_wifi_device(iface, path);
        }

        // 2. Inspect PrimaryConnection
        let prim_path = props
            .get("PrimaryConnection")
            .and_then(|v| <&ObjectPath>::try_from(v).ok())
            .map(|p| p.as_str().to_string());

        if let Some(ref path) = prim_path
            && path != "/"
            && !path.is_empty()
            && let Ok(info) = Self::inspect_active_connection(&dbus, path, wifi_enabled, connectivity).await {
                return Ok(info);
            }

        // 3. Fallback: Check if Ethernet is active even if not primary
        let is_eth = DeviceManager::has_active_ethernet(&dbus).await;
        if is_eth {
            return Ok(NetworkInfo {
                is_connected: true,
                ssid: Some("Ethernet".into()),
                signal_percent: 100,
                is_ethernet: true,
                wifi_enabled,
                connectivity: if connectivity == InternetConnectivity::Unknown {
                    InternetConnectivity::Full
                } else {
                    connectivity
                },
            });
        }

        Ok(NetworkInfo {
            is_connected: false,
            ssid: None,
            signal_percent: 0,
            is_ethernet: false,
            wifi_enabled,
            connectivity,
        })
    }

    /// Read details from active connection object path
    async fn inspect_active_connection(
        dbus: &DbusConnection,
        conn_path: &str,
        wifi_enabled: bool,
        connectivity: InternetConnectivity,
    ) -> Result<NetworkInfo, String> {
        let call_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            conn_path,
            Some(DBUS_PROPERTIES_IFACE),
            "GetAll",
            &(NM_ACTIVE_CONN_IFACE,),
        );

        let reply = tokio::time::timeout(Duration::from_millis(250), call_fut)
            .await
            .map_err(|_| "ActiveConn GetAll timed out".to_string())?
            .map_err(|e| format!("ActiveConn GetAll failed: {e}"))?;

        let props: HashMap<String, OwnedValue> = reply
            .body()
            .deserialize()
            .map_err(|e| format!("Deserialize ActiveConn props failed: {e}"))?;

        let conn_type = props
            .get("Type")
            .and_then(|v| <&str>::try_from(v).ok())
            .unwrap_or("");

        let conn_id = props
            .get("Id")
            .and_then(|v| <&str>::try_from(v).ok())
            .unwrap_or("");

        let is_ethernet = conn_type == "802-3-ethernet";
        let is_wifi = conn_type == "802-11-wireless";

        let signal = if is_wifi {
            Self::read_sys_wifi_signal().unwrap_or(75)
        } else {
            100
        };

        Ok(NetworkInfo {
            is_connected: true,
            ssid: if conn_id.is_empty() {
                None
            } else {
                Some(conn_id.to_string())
            },
            signal_percent: signal,
            is_ethernet,
            wifi_enabled,
            connectivity,
        })
    }

    /// Fast kernel sysfs rfkill check (< 0.01ms, 0 processes)
    pub fn is_wifi_rfkill_enabled() -> bool {
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
        true
    }

    /// Read wireless signal from /proc/net/wireless in < 0.01ms
    pub fn read_sys_wifi_signal() -> Option<u8> {
        let content = std::fs::read_to_string("/proc/net/wireless").ok()?;
        for line in content.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let quality_str = parts[2].trim_end_matches('.');
                if let Ok(quality) = quality_str.parse::<f32>() {
                    let percent = ((quality / 70.0) * 100.0).clamp(0.0, 100.0) as u8;
                    return Some(percent);
                }
            }
        }
        None
    }

    /// Fast sysfs baseline when D-Bus is unreachable (offline mode)
    pub fn sysfs_fallback() -> NetworkInfo {
        let has_net = Self::has_sysfs_active_interface();
        let is_eth = Self::has_sysfs_active_ethernet();
        let wifi_on = Self::is_wifi_rfkill_enabled();
        let signal = Self::read_sys_wifi_signal().unwrap_or(0);

        NetworkInfo {
            is_connected: has_net,
            ssid: None,
            signal_percent: signal,
            is_ethernet: is_eth,
            wifi_enabled: wifi_on,
            connectivity: if has_net {
                InternetConnectivity::Unknown
            } else {
                InternetConnectivity::None
            },
        }
    }

    fn has_sysfs_active_interface() -> bool {
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

    fn has_sysfs_active_ethernet() -> bool {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_service_read_immediate() {
        let state = NetworkService::read_state();
        let _ = state.is_connected;
        let _ = state.wifi_enabled;
        let _ = state.connectivity;
    }

    #[tokio::test]
    async fn test_network_service_refresh_async() {
        let state = NetworkService::refresh_async().await;
        let _ = state.is_connected;
        println!("NetworkService live state: connected={}, ssid={:?}, wifi_enabled={}",
            state.is_connected, state.ssid, state.wifi_enabled);
    }
}
