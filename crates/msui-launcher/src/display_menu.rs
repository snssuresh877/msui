use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::{button, column, container, row, slider, space, text};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use msui_core::{
    BrightnessInfo, BrightnessMonitor, DisplayManager, DisplayMonitorInfo, NightLightManager,
    ThemeManager,
};
use std::time::Duration;

pub fn run_display_menu() -> Result<(), iced_exwlshell::Error> {
    application(
        DisplayMenu::new,
        DisplayMenu::namespace,
        DisplayMenu::update,
        DisplayMenu::view,
    )
    .theme(ThemeManager::current_theme())
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(700, 620),
        anchor: Anchor::empty(), // Centered on screen
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    })
    .subscription(DisplayMenu::subscription)
    .run()
}

pub struct DisplayMenu {
    monitors: Vec<DisplayMonitorInfo>,
    selected_idx: usize,
    brightness_info: Option<BrightnessInfo>,
    brightness_monitor: BrightnessMonitor,
    night_light_active: bool,
    night_light_temp: u32,
    status_msg: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectMonitor(usize),
    SetScale(f32),
    SetMode(String),
    SetRotation(u32),
    SetBrightness(u8),
    AdjustBrightness(i8),
    ToggleNightLight,
    SetNightLightTemp(u32),
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

impl DisplayMenu {
    fn new() -> (Self, Command<Message>) {
        let monitors = DisplayManager::list_monitors();
        let brightness_monitor = BrightnessMonitor::new();
        let brightness_info = brightness_monitor.read();
        let night_light_active = NightLightManager::is_active();
        let night_light_temp = NightLightManager::get_temperature();

        (
            Self {
                monitors,
                selected_idx: 0,
                brightness_info,
                brightness_monitor,
                night_light_active,
                night_light_temp,
                status_msg: None,
            },
            Command::none(),
        )
    }

