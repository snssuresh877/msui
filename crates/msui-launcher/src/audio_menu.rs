use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::{button, column, container, row, scrollable, slider, space, text};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use msui_core::{AudioDevice, AudioMonitor, ThemeManager};
use std::time::Duration;

pub fn run_audio_menu() -> Result<(), iced_exwlshell::Error> {
    application(
        AudioMenu::new,
        AudioMenu::namespace,
        AudioMenu::update,
        AudioMenu::view,
    )
    .theme(ThemeManager::current_theme())
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(680, 580),
        anchor: Anchor::empty(), // Centered on screen
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    })
    .subscription(AudioMenu::subscription)
    .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioTab {
    Outputs,
    Inputs,
}

pub struct AudioMenu {
    sinks: Vec<AudioDevice>,
    sources: Vec<AudioDevice>,
    active_tab: AudioTab,
    status_msg: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SwitchTab(AudioTab),
    Refresh,
    SetDefaultSink(String),
    SetDefaultSource(String),
    SetSinkVolume(String, u8),
    SetSourceVolume(String, u8),
    AdjustSinkVolume(String, i8),
    AdjustSourceVolume(String, i8),
    ToggleSinkMute(String),
    ToggleSourceMute(String),
    Close,
    IcedEvent(Event),
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

impl AudioMenu {
    fn new() -> (Self, Command<Message>) {
        let sinks = AudioMonitor::list_sinks();
        let sources = AudioMonitor::list_sources();

        (
            Self {
                sinks,
                sources,
                active_tab: AudioTab::Outputs,
                status_msg: None,
            },
            Command::none(),
        )
    }

    fn namespace() -> String {
        String::from("msui-audio-menu")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        let tick = iced::time::every(Duration::from_millis(2500)).map(|_| Message::Refresh);
        let events = event::listen().map(Message::IcedEvent);
        iced::Subscription::batch([tick, events])
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SwitchTab(tab) => {
                self.active_tab = tab;
            }
            Message::Refresh => {
                self.sinks = AudioMonitor::list_sinks();
                self.sources = AudioMonitor::list_sources();
            }
            Message::SetDefaultSink(name) => {
                AudioMonitor::set_default_sink(&name);
                self.status_msg = Some(format!("Set default output: {}", &name));
                self.sinks = AudioMonitor::list_sinks();
            }
            Message::SetDefaultSource(name) => {
                AudioMonitor::set_default_source(&name);
                self.status_msg = Some(format!("Set default input: {}", &name));
                self.sources = AudioMonitor::list_sources();
            }
            Message::SetSinkVolume(name, vol) => {
                AudioMonitor::set_sink_volume(&name, vol);
                if let Some(sink) = self.sinks.iter_mut().find(|s| s.name == name) {
                    sink.volume_percent = vol;
                }
            }
            Message::SetSourceVolume(name, vol) => {
                AudioMonitor::set_source_volume(&name, vol);
                if let Some(source) = self.sources.iter_mut().find(|s| s.name == name) {
                    source.volume_percent = vol;
                }
            }
            Message::AdjustSinkVolume(name, delta) => {
                if let Some(sink) = self.sinks.iter_mut().find(|s| s.name == name) {
                    let new_vol = if delta >= 0 {
                        sink.volume_percent.saturating_add(delta as u8).min(100)
                    } else {
                        sink.volume_percent.saturating_sub((-delta) as u8)
                    };
                    sink.volume_percent = new_vol;
                    AudioMonitor::set_sink_volume(&name, new_vol);
                }
            }
            Message::AdjustSourceVolume(name, delta) => {
                if let Some(source) = self.sources.iter_mut().find(|s| s.name == name) {
                    let new_vol = if delta >= 0 {
                        source.volume_percent.saturating_add(delta as u8).min(100)
                    } else {
                        source.volume_percent.saturating_sub((-delta) as u8)
                    };
                    source.volume_percent = new_vol;
                    AudioMonitor::set_source_volume(&name, new_vol);
                }
            }
            Message::ToggleSinkMute(name) => {
                AudioMonitor::toggle_sink_mute(&name);
                if let Some(sink) = self.sinks.iter_mut().find(|s| s.name == name) {
                    sink.is_muted = !sink.is_muted;
                }
            }
            Message::ToggleSourceMute(name) => {
                AudioMonitor::toggle_source_mute(&name);
                if let Some(source) = self.sources.iter_mut().find(|s| s.name == name) {
                    source.is_muted = !source.is_muted;
                }
            }
            Message::Close => {
                std::process::exit(0);
            }
            Message::IcedEvent(Event::Keyboard(KeyboardEvent::KeyPressed { key, .. })) => {
                match key {
                    Key::Named(Named::Escape) => {
                        std::process::exit(0);
                    }
                    Key::Named(Named::Tab) => {
                        self.active_tab = match self.active_tab {
                            AudioTab::Outputs => AudioTab::Inputs,
                            AudioTab::Inputs => AudioTab::Outputs,
                        };
                    }
                    _ => {}
                }
            }
            Message::IcedEvent(_) => {}
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        // Header
        let header_title = text("  Audio Mixer & Devices").size(18);

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
            close_btn
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        // Tab Bar
        let output_tab_label = format!("  Outputs ({})", self.sinks.len());
        let input_tab_label = format!("󰍬  Inputs ({})", self.sources.len());

        let output_tab_btn = button(text(output_tab_label).size(13))
            .style(if self.active_tab == AudioTab::Outputs {
                button::primary
            } else {
                button::secondary
            })
            .padding([6, 16])
            .on_press(Message::SwitchTab(AudioTab::Outputs));

        let input_tab_btn = button(text(input_tab_label).size(13))
            .style(if self.active_tab == AudioTab::Inputs {
                button::primary
            } else {
                button::secondary
            })
            .padding([6, 16])
            .on_press(Message::SwitchTab(AudioTab::Inputs));

        let tab_bar = row![output_tab_btn, input_tab_btn]
            .spacing(10)
            .align_y(Alignment::Center);

        // Device List Content
        let content_list: Element<'_, Message> = match self.active_tab {
            AudioTab::Outputs => {
                if self.sinks.is_empty() {
                    container(
                        text("No audio output devices detected")
                            .size(14)
                            .style(text::secondary),
                    )
                    .center(Length::Fill)
                    .padding(40)
                    .into()
                } else {
                    let mut col = column![].spacing(12);
                    for sink in &self.sinks {
                        col = col.push(self.view_device_card(sink, true));
                    }
                    scrollable(col).height(Length::Fill).into()
                }
            }
            AudioTab::Inputs => {
                if self.sources.is_empty() {
                    container(
                        text("No audio input devices detected")
                            .size(14)
                            .style(text::secondary),
                    )
                    .center(Length::Fill)
                    .padding(40)
                    .into()
                } else {
                    let mut col = column![].spacing(12);
                    for source in &self.sources {
                        col = col.push(self.view_device_card(source, false));
                    }
                    scrollable(col).height(Length::Fill).into()
                }
            }
        };

