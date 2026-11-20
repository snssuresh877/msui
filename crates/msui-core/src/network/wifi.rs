use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tracing::{debug, warn};
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

use crate::network::access_point::AccessPointParser;
use crate::network::cache::CacheManager;
use crate::network::connection::{
    DbusConnection, DBUS_PROPERTIES_IFACE, DEFAULT_DBUS_TIMEOUT, NM_IFACE, NM_PATH, NM_SERVICE,
    NM_WIRELESS_IFACE,
};
use crate::network::device::DeviceManager;
use crate::network::types::{WifiDetails, WifiNetwork};

static IS_SCANNING: AtomicBool = AtomicBool::new(false);
const SCAN_COOLDOWN: Duration = Duration::from_secs(12);

pub struct WifiManager;

impl WifiManager {
    /// Read cached access points immediately (< 0.001ms)
    pub fn get_cached_networks() -> Vec<WifiNetwork> {
        CacheManager::get_access_points()
    }

    /// Guaranteed non-blocking Wi-Fi scan using native D-Bus
    /// Concurrency-guarded with cooldown rate limiting and bounded timeouts.
    pub async fn scan_networks(force: bool) -> Vec<WifiNetwork> {
        // Return existing cached networks if cooldown has not elapsed (unless force requested)
        if !force
            && let Ok(guard) = crate::network::cache::NETWORK_CACHE.read()
                && let Some(t) = guard.last_scan_time
                && t.elapsed() < SCAN_COOLDOWN
                && !guard.access_points.is_empty()
            {
                return guard.access_points.clone();
            }

        // Concurrency guard: only 1 scan in flight across the desktop
        if IS_SCANNING.swap(true, Ordering::SeqCst) {
            return CacheManager::get_access_points();
        }

        let result = Self::execute_dbus_scan(force).await;

        IS_SCANNING.store(false, Ordering::SeqCst);

        result
    }

    /// Execute D-Bus scan against NetworkManager
    async fn execute_dbus_scan(force: bool) -> Vec<WifiNetwork> {
        let dbus = match DbusConnection::system().await {
            Ok(d) => d,
            Err(e) => {
                warn!("Cannot connect to D-Bus for Wi-Fi scan: {e}");
                return CacheManager::get_access_points();
            }
        };

        // 1. Locate primary Wi-Fi device
        let (wifi_path, _iface) = match DeviceManager::find_primary_wifi_device(&dbus).await {
            Some(pair) => pair,
            None => {
                debug!("No Wi-Fi device found on system");
                return Vec::new();
            }
        };

        // 2. Read ActiveAccessPoint property if currently connected
        let active_ap = Self::read_active_access_point(&dbus, &wifi_path).await;

        // 3. Request hardware scan if forced
        if force {
            let options: HashMap<&str, Value<'_>> = HashMap::new();
            let _ = dbus.raw().call_method(
                Some(NM_SERVICE),
                wifi_path.as_str(),
                Some(NM_WIRELESS_IFACE),
                "RequestScan",
                &(options,),
            ).await;
            // Brief non-blocking wait for hardware results to register
            tokio::time::sleep(Duration::from_millis(600)).await;
        }

        // 4. Retrieve all known access points via GetAllAccessPoints
        let call_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            wifi_path.as_str(),
            Some(NM_WIRELESS_IFACE),
            "GetAllAccessPoints",
            &(),
        );

