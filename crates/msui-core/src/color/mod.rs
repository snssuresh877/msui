//! MSUI Adaptive Color Engine based on Material Design 3 HCT.
//!
//! Provides wallpaper color extraction, tonal palettes, semantic tokens,
//! and instant live theme application across Hyprland, Waybar, terminals, GTK, and MSUI.

pub mod contrast;
pub mod exporter;
pub mod extractor;
pub mod hct;
pub mod palette;
pub mod presets;
pub mod quantizer;
pub mod semantic;

pub use exporter::ThemeExporter;
pub use extractor::WallpaperExtractor;
pub use hct::{Hct, Srgb};
pub use palette::{CorePalette, TonalPalette};
pub use presets::{ColorPreset, PRESETS};
pub use quantizer::Quantizer;
pub use semantic::{ColorSchemeMode, SemanticTokens};

use std::path::{Path, PathBuf};
use std::fs;

pub struct ColorEngine;

impl ColorEngine {
    pub fn cache_file_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(home).join(".cache").join("msui").join("current_palette.json"))
    }

    /// Loads the currently active semantic tokens from cache, or falls back to Midnight.
    pub fn current_tokens() -> SemanticTokens {
        if let Some(path) = Self::cache_file_path() {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(tokens) = serde_json::from_str::<SemanticTokens>(&content) {
                        return tokens;
                    }
                }
            }
        }
        // Default to Midnight preset
        ColorPreset::find("midnight").unwrap().to_tokens(ColorSchemeMode::Dark)
    }

    /// Saves semantic tokens to cache and exports them across the desktop live.
    pub fn save_and_apply(tokens: &SemanticTokens) -> anyhow::Result<()> {
        if let Some(path) = Self::cache_file_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let json = serde_json::to_string_pretty(tokens)?;
            let _ = fs::write(path, json);
        }

        ThemeExporter::export_and_apply_all(tokens)?;
        Ok(())
    }

    /// Extracts colors from a wallpaper image and applies the theme across the desktop.
    pub fn apply_wallpaper(path: &Path, mode: ColorSchemeMode) -> anyhow::Result<SemanticTokens> {
        let palette = WallpaperExtractor::extract_palette(path)?;
        let tokens = SemanticTokens::from_palette(&palette, mode);
        Self::save_and_apply(&tokens)?;
        Ok(tokens)
    }

    /// Applies a curated design preset (e.g. "midnight", "ember", "nordic", "ocean", "catppuccin").
    pub fn apply_preset(id: &str, mode: ColorSchemeMode) -> anyhow::Result<SemanticTokens> {
        let preset = ColorPreset::find(id)
            .ok_or_else(|| anyhow::anyhow!("Unknown preset: {}", id))?;
        let tokens = preset.to_tokens(mode);
        Self::save_and_apply(&tokens)?;
        Ok(tokens)
    }

    /// Applies a custom accent color hex (e.g. "#9333ea").
    pub fn apply_accent(hex: &str, mode: ColorSchemeMode) -> anyhow::Result<SemanticTokens> {
        let palette = CorePalette::from_hex(hex)
            .ok_or_else(|| anyhow::anyhow!("Invalid hex color: {}", hex))?;
        let tokens = SemanticTokens::from_palette(&palette, mode);
        Self::save_and_apply(&tokens)?;
        Ok(tokens)
    }
}
