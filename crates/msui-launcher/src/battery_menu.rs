use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::{button, column, container, progress_bar, row, space, text};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use crate::modal_style::{default_modal_card_style, default_subcard_style};
use msui_core::{BatteryInfo, BatteryMonitor, BatteryStatus, PowerProfile, PowerProfileManager, ThemeManager};
use std::time::Duration;

pub fn run_battery_menu() -> Result<(), iced_exwlshell::Error> {
    application(
        BatteryMenu::new,
        BatteryMenu::namespace,
        BatteryMenu::update,
        BatteryMenu::view,
    )
    .theme(ThemeManager::current_theme())
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(680, 560),
        anchor: Anchor::empty(), // Centered on screen
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    })
    .subscription(BatteryMenu::subscription)
    .run()
}

pub struct BatteryMenu {
    profile: PowerProfile,
    battery_info: Option<BatteryInfo>,
    battery_monitor: BatteryMonitor,
    status_msg: Option<String>,
    is_standalone: bool,
    should_close: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectProfile(PowerProfile),
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

impl BatteryMenu {
    pub fn new() -> (Self, Command<Message>) {
        Self::new_internal(true)
    }

    pub fn new_embedded() -> (Self, Command<Message>) {
        Self::new_internal(false)
    }

    fn new_internal(is_standalone: bool) -> (Self, Command<Message>) {
        let profile = PowerProfileManager::get_profile();
        let battery_monitor = BatteryMonitor::new();
        let battery_info = battery_monitor.read();

        (
            Self {
                profile,
                battery_info,
                battery_monitor,
                status_msg: None,
                is_standalone,
                should_close: false,
            },
            Command::none(),
        )
    }

    pub fn should_close(&self) -> bool {
        self.should_close
    }

    pub fn reset_should_close(&mut self) {
        self.should_close = false;
    }

    pub fn reset(&mut self) -> Command<Message> {
        self.profile = PowerProfileManager::get_profile();
        self.battery_info = self.battery_monitor.read();
        self.status_msg = None;
        self.should_close = false;
        Command::none()
    }

    pub fn namespace() -> String {
        String::from("msui-battery-menu")
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let tick = iced::time::every(Duration::from_millis(2500)).map(|_| Message::Refresh);
        let events = event::listen().map(Message::IcedEvent);
        iced::Subscription::batch([tick, events])
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SelectProfile(prof) => {
                if PowerProfileManager::set_profile(prof) {
                    self.profile = prof;
                    self.status_msg = Some(format!("Switched power profile to {}", prof.display_name()));
                } else {
                    self.status_msg = Some(format!("Failed to switch to {}", prof.display_name()));
                }
            }
            Message::Refresh => {
                self.profile = PowerProfileManager::get_profile();
                self.battery_info = self.battery_monitor.read();
            }
            Message::Close => {
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::IcedEvent(Event::Keyboard(KeyboardEvent::KeyPressed { key, .. })) => {
                match key {
                    Key::Named(Named::Escape) => {
                        return self.update(Message::Close);
                    }
                    Key::Character(c) => match c.as_str() {
                        "1" => return self.update(Message::SelectProfile(PowerProfile::PowerSaver)),
                        "2" => return self.update(Message::SelectProfile(PowerProfile::Balanced)),
                        "3" => return self.update(Message::SelectProfile(PowerProfile::Performance)),
                        _ => {}
                    },
                    _ => {}
                }
            }
            Message::IcedEvent(_) => {}
        }
        Command::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        // Header
        let header_title = text("󰂄  Power & Battery Manager").size(18);

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

        // Section 1: Power Profiles
        let profiles_title = text("Power Mode Profiles").size(14).style(text::secondary);

        let profiles = [
            PowerProfile::PowerSaver,
            PowerProfile::Balanced,
            PowerProfile::Performance,
        ];

        let mut profile_cards = row![].spacing(10).width(Length::Fill);
        for p in profiles {
            profile_cards = profile_cards.push(self.view_profile_card(p));
        }

        // Section 2: Battery Telemetry & Diagnostics
        let battery_section = self.view_battery_section();

        // Footer / Status Banner
        let mut footer = row![].align_y(Alignment::Center).spacing(10);
        if let Some(msg) = &self.status_msg {
            footer = footer.push(text(msg).size(12).style(text::success));
        } else {
            footer = footer.push(
                text("Esc: Close • Press 1, 2, 3 to switch profile • Live sysfs telemetry")
                    .size(11)
                    .style(text::secondary),
            );
        }

        let main_layout = column![
            header,
            profiles_title,
            profile_cards,
            space().height(6),
            battery_section,
            footer,
        ]
        .spacing(12)
        .padding(20);

        container(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(default_modal_card_style())
            .into()
    }

    fn view_profile_card(&self, profile: PowerProfile) -> Element<'_, Message> {
        let is_active = self.profile == profile;

        let icon_el = text(profile.icon()).size(24);
        let title_el = text(profile.display_name()).size(14);
        let desc_el = text(profile.description()).size(10).style(text::secondary);

        let status_badge: Element<'_, Message> = if is_active {
            container(text("󰄬 Active").size(11))
                .padding([2, 8])
                .style(default_subcard_style())
                .into()
        } else {
            space().height(0).into()
        };

