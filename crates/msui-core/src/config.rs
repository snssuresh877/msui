use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MsuiConfig {
    #[serde(default)]
    pub bar: BarConfig,
    #[serde(default)]
    pub launcher: LauncherConfig,
    #[serde(default)]
    pub power: PowerConfig,
    #[serde(default)]
    pub gui: GuiConfig,
    #[serde(default)]
    pub color: ColorConfig,
    #[serde(default)]
    pub notification: NotificationConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    #[serde(default = "default_false")]
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
    #[serde(default = "default_false")]
    pub show_settings: bool,
    #[serde(default = "default_true")]
    pub show_terminal: bool,
    #[serde(default = "default_true")]
    pub show_power: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
fn default_false() -> bool { false }

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerConfig {
    #[serde(default = "default_true")]
    pub auto_adaptive: bool,
    #[serde(default = "default_true")]
    pub heavy_workload_boost: bool,
    #[serde(default = "default_true")]
    pub low_battery_notify: bool,
}

impl Default for PowerConfig {
    fn default() -> Self {
        Self {
            auto_adaptive: true,
            heavy_workload_boost: true,
            low_battery_notify: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuiConfig {
    /// Background color hex code (e.g. "#121218" or "121218")
    #[serde(default = "default_gui_background")]
    pub background: String,
    /// Background opacity when frosted/acrylic mode is enabled (0.0 - 1.0, default 0.94)
    #[serde(default = "default_gui_bg_opacity")]
    pub bg_opacity: f32,
    /// Master border switch: true = show border, false = border off!
    #[serde(default = "default_true")]
    pub border_enabled: bool,
    /// Border accent color hex code (e.g. "#9333ea" or "9333ea")
    #[serde(default = "default_gui_border_color")]
    pub border_color: String,
    /// Border opacity (0.0 - 1.0, default 0.45)
    #[serde(default = "default_gui_border_opacity")]
    pub border_opacity: f32,
    /// Border thickness in pixels (default 1.5)
    #[serde(default = "default_gui_border_width")]
    pub border_width: f32,
    /// Window corner radius in pixels (default 18.0)
    #[serde(default = "default_gui_corner_radius")]
    pub corner_radius: f32,
    /// Subcard glass background opacity (default 0.05)
    #[serde(default = "default_gui_subcard_opacity")]
    pub subcard_opacity: f32,
    /// Subcard subtle border toggle (default: true)
    #[serde(default = "default_true")]
    pub subcard_border_enabled: bool,
    /// UI typography font family (default: "JetBrainsMono Nerd Font")
    #[serde(default = "default_gui_font_family")]
    pub font_family: String,
    /// Window animation speed preset: "fast", "default", "smooth" (default: "default")
    #[serde(default = "default_gui_animation_speed")]
    pub animation_speed: String,
    /// Weather temperature unit: "celsius" or "fahrenheit" (default: "celsius")
    #[serde(default = "default_gui_weather_unit")]
    pub weather_unit: String,
}

fn default_gui_background() -> String { "#121218".into() }
fn default_gui_bg_opacity() -> f32 { 0.94 }
fn default_gui_border_color() -> String { "#9333ea".into() }
fn default_gui_border_opacity() -> f32 { 0.45 }
fn default_gui_border_width() -> f32 { 1.5 }
fn default_gui_corner_radius() -> f32 { 18.0 }
fn default_gui_subcard_opacity() -> f32 { 0.05 }
fn default_gui_font_family() -> String { "JetBrainsMono Nerd Font".into() }
fn default_gui_animation_speed() -> String { "default".into() }
fn default_gui_weather_unit() -> String { "celsius".into() }

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            background: default_gui_background(),
            bg_opacity: default_gui_bg_opacity(),
            border_enabled: true,
            border_color: default_gui_border_color(),
            border_opacity: default_gui_border_opacity(),
            border_width: default_gui_border_width(),
            corner_radius: default_gui_corner_radius(),
            subcard_opacity: default_gui_subcard_opacity(),
            subcard_border_enabled: true,
            font_family: default_gui_font_family(),
            animation_speed: default_gui_animation_speed(),
            weather_unit: default_gui_weather_unit(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Notification banner auto-dismiss timeout in seconds (default: 5)
    #[serde(default = "default_notif_timeout")]
    pub timeout_seconds: u32,
}

fn default_notif_timeout() -> u32 { 5 }

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: default_notif_timeout(),
        }
    }
}

/// Parse a hex color string like "#121218" or "121218" into normalized RGB components in [0.0, 1.0].
pub fn parse_hex_color(hex: &str) -> Option<(f32, f32, f32)> {
    let s = hex.trim().trim_start_matches('#');
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some((r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0))
    } else if s.len() == 3 {
        let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
        let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
        let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
        Some((r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0))
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorConfig {
    #[serde(default = "default_color_mode")]
    pub mode: String,
    #[serde(default = "default_color_scheme")]
    pub scheme: String,
    #[serde(default = "default_active_preset")]
    pub active_preset: String,
    #[serde(default)]
    pub accent_override: String,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            mode: default_color_mode(),
            scheme: default_color_scheme(),
            active_preset: default_active_preset(),
            accent_override: String::new(),
        }
    }
}

