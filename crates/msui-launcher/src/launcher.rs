use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::operation::{focus, snap_to, RelativeOffset};
use iced::widget::{button, column, container, image, row, scrollable, space, svg, text, text_input};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, Theme, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use crate::modal_style::modal_card_style;
use msui_core::{DesktopApp, MsuiConfig, scan_desktop_apps};
use std::sync::LazyLock;

static INPUT_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);
static SCROLL_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);

pub fn run_launcher(config: MsuiConfig) -> Result<(), iced_exwlshell::Error> {
    let width = config.launcher.width;
    let height = config.launcher.height;
    let cfg_clone = config.clone();

    application(
        move || Launcher::new(cfg_clone.clone()),
        Launcher::namespace,
        Launcher::update,
        Launcher::view,
    )
    .theme(Theme::CatppuccinMocha)
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(width, height),
        anchor: Anchor::empty(), // Center of the screen
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    })
    .subscription(Launcher::subscription)
    .run()
}

pub struct Launcher {
    config: MsuiConfig,
    query: String,
    all_apps: Vec<DesktopApp>,
    filtered: Vec<DesktopApp>,
    selected_idx: Option<usize>,
    matcher: SkimMatcherV2,
    is_standalone: bool,
    should_close: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    LaunchApp(usize),
    SelectPrevious,
    SelectNext,
    EnsureFocus,
    Close,
    IcedEvent(Event),
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

impl Launcher {
    pub fn new(config: MsuiConfig) -> (Self, Command<Message>) {
        Self::new_internal(config, true)
    }

    pub fn new_embedded(config: MsuiConfig) -> (Self, Command<Message>) {
        Self::new_internal(config, false)
    }

