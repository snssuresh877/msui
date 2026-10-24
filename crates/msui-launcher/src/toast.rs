use iced::widget::{button, column, container, row, space, text};
use iced::{Alignment, Element, Font, Length, Task as Command, Theme};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize, Margin};
use iced_exwlshell::settings::LayerShellSettings;
use msui_core::{
    clear_active_toasts, dismiss_active_toast, load_active_toasts, record_toast_pid,
    remove_toast_pid, IpcClient, IpcCommand, Notification,
};
use std::collections::HashSet;
use std::time::Duration;

pub fn run_toast_window() -> Result<(), iced_exwlshell::Error> {
    record_toast_pid(std::process::id());

    let loaded = load_active_toasts();
    if loaded.is_empty() {
        remove_toast_pid();
        return Ok(());
    }

    let initial_height = ((loaded.len() as u32).min(4) * 115 + 20).clamp(100, 500);

    let res = application(
        ToastApp::new,
        ToastApp::namespace,
        ToastApp::update,
        ToastApp::view,
    )
    .theme(Theme::CatppuccinMocha)
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(380, initial_height),
        anchor: Anchor::Top | Anchor::Right,
        layer: Layer::Overlay,
        margin: Margin {
            top: 40, // Below top bar
            right: 16,
            bottom: 0,
            left: 0,
        },
        keyboard_interactivity: KeyboardInteractivity::None,
        ..Default::default()
    })
    .subscription(ToastApp::subscription)
    .run();

    remove_toast_pid();
    clear_active_toasts();
    res
}

pub struct ToastApp {
    seen_ids: HashSet<u32>,
    dismissed_ids: HashSet<u32>,
    active_notifications: Vec<(Notification, u32)>, // (Notification, ticks_remaining)
    pub is_standalone: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Action(u32, String),
    Dismiss(u32),
    Tick,
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

impl ToastApp {
    pub fn new() -> (Self, Command<Message>) {
        Self::new_with_mode(true)
    }

    pub fn new_embedded() -> (Self, Command<Message>) {
        Self::new_with_mode(false)
    }

    pub fn new_with_mode(is_standalone: bool) -> (Self, Command<Message>) {
        let loaded = load_active_toasts();
        if loaded.is_empty() && is_standalone {
            remove_toast_pid();
            std::process::exit(0);
        }

        let mut seen = HashSet::new();
        let mut active = Vec::new();
        for notif in loaded {
            seen.insert(notif.id);
            // 400ms tick per step (minimum 10 ticks = 4s)
            let ticks = ((notif.expire_timeout_ms / 400).max(10)) as u32;
            active.push((notif, ticks));
        }

        (
            Self {
                seen_ids: seen,
                dismissed_ids: HashSet::new(),
                active_notifications: active,
                is_standalone,
            },
            Command::none(),
        )
    }

    pub fn is_empty(&self) -> bool {
        self.active_notifications.is_empty()
    }

    pub fn namespace() -> String {
        String::from("msui-toast")
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(Duration::from_millis(400)).map(|_| Message::Tick)
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Action(id, key) => {
                self.dismissed_ids.insert(id);
                self.active_notifications.retain(|(n, _)| n.id != id);
                dismiss_active_toast(id);
                let _ = IpcClient::send_blocking(&IpcCommand::NotificationAction {
                    id,
                    action: key,
                });
                if self.active_notifications.is_empty() {
                    remove_toast_pid();
                    clear_active_toasts();
                    if self.is_standalone {
                        std::process::exit(0);
                    }
                }
            }
            Message::Dismiss(id) => {
                self.dismissed_ids.insert(id);
                self.active_notifications.retain(|(n, _)| n.id != id);
                dismiss_active_toast(id);
                let _ = IpcClient::send_blocking(&IpcCommand::NotificationClose {
                    id,
                    reason: 2, // 2 = dismissed by user
                });
                if self.active_notifications.is_empty() {
                    remove_toast_pid();
                    clear_active_toasts();
                    if self.is_standalone {
                        std::process::exit(0);
                    }
                }
            }
            Message::Tick => {
                // Poll active toasts file for newly arrived notifications
                let file_toasts = load_active_toasts();
                for notif in file_toasts {
                    if self.dismissed_ids.contains(&notif.id) {
                        continue;
                    }
                    let already_present = self
                        .active_notifications
                        .iter()
                        .any(|(n, _)| n.id == notif.id);
                    if !already_present && !self.seen_ids.contains(&notif.id) {
                        self.seen_ids.insert(notif.id);
                        let ticks = ((notif.expire_timeout_ms / 400).max(10)) as u32;
                        self.active_notifications.insert(0, (notif, ticks));
                    }
                }

                if self.active_notifications.len() > 6 {
                    self.active_notifications.truncate(6);
                }

                // Decrement ticks for active toasts
                let mut expired = Vec::new();
                for (notif, remaining) in &mut self.active_notifications {
                    if *remaining <= 1 {
                        expired.push(notif.id);
                    } else {
                        *remaining -= 1;
                    }
                }

                for id in expired {
                    self.dismissed_ids.insert(id);
                    self.active_notifications.retain(|(n, _)| n.id != id);
                    dismiss_active_toast(id);
                    let _ = IpcClient::send_blocking(&IpcCommand::NotificationClose {
                        id,
                        reason: 1, // 1 = expired
                    });
                }

                if self.active_notifications.is_empty() {
                    remove_toast_pid();
                    clear_active_toasts();
                    if self.is_standalone {
                        std::process::exit(0);
                    }
                }
            }
        }
        Command::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        if self.active_notifications.is_empty() {
            return space().width(Length::Shrink).height(Length::Shrink).into();
        }

        let mut cards = column![].spacing(8);

        for (notif, _) in &self.active_notifications {
            let app_title = text(format!("󰂚 {}", &notif.app_name))
                .size(11);
            let time_str = text(&notif.timestamp).size(11);

            let close_btn = button(text("✕").size(12))
                .style(button::text)
                .padding([1, 6])
                .on_press(Message::Dismiss(notif.id));

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
                            .on_press(Message::Action(notif.id, action.key)),
                    );
                }
                card_content = card_content.push(action_row);
            }

            let card = container(card_content)
                .width(Length::Fill)
                .padding(10)
                .style(container::bordered_box);

            cards = cards.push(card);
        }

        container(cards)
            .width(Length::Fill)
            .padding(6)
            .into()
    }
}
