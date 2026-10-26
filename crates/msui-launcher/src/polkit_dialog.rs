use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::operation::focus;
use iced::widget::{button, column, container, row, space, text, text_input};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use msui_core::polkit::verify_polkit_credentials;
use msui_core::ThemeManager;
use std::sync::LazyLock;

static PASSWORD_INPUT_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);

#[derive(Debug, Clone)]
pub struct PolkitDialogArgs {
    pub action_id: String,
    pub message: String,
    pub icon_name: String,
    pub user: String,
    pub cookie: String,
    pub command: String,
}

pub fn run_polkit_dialog(args: PolkitDialogArgs) -> Result<(), iced_exwlshell::Error> {
    application(
        move || PolkitDialog::new(args.clone()),
        PolkitDialog::namespace,
        PolkitDialog::update,
        PolkitDialog::view,
    )
    .theme(ThemeManager::current_theme())
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(480, 340),
        anchor: Anchor::empty(), // Centered on screen
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    })
    .subscription(PolkitDialog::subscription)
    .run()
}

pub struct PolkitDialog {
    args: PolkitDialogArgs,
    password: String,
    error_message: Option<String>,
    is_authenticating: bool,
    is_standalone: bool,
    should_close: bool,
    auth_success: Option<bool>,
}

#[derive(Debug, Clone)]
pub enum Message {
    PasswordChanged(String),
    Submit,
    AuthResult(bool),
    Cancel,
    IcedEvent(Event),
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

impl PolkitDialog {
    pub fn new(args: PolkitDialogArgs) -> (Self, Command<Message>) {
        (
            Self {
                args,
                password: String::new(),
                error_message: None,
                is_authenticating: false,
                is_standalone: true,
                should_close: false,
                auth_success: None,
            },
            focus(PASSWORD_INPUT_ID.clone()),
        )
    }

    pub fn new_embedded(args: PolkitDialogArgs) -> (Self, Command<Message>) {
        (
            Self {
                args,
                password: String::new(),
                error_message: None,
                is_authenticating: false,
                is_standalone: false,
                should_close: false,
                auth_success: None,
            },
            focus(PASSWORD_INPUT_ID.clone()),
        )
    }

    pub fn should_close(&self) -> bool {
        self.should_close
    }

    pub fn reset_should_close(&mut self) {
        self.should_close = false;
    }

    pub fn auth_success(&self) -> Option<bool> {
        self.auth_success
    }

    pub fn namespace() -> String {
        String::from("msui-polkit")
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        event::listen().map(Message::IcedEvent)
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::PasswordChanged(val) => {
                self.password = val;
                self.error_message = None;
                Command::none()
            }
            Message::Submit => {
                if self.is_authenticating {
                    return Command::none();
                }
                self.is_authenticating = true;
                self.error_message = None;

                let user = self.args.user.clone();
                let cookie = self.args.cookie.clone();
                let password = self.password.clone();

                Command::perform(
                    async move {
                        verify_polkit_credentials(&user, &cookie, &password)
                            .await
                            .unwrap_or(false)
                    },
                    Message::AuthResult,
                )
            }
            Message::AuthResult(success) => {
                self.is_authenticating = false;
                if success {
                    if self.is_standalone {
                        std::process::exit(0);
                    } else {
                        self.auth_success = Some(true);
                        self.should_close = true;
                        Command::none()
                    }
                } else {
                    self.error_message = Some("Authentication failed. Please try again.".to_string());
                    self.password.clear();
                    focus(PASSWORD_INPUT_ID.clone())
                }
            }
            Message::Cancel => {
                if self.is_standalone {
                    std::process::exit(1);
                } else {
                    self.auth_success = Some(false);
                    self.should_close = true;
                    Command::none()
                }
            }
            Message::IcedEvent(Event::Keyboard(KeyboardEvent::KeyPressed { key, .. })) => {
                match key {
                    Key::Named(Named::Escape) => {
                        if self.is_standalone {
                            std::process::exit(1);
                        } else {
                            self.auth_success = Some(false);
                            self.should_close = true;
                            Command::none()
                        }
                    }
                    Key::Named(Named::Enter) => {
                        return self.update(Message::Submit);
                    }
                    _ => Command::none(),
                }
            }
            Message::IcedEvent(_) => Command::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        // Shield header with action title
        let shield_icon = text("󰌾").size(32);
        let title_col = column![
            text("Authentication Required").size(16),
            text(&self.args.action_id).size(11),
        ]
        .spacing(2);

        let header = row![shield_icon, title_col]
            .spacing(14)
            .align_y(Alignment::Center);

        // Message text
        let msg_clean = if self.args.message.is_empty() {
            "An application is requesting administrative privileges.".to_string()
        } else {
            self.args.message.clone()
        };
        let msg_text = text(msg_clean).size(13);

        // Command detail if present
        let mut details_col = column![].spacing(6);
        if !self.args.command.is_empty() {
            let cmd_box = container(
                row![
                    text("Command:").size(12),
                    text(&self.args.command).size(12),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([6, 10])
            .width(Length::Fill);

            details_col = details_col.push(cmd_box);
        }

        // Target user badge
        let user_box = container(
            row![
                text("󰀉").size(14),
                text(format!("Authenticating as: {}", self.args.user)).size(12),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([4, 8]);
        details_col = details_col.push(user_box);

        // Password input field
        let input_box = text_input("Enter password...", &self.password)
            .id(PASSWORD_INPUT_ID.clone())
            .secure(true)
            .on_input(Message::PasswordChanged)
            .on_submit(Message::Submit)
            .padding(10)
            .size(14);

        // Action buttons
        let cancel_btn = button(text("Cancel (Esc)").size(13))
            .padding([8, 16])
            .on_press(Message::Cancel);

        let auth_label = if self.is_authenticating {
            "Verifying..."
        } else {
            "Authenticate (Enter)"
        };
        let auth_btn = button(text(auth_label).size(13))
            .padding([8, 16])
            .on_press(Message::Submit);

        let button_row = row![
            cancel_btn,
            space().width(Length::Fill),
            auth_btn,
        ]
        .align_y(Alignment::Center);

        let mut content = column![
            header,
            msg_text,
            details_col,
        ]
        .spacing(12);

        // Optional error banner
        if let Some(err) = &self.error_message {
            let err_banner = container(
                row![
                    text("󰅚").size(14),
                    text(err).size(12),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([6, 10])
            .width(Length::Fill);

            content = content.push(err_banner);
        }

        content = content.push(input_box);
        content = content.push(space().height(6));
        content = content.push(button_row);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .style(container::bordered_box)
            .into()
    }
}
