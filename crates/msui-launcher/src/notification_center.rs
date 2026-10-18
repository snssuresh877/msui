use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::operation::focus;
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{event, Alignment, Element, Event, Font, Length, Task as Command, Theme};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use msui_core::{IpcClient, IpcCommand, Notification, GLOBAL_NOTIFICATIONS};
use std::sync::LazyLock;

static INPUT_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);

pub fn run_notification_center() -> Result<(), iced_exwlshell::Error> {
    application(
        NotificationCenter::new,
        NotificationCenter::namespace,
        NotificationCenter::update,
        NotificationCenter::view,
    )
    .theme(Theme::CatppuccinMocha)
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
    .subscription(NotificationCenter::subscription)
    .run()
}

pub struct NotificationCenter {
    query: String,
    all_notifications: Vec<Notification>,
    filtered: Vec<Notification>,
    selected_idx: usize,
}

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    SelectPrevious,
    SelectNext,
    Activate(u32),
    DeleteNotification(u32),
    ClearAll,
    Close,
    IcedEvent(Event),
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

impl NotificationCenter {
    fn new() -> (Self, Command<Message>) {
        let all_notifications = {
            let mut store = GLOBAL_NOTIFICATIONS.lock().unwrap();
            store.load();
            store.mark_all_read();
            store.history()
        };
        let filtered = all_notifications.clone();

        (
            Self {
                query: String::new(),
                all_notifications,
                filtered,
                selected_idx: 0,
            },
            focus(INPUT_ID.clone()),
        )
    }

    fn namespace() -> String {
        String::from("msui-notification-center")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        event::listen().map(Message::IcedEvent)
    }

    fn update_filtered(&mut self) {
        if self.query.trim().is_empty() {
            self.filtered = self.all_notifications.clone();
            self.selected_idx = 0;
            return;
        }

        let q = self.query.to_lowercase();
        self.filtered = self
            .all_notifications
            .iter()
            .filter(|n| {
                n.app_name.to_lowercase().contains(&q)
                    || n.summary.to_lowercase().contains(&q)
                    || n.body.to_lowercase().contains(&q)
            })
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
            Message::Activate(id) => {
                if let Some(notif) = self.filtered.iter().find(|n| n.id == id) {
                    let actions = notif.parsed_actions();
                    let action_key = actions
                        .first()
                        .map(|a| a.key.clone())
                        .unwrap_or_else(|| "default".to_string());
                    let _ = IpcClient::send_blocking(&IpcCommand::NotificationAction {
                        id,
                        action: action_key,
                    });
                }
                std::process::exit(0);
            }
            Message::DeleteNotification(id) => {
                {
                    let mut store = GLOBAL_NOTIFICATIONS.lock().unwrap();
                    store.delete_history(id);
                    self.all_notifications = store.history();
                }
                self.update_filtered();
            }
            Message::ClearAll => {
                {
                    let mut store = GLOBAL_NOTIFICATIONS.lock().unwrap();
                    store.clear_history();
                    self.all_notifications.clear();
                }
                self.update_filtered();
            }
            Message::Close => {
                std::process::exit(0);
            }
            Message::IcedEvent(Event::Keyboard(KeyboardEvent::KeyPressed { key, .. })) => {
                match key {
                    Key::Named(Named::Escape) => {
                        return Command::done(Message::Close);
                    }
                    Key::Named(Named::Enter) => {
                        if let Some(entry) = self.filtered.get(self.selected_idx) {
                            return Command::done(Message::Activate(entry.id));
                        }
                    }
                    Key::Named(Named::ArrowDown) => {
                        return Command::done(Message::SelectNext);
                    }
                    Key::Named(Named::ArrowUp) => {
                        return Command::done(Message::SelectPrevious);
                    }
                    Key::Named(Named::Delete) => {
                        if let Some(entry) = self.filtered.get(self.selected_idx) {
                            return Command::done(Message::DeleteNotification(entry.id));
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
        let input = text_input("󰍉 Search notification history...", &self.query)
            .id(INPUT_ID.clone())
            .on_input(Message::QueryChanged)
            .padding(12)
            .size(16);

        let mut list_col = column![].spacing(8);

        if self.filtered.is_empty() {
            let empty_text = if self.all_notifications.is_empty() {
                "No notifications in history."
            } else {
                "No matching notifications found."
            };
            list_col = list_col.push(
                container(text(empty_text).size(13))
                    .padding(32)
                    .align_x(Alignment::Center),
            );
        } else {
            for (idx, notif) in self.filtered.iter().enumerate() {
                let is_selected = idx == self.selected_idx;

                let app_title = text(format!("󰂚 {}", &notif.app_name)).size(11);
                let time_str = text(&notif.timestamp).size(11);

                let close_btn = button(text("✕").size(11))
                    .style(button::text)
                    .padding([1, 6])
                    .on_press(Message::DeleteNotification(notif.id));

                let card_header = row![
                    app_title,
                    space().width(Length::Fill),
                    time_str,
                    close_btn,
                ]
                .align_y(Alignment::Center);

                let summary = text(&notif.summary).size(13);
                let body = text(&notif.body).size(11);

                let mut card_content = column![card_header, summary, body].spacing(4);

                let actions = notif.parsed_actions();
                if !actions.is_empty() {
                    let mut action_row = row![].spacing(6);
                    for action in actions {
                        action_row = action_row.push(
                            button(text(action.label).size(11))
                                .style(button::secondary)
                                .padding([2, 8])
                                .on_press(Message::Activate(notif.id)),
                        );
                    }
                    card_content = card_content.push(action_row);
                }

                let card_style = if is_selected {
                    container::bordered_box
                } else {
                    container::transparent
                };

                let card = container(card_content)
                    .width(Length::Fill)
                    .padding(10)
                    .style(card_style);

                list_col = list_col.push(card);
            }
        }

        let clear_btn = button(text("󰃢 Clear All").size(11))
            .style(button::danger)
            .padding([3, 8])
            .on_press(Message::ClearAll);

        let footer = row![
            text(format!("{} notifications", self.filtered.len())).size(11),
            space().width(Length::Fill),
            text("↑↓ Navigate • Enter Open • Del Remove • Esc Close").size(11),
            space().width(Length::Fill),
            clear_btn,
        ]
        .align_y(Alignment::Center)
        .padding([4, 6]);

        let container_content = column![
            input,
            scrollable(list_col).height(Length::Fill),
            footer,
        ]
        .spacing(10)
        .padding(14);

        container(container_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(container::bordered_box)
            .into()
    }
}
