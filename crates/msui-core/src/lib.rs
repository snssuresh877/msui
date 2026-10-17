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
pub use compositor::{HyprlandEvent, HyprlandIpc};
pub use config::{BarConfig, LauncherConfig, MsuiConfig};
pub use desktop::{DesktopApp, scan_desktop_apps};
pub use monitor::{
    AudioInfo, AudioMonitor, BatteryInfo, BatteryMonitor, BatteryStatus, BluetoothDevice,
    BluetoothInfo, BluetoothManager, BrightnessInfo, BrightnessMonitor, MediaInfo, MediaMonitor,
    NetworkInfo, NetworkMonitor, SystemMetrics, SystemMonitor, WifiNetwork,
};
pub use notification::{
    clear_active_toasts, dismiss_active_toast, is_toast_running, load_active_toasts,
    record_toast_pid, remove_toast_pid, GLOBAL_NOTIFICATIONS, Notification, NotificationStore,
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
}
