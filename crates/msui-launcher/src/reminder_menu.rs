use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::operation::focus;
use iced::widget::{button, column, container, progress_bar, row, scrollable, slider, space, text, text_input};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use crate::modal_style::{default_modal_card_style, default_subcard_style};
use msui_core::{MsuiConfig, ReminderItem, ReminderManager, ThemeManager};
use std::sync::LazyLock;
use std::time::Duration;

static TITLE_INPUT_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);

pub fn run_reminder_menu(_config: MsuiConfig) -> Result<(), iced_exwlshell::Error> {
    application(
        ReminderMenu::new,
        ReminderMenu::namespace,
        ReminderMenu::update,
        ReminderMenu::view,
    )
    .theme(ThemeManager::current_theme())
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(680, 580),
        anchor: Anchor::empty(), // Centered modal
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    })
    .subscription(ReminderMenu::subscription)
    .run()
}

pub struct ReminderMenu {
    reminders: Vec<ReminderItem>,
    input_title: String,
    duration_mins: u32,
    status_msg: Option<String>,
    is_standalone: bool,
    should_close: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    TitleChanged(String),
    DurationChanged(u32),
    SelectPreset(u32),
    AddReminder,
    DeleteReminder(u64),
    ClearAll,
    Tick,
    Close,
    IcedEvent(Event),
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

impl ReminderMenu {
    pub fn new() -> (Self, Command<Message>) {
        Self::new_internal(true)
    }

    pub fn new_embedded() -> (Self, Command<Message>) {
        Self::new_internal(false)
    }

    fn new_internal(is_standalone: bool) -> (Self, Command<Message>) {
        let reminders = ReminderManager::list_active();
        (
            Self {
                reminders,
                input_title: String::new(),
                duration_mins: 15,
                status_msg: None,
                is_standalone,
                should_close: false,
            },
            focus(TITLE_INPUT_ID.clone()),
        )
    }

    pub fn should_close(&self) -> bool {
        self.should_close
    }

    pub fn reset_should_close(&mut self) {
        self.should_close = false;
    }

    pub fn reset(&mut self) -> Command<Message> {
        self.reminders = ReminderManager::list_active();
        self.input_title.clear();
        self.status_msg = None;
        self.should_close = false;
        focus(TITLE_INPUT_ID.clone())
    }

