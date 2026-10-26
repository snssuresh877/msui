use iced::widget::{container, row, slider, space, text};
use iced::{Alignment, Element, Font, Length, Task as Command};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize, Margin};
use iced_exwlshell::settings::LayerShellSettings;
use crate::modal_style::default_modal_card_style;
use msui_core::{OsdKind, OsdManager, OsdState, ThemeManager};
use std::time::Duration;

pub fn run_osd_window() -> Result<(), iced_exwlshell::Error> {
    let width = 280;
    let height = 64;

    application(
        OsdWindow::new,
        OsdWindow::namespace,
        OsdWindow::update,
        OsdWindow::view,
    )
    .theme(ThemeManager::current_theme())
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(width, height),
        anchor: Anchor::Bottom,
        margin: Margin {
            top: 0,
            right: 0,
            bottom: 80, // Floating 80px above screen bottom
            left: 0,
        },
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::None,
        ..Default::default()
    })
    .subscription(OsdWindow::subscription)
    .run()
}

pub struct OsdWindow {
    state: Option<OsdState>,
    is_standalone: bool,
    last_update: Option<std::time::Instant>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Ignored,
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

impl OsdWindow {
    pub fn new() -> (Self, Command<Message>) {
        OsdManager::record_osd_pid(std::process::id());
        let state = OsdManager::read_state();

        (
            Self {
                state,
                is_standalone: true,
                last_update: Some(std::time::Instant::now()),
            },
            Command::none(),
        )
    }

    pub fn new_embedded() -> (Self, Command<Message>) {
        (
            Self {
                state: None,
                is_standalone: false,
                last_update: None,
            },
            Command::none(),
        )
    }

    pub fn show(&mut self, kind: OsdKind, percent: u8, is_muted: bool) {
        self.state = Some(OsdState {
            kind,
            percent,
            is_muted,
            timestamp_ms: OsdManager::current_time_ms(),
        });
        self.last_update = Some(std::time::Instant::now());
    }

    pub fn is_active(&self) -> bool {
        if let Some(last) = self.last_update {
            last.elapsed() < Duration::from_millis(1500)
        } else {
            false
        }
    }

    pub fn namespace() -> String {
        "msui-osd-window".to_string()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        if self.is_standalone || self.is_active() {
            iced::time::every(Duration::from_millis(50)).map(|_| Message::Tick)
        } else {
            iced::Subscription::none()
        }
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Tick => {
                if self.is_standalone {
                    if let Some(state) = OsdManager::read_state() {
                        let now = OsdManager::current_time_ms();
                        let elapsed = now.saturating_sub(state.timestamp_ms);
                        if elapsed > 1500 {
                            // 1.5 seconds without update: dismiss
                            OsdManager::remove_osd_pid();
                            std::process::exit(0);
                        }
                        self.state = Some(state);
                    } else {
                        OsdManager::remove_osd_pid();
                        std::process::exit(0);
                    }
                } else {
                    if !self.is_active() {
                        self.state = None;
                        self.last_update = None;
                    }
                }
            }
            Message::Ignored => {}
        }
        Command::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = if let Some(state) = &self.state {
            let (icon, label) = match state.kind {
                OsdKind::Volume => {
                    let ic = if state.is_muted {
                        "󰝟"
                    } else if state.percent > 66 {
                        "󰕾"
                    } else if state.percent > 33 {
                        ""
                    } else {
                        ""
                    };
                    let lbl = if state.is_muted {
                        "Muted".to_string()
                    } else {
                        format!("{}%", state.percent)
                    };
                    (ic, lbl)
                }
                OsdKind::Brightness => {
                    let ic = if state.percent > 66 {
                        "󰃠"
                    } else if state.percent > 33 {
                        "󰃟"
                    } else {
                        "󰃞"
                    };
                    (ic, format!("{}%", state.percent))
                }
            };

            let icon_text = text(icon).size(18);
            let bar = slider(0..=100, state.percent, |_| Message::Ignored).width(Length::Fill);
            let label_text = text(label).size(12).width(Length::Fixed(42.0));

            row![icon_text, bar, label_text]
                .spacing(10)
                .align_y(Alignment::Center)
                .into()
        } else {
            space().into()
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([8, 16])
            .center_y(Length::Fill)
            .style(default_modal_card_style())
            .into()
    }
}
