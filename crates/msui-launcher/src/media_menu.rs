use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::{button, column, container, image, row, slider, space, text};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use msui_core::{MediaInfo, MediaMonitor, MsuiConfig, ThemeManager};
use std::path::PathBuf;
use std::time::Duration;

pub fn run_media_menu(_config: MsuiConfig) -> Result<(), iced_exwlshell::Error> {
    let width = 540;
    let height = 240;

    application(
        MediaMenu::new,
        MediaMenu::namespace,
        MediaMenu::update,
        MediaMenu::view,
    )
    .theme(ThemeManager::current_theme())
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(width, height),
        anchor: Anchor::empty(), // Centered on screen
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    })
    .subscription(MediaMenu::subscription)
    .run()
}

pub struct MediaMenu {
    info: Option<MediaInfo>,
    players: Vec<String>,
    selected_player: Option<String>,
    is_standalone: bool,
    should_close: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    PlayPause,
    Next,
    Previous,
    Seek(u64),
    SeekRelative(i64),
    ToggleShuffle,
    CycleLoop,
    SelectPlayer(String),
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

impl MediaMenu {
    pub fn new() -> (Self, Command<Message>) {
        Self::new_internal(true)
    }

    pub fn new_embedded() -> (Self, Command<Message>) {
        Self::new_internal(false)
    }