    fn namespace() -> String {
        String::from("msui-display-menu")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        let tick = iced::time::every(Duration::from_millis(3000)).map(|_| Message::Refresh);
        let events = event::listen().map(Message::IcedEvent);
        iced::Subscription::batch([tick, events])
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SelectMonitor(idx) => {
                if idx < self.monitors.len() {
                    self.selected_idx = idx;
                }
            }
            Message::SetScale(scale) => {
                if let Some(mon) = self.monitors.get(self.selected_idx) {
                    let name = mon.name.clone();
                    if DisplayManager::set_scale(&name, scale) {
                        self.status_msg = Some(format!("Set scale on {} to {:.2}x", name, scale));
                        self.monitors = DisplayManager::list_monitors();
                    } else {
                        self.status_msg = Some(format!("Failed to set scale on {}", name));
                    }
                }
            }
            Message::SetMode(mode) => {
                if let Some(mon) = self.monitors.get(self.selected_idx) {
                    let name = mon.name.clone();
                    if DisplayManager::set_mode(&name, &mode) {
                        self.status_msg = Some(format!("Set display mode on {} to {}", name, mode));
                        self.monitors = DisplayManager::list_monitors();
                    } else {
                        self.status_msg = Some(format!("Failed to set mode on {}", name));
                    }
                }
            }
            Message::SetRotation(rot) => {
                if let Some(mon) = self.monitors.get(self.selected_idx) {
                    let name = mon.name.clone();
                    if DisplayManager::set_rotation(&name, rot) {
                        self.status_msg = Some(format!("Set orientation on {} to {}°", name, rot));
                        self.monitors = DisplayManager::list_monitors();
                    } else {
                        self.status_msg = Some(format!("Failed to rotate {}", name));
                    }
                }
            }
            Message::SetBrightness(val) => {
                BrightnessMonitor::set(val);
                if let Some(b) = &mut self.brightness_info {
                    b.percent = val;
                }
            }
            Message::AdjustBrightness(delta) => {
                if delta >= 0 {
                    BrightnessMonitor::increase(delta as u8);
                } else {
                    BrightnessMonitor::decrease((-delta) as u8);
                }
                self.brightness_info = self.brightness_monitor.read();
            }
            Message::ToggleNightLight => {
                self.night_light_active = NightLightManager::toggle();
                self.status_msg = Some(if self.night_light_active {
                    format!("Night light enabled ({}K)", self.night_light_temp)
                } else {
                    "Night light disabled".to_string()
                });
            }
            Message::SetNightLightTemp(temp) => {
                self.night_light_temp = temp;
                NightLightManager::set_temperature(temp);
                if self.night_light_active {
                    self.status_msg = Some(format!("Night light temperature: {}K", temp));
                }
            }
            Message::Refresh => {
                self.monitors = DisplayManager::list_monitors();
                self.brightness_info = self.brightness_monitor.read();
                self.night_light_active = NightLightManager::is_active();
            }
            Message::Close => {
                std::process::exit(0);
            }
            Message::IcedEvent(Event::Keyboard(KeyboardEvent::KeyPressed { key, .. })) => {
                if let Key::Named(Named::Escape) = key {
                    std::process::exit(0);
                }
            }
            Message::IcedEvent(_) => {}
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        // Header
        let header_title = text("󰍹  Display & Monitor Manager").size(18);

        let refresh_btn = button(text("󰑐 Refresh").size(13))
            .style(button::secondary)
            .padding([6, 12])
            .on_press(Message::Refresh);

        let close_btn = button(text("󰅖").size(15))
            .style(button::danger)
            .padding([4, 10])
            .on_press(Message::Close);

        let header = row![
            header_title,
            space().width(Length::Fill),
            refresh_btn,
            close_btn,
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        // Monitor Selector Tabs (if multiple monitors)
        let mut monitor_tabs = row![].spacing(8);
        for (i, mon) in self.monitors.iter().enumerate() {
            let is_sel = i == self.selected_idx;
            let label = format!("󰍹  {} ({})", mon.name, if mon.name.starts_with("eDP") { "Built-in" } else { "External" });
            let btn = button(text(label).size(12))
                .style(if is_sel { button::primary } else { button::secondary })
                .padding([6, 14])
                .on_press(Message::SelectMonitor(i));
            monitor_tabs = monitor_tabs.push(btn);
        }

        // Active Monitor Configuration Box
        let monitor_config_box = if let Some(mon) = self.monitors.get(self.selected_idx) {
            self.view_monitor_settings(mon)
        } else {
            container(text("No active displays detected").size(14).style(text::secondary))
                .padding(20)
                .into()
        };

        // Section 2: Brightness & Night Light Controls
        let display_adjust_box = self.view_adjustments_section();

        // Footer / Status Banner
        let mut footer = row![].align_y(Alignment::Center).spacing(10);
        if let Some(msg) = &self.status_msg {
            footer = footer.push(text(msg).size(12).style(text::success));
        } else {
            footer = footer.push(
                text("Esc: Close • Live Wayland display controls via Hyprland & hyprsunset")
                    .size(11)
                    .style(text::secondary),
            );
        }

        let main_layout = column![
            header,
            monitor_tabs,
            monitor_config_box,
            display_adjust_box,
            footer,
        ]
        .spacing(12)
        .padding(20);

        container(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(container::bordered_box)
            .into()
    }

    fn view_monitor_settings<'a>(&'a self, mon: &'a DisplayMonitorInfo) -> Element<'a, Message> {
        let desc = text(format!("{} ({})", mon.description, mon.name)).size(14);
        let cur_mode = text(format!(
            "Resolution: {}x{} @ {:.2} Hz • Scale: {:.0}% • Orientation: {}",
            mon.width,
            mon.height,
            mon.refresh_rate,
            mon.scale * 100.0,
            match mon.transform {
                1 => "90°",
                2 => "180°",
                3 => "270°",
                _ => "0° Normal",
            }
        ))
        .size(11)
        .style(text::secondary);

        let info_header = column![desc, cur_mode].spacing(2);

        // Scale Buttons
        let scale_label = text("Display Scaling:").size(12);
        let scales = [(1.0, "100%"), (1.25, "125%"), (1.5, "150%"), (2.0, "200%")];
        let mut scale_row = row![scale_label].spacing(8).align_y(Alignment::Center);
        for (sc, label) in scales {
            let is_cur = (mon.scale - sc).abs() < 0.05;
            let btn = button(text(label).size(11))
                .style(if is_cur { button::primary } else { button::secondary })
                .padding([4, 10])
                .on_press(Message::SetScale(sc));
            scale_row = scale_row.push(btn);
        }

        // Rotation Buttons
        let rot_label = text("Orientation:").size(12);
        let rotations = [(0, "0° Normal"), (90, "90° Right"), (180, "180° Inverted"), (270, "270° Left")];
        let mut rot_row = row![rot_label].spacing(8).align_y(Alignment::Center);
        for (deg, label) in rotations {
            let trans = match deg { 90 => 1, 180 => 2, 270 => 3, _ => 0 };
            let is_cur = mon.transform == trans;
            let btn = button(text(label).size(11))
                .style(if is_cur { button::primary } else { button::secondary })
                .padding([4, 10])
                .on_press(Message::SetRotation(deg));
            rot_row = rot_row.push(btn);
        }

        // Common Modes / Resolution
        let res_label = text("Resolution:").size(12);
        let mut res_row = row![res_label].spacing(6).align_y(Alignment::Center);
        let common_modes = ["1920x1080@60.00Hz", "1680x1050@60.00Hz", "1280x720@60.00Hz"];
        for mode in common_modes {
            let is_cur = format!("{}x{}@{:.2}Hz", mon.width, mon.height, mon.refresh_rate) == mode;
            let btn = button(text(mode.replace("@60.00Hz", "")).size(11))
                .style(if is_cur { button::primary } else { button::secondary })
                .padding([4, 8])
                .on_press(Message::SetMode(mode.to_string()));
            res_row = res_row.push(btn);
        }

        let content = column![info_header, scale_row, rot_row, res_row].spacing(10);

        container(content)
            .padding(14)
            .style(container::bordered_box)
            .into()
    }

    fn view_adjustments_section(&self) -> Element<'_, Message> {
        // Brightness Controls
        let bright_val = self.brightness_info.as_ref().map(|b| b.percent).unwrap_or(50);
        let bright_label = text(format!("󰃠  Screen Brightness: {}%", bright_val)).size(13);

        let bright_down = button(text("-").size(12))
            .style(button::secondary)
            .padding([3, 8])
            .on_press(Message::AdjustBrightness(-5));

        let bright_slider = slider(0..=100, bright_val, Message::SetBrightness).width(Length::Fill);

        let bright_up = button(text("+").size(12))
            .style(button::secondary)
            .padding([3, 8])
            .on_press(Message::AdjustBrightness(5));

        let bright_row = row![bright_label, bright_down, bright_slider, bright_up]
            .spacing(10)
            .align_y(Alignment::Center);

        // Night Light / Blue Light Controls
        let nl_toggle_label = if self.night_light_active {
            "󰃟  Night Light: ON"
        } else {
            "󰃟  Night Light: OFF"
        };

        let nl_btn = button(text(nl_toggle_label).size(12))
            .style(if self.night_light_active {
                button::primary
            } else {
                button::secondary
            })
            .padding([5, 14])
            .on_press(Message::ToggleNightLight);

        let temp_desc = if self.night_light_temp <= 3000 {
            "Candlelight (Max Warm)"
        } else if self.night_light_temp <= 4200 {
            "Warm Amber"
        } else if self.night_light_temp <= 5500 {
            "Soft White"
        } else {
            "Cool Daylight"
        };

        let temp_label = text(format!("{}K ({})", self.night_light_temp, temp_desc)).size(12);

        let temp_slider = slider(2500..=6500, self.night_light_temp, Message::SetNightLightTemp)
            .width(Length::Fill);

        let nl_row = row![nl_btn, temp_slider, temp_label]
            .spacing(12)
            .align_y(Alignment::Center);

        let content = column![bright_row, space().height(4), nl_row].spacing(10);

        container(content)
            .padding(14)
            .style(container::bordered_box)
            .into()
    }
}
