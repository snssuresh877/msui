use chrono::Local;
use iced::widget::{button, container, row, space, text};
use iced::{Alignment, Element, Font, Length, Task as Command};
use iced_exwlshell::get_wlconnection;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, Layer, LayerSize};
use iced_exwlshell::settings::{ExWlSettings, LayerShellSettings, StartMode};
use iced_exwlshell::to_layer_message;
use iced_wayland_subscriber::OutputId;
use iced_wayland_subscriber::shell::{ShellEvent, ShellReceiver};
use iced_wayland_subscriber::workspace::{
    State, Workspace, WorkspaceCapabilities, WorkspaceEvent, WorkspaceGroup,
    WorkspaceId, WorkspaceSnapshot,
};

use msui_core::{
    AudioInfo, AudioMonitor, BatteryInfo, BatteryMonitor, BluetoothInfo, BluetoothManager,
    BrightnessInfo, BrightnessMonitor, MediaInfo, MediaMonitor, MsuiConfig, NetworkInfo,
    NetworkMonitor, SystemControl, SystemMetrics, SystemMonitor, ThemeManager,
};
use std::sync::Arc;
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

    application(
        move || Bar::new(cfg_clone.clone(), shell_events.clone()),
        Bar::namespace,
        Bar::update,
        Bar::view,
    )
    .theme(ThemeManager::current_theme())
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .subscription(Bar::subscription)
    .wl_settings(ExWlSettings {
        layer_settings: LayerShellSettings {
            size: LayerSize::fill_width(height),
            exclusive_zone: height as i32,
            anchor,
            layer,
            start_mode: StartMode::Active,
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
    workspaces: Option<Arc<WorkspaceSnapshot>>,
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
    notification_count: usize,
    bluetooth_info: BluetoothInfo,
}

#[to_layer_message]
#[derive(Debug, Clone)]
pub enum Message {
    ClockTick,
    MetricsTick,
    WorkspacesUpdated(Arc<WorkspaceSnapshot>),
    ActivateWorkspace(WorkspaceId),
    OutputChanged(Option<OutputId>),
    OpenLauncher,
    OpenTerminal,
    OpenPowerMenu,
    OpenClipboard,
    OpenNotifications,
    OpenWifi,
    OpenBluetooth,
    OpenWallpaper,
    OpenTheme,
    TakeScreenshot,
    ToggleMute,
    VolumeUp,
    VolumeDown,
    BrightnessUp,
    BrightnessDown,
    MediaPlayPause,
    Ignored,
}

impl Bar {
    fn new(config: MsuiConfig, shell_events: ShellReceiver) -> Self {
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

        Self {
            config,
            shell_events,
            workspaces: None,
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
            hyprland_workspace: 1,
            notification_count: 0,
            bluetooth_info,
        }
    }

    fn namespace() -> String {
        String::from("msui-bar")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        let clock = iced::time::every(Duration::from_secs(1)).map(|_| Message::ClockTick);
        let metrics = iced::time::every(Duration::from_secs(self.config.bar.sysinfo_refresh_seconds))
            .map(|_| Message::MetricsTick);

        let wayland_workspaces = iced_wayland_subscriber::workspace::listen(get_wlconnection()).map(
            |event| match event {
                WorkspaceEvent::Updated(snapshot) => Message::WorkspacesUpdated(snapshot),
                _ => Message::Ignored,
            },
        );

        let shell_listener = self.shell_events.listen().filter_map(|event| match event {
            ShellEvent::WindowOutputChanged { output, .. } => {
                Some(Message::OutputChanged(output.as_ref().map(OutputId::from)))
            }
            _ => None,
        });

        iced::Subscription::batch([clock, metrics, wayland_workspaces, shell_listener])
    }

    fn group(&self) -> Option<&WorkspaceGroup> {
        self.workspaces.as_ref()?.group_for_output(self.output?)
    }

    fn occupant(&self, slot: u32) -> Option<&Workspace> {
        let snapshot = self.workspaces.as_ref()?;
        let group = self.group()?;
        let slot = slot.to_string();
        snapshot
            .workspaces_in(&group.id)
            .find(|workspace| workspace.name == slot)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ClockTick => {
                self.current_time = Local::now().format(&self.config.bar.time_format).to_string();
            }
            Message::MetricsTick => {
                self.system_metrics = self.system_monitor.sample();
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
                self.notification_count = {
                    let mut store = msui_core::GLOBAL_NOTIFICATIONS.lock().unwrap();
                    store.load();
                    store.history().len()
                };
            }
            Message::WorkspacesUpdated(snapshot) => {
                self.workspaces = Some(snapshot);
            }
            Message::OutputChanged(output) => {
                self.output = output;
            }
            Message::ActivateWorkspace(id) => {
                if let Some(snapshot) = &self.workspaces {
                    let _ = snapshot.activate(&id);
                }
            }
            Message::OpenLauncher => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("launcher"));
            }
            Message::OpenClipboard => {
                let bin = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("msui"));
                spawn_detached(std::process::Command::new(bin).arg("clipboard"));
            }
            Message::OpenNotifications => {
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
            Message::OpenTerminal => {
                let term = &self.config.launcher.terminal;
                spawn_detached(&mut std::process::Command::new(term));
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
            Message::Ignored => {}
            _ => {}
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        // --- Left Section: Launcher Button + Workspaces + Media ---
        let brand_btn = button(text(" 󰣇 msUI ").size(14))
            .style(button::primary)
            .padding([4, 10])
            .on_press(Message::OpenLauncher);

        let mut left_row = row![brand_btn].spacing(8).align_y(Alignment::Center);

        if self.config.bar.show_workspaces {
            let mut ws_row = row![].spacing(3).align_y(Alignment::Center);

            if self.workspaces.is_some() {
                for slot in 1..=8 {
                    let occupant = self.occupant(slot);
                    let is_active = occupant.map_or(false, |w| w.state.contains(State::Active));
                    let label = if is_active {
                        format!("󰮯 {slot}")
                    } else {
                        format!("󰊠 {slot}")
                    };

                    let btn_style = if is_active {
                        button::primary
                    } else {
                        button::secondary
                    };

                    let press = occupant
                        .filter(|w| w.capabilities.contains(WorkspaceCapabilities::Activate))
                        .map(|w| Message::ActivateWorkspace(w.id.clone()));

                    ws_row = ws_row.push(
                        button(text(label).size(13))
                            .style(btn_style)
                            .padding([3, 7])
                            .on_press_maybe(press),
                    );
                }
            } else {
                for slot in 1..=5 {
                    let is_active = slot == self.hyprland_workspace;
                    let label = if is_active {
                        format!("󰮯 {slot}")
                    } else {
                        format!("󰊠 {slot}")
                    };
                    let btn_style = if is_active {
                        button::primary
                    } else {
                        button::secondary
                    };
                    ws_row = ws_row.push(
                        button(text(label).size(13))
                            .style(btn_style)
                            .padding([3, 7]),
                    );
                }
            }
            left_row = left_row.push(ws_row);
        }

        // Media Widget (when playing or paused music exists)
        if self.config.bar.show_media {
            if let Some(media) = &self.media_info {
                let symbol = if media.is_playing { "󰐊" } else { "󰏤" };
                let short_title = if media.title.len() > 20 {
                    format!("{}...", &media.title[..18])
                } else {
                    media.title.clone()
                };
                let media_label = format!(" 󰝚 {short_title} {symbol} ");
                let media_btn = button(text(media_label).size(13))
                    .style(button::secondary)
                    .padding([3, 8])
                    .on_press(Message::MediaPlayPause);
                left_row = left_row.push(media_btn);
            }
        }

        // --- Center Section: Clock & Date ---
        let clock_widget = text(format!("󰃭 {}", &self.current_time)).size(14);

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
                    button::secondary
                } else {
                    button::danger
                })
                .padding([2, 6])
                .on_press(Message::OpenWifi);
            right_row = right_row.push(net_btn);
        }

        // Audio Volume with Mute and +/- controls
        if self.config.bar.show_audio {
            let vol_icon = if self.audio_info.is_muted {
                "󰝟 Muted"
            } else if self.audio_info.volume_percent > 66 {
                ""
            } else if self.audio_info.volume_percent > 33 {
                ""
            } else {
                ""
            };
            let vol_str = if self.audio_info.is_muted {
                vol_icon.to_string()
            } else {
                format!("{vol_icon} {}%", self.audio_info.volume_percent)
            };

            let vol_btn = button(text(vol_str).size(13))
                .style(if self.audio_info.is_muted {
                    button::danger
                } else {
                    button::text
                })
                .padding([2, 5])
                .on_press(Message::ToggleMute);

            let vol_down = button(text("-").size(12))
                .style(button::secondary)
                .padding([2, 6])
                .on_press(Message::VolumeDown);
            let vol_up = button(text("+").size(12))
                .style(button::secondary)
                .padding([2, 6])
                .on_press(Message::VolumeUp);

            let audio_group = row![vol_btn, vol_down, vol_up].spacing(2).align_y(Alignment::Center);
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
                let bright_down = button(text("-").size(12))
                    .style(button::secondary)
                    .padding([2, 6])
                    .on_press(Message::BrightnessDown);
                let bright_up = button(text("+").size(12))
                    .style(button::secondary)
                    .padding([2, 6])
                    .on_press(Message::BrightnessUp);

                let bright_group = row![text(bright_str).size(13), bright_down, bright_up]
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
            right_row = right_row.push(text(ram_str).size(13));
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
                right_row = right_row.push(text(bat_str).size(13));
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
                .style(if is_connected { button::primary } else { button::secondary })
                .padding([2, 6])
                .on_press(Message::OpenBluetooth);
            right_row = right_row.push(bt_btn);
        }

        // Wallpaper Picker Shortcut
        let wall_btn = button(text(" 󰸉 ").size(13))
            .style(button::secondary)
            .padding([2, 6])
            .on_press(Message::OpenWallpaper);
        right_row = right_row.push(wall_btn);

        // Theme Selector Shortcut
        let theme_btn = button(text(" 󰔎 ").size(13))
            .style(button::secondary)
            .padding([2, 6])
            .on_press(Message::OpenTheme);
        right_row = right_row.push(theme_btn);

        // Quick Screenshot Shortcut
        let shot_btn = button(text(" 󰄀 ").size(13))
            .style(button::secondary)
            .padding([2, 6])
            .on_press(Message::TakeScreenshot);
        right_row = right_row.push(shot_btn);

        // Clipboard History Shortcut
        let clip_btn = button(text(" 󰅍 Clip ").size(12))
            .style(button::secondary)
            .padding([2, 6])
            .on_press(Message::OpenClipboard);
        right_row = right_row.push(clip_btn);

        // Notification Bell Icon with Counter
        let notif_label = if self.notification_count > 0 {
            format!(" 󰂚 {} ", self.notification_count)
        } else {
            " 󰂚 ".into()
        };
        let notif_btn = button(text(notif_label).size(12))
            .style(if self.notification_count > 0 {
                button::primary
            } else {
                button::secondary
            })
            .padding([2, 6])
            .on_press(Message::OpenNotifications);
        right_row = right_row.push(notif_btn);

        // Terminal Shortcut
        let term_btn = button(text(" 󰞷 Term ").size(12))
            .style(button::secondary)
            .padding([2, 6])
            .on_press(Message::OpenTerminal);
        right_row = right_row.push(term_btn);

        // Power Menu Trigger Button
        let power_btn = button(text(" 󰐥 ").size(13))
            .style(button::danger)
            .padding([2, 8])
            .on_press(Message::OpenPowerMenu);
        right_row = right_row.push(power_btn);

        // Assemble full bar layout
        let bar_content = row![
            left_row,
            space().width(Length::Fill),
            clock_widget,
            space().width(Length::Fill),
            right_row,
        ]
        .width(Length::Fill)
        .align_y(Alignment::Center)
        .padding([2, 10]);

        container(bar_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn spawn_detached(cmd: &mut std::process::Command) {
    if let Ok(mut child) = cmd.spawn() {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}
