use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::{button, column, container, row, slider, space, text};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize, Margin};
use iced_exwlshell::settings::LayerShellSettings;
use msui_core::{
    AudioInfo, AudioMonitor, BatteryInfo, BatteryMonitor, BluetoothInfo, BluetoothManager,
    BrightnessInfo, BrightnessMonitor, DndManager, MsuiConfig, NetworkInfo, NetworkMonitor,
    NightLightManager, PowerProfile, PowerProfileManager, SystemControl, SystemMetrics,
    SystemMonitor, ThemeManager,
};

pub fn run_control_center(_config: MsuiConfig) -> Result<(), iced_exwlshell::Error> {
    let width = 380;
    let height = 710;

    application(
        ControlCenter::new,
        ControlCenter::namespace,
        ControlCenter::update,
        ControlCenter::view,
    )
    .theme(ThemeManager::current_theme())
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(width, height),
        anchor: Anchor::Top | Anchor::Right,
        margin: Margin {
            top: 36, // 36px from top (below 32px top bar)
            right: 12,
            bottom: 0,
            left: 0,
        },
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    })
    .subscription(ControlCenter::subscription)
    .run()
}

pub struct ControlCenter {
    battery_info: Option<BatteryInfo>,
    system_metrics: SystemMetrics,
    audio_info: AudioInfo,
    brightness_info: Option<BrightnessInfo>,
    brightness_monitor: BrightnessMonitor,
    network_info: NetworkInfo,
    bluetooth_info: BluetoothInfo,
    is_dnd: bool,
    is_night_light: bool,
    night_light_temp: u32,
    active_profile: PowerProfile,
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleWifi,
    OpenWifiManager,
    ToggleBluetooth,
    OpenBluetoothManager,
    ToggleNightLight,
    OpenDisplayManager,
    OpenStorageManager,
    OpenMediaManager,
    ToggleDnd,
    ToggleMute,
    VolumeChanged(u8),
    VolumeStep(i8),
    BrightnessChanged(u8),
    BrightnessStep(i8),
    SetPowerProfile(PowerProfile),
    LockScreen,
    SuspendSystem,
    RebootSystem,
    PowerOffSystem,
    Refresh,
    Close,
    IcedEvent(Event),
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

impl ControlCenter {
    pub fn new() -> (Self, Command<Message>) {
        let battery_info = BatteryMonitor::new().read();
        let mut sys_mon = SystemMonitor::new();
        let system_metrics = sys_mon.sample();
        let audio_info = AudioMonitor::read();
        let brightness_monitor = BrightnessMonitor::new();
        let brightness_info = brightness_monitor.read();
        let network_info = NetworkMonitor::read();
        let bluetooth_info = BluetoothManager::read();
        let is_dnd = DndManager::is_dnd_enabled();
        let is_night_light = NightLightManager::is_active();
        let night_light_temp = NightLightManager::get_temperature();
        let active_profile = PowerProfileManager::get_profile();

        let state = Self {
            battery_info,
            system_metrics,
            audio_info,
            brightness_info,
            brightness_monitor,
            network_info,
            bluetooth_info,
            is_dnd,
            is_night_light,
            night_light_temp,
            active_profile,
        };

        (state, Command::none())
    }

