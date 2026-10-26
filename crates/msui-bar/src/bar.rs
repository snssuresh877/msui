use chrono::Local;
use futures::channel::mpsc::Sender;
use futures::SinkExt;
use iced::widget::{button, container, row, space, text};
use iced::{Alignment, Element, Font, Length, Task as Command, Theme};
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, BlurOption, Layer, LayerSize, Margin};
use iced_exwlshell::settings::{ExWlSettings, LayerShellSettings, StartMode};
use iced_exwlshell::to_layer_message;
use iced_wayland_subscriber::OutputId;
use iced_wayland_subscriber::shell::{ShellEvent, ShellReceiver};

use msui_core::{
    AudioInfo, AudioMonitor, BatteryInfo, BatteryMonitor, BluetoothInfo, BluetoothManager,
    BrightnessInfo, BrightnessMonitor, MediaInfo, MediaMonitor, MsuiConfig, NetworkInfo,
    NetworkMonitor, ReminderItem, ReminderManager, StorageManager, SystemControl, SystemMetrics,
    SystemMonitor, ThemeManager, TrayItem,
};
use std::time::Duration;

pub fn run_bar(config: MsuiConfig) -> Result<(), iced_exwlshell::Error> {
    let (shell_broadcast, shell_events) = iced_wayland_subscriber::shell::channel();

    let height = config.bar.height;
    let layer = match config.bar.layer.to_lowercase().as_str() {
        "overlay" => Layer::Overlay,
        "bottom" => Layer::Bottom,
        "background" => Layer::Background,
        _ => Layer::Top,
    };

    let anchor = match config.bar.anchor.to_lowercase().as_str() {
        "bottom" => Anchor::Bottom | Anchor::Left | Anchor::Right,
        _ => Anchor::Top | Anchor::Left | Anchor::Right,
    };

    let cfg_clone = config.clone();

    let start_mode = if let Some(target) = msui_core::DisplayManager::get_primary_monitor_name() {
        tracing::info!("Targeting primary display output: {}", target);
        StartMode::TargetScreen(target)
    } else {
        StartMode::Active
    };

    let is_modern = config.bar.is_modern();
    let (margin, exclusive_zone) = if is_modern {
        (
            Margin {
                top: config.bar.margin_top as i32,
                right: config.bar.margin_horizontal as i32,
                bottom: 0,
                left: config.bar.margin_horizontal as i32,
            },
            (height + config.bar.margin_top + 4) as i32,
        )
    } else {
        (Margin::default(), height as i32)
    };

    let blur_option = if config.bar.is_frosted() {
        BlurOption::FullRegion
    } else {
        BlurOption::None
    };

    application(
        move || Bar::new(cfg_clone.clone(), shell_events.clone()),
        Bar::namespace,
        Bar::update,
        Bar::view,
    )
    .theme(ThemeManager::current_theme())
    .style(|_state, theme| iced::theme::Style {
        background_color: iced::Color::TRANSPARENT,
        text_color: theme.palette().text,
    })
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .subscription(Bar::subscription)
    .wl_settings(ExWlSettings {
        layer_settings: LayerShellSettings {
            size: LayerSize::fill_width(height),
            exclusive_zone,
            anchor,
            layer,
            margin,
            start_mode,
            blur_option,
            ..Default::default()
        },
        shell_broadcast,
        ..Default::default()
    })
    .run()
}

pub struct Bar {
    config: MsuiConfig,
    shell_events: ShellReceiver,
    output: Option<OutputId>,
    battery_monitor: BatteryMonitor,
    system_monitor: SystemMonitor,
    brightness_monitor: BrightnessMonitor,
    current_time: String,
    battery_info: Option<BatteryInfo>,
    system_metrics: SystemMetrics,
    audio_info: AudioInfo,
    network_info: NetworkInfo,
    brightness_info: Option<BrightnessInfo>,
    media_info: Option<MediaInfo>,
    hyprland_workspace: i32,
    active_window: Option<(String, String)>,
    notification_count: usize,
    bluetooth_info: BluetoothInfo,
    has_removable_drive: bool,
    next_reminder: Option<ReminderItem>,
    tray_items: Vec<TrayItem>,
    last_config_mtime: Option<std::time::SystemTime>,
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    ConfigReloaded,
    ClockTick,
    MetricsTick,
    SwitchWorkspace(i32),
    OutputChanged(Option<OutputId>),
    HyprlandFocusedWindow { class: String, title: String },
    HyprlandWorkspaceChanged(i32),
    OpenLauncher,
    OpenDrawer,
    OpenControlCenter,
    OpenTerminal,
    OpenPowerMenu,
    OpenClipboard,
    OpenNotifications,
    OpenWifi,
    OpenBluetooth,
    OpenWallpaper,
    OpenTheme,
    OpenAudio,
    OpenBattery,
    OpenDisplay,
    OpenMedia,
    OpenStorage,
    OpenReminders,
    TakeScreenshot,
    ToggleMute,
    VolumeUp,
    VolumeDown,
    BrightnessUp,
    BrightnessDown,
    MediaPlayPause,
    ActivateTray(String, String),
    Ignored,
}

