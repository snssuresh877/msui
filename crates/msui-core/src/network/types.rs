use serde::{Deserialize, Serialize};

/// Internet connectivity state according to NetworkManager
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InternetConnectivity {
    #[default]
    Unknown = 0,
    None = 1,
    Portal = 2,
    Limited = 3,
    Full = 4,
}

impl From<u32> for InternetConnectivity {
    fn from(val: u32) -> Self {
        match val {
            1 => Self::None,
            2 => Self::Portal,
            3 => Self::Limited,
            4 => Self::Full,
            _ => Self::Unknown,
        }
    }
}

/// NetworkManager Device Types (NMDeviceType)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DeviceType {
    #[default]
    Unknown = 0,
    Ethernet = 1,
    Wifi = 2,
    Bluetooth = 5,
    Vpn = 14,
    Loopback = 30,
    Other,
}

impl From<u32> for DeviceType {
    fn from(val: u32) -> Self {
        match val {
            1 => Self::Ethernet,
            2 => Self::Wifi,
            5 => Self::Bluetooth,
            14 => Self::Vpn,
            30 => Self::Loopback,
            _ => Self::Other,
        }
    }
}

/// NetworkManager Device States (NMDeviceState)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DeviceState {
    #[default]
    Unknown = 0,
    Unmanaged = 10,
    Unavailable = 20,
    Disconnected = 30,
    Preparing = 40,
    Configuring = 50,
    NeedAuth = 60,
    IpConfig = 70,
    IpCheck = 80,
    Secondaries = 90,
    Activated = 100,
    Deactivating = 110,
    Failed = 120,
}

impl From<u32> for DeviceState {
    fn from(val: u32) -> Self {
        match val {
            10 => Self::Unmanaged,
            20 => Self::Unavailable,
            30 => Self::Disconnected,
            40 => Self::Preparing,
            50 => Self::Configuring,
            60 => Self::NeedAuth,
            70 => Self::IpConfig,
            80 => Self::IpCheck,
            90 => Self::Secondaries,
            100 => Self::Activated,
            110 => Self::Deactivating,
            120 => Self::Failed,
            _ => Self::Unknown,
        }
    }
}

impl DeviceState {
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Activated)
    }

    pub fn is_active_or_activating(&self) -> bool {
        matches!(
            self,
            Self::Preparing
                | Self::Configuring
                | Self::NeedAuth
                | Self::IpConfig
                | Self::IpCheck
                | Self::Secondaries
                | Self::Activated
        )
    }
}

/// Wi-Fi Security mode
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WifiSecurity {
    #[default]
    Open,
    Wep,
    WpaPsk,
    Wpa2Psk,
    Wpa3Sae,
    Enterprise,
}

impl std::fmt::Display for WifiSecurity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "Open"),
            Self::Wep => write!(f, "WEP"),
            Self::WpaPsk => write!(f, "WPA"),
            Self::Wpa2Psk => write!(f, "WPA2"),
            Self::Wpa3Sae => write!(f, "WPA3"),
            Self::Enterprise => write!(f, "Enterprise"),
        }
    }
}

/// Summary Network Information for desktop shell bar and quick settings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub is_connected: bool,
    pub ssid: Option<String>,
    pub signal_percent: u8,
    pub is_ethernet: bool,
    pub wifi_enabled: bool,
    pub connectivity: InternetConnectivity,
}

impl Default for NetworkInfo {
    fn default() -> Self {
        Self {
            is_connected: false,
            ssid: None,
            signal_percent: 0,
            is_ethernet: false,
            wifi_enabled: true,
            connectivity: InternetConnectivity::Unknown,
        }
    }
}

/// Discovered Wi-Fi Access Point representation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub in_use: bool,
    pub ssid: String,
    pub bssid: String,
    pub signal: u8,
    pub security: String,
    pub bars: String,
    pub is_connected: bool,
    pub is_secure: bool,
    pub frequency_mhz: u32,
}

impl WifiNetwork {
    pub fn signal_icon(&self) -> &'static str {
        if self.signal > 75 {
            "󰤨"
        } else if self.signal > 50 {
            "󰤥"
        } else if self.signal > 25 {
            "󰤢"
        } else {
            "󰤟"
        }
    }

    pub fn is_5ghz(&self) -> bool {
        self.frequency_mhz >= 4900 && self.frequency_mhz <= 5900
    }
}

/// Detailed telemetry for the active network connection
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WifiDetails {
    pub ssid: Option<String>,
    pub ip_address: Option<String>,
    pub gateway: Option<String>,
    pub dns: Option<String>,
    pub frequency_mhz: Option<u32>,
    pub signal_dbm: Option<i32>,
    pub signal_percent: u8,
    pub bitrate: Option<String>,
    pub interface_name: Option<String>,
    pub security: Option<String>,
}

/// Saved NetworkManager connection profile
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedProfile {
    pub id: String,
    pub uuid: String,
    pub conn_type: String,
    pub autoconnect: bool,
    pub timestamp: u64,
}

/// Generic Network Device summary
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkDevice {
    pub interface: String,
    pub path: String,
    pub device_type: DeviceType,
    pub state: DeviceState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_types_conversion() {
        assert_eq!(DeviceType::from(1), DeviceType::Ethernet);
        assert_eq!(DeviceType::from(2), DeviceType::Wifi);
        assert_eq!(DeviceState::from(100), DeviceState::Activated);
        assert!(DeviceState::Activated.is_connected());
        assert!(!DeviceState::Disconnected.is_connected());
        assert_eq!(InternetConnectivity::from(4), InternetConnectivity::Full);

        let net = WifiNetwork {
            in_use: true,
            ssid: "TestNet".into(),
            bssid: "00:11:22:33:44:55".into(),
            signal: 80,
            security: "WPA2".into(),
            bars: "▂▄▆█".into(),
            is_connected: true,
            is_secure: true,
            frequency_mhz: 5180,
        };
        assert!(net.is_5ghz());
        assert_eq!(net.signal_icon(), "󰤨");
    }
}
