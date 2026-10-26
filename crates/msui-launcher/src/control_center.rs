use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::{button, column, container, row, scrollable, slider, space, text};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize, Margin};
use iced_exwlshell::settings::LayerShellSettings;
use msui_core::{
    AudioInfo, AudioMonitor, BarConfig, BatteryInfo, BatteryMonitor, BluetoothInfo, BluetoothManager,
    BrightnessInfo, BrightnessMonitor, DndManager, MsuiConfig, NetworkInfo, NetworkMonitor,
    NightLightManager, PowerProfile, PowerProfileManager, SystemControl, SystemMetrics,
    SystemMonitor, ThemeManager,
};

static INITIAL_TAB: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn run_control_center(config: MsuiConfig) -> Result<(), iced_exwlshell::Error> {
    INITIAL_TAB.store(false, std::sync::atomic::Ordering::Relaxed);
    run_control_center_internal(config)
}

pub fn run_control_center_topbar(config: MsuiConfig) -> Result<(), iced_exwlshell::Error> {
    INITIAL_TAB.store(true, std::sync::atomic::Ordering::Relaxed);
    run_control_center_internal(config)
}

fn run_control_center_internal(_config: MsuiConfig) -> Result<(), iced_exwlshell::Error> {
    let width = 400;
    let height = 765;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlCenterTab {
    QuickSettings,
    TopBarSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarModule {
    Brand,
    Workspaces,
    Media,
    Clock,
    RemindersBadge,
    Network,
    Audio,
    Brightness,
    Cpu,
    Memory,
    Storage,
    Battery,
    Bluetooth,
    Wallpaper,
    Theme,
    RemindersBtn,
    Screenshot,
    Clipboard,
    Notifications,
    ControlCenter,
    Terminal,
    Power,
}

impl BarModule {
    pub fn details(&self) -> (&'static str, &'static str, &'static str) {
        match self {
            BarModule::Brand => ("msUI Launcher & Apps", "󰣇", "Leftmost logo button opening app drawer"),
            BarModule::Workspaces => ("Workspace Pager", "󰮯", "Active Hyprland workspace indicators (configurable count)"),
            BarModule::Clock => ("Clock & Calendar", "󰃭", "Current time and date display"),
            BarModule::RemindersBadge => ("Timer Countdown Badge", "󰥔", "Live ticking badge for active reminders"),
            BarModule::Network => ("Wi-Fi & Network Status", "󰤨", "Wi-Fi signal strength and SSID pill"),
            BarModule::Audio => ("Audio Volume Controls", "󰕾", "Volume percentage, mute, and steppers"),
            BarModule::Brightness => ("Backlight Brightness", "󰃠", "Display brightness percentage and steppers"),
            BarModule::Cpu => ("CPU Load Telemetry", "", "Real-time processor load percentage"),
            BarModule::Memory => ("RAM Memory Telemetry", "󰍛", "Active and total RAM consumption"),
            BarModule::Storage => ("Removable USB Indicator", "󱊞", "Shows up when USB flash drive is attached"),
            BarModule::Battery => ("Battery & Power Status", "󰁹", "Battery percentage and charging state"),
            BarModule::Bluetooth => ("Bluetooth Status", "󰂯", "Bluetooth power state and active devices"),
            BarModule::Media => ("Media Player Widget", "󰝚", "Now Playing music title & play/pause"),
            BarModule::Wallpaper => ("Wallpaper Picker", "󰸉", "Quick wallpaper cycle and selector button"),
            BarModule::Theme => ("Theme Switcher", "󰔎", "Catppuccin, Nord, Dracula theme picker"),
            BarModule::RemindersBtn => ("Reminders & Timers", "󰥔", "Quick countdown timer manager button"),
            BarModule::Screenshot => ("Screenshot Tool", "󰄀", "One-click screen capture button"),
            BarModule::Clipboard => ("Clipboard History", "󰅍", "Clipboard manager with search"),
            BarModule::Notifications => ("Notification Bell", "󰂚", "Notification center and unread counter"),
            BarModule::ControlCenter => ("Quick Settings Trigger", "󰒓", "Status bar shortcut to open Control Center"),
            BarModule::Terminal => ("Terminal Launcher", "󰞷", "Quick terminal emulator shortcut"),
            BarModule::Power => ("Session / Power Menu", "󰐥", "System lock, sleep, reboot, and poweroff"),
        }
    }

    pub fn is_enabled(&self, cfg: &BarConfig) -> bool {
        match self {
            BarModule::Brand => cfg.show_brand,
            BarModule::Workspaces => cfg.show_workspaces,
            BarModule::Media => cfg.show_media,
            BarModule::Clock => cfg.show_clock,
            BarModule::RemindersBadge => cfg.show_reminders_badge,
            BarModule::Network => cfg.show_network,
            BarModule::Audio => cfg.show_audio,
            BarModule::Brightness => cfg.show_brightness,
            BarModule::Cpu => cfg.show_cpu,
            BarModule::Memory => cfg.show_memory,
            BarModule::Storage => cfg.show_storage,
            BarModule::Battery => cfg.show_battery,
            BarModule::Bluetooth => cfg.show_bluetooth,
            BarModule::Wallpaper => cfg.show_wallpaper,
            BarModule::Theme => cfg.show_theme,
            BarModule::RemindersBtn => cfg.show_reminders_btn,
            BarModule::Screenshot => cfg.show_screenshot,
            BarModule::Clipboard => cfg.show_clipboard,
            BarModule::Notifications => cfg.show_notifications,
            BarModule::ControlCenter => cfg.show_control_center,
            BarModule::Terminal => cfg.show_terminal,
            BarModule::Power => cfg.show_power,
        }
    }

    pub fn toggle(&self, cfg: &mut BarConfig) {
        match self {
            BarModule::Brand => cfg.show_brand = !cfg.show_brand,
            BarModule::Workspaces => cfg.show_workspaces = !cfg.show_workspaces,
            BarModule::Media => cfg.show_media = !cfg.show_media,
            BarModule::Clock => cfg.show_clock = !cfg.show_clock,
            BarModule::RemindersBadge => cfg.show_reminders_badge = !cfg.show_reminders_badge,
            BarModule::Network => cfg.show_network = !cfg.show_network,
            BarModule::Audio => cfg.show_audio = !cfg.show_audio,
            BarModule::Brightness => cfg.show_brightness = !cfg.show_brightness,
            BarModule::Cpu => cfg.show_cpu = !cfg.show_cpu,
            BarModule::Memory => cfg.show_memory = !cfg.show_memory,
            BarModule::Storage => cfg.show_storage = !cfg.show_storage,
            BarModule::Battery => cfg.show_battery = !cfg.show_battery,
            BarModule::Bluetooth => cfg.show_bluetooth = !cfg.show_bluetooth,
            BarModule::Wallpaper => cfg.show_wallpaper = !cfg.show_wallpaper,
            BarModule::Theme => cfg.show_theme = !cfg.show_theme,
            BarModule::RemindersBtn => cfg.show_reminders_btn = !cfg.show_reminders_btn,
            BarModule::Screenshot => cfg.show_screenshot = !cfg.show_screenshot,
            BarModule::Clipboard => cfg.show_clipboard = !cfg.show_clipboard,
            BarModule::Notifications => cfg.show_notifications = !cfg.show_notifications,
            BarModule::ControlCenter => cfg.show_control_center = !cfg.show_control_center,
            BarModule::Terminal => cfg.show_terminal = !cfg.show_terminal,
            BarModule::Power => cfg.show_power = !cfg.show_power,
        }
    }
}

pub struct ControlCenter {
    active_tab: ControlCenterTab,
    config: MsuiConfig,
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
    is_standalone: bool,
    should_close: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    SwitchTab(ControlCenterTab),
    SetBarStyle(String),
    SetBarTransparency(String),
    SetWorkspaceCount(usize),
    ToggleBarModule(BarModule),
    EnableAllBarModules,
    MinimalBarPreset,
    ToggleWifi,
    OpenWifiManager,
    ToggleBluetooth,
    OpenBluetoothManager,
    ToggleNightLight,
    OpenDisplayManager,
    OpenStorageManager,
    OpenMediaManager,
    OpenReminderManager,
    OpenAppDrawer,
    OpenClipboardManager,
    OpenFShare,
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
        Self::new_internal(true)
    }

    pub fn new_embedded(_config: MsuiConfig) -> (Self, Command<Message>) {
        Self::new_internal(false)
    }

    fn new_internal(is_standalone: bool) -> (Self, Command<Message>) {
        let config = MsuiConfig::load_or_default();
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

        let active_tab = if INITIAL_TAB.load(std::sync::atomic::Ordering::Relaxed) {
            ControlCenterTab::TopBarSettings
        } else {
            ControlCenterTab::QuickSettings
        };

        let state = Self {
            active_tab,
            config,
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
            is_standalone,
            should_close: false,
        };

        (state, Command::none())
    }

    pub fn should_close(&self) -> bool {
        self.should_close
    }

    pub fn reset_should_close(&mut self) {
        self.should_close = false;
    }

    pub fn reset(&mut self) -> Command<Message> {
        self.should_close = false;
        Command::done(Message::Refresh)
    }

    pub fn set_tab(&mut self, tab: ControlCenterTab) {
        self.active_tab = tab;
    }

    pub fn namespace() -> String {
        "msui-control-center".to_string()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        event::listen().map(Message::IcedEvent)
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SwitchTab(tab) => {
                self.active_tab = tab;
            }
            Message::SetBarStyle(style) => {
                self.config.bar.style = style;
                let _ = self.config.save_to_default_path();
            }
            Message::SetBarTransparency(transparency) => {
                self.config.bar.transparency = transparency;
                let _ = self.config.save_to_default_path();
            }
            Message::SetWorkspaceCount(count) => {
                self.config.bar.workspace_count = count.clamp(1, 10);
                let _ = self.config.save_to_default_path();
            }
            Message::ToggleBarModule(module) => {
                module.toggle(&mut self.config.bar);
                let _ = self.config.save_to_default_path();
            }
            Message::EnableAllBarModules => {
                let b = &mut self.config.bar;
                b.show_brand = true;
                b.show_workspaces = true;
                b.show_media = true;
                b.show_clock = true;
                b.show_reminders_badge = true;
                b.show_network = true;
                b.show_audio = true;
                b.show_brightness = true;
                b.show_cpu = true;
                b.show_memory = true;
                b.show_storage = true;
                b.show_battery = true;
                b.show_bluetooth = true;
                b.show_wallpaper = true;
                b.show_theme = true;
                b.show_reminders_btn = true;
                b.show_screenshot = true;
                b.show_clipboard = true;
                b.show_notifications = true;
                b.show_control_center = true;
                b.show_terminal = true;
                b.show_power = true;
                let _ = self.config.save_to_default_path();
            }
            Message::MinimalBarPreset => {
                let b = &mut self.config.bar;
                b.show_brand = true;
                b.show_workspaces = true;
                b.show_media = false;
                b.show_clock = true;
                b.show_reminders_badge = true;
                b.show_network = true;
                b.show_audio = true;
                b.show_brightness = false;
                b.show_cpu = false;
                b.show_memory = false;
                b.show_storage = false;
                b.show_battery = true;
                b.show_bluetooth = false;
                b.show_wallpaper = false;
                b.show_theme = false;
                b.show_reminders_btn = false;
                b.show_screenshot = false;
                b.show_clipboard = false;
                b.show_notifications = true;
                b.show_control_center = true;
                b.show_terminal = false;
                b.show_power = true;
                let _ = self.config.save_to_default_path();
            }
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
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::ToggleBluetooth => {
                BluetoothManager::toggle_power();
                std::thread::sleep(std::time::Duration::from_millis(150));
                return Command::done(Message::Refresh);
            }
            Message::OpenBluetoothManager => {
                spawn_detached("bluetooth");
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::ToggleNightLight => {
                let new_state = NightLightManager::toggle();
                self.is_night_light = new_state;
            }
            Message::OpenDisplayManager => {
                spawn_detached("display");
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::OpenStorageManager => {
                spawn_detached("storage");
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::OpenMediaManager => {
                spawn_detached("media");
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::OpenReminderManager => {
                spawn_detached("reminder");
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::OpenAppDrawer => {
                spawn_detached("drawer");
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::OpenClipboardManager => {
                spawn_detached("clipboard");
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::OpenFShare => {
                spawn_detached("fshare");
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
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
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::SuspendSystem => {
                SystemControl::suspend();
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::RebootSystem => {
                SystemControl::reboot();
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::PowerOffSystem => {
                SystemControl::poweroff();
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
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
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
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
        let title_icon = text(if self.active_tab == ControlCenterTab::QuickSettings { "󰒓" } else { "󰍹" }).size(18);
        let title_label = text(if self.active_tab == ControlCenterTab::QuickSettings { "Quick Settings" } else { "Top Bar Settings" }).size(15);
        let header_left = row![title_icon, title_label].spacing(8).align_y(Alignment::Center);

        let close_btn = button(text("✕").size(12))
            .style(button::danger)
            .padding([3, 8])
            .on_press(Message::Close);

        let header = row![header_left, space().width(Length::Fill), close_btn]
            .align_y(Alignment::Center);

        // --- Tab Navigation Switcher ---
        let is_quick = self.active_tab == ControlCenterTab::QuickSettings;
        let tab_quick = button(
            row![text("󰒓").size(14), text("Quick Settings").size(12)]
                .spacing(6)
                .align_y(Alignment::Center),
        )
        .width(Length::FillPortion(1))
        .style(if is_quick { button::primary } else { button::secondary })
        .padding([6, 10])
        .on_press(Message::SwitchTab(ControlCenterTab::QuickSettings));

        let tab_bar = button(
            row![text("󰍹").size(14), text("Top Bar").size(12)]
                .spacing(6)
                .align_y(Alignment::Center),
        )
        .width(Length::FillPortion(1))
        .style(if !is_quick { button::primary } else { button::secondary })
        .padding([6, 10])
        .on_press(Message::SwitchTab(ControlCenterTab::TopBarSettings));

        let tab_row = row![tab_quick, tab_bar].spacing(8);

        // --- Top Bar Settings View ---
        if self.active_tab == ControlCenterTab::TopBarSettings {
            let is_modern = self.config.bar.is_modern();
            let modern_btn = button(
                row![text("󰍹").size(14), text("Modern Floating").size(11)]
                    .spacing(6)
                    .align_y(Alignment::Center),
            )
            .width(Length::FillPortion(1))
            .style(if is_modern { button::primary } else { button::secondary })
            .padding([7, 10])
            .on_press(Message::SetBarStyle("modern".to_string()));

            let classic_btn = button(
                row![text("󰍺").size(14), text("Classic Docked").size(11)]
                    .spacing(6)
                    .align_y(Alignment::Center),
            )
            .width(Length::FillPortion(1))
            .style(if !is_modern { button::primary } else { button::secondary })
            .padding([7, 10])
            .on_press(Message::SetBarStyle("classic".to_string()));

            let style_card = container(
                column![
                    row![
                        text("Bar Appearance Style").size(12),
                        space().width(Length::Fill),
                        text(if is_modern { "Floating Acrylic Island" } else { "Full-Width Docked" }).size(10).style(text::secondary),
                    ]
                    .align_y(Alignment::Center),
                    row![modern_btn, classic_btn].spacing(8),
                ]
                .spacing(8),
            )
            .style(container::bordered_box)
            .padding([10, 12])
            .width(Length::Fill);

            let is_frosted = self.config.bar.is_frosted();
            let is_transp = self.config.bar.is_transparent();
            let is_solid = self.config.bar.is_solid();

            let frosted_btn = button(
                row![text("󰖲").size(13), text("Frosted Glass").size(11)]
                    .spacing(5)
                    .align_y(Alignment::Center),
            )
            .width(Length::FillPortion(1))
            .style(if is_frosted { button::primary } else { button::secondary })
            .padding([6, 8])
            .on_press(Message::SetBarTransparency("frosted".to_string()));

            let transp_btn = button(
                row![text("󰈈").size(13), text("Pure Transparent").size(11)]
                    .spacing(5)
                    .align_y(Alignment::Center),
            )
            .width(Length::FillPortion(1))
            .style(if is_transp { button::primary } else { button::secondary })
            .padding([6, 8])
            .on_press(Message::SetBarTransparency("transparent".to_string()));

            let solid_btn = button(
                row![text("󰃞").size(13), text("Solid Opaque").size(11)]
                    .spacing(5)
                    .align_y(Alignment::Center),
            )
            .width(Length::FillPortion(1))
            .style(if is_solid { button::primary } else { button::secondary })
            .padding([6, 8])
            .on_press(Message::SetBarTransparency("solid".to_string()));

            let glass_card = container(
                column![
                    row![
                        text("Bar Glass & Transparency").size(12),
                        space().width(Length::Fill),
                        text(if is_frosted {
                            "Frosted Acrylic Blur"
                        } else if is_transp {
                            "100% Pure Transparent"
                        } else {
                            "Solid Theme Fill"
                        })
                        .size(10)
                        .style(text::secondary),
                    ]
                    .align_y(Alignment::Center),
                    row![frosted_btn, transp_btn, solid_btn].spacing(8),
                ]
                .spacing(8),
            )
            .style(container::bordered_box)
            .padding([10, 12])
            .width(Length::Fill);

            let cur_ws = self.config.bar.workspace_count;
            let ws_3_btn = button(
                row![text("󰮯").size(13), text("3 Slots").size(11)]
                    .spacing(5)
                    .align_y(Alignment::Center),
            )
            .width(Length::FillPortion(1))
            .style(if cur_ws == 3 { button::primary } else { button::secondary })
            .padding([6, 8])
            .on_press(Message::SetWorkspaceCount(3));

            let ws_4_btn = button(
                row![text("󰮯").size(13), text("4 Slots").size(11)]
                    .spacing(5)
                    .align_y(Alignment::Center),
            )
            .width(Length::FillPortion(1))
            .style(if cur_ws == 4 { button::primary } else { button::secondary })
            .padding([6, 8])
            .on_press(Message::SetWorkspaceCount(4));

            let ws_5_btn = button(
                row![text("󰮯").size(13), text("5 Slots").size(11)]
                    .spacing(5)
                    .align_y(Alignment::Center),
            )
            .width(Length::FillPortion(1))
            .style(if cur_ws == 5 { button::primary } else { button::secondary })
            .padding([6, 8])
            .on_press(Message::SetWorkspaceCount(5));

            let ws_8_btn = button(
                row![text("󰮯").size(13), text("8 Slots").size(11)]
                    .spacing(5)
                    .align_y(Alignment::Center),
            )
            .width(Length::FillPortion(1))
            .style(if cur_ws == 8 { button::primary } else { button::secondary })
            .padding([6, 8])
            .on_press(Message::SetWorkspaceCount(8));

            let workspace_card = container(
                column![
                    row![
                        text("Workspace Pager Slots").size(12),
                        space().width(Length::Fill),
                        text(format!("{} Workspaces Displayed", cur_ws)).size(10).style(text::secondary),
                    ]
                    .align_y(Alignment::Center),
                    row![ws_3_btn, ws_4_btn, ws_5_btn, ws_8_btn].spacing(8),
                ]
                .spacing(8),
            )
            .style(container::bordered_box)
            .padding([10, 12])
            .width(Length::Fill);

            let btn_enable_all = button(
                row![text("󰐥").size(12), text("Enable All").size(11)]
                    .spacing(4)
                    .align_y(Alignment::Center),
            )
            .style(button::secondary)
            .padding([4, 8])
            .on_press(Message::EnableAllBarModules);

            let btn_minimal = button(
                row![text("󰅖").size(12), text("Minimal Bar").size(11)]
                    .spacing(4)
                    .align_y(Alignment::Center),
            )
            .style(button::secondary)
            .padding([4, 8])
            .on_press(Message::MinimalBarPreset);

            let presets_row = row![
                column![
                    text("Top Bar Modules").size(13),
                    text("Toggle widgets & shortcuts on bar").size(10),
                ].spacing(1),
                space().width(Length::Fill),
                btn_enable_all,
                btn_minimal,
            ]
            .spacing(6)
            .align_y(Alignment::Center);

            let mut modules_col = column![].spacing(6);

            // Group 1: Brand & Navigation
            modules_col = modules_col.push(text("Brand & Navigation").size(11));
            for m in [
                BarModule::Brand,
                BarModule::Workspaces,
                BarModule::Clock,
                BarModule::RemindersBadge,
            ] {
                modules_col = modules_col.push(view_module_row(m, &self.config.bar));
            }

            // Group 2: Hardware & Connectivity
            modules_col = modules_col.push(space().height(4));
            modules_col = modules_col.push(text("Hardware & Connectivity").size(11));
            for m in [
                BarModule::Network,
                BarModule::Audio,
                BarModule::Brightness,
                BarModule::Cpu,
                BarModule::Memory,
                BarModule::Storage,
                BarModule::Battery,
                BarModule::Bluetooth,
            ] {
                modules_col = modules_col.push(view_module_row(m, &self.config.bar));
            }

            // Group 3: Shortcuts & Utilities
            modules_col = modules_col.push(space().height(4));
            modules_col = modules_col.push(text("Shortcuts & Utilities").size(11));
            for m in [
                BarModule::Media,
                BarModule::Wallpaper,
                BarModule::Theme,
                BarModule::RemindersBtn,
                BarModule::Screenshot,
                BarModule::Clipboard,
                BarModule::Notifications,
                BarModule::ControlCenter,
                BarModule::Terminal,
                BarModule::Power,
            ] {
                modules_col = modules_col.push(view_module_row(m, &self.config.bar));
            }

            let scrollable_content = scrollable(modules_col)
                .height(Length::Fill)
                .width(Length::Fill);

            let content = column![
                header,
                tab_row,
                style_card,
                glass_card,
                workspace_card,
                presets_row,
                scrollable_content,
            ]
            .spacing(10)
            .padding(14);

            return container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

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

        // Pill 9: Reminders & Countdown Timers Shortcut
        let reminder_btn = button(
            row![
                text("󰥔").size(16),
                column![
                    text("Timers").size(12),
                    text("Reminders & Alarms").size(10),
                ].spacing(1),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .width(Length::FillPortion(1))
        .style(button::secondary)
        .padding([8, 10])
        .on_press(Message::OpenReminderManager);

        // Pill 10: App Drawer & Window Switcher Shortcut
        let apps_btn = button(
            row![
                text("󱓞").size(16),
                column![
                    text("Apps & Tasks").size(12),
                    text("Window Switcher").size(10),
                ].spacing(1),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .width(Length::FillPortion(1))
        .style(button::secondary)
        .padding([8, 10])
        .on_press(Message::OpenAppDrawer);

        let row_toggles_5 = row![reminder_btn, apps_btn].spacing(8);

        // Pill 11: Clipboard History Shortcut
        let clip_btn = button(
            row![
                text("󰅌").size(16),
                column![
                    text("Clipboard").size(12),
                    text("History & Pins").size(10),
                ].spacing(1),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .width(Length::FillPortion(1))
        .style(button::secondary)
        .padding([8, 10])
        .on_press(Message::OpenClipboardManager);

        // Pill 12: FShare Direct Wi-Fi File Hub Shortcut
        let fshare_btn = button(
            row![
                text("󰅟").size(16),
                column![
                    text("FShare Hub").size(12),
                    text("Wi-Fi Transfer").size(10),
                ].spacing(1),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .width(Length::FillPortion(1))
        .style(button::secondary)
        .padding([8, 10])
        .on_press(Message::OpenFShare);

        let row_toggles_6 = row![clip_btn, fshare_btn].spacing(8);

        let toggles_box = column![row_toggles_1, row_toggles_2, row_toggles_3, row_toggles_4, row_toggles_5, row_toggles_6].spacing(8);

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
            tab_row,
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

fn view_module_row(module: BarModule, cfg: &BarConfig) -> Element<'_, Message> {
    let enabled = module.is_enabled(cfg);
    let (name, icon, desc) = module.details();

    let icon_txt = text(icon).size(15);
    let title_txt = text(name).size(12);
    let desc_txt = text(desc).size(10);

    let info_col = column![title_txt, desc_txt].spacing(1);
    let left_group = row![icon_txt, info_col].spacing(8).align_y(Alignment::Center);

    let toggle_btn = button(
        text(if enabled { "󰄴 ON" } else { "󰅖 OFF" }).size(11),
    )
    .style(if enabled { button::primary } else { button::secondary })
    .padding([4, 10])
    .on_press(Message::ToggleBarModule(module));

    let item_row = row![left_group, space().width(Length::Fill), toggle_btn]
        .spacing(6)
        .align_y(Alignment::Center);

    container(item_row)
        .style(container::bordered_box)
        .padding([6, 10])
        .width(Length::Fill)
        .into()
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
