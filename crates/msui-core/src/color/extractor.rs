//! Wallpaper decoding and dominant color extraction.

use super::hct::{Hct, Srgb};
use super::palette::CorePalette;
use super::quantizer::Quantizer;
use std::path::Path;

pub struct WallpaperExtractor;

impl WallpaperExtractor {
    /// Extracts the dominant seed accent color from a wallpaper file.
    pub fn extract_seed_accent(path: &Path) -> anyhow::Result<Hct> {
        let img = image::open(path)?;
        // Fast downsample to 128x128 for instantaneous extraction (<10ms)
        let thumb = img.thumbnail_exact(128, 128).to_rgba8();

        let pixels: Vec<Srgb> = thumb
            .pixels()
            .map(|p| Srgb::new(p[0], p[1], p[2]))
            .collect();

        Ok(Quantizer::find_dominant_accent(&pixels))
    }

    /// Generates a full Material 3 CorePalette directly from a wallpaper file.
    pub fn extract_palette(path: &Path) -> anyhow::Result<CorePalette> {
        let seed = Self::extract_seed_accent(path)?;
        Ok(CorePalette::from_seed(seed))
    }
}
