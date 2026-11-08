//! Desktop theme template exporter for Hyprland, Waybar, terminals, GTK, and MSUI.

use super::semantic::{ColorSchemeMode, SemanticTokens};
use crate::config::MsuiConfig;
use crate::ipc::{IpcClient, IpcCommand};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct ThemeExporter;

impl ThemeExporter {
    /// Applies the generated semantic tokens live across all supported desktop components.
    pub fn export_and_apply_all(tokens: &SemanticTokens) -> anyhow::Result<()> {
        let home = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/home/m"));

        // 1. Export Hyprland colors
        Self::export_hyprland(&home, tokens)?;

        // 2. Export Waybar CSS
        Self::export_waybar(&home, tokens)?;

        // 3. Export Ghostty terminal theme
        Self::export_ghostty(&home, tokens)?;

        // 4. Export Foot terminal theme
        Self::export_foot(&home, tokens)?;

        // 5. Export GTK 3 & 4 CSS
        Self::export_gtk(&home, tokens)?;

        // 6. Update MSUI GUI config & notify running daemon
        Self::apply_to_msui(tokens)?;

        Ok(())
    }

    pub fn export_hyprland(home: &Path, tokens: &SemanticTokens) -> anyhow::Result<()> {
        let dir = home.join(".config").join("hypr").join("generated");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("colors.conf");

        let clean_bg = tokens.background.trim_start_matches('#');
        let clean_fg = tokens.on_background.trim_start_matches('#');
        let clean_surface = tokens.surface.trim_start_matches('#');
        let clean_primary = tokens.primary.trim_start_matches('#');
        let clean_secondary = tokens.secondary.trim_start_matches('#');
        let clean_outline = tokens.outline.trim_start_matches('#');

        let content = format!(
            "$bg = rgb({clean_bg})\n\
             $fg = rgb({clean_fg})\n\
             $surface = rgb({clean_surface})\n\
             $primary = rgb({clean_primary})\n\
             $secondary = rgb({clean_secondary})\n\
             $outline = rgb({clean_outline})\n\
             $border-active-solid = $primary\n\
             $border-inactive = $outline\n"
        );

        fs::write(&path, content)?;

        // Notify Hyprland to reload config asynchronously
        let _ = Command::new("hyprctl").arg("reload").spawn();
        Ok(())
    }

    pub fn export_waybar(home: &Path, tokens: &SemanticTokens) -> anyhow::Result<()> {
        let dir = home.join(".config").join("waybar");
        if dir.exists() {
            let path = dir.join("colors.css");
            let content = format!(
                "@define-color background {};\n\
                 @define-color foreground {};\n\
                 @define-color surface {};\n\
                 @define-color primary {};\n\
                 @define-color secondary {};\n\
                 @define-color outline {};\n\
                 @define-color error {};\n",
                tokens.background,
                tokens.on_background,
                tokens.surface,
                tokens.primary,
                tokens.secondary,
                tokens.outline,
                tokens.error,
            );
            fs::write(path, content)?;

            // Signal Waybar if running
            let _ = Command::new("killall").args(["-SIGUSR2", "waybar"]).spawn();
        }
        Ok(())
    }

    pub fn export_ghostty(home: &Path, tokens: &SemanticTokens) -> anyhow::Result<()> {
        let dir = home.join(".config").join("ghostty");
        if dir.exists() {
            let path = dir.join("generated-theme.conf");
            let content = format!(
                "background = {}\n\
                 foreground = {}\n\
                 cursor-color = {}\n\
                 selection-background = {}\n\
                 selection-foreground = {}\n",
                tokens.background,
                tokens.on_background,
                tokens.primary,
                tokens.primary,
                tokens.on_primary,
            );
            fs::write(path, content)?;
        }
        Ok(())
    }

    pub fn export_foot(home: &Path, tokens: &SemanticTokens) -> anyhow::Result<()> {
        let dir = home.join(".config").join("foot");
        if dir.exists() {
            let path = dir.join("theme.ini");
            let clean_bg = tokens.background.trim_start_matches('#');
            let clean_fg = tokens.on_background.trim_start_matches('#');
            let clean_primary = tokens.primary.trim_start_matches('#');
            let content = format!(
                "[colors]\n\
                 background={clean_bg}\n\
                 foreground={clean_fg}\n\
                 regular4={clean_primary}\n"
            );
            fs::write(path, content)?;
        }
        Ok(())
    }

    pub fn export_gtk(home: &Path, tokens: &SemanticTokens) -> anyhow::Result<()> {
        let gtk_css = format!(
            "@define-color accent_color {};\n\
             @define-color accent_bg_color {};\n\
             @define-color accent_fg_color {};\n\
             @define-color window_bg_color {};\n\
             @define-color window_fg_color {};\n\
             @define-color view_bg_color {};\n\
             @define-color view_fg_color {};\n\
             @define-color card_bg_color {};\n\
             @define-color card_fg_color {};\n",
            tokens.primary,
            tokens.primary,
            tokens.on_primary,
            tokens.background,
            tokens.on_background,
            tokens.surface,
            tokens.on_surface,
            tokens.surface,
            tokens.on_surface,
        );

        for ver in &["gtk-3.0", "gtk-4.0"] {
            let dir = home.join(".config").join(ver);
            if dir.exists() {
                let _ = fs::write(dir.join("gtk.css"), &gtk_css);
            }
        }
        Ok(())
    }

    pub fn apply_to_msui(tokens: &SemanticTokens) -> anyhow::Result<()> {
        let mut cfg = MsuiConfig::load_or_default();
        cfg.gui.background = tokens.background.clone();
        cfg.gui.border_color = tokens.primary.clone();
        cfg.color.scheme = match tokens.mode {
            ColorSchemeMode::Light => "light".to_string(),
            ColorSchemeMode::Dark => "dark".to_string(),
        };
        let _ = cfg.save_to_default_path();

        // Notify daemon asynchronously via IPC ReloadConfig in a background thread
        // to avoid nested runtime panics or UI thread blocking
        std::thread::spawn(move || {
            let _ = IpcClient::send_blocking(&IpcCommand::ReloadConfig);
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::presets::ColorPreset;
    use crate::color::semantic::ColorSchemeMode;

    #[test]
    fn test_exporter_generates_valid_files() {
        let preset = ColorPreset::find("midnight").unwrap();
        let tokens = preset.to_tokens(ColorSchemeMode::Dark);

        let temp_dir = std::env::temp_dir().join("msui_exporter_test");
        let _ = fs::create_dir_all(&temp_dir);

        assert!(ThemeExporter::export_hyprland(&temp_dir, &tokens).is_ok());
        let hypr_conf = temp_dir.join(".config").join("hypr").join("generated").join("colors.conf");
        assert!(hypr_conf.exists());
        let content = fs::read_to_string(&hypr_conf).unwrap();
        assert!(content.contains("$primary = rgb("));
        assert!(content.contains("$border-active-solid = $primary"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
