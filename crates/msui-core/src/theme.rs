use iced::Theme;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThemeDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub accent_hex: &'static str,
    pub bg_hex: &'static str,
}

pub struct ThemeManager;

pub const BUILTIN_THEMES: &[ThemeDefinition] = &[
    ThemeDefinition {
        id: "catppuccin-mocha",
        name: "Catppuccin Mocha",
        description: "Modern Pastel Slate (Dark)",
        accent_hex: "#cba6f7",
        bg_hex: "#1e1e2e",
    },
    ThemeDefinition {
        id: "catppuccin-macchiato",
        name: "Catppuccin Macchiato",
        description: "Medium Contrast Pastel",
        accent_hex: "#c6a0f6",
        bg_hex: "#24273a",
    },
    ThemeDefinition {
        id: "catppuccin-frappe",
        name: "Catppuccin Frappe",
        description: "Soft Muted Twilight Pastel",
        accent_hex: "#ca9ee6",
        bg_hex: "#303446",
    },
    ThemeDefinition {
        id: "catppuccin-latte",
        name: "Catppuccin Latte",
        description: "Clean Daytime Light Pastel",
        accent_hex: "#8839ef",
        bg_hex: "#eff1f5",
    },
    ThemeDefinition {
        id: "tokyo-night",
        name: "Tokyo Night",
        description: "Cyber Neon & Violet Twilight",
        accent_hex: "#7aa2f7",
        bg_hex: "#1a1b26",
    },
    ThemeDefinition {
        id: "tokyo-night-storm",
        name: "Tokyo Night Storm",
        description: "Deep Twilight Indigo Storm",
        accent_hex: "#7aa2f7",
        bg_hex: "#24283b",
    },
    ThemeDefinition {
        id: "gruvbox-dark",
        name: "Gruvbox Dark",
        description: "Warm Retro Gold & Wood",
        accent_hex: "#fabd2f",
        bg_hex: "#282828",
    },
    ThemeDefinition {
        id: "gruvbox-light",
        name: "Gruvbox Light",
        description: "Warm Retro Paper Light",
        accent_hex: "#b57614",
        bg_hex: "#fbf1c7",
    },
    ThemeDefinition {
        id: "nord",
        name: "Nord",
        description: "Cool Arctic Frost Slate",
        accent_hex: "#88c0d0",
        bg_hex: "#2e3440",
    },
    ThemeDefinition {
        id: "dracula",
        name: "Dracula",
        description: "Vibrant Gothic Purple & Pink",
        accent_hex: "#bd93f9",
        bg_hex: "#282a36",
    },
    ThemeDefinition {
        id: "kanagawa-wave",
        name: "Kanagawa Wave",
        description: "Sumi Ink & Great Wave",
        accent_hex: "#7e9cd8",
        bg_hex: "#1f1f28",
    },
    ThemeDefinition {
        id: "oxocarbon",
        name: "Oxocarbon",
        description: "Minimalist IBM High Contrast",
        accent_hex: "#ff7eb6",
        bg_hex: "#161616",
    },
];

impl ThemeManager {
    pub fn cache_file_path() -> Option<PathBuf> {
        if let Ok(cache) = std::env::var("XDG_CACHE_HOME") {
            return Some(PathBuf::from(cache).join("msui").join("current_theme.txt"));
        }
        if let Ok(home) = std::env::var("HOME") {
            return Some(PathBuf::from(home).join(".cache").join("msui").join("current_theme.txt"));
        }
        None
    }

    pub fn current_theme_name() -> String {
        if let Some(path) = Self::cache_file_path() {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    let trimmed = content.trim();
                    if !trimmed.is_empty() {
                        return trimmed.to_string();
                    }
                }
            }
        }
        "Catppuccin Mocha".to_string()
    }

    pub fn current_theme() -> Theme {
        let name = Self::current_theme_name();
        Self::to_iced_theme(&name)
    }

    pub fn set_theme(name: &str) -> bool {
        if let Some(path) = Self::cache_file_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if fs::write(&path, name).is_ok() {
                tracing::info!("Updated active theme to {}", name);
                return true;
            }
        }
        false
    }

    pub fn to_iced_theme(name: &str) -> Theme {
        match name.to_lowercase().replace(' ', "-").replace('_', "-").as_str() {
            "catppuccin-mocha" => Theme::CatppuccinMocha,
            "catppuccin-macchiato" => Theme::CatppuccinMacchiato,
            "catppuccin-frappe" => Theme::CatppuccinFrappe,
            "catppuccin-latte" => Theme::CatppuccinLatte,
            "tokyo-night" => Theme::TokyoNight,
            "tokyo-night-storm" => Theme::TokyoNightStorm,
            "gruvbox-dark" => Theme::GruvboxDark,
            "gruvbox-light" => Theme::GruvboxLight,
            "nord" => Theme::Nord,
            "dracula" => Theme::Dracula,
            "kanagawa-wave" | "kanagawa" => Theme::KanagawaWave,
            "oxocarbon" => Theme::Oxocarbon,
            _ => Theme::CatppuccinMocha,
        }
    }
}
