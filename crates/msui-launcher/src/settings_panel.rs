use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::{
    button, column, container, row, scrollable, space, text, text_input,
};
use iced::{Alignment, Border, Color, Element, Event, Font, Length, Task as Command, Theme, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use msui_core::{
    DndManager, MsuiConfig, SystemServiceInfo, SystemServiceManager, ThemeManager,
    WallpaperManager,
};
use std::process::Command as StdCommand;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Services,
    Osd,
    Interface,
    Wallpaper,
    Notifications,
    Power,
}

pub struct SettingsPanel {
    active_tab: SettingsTab,
    search_query: String,
    services: Vec<SystemServiceInfo>,
    #[allow(dead_code)]
    is_standalone: bool,
    should_close: bool,
    osd_pos_selected: usize,
    osd_always_percentage: bool,
    osd_volume: bool,
    osd_media_vol: bool,
    osd_brightness: bool,
    osd_caps: bool,
    osd_power: bool,
    blur_enabled: bool,
    animations_enabled: bool,
    frosted_enabled: bool,
    dnd_enabled: bool,
    status_feedback: Option<String>,
    last_toggle_time: Option<Instant>,
}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    SelectTab(SettingsTab),
    SearchChanged(String),
    ToggleService(String),
    OptimizeRam,
    SelectOsdPos(usize),
    ToggleOsdPercentage,
    ToggleOsdVolume,
    ToggleOsdMediaVol,
    ToggleOsdBrightness,
    ToggleOsdCaps,
    ToggleOsdPower,
    ToggleBlur,
    ToggleAnimations,
    ToggleFrosted,
    ToggleDnd,
    SetTheme(String),
    WallpaperNext,
    WallpaperRandom,
    Tick,
    Close,
    Ignored,
}

