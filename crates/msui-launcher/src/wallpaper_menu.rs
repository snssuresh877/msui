use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key, Modifiers};
use iced::widget::operation::{focus, snap_to, RelativeOffset};
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use crate::modal_style::default_modal_card_style;
use msui_core::{ThemeManager, WallpaperItem, WallpaperManager};
use std::path::PathBuf;
use std::sync::LazyLock;

static INPUT_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);
static SCROLL_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);

pub fn run_wallpaper_menu() -> Result<(), iced_exwlshell::Error> {
    application(
        WallpaperMenu::new,
        WallpaperMenu::namespace,
        WallpaperMenu::update,
        WallpaperMenu::view,
    )
    .theme(ThemeManager::current_theme())
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(720, 540),
        anchor: Anchor::empty(), // Centered on screen
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    })
    .subscription(WallpaperMenu::subscription)
    .run()
}

pub struct WallpaperMenu {
    query: String,
    all_wallpapers: Vec<WallpaperItem>,
    filtered: Vec<WallpaperItem>,
    selected_idx: usize,
    current_path: Option<PathBuf>,
    is_standalone: bool,
    should_close: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    SelectPrevious,
    SelectNext,
    EnsureFocus,
    ApplyWallpaper(PathBuf),
    RandomWallpaper,
    Close,
    IcedEvent(Event),
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

impl WallpaperMenu {
    pub fn new() -> (Self, Command<Message>) {
        Self::new_internal(true)
    }

    pub fn new_embedded() -> (Self, Command<Message>) {
        Self::new_internal(false)
    }

    fn new_internal(is_standalone: bool) -> (Self, Command<Message>) {
        let all_wallpapers = WallpaperManager::list();
        let current_path = WallpaperManager::get_current();
        let filtered = all_wallpapers.clone();

        (
            Self {
                query: String::new(),
                all_wallpapers,
                filtered,
                selected_idx: 0,
                current_path,
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
        self.all_wallpapers = WallpaperManager::list();
        self.current_path = WallpaperManager::get_current();
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
        String::from("msui-wallpaper-menu")
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        event::listen().map(Message::IcedEvent)
    }

    fn update_filtered(&mut self) {
        if self.query.trim().is_empty() {
            self.filtered = self.all_wallpapers.clone();
            self.selected_idx = 0;
            return;
        }

        let q = self.query.to_lowercase();
        self.filtered = self
            .all_wallpapers
            .iter()
            .filter(|w| w.name.to_lowercase().contains(&q))
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
            Message::ApplyWallpaper(path) => {
                WallpaperManager::set(&path);
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::RandomWallpaper => {
                if let Some(path) = WallpaperManager::random() {
                    self.current_path = Some(path.clone());
                    if self.is_standalone {
                        std::process::exit(0);
                    } else {
                        self.should_close = true;
                    }
                }
            }
            Message::Close => {
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::IcedEvent(Event::Keyboard(KeyboardEvent::KeyPressed {
                key, modifiers, ..
            })) => {
                if modifiers.contains(Modifiers::CTRL) {
                    if let Key::Character(c) = &key {
                        if c.eq_ignore_ascii_case("r") {
                            return Command::done(Message::RandomWallpaper);
                        }
                    }
                }

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
                    Key::Named(Named::Tab) => {
                        if modifiers.shift() {
                            return Command::done(Message::SelectPrevious);
                        } else {
                            return Command::done(Message::SelectNext);
                        }
                    }
                    Key::Named(Named::Enter) => {
                        if let Some(w) = self.filtered.get(self.selected_idx) {
                            return Command::done(Message::ApplyWallpaper(w.path.clone()));
                        }
                    }
                    _ => {
                        // Any keypress keeps search focused
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
        let input = text_input("󰍉 Search wallpapers...", &self.query)
            .id(INPUT_ID.clone())
            .on_input(Message::QueryChanged)
            .padding(12)
            .size(16);

        let random_btn = button(text("󰑐 Random (Ctrl+R)").size(12))
            .style(button::primary)
            .padding([6, 12])
            .on_press(Message::RandomWallpaper);

        let search_row = row![input, random_btn]
            .spacing(10)
            .align_y(Alignment::Center);

        let mut list_col = column![].spacing(6);

        if self.filtered.is_empty() {
            let empty_text = if self.all_wallpapers.is_empty() {
                "No wallpapers found in ~/Pictures/Wallpapers."
            } else {
                "No matching wallpapers found."
            };
            list_col = list_col.push(
                container(text(empty_text).size(13))
                    .padding(32)
                    .align_x(Alignment::Center),
            );
        } else {
            for (idx, w) in self.filtered.iter().enumerate() {
                let is_selected = idx == self.selected_idx;
                let is_active = self.current_path.as_ref() == Some(&w.path);

                let icon = text("󰸉").size(18);
                let name = text(&w.name).size(14);
                let meta = text(format!("{:.1} MB", w.size_mb)).size(11);
                let text_stack = column![name, meta].spacing(2);

                let mut row_content = row![icon, text_stack]
                    .spacing(12)
                    .align_y(Alignment::Center);

                row_content = row_content.push(space().width(Length::Fill));

                if is_active {
                    row_content = row_content.push(text("󰄬 Active").size(12));
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
                    .on_press(Message::ApplyWallpaper(w.path.clone()));

                list_col = list_col.push(item_btn);
            }
        }

        let footer = row![
            text(format!("{} wallpapers", self.filtered.len())).size(11),
            space().width(Length::Fill),
            text("↑↓ Select • Enter Apply • Ctrl+R Random • Esc Close").size(11),
        ]
        .align_y(Alignment::Center)
        .padding([4, 6]);

        let content = column![
            search_row,
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