    pub fn namespace() -> String {
        "msui-reminders".into()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let tick = iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick);
        let events = event::listen().map(Message::IcedEvent);
        iced::Subscription::batch(vec![tick, events])
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::TitleChanged(val) => {
                self.input_title = val;
            }
            Message::DurationChanged(mins) => {
                self.duration_mins = mins.clamp(1, 240);
            }
            Message::SelectPreset(mins) => {
                self.duration_mins = mins;
            }
            Message::AddReminder => {
                let title = self.input_title.trim();
                let effective_title = if title.is_empty() {
                    format!("Timer ({}m)", self.duration_mins)
                } else {
                    title.to_string()
                };

                let item = ReminderManager::add_reminder(&effective_title, self.duration_mins as u64);
                self.status_msg = Some(format!("󰄬 Added timer: \"{}\" for {} min(s)", item.title, self.duration_mins));
                self.input_title.clear();
                self.reminders = ReminderManager::list_active();
                return focus(TITLE_INPUT_ID.clone());
            }
            Message::DeleteReminder(id) => {
                if ReminderManager::remove_reminder(id) {
                    self.reminders = ReminderManager::list_active();
                }
            }
            Message::ClearAll => {
                ReminderManager::clear_all();
                self.reminders.clear();
                self.status_msg = Some("󰄬 All reminders cleared".into());
            }
            Message::Tick => {
                let newly_expired = ReminderManager::check_and_trigger_expired();
                if !newly_expired.is_empty() {
                    if let Some(first) = newly_expired.first() {
                        self.status_msg = Some(format!("󰥔 Expired: \"{}\"!", first.title));
                    }
                }
                self.reminders = ReminderManager::list_active();
            }
            Message::Close => {
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
            }
            Message::IcedEvent(Event::Keyboard(KeyboardEvent::KeyPressed { key, .. })) => {
                if let Key::Named(Named::Escape) = key {
                    return self.update(Message::Close);
                }
            }
            Message::IcedEvent(_) => {}
        }
        Command::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        // Header
        let active_count = self.reminders.iter().filter(|r| !r.is_expired()).count();
        let header_title = row![
            text("󰥔").size(22),
            text("Reminders & Countdown Timers").size(18),
            container(text(format!("{} active", active_count)).size(11))
                .padding([2, 8])
                .style(container::bordered_box),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let clear_btn = button(text("󰃢 Clear All").size(12))
            .style(button::secondary)
            .padding([5, 10])
            .on_press(Message::ClearAll);

        let close_btn = button(text("󰅖").size(15))
            .style(button::danger)
            .padding([4, 10])
            .on_press(Message::Close);

        let header = row![
            header_title,
            space().width(Length::Fill),
            clear_btn,
            close_btn,
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        // Add Reminder Section
        let create_panel = self.view_create_panel();

        // Active Timers Section
        let timers_section = self.view_timers_section();

        let mut content = column![
            header,
            create_panel,
            timers_section,
        ]
        .spacing(14);

        if let Some(msg) = &self.status_msg {
            let status_badge = container(
                row![
                    text(msg).size(12),
                    space().width(Length::Fill),
                ]
                .align_y(Alignment::Center)
            )
            .padding([6, 12])
            .style(default_subcard_style());

            content = content.push(status_badge);
        }

        container(
            content
                .width(Length::Fill)
                .height(Length::Fill)
        )
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(default_modal_card_style())
        .into()
    }

    fn view_create_panel(&self) -> Element<'_, Message> {
        let title_input = text_input(
            "What do you want to remember? (e.g. Tea break, Push commit)",
            &self.input_title,
        )
        .id(TITLE_INPUT_ID.clone())
        .on_input(Message::TitleChanged)
        .on_submit(Message::AddReminder)
        .padding(10)
        .size(13);

        // Presets: 5m, 10m, 15m, 25m (Pomodoro), 30m, 45m, 60m
        let presets = [5, 10, 15, 25, 30, 45, 60];
        let mut preset_row = row![].spacing(6).align_y(Alignment::Center);

        for p in presets {
            let label = if p == 25 {
                "󰄉 25m Pomo".to_string()
            } else {
                format!("+{}m", p)
            };
            let is_selected = self.duration_mins == p;
            let btn = button(text(label).size(11))
                .style(if is_selected { button::primary } else { button::secondary })
                .padding([4, 8])
                .on_press(Message::SelectPreset(p));
            preset_row = preset_row.push(btn);
        }

        let dur_slider = slider(1..=120, self.duration_mins, Message::DurationChanged)
            .width(Length::FillPortion(2));

        let dur_label = text(format!("{} min(s)", self.duration_mins))
            .size(12)
            .width(Length::Fixed(70.0));

        let add_btn = button(
            row![
                text("󰐕").size(14),
                text("Set Timer").size(12),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        )
        .style(button::primary)
        .padding([6, 14])
        .on_press(Message::AddReminder);

        let duration_controls = row![
            preset_row,
            space().width(Length::Fill),
            dur_label,
            dur_slider,
            add_btn,
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        container(
            column![
                title_input,
                duration_controls,
            ]
            .spacing(10)
        )
        .padding(12)
        .style(default_subcard_style())
        .into()
    }

    fn view_timers_section(&self) -> Element<'_, Message> {
        let section_header = row![
            text("󰥔 Scheduled Timers & Alerts").size(14).style(text::secondary),
            space().width(Length::Fill),
            text(format!("{} timer(s)", self.reminders.len())).size(12).style(text::secondary),
        ]
        .align_y(Alignment::Center);

        if self.reminders.is_empty() {
            let empty_banner = container(
                column![
                    row![
                        text("󰥔").size(24).style(text::secondary),
                        text("No active reminders or timers scheduled.").size(14).style(text::secondary),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                    text("Add a quick timer above or press Super + R anytime!").size(12).style(text::secondary),
                ]
                .spacing(6)
                .align_x(Alignment::Center)
            )
            .padding([28, 16])
            .center_x(Length::Fill)
            .style(default_subcard_style());

            return column![section_header, empty_banner].spacing(8).into();
        }

        let mut timer_cards = column![].spacing(8).width(Length::Fill);
        for reminder in &self.reminders {
            timer_cards = timer_cards.push(self.view_timer_card(reminder));
        }

        let scroll_list = scrollable(timer_cards)
            .spacing(8)
            .width(Length::Fill)
            .height(Length::Fill);

        column![section_header, scroll_list].spacing(8).into()
    }

    fn view_timer_card(&self, item: &ReminderItem) -> Element<'_, Message> {
        let is_expired = item.is_expired();
        let rem_str = item.formatted_remaining();
        let total_mins = (item.total_duration_secs / 60).max(1);

        let badge = if is_expired {
            container(
                row![
                    text("󰥔 Expired!").size(12),
                ]
                .align_y(Alignment::Center)
            )
            .padding([3, 8])
            .style(default_subcard_style())
        } else {
            container(
                row![
                    text("󱫠").size(13),
                    text(rem_str).size(12),
                ]
                .spacing(4)
                .align_y(Alignment::Center)
            )
            .padding([3, 8])
            .style(default_subcard_style())
        };

        let action_btn = if is_expired {
            button(text("󰄬 Done").size(11))
                .style(button::primary)
                .padding([4, 10])
                .on_press(Message::DeleteReminder(item.id))
        } else {
            button(text("󰅖 Cancel").size(11))
                .style(button::secondary)
                .padding([4, 10])
                .on_press(Message::DeleteReminder(item.id))
        };

        let top_row = row![
            text(if is_expired { "󰥔" } else { "󱫠" }).size(18),
            column![
                text(item.title.clone()).size(14),
                text(format!("Total: {} min(s)", total_mins)).size(10).style(text::secondary),
            ]
            .spacing(1),
            space().width(Length::Fill),
            badge,
            action_btn,
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let prog = item.progress();
        let p_bar = progress_bar(0.0..=1.0, prog);

        container(
            column![
                top_row,
                p_bar,
            ]
            .spacing(8)
        )
        .padding(10)
        .width(Length::Fill)
        .style(default_subcard_style())
        .into()
    }
}
