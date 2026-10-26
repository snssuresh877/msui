use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::operation::{focus, snap_to, RelativeOffset};
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use crate::modal_style::default_modal_card_style;
use msui_core::{ThemeDefinition, ThemeManager, BUILTIN_THEMES};
use std::sync::LazyLock;

static INPUT_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);
static SCROLL_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);

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
    is_standalone: bool,
    should_close: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    SelectPrevious,
    SelectNext,
    EnsureFocus,
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
    pub fn new() -> (Self, Command<Message>) {
        Self::new_internal(true)
    }

    pub fn new_embedded() -> (Self, Command<Message>) {
        Self::new_internal(false)
    }

    fn new_internal(is_standalone: bool) -> (Self, Command<Message>) {
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
                is_standalone,
                should_close: false,
            },
            Command::batch([
                focus(INPUT_ID.clone()),
                Command::perform(tokio::time::sleep(std::time::Duration::from_millis(25)), |_| Message::EnsureFocus),
                Command::perform(tokio::time::sleep(std::time::Duration::from_millis(75)), |_| Message::EnsureFocus),
                Command::perform(tokio::time::sleep(std::time::Duration::from_millis(150)), |_| Message::EnsureFocus),
            ]),
        )
    }

    pub fn should_close(&self) -> bool {
        self.should_close
    }

    pub fn reset_should_close(&mut self) {
        self.should_close = false;
    }

    pub fn reset(&mut self) -> Command<Message> {
        self.query.clear();
        self.all_themes = BUILTIN_THEMES.to_vec();
        self.current_theme_name = ThemeManager::current_theme_name();
        self.update_filtered();
        self.should_close = false;
        Command::batch([
            focus(INPUT_ID.clone()),
            Command::perform(tokio::time::sleep(std::time::Duration::from_millis(25)), |_| Message::EnsureFocus),
            Command::perform(tokio::time::sleep(std::time::Duration::from_millis(75)), |_| Message::EnsureFocus),
            Command::perform(tokio::time::sleep(std::time::Duration::from_millis(150)), |_| Message::EnsureFocus),
        ])
    }

    pub fn namespace() -> String {
        String::from("msui-theme-menu")
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
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

    fn scroll_to_selected(&self) -> Command<Message> {
        if self.filtered.len() > 1 {
            let ratio = self.selected_idx as f32 / (self.filtered.len() - 1) as f32;
            snap_to(SCROLL_ID.clone(), RelativeOffset { x: 0.0, y: ratio })
        } else {
            snap_to(SCROLL_ID.clone(), RelativeOffset { x: 0.0, y: 0.0 })
        }
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::EnsureFocus => {
                return focus(INPUT_ID.clone());
            }
            Message::QueryChanged(new_query) => {
                self.query = new_query;
                self.update_filtered();
                return self.scroll_to_selected();
            }
            Message::SelectNext => {
                if !self.filtered.is_empty() {
                    self.selected_idx = (self.selected_idx + 1) % self.filtered.len();
                    return self.scroll_to_selected();
                }
            }
            Message::SelectPrevious => {
                if !self.filtered.is_empty() {
                    if self.selected_idx == 0 {
                        self.selected_idx = self.filtered.len() - 1;
                    } else {
                        self.selected_idx -= 1;
                    }
                    return self.scroll_to_selected();
                }
            }
            Message::ApplyTheme(name) => {
                ThemeManager::set_theme(&name);
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
                    Key::Named(Named::ArrowDown) => {
                        return Command::done(Message::SelectNext);
                    }
                    Key::Named(Named::ArrowUp) => {
                        return Command::done(Message::SelectPrevious);
                    }
                    Key::Named(Named::PageDown) => {
                        if !self.filtered.is_empty() {
                            self.selected_idx = (self.selected_idx + 5).min(self.filtered.len() - 1);
                            return self.scroll_to_selected();
                        }
                    }
                    Key::Named(Named::PageUp) => {
                        if !self.filtered.is_empty() {
                            self.selected_idx = self.selected_idx.saturating_sub(5);
                            return self.scroll_to_selected();
                        }
                    }
                    Key::Named(Named::Home) => {
                        if !self.filtered.is_empty() {
                            self.selected_idx = 0;
                            return self.scroll_to_selected();
                        }
                    }
                    Key::Named(Named::End) => {
                        if !self.filtered.is_empty() {
                            self.selected_idx = self.filtered.len() - 1;
                            return self.scroll_to_selected();
                        }
                    }
                    Key::Named(Named::Enter) => {
                        if let Some(t) = self.filtered.get(self.selected_idx) {
                            return Command::done(Message::ApplyTheme(t.name.to_string()));
                        }
                    }
                    _ => {
                        return focus(INPUT_ID.clone());
                    }
                }
            }
            Message::IcedEvent(Event::Window(_)) => {
                return focus(INPUT_ID.clone());
            }
            _ => {}
        }
        Command::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
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
            scrollable(list_col).id(SCROLL_ID.clone()).height(Length::Fill),
            footer,
        ]
        .spacing(10)
        .padding(14);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(default_modal_card_style())
            .into()
    }
}