fn bar_btn_transparent(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.palette();
    let bg = match status {
        button::Status::Hovered => Some(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.09).into()),
        button::Status::Pressed => Some(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.16).into()),
        _ => None,
    };
    let text_color = match status {
        button::Status::Hovered => iced::Color::WHITE,
        _ => p.text,
    };
    button::Style {
        background: bg,
        text_color,
        border: iced::border::rounded(6),
        ..Default::default()
    }
}

fn bar_btn_stepper(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.palette();
    let bg = match status {
        button::Status::Hovered => Some(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.12).into()),
        button::Status::Pressed => Some(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.20).into()),
        _ => None,
    };
    let text_color = match status {
        button::Status::Hovered => iced::Color::WHITE,
        _ => p.text,
    };
    button::Style {
        background: bg,
        text_color,
        border: iced::border::rounded(4),
        ..Default::default()
    }
}

fn bar_btn_brand(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.palette();
    let bg = match status {
        button::Status::Hovered => Some(iced::Color::from_rgba(p.primary.r, p.primary.g, p.primary.b, 0.24).into()),
        button::Status::Pressed => Some(iced::Color::from_rgba(p.primary.r, p.primary.g, p.primary.b, 0.32).into()),
        _ => Some(iced::Color::from_rgba(p.primary.r, p.primary.g, p.primary.b, 0.12).into()),
    };
    let mut border = iced::border::rounded(6);
    border.width = 1.0;
    border.color = iced::Color::from_rgba(p.primary.r, p.primary.g, p.primary.b, 0.35);
    button::Style {
        background: bg,
        text_color: p.primary,
        border,
        ..Default::default()
    }
}

fn bar_btn_active(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.palette();
    let bg = match status {
        button::Status::Hovered => iced::Color::from_rgba(p.primary.r, p.primary.g, p.primary.b, 0.28),
        button::Status::Pressed => iced::Color::from_rgba(p.primary.r, p.primary.g, p.primary.b, 0.38),
        _ => iced::Color::from_rgba(p.primary.r, p.primary.g, p.primary.b, 0.16),
    };
    let mut border = iced::border::rounded(6);
    border.width = 1.0;
    border.color = iced::Color::from_rgba(p.primary.r, p.primary.g, p.primary.b, 0.45);
    button::Style {
        background: Some(bg.into()),
        text_color: p.primary,
        border,
        ..Default::default()
    }
}

fn bar_btn_inactive_ws(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.palette();
    let bg = match status {
        button::Status::Hovered => Some(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08).into()),
        button::Status::Pressed => Some(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.14).into()),
        _ => None,
    };
    let text_color = match status {
        button::Status::Hovered => iced::Color::WHITE,
        _ => iced::Color::from_rgba(p.text.r, p.text.g, p.text.b, 0.55),
    };
    button::Style {
        background: bg,
        text_color,
        border: iced::border::rounded(6),
        ..Default::default()
    }
}

fn bar_btn_danger(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.palette();
    let bg = match status {
        button::Status::Hovered => Some(iced::Color::from_rgba(p.danger.r, p.danger.g, p.danger.b, 0.28).into()),
        button::Status::Pressed => Some(iced::Color::from_rgba(p.danger.r, p.danger.g, p.danger.b, 0.38).into()),
        _ => Some(iced::Color::from_rgba(p.danger.r, p.danger.g, p.danger.b, 0.16).into()),
    };
    let mut border = iced::border::rounded(6);
    border.width = 1.0;
    border.color = iced::Color::from_rgba(p.danger.r, p.danger.g, p.danger.b, 0.45);
    button::Style {
        background: bg,
        text_color: p.danger,
        border,
        ..Default::default()
    }
}

fn bar_btn_power(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.palette();
    let bg = match status {
        button::Status::Hovered => Some(iced::Color::from_rgba(p.danger.r, p.danger.g, p.danger.b, 0.22).into()),
        button::Status::Pressed => Some(iced::Color::from_rgba(p.danger.r, p.danger.g, p.danger.b, 0.34).into()),
        _ => None,
    };
    let text_color = match status {
        button::Status::Hovered => iced::Color::WHITE,
        _ => p.danger,
    };
    button::Style {
        background: bg,
        text_color,
        border: iced::border::rounded(6),
        ..Default::default()
    }
}

