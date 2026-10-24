use futures::channel::mpsc::Sender;
use futures::SinkExt;
use iced::widget::{container, space};
use iced::{Element, Font, Length, Task};
use iced_exwlshell::daemon;
use iced_exwlshell::reexport::{
    Anchor, KeyboardInteractivity, Layer, LayerSize, Margin, NewLayerShellSettings, OutputOption,
};
use iced_exwlshell::settings::{ExWlSettings, LayerShellSettings, StartMode};
use iced_exwlshell::to_layer_message;
use iced_wayland_subscriber::shell::{self, ShellReceiver};
use msui_bar::{Bar, BarMessage};
use msui_core::ipc::{IpcCommand, IpcServer};
use msui_core::{
    AudioMonitor, BrightnessMonitor, MsuiConfig, OsdKind, PolkitRequest, ThemeManager,
    WallpaperManager,
};
use msui_launcher::{
    AppDrawer, AppDrawerMessage, AudioMenu, AudioMessage, BatteryMenu, BatteryMessage,
    BluetoothMenu, BluetoothMessage, ClipboardMenu, ClipboardMessage, ControlCenter,
    ControlCenterMessage, DisplayMenu, DisplayMessage, FShareMessage, FShareModal, Launcher,
    LauncherMessage, MediaMenu, MediaMessage, NotificationCenter, NotificationCenterMessage,
    OsdMessage, OsdWindow, PolkitDialog, PolkitDialogArgs, PolkitMessage, PowerMenu, PowerMessage,
    ReminderMenu, ReminderMessage, StorageMenu, StorageMessage, ThemeMenu, ThemeMessage, ToastApp,
    ToastMessage, WallpaperDaemon, WallpaperMessage, WallpaperMenu, WallpaperMenuMessage, WifiMenu,
    WifiMessage,
};
use tokio::sync::broadcast;

use crate::ModalManager;

pub enum ActiveModal {
    Launcher(Launcher),
    Power(PowerMenu),
    ControlCenter(ControlCenter),
    Clipboard(ClipboardMenu),
    NotificationCenter(NotificationCenter),
    AppDrawer(AppDrawer),
    Wifi(WifiMenu),
    Bluetooth(BluetoothMenu),
    Audio(AudioMenu),
    Theme(ThemeMenu),
    Wallpaper(WallpaperMenu),
    FShare(FShareModal),
    Battery(BatteryMenu),
    Display(DisplayMenu),
    Media(MediaMenu),
    Reminder(ReminderMenu),
    Storage(StorageMenu),
    Polkit(PolkitDialog),
}

