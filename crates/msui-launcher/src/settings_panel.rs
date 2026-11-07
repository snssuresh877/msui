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
    AudioInfo, AudioMonitor, BatteryMonitor, ColorEngine, ColorSchemeMode,
    CorePalette, DndManager, MsuiConfig, PowerProfile, PowerProfileManager, SemanticTokens,
    SystemServiceInfo, SystemServiceManager, ThemeManager, WallpaperManager, PRESETS,
};
use std::process::Command as StdCommand;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Wallpaper,
    Interface,
    Fonts,
    Bar,
    BarWidgets,
    Dock,
    Launcher,
    Dashboard,
    Notifications,
    Osd,
    Services,
    Power,
    Audio,
}

pub struct SettingsPanel {
    pub config: MsuiConfig,
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
    pub active_blur_preset: u8,
    transparency_enabled: bool,
    last_active_preset: u8,
    animations_enabled: bool,
    frosted_enabled: bool,
    dnd_enabled: bool,
    current_power_profile: PowerProfile,
    conservation_mode: Option<bool>,
    audio_info: AudioInfo,
    active_gaps: (u32, u32),
    active_border_color: String,
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
    ToggleTransparency,
    SetBlurPreset(u8),
    ToggleAnimations,
    ToggleFrosted,
    ToggleDnd,
    SetTheme(String),
    ColorExtractWallpaper,
    ColorSetPreset(String),
    ColorSetMode(ColorSchemeMode),
    ColorSetAccent(String),
    WallpaperNext,
    WallpaperRandom,
    SetBarPosition(String),
    SetBarStyle(String),
    SetBarHeight(u32),
    SetBarTransparency(String),
    SetTimeFormat(String),
    SetWorkspaceCount(usize),
    ToggleBarWidget(&'static str),
    SetLauncherDimensions(u32, u32),
    SetTerminal(String),
    SetMaxResults(usize),
    SetGaps(u32, u32),
    SetBorderColor(String),
    ToggleGuiBorder(bool),
    SetGuiBorderColor(String),
    SetGuiBorderWidth(f32),
    SetGuiBackground(String),
    SetGuiCornerRadius(f32),
    SetPowerProfile(PowerProfile),
    ToggleAutoAdaptive(bool),
    ToggleConservationMode(bool),
    ConservationModeResult(Result<bool, String>),
    StepVolume(i8),
    ToggleAudioMute,
    LaunchPavucontrol,
    SetAnimationSpeed(&'static str),
    SetDockPosition(String),
    SetDockSize(u32),
    SetWeatherUnit(String),
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

pub fn run_settings_panel(config: MsuiConfig) -> Result<(), iced_exwlshell::Error> {
    let width = 960;
    let height = 660;

    application(
        move || SettingsPanel::new_internal(config.clone(), true),
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

fn detect_current_blur_preset() -> u8 {
    let enabled = StdCommand::new("hyprctl")
        .args(["getoption", "decoration:blur:enabled", "-j"])
        .output()
        .map(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            s.contains("\"bool\": true") || s.contains("true")
        })
        .unwrap_or(true);

    if !enabled {
        let op_output = StdCommand::new("hyprctl")
            .args(["getoption", "decoration:active_opacity", "-j"])
            .output();
        let is_solid = op_output
            .map(|o| {
                let s = String::from_utf8_lossy(&o.stdout);
                s.contains("1.0") || s.contains("1")
            })
            .unwrap_or(false);
        if is_solid {
            11
        } else {
            1
        }
    } else {
        let var_output = StdCommand::new("hyprctl")
            .args(["getoption", "decoration:blur:variant", "-j"])
            .output();
        let variant_str = var_output
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        if variant_str.contains("\"int\": 1") || variant_str.contains("\"str\": \"drops\"") {
            2
        } else if variant_str.contains("\"int\": 2") || variant_str.contains("\"str\": \"fluid_jar\"") {
            3
        } else if variant_str.contains("\"int\": 3") || variant_str.contains("\"str\": \"prism\"") {
            4
        } else if variant_str.contains("\"int\": 4") || variant_str.contains("\"str\": \"frost\"") {
            5
        } else if variant_str.contains("\"int\": 5") || variant_str.contains("\"str\": \"acrylic\"") {
            6
        } else if variant_str.contains("\"int\": 6") || variant_str.contains("\"str\": \"aurora\"") {
            7
        } else if variant_str.contains("\"int\": 7") || variant_str.contains("\"str\": \"water\"") {
            8
        } else if variant_str.contains("\"int\": 8") || variant_str.contains("\"str\": \"heat_shimmer\"") {
            9
        } else {
            10 // default kawase
        }
    }
}

impl SettingsPanel {
    pub fn new() -> (Self, Command<SettingsMessage>) {
        Self::new_internal(MsuiConfig::load_or_default(), true)
    }

    pub fn new_embedded(config: MsuiConfig) -> (Self, Command<SettingsMessage>) {
        Self::new_internal(config, false)
    }

    fn new_internal(config: MsuiConfig, is_standalone: bool) -> (Self, Command<SettingsMessage>) {
        let services = SystemServiceManager::list_services();
        let dnd_enabled = DndManager::is_dnd_enabled();
        let audio_info = AudioMonitor::read();
        let current_power_profile = PowerProfileManager::get_profile();
        let active_blur_preset = detect_current_blur_preset();
        let transparency_enabled = active_blur_preset != 11;
        let last_active_preset = if active_blur_preset != 11 { active_blur_preset } else { 10 };

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

        let frosted_enabled = config.bar.is_frosted();

        (
            Self {
                config,
                active_tab: SettingsTab::Wallpaper,
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
                active_blur_preset,
                transparency_enabled,
                last_active_preset,
                animations_enabled,
                frosted_enabled,
                dnd_enabled,
                current_power_profile,
                conservation_mode: BatteryMonitor::read_conservation_mode(),
                audio_info,
                active_gaps: (5, 12),
                active_border_color: "rgba(9333eaff)".to_string(),
                status_feedback: None,
                last_toggle_time: None,
            },
            Command::none(),
        )
    }

    pub fn reset(&mut self) -> Command<SettingsMessage> {
        self.config = MsuiConfig::load_or_default();
        self.services = SystemServiceManager::list_services();
        self.dnd_enabled = DndManager::is_dnd_enabled();
        self.audio_info = AudioMonitor::read();
        self.current_power_profile = PowerProfileManager::get_profile();
        self.conservation_mode = BatteryMonitor::read_conservation_mode();
        self.active_blur_preset = detect_current_blur_preset();
        self.transparency_enabled = self.active_blur_preset != 11;
        if self.active_blur_preset != 11 {
            self.last_active_preset = self.active_blur_preset;
        }
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
                let preset = if self.blur_enabled {
                    if self.last_active_preset != 1 && self.last_active_preset != 11 {
                        self.last_active_preset
                    } else {
                        10
                    }
                } else {
                    1
                };
                self.active_blur_preset = preset;
                self.transparency_enabled = true;
                if preset != 11 {
                    self.last_active_preset = preset;
                }
                self.status_feedback = Some(if self.blur_enabled {
                    "Enabled GPU Blur".to_string()
                } else {
                    "Disabled Blur (Translucent Windows)".to_string()
                });
                std::thread::spawn(move || {
                    let _ = StdCommand::new("hypr-blur")
                        .arg(preset.to_string())
                        .status();
                });
            }
            SettingsMessage::ToggleTransparency => {
                self.transparency_enabled = !self.transparency_enabled;
                let active = self.transparency_enabled;
                let preset = if active {
                    if self.last_active_preset != 11 {
                        self.last_active_preset
                    } else {
                        10
                    }
                } else {
                    11
                };

                self.active_blur_preset = preset;
                self.blur_enabled = preset != 1 && preset != 11;

                self.config.bar.transparency = if active {
                    if self.frosted_enabled { "frosted".to_string() } else { "transparent".to_string() }
                } else {
                    "solid".to_string()
                };
                let _ = self.config.save_to_default_path();

                self.status_feedback = Some(if active {
                    "Enabled Window Transparency (0.84 Translucency)".to_string()
                } else {
                    "Disabled Transparency (100% Solid Opaque Windows)".to_string()
                });

                std::thread::spawn(move || {
                    let _ = StdCommand::new("hypr-blur")
                        .arg(preset.to_string())
                        .status();
                });
            }
            SettingsMessage::SetBlurPreset(preset) => {
                self.active_blur_preset = preset;
                self.blur_enabled = preset != 1 && preset != 11;
                self.transparency_enabled = preset != 11;
                if preset != 11 {
                    self.last_active_preset = preset;
                }
                self.status_feedback = Some(format!("Applied blur preset #{}", preset));
                std::thread::spawn(move || {
                    let _ = StdCommand::new("hypr-blur")
                        .arg(preset.to_string())
                        .status();
                });
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
                self.config.bar.transparency = if self.frosted_enabled {
                    "frosted".to_string()
                } else {
                    "solid".to_string()
                };
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::ToggleDnd => {
                self.dnd_enabled = DndManager::toggle_dnd();
            }
            SettingsMessage::SetTheme(name) => {
                ThemeManager::set_theme(&name);
            }
            SettingsMessage::ColorExtractWallpaper => {
                if let Some(cur) = WallpaperManager::get_current() {
                    let mode = if self.config.color.scheme == "light" {
                        ColorSchemeMode::Light
                    } else {
                        ColorSchemeMode::Dark
                    };
                    if let Ok(tokens) = ColorEngine::apply_wallpaper(&cur, mode) {
                        self.config.color.mode = "dynamic".to_string();
                        self.config.gui.background = tokens.background.clone();
                        self.config.gui.border_color = tokens.primary.clone();
                        let _ = self.config.save_to_default_path();
                    }
                }
            }
            SettingsMessage::ColorSetPreset(preset_id) => {
                let mode = if self.config.color.scheme == "light" {
                    ColorSchemeMode::Light
                } else {
                    ColorSchemeMode::Dark
                };
                if let Ok(tokens) = ColorEngine::apply_preset(&preset_id, mode) {
                    self.config.color.mode = "preset".to_string();
                    self.config.color.active_preset = preset_id;
                    self.config.gui.background = tokens.background.clone();
                    self.config.gui.border_color = tokens.primary.clone();
                    let _ = self.config.save_to_default_path();
                }
            }
            SettingsMessage::ColorSetMode(mode) => {
                self.config.color.scheme = match mode {
                    ColorSchemeMode::Light => "light".to_string(),
                    ColorSchemeMode::Dark => "dark".to_string(),
                };
                let tokens = ColorEngine::current_tokens();
                let pal = CorePalette::from_hex(&tokens.primary)
                    .unwrap_or_else(|| CorePalette::from_hex("#9333ea").unwrap());
                let new_tokens = SemanticTokens::from_palette(&pal, mode);
                let _ = ColorEngine::save_and_apply(&new_tokens);
                self.config.gui.background = new_tokens.background.clone();
                self.config.gui.border_color = new_tokens.primary.clone();
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::ColorSetAccent(hex) => {
                let mode = if self.config.color.scheme == "light" {
                    ColorSchemeMode::Light
                } else {
                    ColorSchemeMode::Dark
                };
                if let Ok(tokens) = ColorEngine::apply_accent(&hex, mode) {
                    self.config.color.mode = "accent".to_string();
                    self.config.color.accent_override = hex.clone();
                    self.config.gui.background = tokens.background.clone();
                    self.config.gui.border_color = tokens.primary.clone();
                    let _ = self.config.save_to_default_path();
                }
            }
            SettingsMessage::WallpaperNext => {
                if let Some(next) = WallpaperManager::cycle() {
                    if self.config.color.mode == "dynamic" {
                        let mode = if self.config.color.scheme == "light" {
                            ColorSchemeMode::Light
                        } else {
                            ColorSchemeMode::Dark
                        };
                        let _ = ColorEngine::apply_wallpaper(&next, mode);
                    }
                }
            }
            SettingsMessage::WallpaperRandom => {
                if let Some(next) = WallpaperManager::random() {
                    if self.config.color.mode == "dynamic" {
                        let mode = if self.config.color.scheme == "light" {
                            ColorSchemeMode::Light
                        } else {
                            ColorSchemeMode::Dark
                        };
                        let _ = ColorEngine::apply_wallpaper(&next, mode);
                    }
                }
            }
            SettingsMessage::SetBarPosition(pos) => {
                self.config.bar.anchor = pos;
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::SetBarStyle(style) => {
                self.config.bar.style = style;
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::SetBarHeight(height) => {
                self.config.bar.height = height;
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::SetBarTransparency(trans) => {
                self.config.bar.transparency = trans.clone();
                self.frosted_enabled = trans == "frosted";
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::SetTimeFormat(fmt) => {
                self.config.bar.time_format = fmt;
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::SetWorkspaceCount(count) => {
                self.config.bar.workspace_count = count;
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::ToggleBarWidget(widget_name) => {
                match widget_name {
                    "show_brand" => self.config.bar.show_brand = !self.config.bar.show_brand,
                    "show_workspaces" => self.config.bar.show_workspaces = !self.config.bar.show_workspaces,
                    "show_active_window" => self.config.bar.show_active_window = !self.config.bar.show_active_window,
                    "show_media" => self.config.bar.show_media = !self.config.bar.show_media,
                    "show_clock" => self.config.bar.show_clock = !self.config.bar.show_clock,
                    "show_network" => self.config.bar.show_network = !self.config.bar.show_network,
                    "show_audio" => self.config.bar.show_audio = !self.config.bar.show_audio,
                    "show_brightness" => self.config.bar.show_brightness = !self.config.bar.show_brightness,
                    "show_battery" => self.config.bar.show_battery = !self.config.bar.show_battery,
                    "show_cpu" => self.config.bar.show_cpu = !self.config.bar.show_cpu,
                    "show_memory" => self.config.bar.show_memory = !self.config.bar.show_memory,
                    "show_storage" => self.config.bar.show_storage = !self.config.bar.show_storage,
                    "show_clipboard" => self.config.bar.show_clipboard = !self.config.bar.show_clipboard,
                    "show_notifications" => self.config.bar.show_notifications = !self.config.bar.show_notifications,
                    "show_control_center" => self.config.bar.show_control_center = !self.config.bar.show_control_center,
                    "show_settings" => self.config.bar.show_settings = !self.config.bar.show_settings,
                    "show_terminal" => self.config.bar.show_terminal = !self.config.bar.show_terminal,
                    "show_power" => self.config.bar.show_power = !self.config.bar.show_power,
                    _ => {}
                }
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::SetLauncherDimensions(w, h) => {
                self.config.launcher.width = w;
                self.config.launcher.height = h;
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::SetTerminal(term) => {
                self.config.launcher.terminal = term;
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::SetMaxResults(res) => {
                self.config.launcher.max_results = res;
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::SetGaps(gin, gout) => {
                self.active_gaps = (gin, gout);
                let _ = StdCommand::new("hyprctl")
                    .args(["keyword", "general:gaps_in", &gin.to_string()])
                    .spawn();
                let _ = StdCommand::new("hyprctl")
                    .args(["keyword", "general:gaps_out", &gout.to_string()])
                    .spawn();
            }
            SettingsMessage::SetBorderColor(color_str) => {
                self.active_border_color = color_str.clone();
                let _ = StdCommand::new("hyprctl")
                    .args(["keyword", "general:col.active_border", &color_str])
                    .spawn();
            }
            SettingsMessage::ToggleGuiBorder(enable) => {
                self.config.gui.border_enabled = enable;
                if enable && self.config.gui.border_width <= 0.0 {
                    self.config.gui.border_width = 1.5;
                }
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::SetGuiBorderColor(color_hex) => {
                self.config.gui.border_color = color_hex;
                self.config.gui.border_enabled = true;
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::SetGuiBorderWidth(w) => {
                self.config.gui.border_width = w;
                if w <= 0.0 {
                    self.config.gui.border_enabled = false;
                } else {
                    self.config.gui.border_enabled = true;
                }
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::SetGuiBackground(bg_hex) => {
                self.config.gui.background = bg_hex;
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::SetGuiCornerRadius(r) => {
                self.config.gui.corner_radius = r;
                let _ = self.config.save_to_default_path();
            }
            SettingsMessage::SetPowerProfile(prof) => {
                self.current_power_profile = prof;
                PowerProfileManager::set_profile(prof);
            }
            SettingsMessage::ToggleAutoAdaptive(enable) => {
                self.config.power.auto_adaptive = enable;
                let _ = self.config.save_to_default_path();
                self.status_feedback = Some(if enable {
                    "Enabled Auto Adaptive Power Mode".into()
                } else {
                    "Switched to Manual Power Mode".into()
                });
            }
            SettingsMessage::ToggleConservationMode(enable) => {
                self.status_feedback = Some(if enable {
                    "Enabling Desk Mode (55–60%)...".to_string()
                } else {
                    "Switching to Full Charge Mode (100%)...".to_string()
                });
                return Command::perform(
                    async move {
                        BatteryMonitor::set_conservation_mode_async(enable)
                            .await
                            .map(|()| enable)
                    },
                    SettingsMessage::ConservationModeResult,
                );
            }
            SettingsMessage::ConservationModeResult(res) => {
                match res {
                    Ok(enable) => {
                        self.conservation_mode = BatteryMonitor::read_conservation_mode();
                        self.status_feedback = Some(if enable {
                            "Enabled Desk Mode (55–60% Charge Limit for Battery Longevity)".to_string()
                        } else {
                            "Switched to Full Charge Mode (Charging to 100%)".to_string()
                        });
                    }
                    Err(e) => {
                        self.status_feedback = Some(format!("Failed to switch charging mode: {e}"));
                    }
                }
            }
            SettingsMessage::StepVolume(delta) => {
                if delta > 0 {
                    AudioMonitor::increase_volume(delta as u8);
                } else if delta < 0 {
                    AudioMonitor::decrease_volume((-delta) as u8);
                }
                self.audio_info = AudioMonitor::read();
            }
            SettingsMessage::ToggleAudioMute => {
                AudioMonitor::toggle_mute();
                self.audio_info = AudioMonitor::read();
            }
            SettingsMessage::LaunchPavucontrol => {
                let _ = StdCommand::new("pavucontrol").spawn();
            }
            SettingsMessage::SetAnimationSpeed(speed) => {
                let speed_val = match speed {
                    "fast" => "windows, 1, 3, default",
                    "smooth" => "windows, 1, 7, default",
                    _ => "windows, 1, 5, default",
                };
                let _ = StdCommand::new("hyprctl")
                    .args(["keyword", "animation", speed_val])
                    .spawn();
            }
            SettingsMessage::SetDockPosition(pos) => {
                self.status_feedback = Some(format!("Dock position set to {}", pos));
            }
            SettingsMessage::SetDockSize(size) => {
                self.status_feedback = Some(format!("Dock size set to {} px", size));
            }
            SettingsMessage::SetWeatherUnit(unit) => {
                self.status_feedback = Some(format!("Weather unit set to {}", unit));
            }
            SettingsMessage::Tick => {
                let should_refresh = self
                    .last_toggle_time
                    .map(|t| t.elapsed() >= Duration::from_secs(4))
                    .unwrap_or(true);

                if should_refresh {
                    self.services = SystemServiceManager::list_services();
                    self.dnd_enabled = DndManager::is_dnd_enabled();
                    self.audio_info = AudioMonitor::read();
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
        .style(crate::modal_style::default_modal_card_style());

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

        // Full list of settings categories matching reference design
        let categories: [(SettingsTab, &'static str, &'static str, &'static str, &'static str); 13] = [
            (
                SettingsTab::Wallpaper,
                "󰏘",
                "Wallpaper & colors",
                "Theme, palette, dark mode",
                "wallpaper theme colors palette background dark",
            ),
            (
                SettingsTab::Interface,
                "󰔄",
                "Interface style",
                "Blur, shadows, corners, gaps",
                "interface style blur shadows corners gaps border kawase transparency transparent opaque translucency",
            ),
            (
                SettingsTab::Fonts,
                "󰛖",
                "Fonts & motion",
                "Fonts, scale, animations",
                "font fonts motion scale size typography text animation",
            ),
            (
                SettingsTab::Bar,
                "󰘵",
                "Bar",
                "Layout, position, appearance",
                "bar topbar bottom height layout style modern island floating docked transparent frosted acrylic solid clock time",
            ),
            (
                SettingsTab::BarWidgets,
                "󱓞",
                "Bar widgets",
                "Add, reorder, configure",
                "widgets modules workspace clock network audio battery brightness cpu ram storage disk clipboard notifications control power",
            ),
            (
                SettingsTab::Dock,
                "󰖮",
                "Dock",
                "Position, pinned apps, sizing",
                "dock taskbar apps bottom icons autohide sizing",
            ),
            (
                SettingsTab::Launcher,
                "󰀻",
                "Launcher",
                "Style, search, terminal",
                "launcher app search dimension size height width terminal foot kitty alacritty ghostty",
            ),
            (
                SettingsTab::Dashboard,
                "󰕮",
                "Dashboard",
                "Widgets, weather, system info",
                "dashboard weather temperature celsius fahrenheit widgets overview",
            ),
            (
                SettingsTab::Notifications,
                "󰂚",
                "Notifications",
                "Popups, do not disturb, rules",
                "notifications dnd do not disturb alerts popups sounds",
            ),
            (
                SettingsTab::Osd,
                "󰔎",
                "On-screen displays",
                "Volume, brightness, caps lock",
                "osd on screen display volume brightness caps lock power overlay position",
            ),
            (
                SettingsTab::Services,
                "󰂯",
                "System Services",
                "Services, RAM & radios",
                "services ram bluetooth vpn protonvpn wifi radio daemons kill optimize",
            ),
            (
                SettingsTab::Power,
                "󰌪",
                "Power & Battery",
                "Profiles, TLP, suspend",
                "power battery tlp profile performance balanced power saver suspend energy acpi",
            ),
            (
                SettingsTab::Audio,
                "󰕾",
                "Audio & Sound",
                "Volume, outputs, microphones",
                "audio sound volume pactl speaker microphone mute sink pavucontrol mixer",
            ),
        ];

        let q = self.search_query.trim().to_lowercase();
        let mut nav_items = column![].spacing(6);
        let mut matched_count = 0;

        for (tab, icon, title, subtitle, keywords) in categories {
            if q.is_empty()
                || title.to_lowercase().contains(&q)
                || subtitle.to_lowercase().contains(&q)
                || keywords.contains(&q)
            {
                nav_items = nav_items.push(self.nav_button(tab, icon, title, subtitle));
                matched_count += 1;
            }
        }

        if matched_count == 0 {
            nav_items = nav_items.push(
                container(
                    column![
                        text("No settings found").size(12).color(Color::from_rgba(1.0, 1.0, 1.0, 0.60)),
                        text("Try searching for 'bar', 'wifi', or 'blur'")
                            .size(10)
                            .color(Color::from_rgba(1.0, 1.0, 1.0, 0.40)),
                    ]
                    .spacing(4),
                )
                .padding([16, 8]),
            );
        }

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
            SettingsTab::Wallpaper => self.view_wallpaper_tab(),
            SettingsTab::Interface => self.view_interface_tab(),
            SettingsTab::Fonts => self.view_fonts_tab(),
            SettingsTab::Bar => self.view_bar_tab(),
            SettingsTab::BarWidgets => self.view_bar_widgets_tab(),
            SettingsTab::Dock => self.view_dock_tab(),
            SettingsTab::Launcher => self.view_launcher_tab(),
            SettingsTab::Dashboard => self.view_dashboard_tab(),
            SettingsTab::Notifications => self.view_notifications_tab(),
            SettingsTab::Osd => self.view_osd_tab(),
            SettingsTab::Services => self.view_services_tab(),
            SettingsTab::Power => self.view_power_tab(),
            SettingsTab::Audio => self.view_audio_tab(),
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

    /// Top Bar Layout Tab
    fn view_bar_tab(&self) -> Element<'_, SettingsMessage> {
        let header = column![
            text("Top Bar Layout & Styling").size(19).color(Color::WHITE),
            text("Configure anchor position, floating island margins, height, and visual transparency")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(4);

        // Position: Top or Bottom
        let is_top = self.config.bar.anchor.to_lowercase() != "bottom";
        let pos_chips = row![
            self.chip_button("Top (Default)", is_top, SettingsMessage::SetBarPosition("top".into())),
            self.chip_button("Bottom", !is_top, SettingsMessage::SetBarPosition("bottom".into())),
        ]
        .spacing(8);

        // Style: Modern Floating Island or Docked Edge-to-Edge
        let is_modern = self.config.bar.is_modern();
        let style_chips = row![
            self.chip_button("Floating Island (Modern)", is_modern, SettingsMessage::SetBarStyle("modern".into())),
            self.chip_button("Edge-to-Edge Docked", !is_modern, SettingsMessage::SetBarStyle("docked".into())),
        ]
        .spacing(8);

        // Height: 28, 32, 36, 40 px
        let h = self.config.bar.height;
        let height_chips = row![
            self.chip_button("28 px (Compact)", h == 28, SettingsMessage::SetBarHeight(28)),
            self.chip_button("32 px (Default)", h == 32, SettingsMessage::SetBarHeight(32)),
            self.chip_button("36 px (Comfortable)", h == 36, SettingsMessage::SetBarHeight(36)),
            self.chip_button("40 px (Spacious)", h == 40, SettingsMessage::SetBarHeight(40)),
        ]
        .spacing(8);

        // Transparency: Frosted, Transparent, Solid
        let is_clear = self.config.bar.is_transparent();
        let is_solid = self.config.bar.is_solid();
        let is_frost = self.config.bar.is_frosted();
        let trans_chips = row![
            self.chip_button("Frosted Acrylic (Blur)", is_frost, SettingsMessage::SetBarTransparency("frosted".into())),
            self.chip_button("Transparent (Clear)", is_clear, SettingsMessage::SetBarTransparency("transparent".into())),
            self.chip_button("Solid Opaque", is_solid, SettingsMessage::SetBarTransparency("solid".into())),
        ]
        .spacing(8);

        // Time Format
        let tf = &self.config.bar.time_format;
        let is_12h = tf.contains("%I") || tf.contains("%p");
        let is_compact = tf == "%H:%M";
        let is_24h = !is_12h && !is_compact;

        let time_chips = row![
            self.chip_button("12-Hour (11:55 AM)", is_12h, SettingsMessage::SetTimeFormat("%a %b %d  %I:%M %p".into())),
            self.chip_button("24-Hour (23:55)", is_24h, SettingsMessage::SetTimeFormat("%a %b %d  %H:%M".into())),
            self.chip_button("Compact Time (%H:%M)", is_compact, SettingsMessage::SetTimeFormat("%H:%M".into())),
        ]
        .spacing(8);

        let content = column![
            header,
            self.settings_card("Bar Position", "Placement of the status bar on your primary display", pos_chips),
            self.settings_card("Island Geometry", "Floating island with rounded corners and margins or edge docked", style_chips),
            self.settings_card("Bar Thickness / Height", "Vertical size of the bar surface in pixels", height_chips),
            self.settings_card("Background & Transparency", "Layer shell blur effect and backdrop opacity", trans_chips),
            self.settings_card("Clock & Date Format", "Choose how timestamp information is rendered in the top bar", time_chips),
        ]
        .spacing(12);

        scrollable(content).height(Length::Fill).into()
    }

    /// Bar Widgets Tab
    fn view_bar_widgets_tab(&self) -> Element<'_, SettingsMessage> {
        let header = column![
            text("Bar Widgets & Modules").size(19).color(Color::WHITE),
            text("Select active modules in the status bar and configure workspace indicator counts")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(4);

        let cur_count = self.config.bar.workspace_count;
        let ws_chips = row![
            self.chip_button("3 Workspaces", cur_count == 3, SettingsMessage::SetWorkspaceCount(3)),
            self.chip_button("4 Workspaces", cur_count == 4, SettingsMessage::SetWorkspaceCount(4)),
            self.chip_button("5 Workspaces", cur_count == 5, SettingsMessage::SetWorkspaceCount(5)),
            self.chip_button("7 Workspaces", cur_count == 7, SettingsMessage::SetWorkspaceCount(7)),
            self.chip_button("10 Workspaces", cur_count == 10, SettingsMessage::SetWorkspaceCount(10)),
        ]
        .spacing(8);

        let modules = column![
            self.setting_toggle_row("Workspace Pager Dots", self.config.bar.show_workspaces, SettingsMessage::ToggleBarWidget("show_workspaces")),
            self.setting_toggle_row("Active Focused Window Title", self.config.bar.show_active_window, SettingsMessage::ToggleBarWidget("show_active_window")),
            self.setting_toggle_row("Now Playing Media Widget", self.config.bar.show_media, SettingsMessage::ToggleBarWidget("show_media")),
            self.setting_toggle_row("Clock & Calendar Widget", self.config.bar.show_clock, SettingsMessage::ToggleBarWidget("show_clock")),
            self.setting_toggle_row("Network & Wi-Fi Indicator", self.config.bar.show_network, SettingsMessage::ToggleBarWidget("show_network")),
            self.setting_toggle_row("Volume & Audio Indicator", self.config.bar.show_audio, SettingsMessage::ToggleBarWidget("show_audio")),
            self.setting_toggle_row("Display Brightness Indicator", self.config.bar.show_brightness, SettingsMessage::ToggleBarWidget("show_brightness")),
            self.setting_toggle_row("Battery & Power Status", self.config.bar.show_battery, SettingsMessage::ToggleBarWidget("show_battery")),
            self.setting_toggle_row("CPU Load Percentage (%)", self.config.bar.show_cpu, SettingsMessage::ToggleBarWidget("show_cpu")),
            self.setting_toggle_row("RAM Utilization Monitor", self.config.bar.show_memory, SettingsMessage::ToggleBarWidget("show_memory")),
            self.setting_toggle_row("Disk Storage Monitor", self.config.bar.show_storage, SettingsMessage::ToggleBarWidget("show_storage")),
            self.setting_toggle_row("Clipboard History Quick-Button", self.config.bar.show_clipboard, SettingsMessage::ToggleBarWidget("show_clipboard")),
            self.setting_toggle_row("Notification Center Bell", self.config.bar.show_notifications, SettingsMessage::ToggleBarWidget("show_notifications")),
            self.setting_toggle_row("Control Center Quick Toggle", self.config.bar.show_control_center, SettingsMessage::ToggleBarWidget("show_control_center")),
            self.setting_toggle_row("System Settings Panel (⚙)", self.config.bar.show_settings, SettingsMessage::ToggleBarWidget("show_settings")),
            self.setting_toggle_row("Terminal Quick-Launch Icon", self.config.bar.show_terminal, SettingsMessage::ToggleBarWidget("show_terminal")),
            self.setting_toggle_row("Power & Session Menu Button", self.config.bar.show_power, SettingsMessage::ToggleBarWidget("show_power")),
        ]
        .spacing(6);

        let content = column![
            header,
            self.settings_card("Workspace Pager Density", "Number of virtual desktop indicator chips shown", ws_chips),
            self.settings_card("Enabled Bar Modules", "Toggle visibility of individual widgets on the bar", modules),
        ]
        .spacing(12);

        scrollable(content).height(Length::Fill).into()
    }

    /// Application Launcher Tab
    fn view_launcher_tab(&self) -> Element<'_, SettingsMessage> {
        let header = column![
            text("Application Launcher").size(19).color(Color::WHITE),
            text("Window dimensions, search density, and default terminal emulator")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(4);

        let (w, h) = (self.config.launcher.width, self.config.launcher.height);
        let dim_chips = row![
            self.chip_button("Compact (640 × 380)", w == 640 && h == 380, SettingsMessage::SetLauncherDimensions(640, 380)),
            self.chip_button("Standard (680 × 420)", w == 680 && h == 420, SettingsMessage::SetLauncherDimensions(680, 420)),
            self.chip_button("Spacious (760 × 480)", w == 760 && h == 480, SettingsMessage::SetLauncherDimensions(760, 480)),
        ]
        .spacing(8);

        let cur_term = self.config.launcher.terminal.to_lowercase();
        let term_chips = row![
            self.chip_button("kitty", cur_term.contains("kitty"), SettingsMessage::SetTerminal("kitty".into())),
            self.chip_button("foot", cur_term.contains("foot"), SettingsMessage::SetTerminal("foot".into())),
            self.chip_button("alacritty", cur_term.contains("alacritty"), SettingsMessage::SetTerminal("alacritty".into())),
            self.chip_button("ghostty", cur_term.contains("ghostty"), SettingsMessage::SetTerminal("ghostty".into())),
        ]
        .spacing(8);

        let max_res = self.config.launcher.max_results;
        let res_chips = row![
            self.chip_button("6 items", max_res == 6, SettingsMessage::SetMaxResults(6)),
            self.chip_button("8 items (Default)", max_res == 8, SettingsMessage::SetMaxResults(8)),
            self.chip_button("10 items", max_res == 10, SettingsMessage::SetMaxResults(10)),
            self.chip_button("12 items", max_res == 12, SettingsMessage::SetMaxResults(12)),
        ]
        .spacing(8);

        let content = column![
            header,
            self.settings_card("Launcher Window Size", "Resolution of the application search overlay", dim_chips),
            self.settings_card("Default Terminal Emulator", "Terminal program executed for command-line entries", term_chips),
            self.settings_card("Max Search Results", "Number of matching applications shown per query", res_chips),
        ]
        .spacing(12);

        scrollable(content).height(Length::Fill).into()
    }

    /// Audio & Sound Tab
    fn view_audio_tab(&self) -> Element<'_, SettingsMessage> {
        let header = column![
            text("Audio & Sound").size(19).color(Color::WHITE),
            text("Master volume, mute controls, and audio device management")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(4);

        let vol = self.audio_info.volume_percent;
        let is_muted = self.audio_info.is_muted;

        let vol_display = row![
            text(format!("{}%", vol)).size(24).color(Color::WHITE),
            space().width(Length::Fixed(16.0)),
            button(text("− 5%").size(12).color(Color::WHITE))
                .padding([6, 12])
                .style(|_t: &Theme, _s| iced::widget::button::Style {
                    background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.08).into()),
                    border: Border { radius: 8.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .on_press(SettingsMessage::StepVolume(-5)),
            button(text("+ 5%").size(12).color(Color::WHITE))
                .padding([6, 12])
                .style(|_t: &Theme, _s| iced::widget::button::Style {
                    background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.08).into()),
                    border: Border { radius: 8.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .on_press(SettingsMessage::StepVolume(5)),
        ]
        .align_y(Alignment::Center);

        let mute_toggle = self.setting_toggle_row("Mute Master Speaker Output", is_muted, SettingsMessage::ToggleAudioMute);

        let mixer_btn = button(
            row![
                text("🎚").size(14),
                text("Open PulseAudio Mixer (Pavucontrol)").size(12).color(Color::WHITE),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([8, 14])
        .style(|_t: &Theme, _s| iced::widget::button::Style {
            background: Some(Color::from_rgb8(139, 92, 246).into()),
            border: Border { radius: 10.0.into(), ..Default::default() },
            ..Default::default()
        })
        .on_press(SettingsMessage::LaunchPavucontrol);

        let content = column![
            header,
            self.settings_card("Master Volume Level", "Adjust system output amplification", vol_display),
            self.settings_card("Speaker Mute", "Instantly silence or unmute the default sink", mute_toggle),
            self.settings_card("Advanced Audio Routing", "Launch external device configuration panel", mixer_btn),
        ]
        .spacing(12);

        scrollable(content).height(Length::Fill).into()
    }

    /// Interface Style Tab
    fn view_interface_tab(&self) -> Element<'_, SettingsMessage> {
        let header = column![
            text("Interface & Visual Style").size(19).color(Color::WHITE),
            text("Universal appearance for all MSUI tools, top bar background, and Hyprland window aesthetics")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(4);

        // --- SECTION 1: Universal MSUI Shell & Tools Appearance ---
        let cur_bg = self.config.gui.background.to_lowercase();
        let bg_row_1 = row![
            self.chip_button("⬛ Obsidian Dark (#121218)", cur_bg == "#121218", SettingsMessage::SetGuiBackground("#121218".into())),
            self.chip_button("🌌 Midnight Navy (#0f172a)", cur_bg == "#0f172a", SettingsMessage::SetGuiBackground("#0f172a".into())),
            self.chip_button("🌑 OLED Black (#000000)", cur_bg == "#000000" || cur_bg == "#050508", SettingsMessage::SetGuiBackground("#000000".into())),
        ]
        .spacing(8);

        let bg_row_2 = row![
            self.chip_button("☕ Slate Mocha (#1e1e2e)", cur_bg == "#1e1e2e", SettingsMessage::SetGuiBackground("#1e1e2e".into())),
            self.chip_button("🌊 Tokyo Twilight (#1a1b26)", cur_bg == "#1a1b26", SettingsMessage::SetGuiBackground("#1a1b26".into())),
            self.chip_button("🌲 Forest Deep (#141c18)", cur_bg == "#141c18", SettingsMessage::SetGuiBackground("#141c18".into())),
        ]
        .spacing(8);

        let bg_group = column![
            text("Theme Background Color").size(12).color(Color::from_rgb8(196, 181, 253)),
            text("Universal dark backdrop applied to all MSUI tools, dialogs, and the Top Bar")
                .size(10)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.45)),
            bg_row_1,
            bg_row_2,
        ]
        .spacing(6);

        let is_border_on = self.config.gui.border_enabled && self.config.gui.border_width > 0.0;
        let border_master = row![
            column![
                text("Window Border Outline").size(12).color(Color::from_rgb8(196, 181, 253)),
                text("Master switch to enable or turn borders completely off across all MSUI tools")
                    .size(10)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.45)),
            ]
            .spacing(2),
            space().width(Length::Fill),
            self.chip_button("󰄬 Border On", is_border_on, SettingsMessage::ToggleGuiBorder(true)),
            self.chip_button("󰅖 Border Off", !is_border_on, SettingsMessage::ToggleGuiBorder(false)),
        ]
        .align_y(Alignment::Center)
        .spacing(8);

        let cur_border_color = self.config.gui.border_color.to_lowercase();
        let border_color_row_1 = row![
            self.chip_button("🟣 Purple Neon", is_border_on && cur_border_color == "#9333ea", SettingsMessage::SetGuiBorderColor("#9333ea".into())),
            self.chip_button("🔵 Cyan Electric", is_border_on && cur_border_color == "#06b6d4", SettingsMessage::SetGuiBorderColor("#06b6d4".into())),
            self.chip_button("🟢 Emerald Green", is_border_on && cur_border_color == "#10b981", SettingsMessage::SetGuiBorderColor("#10b981".into())),
        ]
        .spacing(8);

        let border_color_row_2 = row![
            self.chip_button("🟠 Amber Orange", is_border_on && cur_border_color == "#f97316", SettingsMessage::SetGuiBorderColor("#f97316".into())),
            self.chip_button("🔴 Rose Pink", is_border_on && cur_border_color == "#f43f5e", SettingsMessage::SetGuiBorderColor("#f43f5e".into())),
            self.chip_button("⚪ Frost White", is_border_on && cur_border_color == "#ffffff", SettingsMessage::SetGuiBorderColor("#ffffff".into())),
        ]
        .spacing(8);

        let bw = self.config.gui.border_width;
        let border_width_chips = row![
            self.chip_button("Off (0px)", !is_border_on, SettingsMessage::SetGuiBorderWidth(0.0)),
            self.chip_button("Subtle (1.0px)", is_border_on && (bw - 1.0).abs() < 0.1, SettingsMessage::SetGuiBorderWidth(1.0)),
            self.chip_button("Standard (1.5px)", is_border_on && (bw - 1.5).abs() < 0.1, SettingsMessage::SetGuiBorderWidth(1.5)),
            self.chip_button("Bold (2.5px)", is_border_on && (bw - 2.5).abs() < 0.1, SettingsMessage::SetGuiBorderWidth(2.5)),
        ]
        .spacing(8);

        let border_group = column![
            border_master,
            text("Border Accent Color").size(11).color(Color::from_rgba(1.0, 1.0, 1.0, 0.65)),
            border_color_row_1,
            border_color_row_2,
            text("Border Thickness").size(11).color(Color::from_rgba(1.0, 1.0, 1.0, 0.65)),
            border_width_chips,
        ]
        .spacing(6);

        let rad = self.config.gui.corner_radius;
        let radius_chips = row![
            self.chip_button("Square (0px)", (rad - 0.0).abs() < 0.1, SettingsMessage::SetGuiCornerRadius(0.0)),
            self.chip_button("Compact (10px)", (rad - 10.0).abs() < 0.1, SettingsMessage::SetGuiCornerRadius(10.0)),
            self.chip_button("Rounded (18px)", (rad - 18.0).abs() < 0.1, SettingsMessage::SetGuiCornerRadius(18.0)),
            self.chip_button("Pill (26px)", (rad - 26.0).abs() < 0.1, SettingsMessage::SetGuiCornerRadius(26.0)),
        ]
        .spacing(8);

        let geometry_group = column![
            text("Window Geometry & Corner Radius").size(12).color(Color::from_rgb8(196, 181, 253)),
            radius_chips,
        ]
        .spacing(6);

        let divider = || container(row![]).height(Length::Fixed(1.0)).width(Length::Fill).style(|_t| container::Style {
            background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.06).into()),
            ..Default::default()
        });

        let msui_shell_card_content = column![
            bg_group,
            divider(),
            border_group,
            divider(),
            geometry_group,
        ]
        .spacing(14);

        // --- SECTION 2: Compositor Effects & Glass Engine ---
        let toggles = column![
            self.setting_toggle_row(
                "Window & Interface Transparency",
                self.transparency_enabled,
                SettingsMessage::ToggleTransparency,
            ),
            self.setting_toggle_row(
                "Hyprland GPU Blur (Kawase & Shaders)",
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
        .spacing(6);

        let blur_presets: [(u8, &'static str, &'static str, &'static str); 11] = [
            (1, "🚫", "No Blur", "Disable blur while preserving translucency (Lightweight)"),
            (10, "🌫️", "Dual-Kawase", "Standard dual-kawase pyramid blur (clean & lightweight)"),
            (6, "🪟", "Acrylic Glass", "Modern acrylic glass with max light diffusion & chromatic aberration"),
            (5, "❄️", "Winter Frost", "Micro-textured winter frosted crystal surface"),
            (2, "💧", "Rain Drops", "Rain droplets trickling & condensing on glass"),
            (3, "🧪", "Fluid Jar", "Viscous fluid container with turbulence & liquid physics"),
            (4, "💎", "Prism Refract", "Crystalline prism refraction with rainbow light dispersion"),
            (7, "🌌", "Aurora Neon", "Undulating Northern Lights / Aurora Borealis ambient glow"),
            (8, "🌊", "Water Ripples", "Fluid wave displacement equations rippling across windows"),
            (9, "♨️", "Heat Shimmer", "Atmospheric heat haze mirage distortion waves"),
            (11, "⬛", "Solid Opaque", "100% solid opaque windows matching system theme (zero blur)"),
        ];

        let mut row_1 = row![].spacing(8);
        let mut row_2 = row![].spacing(8);
        let mut row_3 = row![].spacing(8);

        for (i, (id, icon, name, _)) in blur_presets.iter().enumerate() {
            let is_sel = self.active_blur_preset == *id;
            let label = format!("{} {}", icon, name);
            let chip = self.chip_button(label, is_sel, SettingsMessage::SetBlurPreset(*id));
            if i < 4 {
                row_1 = row_1.push(chip);
            } else if i < 8 {
                row_2 = row_2.push(chip);
            } else {
                row_3 = row_3.push(chip);
            }
        }

        let active_desc = blur_presets
            .iter()
            .find(|(id, ..)| *id == self.active_blur_preset)
            .map(|(_, icon, name, desc)| format!("Active Preset: {} {} — {}", icon, name, desc))
            .unwrap_or_else(|| "Custom shader configuration active".to_string());

        let active_desc_text = container(
            text(active_desc)
                .size(11)
                .color(Color::from_rgb8(196, 181, 253))
        )
        .padding([6, 10])
        .style(|_t| container::Style {
            background: Some(Color::from_rgba(139.0 / 255.0, 92.0 / 255.0, 246.0 / 255.0, 0.12).into()),
            border: Border {
                color: Color::from_rgba(139.0 / 255.0, 92.0 / 255.0, 246.0 / 255.0, 0.25),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        });

        let blur_shaders_column = column![
            text("Hyprland 0.57+ Blur Shaders & Appearance Presets").size(12).color(Color::from_rgb8(196, 181, 253)),
            row_1,
            row_2,
            row_3,
            active_desc_text,
        ]
        .spacing(8);

        let compositor_card_content = column![
            toggles,
            divider(),
            blur_shaders_column,
        ]
        .spacing(14);

        // --- SECTION 3: Hyprland Window Management & Tiling ---
        let hypr_border_row_1 = row![
            self.chip_button("🟣 Purple Neon", self.active_border_color == "rgba(9333eaff)", SettingsMessage::SetBorderColor("rgba(9333eaff)".into())),
            self.chip_button("🔵 Cyan Electric", self.active_border_color == "rgba(06b6d4ff)", SettingsMessage::SetBorderColor("rgba(06b6d4ff)".into())),
            self.chip_button("🟢 Emerald Green", self.active_border_color == "rgba(10b981ff)", SettingsMessage::SetBorderColor("rgba(10b981ff)".into())),
        ]
        .spacing(8);

        let hypr_border_row_2 = row![
            self.chip_button("🟠 Amber Orange", self.active_border_color == "rgba(f97316ff)", SettingsMessage::SetBorderColor("rgba(f97316ff)".into())),
            self.chip_button("🔴 Rose Pink", self.active_border_color == "rgba(f43f5eff)", SettingsMessage::SetBorderColor("rgba(f43f5eff)".into())),
        ]
        .spacing(8);

        let gaps_chips = row![
            self.chip_button("Compact (3px / 6px)", self.active_gaps == (3, 6), SettingsMessage::SetGaps(3, 6)),
            self.chip_button("Balanced (5px / 12px)", self.active_gaps == (5, 12), SettingsMessage::SetGaps(5, 12)),
            self.chip_button("Spacious (8px / 20px)", self.active_gaps == (8, 20), SettingsMessage::SetGaps(8, 20)),
        ]
        .spacing(8);

        let hyprland_card_content = column![
            text("Active Window Border Accent").size(12).color(Color::from_rgb8(196, 181, 253)),
            text("Accent outline around the currently focused Hyprland application window")
                .size(10)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.45)),
            hypr_border_row_1,
            hypr_border_row_2,
            divider(),
            text("Window Inner & Outer Gaps").size(12).color(Color::from_rgb8(196, 181, 253)),
            text("Tiling workspace padding between application windows")
                .size(10)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.45)),
            gaps_chips,
        ]
        .spacing(8);

        let content = column![
            header,
            self.settings_card(
                "Universal MSUI Shell & Tools Appearance",
                "Theme background, border accents, and geometry across all MSUI tools & Top Bar",
                msui_shell_card_content
            ),
            self.settings_card(
                "Compositor Effects & Glass Engine",
                "Hardware-accelerated window aesthetics and Hyprland blur shaders",
                compositor_card_content
            ),
            self.settings_card(
                "Hyprland Tiling & Active Window Focus",
                "Active window border highlight and workspace gaps",
                hyprland_card_content
            ),
        ]
        .spacing(14);

        scrollable(content).height(Length::Fill).into()
    }

    /// Fonts & Motion Tab
    fn view_fonts_tab(&self) -> Element<'_, SettingsMessage> {
        let header = column![
            text("Fonts & motion").size(19).color(Color::WHITE),
            text("System typography, UI scale factor, and window animation timing")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(4);

        let anim_chips = row![
            self.chip_button("Snappy (0.15s)", false, SettingsMessage::SetAnimationSpeed("fast")),
            self.chip_button("Balanced (0.25s)", true, SettingsMessage::SetAnimationSpeed("default")),
            self.chip_button("Gentle & Smooth (0.40s)", false, SettingsMessage::SetAnimationSpeed("smooth")),
        ]
        .spacing(8);

        let font_chips = row![
            self.chip_button("JetBrainsMono Nerd Font (Default)", true, SettingsMessage::Ignored),
            self.chip_button("FiraCode Nerd Font", false, SettingsMessage::Ignored),
            self.chip_button("Inter Display", false, SettingsMessage::Ignored),
        ]
        .spacing(8);

        let content = column![
            header,
            self.settings_card("System Typography", "Primary UI interface and monospace font family", font_chips),
            self.settings_card("Window Transition Speed", "Bezier curve animation timing for tiling and floating windows", anim_chips),
        ]
        .spacing(12);

        scrollable(content).height(Length::Fill).into()
    }

    /// Dock Tab
    fn view_dock_tab(&self) -> Element<'_, SettingsMessage> {
        let header = column![
            text("Dock & Quick Launch").size(19).color(Color::WHITE),
            text("Position, pinned application icons, and auto-hide behaviors")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(4);

        let pos_chips = row![
            self.chip_button("Bottom (Default)", true, SettingsMessage::SetDockPosition("bottom".into())),
            self.chip_button("Left Edge", false, SettingsMessage::SetDockPosition("left".into())),
            self.chip_button("Right Edge", false, SettingsMessage::SetDockPosition("right".into())),
        ]
        .spacing(8);

        let size_chips = row![
            self.chip_button("36 px (Compact)", false, SettingsMessage::SetDockSize(36)),
            self.chip_button("48 px (Standard)", true, SettingsMessage::SetDockSize(48)),
            self.chip_button("64 px (Large)", false, SettingsMessage::SetDockSize(64)),
        ]
        .spacing(8);

        let content = column![
            header,
            self.settings_card("Dock Screen Edge", "Screen anchor position for pinned applications", pos_chips),
            self.settings_card("Icon Dimension", "Size of app launcher tiles in pixels", size_chips),
        ]
        .spacing(12);

        scrollable(content).height(Length::Fill).into()
    }

    /// Dashboard Tab
    fn view_dashboard_tab(&self) -> Element<'_, SettingsMessage> {
        let header = column![
            text("Dashboard & Widgets").size(19).color(Color::WHITE),
            text("System overview cards, weather forecast, and desktop utilities")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(4);

        let unit_chips = row![
            self.chip_button("Celsius (°C)", true, SettingsMessage::SetWeatherUnit("celsius".into())),
            self.chip_button("Fahrenheit (°F)", false, SettingsMessage::SetWeatherUnit("fahrenheit".into())),
        ]
        .spacing(8);

        let content = column![
            header,
            self.settings_card("Weather Temperature Unit", "Display units used for weather forecasts and alerts", unit_chips),
        ]
        .spacing(12);

        scrollable(content).height(Length::Fill).into()
    }

    /// Adaptive Color Engine & Wallpaper Tab
    fn view_wallpaper_tab(&self) -> Element<'_, SettingsMessage> {
        let header = column![
            text("Adaptive Color Engine & Wallpaper").size(19).color(Color::WHITE),
            text("Material Design 3 HCT dynamic palettes, wallpaper extraction, and cross-desktop styling")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(4);

        let tokens = ColorEngine::current_tokens();
        let is_light = self.config.color.scheme == "light";
        let color_mode = &self.config.color.mode;

        let parse_color = |hex: &str| -> Color {
            if let Some((r, g, b)) = msui_core::parse_hex_color(hex) {
                Color::from_rgb(r, g, b)
            } else {
                Color::WHITE
            }
        };

        // Source Mode Selector
        let mode_row = row![
            self.chip_button("󰔎 Wallpaper Adaptive", color_mode == "dynamic", SettingsMessage::ColorExtractWallpaper),
            self.chip_button("󰓎 Curated Presets", color_mode == "preset", SettingsMessage::ColorSetPreset("midnight".into())),
            self.chip_button("󰏘 Custom Accent", color_mode == "accent", SettingsMessage::ColorSetAccent("#9333ea".into())),
        ]
        .spacing(8);

        // Theme Mode Selector
        let theme_row = row![
            self.chip_button("󰖔 Dark Mode", !is_light, SettingsMessage::ColorSetMode(ColorSchemeMode::Dark)),
            self.chip_button("󰖙 Light Mode", is_light, SettingsMessage::ColorSetMode(ColorSchemeMode::Light)),
        ]
        .spacing(8);

        // One-click Extract & Apply button
        let extract_btn = button(
            row![
                text("󰄬").size(13).color(Color::WHITE),
                text("Extract From Wallpaper & Apply Desktop").size(12).color(Color::WHITE),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([8, 16])
        .style(|_t, status| {
            let bg = match status {
                iced::widget::button::Status::Hovered => Color::from_rgba(147.0 / 255.0, 51.0 / 255.0, 234.0 / 255.0, 0.90),
                _ => Color::from_rgba(147.0 / 255.0, 51.0 / 255.0, 234.0 / 255.0, 0.70),
            };
            iced::widget::button::Style {
                background: Some(bg.into()),
                border: Border { radius: 10.0.into(), ..Default::default() },
                ..Default::default()
            }
        })
        .on_press(SettingsMessage::ColorExtractWallpaper);

        // Swatches preview
        let swatch_box = |label: &'static str, hex: &str| -> Element<'_, SettingsMessage> {
            let col = parse_color(hex);
            let pill = container(text("").size(1))
                .width(20)
                .height(20)
                .style(move |_| iced::widget::container::Style {
                    background: Some(col.into()),
                    border: Border {
                        radius: 6.0.into(),
                        width: 1.0,
                        color: Color::from_rgba(1.0, 1.0, 1.0, 0.20),
                    },
                    ..Default::default()
                });

            container(
                row![
                    pill,
                    column![
                        text(label).size(11).color(Color::WHITE),
                        text(hex.to_string()).size(10).color(Color::from_rgba(1.0, 1.0, 1.0, 0.55)),
                    ]
                    .spacing(2),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([4, 6])
            .into()
        };

        let swatches_row = row![
            swatch_box("Primary", &tokens.primary),
            swatch_box("Secondary", &tokens.secondary),
            swatch_box("Surface", &tokens.surface),
            swatch_box("Background", &tokens.background),
        ]
        .spacing(10);

        // Live Mockup Card Preview showing generated tokens in action
        let preview_primary_col = parse_color(&tokens.primary);
        let preview_on_primary_col = parse_color(&tokens.on_primary);
        let preview_bg_col = parse_color(&tokens.background);
        let preview_surface_col = parse_color(&tokens.surface);
        let preview_on_surface_col = parse_color(&tokens.on_surface);
        let preview_outline_col = parse_color(&tokens.outline);

        let live_preview_mock = container(
            column![
                row![
                    text("Desktop Mockup Preview").size(12).color(preview_on_surface_col),
                    space().width(Length::Fill),
                    text("WCAG Contrast Verified").size(10).color(preview_primary_col),
                ]
                .align_y(Alignment::Center),
                row![
                    container(
                        text("Primary Action").size(11).color(preview_on_primary_col)
                    )
                    .padding([5, 12])
                    .style(move |_| iced::widget::container::Style {
                        background: Some(preview_primary_col.into()),
                        border: Border { radius: 6.0.into(), ..Default::default() },
                        ..Default::default()
                    }),
                    container(
                        text("Surface Card").size(11).color(preview_on_surface_col)
                    )
                    .padding([5, 12])
                    .style(move |_| iced::widget::container::Style {
                        background: Some(preview_surface_col.into()),
                        border: Border {
                            radius: 6.0.into(),
                            width: 1.0,
                            color: preview_outline_col,
                        },
                        ..Default::default()
                    }),
                    text("Accessible luminance").size(11).color(preview_on_surface_col),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            ]
            .spacing(10),
        )
        .padding(12)
        .width(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            background: Some(preview_bg_col.into()),
            border: Border {
                radius: 10.0.into(),
                width: 1.0,
                color: preview_outline_col,
            },
            ..Default::default()
        });

        let divider = || {
            container(text("").size(1))
                .width(Length::Fill)
                .height(1)
                .style(|_| iced::widget::container::Style {
                    background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.06).into()),
                    ..Default::default()
                })
        };

        let card1_content = column![
            row![
                text("Color Source").size(12).color(Color::WHITE),
                space().width(Length::Fill),
                mode_row,
            ]
            .align_y(Alignment::Center),
            divider(),
            row![
                text("Theme Mode").size(12).color(Color::WHITE),
                space().width(Length::Fill),
                theme_row,
            ]
            .align_y(Alignment::Center),
            divider(),
            row![
                extract_btn,
                space().width(Length::Fill),
                text(format!("Scheme: {}", if is_light { "Light" } else { "Dark" }))
                    .size(11)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
            ]
            .align_y(Alignment::Center),
            divider(),
            column![
                text("Active Semantic Tokens").size(12).color(Color::WHITE),
                swatches_row,
                live_preview_mock,
            ]
            .spacing(8),
        ]
        .spacing(10);

        // Presets Chips in 2 rows of 3 chips each
        let active_preset = &self.config.color.active_preset;
        let mut preset_chips_row1 = row![].spacing(8);
        let mut preset_chips_row2 = row![].spacing(8);

        for (i, p) in PRESETS.iter().enumerate() {
            let label = format!("{} ({})", p.name, p.accent_hex);
            let is_selected = color_mode == "preset" && active_preset == p.id;
            let btn = self.chip_button(label, is_selected, SettingsMessage::ColorSetPreset(p.id.to_string()));
            if i < 3 {
                preset_chips_row1 = preset_chips_row1.push(btn);
            } else {
                preset_chips_row2 = preset_chips_row2.push(btn);
            }
        }

        let presets_layout = column![
            preset_chips_row1,
            preset_chips_row2,
        ]
        .spacing(8);

        let cur_wall = WallpaperManager::get_current()
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
            .unwrap_or_else(|| "None selected".to_string());

        let wall_controls = row![
            button(
                row![
                    text("󰒭").size(13).color(Color::WHITE),
                    text("Next Wallpaper").size(12).color(Color::WHITE),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([8, 14])
            .style(|_t, status| {
                let bg = match status {
                    iced::widget::button::Status::Hovered => Color::from_rgba(139.0 / 255.0, 92.0 / 255.0, 246.0 / 255.0, 0.90),
                    _ => Color::from_rgba(139.0 / 255.0, 92.0 / 255.0, 246.0 / 255.0, 0.70),
                };
                iced::widget::button::Style {
                    background: Some(bg.into()),
                    border: Border { radius: 10.0.into(), ..Default::default() },
                    ..Default::default()
                }
            })
            .on_press(SettingsMessage::WallpaperNext),
            button(
                row![
                    text("󰒝").size(13).color(Color::WHITE),
                    text("Random Wallpaper").size(12).color(Color::WHITE),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([8, 14])
            .style(|_t, status| {
                let bg = match status {
                    iced::widget::button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.16),
                    _ => Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                };
                iced::widget::button::Style {
                    background: Some(bg.into()),
                    border: Border { radius: 10.0.into(), ..Default::default() },
                    ..Default::default()
                }
            })
            .on_press(SettingsMessage::WallpaperRandom),
            space().width(Length::Fill),
            text(format!("Active: {}", cur_wall)).size(11).color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let content = column![
            header,
            self.settings_card("Adaptive Color Engine (Material Design 3 HCT)", "Dynamic palette generation, wallpaper extraction, and cross-desktop theming", card1_content),
            self.settings_card("Curated Design Presets", "Harmonious fallbacks and handcrafted desktop aesthetics", presets_layout),
            self.settings_card("Wallpaper Manager & Rotation", "Active desktop background image and wallpaper cycling", wall_controls),
        ]
        .spacing(12);

        scrollable(content).height(Length::Fill).into()
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

        let timeout_chips = row![
            self.chip_button("3 Seconds", false, SettingsMessage::Ignored),
            self.chip_button("5 Seconds (Default)", true, SettingsMessage::Ignored),
            self.chip_button("8 Seconds", false, SettingsMessage::Ignored),
        ]
        .spacing(8);

        column![
            header,
            self.settings_card("Focus Mode", "Suppress toasts and banner notifications", dnd_row),
            self.settings_card("Notification Banner Timeout", "Duration before toast automatically closes", timeout_chips),
        ]
        .spacing(12)
        .into()
    }

    /// Power & Battery Tab
    fn view_power_tab(&self) -> Element<'_, SettingsMessage> {
        let header = column![
            text("Power & Battery").size(19).color(Color::WHITE),
            text("ACPI energy profiles, TLP background optimizer, and battery life tuning")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
        ]
        .spacing(4);

        let cur_prof = self.current_power_profile;
        let prof_chips = row![
            self.chip_button("⚡ Performance", cur_prof == PowerProfile::Performance, SettingsMessage::SetPowerProfile(PowerProfile::Performance)),
            self.chip_button("⚖ Balanced", cur_prof == PowerProfile::Balanced, SettingsMessage::SetPowerProfile(PowerProfile::Balanced)),
            self.chip_button("🍃 Power Saver", cur_prof == PowerProfile::PowerSaver, SettingsMessage::SetPowerProfile(PowerProfile::PowerSaver)),
        ]
        .spacing(8);

        let prof_info = match cur_prof {
            PowerProfile::Performance => "Performance mode: Unlocks maximum CPU frequencies and turbo responsiveness.",
            PowerProfile::Balanced => "Balanced mode: Dynamic power scaling for balanced daily performance and battery life.",
            PowerProfile::PowerSaver => "Power Saver mode: Limits CPU clock and throttles power draw to maximize battery runtime.",
        };

        let prof_details = column![
            prof_chips,
            text(prof_info).size(11).color(Color::from_rgba(1.0, 1.0, 1.0, 0.55)),
        ]
        .spacing(8);

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

        let auto_adaptive = self.config.power.auto_adaptive;
        let adaptive_toggle = self.setting_toggle_row(
            "⚡ Auto Adaptive Power Mode (Recommended)",
            auto_adaptive,
            SettingsMessage::ToggleAutoAdaptive(!auto_adaptive),
        );
        let adaptive_card = self.settings_card(
            "Intelligent Adaptive Management",
            "Automatic workload detection, AC/battery scaling, and thermal protection",
            column![
                adaptive_toggle,
                text("Automatically scales between Balanced and Whisper-Efficient mode based on AC connection and workload. Manual profile switching is not required.")
                    .size(11)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.55)),
            ]
            .spacing(8),
        );

        let mut cards = column![
            header,
            adaptive_card,
            self.settings_card("ACPI Platform Power Profile", "Hardware energy management mode", prof_details),
        ]
        .spacing(12);

        if self.conservation_mode.is_some() || BatteryMonitor::conservation_mode_path().is_some() {
            let is_desk = self.conservation_mode.unwrap_or(false);
            let mode_chips = row![
                self.chip_button("🛡️ Desk Mode (55–60%)", is_desk, SettingsMessage::ToggleConservationMode(true)),
                self.chip_button("🔋 Full Charge (100%)", !is_desk, SettingsMessage::ToggleConservationMode(false)),
            ]
            .spacing(8);

            let mode_desc = if is_desk {
                "Desk Mode: Battery charging is limited to 55–60% by hardware to protect lithium chemistry from degradation while on AC adapter."
            } else {
                "Full Charge Mode: Battery charges to 100% capacity for maximum mobility, travel, and untethered battery runtime."
            };

            let mode_details = column![
                mode_chips,
                text(mode_desc).size(11).color(Color::from_rgba(1.0, 1.0, 1.0, 0.55)),
            ]
            .spacing(8);

            cards = cards.push(self.settings_card("Battery Health & Charging Limit", "Lenovo hardware conservation mode", mode_details));
        }

        cards = cards.push(self.settings_card("Automated Battery Optimization", "Kernel power daemon management", tlp_row));

        cards.into()
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
                    if is_sel { text("✓ ").size(10).color(Color::WHITE) } else { text("").size(10) },
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

    /// Reusable settings card container
    fn settings_card<'a>(
        &self,
        title: &'static str,
        subtitle: &'static str,
        content: impl Into<Element<'a, SettingsMessage>>,
    ) -> Element<'a, SettingsMessage> {
        container(
            column![
                column![
                    text(title).size(13).color(Color::WHITE),
                    text(subtitle).size(10).color(Color::from_rgba(1.0, 1.0, 1.0, 0.45)),
                ]
                .spacing(2),
                content.into(),
            ]
            .spacing(10),
        )
        .padding(14)
        .width(Length::Fill)
        .style(|_t| container::Style {
            background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.03).into()),
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
                width: 1.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    /// Chip button with active lavender highlight
    fn chip_button(
        &self,
        label: impl Into<String>,
        is_selected: bool,
        msg: SettingsMessage,
    ) -> Element<'_, SettingsMessage> {
        let label_str = label.into();
        button(
            text(label_str)
                .size(11)
                .color(if is_selected {
                    Color::WHITE
                } else {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.70)
                }),
        )
        .padding([7, 13])
        .style(move |_t, status| {
            if is_selected {
                iced::widget::button::Style {
                    background: Some(Color::from_rgb8(139, 92, 246).into()),
                    border: Border {
                        radius: 9.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            } else {
                let bg = match status {
                    iced::widget::button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.09),
                    _ => Color::from_rgba(1.0, 1.0, 1.0, 0.04),
                };
                iced::widget::button::Style {
                    background: Some(bg.into()),
                    border: Border {
                        color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                        width: 1.0,
                        radius: 9.0.into(),
                    },
                    ..Default::default()
                }
            }
        })
        .on_press(msg)
        .into()
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