    fn new_internal(is_standalone: bool) -> (Self, Command<Message>) {
        let players = MediaMonitor::list_players();
        let selected_player = players.first().cloned();
        let info = MediaMonitor::read_player(selected_player.as_deref());

        (
            Self {
                info,
                players,
                selected_player,
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
        self.players = MediaMonitor::list_players();
        if self.selected_player.is_none() || !self.players.contains(self.selected_player.as_ref().unwrap_or(&String::new())) {
            self.selected_player = self.players.first().cloned();
        }
        self.info = MediaMonitor::read_player(self.selected_player.as_deref());
        self.should_close = false;
        Command::none()
    }

    pub fn namespace() -> String {
        "msui-media-menu".to_string()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let tick = iced::time::every(Duration::from_millis(500)).map(|_| Message::Refresh);
        let events = event::listen().map(Message::IcedEvent);
        iced::Subscription::batch([tick, events])
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::PlayPause => {
                MediaMonitor::play_pause();
                if let Some(info) = &mut self.info {
                    info.is_playing = !info.is_playing;
                }
            }
            Message::Next => {
                MediaMonitor::next();
                std::thread::sleep(Duration::from_millis(150));
                return Command::done(Message::Refresh);
            }
            Message::Previous => {
                MediaMonitor::previous();
                std::thread::sleep(Duration::from_millis(150));
                return Command::done(Message::Refresh);
            }
            Message::Seek(pos) => {
                MediaMonitor::seek(pos);
                if let Some(info) = &mut self.info {
                    info.position_secs = pos;
                }
            }
            Message::SeekRelative(offset) => {
                MediaMonitor::seek_relative(offset);
                if let Some(info) = &mut self.info {
                    if offset >= 0 {
                        info.position_secs = info.position_secs.saturating_add(offset as u64);
                    } else {
                        info.position_secs = info.position_secs.saturating_sub((-offset) as u64);
                    }
                }
            }
            Message::ToggleShuffle => {
                MediaMonitor::toggle_shuffle();
                if let Some(info) = &mut self.info {
                    info.shuffle = !info.shuffle;
                }
            }
            Message::CycleLoop => {
                MediaMonitor::cycle_loop();
            }
            Message::SelectPlayer(p) => {
                self.selected_player = Some(p.clone());
                self.info = MediaMonitor::read_player(Some(&p));
            }
            Message::Refresh => {
                self.players = MediaMonitor::list_players();
                if self.selected_player.is_none() || !self.players.contains(self.selected_player.as_ref().unwrap()) {
                    self.selected_player = self.players.first().cloned();
                }
                self.info = MediaMonitor::read_player(self.selected_player.as_deref());
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
                    Key::Named(Named::Space) => {
                        return Command::done(Message::PlayPause);
                    }
                    Key::Named(Named::ArrowRight) => {
                        return Command::done(Message::SeekRelative(5));
                    }
                    Key::Named(Named::ArrowLeft) => {
                        return Command::done(Message::SeekRelative(-5));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Command::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = if let Some(info) = &self.info {
            // Album art element
            let mut local_art_path: Option<PathBuf> = None;
            if let Some(url) = &info.art_url {
                if let Some(stripped) = url.strip_prefix("file://") {
                    let p = PathBuf::from(stripped);
                    if p.exists() {
                        local_art_path = Some(p);
                    }
                } else {
                    let p = PathBuf::from(url);
                    if p.exists() {
                        local_art_path = Some(p);
                    }
                }
            }

            let art_widget: Element<'_, Message> = if let Some(path) = local_art_path {
                container(
                    image(image::Handle::from_path(path))
                        .width(Length::Fixed(140.0))
                        .height(Length::Fixed(140.0))
                )
                .style(container::bordered_box)
                .into()
            } else {
                container(
                    column![
                        text("󰎆").size(48),
                        text("msUI Sound").size(10),
                    ]
                    .spacing(4)
                    .align_x(Alignment::Center)
                )
                .width(Length::Fixed(140.0))
                .height(Length::Fixed(140.0))
                .center_x(Length::Fixed(140.0))
                .center_y(Length::Fixed(140.0))
                .style(container::bordered_box)
                .into()
            };

            // Player header row
            let player_label = if info.player_name.is_empty() {
                "Media Player".to_string()
            } else {
                format!("󰝚 {}", info.player_name)
            };
            let player_pill = text(player_label).size(12);

            let status_badge = text(if info.is_playing { "󰐊 Playing" } else { "󰏤 Paused" }).size(11);

            let close_btn = button(text("✕").size(12))
                .style(button::danger)
                .padding([2, 7])
                .on_press(Message::Close);

            let header = row![
                player_pill,
                space().width(Length::Fill),
                status_badge,
                close_btn,
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            // Track metadata
            let title_str = if info.title.is_empty() { "Unknown Title" } else { &info.title };
            let title_text = text(title_str).size(16);

            let artist_str = if info.artist.is_empty() { "Unknown Artist" } else { &info.artist };
            let album_str = if info.album.is_empty() { "".to_string() } else { format!(" • {}", info.album) };
            let meta_text = text(format!("{}{}", artist_str, album_str)).size(12);

            // Scrubber
            let cur_time = format_time(info.position_secs);
            let tot_time = format_time(info.length_secs);
            let max_len = (info.length_secs as u32).max(1);
            let cur_pos = (info.position_secs as u32).min(max_len);

            let scrubber = slider(0..=max_len, cur_pos, |v| Message::Seek(v as u64)).width(Length::Fill);

            let progress_row = row![
                text(cur_time).size(11),
                scrubber,
                text(tot_time).size(11),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            // Controls
            let prev_btn = button(text("󰒮").size(14))
                .style(button::secondary)
                .padding([4, 10])
                .on_press(Message::Previous);

            let play_btn = button(text(if info.is_playing { "󰏤 Pause" } else { "󰐊 Play" }).size(13))
                .style(button::primary)
                .padding([5, 14])
                .on_press(Message::PlayPause);

            let next_btn = button(text("󰒭").size(14))
                .style(button::secondary)
                .padding([4, 10])
                .on_press(Message::Next);

            let shuffle_btn = button(text("󰒞").size(13))
                .style(if info.shuffle { button::primary } else { button::secondary })
                .padding([4, 8])
                .on_press(Message::ToggleShuffle);

            let loop_btn = button(text("󰑖").size(13))
                .style(button::secondary)
                .padding([4, 8])
                .on_press(Message::CycleLoop);

            let controls_row = row![
                shuffle_btn,
                space().width(Length::Fill),
                prev_btn,
                play_btn,
                next_btn,
                space().width(Length::Fill),
                loop_btn,
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            let details_col = column![
                header,
                title_text,
                meta_text,
                progress_row,
                controls_row,
            ]
            .spacing(8)
            .width(Length::Fill);

            row![art_widget, details_col].spacing(16).into()
        } else {
            // Empty state
            let empty_icon = text("󰝚").size(40);
            let empty_title = text("No Media Currently Playing").size(15);
            let empty_desc = text("Start playback in Spotify, Firefox, MPV, or VLC to view controls here.").size(12);

            let refresh_btn = button(text("󰑐 Refresh").size(12))
                .style(button::primary)
                .padding([5, 12])
                .on_press(Message::Refresh);

            let close_btn = button(text("Close").size(12))
                .style(button::secondary)
                .padding([5, 12])
                .on_press(Message::Close);

            let btn_row = row![refresh_btn, close_btn].spacing(8);

            column![empty_icon, empty_title, empty_desc, btn_row]
                .spacing(10)
                .align_x(Alignment::Center)
                .into()
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(16)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(container::bordered_box)
            .into()
    }
}

fn format_time(secs: u64) -> String {
    let m = secs / 60;
    let s = secs % 60;
    format!("{:02}:{:02}", m, s)
}
