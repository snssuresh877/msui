pub mod compositor;
pub mod config;
pub mod desktop;
pub mod monitor;

pub use compositor::{HyprlandEvent, HyprlandIpc};
pub use config::{BarConfig, LauncherConfig, MsuiConfig};
pub use desktop::{DesktopApp, scan_desktop_apps};
pub use monitor::{BatteryInfo, BatteryMonitor, BatteryStatus, SystemMetrics, SystemMonitor};

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
        // May be None on desktops or Some on laptops
        let _ = mon.read();
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