impl Bar {
    pub fn new(config: MsuiConfig, shell_events: ShellReceiver) -> Self {
        let battery_monitor = BatteryMonitor::new();
        let mut system_monitor = SystemMonitor::new();
        let brightness_monitor = BrightnessMonitor::new();

        let battery_info = battery_monitor.read();
        let system_metrics = system_monitor.sample();
        let current_time = Local::now().format(&config.bar.time_format).to_string();

        let audio_info = AudioMonitor::read();
        let network_info = NetworkMonitor::read();
        let brightness_info = brightness_monitor.read();
        let media_info = MediaMonitor::read();
        let bluetooth_info = BluetoothManager::read();

        let active_window = msui_core::HyprlandIpc::get_active_client()
            .map(|c| (c.class, c.title))
            .filter(|(c, t)| !c.is_empty() || !t.is_empty());

        Self {
            config,
            shell_events,
            output: None,
            battery_monitor,
            system_monitor,
            brightness_monitor,
            current_time,
            battery_info,
            system_metrics,
            audio_info,
            network_info,
            brightness_info,
            media_info,
            hyprland_workspace: msui_core::HyprlandIpc::get_active_workspace_id(),
            active_window,
            notification_count: 0,
            bluetooth_info,
            has_removable_drive: StorageManager::has_removable_devices(),
            next_reminder: ReminderManager::next_expiring(),
            tray_items: msui_core::get_tray_items(),
            last_config_mtime: MsuiConfig::default_config_path()
                .and_then(|p| std::fs::metadata(p).ok())
                .and_then(|m| m.modified().ok()),
        }
    }

    pub fn reload_config(&mut self, config: MsuiConfig) {
        self.config = config;
    }

