use std::time::Duration;
use tracing::{debug, error, warn};
use zbus::Connection;

pub const NM_SERVICE: &str = "org.freedesktop.NetworkManager";
pub const NM_PATH: &str = "/org/freedesktop/NetworkManager";
pub const NM_SETTINGS_PATH: &str = "/org/freedesktop/NetworkManager/Settings";

pub const NM_IFACE: &str = "org.freedesktop.NetworkManager";
pub const NM_DEVICE_IFACE: &str = "org.freedesktop.NetworkManager.Device";
pub const NM_WIRELESS_IFACE: &str = "org.freedesktop.NetworkManager.Device.Wireless";
pub const NM_AP_IFACE: &str = "org.freedesktop.NetworkManager.AccessPoint";
pub const NM_SETTINGS_IFACE: &str = "org.freedesktop.NetworkManager.Settings";
pub const NM_ACTIVE_CONN_IFACE: &str = "org.freedesktop.NetworkManager.Connection.Active";
pub const NM_IP4_CONFIG_IFACE: &str = "org.freedesktop.NetworkManager.IP4Config";
pub const DBUS_PROPERTIES_IFACE: &str = "org.freedesktop.DBus.Properties";

pub const DEFAULT_DBUS_TIMEOUT: Duration = Duration::from_millis(500);

/// Manages system D-Bus connection for NetworkManager operations
#[derive(Clone, Debug)]
pub struct DbusConnection {
    conn: Connection,
}

impl DbusConnection {
    /// Connect to the system D-Bus with a strict bounded timeout
    pub async fn system_timeout(timeout: Duration) -> Result<Self, String> {
        let connect_fut = Connection::system();
        match tokio::time::timeout(timeout, connect_fut).await {
            Ok(Ok(conn)) => {
                debug!("Established system D-Bus connection for NetworkManager");
                Ok(Self { conn })
            }
            Ok(Err(e)) => {
                let err = format!("Failed to connect to system D-Bus: {e}");
                warn!("{err}");
                Err(err)
            }
            Err(_) => {
                let err = format!("System D-Bus connection timed out after {timeout:?}");
                error!("{err}");
                Err(err)
            }
        }
    }

    /// Connect with the default 500ms timeout
    pub async fn system() -> Result<Self, String> {
        Self::system_timeout(DEFAULT_DBUS_TIMEOUT).await
    }

    /// Access the underlying raw zbus Connection
    pub fn raw(&self) -> &Connection {
        &self.conn
    }

    /// Verify whether NetworkManager daemon is active on the bus
    pub async fn is_networkmanager_active(&self) -> bool {
        let check_fut = self.conn.call_method(
            Some(NM_SERVICE),
            NM_PATH,
            Some(DBUS_PROPERTIES_IFACE),
            "Get",
            &(NM_IFACE, "State"),
        );

        match tokio::time::timeout(Duration::from_millis(300), check_fut).await {
            Ok(Ok(reply)) => reply.body().deserialize::<zbus::zvariant::OwnedValue>().is_ok(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dbus_connection_init() {
        let conn = DbusConnection::system().await;
        if let Ok(c) = conn {
            let active = c.is_networkmanager_active().await;
            println!("NetworkManager active on system D-Bus: {active}");
        }
    }
}
