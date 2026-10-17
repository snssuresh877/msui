use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::{button, column, container, row, scrollable, space, text};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use msui_core::{BluetoothInfo, BluetoothManager, ThemeManager};
use std::time::Duration;

pub fn run_bluetooth_menu() -> Result<(), iced_exwlshell::Error> {
    application(
        BluetoothMenu::new,
        BluetoothMenu::namespace,
        BluetoothMenu::update,
        BluetoothMenu::view,
    )
    .theme(ThemeManager::current_theme())
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(680, 500),
        anchor: Anchor::empty(), // Centered on screen
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    })
    .subscription(BluetoothMenu::subscription)
    .run()
}

pub struct BluetoothMenu {
    info: BluetoothInfo,
    selected_idx: usize,
    status_msg: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePower,
    Connect(String),
    Disconnect(String),
    StartScan,
    SelectPrevious,
    SelectNext,
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

impl BluetoothMenu {
    fn new() -> (Self, Command<Message>) {
        let info = BluetoothManager::read();
        (
            Self {
                info,
                selected_idx: 0,
                status_msg: None,
            },
            Command::none(),
        )
    }

    fn namespace() -> String {
        String::from("msui-bluetooth-menu")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        let tick = iced::time::every(Duration::from_secs(2)).map(|_| Message::Refresh);
        let events = event::listen().map(Message::IcedEvent);
        iced::Subscription::batch([tick, events])
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::TogglePower => {
                BluetoothManager::toggle_power();
                self.info = BluetoothManager::read();
                self.status_msg = Some(if self.info.is_powered {
                    "Bluetooth powered ON".to_string()
                } else {
                    "Bluetooth powered OFF".to_string()
                });
            }
            Message::Connect(mac) => {
                self.status_msg = Some(format!("Connecting to {}...", &mac));
                let success = BluetoothManager::connect(&mac);
                self.info = BluetoothManager::read();
                self.status_msg = Some(if success {
                    format!("Connected to {}", &mac)
                } else {
                    format!("Failed to connect to {}", &mac)
                });
            }
            Message::Disconnect(mac) => {
                BluetoothManager::disconnect(&mac);
                self.info = BluetoothManager::read();
                self.status_msg = Some(format!("Disconnected from {}", &mac));
            }
            Message::StartScan => {
                BluetoothManager::start_scan();
                self.status_msg = Some("Scanning for new Bluetooth devices...".to_string());
            }
            Message::SelectNext => {
                if !self.info.paired_devices.is_empty() {
                    self.selected_idx = (self.selected_idx + 1) % self.info.paired_devices.len();
                }
            }
            Message::SelectPrevious => {
                if !self.info.paired_devices.is_empty() {
                    if self.selected_idx == 0 {
                        self.selected_idx = self.info.paired_devices.len() - 1;
                    } else {
                        self.selected_idx -= 1;
                    }
                }
            }
            Message::Refresh => {
                self.info = BluetoothManager::read();
            }
            Message::Close => {
                std::process::exit(0);
            }
            Message::IcedEvent(Event::Keyboard(KeyboardEvent::KeyPressed { key, .. })) => {
                match key {
                    Key::Named(Named::Escape) => {
                        return Command::done(Message::Close);
                    }
                    Key::Named(Named::ArrowDown) => {
                        return Command::done(Message::SelectNext);
                    }
                    Key::Named(Named::ArrowUp) => {
                        return Command::done(Message::SelectPrevious);
                    }
                    Key::Named(Named::Enter) => {
                        if let Some(dev) = self.info.paired_devices.get(self.selected_idx) {
                            if dev.connected {
                                return Command::done(Message::Disconnect(dev.mac.clone()));
                            } else {
                                return Command::done(Message::Connect(dev.mac.clone()));
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let title = text("󰂯 Bluetooth Manager").size(18);

        let power_label = if self.info.is_powered {
            "󰂯 Bluetooth: ON"
        } else {
            "󰂲 Bluetooth: OFF"
        };
        let power_style = if self.info.is_powered {
            button::primary
        } else {
            button::secondary
        };
        let power_btn = button(text(power_label).size(13))
            .style(power_style)
            .padding([6, 14])
            .on_press(Message::TogglePower);

        let scan_btn = button(text("󰑐 Scan Devices").size(13))
            .style(button::secondary)
            .padding([6, 12])
            .on_press(Message::StartScan);

        let header = row![title, space().width(Length::Fill), scan_btn, power_btn]
            .spacing(10)
            .align_y(Alignment::Center)
            .padding([0, 4]);

        let mut dev_list = column![].spacing(8);

        if !self.info.is_powered {
            dev_list = dev_list.push(
                container(text("Bluetooth adapter is turned OFF. Click 'Bluetooth: OFF' to enable.").size(13))
                    .padding(32)
                    .align_x(Alignment::Center),
            );
        } else if self.info.paired_devices.is_empty() {
            dev_list = dev_list.push(
                container(text("No paired Bluetooth devices found. Click 'Scan Devices' to discover.").size(13))
                    .padding(32)
                    .align_x(Alignment::Center),
            );
        } else {
            for (idx, dev) in self.info.paired_devices.iter().enumerate() {
                let is_selected = idx == self.selected_idx;

                let icon = text(&dev.icon).size(20);
                let name = text(&dev.name).size(14);
                let mac = text(&dev.mac).size(11);
                let text_stack = column![name, mac].spacing(2);

                let status_badge = if dev.connected {
                    text("󰄬 Connected").size(12)
                } else {
                    text("Disconnected").size(12)
                };

                let action_btn = if dev.connected {
                    button(text("Disconnect").size(11))
                        .style(button::danger)
                        .padding([4, 10])
                        .on_press(Message::Disconnect(dev.mac.clone()))
                } else {
                    button(text("Connect").size(11))
                        .style(button::primary)
                        .padding([4, 10])
                        .on_press(Message::Connect(dev.mac.clone()))
                };

                let dev_row = row![
                    icon,
                    text_stack,
                    space().width(Length::Fill),
                    status_badge,
                    action_btn,
                ]
                .spacing(14)
                .align_y(Alignment::Center)
                .padding([6, 10]);

                let card_style = if is_selected {
                    container::bordered_box
                } else {
                    container::transparent
                };

                let card = container(dev_row)
                    .width(Length::Fill)
                    .style(card_style);

                dev_list = dev_list.push(card);
            }
        }

        let footer_status = text(self.status_msg.as_deref().unwrap_or("Ready")).size(11);
        let footer_hints = text("↑↓ Select • Enter Toggle Connect • Esc Close").size(11);

        let footer = row![
            footer_status,
            space().width(Length::Fill),
            footer_hints,
        ]
        .align_y(Alignment::Center)
        .padding([4, 6]);

        let content = column![
            header,
            scrollable(dev_list).height(Length::Fill),
            footer,
        ]
        .spacing(12)
        .padding(16);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(container::bordered_box)
            .into()
    }
}