impl TryInto<ExwlShellCustomActionWithId> for SettingsMessage {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

pub fn run_settings_panel(_config: MsuiConfig) -> Result<(), iced_exwlshell::Error> {
    let width = 960;
    let height = 660;

    application(
        SettingsPanel::new,
        SettingsPanel::namespace,
        SettingsPanel::update,
        SettingsPanel::view,
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
    .subscription(SettingsPanel::subscription)
    .run()
}

impl SettingsPanel {
    pub fn new() -> (Self, Command<SettingsMessage>) {
        Self::new_internal(true)
    }

    pub fn new_embedded() -> (Self, Command<SettingsMessage>) {
        Self::new_internal(false)
    }

    fn new_internal(is_standalone: bool) -> (Self, Command<SettingsMessage>) {
        let services = SystemServiceManager::list_services();
        let dnd_enabled = DndManager::is_dnd_enabled();

        // Check blur status from hyprctl
        let blur_enabled = StdCommand::new("hyprctl")
            .args(["getoption", "decoration:blur:enabled", "-j"])
            .output()
            .map(|o| {
                let s = String::from_utf8_lossy(&o.stdout);
                s.contains("\"bool\": true") || s.contains("true")
            })
            .unwrap_or(true);

        // Check animations status from hyprctl
        let animations_enabled = StdCommand::new("hyprctl")
            .args(["getoption", "animations:enabled", "-j"])
            .output()
            .map(|o| {
                let s = String::from_utf8_lossy(&o.stdout);
                s.contains("\"bool\": true") || s.contains("true")
            })
            .unwrap_or(true);

        let cfg = MsuiConfig::load_or_default();
        let frosted_enabled = cfg.bar.is_frosted();

        (
            Self {
                active_tab: SettingsTab::Services,
                search_query: String::new(),
                services,
                is_standalone,
                should_close: false,
                osd_pos_selected: 0,
                osd_always_percentage: false,
                osd_volume: true,
                osd_media_vol: true,
                osd_brightness: true,
                osd_caps: true,
                osd_power: true,
                blur_enabled,
                animations_enabled,
                frosted_enabled,
                dnd_enabled,
                status_feedback: None,
                last_toggle_time: None,
            },
            Command::none(),
        )
    }

    pub fn reset(&mut self) -> Command<SettingsMessage> {
        self.services = SystemServiceManager::list_services();
        self.dnd_enabled = DndManager::is_dnd_enabled();
        self.should_close = false;
        self.last_toggle_time = None;
        Command::none()
    }

    pub fn namespace() -> String {
        "msui-settings-panel".to_string()
    }

    pub fn should_close(&self) -> bool {
        self.should_close
    }

    pub fn reset_should_close(&mut self) {
        self.should_close = false;
    }

    pub fn subscription(&self) -> iced::Subscription<SettingsMessage> {
        iced::Subscription::batch(vec![
            event::listen_with(|event, _status, _window| match event {
                Event::Keyboard(KeyboardEvent::KeyPressed { key, .. }) => match key {
                    Key::Named(Named::Escape) => Some(SettingsMessage::Close),
                    _ => None,
                },
                _ => None,
            }),
            iced::time::every(Duration::from_secs(3)).map(|_| SettingsMessage::Tick),
        ])
    }

    pub fn update(&mut self, message: SettingsMessage) -> Command<SettingsMessage> {
        match message {
            SettingsMessage::SelectTab(tab) => {
                self.active_tab = tab;
            }
            SettingsMessage::SearchChanged(q) => {
                self.search_query = q;
            }
            SettingsMessage::ToggleService(id) => {
                if let Some(svc) = self.services.iter_mut().find(|s| s.id == id) {
                    svc.is_active = !svc.is_active;
                    if svc.is_radio {
                        svc.radio_state = Some(if svc.is_active {
                            "Powered On".to_string()
                        } else {
                            "Soft-Blocked (0 mW)".to_string()
                        });
                    }
                }
                self.last_toggle_time = Some(Instant::now());
                self.status_feedback = Some(format!("Toggled service {}", id));

                let id_clone = id.clone();
                std::thread::spawn(move || {
                    SystemServiceManager::toggle_service(&id_clone);
                });
            }
            SettingsMessage::OptimizeRam => {
                SystemServiceManager::optimize_ram();
                self.services = SystemServiceManager::list_services();
                self.status_feedback = Some("RAM optimization executed.".to_string());
            }
            SettingsMessage::SelectOsdPos(idx) => {
                self.osd_pos_selected = idx;
            }
            SettingsMessage::ToggleOsdPercentage => {
                self.osd_always_percentage = !self.osd_always_percentage;
            }
            SettingsMessage::ToggleOsdVolume => {
                self.osd_volume = !self.osd_volume;
            }
            SettingsMessage::ToggleOsdMediaVol => {
                self.osd_media_vol = !self.osd_media_vol;
            }
            SettingsMessage::ToggleOsdBrightness => {
                self.osd_brightness = !self.osd_brightness;
            }
            SettingsMessage::ToggleOsdCaps => {
                self.osd_caps = !self.osd_caps;
            }
            SettingsMessage::ToggleOsdPower => {
                self.osd_power = !self.osd_power;
            }
            SettingsMessage::ToggleBlur => {
                self.blur_enabled = !self.blur_enabled;
                let _ = StdCommand::new("hyprctl")
                    .args([
                        "keyword",
                        "decoration:blur:enabled",
                        if self.blur_enabled { "true" } else { "false" },
                    ])
                    .spawn();
            }
            SettingsMessage::ToggleAnimations => {
                self.animations_enabled = !self.animations_enabled;
                let _ = StdCommand::new("hyprctl")
                    .args([
                        "keyword",
                        "animations:enabled",
                        if self.animations_enabled { "true" } else { "false" },
                    ])
                    .spawn();
            }
            SettingsMessage::ToggleFrosted => {
                self.frosted_enabled = !self.frosted_enabled;
                let mut cfg = MsuiConfig::load_or_default();
                cfg.bar.transparency = if self.frosted_enabled {
                    "frosted".to_string()
                } else {
                    "solid".to_string()
                };
                let _ = cfg.save_to_default_path();
            }
            SettingsMessage::ToggleDnd => {
                self.dnd_enabled = DndManager::toggle_dnd();
            }
            SettingsMessage::SetTheme(name) => {
                ThemeManager::set_theme(&name);
            }
            SettingsMessage::WallpaperNext => {
                WallpaperManager::cycle();
            }
            SettingsMessage::WallpaperRandom => {
                WallpaperManager::random();
            }
            SettingsMessage::Tick => {
                let should_refresh = self
                    .last_toggle_time
                    .map(|t| t.elapsed() >= Duration::from_secs(4))
                    .unwrap_or(true);

                if should_refresh {
                    self.services = SystemServiceManager::list_services();
                    self.dnd_enabled = DndManager::is_dnd_enabled();
                }
            }
            SettingsMessage::Close => {
                self.should_close = true;
            }
            SettingsMessage::Ignored => {}
        }
        Command::none()
    }

    pub fn view(&self) -> Element<'_, SettingsMessage> {
        let title_bar = row![
            space().width(Length::Fixed(40.0)),
            container(
                text("Settings")
                    .size(15)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.90))
            )
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center),
            button(text("✕").size(14).color(Color::from_rgba(1.0, 1.0, 1.0, 0.70)))
                .padding([4, 10])
                .style(|_t, status| {
                    let bg = match status {
                        iced::widget::button::Status::Hovered => {
                            Color::from_rgba(1.0, 0.2, 0.2, 0.35)
                        }
                        iced::widget::button::Status::Pressed => {
                            Color::from_rgba(1.0, 0.2, 0.2, 0.50)
                        }
                        _ => Color::TRANSPARENT,
                    };
                    iced::widget::button::Style {
                        background: Some(bg.into()),
                        border: Border {
                            radius: 12.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .on_press(SettingsMessage::Close),
        ]
        .align_y(Alignment::Center)
        .padding([8, 16]);

        let body = row![self.view_sidebar(), self.view_content(),]
            .spacing(12)
            .height(Length::Fill);

        let window_container = container(
            column![title_bar, body]
                .spacing(8)
                .padding([8, 14]),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(Color::from_rgba(18.0 / 255.0, 18.0 / 255.0, 24.0 / 255.0, 0.92).into()),
            border: Border {
                color: Color::from_rgba(147.0 / 255.0, 51.0 / 255.0, 234.0 / 255.0, 0.40),
                width: 1.5,
                radius: 18.0.into(),
            },
            ..Default::default()
        });

        window_container.into()
    }

    fn view_sidebar(&self) -> Element<'_, SettingsMessage> {
        let search_box = container(
            row![
                text("🔍").size(13).color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                text_input("Search settings", &self.search_query)
                    .size(13)
                    .on_input(SettingsMessage::SearchChanged)
                    .style(|_t, _s| text_input::Style {
                        background: Color::TRANSPARENT.into(),
                        border: Border::default(),
                        icon: Color::TRANSPARENT,
                        placeholder: Color::from_rgba(1.0, 1.0, 1.0, 0.40),
                        value: Color::WHITE,
                        selection: Color::from_rgb8(139, 92, 246),
                    })
                    .width(Length::Fill),
                container(text("").size(15).color(Color::WHITE))
                    .padding([2, 6])
                    .style(|_t| container::Style {
                        background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.12).into()),
                        border: Border {
                            radius: 12.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([6, 10])
        .style(|_t| container::Style {
            background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.05).into()),
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                width: 1.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        });

        let nav_items = column![
            self.nav_button(
                SettingsTab::Services,
                "󰂯",
                "System Services",
                "Services, RAM & radios",
            ),
            self.nav_button(
                SettingsTab::Osd,
                "󰔎",
                "On-screen displays",
                "Volume, brightness, caps lock",
            ),
            self.nav_button(
                SettingsTab::Interface,
                "󰔄",
                "Interface style",
                "Blur, shadows, corners",
            ),
            self.nav_button(
                SettingsTab::Wallpaper,
                "󰏘",
                "Wallpaper & colors",
                "Theme, palette, dark mode",
            ),
            self.nav_button(
                SettingsTab::Notifications,
                "󰂚",
                "Notifications",
                "Popups, do not disturb, rules",
            ),
            self.nav_button(
                SettingsTab::Power,
                "󰌪",
                "Power & Battery",
                "Profiles, TLP, suspend",
            ),
        ]
        .spacing(6);

        let sidebar = column![search_box, scrollable(nav_items).height(Length::Fill)]
            .spacing(12)
            .width(Length::Fixed(270.0));

        sidebar.into()
    }

    fn nav_button(
        &self,
        tab: SettingsTab,
        icon: &'static str,
        title: &'static str,
        subtitle: &'static str,
    ) -> Element<'_, SettingsMessage> {
        let is_selected = self.active_tab == tab;

        let icon_box = container(text(icon).size(16).color(Color::WHITE))
            .padding([6, 8])
            .style(move |_t| container::Style {
                background: Some(if is_selected {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.20).into()
                } else {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.06).into()
                }),
                border: Border {
                    radius: 10.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let labels = column![
            text(title)
                .size(13)
                .color(if is_selected {
                    Color::WHITE
                } else {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.88)
                }),
            text(subtitle).size(10).color(if is_selected {
                Color::from_rgba(1.0, 1.0, 1.0, 0.70)
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.45)
            }),
        ]
        .spacing(2);

