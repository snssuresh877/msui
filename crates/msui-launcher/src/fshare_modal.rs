use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::operation::focus;
use iced::widget::{
    button, column, container, progress_bar, row, scrollable, space, svg, text, text_input,
};
use iced::{Alignment, Element, Event, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use crate::modal_style::default_modal_card_style;
use msui_core::fshare::{
    default_save_dir, generate_pin, generate_qr_svg, generate_token, get_local_ip,
    load_transfer_history, log_transfer, FShareEvent, FShareServer, FShareServerHandle,
    TransferHistoryEntry,
};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tokio::sync::mpsc;

static INPUT_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);

pub fn run_fshare_modal() -> Result<(), iced_exwlshell::Error> {
    application(
        FShareModal::new,
        FShareModal::namespace,
        FShareModal::update,
        FShareModal::view,
    )
    .theme(FShareModal::theme)
    .default_font(iced::Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..iced::Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(680, 560),
        anchor: Anchor::empty(), // Center of the screen
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    })
    .subscription(FShareModal::subscription)
    .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FShareTab {
    Send,
    Receive,
    History,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendTabMode {
    Recent,
    Browse,
}

#[derive(Debug, Clone)]
pub struct QuickFile {
    pub path: PathBuf,
    pub name: String,
    pub size_str: String,
    pub category: &'static str,
    pub icon: &'static str,
}

#[derive(Debug, Clone)]
pub struct DirEntryItem {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub size_str: String,
    pub icon: &'static str,
}

pub struct FShareModal {
    active_tab: FShareTab,
    send_mode: SendTabMode,
    browse_current_dir: PathBuf,
    browse_entries: Vec<DirEntryItem>,
    send_path_input: String,
    active_file: Option<PathBuf>,
    qr_svg_data: Option<String>,
    active_url: String,
    active_pin: String,
    status_text: String,
    progress_percent: f32,
    speed_mbps: f32,
    is_busy: bool,
    history: Vec<TransferHistoryEntry>,
    quick_files: Vec<QuickFile>,
    selected_category: Option<&'static str>,
    server_handle: Option<FShareServerHandle>,
    is_standalone: bool,
    should_close: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    SwitchTab(FShareTab),
    SelectCategory(Option<&'static str>),
    ToggleSendMode,
    NavigateBrowseDir(PathBuf),
    BrowseDirUp,
    PathInputChanged(String),
    PickFileAndSend(PathBuf),
    StartSendSelected,
    StartReceive,
    StopServer,
    CopyUrl,
    OpenDownloadsFolder,
    EnsureFocus,
    Close,
    IcedEvent(Event),
    ServerStarted(FShareServerHandle),
    ServerEvent(FShareEvent),
}

impl TryInto<ExwlShellCustomActionWithId> for Message {
    type Error = Self;
    fn try_into(self) -> Result<ExwlShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

fn expand_tilde(input: &str) -> PathBuf {
    let mut trimmed = input.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        if trimmed.len() >= 2 {
            trimmed = trimmed[1..trimmed.len() - 1].trim();
        }
    }

    if let Some(stripped) = trimmed.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    } else if trimmed == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home);
        }
    }
    PathBuf::from(trimmed)
}

fn scan_dir(dir: &Path) -> Vec<DirEntryItem> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if name.starts_with('.') {
                continue;
            }
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    dirs.push(DirEntryItem {
                        path,
                        name,
                        is_dir: true,
                        size_str: "Folder".to_string(),
                        icon: "📁",
                    });
                } else if ft.is_file() {
                    let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    let size_str = if len < 1024 {
                        format!("{} B", len)
                    } else if len < 1024 * 1024 {
                        format!("{:.1} KB", len as f64 / 1024.0)
                    } else {
                        format!("{:.2} MB", len as f64 / (1024.0 * 1024.0))
                    };

                    let ext = path.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
                    let icon = match ext.as_str() {
                        "png" | "jpg" | "jpeg" | "webp" | "avif" | "gif" | "svg" => "🖼️",
                        "pdf" => "📕",
                        "xlsx" | "xls" | "csv" => "📊",
                        "docx" | "doc" | "txt" | "md" => "📄",
                        "mp4" | "mkv" | "webm" | "mov" => "🎬",
                        "mp3" | "wav" | "flac" | "ogg" => "🎵",
                        "zip" | "tar" | "gz" | "7z" => "📦",
                        _ => "📄",
                    };

                    files.push(DirEntryItem {
                        path,
                        name,
                        is_dir: false,
                        size_str,
                        icon,
                    });
                }
            }
        }
    }

    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    dirs.extend(files);
    dirs
}

