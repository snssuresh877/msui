//! Tonal palette generation for Material Design 3.
//!
//! A Tonal Palette defines a range of colors of consistent Hue and Chroma,
//! varying only by Tone ($0..=100$).

use super::hct::{Hct, Srgb};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TonalPalette {
    pub hue: f64,
    pub chroma: f64,
}

impl TonalPalette {
    pub fn of(hue: f64, chroma: f64) -> Self {
        Self {
            hue: ((hue % 360.0) + 360.0) % 360.0,
            chroma: chroma.max(0.0),
        }
    }

    pub fn from_hct(hct: Hct) -> Self {
        Self::of(hct.hue, hct.chroma)
    }

    /// Evaluates the color at a specific tone (0.0 = black, 100.0 = white).
    pub fn tone(&self, tone: f64) -> Srgb {
        Hct::new(self.hue, self.chroma, tone).to_srgb()
    }

    pub fn tone_hex(&self, tone: f64) -> String {
        self.tone(tone).to_hex()
    }
}

/// A complete Material 3 Core Palette derived from a seed accent color.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorePalette {
    pub primary: TonalPalette,
    pub secondary: TonalPalette,
    pub tertiary: TonalPalette,
    pub neutral: TonalPalette,
    pub neutral_variant: TonalPalette,
    pub error: TonalPalette,
}

impl CorePalette {
    /// Generates the 6 Material 3 tonal palettes from a single seed color.
    pub fn from_seed(seed: Hct) -> Self {
        let primary_chroma = seed.chroma.max(48.0);
        let primary = TonalPalette::of(seed.hue, primary_chroma);
        let secondary = TonalPalette::of(seed.hue, 16.0);
        let tertiary = TonalPalette::of((seed.hue + 60.0) % 360.0, 24.0);
        let neutral = TonalPalette::of(seed.hue, 4.0);
        let neutral_variant = TonalPalette::of(seed.hue, 8.0);
        let error = TonalPalette::of(25.0, 84.0);

        Self {
            primary,
            secondary,
            tertiary,
            neutral,
            neutral_variant,
            error,
        }
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        Hct::from_hex(hex).map(Self::from_seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_palette_generation() {
        let palette = CorePalette::from_hex("#9333ea").unwrap();
        // Check tone 10 (very dark) vs tone 90 (very light)
        let primary_10 = palette.primary.tone(10.0);
        let primary_90 = palette.primary.tone(90.0);

        assert!(primary_10.to_tone() < primary_90.to_tone());
        assert_eq!(palette.neutral.tone(0.0), Srgb::new(0, 0, 0));
        assert_eq!(palette.neutral.tone(100.0), Srgb::new(255, 255, 255));
    }
}