        let reply = match tokio::time::timeout(Duration::from_millis(600), call_fut).await {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                warn!("GetAllAccessPoints failed: {e}");
                return CacheManager::get_access_points();
            }
            Err(_) => {
                warn!("GetAllAccessPoints timed out");
                return CacheManager::get_access_points();
            }
        };

        let ap_paths: Vec<OwnedObjectPath> = match reply.body().deserialize() {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to deserialize AP paths: {e}");
                return CacheManager::get_access_points();
            }
        };

        // 5. Parse each Access Point
        let mut raw_networks = Vec::with_capacity(ap_paths.len());
        for path in ap_paths {
            if let Ok(net) = AccessPointParser::parse_ap(
                &dbus,
                path.as_str(),
                active_ap.as_deref(),
            )
            .await
            {
                raw_networks.push(net);
            }
        }

        // 6. Deduplicate by SSID, preserving the strongest signal
        let mut dedup_map: HashMap<String, WifiNetwork> = HashMap::new();
        for net in raw_networks {
            if let Some(existing) = dedup_map.get_mut(&net.ssid) {
                if net.is_connected || (!existing.is_connected && net.signal > existing.signal) {
                    *existing = net;
                }
            } else {
                dedup_map.insert(net.ssid.clone(), net);
            }
        }

        let mut networks: Vec<WifiNetwork> = dedup_map.into_values().collect();

        // 7. Sort: connected first, then highest signal descending
        networks.sort_by(|a, b| {
            b.is_connected
                .cmp(&a.is_connected)
                .then_with(|| b.signal.cmp(&a.signal))
        });

        // 8. Update cache
        CacheManager::update_access_points(networks.clone());

        networks
    }

    /// Read active access point object path from wireless device
    async fn read_active_access_point(dbus: &DbusConnection, wifi_path: &str) -> Option<String> {
        let call_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            wifi_path,
            Some(DBUS_PROPERTIES_IFACE),
            "Get",
            &(NM_WIRELESS_IFACE, "ActiveAccessPoint"),
        );

        let reply = tokio::time::timeout(Duration::from_millis(200), call_fut).await.ok()?.ok()?;
        let val: OwnedValue = reply.body().deserialize().ok()?;
        let path = <&ObjectPath>::try_from(&val).ok()?;
        let s = path.as_str();
        if s == "/" || s.is_empty() {
            None
        } else {
            Some(s.to_string())
        }
    }

    /// Toggle Wi-Fi radio on/off natively via D-Bus
    pub async fn set_radio_enabled(enabled: bool) -> Result<(), String> {
        let dbus = DbusConnection::system().await?;

        let val = Value::from(enabled);
        let body = (NM_IFACE, "WirelessEnabled", &val);
        let call_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            NM_PATH,
            Some(DBUS_PROPERTIES_IFACE),
            "Set",
            &body,
        );

        tokio::time::timeout(DEFAULT_DBUS_TIMEOUT, call_fut)
            .await
            .map_err(|_| "WirelessEnabled Set timed out".to_string())?
            .map_err(|e| format!("Failed to set WirelessEnabled: {e}"))?;

        CacheManager::invalidate();
        let _ = crate::network::service::NetworkService::refresh_async().await;

        Ok(())
    }

    /// Connect to a Wi-Fi network natively over D-Bus with optional password
    pub async fn connect_network(ssid: &str, password: Option<&str>) -> Result<(), String> {
        let dbus = DbusConnection::system().await?;

        let (wifi_path, _iface) = DeviceManager::find_primary_wifi_device(&dbus)
            .await
            .ok_or_else(|| "No Wi-Fi interface available to connect".to_string())?;

        // 1. Check if an existing profile exists for this SSID
        if let Some((conn_path, _profile)) =
            crate::network::profiles::ProfilesManager::find_profile_by_ssid(&dbus, ssid).await
        {
            debug!("Found existing profile at {conn_path} for SSID '{ssid}'");

            if let Some(pw) = password
                && !pw.is_empty()
            {
                // Update secrets / PSK on the existing connection
                let mut settings =
                    crate::network::profiles::ProfilesManager::get_settings(&dbus, &conn_path)
                        .await
                        .unwrap_or_default();
                let mut sec_section = settings
                    .remove("802-11-wireless-security")
                    .unwrap_or_default();
                if let Ok(v) = OwnedValue::try_from(Value::from("wpa-psk")) {
                    sec_section.insert("key-mgmt".to_string(), v);
                }
                if let Ok(v) = OwnedValue::try_from(Value::from("open")) {
                    sec_section.insert("auth-alg".to_string(), v);
                }
                if let Ok(v) = OwnedValue::try_from(Value::from(pw.to_string())) {
                    sec_section.insert("psk".to_string(), v);
                }
                settings.insert("802-11-wireless-security".to_string(), sec_section);

                let update_args = (settings,);
                let call_fut = dbus.raw().call_method(
                    Some(NM_SERVICE),
                    conn_path.as_str(),
                    Some(crate::network::profiles::NM_SETTINGS_CONN_IFACE),
                    "Update",
                    &update_args,
                );
                let _ = tokio::time::timeout(DEFAULT_DBUS_TIMEOUT, call_fut).await;
            }

            // Activate existing connection
            let conn_obj = ObjectPath::try_from(conn_path.as_str()).map_err(|e| e.to_string())?;
            let wifi_obj = ObjectPath::try_from(wifi_path.as_str()).map_err(|e| e.to_string())?;
            let ap_obj = ObjectPath::try_from("/").unwrap();
            let act_args = (conn_obj, wifi_obj, ap_obj);
            let call_fut = dbus.raw().call_method(
                Some(NM_SERVICE),
                NM_PATH,
                Some(NM_IFACE),
                "ActivateConnection",
                &act_args,
            );

            tokio::time::timeout(DEFAULT_DBUS_TIMEOUT, call_fut)
                .await
                .map_err(|_| "ActivateConnection timed out".to_string())?
                .map_err(|e| format!("ActivateConnection failed: {e}"))?;
        } else {
            // 2. Create a new connection profile and activate
            debug!("Creating new connection profile for SSID '{ssid}'");
            let mut settings: HashMap<&str, HashMap<&str, Value<'_>>> = HashMap::new();

            let uuid = generate_uuid();
            let mut conn_section: HashMap<&str, Value<'_>> = HashMap::new();
            conn_section.insert("id", Value::from(ssid));
            conn_section.insert("type", Value::from("802-11-wireless"));
            conn_section.insert("uuid", Value::from(uuid.as_str()));
            settings.insert("connection", conn_section);

            let mut wifi_section: HashMap<&str, Value<'_>> = HashMap::new();
            wifi_section.insert("mode", Value::from("infrastructure"));
            let ssid_bytes = ssid.as_bytes().to_vec();
            wifi_section.insert("ssid", Value::from(ssid_bytes));
            if let Some(pw) = password
                && !pw.is_empty()
            {
                wifi_section.insert("security", Value::from("802-11-wireless-security"));
                let mut sec_section: HashMap<&str, Value<'_>> = HashMap::new();
                sec_section.insert("key-mgmt", Value::from("wpa-psk"));
                sec_section.insert("auth-alg", Value::from("open"));
                sec_section.insert("psk", Value::from(pw));
                settings.insert("802-11-wireless-security", sec_section);
            }
            settings.insert("802-11-wireless", wifi_section);

            let mut ipv4_section: HashMap<&str, Value<'_>> = HashMap::new();
            ipv4_section.insert("method", Value::from("auto"));
            settings.insert("ipv4", ipv4_section);

            let mut ipv6_section: HashMap<&str, Value<'_>> = HashMap::new();
            ipv6_section.insert("method", Value::from("auto"));
            settings.insert("ipv6", ipv6_section);

            let wifi_obj = ObjectPath::try_from(wifi_path.as_str()).map_err(|e| e.to_string())?;
            let ap_obj = ObjectPath::try_from("/").unwrap();
            let add_args = (settings, wifi_obj, ap_obj);
            let call_fut = dbus.raw().call_method(
                Some(NM_SERVICE),
                NM_PATH,
                Some(NM_IFACE),
                "AddAndActivateConnection",
                &add_args,
            );

            tokio::time::timeout(DEFAULT_DBUS_TIMEOUT, call_fut)
                .await
                .map_err(|_| "AddAndActivateConnection timed out".to_string())?
                .map_err(|e| format!("AddAndActivateConnection failed: {e}"))?;
        }

        CacheManager::invalidate();
        let _ = crate::network::service::NetworkService::refresh_async().await;

        Ok(())
    }

    /// Disconnect Wi-Fi device natively over D-Bus
    pub async fn disconnect_network() -> Result<(), String> {
        let dbus = DbusConnection::system().await?;

        let (wifi_path, _iface) = DeviceManager::find_primary_wifi_device(&dbus)
            .await
            .ok_or_else(|| "No Wi-Fi interface available to disconnect".to_string())?;

        let call_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            wifi_path.as_str(),
            Some(crate::network::connection::NM_DEVICE_IFACE),
            "Disconnect",
            &(),
        );

        tokio::time::timeout(DEFAULT_DBUS_TIMEOUT, call_fut)
            .await
            .map_err(|_| "Device Disconnect timed out".to_string())?
            .map_err(|e| format!("Device Disconnect failed: {e}"))?;

        CacheManager::invalidate();
        let _ = crate::network::service::NetworkService::refresh_async().await;

        Ok(())
    }

    /// Read comprehensive Wi-Fi connection telemetry (IP, gateway, DNS, bitrate, frequency, signal)
    pub async fn get_connection_details() -> Option<WifiDetails> {
        let dbus = DbusConnection::system().await.ok()?;
        let (wifi_path, iface_name) = DeviceManager::find_primary_wifi_device(&dbus).await?;

        // 1. Bitrate from Wi-Fi device
        let bitrate_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            wifi_path.as_str(),
            Some(DBUS_PROPERTIES_IFACE),
            "Get",
            &(NM_WIRELESS_IFACE, "Bitrate"),
        );
        let bitrate_kbps: Option<u32> = if let Ok(Ok(reply)) =
            tokio::time::timeout(Duration::from_millis(200), bitrate_fut).await
        {
            let val: OwnedValue = reply.body().deserialize().ok()?;
            u32::try_from(&val).ok()
        } else {
            None
        };
        let bitrate_str = bitrate_kbps.map(|kbps| {
            if kbps >= 1000 {
                format!("{} Mb/s", kbps / 1000)
            } else {
                format!("{kbps} Kb/s")
            }
        });

        // 2. Active Access Point details
        let active_ap_path = Self::read_active_access_point(&dbus, &wifi_path).await;
        let mut frequency_mhz = None;
        let mut signal_percent = 0;
        let mut security_str = None;
        let mut ssid_str = None;

        if let Some(ref ap_path) = active_ap_path
            && let Ok(ap_info) = AccessPointParser::parse_ap(&dbus, ap_path, Some(ap_path)).await {
                ssid_str = Some(ap_info.ssid);
                signal_percent = ap_info.signal;
                frequency_mhz = Some(ap_info.frequency_mhz);
                security_str = Some(ap_info.security);
            }

        // 3. Active Connection and IP4Config
        let primary_conn_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            NM_PATH,
            Some(DBUS_PROPERTIES_IFACE),
            "Get",
            &(NM_IFACE, "PrimaryConnection"),
        );
        let mut ip_address = None;
        let mut gateway = None;
        let mut dns = None;

        if let Ok(Ok(reply)) =
            tokio::time::timeout(Duration::from_millis(200), primary_conn_fut).await
            && let Ok(val) = reply.body().deserialize::<OwnedValue>()
            && let Ok(path) = <&ObjectPath>::try_from(&val)
            && path.as_str() != "/"
        {
            let primary_path = path.as_str();

            // Read Ip4Config from active connection
            let ip4_fut = dbus.raw().call_method(
                Some(NM_SERVICE),
                primary_path,
                Some(DBUS_PROPERTIES_IFACE),
                "Get",
                &(crate::network::connection::NM_ACTIVE_CONN_IFACE, "Ip4Config"),
            );

            if let Ok(Ok(ip4_reply)) = tokio::time::timeout(Duration::from_millis(200), ip4_fut).await
                && let Ok(ip4_val) = ip4_reply.body().deserialize::<OwnedValue>()
                && let Ok(ip4_path) = <&ObjectPath>::try_from(&ip4_val)
                && ip4_path.as_str() != "/"
            {
                // Read properties of IP4Config
                let ip4_props_fut = dbus.raw().call_method(
                    Some(NM_SERVICE),
                    ip4_path.as_str(),
                    Some(DBUS_PROPERTIES_IFACE),
                    "GetAll",
                    &(crate::network::connection::NM_IP4_CONFIG_IFACE,),
                );

                if let Ok(Ok(props_reply)) =
                    tokio::time::timeout(Duration::from_millis(250), ip4_props_fut).await
                    && let Ok(props) =
                        props_reply.body().deserialize::<HashMap<String, OwnedValue>>()
                {
                    // Gateway
                    if let Some(gw_val) = props.get("Gateway")
                        && let Ok(gw) = <&str>::try_from(gw_val)
                        && !gw.is_empty()
                    {
                        gateway = Some(gw.to_string());
                    }

                    // AddressData: aa{sv}
                    if let Some(addr_val) = props.get("AddressData")
                        && let Ok(addr_arr) = <&zbus::zvariant::Array>::try_from(addr_val)
                        && let Some(first_dict) = addr_arr.inner().first()
                        && let Ok(dict) = <&zbus::zvariant::Dict>::try_from(first_dict)
                    {
                        for (k, v) in dict.iter() {
                            if let Ok(key) = <&str>::try_from(k)
                                && key == "address"
                                && let Ok(ip) = <&str>::try_from(v) {
                                    ip_address = Some(ip.to_string());
                                    break;
                                }
                        }
                    }

                    // NameserverData: aa{sv}
                    if let Some(dns_val) = props.get("NameserverData")
                        && let Ok(dns_arr) = <&zbus::zvariant::Array>::try_from(dns_val)
                        && let Some(first_dict) = dns_arr.inner().first()
                        && let Ok(dict) = <&zbus::zvariant::Dict>::try_from(first_dict)
                    {
                        for (k, v) in dict.iter() {
                            if let Ok(key) = <&str>::try_from(k)
                                && key == "address"
                                && let Ok(dns_addr) = <&str>::try_from(v) {
                                    dns = Some(dns_addr.to_string());
                                    break;
                                }
                        }
                    }
                }
            }
        }

        let signal_dbm = if signal_percent > 0 {
            Some((signal_percent as i32 / 2) - 100)
        } else {
            None
        };

        Some(WifiDetails {
            ssid: ssid_str,
            ip_address,
            gateway,
            dns,
            frequency_mhz,
            signal_dbm,
            signal_percent,
            bitrate: bitrate_str,
            interface_name: Some(iface_name),
            security: security_str,
        })
    }
}

fn generate_uuid() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let nanos = now.as_nanos();
    let pid = std::process::id();
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (nanos & 0xffffffff) as u32,
        ((nanos >> 32) & 0xffff) as u16,
        ((nanos >> 48) & 0x0fff) as u16,
        (0x8000 | (pid & 0x3fff)) as u16,
        ((nanos >> 64) ^ ((pid as u128) << 32)) & 0xffffffffffff
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cached_networks_empty() {
        let list = WifiManager::get_cached_networks();
        // Should not panic, returns Vec
        let _ = list.len();
    }

    #[tokio::test]
    async fn test_scan_networks_live() {
        // Run a non-force scan over native D-Bus
        let nets = WifiManager::scan_networks(false).await;
        // On a running machine with NetworkManager, networks should either be found or safely return empty
        let _ = nets.len();
    }

    #[tokio::test]
    async fn test_get_connection_details() {
        let details = WifiManager::get_connection_details().await;
        // Should execute cleanly without error or panic
        if let Some(d) = details {
            let _ = d.ip_address;
            let _ = d.bitrate;
        }
    }
}

