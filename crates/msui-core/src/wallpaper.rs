use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WallpaperItem {
    pub name: String,
    pub path: PathBuf,
    pub size_mb: f32,
}

pub struct WallpaperManager;

impl WallpaperManager {
    pub fn wallpaper_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            let home_p = PathBuf::from(home);
            dirs.push(home_p.join("Pictures").join("Wallpapers"));
            dirs.push(home_p.join("Pictures").join("wallpapers"));
            dirs.push(home_p.join("Pictures"));
            dirs.push(home_p.join(".config").join("hypr").join("wallpapers"));
        }
        dirs.push(PathBuf::from("/usr/share/backgrounds"));
        dirs
    }

    pub fn list() -> Vec<WallpaperItem> {
        let mut items = Vec::new();
        let valid_extensions = ["jpg", "jpeg", "png", "webp", "avif", "bmp"];

        for dir in Self::wallpaper_dirs() {
            if !dir.exists() || !dir.is_dir() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                            if valid_extensions.contains(&ext.to_lowercase().as_str()) {
                                let name = path
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("Unknown")
                                    .to_string();

                                let size_mb = entry
                                    .metadata()
                                    .map(|m| m.len() as f32 / (1024.0 * 1024.0))
                                    .unwrap_or(0.0);

                                items.push(WallpaperItem {
                                    name,
                                    path,
                                    size_mb,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Sort alphabetically by name
        items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        items.dedup_by(|a, b| a.path == b.path);
        items
    }

    pub fn cache_file_path() -> Option<PathBuf> {
        if let Ok(cache) = std::env::var("XDG_CACHE_HOME") {
            return Some(PathBuf::from(cache).join("msui").join("current_wallpaper.txt"));
        }
        if let Ok(home) = std::env::var("HOME") {
            return Some(PathBuf::from(home).join(".cache").join("msui").join("current_wallpaper.txt"));
        }
        None
    }

    pub fn get_current() -> Option<PathBuf> {
        if let Some(cache_path) = Self::cache_file_path() {
            if cache_path.exists() {
                if let Ok(content) = fs::read_to_string(&cache_path) {
                    let trimmed = content.trim();
                    let p = PathBuf::from(trimmed);
                    if p.exists() {
                        return Some(p);
                    }
                }
            }
        }

        // Fallback to first available wallpaper
        let all = Self::list();
        if let Some(first) = all.first() {
            Self::set(&first.path);
            return Some(first.path.clone());
        }

        None
    }

    pub fn set(path: &Path) -> bool {
        if !path.exists() {
            return false;
        }

        if let Some(cache_path) = Self::cache_file_path() {
            if let Some(parent) = cache_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if fs::write(&cache_path, path.to_string_lossy().to_string()).is_ok() {
                tracing::info!("Updated active wallpaper to {:?}", path);
                return true;
            }
        }
        false
    }

    pub fn cycle() -> Option<PathBuf> {
        let all = Self::list();
        if all.is_empty() {
            return None;
        }

        let current = Self::get_current();
        let current_idx = current
            .and_then(|cp| all.iter().position(|w| w.path == cp))
            .unwrap_or(0);

        let next_idx = (current_idx + 1) % all.len();
        let next_wallpaper = &all[next_idx];
        Self::set(&next_wallpaper.path);
        Some(next_wallpaper.path.clone())
    }

    pub fn random() -> Option<PathBuf> {
        let all = Self::list();
        if all.is_empty() {
            return None;
        }

        use std::time::SystemTime;
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as usize)
            .unwrap_or(0);

        let rand_idx = nanos % all.len();
        let picked = &all[rand_idx];
        Self::set(&picked.path);
        Some(picked.path.clone())
    }
}
