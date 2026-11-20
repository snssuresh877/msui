use std::collections::HashMap;
use std::time::Duration;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

use crate::network::connection::{
    DbusConnection, DBUS_PROPERTIES_IFACE, DEFAULT_DBUS_TIMEOUT, NM_DEVICE_IFACE, NM_PATH,
    NM_SERVICE,
};
use crate::network::types::{DeviceState, DeviceType, NetworkDevice};

pub struct DeviceManager;

impl DeviceManager {
    /// Retrieve all network devices from NetworkManager
    pub async fn get_all_devices(dbus: &DbusConnection) -> Result<Vec<NetworkDevice>, String> {
        let call_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            NM_PATH,
            Some(NM_SERVICE),
            "GetDevices",
            &(),
        );

        let reply = tokio::time::timeout(DEFAULT_DBUS_TIMEOUT, call_fut)
            .await
            .map_err(|_| "GetDevices timed out".to_string())?
            .map_err(|e| format!("GetDevices failed: {e}"))?;

        let paths: Vec<OwnedObjectPath> = reply
            .body()
            .deserialize()
            .map_err(|e| format!("Failed to parse device paths: {e}"))?;

        let mut devices = Vec::with_capacity(paths.len());
        for path in paths {
            if let Ok(dev) = Self::get_device_details(dbus, path.as_str()).await {
                devices.push(dev);
            }
        }

        Ok(devices)
    }

    /// Read details for a specific device object path
    pub async fn get_device_details(
        dbus: &DbusConnection,
        path: &str,
    ) -> Result<NetworkDevice, String> {
        let call_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            path,
            Some(DBUS_PROPERTIES_IFACE),
            "GetAll",
            &(NM_DEVICE_IFACE,),
        );

        let reply = tokio::time::timeout(Duration::from_millis(300), call_fut)
            .await
            .map_err(|_| format!("GetAll for {path} timed out"))?
            .map_err(|e| format!("GetAll for {path} failed: {e}"))?;

        let props: HashMap<String, OwnedValue> = reply
            .body()
            .deserialize()
            .map_err(|e| format!("Failed to parse device properties: {e}"))?;

        let interface = props
            .get("Interface")
            .and_then(|v| <&str>::try_from(v).ok())
            .unwrap_or("")
            .to_string();

        let device_type_raw = props
            .get("DeviceType")
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0);

        let state_raw = props
            .get("State")
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0);

        Ok(NetworkDevice {
            interface,
            path: path.to_string(),
            device_type: DeviceType::from(device_type_raw),
            state: DeviceState::from(state_raw),
        })
    }

    /// Locate primary Wi-Fi device object path and interface name
    pub async fn find_primary_wifi_device(
        dbus: &DbusConnection,
    ) -> Option<(String, String)> {
        let devices = Self::get_all_devices(dbus).await.ok()?;
        // Prefer connected Wi-Fi device, otherwise first available Wi-Fi device
        let mut first_wifi = None;
        for dev in devices {
            if dev.device_type == DeviceType::Wifi {
                if dev.state.is_connected() {
                    return Some((dev.path, dev.interface));
                }
                if first_wifi.is_none() {
                    first_wifi = Some((dev.path, dev.interface));
                }
            }
        }
        first_wifi
    }

    /// Check if physical Ethernet has carrier and is connected
    pub async fn has_active_ethernet(dbus: &DbusConnection) -> bool {
        if let Ok(devices) = Self::get_all_devices(dbus).await {
            for dev in devices {
                if dev.device_type == DeviceType::Ethernet && dev.state.is_connected() {
                    return true;
                }
            }
        }
        false
    }
}