fn scan_recent_files() -> Vec<QuickFile> {
    let home = match std::env::var_os("HOME") {
        Some(h) => PathBuf::from(h),
        None => return Vec::new(),
    };

    let folders = [
        (home.join("Pictures"), "Pictures", "🖼️"),
        (home.join("Pictures").join("Screenshots"), "Pictures", "📸"),
        (home.join("Downloads"), "Downloads", "📥"),
        (home.join("Documents"), "Documents", "📄"),
    ];

    let mut found: Vec<(std::time::SystemTime, QuickFile)> = Vec::new();

    for (dir, category, default_icon) in &folders {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_file() {
                        let path = entry.path();
                        let filename = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                        if filename.starts_with('.') {
                            continue;
                        }
                        if let Ok(meta) = entry.metadata() {
                            let len = meta.len();
                            let size_str = if len < 1024 {
                                format!("{} B", len)
                            } else if len < 1024 * 1024 {
                                format!("{:.1} KB", len as f64 / 1024.0)
                            } else {
                                format!("{:.2} MB", len as f64 / (1024.0 * 1024.0))
                            };

                            let ext = path.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
                            let icon = match ext.as_str() {
                                "png" | "jpg" | "jpeg" | "webp" | "avif" | "gif" | "svg" => "🖼️",
                                "pdf" => "📕",
                                "xlsx" | "xls" | "csv" => "📊",
                                "docx" | "doc" | "txt" | "md" => "📄",
                                "mp4" | "mkv" | "webm" | "mov" => "🎬",
                                "mp3" | "wav" | "flac" | "ogg" => "🎵",
                                "zip" | "tar" | "gz" | "7z" => "📦",
                                _ => *default_icon,
                            };

                            let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                            found.push((
                                modified,
                                QuickFile {
                                    path,
                                    name: filename,
                                    size_str,
                                    category: *category,
                                    icon,
                                },
                            ));
                        }
                    }
                }
            }
        }
    }

    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.truncate(20);
    found.into_iter().map(|(_, qf)| qf).collect()
}

impl FShareModal {
    pub fn new() -> (Self, Command<Message>) {
        Self::new_internal(true)
    }

    pub fn new_embedded() -> (Self, Command<Message>) {
        Self::new_internal(false)
    }

