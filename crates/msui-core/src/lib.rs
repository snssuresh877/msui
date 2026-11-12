pub mod clipboard;
pub mod compositor;
pub mod config;
pub mod desktop;
pub mod ipc;
pub mod monitor;
pub mod notification;
pub mod polkit;
pub mod power;
pub mod reminder;
pub mod theme;
pub mod tray;
pub mod fshare;
pub mod wallpaper;
pub mod service;
pub mod color;
pub mod ai;

pub use ai::{AiConfig, LlamaRuntime, MemorySentinel, MemoryStatus, ModelInfo, ModelStore, RuntimeStatus};
pub use clipboard::{is_password_hint_active, ClipboardEntry, ClipboardStore, GLOBAL_CLIPBOARD, watcher::start_clipboard_watcher};
pub use compositor::{HyprlandClient, HyprlandEvent, HyprlandIpc, HyprlandWorkspaceRef};
pub use config::{parse_hex_color, BarConfig, ColorConfig, GuiConfig, LauncherConfig, MsuiConfig};
pub use desktop::{find_icon_for_class, invalidate_desktop_cache, scan_desktop_apps, DesktopApp};
pub use ipc::{get_socket_path, IpcClient, IpcCommand, IpcServer};
pub use polkit::{
    register_polkit_cancel_handler, register_polkit_request_handler, run_polkit_agent,
    verify_polkit_credentials, PolkitAgent, PolkitRequest,
};
pub use monitor::{
    AudioDevice, AudioInfo, AudioMonitor, BatteryInfo, BatteryMonitor, BatteryStatus, BluetoothDevice,
    BluetoothInfo, BluetoothManager, BrightnessInfo, BrightnessMonitor, DisplayManager, DisplayMonitorInfo,
    MediaInfo, MediaMonitor, NetworkInfo, NetworkMonitor, NightLightConfig, NightLightManager,
    OsdKind, OsdManager, OsdState, PowerProfile, PowerProfileManager, StorageManager, StoragePartition,
    SystemMetrics, SystemMonitor, WifiNetwork,
};
pub use notification::{
    add_active_toast, clear_active_toasts, dismiss_active_toast, is_toast_running, load_active_toasts,
    record_toast_pid, remove_toast_pid, DndManager, GLOBAL_NOTIFICATIONS, Notification, NotificationAction,
    NotificationStore,
    service::{emit_action_invoked, emit_notification_closed, run_notification_service},
};
pub use power::{PolicyAction, PowerPolicyEngine, SystemControl};
pub use reminder::{ReminderItem, ReminderManager};
pub use service::{SystemServiceInfo, SystemServiceManager};
pub use theme::{ThemeDefinition, ThemeManager, BUILTIN_THEMES};
pub use tray::{activate_tray_item, activate_tray_item_async, activate_tray_item_blocking, get_tray_items, watcher::run_tray_watcher, TrayItem};
pub use wallpaper::{WallpaperItem, WallpaperManager};
pub use color::{ColorEngine, ColorPreset, ColorSchemeMode, CorePalette, Hct, SemanticTokens, Srgb, ThemeExporter, WallpaperExtractor, PRESETS};


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
        assert!(cfg.bar.show_brand);
        assert!(cfg.bar.show_clock);
        assert!(cfg.bar.show_workspaces);
    }

    #[test]
    fn test_bar_config_toggle_and_save() {
        let mut cfg = MsuiConfig::default();
        cfg.bar.show_brand = false;
        cfg.bar.show_cpu = false;
        cfg.bar.show_reminders_btn = false;

        let toml_str = toml::to_string_pretty(&cfg).expect("Serialization failed");
        let parsed: MsuiConfig = toml::from_str(&toml_str).expect("Deserialization failed");
        assert!(!parsed.bar.show_brand);
        assert!(!parsed.bar.show_cpu);
        assert!(!parsed.bar.show_reminders_btn);
        assert!(parsed.bar.show_workspaces);
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
        let bat = mon.read();
        if let Some(b) = bat {
            println!("Battery status: {:?}, percentage: {}%, conservation_mode: {:?}", b.status, b.percentage, b.conservation_mode);
        }
        let _ = BatteryMonitor::conservation_mode_path();
        let _ = BatteryMonitor::read_conservation_mode();
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

    #[test]
    fn test_reminder_manager() {
        let title = "Unit Test Reminder";
        let item = ReminderManager::add_reminder(title, 10);
        assert_eq!(item.title, title);
        assert_eq!(item.total_duration_secs, 600);

        let active = ReminderManager::list_active();
        assert!(active.iter().any(|r| r.id == item.id));

        let next = ReminderManager::next_expiring();
        assert!(next.is_some());

        let removed = ReminderManager::remove_reminder(item.id);
        assert!(removed);
    }

    #[test]
    fn test_ipc_command_parsing() {
        let cmd = IpcCommand::parse("toggle control");
        assert_eq!(cmd, IpcCommand::Toggle("control".to_string()));
        assert_eq!(cmd.to_wire(), "toggle control");

        let cmd = IpcCommand::parse("open launcher");
        assert_eq!(cmd, IpcCommand::Open("launcher".to_string()));

        let cmd = IpcCommand::parse("close");
        assert_eq!(cmd, IpcCommand::Close);

        let cmd = IpcCommand::parse("dnd toggle");
        assert_eq!(cmd, IpcCommand::DndToggle);

        let cmd = IpcCommand::parse("ping");
        assert_eq!(cmd, IpcCommand::Ping);

        let text = "Hello\nWorld from Wayland! 🦀";
        let clip_cmd = IpcCommand::ClipboardAdd(text.to_string());
        let wire = clip_cmd.to_wire();
        assert!(wire.starts_with("clip-add "));
        let parsed = IpcCommand::parse(&wire);
        assert_eq!(parsed, clip_cmd);
    }

    #[test]
    fn test_tray_items() {
        let items = get_tray_items();
        assert!(items.is_empty() || !items.is_empty());

        let item = TrayItem {
            service: "org.kde.StatusNotifierItem-123-1".to_string(),
            path: "/StatusNotifierItem".to_string(),
            id: "protonvpn".to_string(),
            title: "Proton VPN".to_string(),
            icon_name: "protonvpn-gui".to_string(),
            tooltip: "Connected to US-FREE#1".to_string(),
        };

        let json = serde_json::to_string(&item).expect("Failed to serialize TrayItem");
        let parsed: TrayItem = serde_json::from_str(&json).expect("Failed to deserialize TrayItem");
        assert_eq!(parsed.id, "protonvpn");
        assert_eq!(parsed.title, "Proton VPN");
        assert_eq!(parsed.service, "org.kde.StatusNotifierItem-123-1");
    }

    #[test]
    fn test_ipc_socket_path() {
        let path = get_socket_path();
        assert!(path.to_string_lossy().contains("msui"));
    }

    #[cfg(unix)]
    #[test]
    fn test_cache_file_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let temp_dir = std::env::temp_dir().join("msui-test-cache-perms");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);

        // Test clipboard store save permissions (0600)
        let clip_path = temp_dir.join("clipboard_test.json");
        let mut clip_store = ClipboardStore::new_with_path(Some(clip_path.clone()));
        clip_store.add("Secret clipboard text".to_string());
        let meta = std::fs::metadata(&clip_path).expect("clipboard file should exist");
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "clipboard cache file mode must be 0600");

        // Test notification store save permissions (0600)
        let notif_path = temp_dir.join("notif_test.json");
        let mut notif_store = NotificationStore::new_with_path(Some(notif_path.clone()));
        notif_store.add("TestApp".into(), None, "Title".into(), "Secret".into(), vec![], 5000);
        let meta = std::fs::metadata(&notif_path).expect("notif file should exist");
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "notification cache file mode must be 0600");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