fn default_color_mode() -> String { "dynamic".to_string() }
fn default_color_scheme() -> String { "dark".to_string() }
fn default_active_preset() -> String { "midnight".to_string() }

impl Default for MsuiConfig {
    fn default() -> Self {
        Self {
            bar: BarConfig::default(),
            launcher: LauncherConfig::default(),
            power: PowerConfig::default(),
            gui: GuiConfig::default(),
            color: ColorConfig::default(),
            notification: NotificationConfig::default(),
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
            show_theme: false,
            show_reminders_btn: true,
            show_screenshot: true,
            show_clipboard: true,
            show_notifications: true,
            show_control_center: true,
            show_settings: false,
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

    #[test]
    fn test_gui_config_defaults() {
        let cfg = MsuiConfig::default();
        assert_eq!(cfg.gui.background, "#121218");
        assert_eq!(cfg.gui.bg_opacity, 0.94);
        assert!(cfg.gui.border_enabled);
        assert_eq!(cfg.gui.border_color, "#9333ea");
        assert_eq!(cfg.gui.border_opacity, 0.45);
        assert_eq!(cfg.gui.border_width, 1.5);
        assert_eq!(cfg.gui.corner_radius, 18.0);
        assert_eq!(cfg.gui.subcard_opacity, 0.05);
        assert!(cfg.gui.subcard_border_enabled);
    }

    #[test]
    fn test_parse_hex_color() {
        // 6-digit hex
        let (r, g, b) = parse_hex_color("#9333ea").unwrap();
        assert!((r - 147.0 / 255.0).abs() < 0.001);
        assert!((g - 51.0 / 255.0).abs() < 0.001);
        assert!((b - 234.0 / 255.0).abs() < 0.001);

        // Without hash
        let (r, g, b) = parse_hex_color("121218").unwrap();
        assert!((r - 18.0 / 255.0).abs() < 0.001);
        assert!((g - 18.0 / 255.0).abs() < 0.001);
        assert!((b - 24.0 / 255.0).abs() < 0.001);

        // 3-digit shorthand
        let (r, g, b) = parse_hex_color("#fff").unwrap();
        assert_eq!((r, g, b), (1.0, 1.0, 1.0));

        // Invalid
        assert!(parse_hex_color("not_a_hex").is_none());
    }

    #[test]
    fn test_gui_config_deserialize() {
        let toml_str = r##"
        [gui]
        background = "#0f172a"
        border_enabled = false
        border_color = "#06b6d4"
        border_width = 2.0
        corner_radius = 12.0
        "##;
        let cfg: MsuiConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.gui.background, "#0f172a");
        assert!(!cfg.gui.border_enabled);
        assert_eq!(cfg.gui.border_color, "#06b6d4");
        assert_eq!(cfg.gui.border_width, 2.0);
        assert_eq!(cfg.gui.corner_radius, 12.0);
        assert_eq!(cfg.gui.bg_opacity, 0.94); // default preserved
    }
}
