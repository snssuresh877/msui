use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub port: u16,
    pub host: String,
    pub context_size: usize,
    pub cpu_threads: usize,
    pub auto_unload_mins: u64,
    pub min_free_ram_mb: u64,
    pub quantize_kv: bool,
    pub flash_attention: bool,
    pub default_model: Option<String>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "127.0.0.1".to_string(),
            context_size: 2048,
            cpu_threads: 4,
            auto_unload_mins: 5,
            min_free_ram_mb: 1200,
            quantize_kv: true,
            flash_attention: true,
            default_model: None,
        }
    }
}

impl AiConfig {
    pub fn config_dir() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".local/share/msui/ai/config")
        } else {
            let uid = unsafe { libc::getuid() };
            PathBuf::from(format!("/tmp/msui-runtime-{}/ai/config", uid))
        }
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("settings.toml")
    }

    pub fn load() -> Self {
        let p = Self::config_path();
        if p.exists() {
            if let Ok(content) = std::fs::read_to_string(&p) {
                if let Ok(cfg) = toml::from_str::<AiConfig>(&content) {
                    return cfg;
                }
            }
        }
        let def = Self::default();
        let _ = def.save();
        def
    }

    pub fn save(&self) -> std::io::Result<()> {
        let p = Self::config_path();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
            }
        }
        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        #[cfg(unix)]
        {
            use std::fs::OpenOptions;
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut options = OpenOptions::new();
            options.write(true).create(true).truncate(true).mode(0o600);
            if let Ok(mut file) = options.open(&p) {
                let _ = file.write_all(toml_str.as_bytes());
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600));
            }
        }
        #[cfg(not(unix))]
        {
            std::fs::write(p, toml_str)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_config_defaults() {
        let cfg = AiConfig::default();
        assert_eq!(cfg.port, 8080);
        assert_eq!(cfg.context_size, 2048);
        assert_eq!(cfg.cpu_threads, 4);
        assert!(cfg.quantize_kv);
        assert!(cfg.flash_attention);
    }
}
