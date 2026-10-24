use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::{button, column, container, progress_bar, row, scrollable, space, text};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use msui_core::{MsuiConfig, StorageManager, StoragePartition, ThemeManager};
use std::time::Duration;

pub fn run_storage_menu(_config: MsuiConfig) -> Result<(), iced_exwlshell::Error> {
    application(
        StorageMenu::new,
        StorageMenu::namespace,
        StorageMenu::update,
        StorageMenu::view,
    )
    .theme(ThemeManager::current_theme())
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(720, 600),
        anchor: Anchor::empty(), // Centered modal
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    })
    .subscription(StorageMenu::subscription)
    .run()
}

pub struct StorageMenu {
    partitions: Vec<StoragePartition>,
    status_msg: Option<String>,
    is_standalone: bool,
    should_close: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Mount(String),
    Unmount(String),
    Eject(String),
    OpenFiles(String),
    Refresh,
    Close,
    IcedEvent(Event),
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

impl StorageMenu {
    pub fn new() -> (Self, Command<Message>) {
        Self::new_internal(true)
    }

    pub fn new_embedded() -> (Self, Command<Message>) {
        Self::new_internal(false)
    }

    fn new_internal(is_standalone: bool) -> (Self, Command<Message>) {
        let partitions = StorageManager::list_partitions();
        (
            Self {
                partitions,
                status_msg: None,
                is_standalone,
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

    pub fn reset(&mut self) -> Command<Message> {
        self.partitions = StorageManager::list_partitions();
        self.status_msg = None;
        self.should_close = false;
        Command::none()
    }

    pub fn namespace() -> String {
        "msui-storage".into()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let poll = iced::time::every(Duration::from_secs(3)).map(|_| Message::Refresh);
        let events = event::listen().map(Message::IcedEvent);
        iced::Subscription::batch(vec![poll, events])
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Mount(dev_path) => {
                match StorageManager::mount_device(&dev_path) {
                    Ok(mp) => {
                        self.status_msg = Some(format!("󰄬 Mounted {dev_path} at {mp}"));
                        self.partitions = StorageManager::list_partitions();
                    }
                    Err(e) => {
                        self.status_msg = Some(format!("󰅖 Mount error: {e}"));
                    }
                }
            }
            Message::Unmount(dev_path) => {
                match StorageManager::unmount_device(&dev_path) {
                    Ok(()) => {
                        self.status_msg = Some(format!("󰄬 Unmounted {dev_path}"));
                        self.partitions = StorageManager::list_partitions();
                    }
                    Err(e) => {
                        self.status_msg = Some(format!("󰅖 Unmount error: {e}"));
                    }
                }
            }
            Message::Eject(dev_path) => {
                match StorageManager::eject_drive(&dev_path) {
                    Ok(()) => {
                        self.status_msg = Some(format!("󰄬 Safely powered off {dev_path}"));
                        self.partitions = StorageManager::list_partitions();
                    }
                    Err(e) => {
                        self.status_msg = Some(format!("󰅖 Eject error: {e}"));
                    }
                }
            }
            Message::OpenFiles(mount_point) => {
                let _ = StorageManager::open_in_file_manager(&mount_point);
            }
            Message::Refresh => {
                self.partitions = StorageManager::list_partitions();
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
        let header_title = row![
            text("󰋊").size(22),
            text("Disk Storage & Removable Drives").size(18),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let refresh_btn = button(text("󰑐 Refresh").size(13))
            .style(button::secondary)
            .padding([6, 12])
            .on_press(Message::Refresh);

        let close_btn = button(text("󰅖").size(15))
            .style(button::danger)
            .padding([4, 10])
            .on_press(Message::Close);

        let header = row![
            header_title,
            space().width(Length::Fill),
            refresh_btn,
            close_btn,
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        // Overall Storage Stats Summary
        let total_internal_used: u64 = self.partitions.iter().filter(|p| !p.is_removable && p.is_mounted).map(|p| p.used_bytes).sum();
        let total_internal_avail: u64 = self.partitions.iter().filter(|p| !p.is_removable && p.is_mounted).map(|p| p.available_bytes).sum();
        let summary_badge = container(
            row![
                text("Internal Storage:").size(12).style(text::secondary),
                text(format!("{} Used", StoragePartition::format_bytes(total_internal_used))).size(12),
                text("•").size(12).style(text::secondary),
                text(format!("{} Available", StoragePartition::format_bytes(total_internal_avail))).size(12),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .padding([4, 10])
        .style(container::bordered_box);

        // Removable Drives Section
        let removable_parts: Vec<&StoragePartition> = self.partitions.iter().filter(|p| p.is_removable).collect();
        let internal_parts: Vec<&StoragePartition> = self.partitions.iter().filter(|p| !p.is_removable).collect();

        let removable_section = self.view_removable_section(&removable_parts);
        let internal_section = self.view_internal_section(&internal_parts);

        let mut content = column![
            header,
            summary_badge,
            removable_section,
            internal_section,
        ]
        .spacing(14);

        if let Some(msg) = &self.status_msg {
            let status_bar = container(
                row![
                    text(msg).size(13),
                    space().width(Length::Fill),
                ]
                .align_y(Alignment::Center)
            )
            .padding([8, 14])
            .style(container::bordered_box);

            content = content.push(status_bar);
        }

        container(
            scrollable(content)
                .spacing(14)
                .width(Length::Fill)
                .height(Length::Fill)
        )
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(container::bordered_box)
        .into()
    }

    fn view_removable_section<'a>(&self, parts: &[&'a StoragePartition]) -> Element<'a, Message> {
        let section_header = row![
            text("󱊞 Removable Drives & External Media").size(14).style(text::secondary),
            space().width(Length::Fill),
            text(format!("{} device(s)", parts.len())).size(12).style(text::secondary),
        ]
        .align_y(Alignment::Center);

        if parts.is_empty() {
            let empty_banner = container(
                row![
                    text("󱊞").size(18).style(text::secondary),
                    text("No external USB or removable drives detected.").size(13).style(text::secondary),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
            )
            .padding([14, 18])
            .width(Length::Fill)
            .style(container::bordered_box);

            return column![section_header, empty_banner].spacing(8).into();
        }

        let mut drive_cards = column![].spacing(10).width(Length::Fill);
        for part in parts {
            drive_cards = drive_cards.push(self.view_partition_card(part));
        }

        column![section_header, drive_cards].spacing(8).into()
    }

    fn view_internal_section<'a>(&self, parts: &[&'a StoragePartition]) -> Element<'a, Message> {
        let section_header = row![
            text("󰋊 Internal NVMe & System Partitions").size(14).style(text::secondary),
            space().width(Length::Fill),
            text(format!("{} partition(s)", parts.len())).size(12).style(text::secondary),
        ]
        .align_y(Alignment::Center);

        let mut part_cards = column![].spacing(8).width(Length::Fill);
        for part in parts {
            part_cards = part_cards.push(self.view_partition_card(part));
        }

        column![section_header, part_cards].spacing(8).into()
    }

    fn view_partition_card<'a>(&self, part: &'a StoragePartition) -> Element<'a, Message> {
        let icon_str = part.icon();
        let title_str = part.display_title();
        let fs_str = part.fs_type.as_deref().unwrap_or("unknown");
        let dev_path = part.device_path.clone();

        let top_left = row![
            text(icon_str).size(20),
            column![
                text(title_str).size(14),
                row![
                    text(&part.device_path).size(11).style(text::secondary),
                    text("•").size(11).style(text::secondary),
                    text(fs_str).size(11).style(text::secondary),
                    if let Some(model) = &part.drive_model {
                        row![
                            text("•").size(11).style(text::secondary),
                            text(model).size(11).style(text::secondary),
                        ].spacing(4)
                    } else {
                        row![]
                    }
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            ]
            .spacing(2),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let mut action_buttons = row![].spacing(8).align_y(Alignment::Center);

        if part.is_mounted {
            if let Some(mp) = &part.mount_point {
                let mp_clone = mp.clone();
                action_buttons = action_buttons.push(
                    button(text("󰁔 Open").size(12))
                        .style(button::primary)
                        .padding([4, 10])
                        .on_press(Message::OpenFiles(mp_clone))
                );
            }

            if part.is_removable {
                action_buttons = action_buttons.push(
                    button(text("󰅖 Unmount").size(12))
                        .style(button::secondary)
                        .padding([4, 10])
                        .on_press(Message::Unmount(dev_path.clone()))
                );

                action_buttons = action_buttons.push(
                    button(text("󱊞 Eject").size(12))
                        .style(button::danger)
                        .padding([4, 10])
                        .on_press(Message::Eject(dev_path.clone()))
                );
            }
        } else {
            action_buttons = action_buttons.push(
                button(text("󰁔 Mount").size(12))
                    .style(button::primary)
                    .padding([4, 10])
                    .on_press(Message::Mount(dev_path.clone()))
            );

            if part.is_removable {
                action_buttons = action_buttons.push(
                    button(text("󱊞 Eject").size(12))
                        .style(button::danger)
                        .padding([4, 10])
                        .on_press(Message::Eject(dev_path.clone()))
                );
            }
        }

        let header_row = row![
            top_left,
            space().width(Length::Fill),
            action_buttons,
        ]
        .align_y(Alignment::Center);

        // Progress bar and metrics
        let stats_row = if part.is_mounted {
            let progress = (part.use_percent as f32 / 100.0).clamp(0.0, 1.0);
            let bar = progress_bar(0.0..=1.0, progress);

            let usage_info = row![
                text(format!("{}% Used", part.use_percent)).size(11),
                space().width(Length::Fill),
                text(format!(
                    "{} / {} ({} free)",
                    part.used_formatted(),
                    part.total_formatted(),
                    part.available_formatted()
                ))
                .size(11)
                .style(text::secondary),
            ]
            .align_y(Alignment::Center);

            column![bar, usage_info].spacing(4)
        } else {
            let info_text = format!("Total: {} • Not mounted", part.total_formatted());
            column![
                text(info_text).size(12).style(text::secondary)
            ]
        };

        container(
            column![
                header_row,
                stats_row,
            ]
            .spacing(10)
        )
        .padding(12)
        .width(Length::Fill)
        .style(container::bordered_box)
        .into()
    }
}