    pub fn namespace() -> String {
        String::from("msui-bar")
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let clock = iced::time::every(Duration::from_secs(1)).map(|_| Message::ClockTick);
        let metrics = iced::time::every(Duration::from_secs(self.config.bar.sysinfo_refresh_seconds))
            .map(|_| Message::MetricsTick);

        let shell_listener = self.shell_events.listen().filter_map(|event| match event {
            ShellEvent::WindowOutputChanged { output, .. } => {
                Some(Message::OutputChanged(output.as_ref().map(OutputId::from)))
            }
            _ => None,
        });

        let hypr_sub = iced::Subscription::run(hyprland_stream);

        iced::Subscription::batch([clock, metrics, shell_listener, hypr_sub])
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ClockTick => {
                self.current_time = Local::now().format(&self.config.bar.time_format).to_string();
                ReminderManager::check_and_trigger_expired();
                self.next_reminder = ReminderManager::next_expiring();
                self.hyprland_workspace = msui_core::HyprlandIpc::get_active_workspace_id();
                self.tray_items = msui_core::get_tray_items();

                // Live dynamic reload if configuration file modified on disk
                if let Some(cfg_path) = MsuiConfig::default_config_path() {
                    if let Ok(meta) = std::fs::metadata(&cfg_path) {
                        if let Ok(mtime) = meta.modified() {
                            if self.last_config_mtime.map_or(true, |last| mtime > last) {
                                self.last_config_mtime = Some(mtime);
                                self.config = MsuiConfig::load_or_default();
                            }
                        }
                    }
                }
            }
            Message::MetricsTick => {
                self.system_metrics = self.system_monitor.sample();
                self.hyprland_workspace = msui_core::HyprlandIpc::get_active_workspace_id();
                self.tray_items = msui_core::get_tray_items();

                if self.config.bar.show_battery {
                    self.battery_info = self.battery_monitor.read();
                }
                if self.config.bar.show_audio {
                    self.audio_info = AudioMonitor::read();
                }
                if self.config.bar.show_network {
                    self.network_info = NetworkMonitor::read();
                }
                if self.config.bar.show_brightness {
                    self.brightness_info = self.brightness_monitor.read();
                }
                if self.config.bar.show_media {
                    self.media_info = MediaMonitor::read();
                }
                if self.config.bar.show_bluetooth {
                    self.bluetooth_info = BluetoothManager::read();
                }
                self.has_removable_drive = StorageManager::has_removable_devices();
                self.notification_count = {
                    let mut store = msui_core::GLOBAL_NOTIFICATIONS.lock().unwrap();
                    store.load();
                    store.unread_count()
                };
            }
            Message::SwitchWorkspace(id) => {
                self.hyprland_workspace = id;
                msui_core::HyprlandIpc::switch_workspace_sync(id);
            }
            Message::HyprlandFocusedWindow { class, title } => {
                if class.is_empty() && title.is_empty() {
                    self.active_window = None;
                } else {
                    self.active_window = Some((class, title));
                }
            }
            Message::HyprlandWorkspaceChanged(id) => {
                self.hyprland_workspace = id;
            }

            Message::OutputChanged(output) => {
                self.output = output;
            }
            Message::OpenLauncher => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("launcher"));
            }
            Message::OpenDrawer => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("drawer"));
            }
            Message::OpenControlCenter => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("control"));
            }
            Message::OpenClipboard => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("clipboard"));
            }
            Message::OpenNotifications => {
                {
                    let mut store = msui_core::GLOBAL_NOTIFICATIONS.lock().unwrap();
                    store.mark_all_read();
                }
                self.notification_count = 0;
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("notifications"));
            }
            Message::OpenWifi => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("wifi"));
            }
            Message::OpenBluetooth => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("bluetooth"));
            }
            Message::OpenWallpaper => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("wallpaper"));
            }
            Message::OpenTheme => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("theme"));
            }
            Message::OpenAudio => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("audio"));
            }
            Message::OpenBattery => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("battery"));
            }
            Message::OpenDisplay => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("display"));
            }
            Message::OpenMedia => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("media"));
            }
            Message::OpenStorage => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("storage"));
            }
            Message::OpenReminders => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("reminder"));
            }
            Message::OpenTerminal => {
                let term = &self.config.launcher.terminal;
                SystemControl::launch_terminal(term);
            }
            Message::OpenPowerMenu => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("power"));
            }
            Message::TakeScreenshot => {
                SystemControl::screenshot();
            }
            Message::ToggleMute => {
                AudioMonitor::toggle_mute();
                self.audio_info.is_muted = !self.audio_info.is_muted;
            }
            Message::VolumeUp => {
                AudioMonitor::increase_volume(5);
                self.audio_info.volume_percent = self.audio_info.volume_percent.saturating_add(5);
            }
            Message::VolumeDown => {
                AudioMonitor::decrease_volume(5);
                self.audio_info.volume_percent = self.audio_info.volume_percent.saturating_sub(5);
            }
            Message::BrightnessUp => {
                BrightnessMonitor::increase(5);
                if let Some(b) = &mut self.brightness_info {
                    b.percent = (b.percent + 5).min(100);
                }
            }
            Message::BrightnessDown => {
                BrightnessMonitor::decrease(5);
                if let Some(b) = &mut self.brightness_info {
                    b.percent = b.percent.saturating_sub(5);
                }
            }
            Message::MediaPlayPause => {
                MediaMonitor::play_pause();
                if let Some(m) = &mut self.media_info {
                    m.is_playing = !m.is_playing;
                }
            }
            Message::ActivateTray(service, path) => {
                msui_core::activate_tray_item(&service, &path);
            }
            Message::Ignored => {}
            _ => {}
        }
        Command::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        // --- Left Section: Launcher Button + Workspaces + Media ---
        let mut left_row = row![].spacing(8).align_y(Alignment::Center);

        if self.config.bar.show_brand {
            let brand_btn = button(text(" 󰣇 msUI ").size(14))
                .style(bar_btn_brand)
                .padding([4, 10])
                .on_press(Message::OpenDrawer);
            left_row = left_row.push(brand_btn);
        }

        if self.config.bar.show_workspaces {
            let mut ws_row = row![].spacing(3).align_y(Alignment::Center);

            let count = self.config.bar.workspace_count.clamp(1, 10);
            let active = self.hyprland_workspace.max(1);
            let start = if active > count as i32 {
                active - count as i32 + 1
            } else {
                1
            };
            let end = start + count as i32 - 1;

            for slot in start..=end {
                let is_active = slot == self.hyprland_workspace;
                let label = if is_active {
                    format!("󰮯 {slot}")
                } else {
                    format!("󰊠 {slot}")
                };

                let btn_style = if is_active {
                    bar_btn_active
                } else {
                    bar_btn_inactive_ws
                };

                ws_row = ws_row.push(
                    button(text(label).size(13))
                        .style(btn_style)
                        .padding([3, 7])
                        .on_press(Message::SwitchWorkspace(slot)),
                );
            }
            left_row = left_row.push(ws_row);
        }

        if self.config.bar.show_active_window {
            if let Some((class, title)) = &self.active_window {
                let icon = app_icon_for_class(class);
                let display_title = if title.trim().is_empty() {
                    class.as_str()
                } else {
                    title.as_str()
                };
                let truncated = if display_title.chars().count() > 38 {
                    let s: String = display_title.chars().take(37).collect();
                    format!("{s}…")
                } else {
                    display_title.to_string()
                };
                let win_btn = button(text(format!(" {icon}  {truncated} ")).size(13))
                    .style(bar_btn_transparent)
                    .padding([3, 8])
                    .on_press(Message::OpenDrawer);
                left_row = left_row.push(win_btn);
            }
        }

        // Media Widget (when playing or paused music exists)
        if self.config.bar.show_media {
            if let Some(media) = &self.media_info {
                let symbol = if media.is_playing { "󰏤" } else { "󰐊" };
                let short_title = if media.title.len() > 20 {
                    format!("{}…", &media.title[..18])
                } else {
                    media.title.clone()
                };
                let track_btn = button(text(format!(" 󰝚 {short_title} ")).size(13))
                    .style(bar_btn_transparent)
                    .padding([2, 6])
                    .on_press(Message::OpenMedia);
                let play_btn = button(text(symbol).size(12))
                    .style(bar_btn_active)
                    .padding([2, 6])
                    .on_press(Message::MediaPlayPause);
                let media_group = row![track_btn, play_btn].spacing(2).align_y(Alignment::Center);
                left_row = left_row.push(media_group);
            }
        }

        // --- Center Section: Clock & Date + Live Reminder Countdown ---
        let mut center_row = row![].spacing(8).align_y(Alignment::Center);

        if self.config.bar.show_clock {
            let clock_widget = button(text(format!("󰃭 {}", &self.current_time)).size(14))
                .style(bar_btn_transparent)
                .padding([2, 6])
                .on_press(Message::OpenControlCenter);
            center_row = center_row.push(clock_widget);
        }

        if self.config.bar.show_reminders_badge {
            if let Some(reminder) = &self.next_reminder {
                let rem_str = reminder.formatted_remaining();
                let is_expired = reminder.is_expired();
                let label = if is_expired {
                    format!(" 󰥔 Expired: {} ", reminder.title)
                } else {
                    format!(" 󰥔 {} ({}) ", rem_str, reminder.title)
                };
                let timer_pill = button(text(label).size(12))
                    .style(if is_expired { bar_btn_danger } else { bar_btn_active })
                    .padding([2, 8])
                    .on_press(Message::OpenReminders);
                center_row = center_row.push(timer_pill);
            }
        }

        // --- Right Section: Hardware & Status ---
        let mut right_row = row![].spacing(6).align_y(Alignment::Center);

        // Wi-Fi / Network Status with Signal Glyph
        if self.config.bar.show_network {
            let net_text = if self.network_info.is_connected {
                let sig_icon = if self.network_info.signal_percent > 75 {
                    "󰤨"
                } else if self.network_info.signal_percent > 50 {
                    "󰤥"
                } else if self.network_info.signal_percent > 25 {
                    "󰤢"
                } else {
                    "󰤟"
                };
                if let Some(ssid) = &self.network_info.ssid {
                    format!("{sig_icon} {ssid} ({}%)", self.network_info.signal_percent)
                } else if self.network_info.is_ethernet {
                    "󰈀 Eth".into()
                } else {
                    "󰤨 Online".into()
                }
            } else {
                "󰤭 Offline".into()
            };
            let net_btn = button(text(net_text).size(13))
                .style(if self.network_info.is_connected {
                    bar_btn_transparent
                } else {
                    bar_btn_danger
                })
                .padding([2, 6])
                .on_press(Message::OpenWifi);
            right_row = right_row.push(net_btn);
        }

        // Audio Volume with Mute and +/- controls
        if self.config.bar.show_audio {
            let vol_icon = if self.audio_info.is_muted {
                "󰝟"
            } else if self.audio_info.volume_percent > 66 {
                ""
            } else if self.audio_info.volume_percent > 33 {
                ""
            } else {
                ""
            };

            let mute_btn = button(text(vol_icon).size(13))
                .style(if self.audio_info.is_muted {
                    bar_btn_danger
                } else {
                    bar_btn_transparent
                })
                .padding([2, 5])
                .on_press(Message::ToggleMute);

            let vol_text = if self.audio_info.is_muted {
                "Muted".to_string()
            } else {
                format!("{}%", self.audio_info.volume_percent)
            };

            let vol_btn = button(text(vol_text).size(13))
                .style(bar_btn_transparent)
                .padding([2, 4])
                .on_press(Message::OpenAudio);

            let vol_down = button(text("-").size(12))
                .style(bar_btn_stepper)
                .padding([2, 6])
                .on_press(Message::VolumeDown);
            let vol_up = button(text("+").size(12))
                .style(bar_btn_stepper)
                .padding([2, 6])
                .on_press(Message::VolumeUp);

            let audio_group = row![mute_btn, vol_btn, vol_down, vol_up].spacing(2).align_y(Alignment::Center);
            right_row = right_row.push(audio_group);
        }

        // Backlight Brightness with +/- controls
        if self.config.bar.show_brightness {
            if let Some(b) = &self.brightness_info {
                let bright_icon = if b.percent > 70 {
                    "󰃠"
                } else if b.percent > 30 {
                    "󰃟"
                } else {
                    "󰃞"
                };
                let bright_str = format!("{bright_icon} {}%", b.percent);
                let bright_btn = button(text(bright_str).size(13))
                    .style(bar_btn_transparent)
                    .padding([2, 4])
                    .on_press(Message::OpenDisplay);
                let bright_down = button(text("-").size(12))
                    .style(bar_btn_stepper)
                    .padding([2, 6])
                    .on_press(Message::BrightnessDown);
                let bright_up = button(text("+").size(12))
                    .style(bar_btn_stepper)
                    .padding([2, 6])
                    .on_press(Message::BrightnessUp);

                let bright_group = row![bright_btn, bright_down, bright_up]
                    .spacing(2)
                    .align_y(Alignment::Center);
                right_row = right_row.push(bright_group);
            }
        }

        // CPU & RAM stats
        if self.config.bar.show_cpu {
            let cpu_str = format!(" {:.0}%", self.system_metrics.cpu_usage_percent);
            right_row = right_row.push(text(cpu_str).size(13));
        }

        if self.config.bar.show_memory {
            let ram_str = format!(
                "󰍛 {:.1}/{:.1}G",
                self.system_metrics.ram_used_mb as f32 / 1024.0,
                self.system_metrics.ram_total_mb as f32 / 1024.0
            );
            let ram_btn = button(text(ram_str).size(13))
                .style(bar_btn_transparent)
                .padding([2, 4])
                .on_press(Message::OpenStorage);
            right_row = right_row.push(ram_btn);
        }

        // Removable USB Storage indicator
        if self.config.bar.show_storage && self.has_removable_drive {
            let usb_btn = button(text(" 󱊞 USB ").size(12))
                .style(bar_btn_active)
                .padding([2, 6])
                .on_press(Message::OpenStorage);
            right_row = right_row.push(usb_btn);
        }

        // Battery
        if self.config.bar.show_battery {
            if let Some(bat) = &self.battery_info {
                let bat_icon = if bat.is_charging {
                    "󰂄"
                } else if bat.percentage > 90 {
                    "󰁹"
                } else if bat.percentage > 70 {
                    "󰂁"
                } else if bat.percentage > 50 {
                    "󰁿"
                } else if bat.percentage > 30 {
                    "󰁽"
                } else {
                    "󰁻"
                };
                let bat_str = format!("{bat_icon} {}%", bat.percentage);
                let bat_btn = button(text(bat_str).size(13))
                    .style(if bat.percentage <= 20 && !bat.is_charging {
                        bar_btn_danger
                    } else {
                        bar_btn_transparent
                    })
                    .padding([2, 5])
                    .on_press(Message::OpenBattery);
                right_row = right_row.push(bat_btn);
            }
        }

        // Bluetooth Widget
        if self.config.bar.show_bluetooth {
            let is_connected = self.bluetooth_info.is_powered && !self.bluetooth_info.connected_devices.is_empty();
            let (bt_icon, bt_label) = if !self.bluetooth_info.is_powered {
                ("󰂲", " Off".to_string())
            } else if let Some(first_conn) = self.bluetooth_info.connected_devices.first() {
                let name = if first_conn.name.len() > 10 {
                    format!(" {}", &first_conn.name[..10])
                } else {
                    format!(" {}", &first_conn.name)
                };
                ("󰂱", name)
            } else {
                ("󰂯", "".to_string())
            };
            let bt_btn = button(text(format!(" {}{} ", bt_icon, bt_label)).size(12))
                .style(if is_connected { bar_btn_active } else { bar_btn_transparent })
                .padding([2, 6])
                .on_press(Message::OpenBluetooth);
            right_row = right_row.push(bt_btn);
        }

        // Wallpaper Picker Shortcut
        if self.config.bar.show_wallpaper {
            let wall_btn = button(text(" 󰸉 ").size(13))
                .style(bar_btn_transparent)
                .padding([2, 6])
                .on_press(Message::OpenWallpaper);
            right_row = right_row.push(wall_btn);
        }

        // Theme Selector Shortcut
        if self.config.bar.show_theme {
            let theme_btn = button(text(" 󰔎 ").size(13))
                .style(bar_btn_transparent)
                .padding([2, 6])
                .on_press(Message::OpenTheme);
            right_row = right_row.push(theme_btn);
        }

        // Reminders & Timers Shortcut
        if self.config.bar.show_reminders_btn {
            let rem_btn = button(text(" 󰥔 ").size(13))
                .style(bar_btn_transparent)
                .padding([2, 6])
                .on_press(Message::OpenReminders);
            right_row = right_row.push(rem_btn);
        }

        // Quick Screenshot Shortcut
        if self.config.bar.show_screenshot {
            let shot_btn = button(text(" 󰄀 ").size(13))
                .style(bar_btn_transparent)
                .padding([2, 6])
                .on_press(Message::TakeScreenshot);
            right_row = right_row.push(shot_btn);
        }

        // Clipboard History Shortcut
        if self.config.bar.show_clipboard {
            let clip_btn = button(text(" 󰅍 Clip ").size(12))
                .style(bar_btn_transparent)
                .padding([2, 6])
                .on_press(Message::OpenClipboard);
            right_row = right_row.push(clip_btn);
        }

        // FreeDesktop / KDE System Tray (WeChat, Proton VPN, Steam, Discord, etc.)
        if !self.tray_items.is_empty() {
            let mut tray_group = row![].spacing(3).align_y(Alignment::Center);
            for item in &self.tray_items {
                let glyph = if item.id.to_lowercase().contains("wechat")
                    || item.title.to_lowercase().contains("wechat")
                {
                    "󰘑"
                } else if item.id.to_lowercase().contains("proton")
                    || item.title.to_lowercase().contains("proton")
                    || item.id.to_lowercase().contains("vpn")
                {
                    "󰖟"
                } else if item.id.to_lowercase().contains("steam") {
                    "󰓓"
                } else if item.id.to_lowercase().contains("discord") {
                    "󰙯"
                } else if item.id.to_lowercase().contains("telegram") {
                    "󰲸"
                } else {
                    "󰍜"
                };

                let tray_btn = button(text(format!(" {glyph} ")).size(13))
                    .style(bar_btn_transparent)
                    .padding([2, 5])
                    .on_press(Message::ActivateTray(item.service.clone(), item.path.clone()));

                tray_group = tray_group.push(tray_btn);
            }
            right_row = right_row.push(tray_group);
        }

        // Notification Bell Icon with Counter & DND status
        if self.config.bar.show_notifications {
            let is_dnd = msui_core::DndManager::is_dnd_enabled();
            let notif_label = if is_dnd {
                " 󰂛 ".into()
            } else if self.notification_count > 0 {
                format!(" 󰂚 {} ", self.notification_count)
            } else {
                " 󰂚 ".into()
            };
            let notif_btn = button(text(notif_label).size(12))
                .style(if is_dnd {
                    bar_btn_danger
                } else if self.notification_count > 0 {
                    bar_btn_active
                } else {
                    bar_btn_transparent
                })
                .padding([2, 6])
                .on_press(Message::OpenNotifications);
            right_row = right_row.push(notif_btn);
        }

        // Quick Settings Control Center Shortcut
        if self.config.bar.show_control_center {
            let control_btn = button(text(" 󰒓 ").size(12))
                .style(bar_btn_transparent)
                .padding([2, 6])
                .on_press(Message::OpenControlCenter);
            right_row = right_row.push(control_btn);
        }

        // Terminal Shortcut
        if self.config.bar.show_terminal {
            let term_btn = button(text(" 󰞷 Term ").size(12))
                .style(bar_btn_transparent)
                .padding([2, 6])
                .on_press(Message::OpenTerminal);
            right_row = right_row.push(term_btn);
        }

        // Power Menu Trigger Button
        if self.config.bar.show_power {
            let power_btn = button(text(" 󰐥 ").size(13))
                .style(bar_btn_power)
                .padding([2, 8])
                .on_press(Message::OpenPowerMenu);
            right_row = right_row.push(power_btn);
        }

        // Assemble full bar layout
        let bar_content = row![
            left_row,
            space().width(Length::Fill),
            center_row,
            space().width(Length::Fill),
            right_row,
        ]
        .width(Length::Fill)
        .align_y(Alignment::Center)
        .padding([2, 10]);

        let is_modern = self.config.bar.is_modern();
        let is_transp = self.config.bar.is_transparent();
        let is_solid = self.config.bar.is_solid();

        let bar_container = if is_modern {
            let radius = self.config.bar.corner_radius;
            let b_width = self.config.bar.border_width;
            container(bar_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |theme: &Theme| {
                    let p = theme.palette();
                    let mut border = iced::border::rounded(radius);
                    border.width = b_width;
                    if is_transp {
                        border.color = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.05);
                        container::Style {
                            background: None,
                            border,
                            ..Default::default()
                        }
                    } else if is_solid {
                        border.color = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.12);
                        container::Style {
                            background: Some(p.background.into()),
                            border,
                            ..Default::default()
                        }
                    } else {
                        // Frosted Glass
                        border.color = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.14);
                        container::Style {
                            background: Some(iced::Color::from_rgba(p.background.r, p.background.g, p.background.b, 0.42).into()),
                            border,
                            ..Default::default()
                        }
                    }
                })
        } else {
            container(bar_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |theme: &Theme| {
                    let p = theme.palette();
                    if is_transp {
                        container::Style {
                            background: None,
                            ..Default::default()
                        }
                    } else if is_solid {
                        container::Style {
                            background: Some(p.background.into()),
                            ..Default::default()
                        }
                    } else {
                        // Frosted Glass in classic docked mode
                        container::Style {
                            background: Some(iced::Color::from_rgba(p.background.r, p.background.g, p.background.b, 0.45).into()),
                            border: iced::Border {
                                color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                                width: 1.0,
                                radius: 0.0.into(),
                            },
                            ..Default::default()
                        }
                    }
                })
        };

        bar_container.into()
    }
}

