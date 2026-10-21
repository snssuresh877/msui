use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key, Modifiers};
use iced::widget::{button, column, container, image, row, scrollable, space, svg, text, text_input};
use iced::{Alignment, Element, Event, Font, Length, Task as Command, Theme, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use msui_core::{
    find_icon_for_class, scan_desktop_apps, DesktopApp, HyprlandClient, HyprlandIpc, MsuiConfig,
    TrayItem,
};
use std::path::PathBuf;
use std::sync::LazyLock;

static INPUT_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);

pub fn run_app_drawer(config: MsuiConfig) -> Result<(), iced_exwlshell::Error> {
    let width = 780;
    let height = 620;
    let cfg_clone = config.clone();

    application(
        move || AppDrawer::new(cfg_clone.clone()),
        AppDrawer::namespace,
        AppDrawer::update,
        AppDrawer::view,
    )
    .theme(Theme::CatppuccinMocha)
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
    .subscription(AppDrawer::subscription)
    .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawerTab {
    Running,
    Installed,
    All,
}

#[derive(Debug, Clone)]
pub enum DrawerItem {
    Window {
        client: HyprlandClient,
        icon: Option<PathBuf>,
    },
    Tray {
        item: TrayItem,
        icon: Option<PathBuf>,
    },
    App {
        app: DesktopApp,
    },
}

pub struct AppDrawer {
    _config: MsuiConfig,
    tab: DrawerTab,
    query: String,
    clients: Vec<HyprlandClient>,
    tray_items: Vec<TrayItem>,
    all_apps: Vec<DesktopApp>,
    filtered: Vec<DrawerItem>,
    selected_idx: usize,
    matcher: SkimMatcherV2,
}

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    SetTab(DrawerTab),
    SelectPrevious,
    SelectNext,
    ActivateItem(usize),
    ActivateTray(String, String),
    FocusWindow(String),
    CloseWindow(String),
    LaunchApp(usize),
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

impl AppDrawer {
    pub fn new(config: MsuiConfig) -> (Self, Command<Message>) {
        let clients = HyprlandIpc::get_clients();
        let tray_items = msui_core::get_tray_items();
        let all_apps = scan_desktop_apps();

        let mut drawer = Self {
            _config: config,
            tab: DrawerTab::Running,
            query: String::new(),
            clients,
            tray_items,
            all_apps,
            filtered: Vec::new(),
            selected_idx: 0,
            matcher: SkimMatcherV2::default(),
        };

        drawer.rebuild_filtered();

        (drawer, iced::widget::operation::focus(INPUT_ID.clone()))
    }


    pub fn namespace() -> String {
        "msui-app-drawer".to_string()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        event::listen().map(Message::IcedEvent)
    }

    fn rebuild_filtered(&mut self) {
        let q = self.query.trim().to_lowercase();
        let mut items = Vec::new();

        // 1. Windows (if tab is Running or All)
        if self.tab == DrawerTab::Running || self.tab == DrawerTab::All {
            for client in &self.clients {
                let matches = if q.is_empty() {
                    true
                } else {
                    let score_title = self.matcher.fuzzy_match(&client.title, &q).is_some();
                    let score_class = self.matcher.fuzzy_match(&client.class, &q).is_some();
                    let score_init = self.matcher.fuzzy_match(&client.initial_class, &q).is_some();
                    score_title || score_class || score_init
                };

                if matches {
                    let icon = find_icon_for_class(&client.class)
                        .or_else(|| find_icon_for_class(&client.initial_class));
                    items.push(DrawerItem::Window {
                        client: client.clone(),
                        icon,
                    });
                }
            }

            // 1b. Background Tray Services (WeChat, Proton VPN, Steam, etc.)
            for tray_item in &self.tray_items {
                let matches = if q.is_empty() {
                    true
                } else {
                    self.matcher.fuzzy_match(&tray_item.title, &q).is_some()
                        || self.matcher.fuzzy_match(&tray_item.id, &q).is_some()
                        || self.matcher.fuzzy_match(&tray_item.tooltip, &q).is_some()
                };

                if matches {
                    let icon = find_icon_for_class(&tray_item.id)
                        .or_else(|| find_icon_for_class(&tray_item.title))
                        .or_else(|| {
                            if !tray_item.icon_name.is_empty() {
                                find_icon_for_class(&tray_item.icon_name)
                            } else {
                                None
                            }
                        });
                    items.push(DrawerItem::Tray {
                        item: tray_item.clone(),
                        icon,
                    });
                }
            }
        }


        // 2. Apps (if tab is Installed or All, OR if tab is Running and search query is non-empty)
        if self.tab == DrawerTab::Installed || self.tab == DrawerTab::All || (self.tab == DrawerTab::Running && !q.is_empty()) {
            for app in &self.all_apps {
                let matches = if q.is_empty() {
                    true
                } else {
                    self.matcher.fuzzy_match(&app.name, &q).is_some()
                        || self.matcher.fuzzy_match(&app.comment, &q).is_some()
                        || self.matcher.fuzzy_match(&app.id, &q).is_some()
                };

                if matches {
                    // On Running tab with search query, if there's already an active window or tray item in items for this app, skip duplicate
                    if self.tab == DrawerTab::Running && !q.is_empty() {
                        let already_in_windows = items.iter().any(|item| match item {
                            DrawerItem::Window { client, .. } => {
                                client.class.eq_ignore_ascii_case(&app.id)
                                    || client.initial_class.eq_ignore_ascii_case(&app.id)
                                    || client.class.eq_ignore_ascii_case(&app.name)
                                    || app.startup_wm_class.as_ref().is_some_and(|w| {
                                        client.class.eq_ignore_ascii_case(w)
                                            || client.initial_class.eq_ignore_ascii_case(w)
                                    })
                            }
                            DrawerItem::Tray { item, .. } => {
                                item.id.eq_ignore_ascii_case(&app.id)
                                    || item.title.eq_ignore_ascii_case(&app.name)
                            }
                            _ => false,
                        });
                        if already_in_windows {
                            continue;
                        }
                    }
                    items.push(DrawerItem::App { app: app.clone() });
                }
            }
        }

        self.filtered = items;
        if self.selected_idx >= self.filtered.len() {
            self.selected_idx = 0;
        }
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::QueryChanged(new_query) => {
                self.query = new_query;
                self.selected_idx = 0;
                self.rebuild_filtered();
            }
            Message::SetTab(tab) => {
                self.tab = tab;
                self.selected_idx = 0;
                self.rebuild_filtered();
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
            Message::SelectNext => {
                if !self.filtered.is_empty() {
                    self.selected_idx = (self.selected_idx + 1) % self.filtered.len();
                }
            }
            Message::ActivateItem(idx) => {
                if let Some(item) = self.filtered.get(idx) {
                    match item {
                        DrawerItem::Window { client, .. } => {
                            HyprlandIpc::activate_client(client);
                            std::process::exit(0);
                        }
                        DrawerItem::Tray { item, .. } => {
                            msui_core::activate_tray_item(&item.service, &item.path);
                            std::process::exit(0);
                        }
                        DrawerItem::App { app } => {
                            if let Some(client) = app.find_running_client(&self.clients) {
                                HyprlandIpc::activate_client(client);
                            } else if let Some(tray) = app.find_running_tray_item(&self.tray_items) {
                                msui_core::activate_tray_item(&tray.service, &tray.path);
                            } else {
                                let _ = app.launch();
                            }
                            std::process::exit(0);
                        }
                    }
                }
            }
            Message::ActivateTray(service, path) => {
                msui_core::activate_tray_item(&service, &path);
                std::process::exit(0);
            }
            Message::FocusWindow(addr) => {
                HyprlandIpc::focus_window(&addr);
                std::process::exit(0);
            }
            Message::CloseWindow(addr) => {
                HyprlandIpc::close_window(&addr);
                // Small sleep to allow Hyprland to process close
                std::thread::sleep(std::time::Duration::from_millis(150));
                return Command::done(Message::Refresh);
            }
            Message::LaunchApp(idx) => {
                if let Some(item) = self.filtered.get(idx) {
                    match item {
                        DrawerItem::App { app } => {
                            let _ = app.launch();
                            std::process::exit(0);
                        }
                        DrawerItem::Tray { item, .. } => {
                            msui_core::activate_tray_item(&item.service, &item.path);
                            std::process::exit(0);
                        }
                        DrawerItem::Window { client, .. } => {
                            if let Some(app) = self.all_apps.iter().find(|a| {
                                a.name.eq_ignore_ascii_case(&client.class)
                                    || a.id.eq_ignore_ascii_case(&client.class)
                                    || a.id.eq_ignore_ascii_case(&client.initial_class)
                            }) {
                                let _ = app.launch();
                            } else {
                                let _ = std::process::Command::new(&client.initial_class)
                                    .stdin(std::process::Stdio::null())
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .spawn();
                            }
                            std::process::exit(0);
                        }
                    }
                }
            }
            Message::Refresh => {
                self.clients = HyprlandIpc::get_clients();
                self.tray_items = msui_core::get_tray_items();
                self.rebuild_filtered();
            }
            Message::Close => {
                std::process::exit(0);
            }
            Message::IcedEvent(Event::Keyboard(KeyboardEvent::KeyPressed {
                key,
                modifiers,
                ..
            })) => {
                match key {
                    Key::Named(Named::Escape) => {
                        return Command::done(Message::Close);
                    }
                    Key::Named(Named::Tab) => {
                        let next_tab = if modifiers.contains(Modifiers::SHIFT) {
                            match self.tab {
                                DrawerTab::Running => DrawerTab::All,
                                DrawerTab::All => DrawerTab::Installed,
                                DrawerTab::Installed => DrawerTab::Running,
                            }
                        } else {
                            match self.tab {
                                DrawerTab::Running => DrawerTab::Installed,
                                DrawerTab::Installed => DrawerTab::All,
                                DrawerTab::All => DrawerTab::Running,
                            }
                        };
                        return Command::done(Message::SetTab(next_tab));
                    }
                    Key::Named(Named::ArrowRight) if modifiers.contains(Modifiers::CTRL) || modifiers.contains(Modifiers::ALT) => {
                        let next_tab = match self.tab {
                            DrawerTab::Running => DrawerTab::Installed,
                            DrawerTab::Installed => DrawerTab::All,
                            DrawerTab::All => DrawerTab::Running,
                        };
                        return Command::done(Message::SetTab(next_tab));
                    }
                    Key::Named(Named::ArrowLeft) if modifiers.contains(Modifiers::CTRL) || modifiers.contains(Modifiers::ALT) => {
                        let prev_tab = match self.tab {
                            DrawerTab::Running => DrawerTab::All,
                            DrawerTab::All => DrawerTab::Installed,
                            DrawerTab::Installed => DrawerTab::Running,
                        };
                        return Command::done(Message::SetTab(prev_tab));
                    }
                    Key::Named(Named::ArrowDown) => {
                        return Command::done(Message::SelectNext);
                    }
                    Key::Named(Named::ArrowUp) => {
                        return Command::done(Message::SelectPrevious);
                    }
                    Key::Named(Named::Enter) => {
                        if !self.filtered.is_empty() {
                            return Command::done(Message::ActivateItem(self.selected_idx));
                        }
                    }
                    Key::Named(Named::Delete) => {
                        if let Some(DrawerItem::Window { client, .. }) = self.filtered.get(self.selected_idx) {
                            return Command::done(Message::CloseWindow(client.address.clone()));
                        }
                    }
                    Key::Character(ref c) if (c == "w" || c == "W") && modifiers.contains(Modifiers::CTRL) => {
                        if let Some(DrawerItem::Window { client, .. }) = self.filtered.get(self.selected_idx) {
                            return Command::done(Message::CloseWindow(client.address.clone()));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Command::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        // --- 1. Header with Title, Tabs, Refresh, Close ---
        let title_icon = text("󰣆").size(20);
        let title_label = text("App Drawer & Tasks").size(16);
        let header_title = row![title_icon, title_label].spacing(8).align_y(Alignment::Center);

        // Tab buttons
        let running_count = self.clients.len() + self.tray_items.len();
        let running_label = format!("󰖯 Running ({})", running_count);
        let btn_running = button(text(running_label).size(12))
            .style(if self.tab == DrawerTab::Running { button::primary } else { button::secondary })
            .padding([4, 10])
            .on_press(Message::SetTab(DrawerTab::Running));

        let btn_apps = button(text("󰀻 Installed Apps").size(12))
            .style(if self.tab == DrawerTab::Installed { button::primary } else { button::secondary })
            .padding([4, 10])
            .on_press(Message::SetTab(DrawerTab::Installed));

        let btn_all = button(text("󰣆 All").size(12))
            .style(if self.tab == DrawerTab::All { button::primary } else { button::secondary })
            .padding([4, 10])
            .on_press(Message::SetTab(DrawerTab::All));

        let refresh_btn = button(text("󰑐 Refresh").size(12))
            .style(button::secondary)
            .padding([4, 8])
            .on_press(Message::Refresh);

        let close_btn = button(text("✕").size(12))
            .style(button::danger)
            .padding([4, 8])
            .on_press(Message::Close);

        let header_row = row![
            header_title,
            space().width(Length::Fill),
            btn_running,
            btn_apps,
            btn_all,
            refresh_btn,
            close_btn,
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        // --- 2. Search Input ---
        let placeholder = match self.tab {
            DrawerTab::Running => "󰍉 Search running windows by title or class...",
            DrawerTab::Installed => "󰍉 Search installed applications...",
            DrawerTab::All => "󰍉 Search running tasks and desktop apps...",
        };
        let input = text_input(placeholder, &self.query)
            .id(INPUT_ID.clone())
            .on_input(Message::QueryChanged)
            .padding(10)
            .size(15);

        // --- 3. Items List ---
        let mut list_col = column![].spacing(6);

        if self.filtered.is_empty() {
            let empty_msg = if self.tab == DrawerTab::Running && self.clients.is_empty() {
                "No active or running windows found on any workspace."
            } else {
                "No applications or tasks match your search query."
            };
            list_col = list_col.push(
                container(text(empty_msg).size(13))
                    .padding(24)
                    .center_x(Length::Fill),
            );
        } else {
            for (idx, item) in self.filtered.iter().enumerate() {
                let is_selected = idx == self.selected_idx;

                match item {
                    DrawerItem::Window { client, icon } => {
                        let icon_elem: Element<'_, Message> = match icon {
                            Some(p) if p.extension().is_some_and(|e| e == "svg") => {
                                svg(svg::Handle::from_path(p))
                                    .width(Length::Fixed(28.0))
                                    .height(Length::Fixed(28.0))
                                    .into()
                            }
                            Some(p) if p.extension().is_some_and(|e| e == "png") => {
                                image(image::Handle::from_path(p))
                                    .width(Length::Fixed(28.0))
                                    .height(Length::Fixed(28.0))
                                    .into()
                            }
                            _ => text("󰖯").size(24).into(),
                        };

                        let title_str = if client.title.is_empty() {
                            client.class.clone()
                        } else if client.title.len() > 50 {
                            format!("{}…", &client.title[..50])
                        } else {
                            client.title.clone()
                        };
                        let win_title = text(title_str).size(14);

                        let win_meta = text(format!(
                            "Class: {} • PID: {}",
                            client.class, client.pid
                        ))
                        .size(11);

                        let info_col = column![win_title, win_meta].spacing(2);

                        // Badges
                        let is_minimized = client.workspace.id < 0 || client.workspace.name.starts_with("special:");
                        let ws_badge = if is_minimized {
                            text("󱊖 Minimized").size(11)
                        } else {
                            text(format!("󰮯 WS {}", client.workspace.id)).size(11)
                        };
                        let is_focused = client.focus_history_id == 0;
                        let active_badge = if is_focused {
                            Some(text("󰄬 Active").size(11))
                        } else {
                            None
                        };

                        let focus_label = if is_minimized {
                            "󱊖 Restore"
                        } else {
                            "󰁔 Switch"
                        };

                        let focus_btn = button(text(focus_label).size(11))
                            .style(button::primary)
                            .padding([3, 8])
                            .on_press(Message::FocusWindow(client.address.clone()));

                        let relaunch_btn = button(text("󰐊 Relaunch").size(11))
                            .style(button::secondary)
                            .padding([3, 8])
                            .on_press(Message::LaunchApp(idx));

                        let close_win_btn = button(text("✕ Close").size(11))
                            .style(button::danger)
                            .padding([3, 8])
                            .on_press(Message::CloseWindow(client.address.clone()));

                        let mut item_row = row![icon_elem, info_col]
                            .spacing(12)
                            .align_y(Alignment::Center);

                        item_row = item_row.push(space().width(Length::Fill));
                        item_row = item_row.push(ws_badge);
                        if let Some(badge) = active_badge {
                            item_row = item_row.push(badge);
                        }
                        item_row = item_row.push(focus_btn);
                        item_row = item_row.push(relaunch_btn);
                        item_row = item_row.push(close_win_btn);

                        let item_container = button(item_row)
                            .width(Length::Fill)
                            .style(if is_selected {
                                button::primary
                            } else {
                                button::secondary
                            })
                            .padding([6, 10])
                            .on_press(Message::ActivateItem(idx));

                        list_col = list_col.push(item_container);
                    }
                    DrawerItem::Tray { item, icon } => {
                        let icon_elem: Element<'_, Message> = match icon {
                            Some(p) if p.extension().is_some_and(|e| e == "svg") => {
                                svg(svg::Handle::from_path(p))
                                    .width(Length::Fixed(28.0))
                                    .height(Length::Fixed(28.0))
                                    .into()
                            }
                            Some(p) if p.extension().is_some_and(|e| e == "png") => {
                                image(image::Handle::from_path(p))
                                    .width(Length::Fixed(28.0))
                                    .height(Length::Fixed(28.0))
                                    .into()
                            }
                            _ => {
                                let glyph = if item.id.to_lowercase().contains("wechat")
                                    || item.title.to_lowercase().contains("wechat")
                                {
                                    "󰘑"
                                } else if item.id.to_lowercase().contains("proton")
                                    || item.title.to_lowercase().contains("proton")
                                    || item.id.to_lowercase().contains("vpn")
                                {
                                    "󰖟"
                                } else if item.id.to_lowercase().contains("steam") {
                                    "󰓓"
                                } else if item.id.to_lowercase().contains("discord") {
                                    "󰙯"
                                } else if item.id.to_lowercase().contains("telegram") {
                                    "󰲸"
                                } else {
                                    "󰍜"
                                };
                                text(glyph).size(24).into()
                            }
                        };

                        let item_title = text(&item.title).size(14);
                        let meta_str = if !item.tooltip.is_empty() && item.tooltip != item.title {
                            format!("System Tray • {}", item.tooltip)
                        } else {
                            "System Tray • Background Service".to_string()
                        };
                        let item_meta = text(meta_str).size(11);
                        let info_col = column![item_title, item_meta].spacing(2);

                        let tray_badge = text("󱊖 Background Tray").size(11);

                        let open_btn = button(text("󰁔 Open Window").size(11))
                            .style(button::primary)
                            .padding([3, 8])
                            .on_press(Message::ActivateTray(item.service.clone(), item.path.clone()));

                        let mut item_row = row![icon_elem, info_col]
                            .spacing(12)
                            .align_y(Alignment::Center);

                        item_row = item_row.push(space().width(Length::Fill));
                        item_row = item_row.push(tray_badge);
                        item_row = item_row.push(open_btn);

                        let item_container = button(item_row)
                            .width(Length::Fill)
                            .style(if is_selected {
                                button::primary
                            } else {
                                button::secondary
                            })
                            .padding([6, 10])
                            .on_press(Message::ActivateItem(idx));

                        list_col = list_col.push(item_container);
                    }
                    DrawerItem::App { app } => {
                        let icon_elem: Element<'_, Message> = match &app.icon {
                            Some(p) if p.extension().is_some_and(|e| e == "svg") => {
                                svg(svg::Handle::from_path(p))
                                    .width(Length::Fixed(28.0))
                                    .height(Length::Fixed(28.0))
                                    .into()
                            }
                            Some(p) if p.extension().is_some_and(|e| e == "png") => {
                                image(image::Handle::from_path(p))
                                    .width(Length::Fixed(28.0))
                                    .height(Length::Fixed(28.0))
                                    .into()
                            }
                            _ => text("󰵆").size(24).into(),
                        };

                        let app_name = text(&app.name).size(14);
                        let comment_str = if app.comment.is_empty() {
                            format!("Launch {}", &app.name)
                        } else if app.comment.len() > 60 {
                            format!("{}…", &app.comment[..60])
                        } else {
                            app.comment.clone()
                        };
                        let app_comment = text(comment_str).size(11);

                        let info_col = column![app_name, app_comment].spacing(2);

                        let running_client = app.find_running_client(&self.clients);
                        let running_badge = if running_client.is_some() {
                            Some(text("󰖯 Running").size(11))
                        } else {
                            None
                        };

                        let mut item_row = row![icon_elem, info_col]
                            .spacing(12)
                            .align_y(Alignment::Center);

                        item_row = item_row.push(space().width(Length::Fill));
                        if let Some(badge) = running_badge {
                            item_row = item_row.push(badge);
                        }

                        if let Some(client) = running_client {
                            let is_minimized = client.workspace.id < 0 || client.workspace.name.starts_with("special:");
                            let focus_label = if is_minimized { "󱊖 Restore" } else { "󰁔 Switch" };
                            let focus_btn = button(text(focus_label).size(11))
                                .style(button::primary)
                                .padding([3, 8])
                                .on_press(Message::FocusWindow(client.address.clone()));
                            item_row = item_row.push(focus_btn);

                            let relaunch_btn = button(text("󰐊 New Instance").size(11))
                                .style(button::secondary)
                                .padding([3, 8])
                                .on_press(Message::LaunchApp(idx));
                            item_row = item_row.push(relaunch_btn);
                        } else {
                            let launch_btn = button(text("󰐊 Launch").size(11))
                                .style(button::primary)
                                .padding([3, 8])
                                .on_press(Message::LaunchApp(idx));
                            item_row = item_row.push(launch_btn);
                        }

                        let item_container = button(item_row)
                            .width(Length::Fill)
                            .style(if is_selected {
                                button::primary
                            } else {
                                button::secondary
                            })
                            .padding([6, 10])
                            .on_press(Message::ActivateItem(idx));

                        list_col = list_col.push(item_container);
                    }
                }
            }
        }

        let scroll = scrollable(list_col).height(Length::Fill);

        // --- 4. Footer ---
        let footer = row![
            text("↑↓ Navigate").size(11),
            text("•").size(11),
            text("Enter Focus/Launch").size(11),
            text("•").size(11),
            text("Del / Ctrl+W Close Window").size(11),
            text("•").size(11),
            text("Esc Close").size(11),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let content = column![header_row, input, scroll, footer]
            .spacing(12)
            .padding(16);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
