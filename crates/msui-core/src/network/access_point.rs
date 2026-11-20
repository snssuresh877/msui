use std::collections::HashMap;
use std::time::Duration;
use zbus::zvariant::OwnedValue;

use crate::network::connection::{
    DbusConnection, DBUS_PROPERTIES_IFACE, NM_AP_IFACE, NM_SERVICE,
};
use crate::network::types::WifiNetwork;

pub struct AccessPointParser;

impl AccessPointParser {
    /// Read and parse an AccessPoint D-Bus object path into a WifiNetwork model
    pub async fn parse_ap(
        dbus: &DbusConnection,
        ap_path: &str,
        active_ap_path: Option<&str>,
    ) -> Result<WifiNetwork, String> {
        let call_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            ap_path,
            Some(DBUS_PROPERTIES_IFACE),
            "GetAll",
            &(NM_AP_IFACE,),
        );

        let reply = tokio::time::timeout(Duration::from_millis(250), call_fut)
            .await
            .map_err(|_| format!("AP GetAll for {ap_path} timed out"))?
            .map_err(|e| format!("AP GetAll for {ap_path} failed: {e}"))?;

        let props: HashMap<String, OwnedValue> = reply
            .body()
            .deserialize()
            .map_err(|e| format!("Failed to parse AP properties: {e}"))?;

        // 1. SSID: Vec<u8> byte array
        let ssid = props
            .get("Ssid")
            .and_then(|v| {
                if let Ok(arr) = <&zbus::zvariant::Array>::try_from(v) {
                    let bytes: Vec<u8> = arr
                        .inner()
                        .iter()
                        .filter_map(|val| u8::try_from(val).ok())
                        .collect();
                    Some(String::from_utf8_lossy(&bytes).trim().to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        if ssid.is_empty() {
            return Err("Hidden or empty SSID".to_string());
        }

        // 2. BSSID (Hardware address)
        let bssid = props
            .get("HwAddress")
            .and_then(|v| <&str>::try_from(v).ok())
            .unwrap_or("")
            .to_string();

        // 3. Signal strength percentage (0..100)
        let signal = props
            .get("Strength")
            .and_then(|v| u8::try_from(v).ok())
            .unwrap_or(0);

        // 4. Frequency (MHz)
        let frequency_mhz = props
            .get("Frequency")
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0);

        // 5. Security flags
        let wpa_flags = props
            .get("WpaFlags")
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0);
        let rsn_flags = props
            .get("RsnFlags")
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0);

        let (security, is_secure) = Self::parse_security(wpa_flags, rsn_flags);

        // 6. Signal bars
        let bars = Self::calculate_bars(signal).to_string();

        // 7. Check if this AP is currently connected
        let is_connected = active_ap_path.map(|p| p == ap_path).unwrap_or(false);

        Ok(WifiNetwork {
            in_use: is_connected,
            ssid,
            bssid,
            signal,
            security,
            bars,
            is_connected,
            is_secure,
            frequency_mhz,
        })
    }

    /// Parse WPA/RSN bitflags into human-readable security strings
    pub fn parse_security(wpa_flags: u32, rsn_flags: u32) -> (String, bool) {
        // NM_802_11_AP_SEC constants:
        // WPA3 SAE: rsn_flags & 0x400 (or contains SAE key mgmt)
        if (rsn_flags & 0x400) != 0 || (rsn_flags & 0x200) != 0 {
            ("WPA3".to_string(), true)
        } else if rsn_flags != 0 {
            if (rsn_flags & 0x8) != 0 {
                ("WPA2 Enterprise".to_string(), true)
            } else {
                ("WPA2".to_string(), true)
            }
        } else if wpa_flags != 0 {
            if (wpa_flags & 0x8) != 0 {
                ("WPA Enterprise".to_string(), true)
            } else {
                ("WPA".to_string(), true)
            }
        } else {
            ("Open".to_string(), false)
        }
    }

    /// Calculate signal bars from percentage
    pub fn calculate_bars(signal: u8) -> &'static str {
        if signal > 75 {
            "▂▄▆█"
        } else if signal > 50 {
            "▂▄▆_"
        } else if signal > 25 {
            "▂▄__"
        } else {
            "▂___"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_bars() {
        assert_eq!(AccessPointParser::calculate_bars(80), "▂▄▆█");
        assert_eq!(AccessPointParser::calculate_bars(60), "▂▄▆_");
        assert_eq!(AccessPointParser::calculate_bars(30), "▂▄__");
        assert_eq!(AccessPointParser::calculate_bars(10), "▂___");
    }

    #[test]
    fn test_parse_security() {
        let (sec, is_sec) = AccessPointParser::parse_security(0, 0);
        assert_eq!(sec, "Open");
        assert!(!is_sec);

        let (sec, is_sec) = AccessPointParser::parse_security(0, 0x400);
        assert_eq!(sec, "WPA3");
        assert!(is_sec);

        let (sec, is_sec) = AccessPointParser::parse_security(0, 0x10);
        assert_eq!(sec, "WPA2");
        assert!(is_sec);

        let (sec, is_sec) = AccessPointParser::parse_security(0, 0x8);
        assert_eq!(sec, "WPA2 Enterprise");
        assert!(is_sec);
    }
}
