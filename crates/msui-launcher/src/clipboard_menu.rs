use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::operation::{focus, snap_to, RelativeOffset};
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, Theme, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use crate::modal_style::default_modal_card_style;
use msui_core::{ClipboardEntry, ClipboardStore, GLOBAL_CLIPBOARD};
use std::sync::LazyLock;

static INPUT_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);
static SCROLL_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);

pub fn run_clipboard_menu() -> Result<(), iced_exwlshell::Error> {
    application(
        ClipboardMenu::new,
        ClipboardMenu::namespace,
        ClipboardMenu::update,
        ClipboardMenu::view,
    )
    .theme(Theme::CatppuccinMocha)
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(680, 480),
        anchor: Anchor::empty(), // Centered on screen
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    })
    .subscription(ClipboardMenu::subscription)
    .run()
}

pub struct ClipboardMenu {
    query: String,
    all_entries: Vec<ClipboardEntry>,
    filtered: Vec<ClipboardEntry>,
    selected_idx: usize,
    is_standalone: bool,
    should_close: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    SelectPrevious,
    SelectNext,
    EnsureFocus,
    CopyAndExit(usize),
    DeleteEntry(usize),
    ClearHistory,
    Close,
    IcedEvent(Event),
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

impl ClipboardMenu {
    pub fn new() -> (Self, Command<Message>) {
        Self::new_internal(true)
    }

    pub fn new_embedded() -> (Self, Command<Message>) {
        Self::new_internal(false)
    }

    fn new_internal(is_standalone: bool) -> (Self, Command<Message>) {
        let all_entries = {
            let store = GLOBAL_CLIPBOARD.lock().unwrap();
            store.entries()
        };
        let filtered = all_entries.clone();

        (
            Self {
                query: String::new(),
                all_entries,
                filtered,
                selected_idx: 0,
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
        self.all_entries = {
            let store = GLOBAL_CLIPBOARD.lock().unwrap();
            store.entries()
        };
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
        String::from("msui-clipboard-menu")
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        event::listen().map(Message::IcedEvent)
    }

    fn update_filtered(&mut self) {
        if self.query.trim().is_empty() {
            self.filtered = self.all_entries.clone();
            self.selected_idx = 0;
            return;
        }

        let q = self.query.to_lowercase();
        self.filtered = self
            .all_entries
            .iter()
            .filter(|e| e.text.to_lowercase().contains(&q))
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
            Message::CopyAndExit(idx) => {
                if let Some(entry) = self.filtered.get(idx) {
                    ClipboardStore::copy_to_system(&entry.text);
                    if self.is_standalone {
                        std::process::exit(0);
                    } else {
                        self.should_close = true;
                    }
                }
            }
            Message::DeleteEntry(id) => {
                {
                    let mut store = GLOBAL_CLIPBOARD.lock().unwrap();
                    store.delete(id);
                    self.all_entries = store.entries();
                }
                self.update_filtered();
            }
            Message::ClearHistory => {
                {
                    let mut store = GLOBAL_CLIPBOARD.lock().unwrap();
                    store.clear();
                    self.all_entries.clear();
                }
                self.update_filtered();
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
                        if !self.filtered.is_empty() {
                            return Command::done(Message::CopyAndExit(self.selected_idx));
                        }
                    }
                    Key::Named(Named::Delete) => {
                        if let Some(entry) = self.filtered.get(self.selected_idx) {
                            return Command::done(Message::DeleteEntry(entry.id));
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
        let input = text_input("󰍉 Search clipboard history...", &self.query)
            .id(INPUT_ID.clone())
            .on_input(Message::QueryChanged)
            .padding(12)
            .size(16);

        let mut list_col = column![].spacing(6);

        if self.filtered.is_empty() {
            let empty_text = if self.all_entries.is_empty() {
                "Clipboard history is empty. Copy any text with Ctrl+C to start tracking."
            } else {
                "No matching clips found."
            };
            list_col = list_col.push(
                container(text(empty_text).size(13))
                    .padding(24)
                    .align_x(Alignment::Center),
            );
        } else {
            for (idx, entry) in self.filtered.iter().enumerate() {
                let is_selected = idx == self.selected_idx;

                let icon = text("󰅍").size(18);
                let preview_text = text(&entry.preview).size(14);
                let meta_str = format!("{} • {} chars", &entry.timestamp, entry.char_count);
                let meta_text = text(meta_str).size(11);

                let text_stack = column![preview_text, meta_text].spacing(2);

                let mut row_content = row![icon, text_stack]
                    .spacing(12)
                    .align_y(Alignment::Center);

                if is_selected {
                    row_content = row_content.push(space().width(Length::Fill));
                    row_content = row_content.push(text("⏎ Copy").size(11));
                }

                let btn_style = if is_selected {
                    button::primary
                } else {
                    button::secondary
                };

                let entry_btn = button(row_content)
                    .width(Length::Fill)
                    .style(btn_style)
                    .padding([8, 12])
                    .on_press(Message::CopyAndExit(idx));

                list_col = list_col.push(entry_btn);
            }
        }

        let clear_btn = button(text("󰃢 Clear All").size(11))
            .style(button::danger)
            .padding([3, 8])
            .on_press(Message::ClearHistory);

        let footer = row![
            text(format!("{} items", self.filtered.len())).size(11),
            space().width(Length::Fill),
            text("↑↓ Navigate • Enter Copy • Del Remove • Esc Close").size(11),
            space().width(Length::Fill),
            clear_btn,
        ]
        .align_y(Alignment::Center)
        .padding([4, 6]);

        let container_content = column![
            input,
            scrollable(list_col).id(SCROLL_ID.clone()).height(Length::Fill),
            footer,
        ]
        .spacing(10)
        .padding(14);

        container(container_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(default_modal_card_style())
            .into()
    }
}