    fn new_internal(config: MsuiConfig, is_standalone: bool) -> (Self, Command<Message>) {
        let all_apps = scan_desktop_apps();
        let filtered = all_apps.iter().take(config.launcher.max_results).cloned().collect();

        (
            Self {
                config,
                query: String::new(),
                all_apps,
                filtered,
                selected_idx: None,
                matcher: SkimMatcherV2::default(),
                is_standalone,
                should_close: false,
            },
            Command::batch([
                focus(INPUT_ID.clone()),
                Command::perform(tokio::time::sleep(std::time::Duration::from_millis(20)), |_| Message::EnsureFocus),
                Command::perform(tokio::time::sleep(std::time::Duration::from_millis(50)), |_| Message::EnsureFocus),
                Command::perform(tokio::time::sleep(std::time::Duration::from_millis(100)), |_| Message::EnsureFocus),
                Command::perform(tokio::time::sleep(std::time::Duration::from_millis(180)), |_| Message::EnsureFocus),
                Command::perform(tokio::time::sleep(std::time::Duration::from_millis(300)), |_| Message::EnsureFocus),
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
        self.all_apps = scan_desktop_apps();
        self.update_filtered();
        self.selected_idx = None;
        self.should_close = false;
        Command::batch([
            focus(INPUT_ID.clone()),
            Command::perform(tokio::time::sleep(std::time::Duration::from_millis(20)), |_| Message::EnsureFocus),
            Command::perform(tokio::time::sleep(std::time::Duration::from_millis(50)), |_| Message::EnsureFocus),
            Command::perform(tokio::time::sleep(std::time::Duration::from_millis(100)), |_| Message::EnsureFocus),
            Command::perform(tokio::time::sleep(std::time::Duration::from_millis(180)), |_| Message::EnsureFocus),
            Command::perform(tokio::time::sleep(std::time::Duration::from_millis(300)), |_| Message::EnsureFocus),
        ])
    }

    pub fn namespace() -> String {
        String::from("msui-launcher")
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        event::listen().map(Message::IcedEvent)
    }

    fn update_filtered(&mut self) {
        if self.query.trim().is_empty() {
            self.filtered = self
                .all_apps
                .iter()
                .take(self.config.launcher.max_results)
                .cloned()
                .collect();
            self.selected_idx = None;
            return;
        }

        let mut scored: Vec<(i64, DesktopApp)> = self
            .all_apps
            .iter()
            .filter_map(|app| {
                let score_name = self.matcher.fuzzy_match(&app.name, &self.query);
                let score_comment = self.matcher.fuzzy_match(&app.comment, &self.query);
                let score_id = self.matcher.fuzzy_match(&app.id, &self.query);
                let score_cats = app.categories.as_ref().and_then(|cats| {
                    cats.iter().filter_map(|c| self.matcher.fuzzy_match(c, &self.query)).max()
                });
                let mut best = score_name;
                if let Some(s) = score_comment {
                    best = Some(best.map_or(s, |b| b.max(s)));
                }
                if let Some(s) = score_id {
                    best = Some(best.map_or(s, |b| b.max(s)));
                }
                if let Some(s) = score_cats {
                    best = Some(best.map_or(s, |b| b.max(s)));
                }
                Some((best?, app.clone()))
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));

        self.filtered = scored
            .into_iter()
            .take(self.config.launcher.max_results)
            .map(|(_, app)| app)
            .collect();

        self.selected_idx = None;
    }

    fn scroll_to_selected(&self) -> Command<Message> {
        if let Some(idx) = self.selected_idx {
            if self.filtered.len() > 1 {
                let ratio = idx as f32 / (self.filtered.len() - 1) as f32;
                snap_to(SCROLL_ID.clone(), RelativeOffset { x: 0.0, y: ratio })
            } else {
                snap_to(SCROLL_ID.clone(), RelativeOffset { x: 0.0, y: 0.0 })
            }
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
                    self.selected_idx = match self.selected_idx {
                        None => Some(0),
                        Some(idx) => Some((idx + 1) % self.filtered.len()),
                    };
                    return self.scroll_to_selected();
                }
            }
            Message::SelectPrevious => {
                if !self.filtered.is_empty() {
                    self.selected_idx = match self.selected_idx {
                        None => Some(self.filtered.len() - 1),
                        Some(0) => None,
                        Some(idx) => Some(idx - 1),
                    };
                    return self.scroll_to_selected();
                }
            }
            Message::LaunchApp(idx) => {
                if let Some(app) = self.filtered.get(idx) {
                    let clients = msui_core::HyprlandIpc::get_clients();
                    if let Some(client) = app.find_running_client(&clients) {
                        msui_core::HyprlandIpc::activate_client(client);
                    } else {
                        let tray_items = msui_core::get_tray_items();
                        if let Some(tray) = app.find_running_tray_item(&tray_items) {
                            msui_core::activate_tray_item_blocking(&tray.service, &tray.path);
                        }
                        let _ = app.launch();
                    }
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
            Message::IcedEvent(Event::Keyboard(KeyboardEvent::KeyPressed { key, modifiers, .. })) => {
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
                            let next = match self.selected_idx {
                                None => 5.min(self.filtered.len() - 1),
                                Some(idx) => (idx + 5).min(self.filtered.len() - 1),
                            };
                            self.selected_idx = Some(next);
                            return self.scroll_to_selected();
                        }
                    }
                    Key::Named(Named::PageUp) => {
                        if !self.filtered.is_empty() {
                            self.selected_idx = match self.selected_idx {
                                None | Some(0..=5) => None,
                                Some(idx) => Some(idx - 5),
                            };
                            return self.scroll_to_selected();
                        }
                    }
                    Key::Named(Named::Home) => {
                        self.selected_idx = None;
                        return self.scroll_to_selected();
                    }
                    Key::Named(Named::End) => {
                        if !self.filtered.is_empty() {
                            self.selected_idx = Some(self.filtered.len() - 1);
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
                        if let Some(idx) = self.selected_idx {
                            return Command::done(Message::LaunchApp(idx));
                        } else if !self.filtered.is_empty() {
                            return Command::done(Message::LaunchApp(0));
                        }
                    }
                    _ => {
                        // Any typing event ensures the search box is focused!
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
        let placeholder = if self.config.launcher.placeholder.is_empty() {
            "󰍉 Type to search applications...".to_string()
        } else {
            format!("󰍉 {}", self.config.launcher.placeholder)
        };
        let input = text_input(&placeholder, &self.query)
            .id(INPUT_ID.clone())
            .on_input(Message::QueryChanged)
            .padding(12)
            .size(16);

        let mut app_list = column![].spacing(6);

        for (idx, app) in self.filtered.iter().enumerate() {
            let is_selected = self.selected_idx == Some(idx);

            let icon_element: Element<'_, Message> = match &app.icon {
                Some(p) if p.extension().is_some_and(|e| e == "svg") => {
                    svg(svg::Handle::from_path(p))
                        .width(Length::Fixed(32.0))
                        .height(Length::Fixed(32.0))
                        .into()
                }
                Some(p) if p.extension().is_some_and(|e| e == "png") => {
                    image(image::Handle::from_path(p))
                        .width(Length::Fixed(32.0))
                        .height(Length::Fixed(32.0))
                        .into()
                }
                _ => text("󰵆 ").size(24).into(),
            };

            let title_text = text(&app.name).size(15);
            let desc = if app.comment.is_empty() {
                format!("Launch {}", &app.name)
            } else {
                app.comment.clone()
            };
            let desc_text = text(desc).size(11);

            let label_col = column![title_text, desc_text].spacing(2);

            let mut app_row = row![icon_element, label_col]
                .spacing(12)
                .align_y(Alignment::Center);

            if is_selected {
                app_row = app_row.push(space().width(Length::Fill));
                app_row = app_row.push(text("⏎ Launch").size(11));
            }

            let btn_style = if is_selected {
                button::primary
            } else {
                button::secondary
            };

            let app_btn = button(app_row)
                .width(Length::Fill)
                .style(btn_style)
                .padding([6, 10])
                .on_press(Message::LaunchApp(idx));

            app_list = app_list.push(app_btn);
        }

        let footer = row![
            text("Type to search").size(11).color([0.65, 0.67, 0.78]),
            text("•").size(11).color([0.45, 0.47, 0.58]),
            text("↓ Navigate list").size(11).color([0.65, 0.67, 0.78]),
            text("•").size(11).color([0.45, 0.47, 0.58]),
            text("Enter Launch").size(11).color([0.54, 0.71, 0.98]),
            text("•").size(11).color([0.45, 0.47, 0.58]),
            text("Esc Close").size(11).color([0.65, 0.67, 0.78]),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let container_content = column![
            input,
            scrollable(app_list).id(SCROLL_ID.clone()).height(Length::Fill),
            footer,
        ]
        .spacing(10)
        .padding(14);

        container(container_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(modal_card_style(self.config.bar.is_frosted()))
            .into()
    }
}
