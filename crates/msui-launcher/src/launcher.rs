use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::operation::focus;
use iced::widget::{button, column, container, image, row, scrollable, space, svg, text, text_input};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, Theme, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use msui_core::{DesktopApp, MsuiConfig, scan_desktop_apps};
use std::sync::LazyLock;

static INPUT_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);

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
    selected_idx: usize,
    matcher: SkimMatcherV2,
}

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    LaunchApp(usize),
    SelectPrevious,
    SelectNext,
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
    fn new(config: MsuiConfig) -> (Self, Command<Message>) {
        let all_apps = scan_desktop_apps();
        let filtered = all_apps.iter().take(config.launcher.max_results).cloned().collect();

        (
            Self {
                config,
                query: String::new(),
                all_apps,
                filtered,
                selected_idx: 0,
                matcher: SkimMatcherV2::default(),
            },
            focus(INPUT_ID.clone()),
        )
    }

    fn namespace() -> String {
        String::from("msui-launcher")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
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
            self.selected_idx = 0;
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
                    std::process::exit(0);
                }
            }
            Message::Close => {
                std::process::exit(0);
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
                    Key::Named(Named::Enter) => {
                        if !self.filtered.is_empty() {
                            return Command::done(Message::LaunchApp(self.selected_idx));
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
        let placeholder = format!("󰍉 {}", self.config.launcher.placeholder);
        let input = text_input(&placeholder, &self.query)
            .id(INPUT_ID.clone())
            .on_input(Message::QueryChanged)
            .padding(12)
            .size(16);

        let mut app_list = column![].spacing(6);

        for (idx, app) in self.filtered.iter().enumerate() {
            let is_selected = idx == self.selected_idx;

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
            text("↑↓ Navigate").size(11),
            text("•").size(11),
            text("Enter Launch").size(11),
            text("•").size(11),
            text("Esc Close").size(11),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let container_content = column![
            input,
            scrollable(app_list).height(Length::Fill),
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
