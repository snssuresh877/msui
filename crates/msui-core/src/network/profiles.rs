use std::collections::HashMap;
use tracing::debug;
use zbus::zvariant::{Array, OwnedObjectPath, OwnedValue};

use crate::network::cache::CacheManager;
use crate::network::connection::{
    DbusConnection, DEFAULT_DBUS_TIMEOUT, NM_SERVICE, NM_SETTINGS_IFACE, NM_SETTINGS_PATH,
};
use crate::network::types::SavedProfile;

pub const NM_SETTINGS_CONN_IFACE: &str = "org.freedesktop.NetworkManager.Settings.Connection";

pub struct ProfilesManager;

impl ProfilesManager {
    /// List all saved NetworkManager profiles
    pub async fn list_profiles(dbus: &DbusConnection) -> Result<Vec<SavedProfile>, String> {
        let call_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            NM_SETTINGS_PATH,
            Some(NM_SETTINGS_IFACE),
            "ListConnections",
            &(),
        );

        let reply = tokio::time::timeout(DEFAULT_DBUS_TIMEOUT, call_fut)
            .await
            .map_err(|_| "ListConnections timed out".to_string())?
            .map_err(|e| format!("ListConnections failed: {e}"))?;

        let conn_paths: Vec<OwnedObjectPath> = reply
            .body()
            .deserialize()
            .map_err(|e| format!("Failed to deserialize connection paths: {e}"))?;

        let mut profiles = Vec::with_capacity(conn_paths.len());

        for path in conn_paths {
            if let Ok(profile) = Self::read_profile(dbus, path.as_str()).await {
                profiles.push(profile);
            }
        }

        // Sort most recently used first
        profiles.sort_by_key(|b| std::cmp::Reverse(b.timestamp));

