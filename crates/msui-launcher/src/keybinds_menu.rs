use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyboardEvent, Key};
use iced::widget::operation::{focus, snap_to, RelativeOffset};
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Alignment, Border, Color, Element, Event, Font, Length, Task as Command, event};
use iced_exwlshell::actions::ExwlShellCustomActionWithId;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use crate::modal_style::{modal_card_style, subcard_style};
use msui_core::{MsuiConfig, ThemeManager};
use std::sync::LazyLock;

static INPUT_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);
static SCROLL_ID: LazyLock<iced::widget::Id> = LazyLock::new(iced::widget::Id::unique);

pub fn run_keybinds_menu(config: MsuiConfig) -> Result<(), iced_exwlshell::Error> {
    let width = 780;
    let height = 600;
    let cfg_clone = config.clone();

    application(
        move || KeybindsMenu::new(cfg_clone.clone()),
        KeybindsMenu::namespace,
        KeybindsMenu::update,
        KeybindsMenu::view,
    )
    .theme(ThemeManager::current_theme())
    .default_font(Font {
        family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
        ..Font::DEFAULT
    })
    .layer_settings(LayerShellSettings {
        size: LayerSize::px(width, height),
        anchor: Anchor::empty(), // Centered
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    })
    .subscription(KeybindsMenu::subscription)
    .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeybindCategory {
    All,
    Launchers,
    Apps,
    WindowLayout,
    System,
    CaptureUtil,
    Navigation,
}

impl KeybindCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Launchers => "Launchers",
            Self::Apps => "Apps & Files",
            Self::WindowLayout => "Window & Layout",
            Self::System => "System (SUPER+CTRL)",
            Self::CaptureUtil => "Capture & Tools",
            Self::Navigation => "Navigation & Scratch",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Launchers => "Launcher",
            Self::Apps => "Apps",
            Self::WindowLayout => "Window",
            Self::System => "System",
            Self::CaptureUtil => "Utility",
            Self::Navigation => "Navigate",
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeybindItem {
    pub keys: Vec<String>,
    pub description: String,
    pub category: KeybindCategory,
    pub raw_keys: String,
}

pub struct KeybindsMenu {
    pub config: MsuiConfig,
    query: String,
    category: KeybindCategory,
    all_items: Vec<KeybindItem>,
    filtered: Vec<KeybindItem>,
    matcher: SkimMatcherV2,
    is_standalone: bool,
    should_close: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    SetCategory(KeybindCategory),
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

impl KeybindsMenu {
    pub fn new(config: MsuiConfig) -> (Self, Command<Message>) {
        Self::new_internal(config, true)
    }

    pub fn new_embedded(config: MsuiConfig) -> (Self, Command<Message>) {
        Self::new_internal(config, false)
    }

    fn new_internal(config: MsuiConfig, is_standalone: bool) -> (Self, Command<Message>) {
        let all_items = Self::load_keybinds();
        let filtered = all_items.clone();

        (
            Self {
                config,
                query: String::new(),
                category: KeybindCategory::All,
                all_items,
                filtered,
                matcher: SkimMatcherV2::default(),
                is_standalone,
                should_close: false,
            },
            Command::batch([
                focus(INPUT_ID.clone()),
                Command::perform(tokio::time::sleep(std::time::Duration::from_millis(25)), |_| Message::EnsureFocus),
                Command::perform(tokio::time::sleep(std::time::Duration::from_millis(75)), |_| Message::EnsureFocus),
                Command::perform(tokio::time::sleep(std::time::Duration::from_millis(150)), |_| Message::EnsureFocus),
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
        self.all_items = Self::load_keybinds();
        self.query.clear();
        self.category = KeybindCategory::All;
        self.rebuild_filtered();
        self.should_close = false;
        Command::batch([
            focus(INPUT_ID.clone()),
            Command::perform(tokio::time::sleep(std::time::Duration::from_millis(25)), |_| Message::EnsureFocus),
            Command::perform(tokio::time::sleep(std::time::Duration::from_millis(75)), |_| Message::EnsureFocus),
        ])
    }

    pub fn namespace() -> String {
        "msui-keybinds-menu".to_string()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        event::listen().map(Message::IcedEvent)
    }

    fn find_binds_lua_path() -> Option<std::path::PathBuf> {
        if let Ok(home) = std::env::var("HOME") {
            let p = std::path::PathBuf::from(home).join(".config/hypr/modules/binds.lua");
            if p.exists() {
                return Some(p);
            }
        }
        let dotfiles_path = std::path::PathBuf::from("dotfiles/hypr/modules/binds.lua");
        if dotfiles_path.exists() {
            return Some(dotfiles_path);
        }
        None
    }

    fn categorize_bind(keys: &[String], desc: &str) -> KeybindCategory {
        let desc_l = desc.to_lowercase();
        let raw = keys.join(" ").to_lowercase();

        if raw == "super a"
            || raw == "super space"
            || raw == "super tab"
            || raw == "super k"
            || raw == "super i"
            || raw == "super shift i"
            || raw == "super ctrl space"
            || desc_l.contains("launcher")
            || desc_l.contains("control center")
            || desc_l.contains("app drawer")
            || desc_l.contains("cheatsheet")
        {
            return KeybindCategory::Launchers;
        }

        if desc_l.contains("screenshot")
            || desc_l.contains("color picker")
            || desc_l.contains("ocr")
            || desc_l.contains("recording")
            || desc_l.contains("capture")
        {
            return KeybindCategory::CaptureUtil;
        }

        if desc_l.contains("workspace")
            || desc_l.contains("scratchpad")
            || desc_l.contains("minimize")
            || desc_l.contains("focus")
            || desc_l.contains("move window")
            || raw.contains("left")
            || raw.contains("right")
            || raw.contains("up")
            || raw.contains("down")
            || raw.contains("mouse")
        {
            return KeybindCategory::Navigation;
        }

        if keys.iter().any(|k| k.eq_ignore_ascii_case("ctrl"))
            && keys.iter().any(|k| k.eq_ignore_ascii_case("super"))
        {
            return KeybindCategory::System;
        }

        if desc_l.contains("terminal")
            || desc_l.contains("browser")
            || desc_l.contains("file manager")
            || desc_l.contains("yazi")
            || desc_l.contains("ai")
            || desc_l.contains("editor")
            || desc_l.contains("obsidian")
            || desc_l.contains("wechat")
        {
            return KeybindCategory::Apps;
        }

        if desc_l.contains("power")
            || desc_l.contains("reboot")
            || desc_l.contains("logout")
            || desc_l.contains("lock")
            || desc_l.contains("mixer")
            || desc_l.contains("audio")
            || desc_l.contains("bluetooth")
            || desc_l.contains("battery")
            || desc_l.contains("media")
            || desc_l.contains("storage")
            || desc_l.contains("cleanup")
            || desc_l.contains("updates")
            || desc_l.contains("disturb")
        {
            return KeybindCategory::System;
        }

        KeybindCategory::WindowLayout
    }

    fn parse_binds_lua(content: &str) -> Vec<KeybindItem> {
        let mut items = Vec::new();
        let mut current_keys: Option<Vec<String>> = None;

        for line in content.lines() {
            let trimmed = line.trim();

            if let Some(pos) = trimmed.find("hl.bind(") {
                let after = &trimmed[pos + "hl.bind(".len()..];
                let key_part = if let Some(comma_pos) = after.find(',') {
                    &after[..comma_pos]
                } else {
                    after
                };

                let cleaned = key_part
                    .replace("MOD", "SUPER")
                    .replace("..", "")
                    .replace('"', "")
                    .replace('\'', "")
                    .trim()
                    .to_string();

                if !cleaned.is_empty() {
                    let parts: Vec<String> = cleaned
                        .split('+')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .map(|s| {
                            if s.contains("numberkey") {
                                "1..9".to_string()
                            } else if s.eq_ignore_ascii_case("return") {
                                "RETURN".to_string()
                            } else if s.eq_ignore_ascii_case("left") {
                                "Left".to_string()
                            } else if s.eq_ignore_ascii_case("right") {
                                "Right".to_string()
                            } else if s.eq_ignore_ascii_case("up") {
                                "Up".to_string()
                            } else if s.eq_ignore_ascii_case("down") {
                                "Down".to_string()
                            } else {
                                s.to_string()
                            }
                        })
                        .collect();

                    if !parts.is_empty() {
                        current_keys = Some(parts);
                    }
                }
            }

            if let Some(keys) = &current_keys {
                if let Some(desc_idx) = trimmed.find("description =") {
                    let rest = &trimmed[desc_idx + "description =".len()..];
                    let desc = rest
                        .trim()
                        .trim_matches(|c| c == '"' || c == '\'' || c == ',' || c == '{' || c == '}')
                        .trim();

                    if !desc.is_empty() {
                        let desc_str = if desc.starts_with("Workspace ") {
                            "Switch to workspace 1..9".to_string()
                        } else if desc.starts_with("Move window to workspace ") {
                            "Move window to workspace 1..9".to_string()
                        } else {
                            desc.to_string()
                        };

                        let category = Self::categorize_bind(keys, &desc_str);
                        let raw_keys = format!("{} {}", keys.join(" ").to_lowercase(), desc_str.to_lowercase());

                        if !items.iter().any(|it: &KeybindItem| it.keys == *keys && it.description == desc_str) {
                            items.push(KeybindItem {
                                keys: keys.clone(),
                                description: desc_str,
                                category,
                                raw_keys,
                            });
                        }
                        current_keys = None;
                    }
                }
            }
        }

        items
    }

    fn load_keybinds() -> Vec<KeybindItem> {
        if let Some(path) = Self::find_binds_lua_path() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let parsed = Self::parse_binds_lua(&content);
                if !parsed.is_empty() {
                    return parsed;
                }
            }
        }
        Self::fallback_keybinds()
    }

    fn fallback_keybinds() -> Vec<KeybindItem> {
        vec![
            // 1. Command Center & Launchers
            KeybindItem {
                keys: vec!["SUPER".into(), "A".into()],
                description: "msUI Quick Settings Control Center".into(),
                category: KeybindCategory::Launchers,
                raw_keys: "super a control center".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "SPACE".into()],
                description: "msUI Application Launcher".into(),
                category: KeybindCategory::Launchers,
                raw_keys: "super space launcher search".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "TAB".into()],
                description: "msUI App Drawer & Window Task Switcher".into(),
                category: KeybindCategory::Launchers,
                raw_keys: "super tab drawer tasks switch".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "I".into()],
                description: "msUI Wi-Fi & Internet Manager GUI".into(),
                category: KeybindCategory::Launchers,
                raw_keys: "super i wifi network internet".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "SHIFT".into(), "I".into()],
                description: "Wi-Fi Popdown TUI (wifitui)".into(),
                category: KeybindCategory::Launchers,
                raw_keys: "super shift i wifi tui wifitui terminal".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "CTRL".into(), "SPACE".into()],
                description: "msUI System Settings Control Panel".into(),
                category: KeybindCategory::Launchers,
                raw_keys: "super ctrl space settings panel config".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "K".into()],
                description: "msUI Keyboard Shortcuts Cheatsheet".into(),
                category: KeybindCategory::Launchers,
                raw_keys: "super k keybinds shortcuts cheatsheet help".into(),
            },

            // 2. Applications & Files
            KeybindItem {
                keys: vec!["SUPER".into(), "ENTER".into()],
                description: "Foot Terminal (in current directory)".into(),
                category: KeybindCategory::Apps,
                raw_keys: "super return enter foot terminal".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "SHIFT".into(), "ENTER".into()],
                description: "Kitty Terminal (in current directory)".into(),
                category: KeybindCategory::Apps,
                raw_keys: "super shift return enter kitty terminal".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "Q".into()],
                description: "Second Terminal (Kitty)".into(),
                category: KeybindCategory::Apps,
                raw_keys: "super q second terminal kitty".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "E".into()],
                description: "GUI File Manager (Flea)".into(),
                category: KeybindCategory::Apps,
                raw_keys: "super e file manager flea gui".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "SHIFT".into(), "E".into()],
                description: "Yazi TUI File Manager".into(),
                category: KeybindCategory::Apps,
                raw_keys: "super shift e yazi tui file manager".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "CTRL".into(), "E".into()],
                description: "Yazi File Manager in Foot Terminal".into(),
                category: KeybindCategory::Apps,
                raw_keys: "super ctrl e yazi foot file manager".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "B".into()],
                description: "Web Browser (Firefox)".into(),
                category: KeybindCategory::Apps,
                raw_keys: "super b browser firefox web internet".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "S".into()],
                description: "LocalSend Nearby File Sharing".into(),
                category: KeybindCategory::Apps,
                raw_keys: "super s localsend share transfer files".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "O".into()],
                description: "Local AI Assistant Floating Window".into(),
                category: KeybindCategory::Apps,
                raw_keys: "super o ai assistant chat local llm".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "ALT".into(), "A".into()],
                description: "Toggle Local AI Engine On/Off (0 MB RAM Free)".into(),
                category: KeybindCategory::Apps,
                raw_keys: "super alt a ai engine toggle memory ram".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "SHIFT".into(), "A".into()],
                description: "AI Explain Selected Screen Text".into(),
                category: KeybindCategory::Apps,
                raw_keys: "super shift a ai explain selection text ocr".into(),
            },

            // 3. Window & Layout
            KeybindItem {
                keys: vec!["SUPER".into(), "F".into()],
                description: "Toggle Active Window Floating State".into(),
                category: KeybindCategory::WindowLayout,
                raw_keys: "super f float floating toggle window".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "C".into()],
                description: "Close Active Window".into(),
                category: KeybindCategory::WindowLayout,
                raw_keys: "super c close kill window".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "J".into()],
                description: "Toggle Split Orientation".into(),
                category: KeybindCategory::WindowLayout,
                raw_keys: "super j split togglesplit layout".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "D".into()],
                description: "Switch to Dwindle Layout".into(),
                category: KeybindCategory::WindowLayout,
                raw_keys: "super d dwindle layout".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "ALT".into(), "M".into()],
                description: "Switch to Master Layout".into(),
                category: KeybindCategory::WindowLayout,
                raw_keys: "super alt m master layout".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "G".into()],
                description: "Switch to Scrolling Layout".into(),
                category: KeybindCategory::WindowLayout,
                raw_keys: "super g scroll scrolling layout".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "SHIFT".into(), "F".into()],
                description: "Switch to Monocle Layout".into(),
                category: KeybindCategory::WindowLayout,
                raw_keys: "super shift f monocle fullscreen layout".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "W".into()],
                description: "Toggle Top Status Bar Visibility".into(),
                category: KeybindCategory::WindowLayout,
                raw_keys: "super w topbar bar toggle status".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "ALT".into(), "U".into()],
                description: "msUI Desktop Theme Switcher".into(),
                category: KeybindCategory::WindowLayout,
                raw_keys: "super alt u theme switcher appearance colors".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "ALT".into(), "W".into()],
                description: "Switch Desktop Wallpaper".into(),
                category: KeybindCategory::WindowLayout,
                raw_keys: "super alt w wallpaper background image".into(),
            },

            // 4. System (SUPER+CTRL)
            KeybindItem {
                keys: vec!["SUPER".into(), "CTRL".into(), "A".into()],
                description: "Audio Settings & Routing (msUI)".into(),
                category: KeybindCategory::System,
                raw_keys: "super ctrl a audio volume routing mixer".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "CTRL".into(), "B".into()],
                description: "Bluetooth Settings Manager (msUI)".into(),
                category: KeybindCategory::System,
                raw_keys: "super ctrl b bluetooth devices bt".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "CTRL".into(), "W".into()],
                description: "Network & Wi-Fi Settings (msUI)".into(),
                category: KeybindCategory::System,
                raw_keys: "super ctrl w network wifi internet".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "CTRL".into(), "P".into()],
                description: "Battery Diagnostics & Power Profiles (msUI)".into(),
                category: KeybindCategory::System,
                raw_keys: "super ctrl p battery power profiles conservation".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "CTRL".into(), "M".into()],
                description: "Media Player Controller (msUI)".into(),
                category: KeybindCategory::System,
                raw_keys: "super ctrl m media music player mpris".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "CTRL".into(), "D".into()],
                description: "Storage & Removable Drives (msUI)".into(),
                category: KeybindCategory::System,
                raw_keys: "super ctrl d storage disks usb mount".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "CTRL".into(), "S".into()],
                description: "Share Menu (LocalSend)".into(),
                category: KeybindCategory::System,
                raw_keys: "super ctrl s share localsend transfer".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "CTRL".into(), "T".into()],
                description: "Activity Monitor (btop)".into(),
                category: KeybindCategory::System,
                raw_keys: "super ctrl t activity btop monitor cpu ram tasks".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "CTRL".into(), "C".into()],
                description: "Capture Controls Menu".into(),
                category: KeybindCategory::System,
                raw_keys: "super ctrl c capture screenshot record menu".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "CTRL".into(), "O".into()],
                description: "System Toggle Menu".into(),
                category: KeybindCategory::System,
                raw_keys: "super ctrl o toggle system settings menu".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "CTRL".into(), "H".into()],
                description: "Hardware Reload Menu".into(),
                category: KeybindCategory::System,
                raw_keys: "super ctrl h hardware reload gpu wifi audio".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "CTRL".into(), ".".into()],
                description: "Media & Video Transcoding Menu".into(),
                category: KeybindCategory::System,
                raw_keys: "super ctrl period transcode convert video media".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "L".into()],
                description: "Lock Screen (hyprlock)".into(),
                category: KeybindCategory::System,
                raw_keys: "super l lock screen hyprlock".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "P".into()],
                description: "Audio Mixer (pavucontrol)".into(),
                category: KeybindCategory::System,
                raw_keys: "super p audio mixer pavucontrol volume".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "SHIFT".into(), "P".into()],
                description: "Interactive Power & Session Menu".into(),
                category: KeybindCategory::System,
                raw_keys: "super shift p power shutdown reboot sleep menu".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "SHIFT".into(), "R".into()],
                description: "Reboot System".into(),
                category: KeybindCategory::System,
                raw_keys: "super shift r reboot restart".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "SHIFT".into(), "X".into()],
                description: "Logout / Exit Hyprland Session".into(),
                category: KeybindCategory::System,
                raw_keys: "super shift x logout exit session quit".into(),
            },

            // 5. Capture & Utilities
            KeybindItem {
                keys: vec!["Print".into()],
                description: "Screenshot Active Window (grim + wl-copy)".into(),
                category: KeybindCategory::CaptureUtil,
                raw_keys: "print screenshot window active grim".into(),
            },
            KeybindItem {
                keys: vec!["SHIFT".into(), "Print".into()],
                description: "Screenshot Selected Screen Area (slurp + grim)".into(),
                category: KeybindCategory::CaptureUtil,
                raw_keys: "shift print screenshot area slurp grim crop".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "SHIFT".into(), "C".into()],
                description: "Color Picker (hyprpicker)".into(),
                category: KeybindCategory::CaptureUtil,
                raw_keys: "super shift c color picker hyprpicker hex rgb".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "SHIFT".into(), "T".into()],
                description: "OCR Text Recognition from Selection".into(),
                category: KeybindCategory::CaptureUtil,
                raw_keys: "super shift t ocr text recognize tesseract".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "ALT".into(), "R".into()],
                description: "Toggle Screen Recording (wf-recorder)".into(),
                category: KeybindCategory::CaptureUtil,
                raw_keys: "super alt r screen recording record wf-recorder video".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "SHIFT".into(), "V".into()],
                description: "Clipboard History Manager (msUI)".into(),
                category: KeybindCategory::CaptureUtil,
                raw_keys: "super shift v clipboard manager history paste".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "N".into()],
                description: "Notification Center (msUI)".into(),
                category: KeybindCategory::CaptureUtil,
                raw_keys: "super n notifications center alerts".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "R".into()],
                description: "Reminders & Timers Manager (msUI)".into(),
                category: KeybindCategory::CaptureUtil,
                raw_keys: "super r reminder timers clock alarm".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "SHIFT".into(), "N".into()],
                description: "Toggle Do Not Disturb (DND)".into(),
                category: KeybindCategory::CaptureUtil,
                raw_keys: "super shift n do not disturb dnd quiet".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "U".into()],
                description: "System Package Updates".into(),
                category: KeybindCategory::CaptureUtil,
                raw_keys: "super u system updates upgrade pacman".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "Y".into()],
                description: "System Cache & Memory Cleanup".into(),
                category: KeybindCategory::CaptureUtil,
                raw_keys: "super y system cleanup clean garbage cache".into(),
            },

            // 6. Navigation & Workspaces
            KeybindItem {
                keys: vec!["SUPER".into(), "← / → / ↑ / ↓".into()],
                description: "Focus Window in Direction (Left, Right, Up, Down)".into(),
                category: KeybindCategory::Navigation,
                raw_keys: "super left right up down focus window navigate".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "SHIFT".into(), "← / → / ↑ / ↓".into()],
                description: "Move Active Window in Direction".into(),
                category: KeybindCategory::Navigation,
                raw_keys: "super shift left right up down move window".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "1 - 10".into()],
                description: "Switch to Workspace 1 through 10".into(),
                category: KeybindCategory::Navigation,
                raw_keys: "super 1 2 3 4 5 6 7 8 9 10 workspace switch".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "SHIFT".into(), "1 - 10".into()],
                description: "Move Window to Workspace 1 through 10".into(),
                category: KeybindCategory::Navigation,
                raw_keys: "super shift 1 2 3 4 5 6 7 8 9 10 move workspace".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "Scroll".into()],
                description: "Cycle Next / Previous Workspace".into(),
                category: KeybindCategory::Navigation,
                raw_keys: "super mouse scroll wheel next previous workspace".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "Mouse 1".into()],
                description: "Drag & Move Window".into(),
                category: KeybindCategory::Navigation,
                raw_keys: "super mouse 1 drag move left click".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "Mouse 2".into()],
                description: "Resize Window with Cursor".into(),
                category: KeybindCategory::Navigation,
                raw_keys: "super mouse 2 resize right click".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "`".into()],
                description: "Toggle Dropdown Terminal Scratchpad".into(),
                category: KeybindCategory::Navigation,
                raw_keys: "super grave term dropdown terminal scratchpad".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "M".into()],
                description: "Minimize Active Window to Scratchpad".into(),
                category: KeybindCategory::Navigation,
                raw_keys: "super m minimize window scratchpad hide".into(),
            },
            KeybindItem {
                keys: vec!["SUPER".into(), "SHIFT".into(), "M".into()],
                description: "Toggle Minimized Scratchpad Overlay".into(),
                category: KeybindCategory::Navigation,
                raw_keys: "super shift m toggle scratchpad minimized overlay restore".into(),
            },
        ]
    }

    fn rebuild_filtered(&mut self) {
        let q = self.query.trim().to_lowercase();
        let cat = self.category;

        self.filtered = self
            .all_items
            .iter()
            .filter(|item| {
                if cat != KeybindCategory::All && item.category != cat {
                    return false;
                }
                if q.is_empty() {
                    return true;
                }
                let score_desc = self.matcher.fuzzy_match(&item.description, &q).is_some();
                let score_raw = self.matcher.fuzzy_match(&item.raw_keys, &q).is_some();
                score_desc || score_raw
            })
            .cloned()
            .collect();
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::QueryChanged(new_query) => {
                self.query = new_query;
                self.rebuild_filtered();
            }
            Message::SetCategory(cat) => {
                self.category = cat;
                self.rebuild_filtered();
                return snap_to(SCROLL_ID.clone(), RelativeOffset::START);
            }
            Message::EnsureFocus => {
                return focus(INPUT_ID.clone());
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
                    _ => {
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
        let p = ThemeManager::current_theme().palette();

        // 1. Header
        let icon_elem = text("󰌌").size(22);
        let title_elem = text("Keyboard Shortcuts").size(16);
        let count_badge = container(
            text(format!("{} shortcuts", self.filtered.len())).size(11)
        )
        .padding([2, 8])
        .style(move |_| container::Style {
            background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.08).into()),
            border: Border {
                radius: 12.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        let close_btn = button(text("✕").size(14))
            .padding([4, 8])
            .style(button::subtle)
            .on_press(Message::Close);

        let header = row![
            icon_elem,
            title_elem,
            count_badge,
            space().width(Length::Fill),
            close_btn
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        // 2. Search Input
        let search_bar = container(
            text_input("󰍉  Type to search shortcuts (e.g. wifi, record, term, split)...", &self.query)
                .id(INPUT_ID.clone())
                .on_input(Message::QueryChanged)
                .size(13)
                .padding([10, 14])
        )
        .width(Length::Fill);

        // 3. Category Filter Tabs
        let categories = [
            KeybindCategory::All,
            KeybindCategory::Launchers,
            KeybindCategory::Apps,
            KeybindCategory::WindowLayout,
            KeybindCategory::System,
            KeybindCategory::CaptureUtil,
            KeybindCategory::Navigation,
        ];

        let mut cat_row = row![].spacing(6).align_y(Alignment::Center);
        for cat in categories {
            let is_selected = self.category == cat;
            let cat_label = cat.label();
            let cat_btn = button(text(cat_label).size(11))
                .padding([4, 10])
                .style(if is_selected { button::primary } else { button::secondary })
                .on_press(Message::SetCategory(cat));
            cat_row = cat_row.push(cat_btn);
        }
        let cat_bar = scrollable(cat_row).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::default()));

        // 4. Keybinding Rows
        let list_content: Element<'_, Message> = if self.filtered.is_empty() {
            container(
                column![
                    text("󰍉").size(32),
                    text(format!("No shortcuts matching \"{}\"", self.query)).size(14),
                    text("Try a different search query or clear the filter").size(12),
                ]
                .spacing(8)
                .align_x(Alignment::Center)
            )
            .width(Length::Fill)
            .padding(40)
            .align_x(Alignment::Center)
            .into()
        } else {
            let mut col = column![].spacing(6).width(Length::Fill);

            for item in &self.filtered {
                // Key badges
                let mut keys_row = row![].spacing(4).align_y(Alignment::Center);
                for (idx, k) in item.keys.iter().enumerate() {
                    if idx > 0 {
                        keys_row = keys_row.push(text("+").size(11));
                    }
                    let key_chip = container(
                        text(k).size(11)
                    )
                    .padding([2, 6])
                    .style(move |_| container::Style {
                        background: Some(Color::from_rgba(p.primary.r, p.primary.g, p.primary.b, 0.18).into()),
                        border: Border {
                            color: Color::from_rgba(p.primary.r, p.primary.g, p.primary.b, 0.45),
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    });
                    keys_row = keys_row.push(key_chip);
                }

                let keys_box = container(keys_row).width(Length::Fixed(240.0));
                let desc_box = container(text(&item.description).size(12)).width(Length::Fill);

                let cat_tag = container(
                    text(item.category.short_label()).size(10)
                )
                .padding([2, 6])
                .style(move |_| container::Style {
                    background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.05).into()),
                    border: Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                });

                let item_card = container(
                    row![keys_box, desc_box, cat_tag]
                        .spacing(12)
                        .align_y(Alignment::Center)
                )
                .padding([8, 12])
                .style(subcard_style(crate::modal_style::is_frosted()))
                .width(Length::Fill);

                col = col.push(item_card);
            }

            scrollable(col)
                .id(SCROLL_ID.clone())
                .height(Length::Fill)
                .into()
        };

        // Modal Content Wrap
        let content = column![
            header,
            search_bar,
            cat_bar,
            list_content,
        ]
        .spacing(12)
        .padding(18);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(modal_card_style(crate::modal_style::is_frosted()))
            .into()
    }
}
