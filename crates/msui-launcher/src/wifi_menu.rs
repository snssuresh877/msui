use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::operation::focus;
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize, Margin};
use iced_exwlshell::settings::LayerShellSettings;
use std::sync::LazyLock;
use std::time::Duration;

use crate::modal_style::default_modal_card_style;
use msui_core::network::{
    NetworkService, ProfilesManager, WifiDetails, WifiManager, WifiNetwork,
};
use msui_core::ThemeManager;

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
        size: LayerSize::px(440, 620),
        anchor: Anchor::Top | Anchor::Right,
        margin: Margin {
            top: 38,
            right: 12,
            bottom: 0,
            left: 0,
        },
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::OnDemand,
        ..Default::default()
    })
    .subscription(WifiMenu::subscription)
    .run()
}

pub struct WifiMenu {
    networks: Vec<WifiNetwork>,
    search_query: String,
    connection_details: Option<WifiDetails>,
    is_enabled: bool,
    is_scanning: bool,
    selected_idx: usize,
    password_prompt_ssid: Option<String>,
    password_input: String,
    show_password: bool,
    status_msg: Option<(String, bool)>, // (message, is_error)
    is_standalone: bool,
    should_close: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleRadio,
    RadioToggled(bool),
    Rescan,
    ScanDone(Vec<WifiNetwork>),
    DetailsLoaded(Option<WifiDetails>),
    SearchChanged(String),
    ToggleShowPassword,
    SelectPrevious,
    SelectNext,
    SelectNetwork(usize),
    PromptPassword(String),
    PasswordChanged(String),
    SubmitConnect,
    CancelPassword,
    ConnectDone(Result<String, String>),
    Disconnect,
    DisconnectDone(Result<(), String>),
    ForgetNetwork(String),
    ForgetDone(Result<(), String>),
    LaunchTui,
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
        let net_state = NetworkService::read_state();
        let is_enabled = net_state.wifi_enabled;
        let networks = WifiManager::get_cached_networks();

        let (init_cmd, is_scanning) = if is_enabled {
            (
                Command::batch([
                    Command::perform(WifiManager::scan_networks(false), Message::ScanDone),
                    Command::perform(WifiManager::get_connection_details(), Message::DetailsLoaded),
                ]),
                true,
            )
        } else {
            (Command::none(), false)
        };

