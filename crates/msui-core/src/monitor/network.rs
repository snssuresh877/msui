pub use crate::network::{InternetConnectivity, NetworkInfo, WifiNetwork};
pub use crate::network::NetworkService;
pub use crate::network::WifiManager;

pub struct NetworkMonitor;

impl NetworkMonitor {
    /// Ultra-fast non-blocking read (< 0.001ms) of active network state
    #[inline]
    pub fn read() -> NetworkInfo {
        crate::network::NetworkService::read_state()
    }

    /// Check whether Wi-Fi radio is enabled (< 0.001ms)
    #[inline]
    pub fn is_wifi_enabled() -> bool {
        crate::network::NetworkService::read_state().wifi_enabled
    }

    /// Immediate non-blocking read of cached access points (< 0.001ms)
    #[inline]
    pub fn cached_access_points() -> Vec<WifiNetwork> {
        crate::network::WifiManager::get_cached_networks()
    }

    /// Toggle Wi-Fi radio asynchronously via native D-Bus
    #[inline]
    pub fn set_wifi_enabled(enabled: bool) {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let _ = crate::network::WifiManager::set_radio_enabled(enabled).await;
            });
        } else {
            let _ = std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(crate::network::WifiManager::set_radio_enabled(enabled))
            });
        }
    }

    #[inline]
    pub fn next_generation() -> u64 {
        crate::network::CacheManager::next_generation()
    }

    #[inline]
    pub fn current_generation() -> u64 {
        crate::network::CacheManager::generation()
    }

    /// Invalidate cache to force a fresh read
    #[inline]
    pub fn invalidate_cache() {
        crate::network::CacheManager::invalidate();
    }

    /// Check if any network interface is connected
    #[inline]
    pub fn has_active_interface() -> bool {
        crate::network::NetworkService::read_state().is_connected
    }

    /// Check if active connection is over wired Ethernet
    #[inline]
    pub fn has_active_ethernet() -> bool {
        crate::network::NetworkService::read_state().is_ethernet
    }

    /// Perform a Wi-Fi scan and return results
    #[inline]
    pub fn scan_wifi() -> Vec<WifiNetwork> {
        Self::scan_wifi_guarded(1, true)
    }

    /// Wi-Fi scan using native D-Bus (without spawning nmcli subprocesses)
    #[inline]
    pub fn scan_wifi_guarded(_gen_id: u64, force: bool) -> Vec<WifiNetwork> {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            tokio::task::block_in_place(|| {
                handle.block_on(crate::network::WifiManager::scan_networks(force))
            })
        } else {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(crate::network::WifiManager::scan_networks(force))
        }
    }

    /// Request hardware Wi-Fi rescan via native D-Bus
    #[inline]
    pub fn rescan_wifi() {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let _ = crate::network::WifiManager::scan_networks(true).await;
            });
        }
    }

    /// Connect to Wi-Fi network via native D-Bus
    #[inline]
    pub fn connect_wifi(ssid: &str, password: Option<&str>) -> Result<(), String> {
        let ssid = ssid.to_string();
        let pw = password.map(|s| s.to_string());
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            tokio::task::block_in_place(|| {
                handle.block_on(crate::network::WifiManager::connect_network(&ssid, pw.as_deref()))
            })
        } else {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(crate::network::WifiManager::connect_network(&ssid, pw.as_deref()))
        }
    }

    /// Disconnect Wi-Fi network via native D-Bus
    #[inline]
    pub fn disconnect_wifi() -> bool {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            tokio::task::block_in_place(|| {
                handle.block_on(crate::network::WifiManager::disconnect_network()).is_ok()
            })
        } else {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(crate::network::WifiManager::disconnect_network()).is_ok()
        }
    }
}