        Ok(profiles)
    }

    /// Read single profile settings from a connection object path
    pub async fn read_profile(dbus: &DbusConnection, path: &str) -> Result<SavedProfile, String> {
        let settings = Self::get_settings(dbus, path).await?;

        let (id, uuid, conn_type, timestamp, autoconnect) = if let Some(conn) = settings.get("connection") {
            let id = conn
                .get("id")
                .and_then(|v| <&str>::try_from(v).ok())
                .unwrap_or("")
                .to_string();
            let uuid = conn
                .get("uuid")
                .and_then(|v| <&str>::try_from(v).ok())
                .unwrap_or("")
                .to_string();
            let conn_type = conn
                .get("type")
                .and_then(|v| <&str>::try_from(v).ok())
                .unwrap_or("")
                .to_string();
            let timestamp = conn
                .get("timestamp")
                .and_then(|v| u64::try_from(v).ok())
                .unwrap_or(0);
            let autoconnect = conn
                .get("autoconnect")
                .and_then(|v| bool::try_from(v).ok())
                .unwrap_or(true);
            (id, uuid, conn_type, timestamp, autoconnect)
        } else {
            return Err("Missing 'connection' section in settings".to_string());
        };

        Ok(SavedProfile {
            id,
            uuid,
            conn_type,
            autoconnect,
            timestamp,
        })
    }

    /// Retrieve the full settings dictionary for a connection object path
    pub async fn get_settings(
        dbus: &DbusConnection,
        path: &str,
    ) -> Result<HashMap<String, HashMap<String, OwnedValue>>, String> {
        let call_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            path,
            Some(NM_SETTINGS_CONN_IFACE),
            "GetSettings",
            &(),
        );

        let reply = tokio::time::timeout(DEFAULT_DBUS_TIMEOUT, call_fut)
            .await
            .map_err(|_| format!("GetSettings for {path} timed out"))?
            .map_err(|e| format!("GetSettings for {path} failed: {e}"))?;

        let settings: HashMap<String, HashMap<String, OwnedValue>> = reply
            .body()
            .deserialize()
            .map_err(|e| format!("Failed to deserialize settings: {e}"))?;

        Ok(settings)
    }

    /// Find an existing Wi-Fi profile by SSID or Connection ID
    pub async fn find_profile_by_ssid(
        dbus: &DbusConnection,
        target_ssid: &str,
    ) -> Option<(String, SavedProfile)> {
        let call_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            NM_SETTINGS_PATH,
            Some(NM_SETTINGS_IFACE),
            "ListConnections",
            &(),
        );

        let reply = tokio::time::timeout(DEFAULT_DBUS_TIMEOUT, call_fut).await.ok()?.ok()?;
        let conn_paths: Vec<OwnedObjectPath> = reply.body().deserialize().ok()?;

        for path in conn_paths {
            let path_str = path.as_str();
            if let Ok(settings) = Self::get_settings(dbus, path_str).await {
                let mut matched = false;

                // Check 802-11-wireless.ssid
                if let Some(wifi) = settings.get("802-11-wireless")
                    && let Some(v) = wifi.get("ssid")
                        && let Ok(arr) = <&Array>::try_from(v) {
                            let bytes: Vec<u8> = arr
                                .inner()
                                .iter()
                                .filter_map(|val| u8::try_from(val).ok())
                                .collect();
                            let s = String::from_utf8_lossy(&bytes).trim().to_string();
                            if s == target_ssid {
                                matched = true;
                            }
                        }

                // Check connection.id
                if !matched && let Some(conn) = settings.get("connection")
                    && let Some(v) = conn.get("id")
                        && let Ok(id_str) = <&str>::try_from(v)
                            && id_str == target_ssid {
                                matched = true;
                            }

                if matched
                    && let Ok(profile) = Self::read_profile(dbus, path_str).await {
                        return Some((path_str.to_string(), profile));
                    }
            }
        }

        None
    }

    /// Forget / Delete a saved profile by UUID or SSID
    pub async fn forget_profile(uuid_or_ssid: &str) -> Result<(), String> {
        let dbus = DbusConnection::system().await?;

        // 1. List connections
        let call_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            NM_SETTINGS_PATH,
            Some(NM_SETTINGS_IFACE),
            "ListConnections",
            &(),
        );

        let reply = tokio::time::timeout(DEFAULT_DBUS_TIMEOUT, call_fut)
            .await
            .map_err(|_| "ListConnections timed out".to_string())?
            .map_err(|e| format!("ListConnections failed: {e}"))?;

        let conn_paths: Vec<OwnedObjectPath> = reply
            .body()
            .deserialize()
            .map_err(|e| format!("Failed to deserialize connection paths: {e}"))?;

        let mut target_path: Option<String> = None;

        for path in conn_paths {
            let path_str = path.as_str();
            if let Ok(settings) = Self::get_settings(&dbus, path_str).await
                && let Some(conn) = settings.get("connection") {
                    let uuid_match = conn
                        .get("uuid")
                        .and_then(|v| <&str>::try_from(v).ok())
                        .map(|u| u == uuid_or_ssid)
                        .unwrap_or(false);
                    let id_match = conn
                        .get("id")
                        .and_then(|v| <&str>::try_from(v).ok())
                        .map(|i| i == uuid_or_ssid)
                        .unwrap_or(false);

                    let ssid_match = settings
                        .get("802-11-wireless")
                        .and_then(|w| w.get("ssid"))
                        .and_then(|v| {
                            let arr = <&Array>::try_from(v).ok()?;
                            let bytes: Vec<u8> = arr
                                .inner()
                                .iter()
                                .filter_map(|val| u8::try_from(val).ok())
                                .collect();
                            Some(String::from_utf8_lossy(&bytes).trim() == uuid_or_ssid)
                        })
                        .unwrap_or(false);

                    if uuid_match || id_match || ssid_match {
                        target_path = Some(path_str.to_string());
                        break;
                    }
                }
        }

        let delete_path = target_path
            .ok_or_else(|| format!("Network profile '{uuid_or_ssid}' not found in saved connections"))?;

        debug!("Deleting network connection profile at {delete_path}");
        let call_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            delete_path.as_str(),
            Some(NM_SETTINGS_CONN_IFACE),
            "Delete",
            &(),
        );

        tokio::time::timeout(DEFAULT_DBUS_TIMEOUT, call_fut)
            .await
            .map_err(|_| format!("Delete profile timed out for {delete_path}"))?
            .map_err(|e| format!("Delete profile failed: {e}"))?;

        CacheManager::invalidate();
        let _ = crate::network::service::NetworkService::refresh_async().await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_saved_profiles() {
        if let Ok(dbus) = DbusConnection::system().await {
            let res = ProfilesManager::list_profiles(&dbus).await;
            assert!(res.is_ok());
            let profiles = res.unwrap();
            let _ = profiles.len();
        }
    }
}
