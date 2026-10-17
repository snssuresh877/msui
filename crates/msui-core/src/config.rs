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
    #[serde(default = "default_time_format")]
    pub time_format: String,
    #[serde(default = "default_battery_refresh")]
    pub battery_refresh_seconds: u64,
    #[serde(default = "default_sysinfo_refresh")]
    pub sysinfo_refresh_seconds: u64,
    #[serde(default = "default_true")]
    pub show_workspaces: bool,
    #[serde(default = "default_true")]
    pub show_battery: bool,
    #[serde(default = "default_true")]
    pub show_cpu: bool,
    #[serde(default = "default_true")]
    pub show_memory: bool,
    #[serde(default = "default_true")]
    pub show_audio: bool,
    #[serde(default = "default_true")]
    pub show_network: bool,
    #[serde(default = "default_true")]
    pub show_brightness: bool,
    #[serde(default = "default_true")]
    pub show_media: bool,
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

fn default_bar_height() -> u32 { 32 }
fn default_bar_layer() -> String { "top".into() }
fn default_bar_anchor() -> String { "top".into() }
fn default_time_format() -> String { "%a %b %d  %I:%M %p".into() }
fn default_battery_refresh() -> u64 { 5 }
fn default_sysinfo_refresh() -> u64 { 2 }
fn default_true() -> bool { true }

fn default_launcher_width() -> u32 { 680 }
fn default_launcher_height() -> u32 { 420 }
fn default_max_results() -> usize { 8 }
fn default_launcher_layer() -> String { "overlay".into() }
fn default_placeholder() -> String { "Search applications...".into() }
fn default_terminal() -> String { "alacritty".into() }

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
            time_format: default_time_format(),
            battery_refresh_seconds: default_battery_refresh(),
            sysinfo_refresh_seconds: default_sysinfo_refresh(),
            show_workspaces: true,
            show_battery: true,
            show_cpu: true,
            show_memory: true,
            show_audio: true,
            show_network: true,
            show_brightness: true,
            show_media: true,
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
}
