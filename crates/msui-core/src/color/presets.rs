//! Curated design presets providing harmonious fallbacks and manual theme selections.

use super::palette::CorePalette;
use super::semantic::{ColorSchemeMode, SemanticTokens};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub accent_hex: &'static str,
    pub bg_hex: &'static str,
}

pub const PRESETS: &[ColorPreset] = &[
    ColorPreset {
        id: "midnight",
        name: "Midnight",
        description: "Dark charcoal backdrop with vivid violet neon accents",
        accent_hex: "#9333ea",
        bg_hex: "#121218",
    },
    ColorPreset {
        id: "ember",
        name: "Ember",
        description: "Dark graphite surface with warm amber gold accents",
        accent_hex: "#f59e0b",
        bg_hex: "#181512",
    },
    ColorPreset {
        id: "nordic",
        name: "Nordic",
        description: "Cool arctic slate with vibrant mint emerald accents",
        accent_hex: "#10b981",
        bg_hex: "#141c18",
    },
    ColorPreset {
        id: "ocean",
        name: "Ocean",
        description: "Deep oceanic navy with electric cyan accents",
        accent_hex: "#06b6d4",
        bg_hex: "#0f172a",
    },
    ColorPreset {
        id: "catppuccin",
        name: "Catppuccin Mocha",
        description: "Modern pastel slate with soft mauve accents",
        accent_hex: "#cba6f7",
        bg_hex: "#1e1e2e",
    },
    ColorPreset {
        id: "tokyo-night",
        name: "Tokyo Night",
        description: "Cyber twilight indigo with vibrant neon blue",
        accent_hex: "#7aa2f7",
        bg_hex: "#1a1b26",
    },
];

impl ColorPreset {
    pub fn find(id: &str) -> Option<&'static ColorPreset> {
        let norm = id.to_lowercase().replace(' ', "-").replace('_', "-");
        PRESETS.iter().find(|p| p.id == norm)
    }

    pub fn to_palette(&self) -> CorePalette {
        CorePalette::from_hex(self.accent_hex).unwrap_or_else(|| {
            CorePalette::from_hex("#9333ea").unwrap()
        })
    }

    pub fn to_tokens(&self, mode: ColorSchemeMode) -> SemanticTokens {
        let palette = self.to_palette();
        let mut tokens = SemanticTokens::from_palette(&palette, mode);
        // If in dark mode, preserve the preset's signature tuned dark background
        if mode.is_dark() {
            tokens.background = self.bg_hex.to_string();
        }
        tokens
    }
}
