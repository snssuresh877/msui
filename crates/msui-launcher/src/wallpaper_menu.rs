use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key, Modifiers};
use iced::widget::operation::focus;
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use msui_core::{ThemeManager, WallpaperItem, WallpaperManager};
use std::path::PathBuf;
use std::sync::LazyLock;

static INPUT_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);

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
}

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    SelectPrevious,
    SelectNext,
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
    fn new() -> (Self, Command<Message>) {
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
            },
            focus(INPUT_ID.clone()),
        )
    }

    fn namespace() -> String {
        String::from("msui-wallpaper-menu")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
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
            Message::ApplyWallpaper(path) => {
                WallpaperManager::set(&path);
                std::process::exit(0);
            }
            Message::RandomWallpaper => {
                if let Some(path) = WallpaperManager::random() {
                    self.current_path = Some(path.clone());
                    std::process::exit(0);
                }
            }
            Message::Close => {
                std::process::exit(0);
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
                    Key::Named(Named::Enter) => {
                        if let Some(w) = self.filtered.get(self.selected_idx) {
                            return Command::done(Message::ApplyWallpaper(w.path.clone()));
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