        let card_content = column![
            row![icon_el, space().width(Length::Fill), status_badge].align_y(Alignment::Center),
            title_el,
            desc_el,
        ]
        .spacing(6);

        button(card_content)
            .width(Length::FillPortion(1))
            .padding(12)
            .style(if is_active {
                button::primary
            } else {
                button::secondary
            })
            .on_press(Message::SelectProfile(profile))
            .into()
    }

    fn view_battery_section(&self) -> Element<'_, Message> {
        let Some(bat) = &self.battery_info else {
            return container(
                text("No battery detected (Desktop or AC Only)")
                    .size(14)
                    .style(text::secondary),
            )
            .padding(30)
            .center(Length::Fill)
            .style(default_subcard_style())
            .into();
        };

        let bat_icon = if bat.is_charging {
            "󰂄"
        } else if bat.percentage > 80 {
            "󰁹"
        } else if bat.percentage > 60 {
            "󰂀"
        } else if bat.percentage > 40 {
            "󰁼"
        } else if bat.percentage > 20 {
            "󰁼"
        } else {
            "󰁺"
        };

        let percent_str = format!("{bat_icon}  {}%", bat.percentage);
        let status_badge = if bat.is_charging {
            "󰚥 Charging (AC Connected)"
        } else if bat.is_ac_online {
            "󰚥 AC Connected (Full)"
        } else {
            "󰚦 On Battery"
        };

        let top_row = row![
            text(percent_str).size(22),
            space().width(Length::Fill),
            container(text(status_badge).size(12))
                .padding([4, 10])
                .style(default_subcard_style()),
        ]
        .align_y(Alignment::Center);

        // Progress bar (0.0 to 1.0)
        let progress = (bat.percentage as f32 / 100.0).clamp(0.0, 1.0);
        let bar = progress_bar(0.0..=1.0, progress);

        // Time estimate string
        let time_str = if let Some(mins) = bat.time_remaining_mins {
            let h = mins / 60;
            let m = mins % 60;
            if bat.status == BatteryStatus::Discharging {
                format!("󰥔 ~{h}h {m}m remaining")
            } else if bat.is_charging {
                format!("󰥔 ~{h}h {m}m until full")
            } else {
                "󰥔 Battery Full".to_string()
            }
        } else if bat.is_charging {
            "󰥔 Calculating remaining time...".to_string()
        } else {
            "󰥔 AC Power / Idle".to_string()
        };

        let power_str = if bat.power_watts > 0.05 {
            format!("⚡ {:.1} W power rate", bat.power_watts)
        } else {
            "⚡ Idle".to_string()
        };

        let health_str = format!("󰂃 {:.1}% health ({:.1}/{:.1} Wh)", bat.health_percent, bat.energy_full_wh, bat.energy_full_design_wh);
        let volt_str = format!("󰓅 {:.2} V ({})", bat.voltage_volts, bat.technology);
        let cycle_str = if let Some(cycles) = bat.cycle_count {
            format!("󰑐 {cycles} cycles")
        } else {
            "󰑐 Cycles N/A".to_string()
        };

        let grid_row_1 = row![
            column![text(time_str).size(12), text(power_str).size(12).style(text::secondary)].spacing(4).width(Length::FillPortion(1)),
            column![text(health_str).size(12), text(volt_str).size(12).style(text::secondary)].spacing(4).width(Length::FillPortion(1)),
        ]
        .spacing(14);

        let grid_row_2 = row![
            text(format!("Current Charge: {:.1} Wh", bat.energy_now_wh)).size(11).style(text::secondary),
            space().width(Length::Fill),
            text(cycle_str).size(11).style(text::secondary),
        ];

        let content = column![
            top_row,
            bar,
            space().height(4),
            grid_row_1,
            grid_row_2,
        ]
        .spacing(10);

        container(content)
            .padding(16)
            .style(default_modal_card_style())
            .into()
    }
}