        (
            Self {
                networks,
                search_query: String::new(),
                connection_details: None,
                is_enabled,
                is_scanning,
                selected_idx: 0,
                password_prompt_ssid: None,
                password_input: String::new(),
                show_password: false,
                status_msg: None,
                is_standalone,
                should_close: false,
            },
            init_cmd,
        )
    }

    pub fn should_close(&self) -> bool {
        self.should_close
    }

    pub fn reset_should_close(&mut self) {
        self.should_close = false;
    }

    pub fn reset(&mut self) -> Command<Message> {
        let net_state = NetworkService::read_state();
        self.is_enabled = net_state.wifi_enabled;
        self.networks = WifiManager::get_cached_networks();
        self.search_query.clear();
        self.password_prompt_ssid = None;
        self.password_input.clear();
        self.show_password = false;
        self.status_msg = None;
        self.should_close = false;

        if self.is_enabled {
            self.is_scanning = true;
            Command::batch([
                Command::perform(WifiManager::scan_networks(false), Message::ScanDone),
                Command::perform(WifiManager::get_connection_details(), Message::DetailsLoaded),
            ])
        } else {
            self.is_scanning = false;
            Command::none()
        }
    }

    pub fn namespace() -> String {
        String::from("msui-wifi-menu")
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let events = event::listen().map(Message::IcedEvent);
        let tick = iced::time::every(Duration::from_secs(12)).map(|_| Message::Rescan);
        iced::Subscription::batch([events, tick])
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ToggleRadio => {
                let will_enable = !self.is_enabled;
                self.is_enabled = will_enable;
                if will_enable {
                    self.is_scanning = true;
                    self.status_msg = Some(("Enabling Wi-Fi...".into(), false));
                    return Command::perform(
                        async move {
                            let _ = WifiManager::set_radio_enabled(true).await;
                            WifiManager::scan_networks(true).await
                        },
                        Message::ScanDone,
                    );
                } else {
                    self.networks.clear();
                    self.connection_details = None;
                    self.is_scanning = false;
                    self.status_msg = Some(("Wi-Fi disabled".into(), false));
                    return Command::perform(
                        async move {
                            let _ = WifiManager::set_radio_enabled(false).await;
                        },
                        |_| Message::RadioToggled(false),
                    );
                }
            }
            Message::RadioToggled(is_on) => {
                self.is_enabled = is_on;
            }
            Message::Rescan => {
                if self.is_enabled {
                    self.is_scanning = true;
                    return Command::batch([
                        Command::perform(WifiManager::scan_networks(true), Message::ScanDone),
                        Command::perform(WifiManager::get_connection_details(), Message::DetailsLoaded),
                    ]);
                }
            }
            Message::ScanDone(scanned) => {
                self.is_scanning = false;
                self.networks = scanned;
                if self.selected_idx >= self.networks.len() && !self.networks.is_empty() {
                    self.selected_idx = 0;
                }
            }
            Message::DetailsLoaded(details) => {
                self.connection_details = details;
            }
            Message::SearchChanged(query) => {
                self.search_query = query;
                self.selected_idx = 0;
            }
            Message::ToggleShowPassword => {
                self.show_password = !self.show_password;
            }
            Message::SelectNext => {
                let filtered_len = self.filtered_networks().len();
                if filtered_len > 0 && self.password_prompt_ssid.is_none() {
                    self.selected_idx = (self.selected_idx + 1) % filtered_len;
                }
            }
            Message::SelectPrevious => {
                let filtered_len = self.filtered_networks().len();
                if filtered_len > 0 && self.password_prompt_ssid.is_none() {
                    if self.selected_idx == 0 {
                        self.selected_idx = filtered_len - 1;
                    } else {
                        self.selected_idx -= 1;
                    }
                }
            }
            Message::SelectNetwork(idx) => {
                let network_opt = self.filtered_networks().get(idx).map(|n| {
                    (n.is_connected, n.is_secure, n.ssid.clone())
                });
                if let Some((is_conn, is_sec, ssid)) = network_opt {
                    self.selected_idx = idx;
                    if is_conn {
                        return Command::none();
                    }
                    if is_sec {
                        return Command::done(Message::PromptPassword(ssid));
                    } else {
                        self.status_msg = Some((format!("Connecting to {}...", ssid), false));
                        return Command::perform(
                            async move {
                                WifiManager::connect_network(&ssid, None).await.map(|_| ssid)
                            },
                            Message::ConnectDone,
                        );
                    }
                }
            }
            Message::PromptPassword(ssid) => {
                self.password_prompt_ssid = Some(ssid);
                self.password_input.clear();
                self.show_password = false;
                return focus(PASSWORD_INPUT_ID.clone());
            }
            Message::PasswordChanged(new_pwd) => {
                self.password_input = new_pwd;
            }
            Message::CancelPassword => {
                self.password_prompt_ssid = None;
                self.password_input.clear();
                self.show_password = false;
            }
            Message::SubmitConnect => {
                if let Some(ssid) = self.password_prompt_ssid.take() {
                    let pwd = self.password_input.clone();
                    self.password_input.clear();
                    self.show_password = false;
                    self.status_msg = Some((format!("Connecting to \"{}\"...", ssid), false));
                    return Command::perform(
                        async move {
                            WifiManager::connect_network(&ssid, Some(&pwd))
                                .await
                                .map(|_| ssid)
                        },
                        Message::ConnectDone,
                    );
                }
            }
            Message::ConnectDone(result) => {
                match result {
                    Ok(ssid) => {
                        self.status_msg =
                            Some((format!("Connected to \"{}\"", ssid), false));
                        self.is_scanning = true;
                        return Command::batch([
                            Command::perform(WifiManager::scan_networks(false), Message::ScanDone),
                            Command::perform(
                                WifiManager::get_connection_details(),
                                Message::DetailsLoaded,
                            ),
                        ]);
                    }
                    Err(err) => {
                        self.status_msg = Some((format!("Connection failed: {}", err), true));
                    }
                }
            }
            Message::Disconnect => {
                self.status_msg = Some(("Disconnecting...".into(), false));
                return Command::perform(
                    async move { WifiManager::disconnect_network().await },
                    Message::DisconnectDone,
                );
            }
            Message::DisconnectDone(res) => {
                match res {
                    Ok(_) => {
                        self.status_msg = Some(("Disconnected from Wi-Fi".into(), false));
                        self.connection_details = None;
                        self.is_scanning = true;
                        return Command::perform(
                            WifiManager::scan_networks(false),
                            Message::ScanDone,
                        );
                    }
                    Err(e) => {
                        self.status_msg = Some((format!("Disconnect failed: {e}"), true));
                    }
                }
            }
            Message::ForgetNetwork(ssid) => {
                self.status_msg = Some((format!("Forgetting profile \"{}\"...", ssid), false));
                return Command::perform(
                    async move { ProfilesManager::forget_profile(&ssid).await },
                    Message::ForgetDone,
                );
            }
            Message::ForgetDone(res) => match res {
                Ok(_) => {
                    self.status_msg = Some(("Profile removed".into(), false));
                    self.connection_details = None;
                    self.is_scanning = true;
                    return Command::batch([
                        Command::perform(WifiManager::scan_networks(false), Message::ScanDone),
                        Command::perform(
                            WifiManager::get_connection_details(),
                            Message::DetailsLoaded,
                        ),
                    ]);
                }
                Err(e) => {
                    self.status_msg = Some((format!("Forget failed: {e}"), true));
                }
            },
            Message::LaunchTui => {
                return Command::perform(
                    async {
                        let _ = tokio::process::Command::new(
                            "/home/m/.config/hypr/scripts/ui/open_wifi_tui.sh",
                        )
                        .spawn();
                    },
                    |_| Message::Close,
                );
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

    fn filtered_networks(&self) -> Vec<&WifiNetwork> {
        let query = self.search_query.trim().to_lowercase();
        if query.is_empty() {
            self.networks.iter().collect()
        } else {
            self.networks
                .iter()
                .filter(|net| net.ssid.to_lowercase().contains(&query))
                .collect()
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let title = text("󰤨 Wi-Fi").size(16);

        let tui_btn = button(text("󰞷 TUI").size(11))
            .style(button::secondary)
            .padding([4, 8])
            .on_press(Message::LaunchTui);

        let rescan_label = if self.is_scanning {
            "󰑐 Scanning..."
        } else {
            "󰑐 Scan"
        };
        let rescan_btn = button(text(rescan_label).size(11))
            .style(button::secondary)
            .padding([4, 8])
            .on_press_maybe(if self.is_scanning || !self.is_enabled {
                None
            } else {
                Some(Message::Rescan)
            });

        let power_label = if self.is_enabled {
            "󰤨 ON"
        } else {
            "󰤮 OFF"
        };
        let power_style = if self.is_enabled {
            button::primary
        } else {
            button::secondary
        };
        let power_btn = button(text(power_label).size(11))
            .style(power_style)
            .padding([4, 8])
            .on_press(Message::ToggleRadio);

        let header = row![
            title,
            space().width(Length::Fill),
            tui_btn,
            rescan_btn,
            power_btn
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let mut content_col = column![header].spacing(10);

        // Status Banner
        if let Some((msg, _is_error)) = &self.status_msg {
            let banner_text = text(msg).size(11);
            let banner_container = container(banner_text)
                .width(Length::Fill)
                .padding([5, 10])
                .style(container::bordered_box);
            content_col = content_col.push(banner_container);
        }

        // Active Connection Details Card
        if let Some(details) = &self.connection_details
            && let Some(ref current_ssid) = details.ssid
        {
            let conn_title = row![
                text(format!("󰄬 Connected: {current_ssid}")).size(13),
                space().width(Length::Fill),
                button(text("Forget").size(10))
                    .style(button::danger)
                    .padding([3, 7])
                    .on_press(Message::ForgetNetwork(current_ssid.clone())),
                button(text("Disconnect").size(10))
                    .style(button::secondary)
                    .padding([3, 7])
                    .on_press(Message::Disconnect),
            ]
            .spacing(6)
            .align_y(Alignment::Center);

            let mut badge_row = row![].spacing(6).align_y(Alignment::Center);

            if let Some(freq) = details.frequency_mhz {
                let band = if freq >= 4900 { "5 GHz" } else { "2.4 GHz" };
                badge_row = badge_row.push(
                    container(text(format!("󰤨 {band}")).size(10))
                        .padding([2, 6])
                        .style(container::bordered_box),
                );
            }

            if let Some(ref ip) = details.ip_address {
                badge_row = badge_row.push(
                    container(text(format!("󰩟 {ip}")).size(10))
                        .padding([2, 6])
                        .style(container::bordered_box),
                );
            }

            if let Some(ref speed) = details.bitrate {
                badge_row = badge_row.push(
                    container(text(format!("󰓅 {speed}")).size(10))
                        .padding([2, 6])
                        .style(container::bordered_box),
                );
            }

            if let Some(dbm) = details.signal_dbm {
                badge_row = badge_row.push(
                    container(text(format!("{dbm} dBm")).size(10))
                        .padding([2, 6])
                        .style(container::bordered_box),
                );
            }

            let details_card = container(column![conn_title, badge_row].spacing(6))
                .width(Length::Fill)
                .padding(10)
                .style(container::bordered_box);

            content_col = content_col.push(details_card);
        }

        // Password Prompt Card
        if let Some(prompt_ssid) = &self.password_prompt_ssid {
            let prompt_title =
                text(format!("󰌾 Password for \"{}\"", prompt_ssid)).size(13);

            let eye_icon = if self.show_password { "󰈈" } else { "󰈉" };
            let eye_btn = button(text(eye_icon).size(13))
                .style(button::secondary)
                .padding([6, 8])
                .on_press(Message::ToggleShowPassword);

            let input = text_input("Password...", &self.password_input)
                .id(PASSWORD_INPUT_ID.clone())
                .secure(!self.show_password)
                .on_input(Message::PasswordChanged)
                .on_submit(Message::SubmitConnect)
                .padding(8)
                .size(13);

            let input_row = row![input, eye_btn].spacing(6).align_y(Alignment::Center);

            let connect_btn = button(text("Connect").size(11))
                .style(button::primary)
                .padding([5, 12])
                .on_press(Message::SubmitConnect);

            let cancel_btn = button(text("Cancel").size(11))
                .style(button::secondary)
                .padding([5, 12])
                .on_press(Message::CancelPassword);

            let action_row = row![connect_btn, cancel_btn]
                .spacing(8)
                .align_y(Alignment::Center);

            let prompt_card =
                container(column![prompt_title, input_row, action_row].spacing(8))
                    .width(Length::Fill)
                    .padding(10)
                    .style(container::bordered_box);

            content_col = content_col.push(prompt_card);
        }

        // Search Input Filter
        if self.is_enabled && !self.networks.is_empty() {
            let search_bar = text_input("󰍉 Search Wi-Fi networks...", &self.search_query)
                .on_input(Message::SearchChanged)
                .padding(6)
                .size(12);
            content_col = content_col.push(search_bar);
        }

        // Network List
        let mut list_col = column![].spacing(4);

        if !self.is_enabled {
            list_col = list_col.push(
                container(text("Wi-Fi is turned OFF. Click 'ON' to enable.").size(12))
                    .padding(24)
                    .align_x(Alignment::Center),
            );
        } else {
            let filtered = self.filtered_networks();
            if filtered.is_empty() {
                let empty_text = if self.is_scanning {
                    "Scanning for Wi-Fi networks..."
                } else if !self.search_query.is_empty() {
                    "No networks match your search"
                } else {
                    "No Wi-Fi networks found"
                };
                list_col = list_col.push(
                    container(text(empty_text).size(12))
                        .padding(24)
                        .align_x(Alignment::Center),
                );
            } else {
                for (idx, net) in filtered.iter().enumerate() {
                    let is_selected = idx == self.selected_idx;

                    let icon = text(net.signal_icon()).size(16);
                    let ssid = text(&net.ssid).size(13);

                    let sec_label = if net.is_secure {
                        format!("󰌾 {}", net.security)
                    } else {
                        "Open".to_string()
                    };
                    let meta = text(format!("{} • {}% {}", sec_label, net.signal, net.bars))
                        .size(10);
                    let text_stack = column![ssid, meta].spacing(1);

                    let mut row_content = row![icon, text_stack]
                        .spacing(10)
                        .align_y(Alignment::Center);

                    row_content = row_content.push(space().width(Length::Fill));

                    if net.is_connected {
                        let connected_badge = text("󰄬 Active").size(11);
                        row_content = row_content.push(connected_badge);
                    } else {
                        let connect_btn = button(text("Connect").size(10))
                            .style(button::primary)
                            .padding([3, 8])
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
                        .padding([5, 10])
                        .on_press(Message::SelectNetwork(idx));

                    list_col = list_col.push(item_card);
                }
            }
        }

        let filtered_count = self.filtered_networks().len();
        let footer = row![
            text(format!("{} networks", filtered_count)).size(10),
            space().width(Length::Fill),
            text("↑↓ Select • Enter Connect • Esc Close").size(10),
        ]
        .align_y(Alignment::Center)
        .padding([2, 4]);

        content_col = content_col.push(scrollable(list_col).height(Length::Fill));
        content_col = content_col.push(footer);

        container(content_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(14)
            .style(default_modal_card_style())
            .into()
    }
}
