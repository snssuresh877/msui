pub mod compositor;
pub mod config;
pub mod desktop;
pub mod monitor;
pub mod power;

pub use compositor::{HyprlandEvent, HyprlandIpc};
pub use config::{BarConfig, LauncherConfig, MsuiConfig};
pub use desktop::{DesktopApp, scan_desktop_apps};
pub use monitor::{
    AudioInfo, AudioMonitor, BatteryInfo, BatteryMonitor, BatteryStatus, BrightnessInfo,
    BrightnessMonitor, MediaInfo, MediaMonitor, NetworkInfo, NetworkMonitor, SystemMetrics,
    SystemMonitor,
};
pub use power::SystemControl;

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
        // Either connected or disconnected
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
}
