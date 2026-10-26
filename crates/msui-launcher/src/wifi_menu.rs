use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::operation::focus;
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use crate::modal_style::default_modal_card_style;
use msui_core::{NetworkMonitor, ThemeManager, WifiNetwork};
use std::sync::LazyLock;
use std::time::Duration;

static PASSWORD_INPUT_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);

pub fn run_wifi_menu() -> Result<(), iced_exwlshell::Error> {
    application(
        WifiMenu::new,
        WifiMenu::namespace,
        WifiMenu::update,
        WifiMenu::view,
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
    .subscription(WifiMenu::subscription)
    .run()
}

pub struct WifiMenu {
    networks: Vec<WifiNetwork>,
    is_enabled: bool,
    selected_idx: usize,
    password_prompt_ssid: Option<String>,
    password_input: String,
    status_msg: Option<(String, bool)>, // (message, is_error)
    is_standalone: bool,
    should_close: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleRadio,
    Rescan,
    ScanDone(Vec<WifiNetwork>),
    SelectPrevious,
    SelectNext,
    SelectNetwork(usize),
    PromptPassword(String),
    PasswordChanged(String),
    SubmitConnect,
    CancelPassword,
    ConnectDone(Result<String, String>),
    Disconnect,
    Close,
    IcedEvent(Event),
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

impl WifiMenu {
    pub fn new() -> (Self, Command<Message>) {
        Self::new_internal(true)
    }

    pub fn new_embedded() -> (Self, Command<Message>) {
        Self::new_internal(false)
    }

    fn new_internal(is_standalone: bool) -> (Self, Command<Message>) {
        let is_enabled = NetworkMonitor::is_wifi_enabled();
        let networks = if is_enabled {
            NetworkMonitor::scan_wifi()
        } else {
            Vec::new()
        };

        (
            Self {
                networks,
                is_enabled,
                selected_idx: 0,
                password_prompt_ssid: None,
                password_input: String::new(),
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
        self.is_enabled = NetworkMonitor::is_wifi_enabled();
        self.networks = if self.is_enabled {
            NetworkMonitor::scan_wifi()
        } else {
            Vec::new()
        };
        self.password_prompt_ssid = None;
        self.password_input.clear();
        self.status_msg = None;
        self.should_close = false;
        Command::none()
    }

    pub fn namespace() -> String {
        String::from("msui-wifi-menu")
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let events = event::listen().map(Message::IcedEvent);
        let tick = iced::time::every(Duration::from_secs(6)).map(|_| Message::Rescan);
        iced::Subscription::batch([events, tick])
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ToggleRadio => {
                NetworkMonitor::toggle_wifi_radio();
                self.is_enabled = NetworkMonitor::is_wifi_enabled();
                if self.is_enabled {
                    self.networks = NetworkMonitor::scan_wifi();
                    self.status_msg = Some(("Wi-Fi enabled".into(), false));
                } else {
                    self.networks.clear();
                    self.status_msg = Some(("Wi-Fi disabled".into(), false));
                }
            }
            Message::Rescan => {
                if self.is_enabled {
                    NetworkMonitor::rescan_wifi();
                    return Command::perform(
                        async {
                            tokio::time::sleep(Duration::from_millis(800)).await;
                            NetworkMonitor::scan_wifi()
                        },
                        Message::ScanDone,
                    );
                }
            }
            Message::ScanDone(scanned) => {
                self.networks = scanned;
                self.is_enabled = NetworkMonitor::is_wifi_enabled();
            }
            Message::SelectNext => {
                if !self.networks.is_empty() && self.password_prompt_ssid.is_none() {
                    self.selected_idx = (self.selected_idx + 1) % self.networks.len();
                }
            }
            Message::SelectPrevious => {
                if !self.networks.is_empty() && self.password_prompt_ssid.is_none() {
                    if self.selected_idx == 0 {
                        self.selected_idx = self.networks.len() - 1;
                    } else {
                        self.selected_idx -= 1;
                    }
                }
            }
            Message::SelectNetwork(idx) => {
                if let Some(net) = self.networks.get(idx) {
                    self.selected_idx = idx;
                    if net.is_connected {
                        // Already connected
                        return Command::none();
                    }
                    if net.is_secure {
                        return Command::done(Message::PromptPassword(net.ssid.clone()));
                    } else {
                        let ssid = net.ssid.clone();
                        self.status_msg = Some((format!("Connecting to {}...", &ssid), false));
                        return Command::perform(
                            async move {
                                NetworkMonitor::connect_wifi(&ssid, None)
                                    .map(|_| ssid)
                            },
                            Message::ConnectDone,
                        );
                    }
                }
            }
            Message::PromptPassword(ssid) => {
                self.password_prompt_ssid = Some(ssid);
                self.password_input.clear();
                return focus(PASSWORD_INPUT_ID.clone());
            }
            Message::PasswordChanged(new_pwd) => {
                self.password_input = new_pwd;
            }
            Message::CancelPassword => {
                self.password_prompt_ssid = None;
                self.password_input.clear();
            }
            Message::SubmitConnect => {
                if let Some(ssid) = self.password_prompt_ssid.take() {
                    let pwd = self.password_input.clone();
                    self.password_input.clear();
                    self.status_msg = Some((format!("Connecting to \"{}\"...", &ssid), false));
                    return Command::perform(
                        async move {
                            NetworkMonitor::connect_wifi(&ssid, Some(&pwd))
                                .map(|_| ssid)
                        },
                        Message::ConnectDone,
                    );
                }
            }
            Message::ConnectDone(result) => {
                match result {
                    Ok(ssid) => {
                        self.status_msg = Some((format!("Successfully connected to \"{}\"", ssid), false));
                        self.networks = NetworkMonitor::scan_wifi();
                    }
                    Err(err) => {
                        self.status_msg = Some((format!("Connection failed: {}", err), true));
                    }
                }
            }
            Message::Disconnect => {
                if NetworkMonitor::disconnect_wifi() {
                    self.status_msg = Some(("Disconnected from Wi-Fi".into(), false));
                    self.networks = NetworkMonitor::scan_wifi();
                } else {
                    self.status_msg = Some(("Failed to disconnect".into(), true));
                }
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
                        if self.password_prompt_ssid.is_some() {
                            return Command::done(Message::CancelPassword);
                        } else {
                            return Command::done(Message::Close);
                        }
                    }
                    Key::Named(Named::ArrowDown) => {
                        if self.password_prompt_ssid.is_none() {
                            return Command::done(Message::SelectNext);
                        }
                    }
                    Key::Named(Named::ArrowUp) => {
                        if self.password_prompt_ssid.is_none() {
                            return Command::done(Message::SelectPrevious);
                        }
                    }
                    Key::Named(Named::Enter) => {
                        if self.password_prompt_ssid.is_some() {
                            return Command::done(Message::SubmitConnect);
                        } else if !self.networks.is_empty() {
                            return Command::done(Message::SelectNetwork(self.selected_idx));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Command::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let title = text("󰤨 Wi-Fi Network Manager").size(18);

        let rescan_btn = button(text("󰑐 Rescan").size(12))
            .style(button::secondary)
            .padding([5, 12])
            .on_press(Message::Rescan);

        let power_label = if self.is_enabled {
            "󰤨 Wi-Fi: ON"
        } else {
            "󰤮 Wi-Fi: OFF"
        };
        let power_style = if self.is_enabled {
            button::primary
        } else {
            button::secondary
        };
        let power_btn = button(text(power_label).size(12))
            .style(power_style)
            .padding([5, 12])
            .on_press(Message::ToggleRadio);

        let header = row![title, space().width(Length::Fill), rescan_btn, power_btn]
            .spacing(10)
            .align_y(Alignment::Center);

        let mut content_col = column![header].spacing(12);

        // Status Banner
        if let Some((msg, is_error)) = &self.status_msg {
            let banner_text = text(msg).size(12);
            let banner_container = container(banner_text)
                .width(Length::Fill)
                .padding([6, 12])
                .style(if *is_error {
                    container::bordered_box
                } else {
                    container::bordered_box
                });
            content_col = content_col.push(banner_container);
        }

        // Password Prompt Card
        if let Some(prompt_ssid) = &self.password_prompt_ssid {
            let prompt_title = text(format!("󰌾 Enter password for \"{}\"", prompt_ssid)).size(14);
            let input = text_input("Password...", &self.password_input)
                .id(PASSWORD_INPUT_ID.clone())
                .secure(true)
                .on_input(Message::PasswordChanged)
                .on_submit(Message::SubmitConnect)
                .padding(10)
                .size(14);

            let connect_btn = button(text("Connect").size(12))
                .style(button::primary)
                .padding([6, 14])
                .on_press(Message::SubmitConnect);

            let cancel_btn = button(text("Cancel").size(12))
                .style(button::secondary)
                .padding([6, 14])
                .on_press(Message::CancelPassword);

            let action_row = row![connect_btn, cancel_btn]
                .spacing(8)
                .align_y(Alignment::Center);

            let prompt_card = container(column![prompt_title, input, action_row].spacing(10))
                .width(Length::Fill)
                .padding(14)
                .style(container::bordered_box);

            content_col = content_col.push(prompt_card);
        }

        // Network List
        let mut list_col = column![].spacing(6);

        if !self.is_enabled {
            list_col = list_col.push(
                container(text("Wi-Fi is turned OFF. Click 'Wi-Fi: OFF' to enable.").size(13))
                    .padding(32)
                    .align_x(Alignment::Center),
            );
        } else if self.networks.is_empty() {
            list_col = list_col.push(
                container(text("No Wi-Fi networks found. Scanning...").size(13))
                    .padding(32)
                    .align_x(Alignment::Center),
            );
        } else {
            for (idx, net) in self.networks.iter().enumerate() {
                let is_selected = idx == self.selected_idx;

                let signal_icon = if net.signal > 75 {
                    "󰤨"
                } else if net.signal > 50 {
                    "󰤥"
                } else if net.signal > 25 {
                    "󰤢"
                } else {
                    "󰤟"
                };

                let icon = text(signal_icon).size(18);
                let ssid = text(&net.ssid).size(14);

                let security_str = if net.is_secure {
                    format!("󰌾 {}", &net.security)
                } else {
                    "Open".to_string()
                };
                let meta = text(format!("{} • {}% {}", security_str, net.signal, net.bars)).size(11);
                let text_stack = column![ssid, meta].spacing(2);

                let mut row_content = row![icon, text_stack]
                    .spacing(12)
                    .align_y(Alignment::Center);

                row_content = row_content.push(space().width(Length::Fill));

                if net.is_connected {
                    let connected_badge = text("󰄬 Connected").size(12);
                    let disconnect_btn = button(text("Disconnect").size(11))
                        .style(button::danger)
                        .padding([4, 10])
                        .on_press(Message::Disconnect);

                    row_content = row_content.push(connected_badge);
                    row_content = row_content.push(disconnect_btn);
                } else {
                    let connect_btn = button(text("Connect").size(11))
                        .style(button::primary)
                        .padding([4, 10])
                        .on_press(Message::SelectNetwork(idx));

                    row_content = row_content.push(connect_btn);
                }

                let btn_style = if is_selected {
                    button::primary
                } else {
                    button::secondary
                };

                let item_card = button(row_content)
                    .width(Length::Fill)
                    .style(btn_style)
                    .padding([6, 12])
                    .on_press(Message::SelectNetwork(idx));

                list_col = list_col.push(item_card);
            }
        }

        let footer = row![
            text(format!("{} networks found", self.networks.len())).size(11),
            space().width(Length::Fill),
            text("↑↓ Select • Enter Connect • Esc Close").size(11),
        ]
        .align_y(Alignment::Center)
        .padding([4, 6]);

        content_col = content_col.push(scrollable(list_col).height(Length::Fill));
        content_col = content_col.push(footer);

        container(content_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(16)
            .style(default_modal_card_style())
            .into()
    }
}
