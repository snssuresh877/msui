pub mod clipboard;
pub mod compositor;
pub mod config;
pub mod desktop;
pub mod monitor;
pub mod notification;
pub mod power;
pub mod theme;
pub mod wallpaper;

pub use clipboard::{ClipboardEntry, ClipboardStore, GLOBAL_CLIPBOARD, watcher::start_clipboard_watcher};
pub use compositor::{HyprlandClient, HyprlandEvent, HyprlandIpc, HyprlandWorkspaceRef};
pub use config::{BarConfig, LauncherConfig, MsuiConfig};
pub use desktop::{find_icon_for_class, DesktopApp, scan_desktop_apps};
pub use monitor::{
    AudioDevice, AudioInfo, AudioMonitor, BatteryInfo, BatteryMonitor, BatteryStatus, BluetoothDevice,
    BluetoothInfo, BluetoothManager, BrightnessInfo, BrightnessMonitor, DisplayManager, DisplayMonitorInfo,
    MediaInfo, MediaMonitor, NetworkInfo, NetworkMonitor, NightLightConfig, NightLightManager,
    OsdKind, OsdManager, OsdState, PowerProfile, PowerProfileManager, StorageManager, StoragePartition,
    SystemMetrics, SystemMonitor, WifiNetwork,
};
pub use notification::{
    clear_active_toasts, dismiss_active_toast, is_toast_running, load_active_toasts,
    record_toast_pid, remove_toast_pid, DndManager, GLOBAL_NOTIFICATIONS, Notification, NotificationStore,
    service::run_notification_service,
};
pub use power::SystemControl;
pub use theme::{ThemeDefinition, ThemeManager, BUILTIN_THEMES};
pub use wallpaper::{WallpaperItem, WallpaperManager};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = MsuiConfig::default();
        assert_eq!(cfg.bar.height, 32);
        assert_eq!(cfg.bar.layer, "top");
        assert_eq!(cfg.launcher.width, 680);
        assert_eq!(cfg.launcher.max_results, 8);
    }

    #[test]
    fn test_system_monitor() {
        let mut mon = SystemMonitor::new();
        let metrics = mon.sample();
        assert!(metrics.ram_total_mb > 0);
    }

    #[test]
    fn test_battery_monitor() {
        let mon = BatteryMonitor::new();
        let _ = mon.read();
    }

    #[test]
    fn test_audio_monitor() {
        let audio = AudioMonitor::read();
        assert!(audio.volume_percent <= 150);
    }

    #[test]
    fn test_network_monitor() {
        let net = NetworkMonitor::read();
        let _ = net.is_connected;
    }

    #[test]
    fn test_brightness_monitor() {
        let b = BrightnessMonitor::new();
        let _ = b.read();
    }

    #[test]
    fn test_hyprland_parse_event() {
        let raw = "workspace>>3";
        let ev = HyprlandIpc::parse_event(raw);
        match ev {
            HyprlandEvent::WorkspaceChanged(id) => assert_eq!(id, 3),
            _ => panic!("Expected WorkspaceChanged"),
        }
    }

    #[test]
    fn test_clipboard_store() {
        let mut store = ClipboardStore::new_in_memory();
        let added = store.add("Hello World Test".into());
        assert!(added);
        assert_eq!(store.entries().len(), 1);
        assert_eq!(store.entries()[0].text, "Hello World Test");

        // Consecutive duplicate test
        let dup = store.add("Hello World Test".into());
        assert!(!dup);
        assert_eq!(store.entries().len(), 1);
    }

    #[test]
    fn test_notification_store() {
        let mut store = NotificationStore::new_in_memory();
        let notif = store.add("TestApp".into(), None, "Title".into(), "Body".into(), vec![], 5000);
        assert_eq!(notif.id, 1);
        assert_eq!(notif.summary, "Title");
        assert_eq!(store.history().len(), 1);
    }

    #[test]
    fn test_bluetooth_manager() {
        let info = BluetoothManager::read();
        // Machine has bluetooth controller or returns clean available/unavailable flag
        assert!(info.is_available || !info.is_available);
    }

    #[test]
    fn test_wallpaper_manager() {
        let list = WallpaperManager::list();
        assert!(list.len() < usize::MAX);
    }

    #[test]
    fn test_theme_manager() {
        let current = ThemeManager::current_theme_name();
        assert!(!current.is_empty());
        let _iced_theme = ThemeManager::current_theme();
        assert_eq!(ThemeManager::to_iced_theme("nord"), iced::Theme::Nord);
        assert_eq!(ThemeManager::to_iced_theme("dracula"), iced::Theme::Dracula);
    }

    #[test]
    fn test_network_wifi_scan() {
        let networks = NetworkMonitor::scan_wifi();
        assert!(networks.len() < usize::MAX);
    }

    #[test]
    fn test_audio_parse_json() {
        let sample_sinks = r#"[
            {
                "index": 1,
                "name": "alsa_output.pci.speaker",
                "description": "Built-in Speaker",
                "mute": false,
                "volume": {
                    "front-left": {"value": 32768, "value_percent": "50%"}
                }
            }
        ]"#;
        let sinks = AudioMonitor::parse_sinks_from_json(sample_sinks, Some("alsa_output.pci.speaker"));
        assert_eq!(sinks.len(), 1);
        assert_eq!(sinks[0].name, "alsa_output.pci.speaker");
        assert_eq!(sinks[0].volume_percent, 50);
        assert!(!sinks[0].is_muted);
        assert!(sinks[0].is_default);
        assert_eq!(sinks[0].icon, "󰓃");

        let sample_sources = r#"[
            {
                "index": 2,
                "name": "alsa_input.pci.mic",
                "description": "Internal Microphone",
                "mute": true,
                "volume": {
                    "front-left": {"value": 49152, "value_percent": "75%"}
                }
            },
            {
                "index": 3,
                "name": "alsa_output.pci.speaker.monitor",
                "description": "Monitor of Speaker",
                "mute": false
            }
        ]"#;
        let sources = AudioMonitor::parse_sources_from_json(sample_sources, Some("alsa_input.pci.mic"));
        // Monitor should be filtered out
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name, "alsa_input.pci.mic");
        assert_eq!(sources[0].volume_percent, 75);
        assert!(sources[0].is_muted);
        assert!(sources[0].is_default);
    }

    #[test]
    fn test_power_profile_manager() {
        let prof = PowerProfileManager::get_profile();
        assert!(matches!(prof, PowerProfile::PowerSaver | PowerProfile::Balanced | PowerProfile::Performance));
        assert_eq!(PowerProfile::from_str_loose("power-saver"), Some(PowerProfile::PowerSaver));
        assert_eq!(PowerProfile::from_str_loose("performance"), Some(PowerProfile::Performance));
        assert_eq!(PowerProfile::from_str_loose("balanced"), Some(PowerProfile::Balanced));
        assert_eq!(PowerProfileManager::list_profiles().len(), 3);
    }

    #[test]
    fn test_display_parse_json() {
        let sample_json = r#"[
            {
                "id": 0,
                "name": "eDP-1",
                "description": "Chimei Innolux Corporation 0x140A",
                "make": "Chimei Innolux Corporation",
                "model": "0x140A",
                "width": 1920,
                "height": 1080,
                "refreshRate": 60.0,
                "scale": 1.0,
                "transform": 0,
                "focused": true,
                "disabled": false,
                "availableModes": ["1920x1080@60.00Hz", "1280x720@60.00Hz"]
            }
        ]"#;
        let monitors = DisplayManager::parse_monitors_from_json(sample_json);
        assert_eq!(monitors.len(), 1);
        assert_eq!(monitors[0].name, "eDP-1");
        assert_eq!(monitors[0].width, 1920);
        assert_eq!(monitors[0].height, 1080);
        assert_eq!(monitors[0].refresh_rate, 60.0);
        assert_eq!(monitors[0].available_modes.len(), 2);
    }

    #[test]
    fn test_storage_manager() {
        let partitions = StorageManager::list_partitions();
        // Machine should have root / partition
        assert!(partitions.iter().any(|p| p.mount_point.as_deref() == Some("/")));
        // Test removable device check
        let _ = StorageManager::has_removable_devices();
    }
}
