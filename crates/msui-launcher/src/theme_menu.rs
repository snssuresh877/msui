use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::operation::focus;
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use msui_core::{ThemeDefinition, ThemeManager, BUILTIN_THEMES};
use std::sync::LazyLock;

static INPUT_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);

pub fn run_theme_menu() -> Result<(), iced_exwlshell::Error> {
    application(
        ThemeMenu::new,
        ThemeMenu::namespace,
        ThemeMenu::update,
        ThemeMenu::view,
    )
    .theme(ThemeManager::current_theme())
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(680, 520),
        anchor: Anchor::empty(), // Centered on screen
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    })
    .subscription(ThemeMenu::subscription)
    .run()
}

pub struct ThemeMenu {
    query: String,
    all_themes: Vec<ThemeDefinition>,
    filtered: Vec<ThemeDefinition>,
    selected_idx: usize,
    current_theme_name: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    SelectPrevious,
    SelectNext,
    ApplyTheme(String),
    Close,
    IcedEvent(Event),
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

impl ThemeMenu {
    fn new() -> (Self, Command<Message>) {
        let all_themes: Vec<ThemeDefinition> = BUILTIN_THEMES.to_vec();
        let current_theme_name = ThemeManager::current_theme_name();
        let filtered = all_themes.clone();

        (
            Self {
                query: String::new(),
                all_themes,
                filtered,
                selected_idx: 0,
                current_theme_name,
            },
            focus(INPUT_ID.clone()),
        )
    }

    fn namespace() -> String {
        String::from("msui-theme-menu")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        event::listen().map(Message::IcedEvent)
    }

    fn update_filtered(&mut self) {
        if self.query.trim().is_empty() {
            self.filtered = self.all_themes.clone();
            self.selected_idx = 0;
            return;
        }

        let q = self.query.to_lowercase();
        self.filtered = self
            .all_themes
            .iter()
            .filter(|t| t.name.to_lowercase().contains(&q) || t.description.to_lowercase().contains(&q))
            .cloned()
            .collect();
        self.selected_idx = 0;
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::QueryChanged(new_query) => {
                self.query = new_query;
                self.update_filtered();
            }
            Message::SelectNext => {
                if !self.filtered.is_empty() {
                    self.selected_idx = (self.selected_idx + 1) % self.filtered.len();
                }
            }
            Message::SelectPrevious => {
                if !self.filtered.is_empty() {
                    if self.selected_idx == 0 {
                        self.selected_idx = self.filtered.len() - 1;
                    } else {
                        self.selected_idx -= 1;
                    }
                }
            }
            Message::ApplyTheme(name) => {
                ThemeManager::set_theme(&name);
                std::process::exit(0);
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
                        if let Some(t) = self.filtered.get(self.selected_idx) {
                            return Command::done(Message::ApplyTheme(t.name.to_string()));
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
        let input = text_input("󰍉 Search themes...", &self.query)
            .id(INPUT_ID.clone())
            .on_input(Message::QueryChanged)
            .padding(12)
            .size(16);

        let mut list_col = column![].spacing(6);

        if self.filtered.is_empty() {
            list_col = list_col.push(
                container(text("No matching themes found.").size(13))
                    .padding(32)
                    .align_x(Alignment::Center),
            );
        } else {
            for (idx, theme_def) in self.filtered.iter().enumerate() {
                let is_selected = idx == self.selected_idx;
                let is_active = self.current_theme_name.eq_ignore_ascii_case(theme_def.name)
                    || self.current_theme_name.eq_ignore_ascii_case(theme_def.id);

                let icon = text("󰔎").size(18);
                let name = text(theme_def.name).size(14);
                let desc = text(theme_def.description).size(11);
                let text_stack = column![name, desc].spacing(2);

                let palette_preview = text(format!("{} • {}", theme_def.accent_hex, theme_def.bg_hex)).size(11);

                let mut row_content = row![icon, text_stack, space().width(Length::Fill), palette_preview]
                    .spacing(12)
                    .align_y(Alignment::Center);

                if is_active {
                    row_content = row_content.push(text(" 󰄬 Active ").size(12));
                }

                let btn_style = if is_selected {
                    button::primary
                } else {
                    button::secondary
                };

                let item_btn = button(row_content)
                    .width(Length::Fill)
                    .style(btn_style)
                    .padding([8, 12])
                    .on_press(Message::ApplyTheme(theme_def.name.to_string()));

                list_col = list_col.push(item_btn);
            }
        }

        let footer = row![
            text(format!("{} themes available", self.filtered.len())).size(11),
            space().width(Length::Fill),
            text("↑↓ Select • Enter Apply • Esc Close").size(11),
        ]
        .align_y(Alignment::Center)
        .padding([4, 6]);

        let content = column![
            input,
            scrollable(list_col).height(Length::Fill),
            footer,
        ]
        .spacing(10)
        .padding(14);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(container::bordered_box)
            .into()
    }
}