fn spawn_detached(cmd: &mut std::process::Command) {
    if let Ok(mut child) = cmd.spawn() {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}

fn hyprland_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(100, |mut output: Sender<Message>| async move {
        loop {
            if let Some(sock_path) = msui_core::HyprlandIpc::socket2_path() {
                if let Ok(stream) = tokio::net::UnixStream::connect(sock_path).await {
                    let reader = tokio::io::BufReader::new(stream);
                    let mut lines = tokio::io::AsyncBufReadExt::lines(reader);
                    while let Ok(Some(line)) = lines.next_line().await {
                        let event = msui_core::HyprlandIpc::parse_event(&line);
                        match event {
                            msui_core::HyprlandEvent::FocusedWindow { class, title } => {
                                let _ = output.send(Message::HyprlandFocusedWindow { class, title }).await;
                            }
                            msui_core::HyprlandEvent::WorkspaceChanged(id) => {
                                let _ = output.send(Message::HyprlandWorkspaceChanged(id)).await;
                            }
                            _ => {}
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    })
}

fn app_icon_for_class(class: &str) -> &'static str {
    let lower = class.to_lowercase();
    if lower.contains("term") || lower.contains("kitty") || lower.contains("alacritty") || lower.contains("foot") || lower.contains("ghostty") || lower.contains("wezterm") {
        "󰄛"
    } else if lower.contains("code") || lower.contains("cursor") || lower.contains("zed") {
        "󰨞"
    } else if lower.contains("firefox") || lower.contains("librewolf") || lower.contains("floorp") || lower.contains("zen") {
        "󰈹"
    } else if lower.contains("chrome") || lower.contains("chromium") || lower.contains("brave") || lower.contains("edge") {
        "󰊯"
    } else if lower.contains("thunar") || lower.contains("nautilus") || lower.contains("dolphin") || lower.contains("nemo") || lower.contains("file") {
        "󰉋"
    } else if lower.contains("spotify") || lower.contains("music") {
        "󰓇"
    } else if lower.contains("discord") || lower.contains("vesktop") || lower.contains("webcord") {
        "󰙯"
    } else if lower.contains("telegram") {
        "󰍡"
    } else if lower.contains("steam") {
        "󰓓"
    } else if lower.contains("obsidian") || lower.contains("notion") || lower.contains("notes") {
        "󰠮"
    } else if lower.contains("gimp") || lower.contains("inkscape") || lower.contains("krita") {
        "󰽉"
    } else if lower.contains("vlc") || lower.contains("mpv") || lower.contains("video") {
        "󰕼"
    } else if lower.contains("nvim") || lower.contains("vim") || lower.contains("kate") || lower.contains("gedit") {
        "󰅩"
    } else if lower.contains("settings") || lower.contains("control") {
        "󰒓"
    } else if lower.contains("antigravity") || lower.contains("gemini") || lower.contains("ai") {
        "󱚣"
    } else {
        ""
    }
}
