use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsuiConfig {
    #[serde(default)]
    pub bar: BarConfig,
    #[serde(default)]
    pub launcher: LauncherConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarConfig {
    #[serde(default = "default_bar_height")]
    pub height: u32,
    #[serde(default = "default_bar_layer")]
    pub layer: String,
    #[serde(default = "default_bar_anchor")]
    pub anchor: String,
    #[serde(default = "default_bar_style")]
    pub style: String,
    #[serde(default = "default_bar_transparency")]
    pub transparency: String,
    #[serde(default = "default_margin_top")]
    pub margin_top: u32,
    #[serde(default = "default_margin_horizontal")]
    pub margin_horizontal: u32,
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f32,
    #[serde(default = "default_border_width")]
    pub border_width: f32,
    #[serde(default = "default_time_format")]
    pub time_format: String,
    #[serde(default = "default_battery_refresh")]
    pub battery_refresh_seconds: u64,
    #[serde(default = "default_sysinfo_refresh")]
    pub sysinfo_refresh_seconds: u64,
    #[serde(default = "default_true")]
    pub show_brand: bool,
    #[serde(default = "default_true")]
    pub show_workspaces: bool,
    #[serde(default = "default_workspace_count")]
    pub workspace_count: usize,
    #[serde(default = "default_true")]
    pub show_active_window: bool,
    #[serde(default = "default_true")]
    pub show_media: bool,
    #[serde(default = "default_true")]
    pub show_clock: bool,
    #[serde(default = "default_true")]
    pub show_reminders_badge: bool,
    #[serde(default = "default_true")]
    pub show_network: bool,
    #[serde(default = "default_true")]
    pub show_audio: bool,
    #[serde(default = "default_true")]
    pub show_brightness: bool,
    #[serde(default = "default_true")]
    pub show_cpu: bool,
    #[serde(default = "default_true")]
    pub show_memory: bool,
    #[serde(default = "default_true")]
    pub show_storage: bool,
    #[serde(default = "default_true")]
    pub show_battery: bool,
    #[serde(default = "default_true")]
    pub show_bluetooth: bool,
    #[serde(default = "default_true")]
    pub show_wallpaper: bool,
    #[serde(default = "default_true")]
    pub show_theme: bool,
    #[serde(default = "default_true")]
    pub show_reminders_btn: bool,
    #[serde(default = "default_true")]
    pub show_screenshot: bool,
    #[serde(default = "default_true")]
    pub show_clipboard: bool,
    #[serde(default = "default_true")]
    pub show_notifications: bool,
    #[serde(default = "default_true")]
    pub show_control_center: bool,
    #[serde(default = "default_true")]
    pub show_terminal: bool,
    #[serde(default = "default_true")]
    pub show_power: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    #[serde(default = "default_launcher_width")]
    pub width: u32,
    #[serde(default = "default_launcher_height")]
    pub height: u32,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default = "default_launcher_layer")]
    pub layer: String,
    #[serde(default = "default_placeholder")]
    pub placeholder: String,
    #[serde(default = "default_terminal")]
    pub terminal: String,
}

impl BarConfig {
    pub fn is_modern(&self) -> bool {
        let s = self.style.to_lowercase();
        s == "modern" || s == "floating" || s == "island"
    }

    pub fn is_transparent(&self) -> bool {
        matches!(self.transparency.to_lowercase().as_str(), "transparent" | "clear" | "none")
    }

    pub fn is_solid(&self) -> bool {
        matches!(self.transparency.to_lowercase().as_str(), "solid" | "opaque")
    }

    pub fn is_frosted(&self) -> bool {
        !self.is_transparent() && !self.is_solid()
    }
}

fn default_bar_height() -> u32 { 32 }
fn default_bar_layer() -> String { "top".into() }
fn default_bar_anchor() -> String { "top".into() }
fn default_bar_style() -> String { "modern".into() }
fn default_bar_transparency() -> String { "frosted".into() }
fn default_margin_top() -> u32 { 6 }
fn default_margin_horizontal() -> u32 { 12 }
fn default_corner_radius() -> f32 { 14.0 }
fn default_border_width() -> f32 { 1.0 }
fn default_time_format() -> String { "%a %b %d  %I:%M %p".into() }
fn default_battery_refresh() -> u64 { 5 }
fn default_sysinfo_refresh() -> u64 { 2 }
fn default_workspace_count() -> usize { 3 }
fn default_true() -> bool { true }