    pub fn namespace() -> String {
        "msui-control-center".to_string()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        event::listen().map(Message::IcedEvent)
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ToggleWifi => {
                let turn_on = !self.network_info.is_connected && self.network_info.ssid.is_none();
                let _ = std::process::Command::new("nmcli")
                    .args(["radio", "wifi", if turn_on { "on" } else { "off" }])
                    .output();
                std::thread::sleep(std::time::Duration::from_millis(150));
                return Command::done(Message::Refresh);
            }
            Message::OpenWifiManager => {
                spawn_detached("wifi");
                std::process::exit(0);
            }
            Message::ToggleBluetooth => {
                BluetoothManager::toggle_power();
                std::thread::sleep(std::time::Duration::from_millis(150));
                return Command::done(Message::Refresh);
            }
            Message::OpenBluetoothManager => {
                spawn_detached("bluetooth");
                std::process::exit(0);
            }
            Message::ToggleNightLight => {
                let new_state = NightLightManager::toggle();
                self.is_night_light = new_state;
            }
            Message::OpenDisplayManager => {
                spawn_detached("display");
                std::process::exit(0);
            }
            Message::OpenStorageManager => {
                spawn_detached("storage");
                std::process::exit(0);
            }
            Message::OpenMediaManager => {
                spawn_detached("media");
                std::process::exit(0);
            }
            Message::ToggleDnd => {
                self.is_dnd = DndManager::toggle_dnd();
            }
            Message::ToggleMute => {
                AudioMonitor::toggle_mute();
                self.audio_info.is_muted = !self.audio_info.is_muted;
            }
            Message::VolumeChanged(vol) => {
                AudioMonitor::set_volume(vol);
                self.audio_info.volume_percent = vol;
                self.audio_info.is_muted = false;
            }
            Message::VolumeStep(delta) => {
                if delta > 0 {
                    AudioMonitor::increase_volume(delta as u8);
                } else {
                    AudioMonitor::decrease_volume((-delta) as u8);
                }
                self.audio_info = AudioMonitor::read();
            }
            Message::BrightnessChanged(val) => {
                BrightnessMonitor::set(val);
                self.brightness_info = self.brightness_monitor.read();
            }
            Message::BrightnessStep(delta) => {
                if delta > 0 {
                    BrightnessMonitor::increase(delta as u8);
                } else {
                    BrightnessMonitor::decrease((-delta) as u8);
                }
                self.brightness_info = self.brightness_monitor.read();
            }
            Message::SetPowerProfile(prof) => {
                PowerProfileManager::set_profile(prof);
                self.active_profile = prof;
            }
            Message::LockScreen => {
                SystemControl::lock();
                std::process::exit(0);
            }
            Message::SuspendSystem => {
                SystemControl::suspend();
                std::process::exit(0);
            }
            Message::RebootSystem => {
                SystemControl::reboot();
                std::process::exit(0);
            }
            Message::PowerOffSystem => {
                SystemControl::poweroff();
                std::process::exit(0);
            }
            Message::Refresh => {
                self.battery_info = BatteryMonitor::new().read();
                let mut sys_mon = SystemMonitor::new();
                self.system_metrics = sys_mon.sample();
                self.audio_info = AudioMonitor::read();
                self.brightness_info = self.brightness_monitor.read();
                self.network_info = NetworkMonitor::read();
                self.bluetooth_info = BluetoothManager::read();
                self.is_dnd = DndManager::is_dnd_enabled();
                self.is_night_light = NightLightManager::is_active();
                self.night_light_temp = NightLightManager::get_temperature();
                self.active_profile = PowerProfileManager::get_profile();
            }
            Message::Close => {
                std::process::exit(0);
            }
            Message::IcedEvent(Event::Keyboard(KeyboardEvent::KeyPressed { key, .. })) => {
                if let Key::Named(Named::Escape) = key {
                    return Command::done(Message::Close);
                }
            }
            _ => {}
        }
        Command::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        // --- 1. Header: Quick Settings Title, Battery & Close ---
        let title_icon = text("󰒓").size(18);
        let title_label = text("Quick Settings").size(15);
        let header_left = row![title_icon, title_label].spacing(8).align_y(Alignment::Center);

        let close_btn = button(text("✕").size(12))
            .style(button::danger)
            .padding([3, 8])
            .on_press(Message::Close);

        let header = row![header_left, space().width(Length::Fill), close_btn]
            .align_y(Alignment::Center);

