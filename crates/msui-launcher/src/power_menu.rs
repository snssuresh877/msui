use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Event, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use crate::modal_style::default_modal_card_style;
use msui_core::SystemControl;

pub fn run_power_menu() -> Result<(), iced_exwlshell::Error> {
    application(
        PowerMenu::new,
        PowerMenu::namespace,
        PowerMenu::update,
        PowerMenu::view,
    )
    .theme(iced::Theme::CatppuccinMocha)
    .default_font(iced::Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..iced::Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(500, 260),
        anchor: Anchor::empty(), // Centered on screen
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    })
    .subscription(PowerMenu::subscription)
    .run()
}

pub struct PowerMenu {
    is_standalone: bool,
    should_close: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Lock,
    Suspend,
    Reboot,
    Poweroff,
    Logout,
    Close,
    IcedEvent(Event),
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

impl PowerMenu {
    pub fn new() -> (Self, Command<Message>) {
        (
            Self {
                is_standalone: true,
                should_close: false,
            },
            Command::none(),
        )
    }

    pub fn new_embedded() -> (Self, Command<Message>) {
        (
            Self {
                is_standalone: false,
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

    pub fn reset(&mut self) {
        self.should_close = false;
    }

    pub fn namespace() -> String {
        String::from("msui-power-menu")
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        event::listen().map(Message::IcedEvent)
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Lock => {
                SystemControl::lock();
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::Suspend => {
                SystemControl::suspend();
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::Reboot => {
                SystemControl::reboot();
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::Poweroff => {
                SystemControl::poweroff();
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::Logout => {
                SystemControl::logout();
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
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
                        return Command::done(Message::Close);
                    }
                    Key::Character(c) => match c.to_lowercase().as_str() {
                        "l" => return Command::done(Message::Lock),
                        "s" => return Command::done(Message::Suspend),
                        "r" => return Command::done(Message::Reboot),
                        "p" => return Command::done(Message::Poweroff),
                        "o" => return Command::done(Message::Logout),
                        _ => {}
                    },
                    _ => {}
                }
            }
            _ => {}
        }
        Command::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let header = text("Power & Session").size(20);

        let lock_btn = button(column![text("🔒 Lock").size(16), text("[L]").size(11)])
            .padding(12)
            .style(button::secondary)
            .on_press(Message::Lock);

        let sleep_btn = button(column![text("💤 Sleep").size(16), text("[S]").size(11)])
            .padding(12)
            .style(button::secondary)
            .on_press(Message::Suspend);

        let reboot_btn = button(column![text("🔄 Reboot").size(16), text("[R]").size(11)])
            .padding(12)
            .style(button::secondary)
            .on_press(Message::Reboot);

        let poweroff_btn = button(column![text("⏻ Power Off").size(16), text("[P]").size(11)])
            .padding(12)
            .style(button::danger)
            .on_press(Message::Poweroff);

        let logout_btn = button(column![text("🚪 Log Out").size(16), text("[O]").size(11)])
            .padding(12)
            .style(button::secondary)
            .on_press(Message::Logout);

        let actions_row = row![lock_btn, sleep_btn, reboot_btn, logout_btn, poweroff_btn]
            .spacing(8)
            .align_y(Alignment::Center);

        let cancel_btn = button(text("Cancel (Esc)"))
            .style(button::text)
            .on_press(Message::Close);

        let content = column![header, actions_row, cancel_btn]
            .spacing(16)
            .align_x(Alignment::Center)
            .padding(18);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(default_modal_card_style())
            .into()
    }
}