impl ActiveModal {
    pub fn kind(&self) -> &'static str {
        match self {
            ActiveModal::Launcher(_) => "launcher",
            ActiveModal::Power(_) => "power",
            ActiveModal::ControlCenter(_) => "control",
            ActiveModal::Clipboard(_) => "clipboard",
            ActiveModal::NotificationCenter(_) => "notifications",
            ActiveModal::AppDrawer(_) => "drawer",
            ActiveModal::Wifi(_) => "wifi",
            ActiveModal::Bluetooth(_) => "bluetooth",
            ActiveModal::Audio(_) => "audio",
            ActiveModal::Theme(_) => "theme",
            ActiveModal::Wallpaper(_) => "wallpaper",
            ActiveModal::FShare(_) => "fshare",
            ActiveModal::Battery(_) => "battery",
            ActiveModal::Display(_) => "display",
            ActiveModal::Media(_) => "media",
            ActiveModal::Reminder(_) => "reminder",
            ActiveModal::Storage(_) => "storage",
            ActiveModal::Polkit(_) => "polkit",
        }
    }

    pub fn should_close(&self) -> bool {
        match self {
            ActiveModal::Launcher(m) => m.should_close(),
            ActiveModal::Power(m) => m.should_close(),
            ActiveModal::ControlCenter(m) => m.should_close(),
            ActiveModal::Clipboard(m) => m.should_close(),
            ActiveModal::NotificationCenter(m) => m.should_close(),
            ActiveModal::AppDrawer(m) => m.should_close(),
            ActiveModal::Wifi(m) => m.should_close(),
            ActiveModal::Bluetooth(m) => m.should_close(),
            ActiveModal::Audio(m) => m.should_close(),
            ActiveModal::Theme(m) => m.should_close(),
            ActiveModal::Wallpaper(m) => m.should_close(),
            ActiveModal::FShare(m) => m.should_close(),
            ActiveModal::Battery(m) => m.should_close(),
            ActiveModal::Display(m) => m.should_close(),
            ActiveModal::Media(m) => m.should_close(),
            ActiveModal::Reminder(m) => m.should_close(),
            ActiveModal::Storage(m) => m.should_close(),
            ActiveModal::Polkit(m) => m.should_close(),
        }
    }

    #[allow(dead_code)]
    pub fn reset_should_close(&mut self) {
        match self {
            ActiveModal::Launcher(m) => m.reset_should_close(),
            ActiveModal::Power(m) => m.reset_should_close(),
            ActiveModal::ControlCenter(m) => m.reset_should_close(),
            ActiveModal::Clipboard(m) => m.reset_should_close(),
            ActiveModal::NotificationCenter(m) => m.reset_should_close(),
            ActiveModal::AppDrawer(m) => m.reset_should_close(),
            ActiveModal::Wifi(m) => m.reset_should_close(),
            ActiveModal::Bluetooth(m) => m.reset_should_close(),
            ActiveModal::Audio(m) => m.reset_should_close(),
            ActiveModal::Theme(m) => m.reset_should_close(),
            ActiveModal::Wallpaper(m) => m.reset_should_close(),
            ActiveModal::FShare(m) => m.reset_should_close(),
            ActiveModal::Battery(m) => m.reset_should_close(),
            ActiveModal::Display(m) => m.reset_should_close(),
            ActiveModal::Media(m) => m.reset_should_close(),
            ActiveModal::Reminder(m) => m.reset_should_close(),
            ActiveModal::Storage(m) => m.reset_should_close(),
            ActiveModal::Polkit(m) => m.reset_should_close(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ModalMessage {
    Launcher(LauncherMessage),
    Power(PowerMessage),
    ControlCenter(ControlCenterMessage),
    Clipboard(ClipboardMessage),
    NotificationCenter(NotificationCenterMessage),
    AppDrawer(AppDrawerMessage),
    Wifi(WifiMessage),
    Bluetooth(BluetoothMessage),
    Audio(AudioMessage),
    Theme(ThemeMessage),
    Wallpaper(WallpaperMenuMessage),
    FShare(FShareMessage),
    Battery(BatteryMessage),
    Display(DisplayMessage),
    Media(MediaMessage),
    Reminder(ReminderMessage),
    Storage(StorageMessage),
    Polkit(PolkitMessage),
}

#[to_layer_message(multi)]
#[derive(Debug, Clone)]
pub enum DaemonMessage {
    Bar(BarMessage),
    Wallpaper(WallpaperMessage),
    Toast(ToastMessage),
    Osd(OsdMessage),
    Modal(ModalMessage),
    Ipc(IpcCommand),
    PolkitRequest(PolkitRequest),
    PolkitCancel(String),
    WindowClosed(iced::window::Id),
    Ignored,
}

static IPC_BROADCAST_TX: std::sync::LazyLock<broadcast::Sender<IpcCommand>> =
    std::sync::LazyLock::new(|| {
        let (tx, _) = broadcast::channel(64);
        tx
    });

fn ipc_stream() -> impl iced::futures::Stream<Item = DaemonMessage> {
    iced::stream::channel(100, |mut output: Sender<DaemonMessage>| async move {
        let mut rx = IPC_BROADCAST_TX.subscribe();
        while let Ok(cmd) = rx.recv().await {
            let _ = output.send(DaemonMessage::Ipc(cmd)).await;
        }
    })
}

fn polkit_stream() -> impl iced::futures::Stream<Item = DaemonMessage> {
    iced::stream::channel(10, |mut output: Sender<DaemonMessage>| async move {
        let (req_tx, mut req_rx) = tokio::sync::mpsc::unbounded_channel::<msui_core::PolkitRequest>();
        let (cancel_tx, mut cancel_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        msui_core::register_polkit_request_handler(req_tx);
        msui_core::register_polkit_cancel_handler(cancel_tx);

        loop {
            tokio::select! {
                Some(req) = req_rx.recv() => {
                    let _ = output.send(DaemonMessage::PolkitRequest(req)).await;
                }
                Some(cookie) = cancel_rx.recv() => {
                    let _ = output.send(DaemonMessage::PolkitCancel(cookie)).await;
                }
                else => break,
            }
        }
    })
}

pub struct UnifiedShell {
    config: MsuiConfig,
    bar_id: Option<iced::window::Id>,
    wallpaper_id: Option<iced::window::Id>,
    toast_id: Option<iced::window::Id>,
    osd_id: Option<iced::window::Id>,
    modal_id: Option<iced::window::Id>,

    bar: Bar,
    wallpaper: WallpaperDaemon,
    toast: ToastApp,
    osd: OsdWindow,
    active_modal: Option<ActiveModal>,
    polkit_request: Option<PolkitRequest>,
}

impl UnifiedShell {
    fn new(
        config: MsuiConfig,
        shell_events: ShellReceiver,
    ) -> (Self, Task<DaemonMessage>) {
        let output_option = if let Some(target) = msui_core::DisplayManager::get_primary_monitor_name() {
            OutputOption::OutputName(target)
        } else {
            OutputOption::Active
        };

        // 1. Wallpaper Layer Shell surface (Background)
        let (wallpaper_id, wallpaper_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
            size: LayerSize::FILL,
            layer: Layer::Background,
            anchor: Anchor::all(),
            exclusive_zone: Some(-1),
            keyboard_interactivity: KeyboardInteractivity::None,
            output_option: output_option.clone(),
            ..Default::default()
        });

        // 2. Bar Layer Shell surface (Top)
        let height = config.bar.height;
        let bar_layer = match config.bar.layer.to_lowercase().as_str() {
            "overlay" => Layer::Overlay,
            "bottom" => Layer::Bottom,
            "background" => Layer::Background,
            _ => Layer::Top,
        };
        let bar_anchor = match config.bar.anchor.to_lowercase().as_str() {
            "bottom" => Anchor::Bottom | Anchor::Left | Anchor::Right,
            _ => Anchor::Top | Anchor::Left | Anchor::Right,
        };

        let (bar_id, bar_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
            size: LayerSize::fill_width(height),
            layer: bar_layer,
            anchor: bar_anchor,
            exclusive_zone: Some(height as i32),
            keyboard_interactivity: KeyboardInteractivity::None,
            output_option,
            ..Default::default()
        });

        let (wallpaper, wall_cmd) = WallpaperDaemon::new();
        let bar = Bar::new(config.clone(), shell_events);
        let (toast, toast_cmd) = ToastApp::new_embedded();
        let (osd, osd_cmd) = OsdWindow::new_embedded();

        let shell = Self {
            config,
            bar_id: Some(bar_id),
            wallpaper_id: Some(wallpaper_id),
            toast_id: None,
            osd_id: None,
            modal_id: None,
            bar,
            wallpaper,
            toast,
            osd,
            active_modal: None,
            polkit_request: None,
        };

        (
            shell,
            Task::batch([
                wallpaper_task,
                bar_task,
                wall_cmd.map(DaemonMessage::Wallpaper),
                toast_cmd.map(DaemonMessage::Toast),
                osd_cmd.map(DaemonMessage::Osd),
            ]),
        )
    }

    fn open_bar(&mut self) -> Task<DaemonMessage> {
        let height = self.config.bar.height;
        let bar_layer = match self.config.bar.layer.to_lowercase().as_str() {
            "overlay" => Layer::Overlay,
            "bottom" => Layer::Bottom,
            "background" => Layer::Background,
            _ => Layer::Top,
        };
        let bar_anchor = match self.config.bar.anchor.to_lowercase().as_str() {
            "bottom" => Anchor::Bottom | Anchor::Left | Anchor::Right,
            _ => Anchor::Top | Anchor::Left | Anchor::Right,
        };
        let output_option = if let Some(target) = msui_core::DisplayManager::get_primary_monitor_name() {
            OutputOption::OutputName(target)
        } else {
            OutputOption::Active
        };

        let (bid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
            size: LayerSize::fill_width(height),
            layer: bar_layer,
            anchor: bar_anchor,
            exclusive_zone: Some(height as i32),
            keyboard_interactivity: KeyboardInteractivity::None,
            output_option,
            ..Default::default()
        });
        self.bar_id = Some(bid);
        open_task
    }

    fn close_active_modal(&mut self) -> Task<DaemonMessage> {
        let mut tasks = Vec::new();
        if let Some(id) = self.modal_id.take() {
            tracing::info!("Closing active modal {:?}", self.active_modal.as_ref().map(|m| m.kind()));
            tasks.push(iced::window::close(id));
        }
        if let Some(req) = self.polkit_request.take() {
            if let Ok(mut guard) = req.responder.lock() {
                if let Some(tx) = guard.take() {
                    let _ = tx.send(false);
                }
            }
        }
        self.active_modal = None;
        #[cfg(target_os = "linux")]
        unsafe {
            libc::malloc_trim(0);
        }
        Task::batch(tasks)
    }

    fn toggle_modal(&mut self, target: &str) -> Task<DaemonMessage> {
        let target_norm = match target {
            "launcher" => "launcher",
            "power" => "power",
            "control" | "quicksettings" | "cc" => "control",
            "clipboard" | "clip" => "clipboard",
            "notifications" | "notifs" => "notifications",
            "drawer" | "tasks" | "apps" | "appdrawer" => "drawer",
            "wifi" | "network" => "wifi",
            "bluetooth" | "bt" => "bluetooth",
            "audio" | "mixer" => "audio",
            "theme" | "themes" => "theme",
            "wallpaper" | "wall" => "wallpaper",
            "fshare" | "share" | "fileshare" => "fshare",
            _ => return Task::none(),
        };

        // If currently open modal is the same target, toggle it off!
        if self.active_modal.as_ref().map(|m| m.kind()) == Some(target_norm) {
            return self.close_active_modal();
        }

        let mut tasks = Vec::new();

        // If a different modal is open, close its window first
        if let Some(old_id) = self.modal_id.take() {
            tasks.push(iced::window::close(old_id));
        }

        let output_option = if let Some(primary) = msui_core::DisplayManager::get_primary_monitor_name() {
            OutputOption::OutputName(primary)
        } else {
            OutputOption::Active
        };

        match target_norm {
            "launcher" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(self.config.launcher.width, self.config.launcher.height),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let (mut launcher, init_cmd) = Launcher::new_embedded(self.config.clone());
                let reset_cmd = launcher.reset().map(|m| DaemonMessage::Modal(ModalMessage::Launcher(m)));
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::Launcher(launcher));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::Launcher(m))));
                tasks.push(reset_cmd);
            }
            "power" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(500, 260),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let (mut power, init_cmd) = PowerMenu::new_embedded();
                power.reset();
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::Power(power));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::Power(m))));
            }
            "control" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(400, 765),
                    layer: Layer::Overlay,
                    anchor: Anchor::Top | Anchor::Right,
                    margin: Some(Margin {
                        top: self.config.bar.height as i32 + 8,
                        right: 12,
                        bottom: 0,
                        left: 0,
                    }),
                    keyboard_interactivity: KeyboardInteractivity::OnDemand,
                    output_option,
                    ..Default::default()
                });
                let (mut cc, init_cmd) = ControlCenter::new_embedded(self.config.clone());
                let reset_cmd = cc.reset().map(|m| DaemonMessage::Modal(ModalMessage::ControlCenter(m)));
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::ControlCenter(cc));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::ControlCenter(m))));
                tasks.push(reset_cmd);
            }
            "clipboard" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(680, 480),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let (mut clip, init_cmd) = ClipboardMenu::new_embedded();
                let reset_cmd = clip.reset().map(|m| DaemonMessage::Modal(ModalMessage::Clipboard(m)));
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::Clipboard(clip));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::Clipboard(m))));
                tasks.push(reset_cmd);
            }
            "notifications" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(680, 520),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let (mut notifs, init_cmd) = NotificationCenter::new_embedded();
                let reset_cmd = notifs.reset().map(|m| DaemonMessage::Modal(ModalMessage::NotificationCenter(m)));
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::NotificationCenter(notifs));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::NotificationCenter(m))));
                tasks.push(reset_cmd);
            }
            "drawer" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(780, 620),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let (mut drawer, init_cmd) = AppDrawer::new_embedded(self.config.clone());
                let reset_cmd = drawer.reset().map(|m| DaemonMessage::Modal(ModalMessage::AppDrawer(m)));
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::AppDrawer(drawer));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::AppDrawer(m))));
                tasks.push(reset_cmd);
            }
            "wifi" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(680, 560),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let (mut wifi, init_cmd) = WifiMenu::new_embedded();
                let reset_cmd = wifi.reset().map(|m| DaemonMessage::Modal(ModalMessage::Wifi(m)));
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::Wifi(wifi));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::Wifi(m))));
                tasks.push(reset_cmd);
            }
            "bluetooth" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(680, 500),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let (mut bt, init_cmd) = BluetoothMenu::new_embedded();
                let reset_cmd = bt.reset().map(|m| DaemonMessage::Modal(ModalMessage::Bluetooth(m)));
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::Bluetooth(bt));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::Bluetooth(m))));
                tasks.push(reset_cmd);
            }
            "audio" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(680, 580),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let (mut audio, init_cmd) = AudioMenu::new_embedded();
                let reset_cmd = audio.reset().map(|m| DaemonMessage::Modal(ModalMessage::Audio(m)));
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::Audio(audio));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::Audio(m))));
                tasks.push(reset_cmd);
            }
            "theme" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(680, 520),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let (mut theme, init_cmd) = ThemeMenu::new_embedded();
                let reset_cmd = theme.reset().map(|m| DaemonMessage::Modal(ModalMessage::Theme(m)));
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::Theme(theme));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::Theme(m))));
                tasks.push(reset_cmd);
            }
            "wallpaper" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(720, 540),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let (mut wall, init_cmd) = WallpaperMenu::new_embedded();
                let reset_cmd = wall.reset().map(|m| DaemonMessage::Modal(ModalMessage::Wallpaper(m)));
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::Wallpaper(wall));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::Wallpaper(m))));
                tasks.push(reset_cmd);
            }
            "fshare" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(680, 560),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let (mut fshare, init_cmd) = FShareModal::new_embedded();
                let reset_cmd = fshare.reset().map(|m| DaemonMessage::Modal(ModalMessage::FShare(m)));
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::FShare(fshare));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::FShare(m))));
                tasks.push(reset_cmd);
            }
            "battery" | "bat" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(680, 560),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let (mut bat, init_cmd) = BatteryMenu::new_embedded();
                let reset_cmd = bat.reset().map(|m| DaemonMessage::Modal(ModalMessage::Battery(m)));
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::Battery(bat));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::Battery(m))));
                tasks.push(reset_cmd);
            }
            "display" | "disp" | "monitors" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(720, 560),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let (mut disp, init_cmd) = DisplayMenu::new_embedded();
                let reset_cmd = disp.reset().map(|m| DaemonMessage::Modal(ModalMessage::Display(m)));
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::Display(disp));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::Display(m))));
                tasks.push(reset_cmd);
            }
            "media" | "player" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(540, 260),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let (mut med, init_cmd) = MediaMenu::new_embedded();
                let reset_cmd = med.reset().map(|m| DaemonMessage::Modal(ModalMessage::Media(m)));
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::Media(med));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::Media(m))));
                tasks.push(reset_cmd);
            }
            "reminder" | "reminders" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(640, 540),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let (mut rem, init_cmd) = ReminderMenu::new_embedded();
                let reset_cmd = rem.reset().map(|m| DaemonMessage::Modal(ModalMessage::Reminder(m)));
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::Reminder(rem));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::Reminder(m))));
                tasks.push(reset_cmd);
            }
            "storage" | "disks" => {
                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(680, 540),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let (mut stor, init_cmd) = StorageMenu::new_embedded();
                let reset_cmd = stor.reset().map(|m| DaemonMessage::Modal(ModalMessage::Storage(m)));
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::Storage(stor));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::Storage(m))));
                tasks.push(reset_cmd);
            }
            _ => {}
        }

        Task::batch(tasks)
    }

    fn namespace() -> String {
        String::from("msui-desktop-daemon")
    }

    fn theme(&self, _id: iced::window::Id) -> Option<iced::Theme> {
        Some(ThemeManager::current_theme())
    }

    fn scale_factor(&self, _id: iced::window::Id) -> f32 {
        1.0
    }

    fn subscription(&self) -> iced::Subscription<DaemonMessage> {
        let wall_sub = self.wallpaper.subscription().map(DaemonMessage::Wallpaper);
        let toast_sub = self.toast.subscription().map(DaemonMessage::Toast);
        let osd_sub = self.osd.subscription().map(DaemonMessage::Osd);
        let close_sub = iced::window::close_events().map(DaemonMessage::WindowClosed);

        let bar_sub = if self.bar_id.is_some() {
            self.bar.subscription().map(DaemonMessage::Bar)
        } else {
            iced::Subscription::none()
        };

        let modal_sub = if let Some(modal) = &self.active_modal {
            match modal {
                ActiveModal::Launcher(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::Launcher(msg))),
                ActiveModal::Power(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::Power(msg))),
                ActiveModal::ControlCenter(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::ControlCenter(msg))),
                ActiveModal::Clipboard(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::Clipboard(msg))),
                ActiveModal::NotificationCenter(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::NotificationCenter(msg))),
                ActiveModal::AppDrawer(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::AppDrawer(msg))),
                ActiveModal::Wifi(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::Wifi(msg))),
                ActiveModal::Bluetooth(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::Bluetooth(msg))),
                ActiveModal::Audio(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::Audio(msg))),
                ActiveModal::Theme(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::Theme(msg))),
                ActiveModal::Wallpaper(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::Wallpaper(msg))),
                ActiveModal::FShare(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::FShare(msg))),
                ActiveModal::Battery(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::Battery(msg))),
                ActiveModal::Display(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::Display(msg))),
                ActiveModal::Media(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::Media(msg))),
                ActiveModal::Reminder(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::Reminder(msg))),
                ActiveModal::Storage(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::Storage(msg))),
                ActiveModal::Polkit(m) => m.subscription().map(|msg| DaemonMessage::Modal(ModalMessage::Polkit(msg))),
            }
        } else {
            iced::Subscription::none()
        };

        let ipc_sub = iced::Subscription::run(ipc_stream);
        let polkit_sub = iced::Subscription::run(polkit_stream);

        iced::Subscription::batch([
            bar_sub,
            wall_sub,
            toast_sub,
            osd_sub,
            modal_sub,
            close_sub,
            ipc_sub,
            polkit_sub,
        ])
    }

    fn update(&mut self, message: DaemonMessage) -> Task<DaemonMessage> {
        match message {
            DaemonMessage::Bar(msg) => {
                match &msg {
                    BarMessage::OpenLauncher => return self.toggle_modal("launcher"),
                    BarMessage::OpenPowerMenu => return self.toggle_modal("power"),
                    BarMessage::OpenControlCenter => return self.toggle_modal("control"),
                    BarMessage::OpenClipboard => return self.toggle_modal("clipboard"),
                    BarMessage::OpenNotifications => return self.toggle_modal("notifications"),
                    BarMessage::OpenDrawer => return self.toggle_modal("drawer"),
                    BarMessage::OpenWifi => return self.toggle_modal("wifi"),
                    BarMessage::OpenBluetooth => return self.toggle_modal("bluetooth"),
                    BarMessage::OpenAudio => return self.toggle_modal("audio"),
                    BarMessage::OpenTheme => return self.toggle_modal("theme"),
                    BarMessage::OpenWallpaper => return self.toggle_modal("wallpaper"),
                    BarMessage::OpenBattery => return self.toggle_modal("battery"),
                    BarMessage::OpenDisplay => return self.toggle_modal("display"),
                    BarMessage::OpenMedia => return self.toggle_modal("media"),
                    BarMessage::OpenStorage => return self.toggle_modal("storage"),
                    BarMessage::OpenReminders => return self.toggle_modal("reminder"),
                    _ => {}
                }
                self.bar.update(msg).map(DaemonMessage::Bar)
            }
            DaemonMessage::Wallpaper(msg) => {
                self.wallpaper.update(msg).map(DaemonMessage::Wallpaper)
            }
            DaemonMessage::Modal(msg) => {
                let cmd = match (msg, self.active_modal.as_mut()) {
                    (ModalMessage::Launcher(m), Some(ActiveModal::Launcher(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::Launcher(x)))
                    }
                    (ModalMessage::Power(m), Some(ActiveModal::Power(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::Power(x)))
                    }
                    (ModalMessage::ControlCenter(m), Some(ActiveModal::ControlCenter(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::ControlCenter(x)))
                    }
                    (ModalMessage::Clipboard(m), Some(ActiveModal::Clipboard(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::Clipboard(x)))
                    }
                    (ModalMessage::NotificationCenter(m), Some(ActiveModal::NotificationCenter(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::NotificationCenter(x)))
                    }
                    (ModalMessage::AppDrawer(m), Some(ActiveModal::AppDrawer(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::AppDrawer(x)))
                    }
                    (ModalMessage::Wifi(m), Some(ActiveModal::Wifi(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::Wifi(x)))
                    }
                    (ModalMessage::Bluetooth(m), Some(ActiveModal::Bluetooth(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::Bluetooth(x)))
                    }
                    (ModalMessage::Audio(m), Some(ActiveModal::Audio(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::Audio(x)))
                    }
                    (ModalMessage::Theme(m), Some(ActiveModal::Theme(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::Theme(x)))
                    }
                    (ModalMessage::Wallpaper(m), Some(ActiveModal::Wallpaper(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::Wallpaper(x)))
                    }
                    (ModalMessage::FShare(m), Some(ActiveModal::FShare(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::FShare(x)))
                    }
                    (ModalMessage::Battery(m), Some(ActiveModal::Battery(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::Battery(x)))
                    }
                    (ModalMessage::Display(m), Some(ActiveModal::Display(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::Display(x)))
                    }
                    (ModalMessage::Media(m), Some(ActiveModal::Media(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::Media(x)))
                    }
                    (ModalMessage::Reminder(m), Some(ActiveModal::Reminder(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::Reminder(x)))
                    }
                    (ModalMessage::Storage(m), Some(ActiveModal::Storage(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::Storage(x)))
                    }
                    (ModalMessage::Polkit(m), Some(ActiveModal::Polkit(inst))) => {
                        inst.update(m).map(|x| DaemonMessage::Modal(ModalMessage::Polkit(x)))
                    }
                    _ => Task::none(),
                };

                if let Some(modal) = &self.active_modal {
                    if modal.should_close() {
                        if let ActiveModal::Polkit(dlg) = modal {
                            let success = dlg.auth_success().unwrap_or(false);
                            if let Some(req) = self.polkit_request.take() {
                                if let Ok(mut guard) = req.responder.lock() {
                                    if let Some(tx) = guard.take() {
                                        let _ = tx.send(success);
                                    }
                                }
                            }
                        }
                        return Task::batch([cmd, self.close_active_modal()]);
                    }
                }
                cmd
            }
            DaemonMessage::Toast(msg) => {
                let toast_cmd = self.toast.update(msg).map(DaemonMessage::Toast);

                if self.toast.is_empty() {
                    if let Some(tid) = self.toast_id.take() {
                        return Task::batch([toast_cmd, iced::window::close(tid)]);
                    }
                } else if self.toast_id.is_none() {
                    let output_option = if let Some(target) = msui_core::DisplayManager::get_primary_monitor_name() {
                        OutputOption::OutputName(target)
                    } else {
                        OutputOption::Active
                    };

                    let (tid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                        size: LayerSize::px(380, 250),
                        layer: Layer::Overlay,
                        anchor: Anchor::Top | Anchor::Right,
                        margin: Some(Margin {
                            top: 40,
                            right: 16,
                            bottom: 0,
                            left: 0,
                        }),
                        keyboard_interactivity: KeyboardInteractivity::None,
                        output_option,
                        ..Default::default()
                    });
                    self.toast_id = Some(tid);
                    return Task::batch([toast_cmd, open_task]);
                }

                toast_cmd
            }
            DaemonMessage::Osd(msg) => {
                let osd_cmd = self.osd.update(msg).map(DaemonMessage::Osd);
                if !self.osd.is_active() {
                    if let Some(oid) = self.osd_id.take() {
                        return Task::batch([osd_cmd, iced::window::close(oid)]);
                    }
                }
                osd_cmd
            }
            DaemonMessage::Ipc(cmd) => {
                match cmd {
                    IpcCommand::Toggle(target) if target == "bar" => {
                        if let Some(bid) = self.bar_id.take() {
                            tracing::info!("Toggle Bar: Closing bar window");
                            iced::window::close(bid)
                        } else {
                            tracing::info!("Toggle Bar: Opening bar window");
                            self.open_bar()
                        }
                    }
                    IpcCommand::Toggle(target) => {
                        self.toggle_modal(&target)
                    }
                    IpcCommand::Open(target) if target == "bar" => {
                        if self.bar_id.is_none() {
                            tracing::info!("Open Bar: Opening bar window");
                            self.open_bar()
                        } else {
                            Task::none()
                        }
                    }
                    IpcCommand::Open(target) => {
                        if self.active_modal.as_ref().map(|m| m.kind()) != Some(&target) {
                            self.toggle_modal(&target)
                        } else {
                            Task::none()
                        }
                    }
                    IpcCommand::Close => {
                        if self.modal_id.is_some() {
                            self.close_active_modal()
                        } else if let Some(bid) = self.bar_id.take() {
                            tracing::info!("Close Bar: Closing bar window");
                            iced::window::close(bid)
                        } else {
                            Task::none()
                        }
                    }
                    IpcCommand::ReloadConfig => {
                        tracing::info!("ReloadConfig: Reloading configuration from disk...");
                        self.config = MsuiConfig::load_or_default();
                        self.bar.reload_config(self.config.clone());
                        Task::none()
                    }
                    IpcCommand::WallpaperCycle => {
                        WallpaperManager::cycle();
                        let task = self.wallpaper.update(WallpaperMessage::CheckFile).map(DaemonMessage::Wallpaper);
                        #[cfg(target_os = "linux")]
                        unsafe {
                            libc::malloc_trim(0);
                        }
                        task
                    }
                    IpcCommand::DndToggle => {
                        msui_core::DndManager::toggle_dnd();
                        Task::none()
                    }
                    IpcCommand::DndSet(on) => {
                        msui_core::DndManager::set_dnd(on);
                        Task::none()
                    }
                    IpcCommand::Osd { kind, action } => {
                        match kind.to_lowercase().as_str() {
                            "volume" | "vol" => {
                                match action.to_lowercase().as_str() {
                                    "up" | "+" => { AudioMonitor::increase_volume(5); }
                                    "down" | "-" => { AudioMonitor::decrease_volume(5); }
                                    "mute" | "toggle-mute" => { AudioMonitor::toggle_mute(); }
                                    val if val.parse::<u8>().is_ok() => {
                                        let v = val.parse::<u8>().unwrap();
                                        AudioMonitor::set_volume(v);
                                    }
                                    _ => {}
                                }
                                let audio = AudioMonitor::read();
                                self.osd.show(OsdKind::Volume, audio.volume_percent.min(100), audio.is_muted);
                            }
                            "brightness" | "bright" => {
                                match action.to_lowercase().as_str() {
                                    "up" | "+" => { BrightnessMonitor::increase(5); }
                                    "down" | "-" => { BrightnessMonitor::decrease(5); }
                                    val if val.parse::<u8>().is_ok() => {
                                        let v = val.parse::<u8>().unwrap();
                                        BrightnessMonitor::set(v);
                                    }
                                    _ => {}
                                }
                                let bright_mon = BrightnessMonitor::new();
                                let bright = bright_mon.read().map(|b| b.percent).unwrap_or(50);
                                self.osd.show(OsdKind::Brightness, bright, false);
                            }
                            _ => {}
                        }

                        if self.osd_id.is_none() {
                            let output_option = if let Some(target) = msui_core::DisplayManager::get_primary_monitor_name() {
                                OutputOption::OutputName(target)
                            } else {
                                OutputOption::Active
                            };

                            let (oid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                                size: LayerSize::px(280, 64),
                                layer: Layer::Overlay,
                                anchor: Anchor::Bottom,
                                margin: Some(Margin {
                                    top: 0,
                                    right: 0,
                                    bottom: 80,
                                    left: 0,
                                }),
                                keyboard_interactivity: KeyboardInteractivity::None,
                                output_option,
                                ..Default::default()
                            });
                            self.osd_id = Some(oid);
                            open_task
                        } else {
                            Task::none()
                        }
                    }
                    IpcCommand::NotificationAction { id, action } => {
                        tokio::spawn(async move {
                            let _ = msui_core::emit_action_invoked(id, &action).await;
                            let _ = msui_core::emit_notification_closed(id, 2).await;
                        });
                        Task::none()
                    }
                    IpcCommand::NotificationClose { id, reason } => {
                        tokio::spawn(async move {
                            let _ = msui_core::emit_notification_closed(id, reason).await;
                        });
                        Task::none()
                    }
                    _ => Task::none(),
                }
            }
            DaemonMessage::PolkitRequest(req) => {
                let mut tasks = Vec::new();
                if let Some(mid) = self.modal_id.take() {
                    tasks.push(iced::window::close(mid));
                }
                if let Some(old_req) = self.polkit_request.take() {
                    if let Ok(mut guard) = old_req.responder.lock() {
                        if let Some(tx) = guard.take() {
                            let _ = tx.send(false);
                        }
                    }
                }
                self.polkit_request = Some(req.clone());

                let output_option = if let Some(target) = msui_core::DisplayManager::get_primary_monitor_name() {
                    OutputOption::OutputName(target)
                } else {
                    OutputOption::Active
                };

                let (mid, open_task) = DaemonMessage::layershell_open(NewLayerShellSettings {
                    size: LayerSize::px(480, 360),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(), // Centered on screen
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option,
                    ..Default::default()
                });
                let args = PolkitDialogArgs {
                    action_id: req.action_id.clone(),
                    message: req.message.clone(),
                    icon_name: req.icon_name.clone(),
                    user: req.user.clone(),
                    cookie: req.cookie.clone(),
                    command: req.command.clone(),
                };
                let (dlg, init_cmd) = PolkitDialog::new_embedded(args);
                self.modal_id = Some(mid);
                self.active_modal = Some(ActiveModal::Polkit(dlg));
                tasks.push(open_task);
                tasks.push(init_cmd.map(|m| DaemonMessage::Modal(ModalMessage::Polkit(m))));
                return Task::batch(tasks);
            }
            DaemonMessage::PolkitCancel(cookie) => {
                if let Some(req) = &self.polkit_request {
                    if req.cookie == cookie {
                        return self.close_active_modal();
                    }
                }
                Task::none()
            }
            DaemonMessage::WindowClosed(id) => {
                if Some(id) == self.toast_id {
                    self.toast_id = None;
                }
                if Some(id) == self.bar_id {
                    self.bar_id = None;
                }
                if Some(id) == self.wallpaper_id {
                    self.wallpaper_id = None;
                }
                if Some(id) == self.osd_id {
                    self.osd_id = None;
                }
                if Some(id) == self.modal_id {
                    self.modal_id = None;
                    self.active_modal = None;
                    if let Some(req) = self.polkit_request.take() {
                        if let Ok(mut guard) = req.responder.lock() {
                            if let Some(tx) = guard.take() {
                                let _ = tx.send(false);
                            }
                        }
                    }
                }
                #[cfg(target_os = "linux")]
                unsafe {
                    libc::malloc_trim(0);
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn view(&self, id: iced::window::Id) -> Element<'_, DaemonMessage> {
        if Some(id) == self.wallpaper_id {
            self.wallpaper.view().map(DaemonMessage::Wallpaper)
        } else if Some(id) == self.bar_id {
            self.bar.view().map(DaemonMessage::Bar)
        } else if Some(id) == self.toast_id {
            self.toast.view().map(DaemonMessage::Toast)
        } else if Some(id) == self.osd_id {
            self.osd.view().map(DaemonMessage::Osd)
        } else if Some(id) == self.modal_id {
            if let Some(modal) = &self.active_modal {
                match modal {
                    ActiveModal::Launcher(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::Launcher(x))),
                    ActiveModal::Power(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::Power(x))),
                    ActiveModal::ControlCenter(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::ControlCenter(x))),
                    ActiveModal::Clipboard(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::Clipboard(x))),
                    ActiveModal::NotificationCenter(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::NotificationCenter(x))),
                    ActiveModal::AppDrawer(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::AppDrawer(x))),
                    ActiveModal::Wifi(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::Wifi(x))),
                    ActiveModal::Bluetooth(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::Bluetooth(x))),
                    ActiveModal::Audio(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::Audio(x))),
                    ActiveModal::Theme(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::Theme(x))),
                    ActiveModal::Wallpaper(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::Wallpaper(x))),
                    ActiveModal::FShare(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::FShare(x))),
                    ActiveModal::Battery(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::Battery(x))),
                    ActiveModal::Display(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::Display(x))),
                    ActiveModal::Media(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::Media(x))),
                    ActiveModal::Reminder(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::Reminder(x))),
                    ActiveModal::Storage(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::Storage(x))),
                    ActiveModal::Polkit(m) => m.view().map(|x| DaemonMessage::Modal(ModalMessage::Polkit(x))),
                }
            } else {
                container(space()).width(Length::Shrink).height(Length::Shrink).into()
            }
        } else {
            container(space()).width(Length::Shrink).height(Length::Shrink).into()
        }
    }
}