        let content = row![icon_box, labels]
            .spacing(10)
            .align_y(Alignment::Center)
            .width(Length::Fill);

        button(content)
            .padding([8, 10])
            .width(Length::Fill)
            .style(move |_t, status| {
                if is_selected {
                    iced::widget::button::Style {
                        background: Some(Color::from_rgb8(139, 92, 246).into()), // Lavender / Purple pill
                        border: Border {
                            radius: 12.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                } else {
                    let bg = match status {
                        iced::widget::button::Status::Hovered => {
                            Color::from_rgba(1.0, 1.0, 1.0, 0.08)
                        }
                        iced::widget::button::Status::Pressed => {
                            Color::from_rgba(1.0, 1.0, 1.0, 0.12)
                        }
                        _ => Color::TRANSPARENT,
                    };
                    iced::widget::button::Style {
                        background: Some(bg.into()),
                        border: Border {
                            radius: 12.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }
            })
            .on_press(SettingsMessage::SelectTab(tab))
            .into()
    }

    fn view_content(&self) -> Element<'_, SettingsMessage> {
        let content_view: Element<'_, SettingsMessage> = match self.active_tab {
            SettingsTab::Services => self.view_services_tab(),
            SettingsTab::Osd => self.view_osd_tab(),
            SettingsTab::Interface => self.view_interface_tab(),
            SettingsTab::Wallpaper => self.view_wallpaper_tab(),
            SettingsTab::Notifications => self.view_notifications_tab(),
            SettingsTab::Power => self.view_power_tab(),
        };

        let container_style = |_t: &Theme| container::Style {
            background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.02).into()),
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.05),
                width: 1.0,
                radius: 14.0.into(),
            },
            ..Default::default()
        };

        container(content_view)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(16)
            .style(container_style)
            .into()
    }

    /// System Services Tab
    fn view_services_tab(&self) -> Element<'_, SettingsMessage> {
        let header = column![
            text("System Services & Daemons")
                .size(19)
                .color(Color::WHITE),
            text("Control background services and hardware radios to reduce idle RAM and save battery.")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(4);

        // Calculate summary stats
        let active_count = self.services.iter().filter(|s| s.is_active).count();
        let total_ram_mb: u64 = self.services.iter().map(|s| s.ram_mb).sum();

        let summary_card = container(
            row![
                column![
                    text("Active Background Services")
                        .size(11)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.60)),
                    text(format!("{} of {} Running", active_count, self.services.len()))
                        .size(15)
                        .color(Color::WHITE),
                ]
                .spacing(2),
                space().width(Length::Fill),
                column![
                    text("RAM Consumed by Daemons")
                        .size(11)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.60)),
                    text(format!("{} MB in use", total_ram_mb))
                        .size(15)
                        .color(Color::from_rgb8(167, 139, 250)),
                ]
                .spacing(2),
                space().width(Length::Fill),
                button(
                    row![
                        text("⚡").size(14),
                        text("Optimize RAM").size(12).color(Color::WHITE)
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .padding([8, 14])
                .style(|_t, status| {
                    let bg = match status {
                        iced::widget::button::Status::Hovered => {
                            Color::from_rgba(139.0 / 255.0, 92.0 / 255.0, 246.0 / 255.0, 0.85)
                        }
                        iced::widget::button::Status::Pressed => {
                            Color::from_rgba(139.0 / 255.0, 92.0 / 255.0, 246.0 / 255.0, 0.65)
                        }
                        _ => Color::from_rgb8(139, 92, 246),
                    };
                    iced::widget::button::Style {
                        background: Some(bg.into()),
                        border: Border {
                            radius: 10.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .on_press(SettingsMessage::OptimizeRam),
            ]
            .align_y(Alignment::Center)
            .padding(12),
        )
        .style(|_t| container::Style {
            background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.04).into()),
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                width: 1.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        });

        let mut service_rows = column![].spacing(8);

        for svc in &self.services {
            let id_clone = svc.id.clone();
            let is_on = svc.is_active;

            let icon_view = container(text(&svc.icon).size(18).color(Color::WHITE))
                .padding([8, 10])
                .style(move |_t| container::Style {
                    background: Some(if is_on {
                        Color::from_rgba(139.0 / 255.0, 92.0 / 255.0, 246.0 / 255.0, 0.25).into()
                    } else {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.05).into()
                    }),
                    border: Border {
                        radius: 10.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                });

            let status_badge = if is_on {
                let txt = if svc.ram_mb > 0 {
                    format!("Active ({} MB)", svc.ram_mb)
                } else if let Some(ref r) = svc.radio_state {
                    r.clone()
                } else {
                    "Active".to_string()
                };
                container(text(txt).size(10).color(Color::from_rgb8(167, 243, 208)))
                    .padding([2, 8])
                    .style(|_t| container::Style {
                        background: Some(Color::from_rgba(16.0 / 255.0, 185.0 / 255.0, 129.0 / 255.0, 0.20).into()),
                        border: Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
            } else {
                let txt = if let Some(ref r) = svc.radio_state {
                    r.clone()
                } else {
                    "Stopped (0 MB)".to_string()
                };
                container(text(txt).size(10).color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)))
                    .padding([2, 8])
                    .style(|_t| container::Style {
                        background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.06).into()),
                        border: Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
            };

            let details = column![
                row![text(&svc.name).size(13).color(Color::WHITE), space().width(8), status_badge]
                    .align_y(Alignment::Center),
                text(&svc.description)
                    .size(10)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.45)),
            ]
            .spacing(3);

            let toggle = self.toggle_indicator(is_on);

            let card = button(
                row![icon_view, details, space().width(Length::Fill), toggle]
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .padding(10),
            )
            .width(Length::Fill)
            .style(move |_t, status| {
                let bg = match status {
                    iced::widget::button::Status::Hovered => {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.06)
                    }
                    iced::widget::button::Status::Pressed => {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.09)
                    }
                    _ => Color::from_rgba(1.0, 1.0, 1.0, 0.03),
                };
                iced::widget::button::Style {
                    background: Some(bg.into()),
                    border: Border {
                        color: if is_on {
                            Color::from_rgba(139.0 / 255.0, 92.0 / 255.0, 246.0 / 255.0, 0.35)
                        } else {
                            Color::from_rgba(1.0, 1.0, 1.0, 0.06)
                        },
                        width: 1.0,
                        radius: 12.0.into(),
                    },
                    ..Default::default()
                }
            })
            .on_press(SettingsMessage::ToggleService(id_clone));

            service_rows = service_rows.push(card);
        }

        let main_col = column![
            header,
            summary_card,
            scrollable(service_rows).height(Length::Fill)
        ]
        .spacing(12);

        main_col.into()
    }

    /// On-screen displays Tab (Replicating user's screenshot exactly!)
    fn view_osd_tab(&self) -> Element<'_, SettingsMessage> {
        let header = column![
            text("On-screen displays").size(19).color(Color::WHITE),
            text("Position and visibility of overlays and popups")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(4);

        let pos_tags = [
            "Default",
            "Volume",
            "Media volume",
            "Media playback",
            "Brightness",
            "Idle inhibitor",
            "Microphone",
            "Caps lock",
            "Touchpad",
            "Power profile",
            "Audio output switch",
        ];

        let mut chip_buttons: Vec<Element<'_, SettingsMessage>> = Vec::new();
        for (i, tag) in pos_tags.iter().enumerate() {
            let is_sel = self.osd_pos_selected == i;
            let chip = button(
                row![
                    if is_sel { text("✓ ").size(10).color(Color::WHITE) } else { text("").size(0) },
                    text(*tag).size(11).color(if is_sel {
                        Color::WHITE
                    } else {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.80)
                    })
                ]
                .align_y(Alignment::Center),
            )
            .padding([5, 10])
            .style(move |_t, status| {
                if is_sel {
                    iced::widget::button::Style {
                        background: Some(Color::from_rgb8(139, 92, 246).into()),
                        border: Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                } else {
                    let bg = match status {
                        iced::widget::button::Status::Hovered => {
                            Color::from_rgba(1.0, 1.0, 1.0, 0.10)
                        }
                        _ => Color::from_rgba(1.0, 1.0, 1.0, 0.05),
                    };
                    iced::widget::button::Style {
                        background: Some(bg.into()),
                        border: Border {
                            color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                            width: 1.0,
                            radius: 8.0.into(),
                        },
                        ..Default::default()
                    }
                }
            })
            .on_press(SettingsMessage::SelectOsdPos(i))
            .into();
            chip_buttons.push(chip);
        }

        let mut row1 = row![].spacing(6);
        let mut row2 = row![].spacing(6);
        for (i, chip) in chip_buttons.into_iter().enumerate() {
            if i < 5 {
                row1 = row1.push(chip);
            } else {
                row2 = row2.push(chip);
            }
        }
        let pos_chips = column![row1, row2].spacing(6);

        // Screen mockup preview (matching image)
        let screen_mockup = container(
            column![
                // Top slots
                row![
                    container(space().width(Length::Fixed(40.0)).height(Length::Fixed(12.0)))
                        .style(|_t| container::Style {
                            background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.15).into()),
                            border: Border { radius: 4.0.into(), ..Default::default() },
                            ..Default::default()
                        }),
                    space().width(Length::Fill),
                    container(space().width(Length::Fixed(40.0)).height(Length::Fixed(12.0)))
                        .style(|_t| container::Style {
                            background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.15).into()),
                            border: Border { radius: 4.0.into(), ..Default::default() },
                            ..Default::default()
                        }),
                    space().width(Length::Fill),
                    container(space().width(Length::Fixed(40.0)).height(Length::Fixed(12.0)))
                        .style(|_t| container::Style {
                            background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.15).into()),
                            border: Border { radius: 4.0.into(), ..Default::default() },
                            ..Default::default()
                        }),
                ]
                .align_y(Alignment::Center),
                space().height(Length::Fill),
                // Bottom slots with Purple Highlight Pill in center!
                row![
                    container(space().width(Length::Fixed(40.0)).height(Length::Fixed(12.0)))
                        .style(|_t| container::Style {
                            background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.15).into()),
                            border: Border { radius: 4.0.into(), ..Default::default() },
                            ..Default::default()
                        }),
                    space().width(Length::Fill),
                    // Active Bottom-Center OSD Highlight
                    container(space().width(Length::Fixed(60.0)).height(Length::Fixed(14.0)))
                        .style(|_t| container::Style {
                            background: Some(Color::from_rgb8(167, 139, 250).into()), // Purple highlight!
                            border: Border { radius: 5.0.into(), ..Default::default() },
                            ..Default::default()
                        }),
                    space().width(Length::Fill),
                    container(space().width(Length::Fixed(40.0)).height(Length::Fixed(12.0)))
                        .style(|_t| container::Style {
                            background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.15).into()),
                            border: Border { radius: 4.0.into(), ..Default::default() },
                            ..Default::default()
                        }),
                ]
                .align_y(Alignment::Center),
            ]
            .padding(14),
        )
        .width(Length::Fixed(240.0))
        .height(Length::Fixed(110.0))
        .style(|_t| container::Style {
            background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.04).into()),
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.10),
                width: 1.5,
                radius: 12.0.into(),
            },
            ..Default::default()
        });

        let percentage_toggle = self.setting_toggle_row(
            "Always show percentage",
            self.osd_always_percentage,
            SettingsMessage::ToggleOsdPercentage,
        );

        let show_for_section = column![
            text("Show for").size(13).color(Color::WHITE),
            self.setting_toggle_row("Volume", self.osd_volume, SettingsMessage::ToggleOsdVolume),
            self.setting_toggle_row("Media volume", self.osd_media_vol, SettingsMessage::ToggleOsdMediaVol),
            self.setting_toggle_row("Brightness", self.osd_brightness, SettingsMessage::ToggleOsdBrightness),
            self.setting_toggle_row("Caps lock", self.osd_caps, SettingsMessage::ToggleOsdCaps),
            self.setting_toggle_row("Power profile", self.osd_power, SettingsMessage::ToggleOsdPower),
        ]
        .spacing(6);

        let main_col = column![
            header,
            column![
                text("Position").size(12).color(Color::from_rgba(1.0, 1.0, 1.0, 0.80)),
                text("Bottom Center").size(10).color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                pos_chips,
            ]
            .spacing(4),
            row![
                screen_mockup,
                space().width(16),
                column![percentage_toggle, show_for_section].width(Length::Fill),
            ]
            .spacing(12),
        ]
        .spacing(12);

        scrollable(main_col).height(Length::Fill).into()
    }

    /// Interface Style Tab
    fn view_interface_tab(&self) -> Element<'_, SettingsMessage> {
        let header = column![
            text("Interface style").size(19).color(Color::WHITE),
            text("Hyprland compositor blur, shadows, corners, and window animations")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(4);

        let rows = column![
            self.setting_toggle_row(
                "Hyprland GPU Blur (Kawase)",
                self.blur_enabled,
                SettingsMessage::ToggleBlur,
            ),
            self.setting_toggle_row(
                "Desktop Smooth Animations",
                self.animations_enabled,
                SettingsMessage::ToggleAnimations,
            ),
            self.setting_toggle_row(
                "Frosted Acrylic Glass (msUI Shell)",
                self.frosted_enabled,
                SettingsMessage::ToggleFrosted,
            ),
        ]
        .spacing(8);

        column![header, rows].spacing(16).into()
    }

    /// Wallpaper & Colors Tab
    fn view_wallpaper_tab(&self) -> Element<'_, SettingsMessage> {
        let header = column![
            text("Wallpaper & colors").size(19).color(Color::WHITE),
            text("Theme palettes, wallpaper cycling, and desktop styling")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(4);

        let themes = [
            ("Catppuccin Mocha", "catppuccin-mocha"),
            ("Tokyo Night", "tokyo-night"),
            ("Nord Frost", "nord"),
            ("Rosé Pine", "rose-pine"),
            ("Dracula Dark", "dracula"),
        ];

        let mut theme_buttons: Vec<Element<'_, SettingsMessage>> = Vec::new();
        for (name, id) in themes {
            let id_str = id.to_string();
            let chip = button(text(name).size(11).color(Color::WHITE))
                .padding([8, 12])
                .style(|_t, status| {
                    let bg = match status {
                        iced::widget::button::Status::Hovered => {
                            Color::from_rgba(139.0 / 255.0, 92.0 / 255.0, 246.0 / 255.0, 0.50)
                        }
                        _ => Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                    };
                    iced::widget::button::Style {
                        background: Some(bg.into()),
                        border: Border {
                            radius: 10.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .on_press(SettingsMessage::SetTheme(id_str))
                .into();
            theme_buttons.push(chip);
        }

        let mut theme_row = row![].spacing(8);
        for chip in theme_buttons {
            theme_row = theme_row.push(chip);
        }
        let theme_chips = theme_row;

        let wall_controls = row![
            button(text("Next Wallpaper").size(12).color(Color::WHITE))
                .padding([8, 14])
                .style(|_t, _s| iced::widget::button::Style {
                    background: Some(Color::from_rgb8(139, 92, 246).into()),
                    border: Border { radius: 10.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .on_press(SettingsMessage::WallpaperNext),
            button(text("Random Wallpaper").size(12).color(Color::WHITE))
                .padding([8, 14])
                .style(|_t, _s| iced::widget::button::Style {
                    background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.10).into()),
                    border: Border { radius: 10.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .on_press(SettingsMessage::WallpaperRandom),
        ]
        .spacing(10);

        column![
            header,
            column![text("Theme Presets").size(13).color(Color::WHITE), theme_chips].spacing(8),
            column![text("Wallpaper Switcher").size(13).color(Color::WHITE), wall_controls].spacing(8),
        ]
        .spacing(16)
        .into()
    }

    /// Notifications Tab
    fn view_notifications_tab(&self) -> Element<'_, SettingsMessage> {
        let header = column![
            text("Notifications & Focus").size(19).color(Color::WHITE),
            text("Do Not Disturb, popup previews, and notification rules")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(4);

        let dnd_row = self.setting_toggle_row(
            "Do Not Disturb (Silence All Popups)",
            self.dnd_enabled,
            SettingsMessage::ToggleDnd,
        );

        column![header, dnd_row].spacing(16).into()
    }

    /// Power & Battery Tab
    fn view_power_tab(&self) -> Element<'_, SettingsMessage> {
        let header = column![
            text("Power & Battery").size(19).color(Color::WHITE),
            text("Energy modes, TLP daemon, and suspend timeout")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(4);

        let tlp_active = self
            .services
            .iter()
            .find(|s| s.id == "tlp")
            .map(|s| s.is_active)
            .unwrap_or(false);

        let tlp_row = self.setting_toggle_row(
            "TLP Background Power Optimizer",
            tlp_active,
            SettingsMessage::ToggleService("tlp".to_string()),
        );

        column![header, tlp_row].spacing(16).into()
    }

    /// Helper for setting toggle rows (Entire row is a clickable button with hover state)
    fn setting_toggle_row(
        &self,
        label: &'static str,
        is_on: bool,
        msg: SettingsMessage,
    ) -> Element<'_, SettingsMessage> {
        let toggle = self.toggle_indicator(is_on);

        button(
            row![
                text(label).size(12).color(Color::WHITE),
                space().width(Length::Fill),
                toggle,
            ]
            .align_y(Alignment::Center)
            .padding([10, 14]),
        )
        .width(Length::Fill)
        .style(move |_t, status| {
            let bg = match status {
                iced::widget::button::Status::Hovered => {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.06)
                }
                iced::widget::button::Status::Pressed => {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.09)
                }
                _ => Color::from_rgba(1.0, 1.0, 1.0, 0.03),
            };
            iced::widget::button::Style {
                background: Some(bg.into()),
                border: Border {
                    color: if is_on {
                        Color::from_rgba(139.0 / 255.0, 92.0 / 255.0, 246.0 / 255.0, 0.35)
                    } else {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.06)
                    },
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            }
        })
        .on_press(msg)
        .into()
    }

    /// Custom Switch Toggle Indicator matching screenshot (Purple pill with white checkmark when ON)
    fn toggle_indicator(&self, is_on: bool) -> Element<'_, SettingsMessage> {
        let content = if is_on {
            row![
                space().width(Length::Fill),
                container(text("✓").size(11).color(Color::WHITE))
                    .padding([2, 5]),
            ]
            .align_y(Alignment::Center)
        } else {
            row![
                container(text("○").size(11).color(Color::from_rgba(1.0, 1.0, 1.0, 0.60)))
                    .padding([2, 4]),
                space().width(Length::Fill),
            ]
            .align_y(Alignment::Center)
        };

        container(content)
            .width(Length::Fixed(46.0))
            .height(Length::Fixed(24.0))
            .style(move |_t| {
                if is_on {
                    container::Style {
                        background: Some(Color::from_rgb8(139, 92, 246).into()), // Purple
                        border: Border {
                            radius: 12.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                } else {
                    container::Style {
                        background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.15).into()),
                        border: Border {
                            radius: 12.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }
            })
            .into()
    }
}