    fn new_internal(is_standalone: bool) -> (Self, Command<Message>) {
        let history = load_transfer_history();
        let quick_files = scan_recent_files();
        let home_dir = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
        let browse_entries = scan_dir(&home_dir);

        let modal = Self {
            active_tab: FShareTab::Send,
            send_mode: SendTabMode::Recent,
            browse_current_dir: home_dir,
            browse_entries,
            send_path_input: String::new(),
            active_file: None,
            qr_svg_data: None,
            active_url: String::new(),
            active_pin: String::new(),
            status_text: "Select a file to generate a sharing link & QR code.".to_string(),
            progress_percent: 0.0,
            speed_mbps: 0.0,
            is_busy: false,
            history,
            quick_files,
            selected_category: None,
            server_handle: None,
            is_standalone,
            should_close: false,
        };

        (
            modal,
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
        self.should_close = false;
        self.progress_percent = 0.0;
        self.speed_mbps = 0.0;
        self.is_busy = false;
        self.history = load_transfer_history();
        self.quick_files = scan_recent_files();
        self.selected_category = None;
        self.send_mode = SendTabMode::Recent;
        let home_dir = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
        self.browse_current_dir = home_dir.clone();
        self.browse_entries = scan_dir(&home_dir);
        if let Some(h) = self.server_handle.take() {
            h.stop();
        }
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
        String::from("msui-fshare-hub")
    }

    pub fn theme(&self) -> iced::Theme {
        iced::Theme::CatppuccinMocha
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let event_sub = event::listen().map(Message::IcedEvent);
        iced::Subscription::batch([event_sub])
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::EnsureFocus => {
                return focus(INPUT_ID.clone());
            }
            Message::SwitchTab(tab) => {
                self.active_tab = tab;
                if let Some(h) = self.server_handle.take() {
                    h.stop();
                }
                self.qr_svg_data = None;
                self.active_url.clear();
                self.active_pin.clear();
                self.progress_percent = 0.0;
                self.speed_mbps = 0.0;
                self.is_busy = false;

                if tab == FShareTab::Receive {
                    return self.update(Message::StartReceive);
                } else if tab == FShareTab::History {
                    self.history = load_transfer_history();
                } else if tab == FShareTab::Send {
                    self.quick_files = scan_recent_files();
                    return focus(INPUT_ID.clone());
                }
                Command::none()
            }
            Message::SelectCategory(cat) => {
                self.selected_category = cat;
                match cat {
                    Some("Pictures") => self.send_path_input = "~/Pictures".to_string(),
                    Some("Downloads") => self.send_path_input = "~/Downloads".to_string(),
                    Some("Documents") => self.send_path_input = "~/Documents".to_string(),
                    _ => self.send_path_input.clear(),
                }
                Command::none()
            }
            Message::ToggleSendMode => {
                match self.send_mode {
                    SendTabMode::Recent => {
                        self.send_mode = SendTabMode::Browse;
                        let target = expand_tilde(&self.send_path_input);
                        if target.is_dir() {
                            self.browse_current_dir = target;
                        } else if let Some(parent) = target.parent() {
                            if parent.is_dir() {
                                self.browse_current_dir = parent.to_path_buf();
                            }
                        }
                        self.browse_entries = scan_dir(&self.browse_current_dir);
                    }
                    SendTabMode::Browse => {
                        self.send_mode = SendTabMode::Recent;
                    }
                }
                Command::none()
            }
            Message::NavigateBrowseDir(dir) => {
                if dir.is_dir() {
                    self.browse_current_dir = dir.clone();
                    self.browse_entries = scan_dir(&self.browse_current_dir);
                    self.send_path_input = dir.to_string_lossy().to_string();
                }
                Command::none()
            }
            Message::BrowseDirUp => {
                if let Some(parent) = self.browse_current_dir.parent() {
                    if parent.is_dir() {
                        self.browse_current_dir = parent.to_path_buf();
                        self.browse_entries = scan_dir(&self.browse_current_dir);
                        self.send_path_input = self.browse_current_dir.to_string_lossy().to_string();
                    }
                }
                Command::none()
            }
            Message::PathInputChanged(path) => {
                self.send_path_input = path;
                Command::none()
            }
            Message::PickFileAndSend(path) => {
                self.send_path_input = path.to_string_lossy().to_string();
                self.active_file = Some(path);
                self.start_send_server()
            }
            Message::StartSendSelected => {
                let p = expand_tilde(&self.send_path_input);
                if p.exists() {
                    if p.is_file() {
                        self.active_file = Some(p);
                        self.start_send_server()
                    } else if p.is_dir() {
                        self.send_mode = SendTabMode::Browse;
                        self.browse_current_dir = p.clone();
                        self.browse_entries = scan_dir(&p);
                        self.status_text = format!("Browsing: {}", p.display());
                        Command::none()
                    } else {
                        self.status_text = format!("Invalid file: {}", self.send_path_input);
                        Command::none()
                    }
                } else {
                    self.status_text = format!("File does not exist: {}", self.send_path_input);
                    Command::none()
                }
            }
            Message::StartReceive => {
                self.start_receive_server()
            }
            Message::StopServer => {
                if let Some(h) = self.server_handle.take() {
                    h.stop();
                }
                self.qr_svg_data = None;
                self.active_url.clear();
                self.active_pin.clear();
                self.is_busy = false;
                self.status_text = "Server stopped.".to_string();
                Command::none()
            }
            Message::CopyUrl => {
                if !self.active_url.is_empty() {
                    let url = self.active_url.clone();
                    self.status_text = "URL copied to clipboard!".to_string();
                    return iced::clipboard::write(url);
                }
                Command::none()
            }
            Message::OpenDownloadsFolder => {
                let dir = default_save_dir();
                let _ = std::process::Command::new("xdg-open")
                    .arg(&dir)
                    .spawn();
                Command::none()
            }
            Message::Close => {
                if let Some(h) = self.server_handle.take() {
                    h.stop();
                }
                if self.is_standalone {
                    std::process::exit(0);
                } else {
                    self.should_close = true;
                }
                Command::none()
            }
            Message::IcedEvent(Event::Keyboard(KeyboardEvent::KeyPressed { key, .. })) => {
                if key == Key::Named(Named::Escape) {
                    return self.update(Message::Close);
                }
                Command::none()
            }
            Message::IcedEvent(Event::Window(_)) => {
                if self.active_tab == FShareTab::Send {
                    return focus(INPUT_ID.clone());
                }
                Command::none()
            }
            Message::ServerEvent(event) => {
                match event {
                    FShareEvent::Started { url, pin } => {
                        self.active_url = url.clone();
                        self.active_pin = pin;
                        match generate_qr_svg(&url) {
                            Ok(svg) => self.qr_svg_data = Some(svg),
                            Err(e) => self.status_text = format!("QR error: {:?}", e),
                        }
                        self.status_text = "Scan QR code or open URL on your phone.".to_string();
                        self.is_busy = false;
                    }
                    FShareEvent::ClientConnected { peer_ip } => {
                        self.status_text = format!("📱 Phone connected ({})!", peer_ip);
                        self.is_busy = true;
                    }
                    FShareEvent::Progress { transferred_bytes, total_bytes, speed_mbps } => {
                        if total_bytes > 0 {
                            self.progress_percent = (transferred_bytes as f32 / total_bytes as f32).min(1.0);
                        }
                        self.speed_mbps = speed_mbps;
                        let mb_sent = transferred_bytes as f32 / (1024.0 * 1024.0);
                        let mb_total = total_bytes as f32 / (1024.0 * 1024.0);
                        self.status_text = format!("Transferring: {:.1} / {:.1} MB ({:.1} MB/s)", mb_sent, mb_total, speed_mbps);
                    }
                    FShareEvent::FileReceived { filename, size_bytes, path: _ } => {
                        log_transfer("Received", &filename, size_bytes);
                        self.history = load_transfer_history();
                        self.status_text = format!("✅ Received '{}' ({:.2} MB)", filename, size_bytes as f64 / (1024.0 * 1024.0));
                        self.progress_percent = 1.0;
                        self.is_busy = false;
                    }
                    FShareEvent::FileSent { filename, size_bytes } => {
                        log_transfer("Sent", &filename, size_bytes);
                        self.history = load_transfer_history();
                        self.status_text = format!("✅ Successfully sent '{}'!", filename);
                        self.progress_percent = 1.0;
                        self.is_busy = false;
                    }
                    FShareEvent::Error(err) => {
                        self.status_text = format!("❌ Error: {}", err);
                        self.is_busy = false;
                    }
                    FShareEvent::Completed => {
                        self.is_busy = false;
                    }
                }
                Command::none()
            }
            Message::ServerStarted(handle) => {
                self.server_handle = Some(handle);
                Command::none()
            }
            _ => Command::none(),
        }
    }

