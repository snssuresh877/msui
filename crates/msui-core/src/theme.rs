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
        let cfg = crate::config::MsuiConfig::load_or_default();
        if cfg.color.mode == "dynamic" {
            let tokens = crate::color::ColorEngine::current_tokens();
            let p_bg = crate::config::parse_hex_color(&tokens.background).unwrap_or((0.1, 0.1, 0.1));
            let p_fg = crate::config::parse_hex_color(&tokens.on_background).unwrap_or((0.9, 0.9, 0.9));
            let p_pri = crate::config::parse_hex_color(&tokens.primary).unwrap_or((0.5, 0.3, 0.9));
            let p_err = crate::config::parse_hex_color(&tokens.error).unwrap_or((0.8, 0.2, 0.2));

            let palette = iced::theme::Palette {
                background: iced::Color::from_rgb(p_bg.0, p_bg.1, p_bg.2),
                text: iced::Color::from_rgb(p_fg.0, p_fg.1, p_fg.2),
                primary: iced::Color::from_rgb(p_pri.0, p_pri.1, p_pri.2),
                success: iced::Color::from_rgb8(34, 197, 94),
                danger: iced::Color::from_rgb(p_err.0, p_err.1, p_err.2),
                warning: iced::Color::from_rgb8(234, 179, 8),
            };
            return Theme::custom(format!("msui-{}", tokens.mode.as_str()), palette);
        }

        let name = Self::current_theme_name();
        if cfg.color.scheme == "light" {
            match name.to_lowercase().replace(' ', "-").replace('_', "-").as_str() {
                "gruvbox-dark" | "gruvbox-light" => Theme::GruvboxLight,
                "tokyo-night" | "tokyo-night-storm" | "tokyo-night-light" => Theme::TokyoNightLight,
                _ => Theme::CatppuccinLatte,
            }
        } else {
            Self::to_iced_theme(&name)
        }
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