        // --- 2. Quick Toggles Grid (2 columns x 3 rows) ---
        // Pill 1: Wi-Fi
        let wifi_active = self.network_info.is_connected;
        let wifi_name = self.network_info.ssid.clone().unwrap_or_else(|| "Off".into());
        let wifi_btn = button(
            row![
                text(if wifi_active { "󰖩" } else { "󰖪" }).size(16),
                column![
                    text("Wi-Fi").size(12),
                    text(if wifi_active { wifi_name } else { "Disconnected".into() }).size(10),
                ].spacing(1),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .width(Length::FillPortion(1))
        .style(if wifi_active { button::primary } else { button::secondary })
        .padding([8, 10])
        .on_press(Message::ToggleWifi);

        let wifi_detail = button(text("󰅂").size(12))
            .style(button::secondary)
            .padding([8, 6])
            .on_press(Message::OpenWifiManager);

        let wifi_group = row![wifi_btn, wifi_detail].spacing(2);

        // Pill 2: Bluetooth
        let bt_active = self.bluetooth_info.is_powered;
        let bt_sub = if !bt_active {
            "Off".to_string()
        } else if let Some(d) = self.bluetooth_info.connected_devices.first() {
            d.name.clone()
        } else {
            "No devices".to_string()
        };
        let bt_btn = button(
            row![
                text(if bt_active { "󰂯" } else { "󰂲" }).size(16),
                column![
                    text("Bluetooth").size(12),
                    text(bt_sub).size(10),
                ].spacing(1),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .width(Length::FillPortion(1))
        .style(if bt_active { button::primary } else { button::secondary })
        .padding([8, 10])
        .on_press(Message::ToggleBluetooth);

        let bt_detail = button(text("󰅂").size(12))
            .style(button::secondary)
            .padding([8, 6])
            .on_press(Message::OpenBluetoothManager);

        let bt_group = row![bt_btn, bt_detail].spacing(2);

        let row_toggles_1 = row![wifi_group, bt_group].spacing(8);

        // Pill 3: Night Light
        let nl_btn = button(
            row![
                text("󰌵").size(16),
                column![
                    text("Night Light").size(12),
                    text(if self.is_night_light { format!("{}K Active", self.night_light_temp) } else { "Off".into() }).size(10),
                ].spacing(1),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .width(Length::FillPortion(1))
        .style(if self.is_night_light { button::primary } else { button::secondary })
        .padding([8, 10])
        .on_press(Message::ToggleNightLight);

        // Pill 4: Do Not Disturb
        let dnd_btn = button(
            row![
                text(if self.is_dnd { "󰂛" } else { "󰂚" }).size(16),
                column![
                    text("Do Not Disturb").size(12),
                    text(if self.is_dnd { "Muted" } else { "Alerts On" }).size(10),
                ].spacing(1),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .width(Length::FillPortion(1))
        .style(if self.is_dnd { button::danger } else { button::secondary })
        .padding([8, 10])
        .on_press(Message::ToggleDnd);

        let row_toggles_2 = row![nl_btn, dnd_btn].spacing(8);

        // Pill 5: Audio Mute
        let is_muted = self.audio_info.is_muted;
        let mute_sub = if is_muted {
            "Muted".to_string()
        } else {
            format!("{}%", self.audio_info.volume_percent)
        };
        let mute_btn = button(
            row![
                text(if is_muted { "󰝟" } else { "󰕾" }).size(16),
                column![
                    text("Audio Mute").size(12),
                    text(mute_sub).size(10),
                ].spacing(1),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .width(Length::FillPortion(1))
        .style(if is_muted { button::danger } else { button::secondary })
        .padding([8, 10])
        .on_press(Message::ToggleMute);

        // Pill 6: Displays Manager Shortcut
        let disp_btn = button(
            row![
                text("󰍹").size(16),
                column![
                    text("Displays").size(12),
                    text("Modes & Scale").size(10),
                ].spacing(1),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .width(Length::FillPortion(1))
        .style(button::secondary)
        .padding([8, 10])
        .on_press(Message::OpenDisplayManager);

        let row_toggles_3 = row![mute_btn, disp_btn].spacing(8);

        // Pill 7: Storage & Removable Drives Shortcut
        let storage_btn = button(
            row![
                text("󰋊").size(16),
                column![
                    text("Storage").size(12),
                    text("Drives & USB").size(10),
                ].spacing(1),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .width(Length::FillPortion(1))
        .style(button::secondary)
        .padding([8, 10])
        .on_press(Message::OpenStorageManager);

        // Pill 8: Media Player Shortcut
        let media_btn = button(
            row![
                text("󰝚").size(16),
                column![
                    text("Media").size(12),
                    text("MPRIS Player").size(10),
                ].spacing(1),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .width(Length::FillPortion(1))
        .style(button::secondary)
        .padding([8, 10])
        .on_press(Message::OpenMediaManager);

        let row_toggles_4 = row![storage_btn, media_btn].spacing(8);

        let toggles_box = column![row_toggles_1, row_toggles_2, row_toggles_3, row_toggles_4].spacing(8);

        // --- 3. Sliders Section: Volume & Brightness ---
        let vol_val = self.audio_info.volume_percent.min(100);
        let vol_down = button(text("-").size(12)).style(button::secondary).padding([3, 7]).on_press(Message::VolumeStep(-5));
        let vol_up = button(text("+").size(12)).style(button::secondary).padding([3, 7]).on_press(Message::VolumeStep(5));
        let vol_slider = slider(0..=100, vol_val, Message::VolumeChanged).width(Length::Fill);
        let vol_row = row![
            text("󰕾").size(14),
            vol_down,
            vol_slider,
            vol_up,
            text(format!("{}%", vol_val)).size(11).width(Length::Fixed(36.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let bright_val = self.brightness_info.as_ref().map(|b| b.percent).unwrap_or(50);
        let bright_down = button(text("-").size(12)).style(button::secondary).padding([3, 7]).on_press(Message::BrightnessStep(-5));
        let bright_up = button(text("+").size(12)).style(button::secondary).padding([3, 7]).on_press(Message::BrightnessStep(5));
        let bright_slider = slider(1..=100, bright_val, Message::BrightnessChanged).width(Length::Fill);
        let bright_row = row![
            text("󰃠").size(14),
            bright_down,
            bright_slider,
            bright_up,
            text(format!("{}%", bright_val)).size(11).width(Length::Fixed(36.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let sliders_card = container(
            column![
                text("Volume & Brightness").size(12),
                vol_row,
                bright_row,
            ].spacing(10)
        )
        .padding(12)
        .style(container::bordered_box);

        // --- 4. Power Profiles Segmented Control ---
        let prof_saver = button(text("🍃 Saver").size(11))
            .width(Length::FillPortion(1))
            .style(if self.active_profile == PowerProfile::PowerSaver { button::primary } else { button::secondary })
            .padding([4, 6])
            .on_press(Message::SetPowerProfile(PowerProfile::PowerSaver));

        let prof_balanced = button(text("⚖ Balanced").size(11))
            .width(Length::FillPortion(1))
            .style(if self.active_profile == PowerProfile::Balanced { button::primary } else { button::secondary })
            .padding([4, 6])
            .on_press(Message::SetPowerProfile(PowerProfile::Balanced));

        let prof_perf = button(text("⚡ Performance").size(11))
            .width(Length::FillPortion(1))
            .style(if self.active_profile == PowerProfile::Performance { button::primary } else { button::secondary })
            .padding([4, 6])
            .on_press(Message::SetPowerProfile(PowerProfile::Performance));

        let profiles_card = container(
            column![
                text("Power Mode Profile").size(12),
                row![prof_saver, prof_balanced, prof_perf].spacing(6),
            ].spacing(6)
        )
        .padding(10)
        .style(container::bordered_box);

        // --- 5. Battery & Telemetry Status Card ---
        let (bat_icon, bat_label) = if let Some(bat) = &self.battery_info {
            let icon = if bat.is_charging { "󰂄" } else { "󰁹" };
            let time_str = if let Some(m) = bat.time_remaining_mins {
                format!(" (~{}h {}m remaining)", m / 60, m % 60)
            } else {
                "".to_string()
            };
            (icon, format!("{}% {}{}", bat.percentage, if bat.is_charging { "Charging" } else { "Discharging" }, time_str))
        } else {
            ("󰚥", "AC Power Connected".to_string())
        };

        let cpu_str = format!(" CPU: {:.0}%", self.system_metrics.cpu_usage_percent);
        let ram_str = format!("󰍛 RAM: {:.1}G", self.system_metrics.ram_used_mb as f32 / 1024.0);

        let telemetry_card = container(
            row![
                text(bat_icon).size(16),
                text(bat_label).size(11),
                space().width(Length::Fill),
                text(cpu_str).size(11),
                text("•").size(11),
                text(ram_str).size(11),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .padding([8, 12])
        .style(container::bordered_box);

        // --- 6. Footer Session Actions ---
        let btn_lock = button(text("󰌾 Lock").size(11))
            .width(Length::FillPortion(1))
            .style(button::secondary)
            .padding([5, 6])
            .on_press(Message::LockScreen);

        let btn_suspend = button(text("󰤄 Sleep").size(11))
            .width(Length::FillPortion(1))
            .style(button::secondary)
            .padding([5, 6])
            .on_press(Message::SuspendSystem);

        let btn_reboot = button(text("󰜉 Reboot").size(11))
            .width(Length::FillPortion(1))
            .style(button::secondary)
            .padding([5, 6])
            .on_press(Message::RebootSystem);

        let btn_power = button(text("󰐥 Off").size(11))
            .width(Length::FillPortion(1))
            .style(button::danger)
            .padding([5, 6])
            .on_press(Message::PowerOffSystem);

        let session_row = row![btn_lock, btn_suspend, btn_reboot, btn_power].spacing(6);

        let content = column![
            header,
            toggles_box,
            sliders_card,
            profiles_card,
            telemetry_card,
            session_row,
        ]
        .spacing(10)
        .padding(14);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn spawn_detached(cmd: &str) {
    if let Ok(exe) = std::env::current_exe() {
        if let Ok(mut child) = std::process::Command::new(exe).arg(cmd).spawn() {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
    }
}