fn spawn_config_file_watcher() {
    #[cfg(target_os = "linux")]
    tokio::spawn(async {
        use std::os::unix::ffi::OsStrExt;

        let inotify_fd = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
        if inotify_fd < 0 {
            tracing::warn!("Failed to initialize inotify for auto config hot-reloading");
            return;
        }

        let mut watch_dirs = Vec::new();
        if let Some(cfg_path) = MsuiConfig::default_config_path() {
            if let Some(parent) = cfg_path.parent() {
                let _ = std::fs::create_dir_all(parent);
                watch_dirs.push(parent.to_path_buf());
            }
        }
        if let Some(theme_path) = ThemeManager::cache_file_path() {
            if let Some(parent) = theme_path.parent() {
                let _ = std::fs::create_dir_all(parent);
                watch_dirs.push(parent.to_path_buf());
            }
        }

        for dir in &watch_dirs {
            let mut c_path = dir.as_os_str().as_bytes().to_vec();
            c_path.push(0);
            let mask = libc::IN_CLOSE_WRITE | libc::IN_MOVED_TO | libc::IN_CREATE;
            let wd = unsafe { libc::inotify_add_watch(inotify_fd, c_path.as_ptr() as *const libc::c_char, mask) };
            if wd >= 0 {
                tracing::info!("Live config hot-reloader active on {}", dir.display());
            }
        }

        let mut buf = [0u8; 4096];
        loop {
            let res = unsafe { libc::read(inotify_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if res > 0 {
                // Debounce rapid writes from text editors (swap files, multi-writes, atomic renames)
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                while unsafe { libc::read(inotify_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) } > 0 {}
                tracing::info!("Auto-detected msui.toml / theme changes on disk! Live hot-reloading...");
                let _ = IPC_BROADCAST_TX.send(IpcCommand::ReloadConfig);
            } else {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            }
        }
    });
}

pub fn run_daemon(config: MsuiConfig) -> anyhow::Result<()> {
    tracing::info!("Starting unified msUI Desktop Shell Daemon (Single Process)...");

    // Signal notification service to avoid spawning external toast child process
    msui_core::notification::service::IS_UNIFIED_DAEMON.store(true, std::sync::atomic::Ordering::SeqCst);

    // Start background Tokio runtime for clipboard watcher, D-Bus notifications, Polkit, SNI tray, IPC server
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_stack_size(1024 * 1024)
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        rt.block_on(async move {
            tokio::spawn(msui_core::start_clipboard_watcher());
            tokio::spawn(msui_core::run_notification_service());
            tokio::spawn(msui_core::run_tray_watcher());
            spawn_config_file_watcher();
            tokio::spawn(async {
                if let Err(e) = msui_core::run_polkit_agent().await {
                    tracing::error!("msUI Polkit Agent error: {:?}", e);
                }
            });

            let (tx, mut rx) = tokio::sync::mpsc::channel(32);
            let server = IpcServer::new();
            if let Err(e) = server.start(tx).await {
                tracing::warn!("Failed to start IPC server: {:?}", e);
            } else {
                tokio::spawn(async move {
                    let mut modal_mgr = ModalManager::new();
                    while let Some(cmd) = rx.recv().await {
                        // Forward to Iced event loop for bar toggle, wallpaper, DND, modals, etc.
                        let _ = IPC_BROADCAST_TX.send(cmd.clone());
                        // Handle modal lifecycles for any non-in-process modals
                        modal_mgr.handle_command(cmd).await;
                    }
                });
            }

            // Periodic 60-second heap memory trim to prevent long-running fragmentation
            tokio::spawn(async {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    #[cfg(target_os = "linux")]
                    unsafe {
                        libc::malloc_trim(0);
                    }
                }
            });

            // Reclaim initial startup memory allocations immediately
            #[cfg(target_os = "linux")]
            unsafe {
                libc::malloc_trim(0);
            }

            std::future::pending::<()>().await;
            drop(server);
        });
    });

    let (shell_broadcast, shell_events) = shell::channel();

    let cfg_clone = config.clone();

    let res = daemon(
        move || UnifiedShell::new(cfg_clone.clone(), shell_events.clone()),
        UnifiedShell::namespace,
        UnifiedShell::update,
        UnifiedShell::view,
    )
    .theme(UnifiedShell::theme)
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .subscription(UnifiedShell::subscription)
    .scale_factor(UnifiedShell::scale_factor)
    .wl_settings(ExWlSettings {
        layer_settings: LayerShellSettings {
            start_mode: StartMode::Background,
            ..Default::default()
        },
        shell_broadcast,
        keep_compositor_alive: true,
        ..Default::default()
    })
    .run();

    if let Err(e) = res {
        tracing::error!("msUI unified daemon exited with error: {:?}", e);
    }

    Ok(())
}