fn default_launcher_width() -> u32 { 680 }
fn default_launcher_height() -> u32 { 420 }
fn default_max_results() -> usize { 8 }
fn default_launcher_layer() -> String { "overlay".into() }
fn default_placeholder() -> String { "Search applications...".into() }
fn default_terminal() -> String {
    for term in ["kitty", "foot", "alacritty", "ghostty", "wezterm", "xterm", "gnome-terminal", "konsole"] {
        if let Ok(child) = std::process::Command::new("which").arg(term).output() {
            if child.status.success() {
                return term.to_string();
            }
        }
    }
    "alacritty".into()
}

impl Default for MsuiConfig {
    fn default() -> Self {
        Self {
            bar: BarConfig::default(),
            launcher: LauncherConfig::default(),
        }
    }
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            height: default_bar_height(),
            layer: default_bar_layer(),
            anchor: default_bar_anchor(),
            style: default_bar_style(),
            transparency: default_bar_transparency(),
            margin_top: default_margin_top(),
            margin_horizontal: default_margin_horizontal(),
            corner_radius: default_corner_radius(),
            border_width: default_border_width(),
            time_format: default_time_format(),
            battery_refresh_seconds: default_battery_refresh(),
            sysinfo_refresh_seconds: default_sysinfo_refresh(),
            show_brand: true,
            show_workspaces: true,
            workspace_count: default_workspace_count(),
            show_active_window: true,
            show_media: true,
            show_clock: true,
            show_reminders_badge: true,
            show_network: true,
            show_audio: true,
            show_brightness: true,
            show_cpu: true,
            show_memory: true,
            show_storage: true,
            show_battery: true,
            show_bluetooth: true,
            show_wallpaper: true,
            show_theme: true,
            show_reminders_btn: true,
            show_screenshot: true,
            show_clipboard: true,
            show_notifications: true,
            show_control_center: true,
            show_terminal: true,
            show_power: true,
        }
    }
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            width: default_launcher_width(),
            height: default_launcher_height(),
            max_results: default_max_results(),
            layer: default_launcher_layer(),
            placeholder: default_placeholder(),
            terminal: default_terminal(),
        }
    }
}

impl MsuiConfig {
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn load_or_default() -> Self {
        if let Some(config_path) = Self::default_config_path() {
            if config_path.exists() {
                if let Ok(cfg) = Self::load_from_path(&config_path) {
                    tracing::info!("Loaded config from {}", config_path.display());
                    return cfg;
                }
            }
        }
        tracing::warn!("Using default msUI configuration");
        Self::default()
    }

    pub fn default_config_path() -> Option<PathBuf> {
        if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
            let path = PathBuf::from(config_home).join("msui").join("msui.toml");
            return Some(path);
        }
        if let Ok(home) = std::env::var("HOME") {
            let path = PathBuf::from(home).join(".config").join("msui").join("msui.toml");
            return Some(path);
        }
        None
    }

    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn save_to_default_path(&self) -> Result<()> {
        let path = Self::default_config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine default config path"))?;
        self.save_to_path(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar_config_defaults() {
        let bar = BarConfig::default();
        assert_eq!(bar.style, "modern");
        assert!(bar.is_modern());
        assert_eq!(bar.margin_top, 6);
        assert_eq!(bar.margin_horizontal, 12);
        assert_eq!(bar.corner_radius, 14.0);
        assert_eq!(bar.border_width, 1.0);
        assert_eq!(bar.workspace_count, 3);
        assert_eq!(bar.transparency, "frosted");
        assert!(bar.is_frosted());
    }

    #[test]
    fn test_bar_config_transparency() {
        let mut bar = BarConfig::default();
        assert!(bar.is_frosted());
        assert!(!bar.is_transparent());
        assert!(!bar.is_solid());

        bar.transparency = "transparent".into();
        assert!(bar.is_transparent());
        assert!(!bar.is_frosted());
        assert!(!bar.is_solid());

        bar.transparency = "solid".into();
        assert!(bar.is_solid());
        assert!(!bar.is_frosted());
        assert!(!bar.is_transparent());
    }

    #[test]
    fn test_bar_config_is_modern() {
        let mut bar = BarConfig::default();
        bar.style = "classic".into();
        assert!(!bar.is_modern());
        bar.style = "docked".into();
        assert!(!bar.is_modern());
        bar.style = "floating".into();
        assert!(bar.is_modern());
        bar.style = "island".into();
        assert!(bar.is_modern());
        bar.style = "modern".into();
        assert!(bar.is_modern());
    }

    #[test]
    fn test_config_deserialize_toml() {
        let toml_str = r#"
        [bar]
        style = "classic"
        margin_top = 0
        "#;
        let cfg: MsuiConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.bar.style, "classic");
        assert!(!cfg.bar.is_modern());
        assert_eq!(cfg.bar.margin_top, 0);
        assert_eq!(cfg.bar.margin_horizontal, 12);
    }
}