    fn start_send_server(&mut self) -> Command<Message> {
        let file = match &self.active_file {
            Some(f) if f.exists() => f.clone(),
            _ => {
                self.status_text = "Please specify a valid file to send.".to_string();
                return Command::none();
            }
        };

        if let Some(h) = self.server_handle.take() {
            h.stop();
        }

        let local_ip = match get_local_ip() {
            Some(ip) => ip,
            None => {
                self.status_text = "Could not detect active local Wi-Fi IP.".to_string();
                return Command::none();
            }
        };

        let token = generate_token();
        let pin = generate_pin();

        self.status_text = "Starting direct Wi-Fi micro-server...".to_string();
        self.progress_percent = 0.0;
        self.speed_mbps = 0.0;

        let f_clone = file.clone();
        let ip_clone = local_ip.clone();
        let t_clone = token.clone();
        let p_clone = pin.clone();

        Command::run(
            iced::stream::channel(100, |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                use iced::futures::SinkExt;
                let (tx, mut rx) = mpsc::unbounded_channel();
                match FShareServer::start_send(f_clone, ip_clone, t_clone, p_clone, tx).await {
                    Ok(handle) => {
                        let _ = output.send(Message::ServerStarted(handle)).await;
                        while let Some(event) = rx.recv().await {
                            let _ = output.send(Message::ServerEvent(event)).await;
                        }
                    }
                    Err(e) => {
                        let _ = output.send(Message::ServerEvent(FShareEvent::Error(format!("Failed to start server: {:?}", e)))).await;
                    }
                }
            }),
            |msg| msg,
        )
    }

    fn start_receive_server(&mut self) -> Command<Message> {
        if let Some(h) = self.server_handle.take() {
            h.stop();
        }

        let local_ip = match get_local_ip() {
            Some(ip) => ip,
            None => {
                self.status_text = "Could not detect active local Wi-Fi IP.".to_string();
                return Command::none();
            }
        };

        let save_dir = default_save_dir();
        let token = generate_token();
        let pin = generate_pin();

        self.status_text = "Starting receive dropzone server...".to_string();
        self.progress_percent = 0.0;
        self.speed_mbps = 0.0;

        let dir_clone = save_dir.clone();
        let ip_clone = local_ip.clone();
        let t_clone = token.clone();
        let p_clone = pin.clone();

        Command::run(
            iced::stream::channel(100, |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                use iced::futures::SinkExt;
                let (tx, mut rx) = mpsc::unbounded_channel();
                match FShareServer::start_recv(dir_clone, ip_clone, t_clone, p_clone, tx).await {
                    Ok(handle) => {
                        let _ = output.send(Message::ServerStarted(handle)).await;
                        while let Some(event) = rx.recv().await {
                            let _ = output.send(Message::ServerEvent(event)).await;
                        }
                    }
                    Err(e) => {
                        let _ = output.send(Message::ServerEvent(FShareEvent::Error(format!("Failed to start receive server: {:?}", e)))).await;
                    }
                }
            }),
            |msg| msg,
        )
    }

    pub fn view(&self) -> Element<'_, Message> {
        let title_row = row![
            text("msUI FShare").size(20).color([0.80, 0.84, 0.96]),
            space().width(Length::Fill),
            text("Esc to Close").size(13).color([0.55, 0.57, 0.68]),
        ]
        .align_y(Alignment::Center);

        // Tab Selector Row
        let tab_btn = |tab: FShareTab, label: &'static str| {
            let is_active = self.active_tab == tab;
            let bg = if is_active {
                iced::Color::from_rgb(0.54, 0.71, 0.98) // Catppuccin Blue
            } else {
                iced::Color::from_rgb(0.19, 0.20, 0.27) // Surface0
            };
            let text_color = if is_active {
                iced::Color::from_rgb(0.07, 0.07, 0.11)
            } else {
                iced::Color::from_rgb(0.80, 0.84, 0.96)
            };

            button(text(label).size(14).color(text_color))
                .padding([8, 16])
                .style(move |_, _| button::Style {
                    background: Some(bg.into()),
                    border: iced::border::rounded(10),
                    ..Default::default()
                })
                .on_press(Message::SwitchTab(tab))
        };

        let tabs = row![
            tab_btn(FShareTab::Send, "📤 Send to Phone"),
            tab_btn(FShareTab::Receive, "📥 Receive from Phone"),
            tab_btn(FShareTab::History, "📜 Transfer History"),
        ]
        .spacing(10);

        let content_area: Element<'_, Message> = match self.active_tab {
            FShareTab::Send => self.view_send_tab(),
            FShareTab::Receive => self.view_receive_tab(),
            FShareTab::History => self.view_history_tab(),
        };

        let main_card = container(
            column![
                title_row,
                tabs,
                content_area,
            ]
            .spacing(18),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(24)
        .style(default_modal_card_style());

        container(main_card)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn view_send_tab(&self) -> Element<'_, Message> {
        let toggle_label = match self.send_mode {
            SendTabMode::Recent => "📁 Browse",
            SendTabMode::Browse => "📜 Recent",
        };

        let input_row = row![
            text_input("Enter file/folder path or choose below...", &self.send_path_input)
                .id(INPUT_ID.clone())
                .on_input(Message::PathInputChanged)
                .on_submit(Message::StartSendSelected)
                .padding(10)
                .width(Length::Fill),
            button(text(toggle_label).size(13))
                .padding([8, 14])
                .on_press(Message::ToggleSendMode),
            button(text("Share").size(13))
                .padding([8, 16])
                .style(button::primary)
                .on_press(Message::StartSendSelected),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let body: Element<'_, Message> = if let Some(svg_data) = &self.qr_svg_data {
            let qr_handle = svg::Handle::from_memory(svg_data.as_bytes().to_vec());
            let qr_widget = container(
                svg(qr_handle)
                    .width(Length::Fixed(190.0))
                    .height(Length::Fixed(190.0)),
            )
            .padding(10)
            .style(|_| container::Style {
                background: Some(iced::Color::from_rgb(0.09, 0.09, 0.15).into()),
                border: iced::border::rounded(14),
                ..Default::default()
            });

            let file_desc = if let Some(f) = &self.active_file {
                let name = f.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let size_mb = std::fs::metadata(f).map(|m| m.len() as f64 / (1024.0 * 1024.0)).unwrap_or(0.0);
                format!("File: {} ({:.2} MB)", name, size_mb)
            } else {
                "Ready to download".to_string()
            };

            let details = column![
                text(file_desc).size(15).color([1.0, 1.0, 1.0]),
                text(format!("URL: {}", self.active_url)).size(13).color([0.54, 0.71, 0.98]),
                text(format!("PIN: {}", self.active_pin)).size(13).color([0.65, 0.89, 0.63]),
                row![
                    button(text("📋 Copy Link").size(13)).padding([6, 12]).on_press(Message::CopyUrl),
                    button(text("⏹ Stop").size(13)).padding([6, 12]).on_press(Message::StopServer),
                ]
                .spacing(8),
            ]
            .spacing(10);

            row![qr_widget, details].spacing(20).align_y(Alignment::Center).into()
        } else {
            match self.send_mode {
                SendTabMode::Recent => self.view_recent_files(),
                SendTabMode::Browse => self.view_directory_browser(),
            }
        };

        let prog_bar = progress_bar(0.0..=1.0, self.progress_percent);

        let status_row = row![
            text(&self.status_text).size(13).color([0.65, 0.89, 0.63]),
            space().width(Length::Fill),
            text(format!("{:.0}%", self.progress_percent * 100.0)).size(13).color([0.80, 0.84, 0.96]),
        ];

        column![
            input_row,
            body,
            space().height(Length::Fill),
            prog_bar,
            status_row,
        ]
        .spacing(14)
        .into()
    }

    fn view_recent_files(&self) -> Element<'_, Message> {
        let cat_chip = |cat: Option<&'static str>, label: &'static str| {
            let is_sel = self.selected_category == cat;
            let bg = if is_sel {
                iced::Color::from_rgb(0.54, 0.71, 0.98)
            } else {
                iced::Color::from_rgb(0.18, 0.18, 0.25)
            };
            let text_color = if is_sel {
                iced::Color::from_rgb(0.07, 0.07, 0.11)
            } else {
                iced::Color::from_rgb(0.75, 0.78, 0.90)
            };

            button(text(label).size(12).color(text_color))
                .padding([5, 12])
                .style(move |_, _| button::Style {
                    background: Some(bg.into()),
                    border: iced::border::rounded(8),
                    ..Default::default()
                })
                .on_press(Message::SelectCategory(cat))
        };

        let chips_row = row![
            text("Quick Share:").size(13).color([0.55, 0.57, 0.68]),
            cat_chip(None, "All Recent"),
            cat_chip(Some("Pictures"), "🖼️ Pictures"),
            cat_chip(Some("Downloads"), "📥 Downloads"),
            cat_chip(Some("Documents"), "📄 Documents"),
            button(text("📁 Browse Folders").size(11).color([0.54, 0.71, 0.98]))
                .padding([4, 10])
                .style(|_, _| button::Style {
                    background: Some(iced::Color::from_rgb(0.18, 0.18, 0.25).into()),
                    border: iced::border::rounded(8),
                    ..Default::default()
                })
                .on_press(Message::ToggleSendMode),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let filtered_files: Vec<&QuickFile> = self
            .quick_files
            .iter()
            .filter(|f| match self.selected_category {
                Some(cat) => f.category == cat,
                None => true,
            })
            .take(6)
            .collect();

        let mut files_col = column![].spacing(6);
        if filtered_files.is_empty() {
            files_col = files_col.push(
                text("No recent files found in this category. Click 'Browse' to explore folders.")
                    .size(12)
                    .color([0.55, 0.57, 0.68]),
            );
        } else {
            for qf in filtered_files {
                let cat_color = match qf.category {
                    "Pictures" => [0.58, 0.89, 0.87],
                    "Downloads" => [0.54, 0.71, 0.98],
                    _ => [0.98, 0.89, 0.62],
                };

                let item = button(
                    row![
                        text(qf.icon).size(15),
                        text(&qf.name).size(12).color([0.90, 0.92, 0.98]).width(Length::Fill),
                        text(qf.category).size(10).color(cat_color),
                        text(&qf.size_str).size(11).color([0.65, 0.67, 0.78]),
                        text("Share ⏎").size(11).color([0.54, 0.71, 0.98]),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([6, 10])
                .style(|_, _| button::Style {
                    background: Some(iced::Color::from_rgb(0.16, 0.16, 0.23).into()),
                    border: iced::border::rounded(8),
                    ..Default::default()
                })
                .on_press(Message::PickFileAndSend(qf.path.clone()));

                files_col = files_col.push(item);
            }
        }

        column![
            chips_row,
            scrollable(files_col).height(Length::Fixed(210.0)),
        ]
        .spacing(10)
        .into()
    }

    fn view_directory_browser(&self) -> Element<'_, Message> {
        let nav_bar = row![
            button(text("⬅ Recent").size(12))
                .padding([5, 10])
                .on_press(Message::ToggleSendMode),
            button(text("⬆ Up").size(12))
                .padding([5, 10])
                .on_press(Message::BrowseDirUp),
            text(self.browse_current_dir.to_string_lossy())
                .size(12)
                .color([0.54, 0.71, 0.98])
                .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let place_btn = |path: PathBuf, icon: &'static str, label: &'static str| {
            let is_curr = self.browse_current_dir == path;
            let bg = if is_curr {
                iced::Color::from_rgb(0.54, 0.71, 0.98)
            } else {
                iced::Color::from_rgb(0.18, 0.18, 0.25)
            };
            let text_color = if is_curr {
                iced::Color::from_rgb(0.07, 0.07, 0.11)
            } else {
                iced::Color::from_rgb(0.75, 0.78, 0.90)
            };

            button(text(format!("{} {}", icon, label)).size(11).color(text_color))
                .padding([4, 10])
                .style(move |_, _| button::Style {
                    background: Some(bg.into()),
                    border: iced::border::rounded(8),
                    ..Default::default()
                })
                .on_press(Message::NavigateBrowseDir(path))
        };

        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
        let places_row = row![
            place_btn(home.clone(), "🏠", "Home"),
            place_btn(home.join("Pictures"), "🖼️", "Pictures"),
            place_btn(home.join("Downloads"), "📥", "Downloads"),
            place_btn(home.join("Documents"), "📄", "Documents"),
            place_btn(home.join("Projects"), "💼", "Projects"),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let mut entries_col = column![].spacing(6);
        if self.browse_entries.is_empty() {
            entries_col = entries_col.push(
                text("Empty folder.").size(12).color([0.55, 0.57, 0.68]),
            );
        } else {
            for entry in &self.browse_entries {
                if entry.is_dir {
                    let folder_row = button(
                        row![
                            text("📁").size(15),
                            text(&entry.name).size(12).color([0.90, 0.92, 0.98]).width(Length::Fill),
                            text("Folder").size(11).color([0.55, 0.57, 0.68]),
                            text("›").size(14).color([0.55, 0.57, 0.68]),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    )
                    .padding([6, 10])
                    .width(Length::Fill)
                    .style(|_, _| button::Style {
                        background: Some(iced::Color::from_rgb(0.16, 0.16, 0.23).into()),
                        border: iced::border::rounded(8),
                        ..Default::default()
                    })
                    .on_press(Message::NavigateBrowseDir(entry.path.clone()));

                    entries_col = entries_col.push(folder_row);
                } else {
                    let file_row = button(
                        row![
                            text(entry.icon).size(15),
                            text(&entry.name).size(12).color([0.90, 0.92, 0.98]).width(Length::Fill),
                            text(&entry.size_str).size(11).color([0.65, 0.67, 0.78]),
                            text("Share ⏎").size(11).color([0.54, 0.71, 0.98]),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    )
                    .width(Length::Fill)
                    .padding([6, 10])
                    .style(|_, _| button::Style {
                        background: Some(iced::Color::from_rgb(0.14, 0.14, 0.20).into()),
                        border: iced::border::rounded(8),
                        ..Default::default()
                    })
                    .on_press(Message::PickFileAndSend(entry.path.clone()));

                    entries_col = entries_col.push(file_row);
                }
            }
        }

        column![
            nav_bar,
            places_row,
            scrollable(entries_col).height(Length::Fixed(210.0)),
        ]
        .spacing(8)
        .into()
    }

    fn view_receive_tab(&self) -> Element<'_, Message> {
        let body: Element<'_, Message> = if let Some(svg_data) = &self.qr_svg_data {
            let qr_handle = svg::Handle::from_memory(svg_data.as_bytes().to_vec());
            let qr_widget = container(
                svg(qr_handle)
                    .width(Length::Fixed(190.0))
                    .height(Length::Fixed(190.0)),
            )
            .padding(10)
            .style(|_| container::Style {
                background: Some(iced::Color::from_rgb(0.09, 0.09, 0.15).into()),
                border: iced::border::rounded(14),
                ..Default::default()
            });

            let details = column![
                text("Scan with your phone camera to send files to this PC").size(14).color([1.0, 1.0, 1.0]),
                text(format!("URL: {}", self.active_url)).size(13).color([0.54, 0.71, 0.98]),
                text("Target Folder: ~/Downloads/msui-fshare/").size(12).color([0.68, 0.70, 0.78]),
                row![
                    button(text("📂 Open Folder").size(13)).padding([6, 12]).on_press(Message::OpenDownloadsFolder),
                    button(text("📋 Copy Link").size(13)).padding([6, 12]).on_press(Message::CopyUrl),
                    button(text("⏹ Stop").size(13)).padding([6, 12]).on_press(Message::StopServer),
                ]
                .spacing(8),
            ]
            .spacing(10);

            row![qr_widget, details].spacing(20).align_y(Alignment::Center).into()
        } else {
            column![
                text("Receive Server Ready").size(14).color([1.0, 1.0, 1.0]),
                button(text("Start Receiver").size(14)).padding([10, 20]).on_press(Message::StartReceive),
            ]
            .spacing(12)
            .into()
        };

        let prog_bar = progress_bar(0.0..=1.0, self.progress_percent);

        let status_row = row![
            text(&self.status_text).size(13).color([0.65, 0.89, 0.63]),
            space().width(Length::Fill),
            text(format!("{:.0}%", self.progress_percent * 100.0)).size(13).color([0.80, 0.84, 0.96]),
        ];

        column![
            body,
            space().height(Length::Fill),
            prog_bar,
            status_row,
        ]
        .spacing(14)
        .into()
    }

    fn view_history_tab(&self) -> Element<'_, Message> {
        if self.history.is_empty() {
            return column![
                text("No transfer history yet.").size(14).color([0.55, 0.57, 0.68]),
                text("Transferred files will appear here.").size(12).color([0.45, 0.47, 0.58]),
            ]
            .spacing(8)
            .into();
        }

        let mut rows = column![].spacing(8);
        for item in &self.history {
            let is_sent = item.direction == "Sent";
            let tag_color = if is_sent { [0.54, 0.71, 0.98] } else { [0.65, 0.89, 0.63] };
            let size_mb = item.size_bytes as f64 / (1024.0 * 1024.0);

            let row_item = container(
                row![
                    text(format!("[{}]", item.direction)).size(12).color(tag_color),
                    text(&item.filename).size(13).color([1.0, 1.0, 1.0]).width(Length::Fill),
                    text(format!("{:.2} MB", size_mb)).size(12).color([0.68, 0.70, 0.78]),
                    text(&item.timestamp).size(11).color([0.55, 0.57, 0.68]),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .padding([8, 12])
            .style(|_| container::Style {
                background: Some(iced::Color::from_rgb(0.16, 0.16, 0.23).into()),
                border: iced::border::rounded(8),
                ..Default::default()
            });

            rows = rows.push(row_item);
        }

        column![
            row![
                text("Recent Local Transfers").size(14).color([1.0, 1.0, 1.0]),
                space().width(Length::Fill),
                button(text("📂 Downloads").size(12)).padding([4, 10]).on_press(Message::OpenDownloadsFolder),
            ]
            .align_y(Alignment::Center),
            scrollable(rows).height(Length::Fill),
        ]
        .spacing(12)
        .into()
    }
}
