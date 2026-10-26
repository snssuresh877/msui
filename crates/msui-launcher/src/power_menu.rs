use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::{button, column, container, row, space, text};
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
        size: LayerSize::px(520, 290),
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
                        "c" => return Command::done(Message::Close),
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
        let header = text("Power & Session")
            .size(16)
            .style(|_theme: &iced::Theme| text::Style {
                color: Some(iced::Color::from_rgba(0.92, 0.94, 0.98, 0.95)),
            });

        let lock_card = Self::power_card("󰌾", "Lock", "[L]", Message::Lock);
        let suspend_card = Self::power_card("󰤄", "Suspend", "[S]", Message::Suspend);
        let reboot_card = Self::power_card("󰑓", "Reboot", "[R]", Message::Reboot);
        let shutdown_card = Self::power_card("󰐥", "Shutdown", "[P]", Message::Poweroff);
        let logout_card = Self::power_card("󰗽", "Log Out", "[O]", Message::Logout);
        let cancel_card = Self::power_card("󰅖", "Cancel", "[Esc]", Message::Close);

        let row_1 = row![lock_card, suspend_card, reboot_card]
            .spacing(12)
            .width(Length::Fill);

        let row_2 = row![shutdown_card, logout_card, cancel_card]
            .spacing(12)
            .width(Length::Fill);

        let grid = column![row_1, row_2]
            .spacing(12)
            .width(Length::Fill);

        let content = column![header, space().height(4), grid]
            .spacing(12)
            .align_x(Alignment::Center)
            .padding(20);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(default_modal_card_style())
            .into()
    }

    fn power_card<'a>(
        icon: &'static str,
        label: &'static str,
        shortcut: &'static str,
        msg: Message,
    ) -> Element<'a, Message> {
        let icon_text = text(icon).size(26);
        let label_text = text(label).size(14);
        let shortcut_text = text(shortcut)
            .size(11)
            .style(|_theme: &iced::Theme| text::Style {
                color: Some(iced::Color::from_rgba(0.80, 0.84, 0.95, 0.60)),
            });

        let col = column![icon_text, label_text, shortcut_text]
            .spacing(3)
            .align_x(Alignment::Center);

        button(col)
            .width(Length::FillPortion(1))
            .height(Length::Fixed(96.0))
            .style(power_card_style)
            .padding([12, 8])
            .on_press(msg)
            .into()
    }
}

fn power_card_style(theme: &iced::Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let p = theme.palette();
    let is_frosted = crate::modal_style::is_frosted();

    let (bg, border_color) = match status {
        iced::widget::button::Status::Hovered => {
            if is_frosted {
                (
                    iced::Color::from_rgba(65.0 / 255.0, 75.0 / 255.0, 115.0 / 255.0, 0.88),
                    iced::Color::from_rgba(1.0, 1.0, 1.0, 0.25),
                )
            } else {
                (
                    iced::Color::from_rgba(p.primary.r, p.primary.g, p.primary.b, 0.22),
                    iced::Color::from_rgba(p.primary.r, p.primary.g, p.primary.b, 0.45),
                )
            }
        }
        iced::widget::button::Status::Pressed => {
            if is_frosted {
                (
                    iced::Color::from_rgba(38.0 / 255.0, 44.0 / 255.0, 68.0 / 255.0, 0.95),
                    iced::Color::from_rgba(1.0, 1.0, 1.0, 0.16),
                )
            } else {
                (
                    iced::Color::from_rgba(p.primary.r, p.primary.g, p.primary.b, 0.32),
                    iced::Color::from_rgba(p.primary.r, p.primary.g, p.primary.b, 0.55),
                )
            }
        }
        _ => {
            if is_frosted {
                (
                    iced::Color::from_rgba(45.0 / 255.0, 52.0 / 255.0, 78.0 / 255.0, 0.68),
                    iced::Color::from_rgba(1.0, 1.0, 1.0, 0.10),
                )
            } else {
                (
                    iced::Color::from_rgba(p.text.r, p.text.g, p.text.b, 0.08),
                    iced::Color::from_rgba(p.text.r, p.text.g, p.text.b, 0.12),
                )
            }
        }
    };

    let mut border = iced::border::rounded(14);
    border.width = 1.0;
    border.color = border_color;

    iced::widget::button::Style {
        background: Some(bg.into()),
        text_color: iced::Color::WHITE,
        border,
        ..Default::default()
    }
}