        // Footer / Status Banner
        let mut footer = row![].align_y(Alignment::Center).spacing(10);
        if let Some(msg) = &self.status_msg {
            footer = footer.push(text(msg).size(12).style(text::success));
        } else {
            footer = footer.push(
                text("Esc: Close • Tab: Switch Output/Input • Drag slider to adjust")
                    .size(11)
                    .style(text::secondary),
            );
        }

        let main_layout = column![
            header,
            tab_bar,
            container(content_list).height(Length::Fill),
            footer,
        ]
        .spacing(14)
        .padding(20);

        container(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(container::bordered_box)
            .into()
    }

    fn view_device_card<'a>(&'a self, device: &'a AudioDevice, is_sink: bool) -> Element<'a, Message> {
        let icon_str = &device.icon;
        let icon_el = text(icon_str).size(22);

        let desc_el = text(&device.description).size(14);
        let name_el = text(&device.name).size(11).style(text::secondary);
        let info_col = column![desc_el, name_el].spacing(2);

        // Active Badge or Set Default Button
        let action_btn: Element<'a, Message> = if device.is_default {
            container(text("󰄬 Active").size(12))
                .style(container::bordered_box)
                .padding([4, 10])
                .into()
        } else {
            let name_clone = device.name.clone();
            button(text("Set Default").size(12))
                .style(button::secondary)
                .padding([4, 10])
                .on_press(if is_sink {
                    Message::SetDefaultSink(name_clone)
                } else {
                    Message::SetDefaultSource(name_clone)
                })
                .into()
        };

        let card_header = row![
            icon_el,
            info_col,
            space().width(Length::Fill),
            action_btn,
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        // Volume controls row
        let mute_btn_label = if device.is_muted {
            if is_sink { "󰝟 Muted" } else { "󰍭 Muted" }
        } else if is_sink {
            "󰕾 Mute"
        } else {
            "󰍬 Mute"
        };

        let name_for_mute = device.name.clone();
        let mute_btn = button(text(mute_btn_label).size(12))
            .style(if device.is_muted {
                button::danger
            } else {
                button::secondary
            })
            .padding([4, 10])
            .on_press(if is_sink {
                Message::ToggleSinkMute(name_for_mute)
            } else {
                Message::ToggleSourceMute(name_for_mute)
            });

        let name_for_down = device.name.clone();
        let down_btn = button(text("-").size(12))
            .style(button::secondary)
            .padding([3, 8])
            .on_press(if is_sink {
                Message::AdjustSinkVolume(name_for_down, -5)
            } else {
                Message::AdjustSourceVolume(name_for_down, -5)
            });

        let name_for_slider = device.name.clone();
        let vol_slider = slider(0..=100, device.volume_percent, move |val| {
            if is_sink {
                Message::SetSinkVolume(name_for_slider.clone(), val)
            } else {
                Message::SetSourceVolume(name_for_slider.clone(), val)
            }
        })
        .width(Length::Fill);

        let name_for_up = device.name.clone();
        let up_btn = button(text("+").size(12))
            .style(button::secondary)
            .padding([3, 8])
            .on_press(if is_sink {
                Message::AdjustSinkVolume(name_for_up, 5)
            } else {
                Message::AdjustSourceVolume(name_for_up, 5)
            });

        let vol_label = text(format!("{:>3}%", device.volume_percent)).size(12);

        let controls_row = row![
            mute_btn,
            down_btn,
            vol_slider,
            up_btn,
            vol_label,
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let card_content = column![card_header, controls_row].spacing(10);

        container(card_content)
            .padding(14)
            .style(container::bordered_box)
            .into()
    }
}
