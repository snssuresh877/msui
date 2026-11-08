//! Semantic tokens mapping Material 3 core palettes to application UI roles.

use super::hct::Srgb;
use super::palette::CorePalette;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ColorSchemeMode {
    #[default]
    Dark,
    Light,
}

impl ColorSchemeMode {
    pub fn is_dark(&self) -> bool {
        matches!(self, Self::Dark)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

/// A complete set of semantic color tokens in hex format (#rrggbb).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticTokens {
    pub mode: ColorSchemeMode,

    // Primary Accents
    pub primary: String,
    pub on_primary: String,
    pub primary_container: String,
    pub on_primary_container: String,

    // Secondary Accents
    pub secondary: String,
    pub on_secondary: String,
    pub secondary_container: String,
    pub on_secondary_container: String,

    // Tertiary Accents (Pop badges, alerts)
    pub tertiary: String,
    pub on_tertiary: String,
    pub tertiary_container: String,
    pub on_tertiary_container: String,

    // Background & Surfaces
    pub background: String,
    pub on_background: String,
    pub surface: String,
    pub on_surface: String,
    pub surface_variant: String,
    pub on_surface_variant: String,
    pub surface_dim: String,
    pub surface_bright: String,

    // Borders & Dividers
    pub outline: String,
    pub outline_variant: String,

    // Errors
    pub error: String,
    pub on_error: String,
}

impl SemanticTokens {
    pub fn from_palette(palette: &CorePalette, mode: ColorSchemeMode) -> Self {
        match mode {
            ColorSchemeMode::Dark => Self {
                mode,
                primary: palette.primary.tone_hex(80.0),
                on_primary: palette.primary.tone_hex(20.0),
                primary_container: palette.primary.tone_hex(30.0),
                on_primary_container: palette.primary.tone_hex(90.0),

                secondary: palette.secondary.tone_hex(80.0),
                on_secondary: palette.secondary.tone_hex(20.0),
                secondary_container: palette.secondary.tone_hex(30.0),
                on_secondary_container: palette.secondary.tone_hex(90.0),

                tertiary: palette.tertiary.tone_hex(80.0),
                on_tertiary: palette.tertiary.tone_hex(20.0),
                tertiary_container: palette.tertiary.tone_hex(30.0),
                on_tertiary_container: palette.tertiary.tone_hex(90.0),

                background: palette.neutral.tone_hex(6.0),
                on_background: palette.neutral.tone_hex(90.0),
                surface: palette.neutral.tone_hex(10.0),
                on_surface: palette.neutral.tone_hex(90.0),
                surface_variant: palette.neutral_variant.tone_hex(20.0),
                on_surface_variant: palette.neutral_variant.tone_hex(80.0),
                surface_dim: palette.neutral.tone_hex(6.0),
                surface_bright: palette.neutral.tone_hex(24.0),

                outline: palette.neutral_variant.tone_hex(60.0),
                outline_variant: palette.neutral_variant.tone_hex(30.0),

                error: palette.error.tone_hex(80.0),
                on_error: palette.error.tone_hex(20.0),
            },
            ColorSchemeMode::Light => Self {
                mode,
                primary: palette.primary.tone_hex(40.0),
                on_primary: palette.primary.tone_hex(100.0),
                primary_container: palette.primary.tone_hex(90.0),
                on_primary_container: palette.primary.tone_hex(10.0),

                secondary: palette.secondary.tone_hex(40.0),
                on_secondary: palette.secondary.tone_hex(100.0),
                secondary_container: palette.secondary.tone_hex(90.0),
                on_secondary_container: palette.secondary.tone_hex(10.0),

                tertiary: palette.tertiary.tone_hex(40.0),
                on_tertiary: palette.tertiary.tone_hex(100.0),
                tertiary_container: palette.tertiary.tone_hex(90.0),
                on_tertiary_container: palette.tertiary.tone_hex(10.0),

                background: palette.neutral.tone_hex(94.0),
                on_background: palette.neutral.tone_hex(10.0),
                surface: palette.neutral.tone_hex(98.0),
                on_surface: palette.neutral.tone_hex(10.0),
                surface_variant: palette.neutral_variant.tone_hex(90.0),
                on_surface_variant: palette.neutral_variant.tone_hex(25.0),
                surface_dim: palette.neutral.tone_hex(86.0),
                surface_bright: palette.neutral.tone_hex(99.0),

                outline: palette.neutral_variant.tone_hex(55.0),
                outline_variant: palette.neutral_variant.tone_hex(82.0),

                error: palette.error.tone_hex(40.0),
                on_error: palette.error.tone_hex(100.0),
            },
        }
    }

    pub fn primary_rgb(&self) -> Srgb {
        Srgb::from_hex(&self.primary).unwrap_or(Srgb::new(147, 51, 234))
    }

    pub fn background_rgb(&self) -> Srgb {
        Srgb::from_hex(&self.background).unwrap_or(Srgb::new(18, 18, 24))
    }

    pub fn surface_rgb(&self) -> Srgb {
        Srgb::from_hex(&self.surface).unwrap_or(Srgb::new(26, 26, 36))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::contrast::satisfies_wcag_aa;

    #[test]
    fn test_semantic_contrast_guarantees() {
        let palette = CorePalette::from_hex("#9333ea").unwrap();

        // Dark mode
        let dark = SemanticTokens::from_palette(&palette, ColorSchemeMode::Dark);
        let dark_primary = Srgb::from_hex(&dark.primary).unwrap();
        let dark_on_primary = Srgb::from_hex(&dark.on_primary).unwrap();
        assert!(satisfies_wcag_aa(dark_on_primary, dark_primary));

        let dark_bg = Srgb::from_hex(&dark.background).unwrap();
        let dark_on_bg = Srgb::from_hex(&dark.on_background).unwrap();
        assert!(satisfies_wcag_aa(dark_on_bg, dark_bg));

        // Light mode
        let light = SemanticTokens::from_palette(&palette, ColorSchemeMode::Light);
        let light_primary = Srgb::from_hex(&light.primary).unwrap();
        let light_on_primary = Srgb::from_hex(&light.on_primary).unwrap();
        assert!(satisfies_wcag_aa(light_on_primary, light_primary));
    }
}
