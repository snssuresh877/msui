use chrono::{DateTime, Local};
use iced::Color;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileCategory {
    Directory,
    Code,
    Archive,
    Image,
    Audio,
    Video,
    Document,
    Text,
    Executable,
    Symlink,
    Other,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub is_hidden: bool,
    pub size_bytes: u64,
    pub human_size: String,
    pub modified_time: SystemTime,
    pub human_date: String,
    pub permissions_mode: u32,
    pub human_permissions: String,
    pub extension: String,
    pub category: FileCategory,
    pub icon_glyph: &'static str,
    pub accent_color: Color,
}

impl FileEntry {
    pub fn from_path(path: &Path, meta: Option<&Metadata>) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().to_string();
        let is_hidden = name.starts_with('.');

        let (size_bytes, modified_time, permissions_mode, is_dir, is_symlink) = match meta {
            Some(m) => {
                let is_symlink = m.file_type().is_symlink();
                let is_dir = m.is_dir();
                let size = if is_dir { 0 } else { m.len() };
                let modified = m.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                let mode = m.mode();
                (size, modified, mode, is_dir, is_symlink)
            }
            None => {
                let m = std::fs::symlink_metadata(path).ok()?;
                let is_symlink = m.file_type().is_symlink();
                let is_dir = m.is_dir();
                let size = if is_dir { 0 } else { m.len() };
                let modified = m.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                let mode = m.mode();
                (size, modified, mode, is_dir, is_symlink)
            }
        };

        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let human_size = if is_dir {
            format_folder_size(path)
        } else {
            format_size(size_bytes)
        };

        let dt: DateTime<Local> = modified_time.into();
        let human_date = dt.format("%Y-%m-%d %H:%M").to_string();
        let human_permissions = format_permissions(permissions_mode, is_dir);

        let (category, icon_glyph, accent_color) =
            determine_category(&name, is_dir, is_symlink, permissions_mode, &extension);

        Some(Self {
            name,
            path: path.to_path_buf(),
            is_dir,
            is_symlink,
            is_hidden,
            size_bytes,
            human_size,
            modified_time,
            human_date,
            permissions_mode,
            human_permissions,
            extension,
            category,
            icon_glyph,
            accent_color,
        })
    }
}

fn format_folder_size(path: &Path) -> String {
    // Return count of items directly in directory if readable, or "Directory"
    if let Ok(entries) = std::fs::read_dir(path) {
        let count = entries.filter_map(Result::ok).count();
        format!("{count} items")
    } else {
        "folder".into()
    }
}

pub fn format_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;
    const TIB: u64 = GIB * 1024;

    if bytes >= TIB {
        format!("{:.1} TiB", bytes as f64 / TIB as f64)
    } else if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn format_permissions(mode: u32, is_dir: bool) -> String {
    let d = if is_dir { 'd' } else { '-' };
    let r1 = if mode & 0o400 != 0 { 'r' } else { '-' };
    let w1 = if mode & 0o200 != 0 { 'w' } else { '-' };
    let x1 = if mode & 0o100 != 0 { 'x' } else { '-' };
    let r2 = if mode & 0o040 != 0 { 'r' } else { '-' };
    let w2 = if mode & 0o020 != 0 { 'w' } else { '-' };
    let x2 = if mode & 0o010 != 0 { 'x' } else { '-' };
    let r3 = if mode & 0o004 != 0 { 'r' } else { '-' };
    let w3 = if mode & 0o002 != 0 { 'w' } else { '-' };
    let x3 = if mode & 0o001 != 0 { 'x' } else { '-' };

    format!("{d}{r1}{w1}{x1}{r2}{w2}{x2}{r3}{w3}{x3}")
}

fn determine_category(
    name: &str,
    is_dir: bool,
    is_symlink: bool,
    mode: u32,
    ext: &str,
) -> (FileCategory, &'static str, Color) {
    // Catppuccin Mocha Palette
    let blue = Color::from_rgb8(137, 180, 250); // #89b4fa
    let sapphire = Color::from_rgb8(116, 199, 236); // #74c7ec
    let teal = Color::from_rgb8(148, 226, 213); // #94e2d5
    let green = Color::from_rgb8(166, 227, 161); // #a6e3a1
    let yellow = Color::from_rgb8(249, 226, 175); // #f9e2af
    let peach = Color::from_rgb8(250, 179, 135); // #fab387
    let maroon = Color::from_rgb8(235, 160, 172); // #eba0ac
    let red = Color::from_rgb8(243, 139, 168); // #f38ba8
    let mauve = Color::from_rgb8(203, 166, 247); // #cba6f7
    let pink = Color::from_rgb8(245, 194, 231); // #f5c2e7
    let subtext = Color::from_rgb8(166, 173, 200); // #a6adc8

    if is_symlink {
        return (FileCategory::Symlink, "󰌷 ", teal);
    }

    if is_dir {
        match name {
            ".git" => return (FileCategory::Directory, "󰊢 ", red),
            ".github" => return (FileCategory::Directory, "󰊤 ", mauve),
            ".config" => return (FileCategory::Directory, "󱁿 ", mauve),
            ".cargo" | "cargo" => return (FileCategory::Directory, "󱘗 ", peach),
            "Downloads" => return (FileCategory::Directory, "󰉍 ", sapphire),
            "Documents" => return (FileCategory::Directory, "󰈙 ", blue),
            "Pictures" => return (FileCategory::Directory, "󰋩 ", yellow),
            "Music" => return (FileCategory::Directory, "󰝚 ", pink),
            "Videos" => return (FileCategory::Directory, "󰿎 ", maroon),
            "Desktop" => return (FileCategory::Directory, "󰇄 ", blue),
            "Projects" | "code" | "dev" | "src" => {
                return (FileCategory::Directory, "󰅩 ", peach);
            }
            _ => return (FileCategory::Directory, "󰉋 ", blue),
        }
    }

    match ext {
        "rs" => (FileCategory::Code, "󱘗 ", peach),
        "py" | "pyw" => (FileCategory::Code, "󰌠 ", yellow),
        "js" | "mjs" | "cjs" => (FileCategory::Code, "󰌞 ", yellow),
        "ts" | "mts" | "cts" => (FileCategory::Code, "󰛦 ", blue),
        "jsx" | "tsx" => (FileCategory::Code, "󰌞 ", sapphire),
        "c" | "h" => (FileCategory::Code, "󰙲 ", sapphire),
        "cpp" | "cxx" | "cc" | "hpp" => (FileCategory::Code, "󰙲 ", blue),
        "go" => (FileCategory::Code, "󰟓 ", sapphire),
        "java" | "class" | "jar" => (FileCategory::Code, "󰬷 ", red),
        "html" | "htm" => (FileCategory::Code, "󰌝 ", peach),
        "css" | "scss" | "sass" | "less" => (FileCategory::Code, "󰌝 ", sapphire),
        "json" | "toml" | "yaml" | "yml" | "xml" | "ron" => (FileCategory::Code, "󰘦 ", mauve),
        "sh" | "bash" | "zsh" | "fish" => (FileCategory::Code, "󰞷 ", green),
        "lua" => (FileCategory::Code, "󰢱 ", sapphire),
        "sql" => (FileCategory::Code, "󰆼 ", mauve),

        // Archives
        "zip" | "tar" | "gz" | "zst" | "xz" | "7z" | "bz2" | "rar" | "tgz" => {
            (FileCategory::Archive, "󰛫 ", mauve)
        }

        // Images
        "png" | "jpg" | "jpeg" | "webp" | "svg" | "gif" | "bmp" | "ico" | "avif" => {
            (FileCategory::Image, "󰋩 ", yellow)
        }

        // Audio
        "mp3" | "flac" | "wav" | "ogg" | "m4a" | "aac" | "opus" => {
            (FileCategory::Audio, "󰝚 ", pink)
        }

        // Videos
        "mp4" | "mkv" | "avi" | "webm" | "mov" | "flv" | "wmv" => {
            (FileCategory::Video, "󰿎 ", maroon)
        }

        // Documents
        "pdf" => (FileCategory::Document, "󰈙 ", red),
        "doc" | "docx" | "odt" | "rtf" => (FileCategory::Document, "󰈙 ", blue),
        "xls" | "xlsx" | "ods" => (FileCategory::Document, "󰈙 ", green),
        "ppt" | "pptx" | "odp" => (FileCategory::Document, "󰈙 ", peach),

        // Text
        "txt" | "md" | "markdown" | "log" | "csv" | "tsv" | "ini" | "conf" => {
            (FileCategory::Text, "󰈙 ", subtext)
        }

        _ => {
            if mode & 0o111 != 0 {
                (FileCategory::Executable, "󰅱 ", green)
            } else {
                (FileCategory::Other, "󰈔 ", subtext)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.0 KiB");
        assert_eq!(format_size(1024 * 1024 * 5), "5.0 MiB");
        assert_eq!(format_size(1024 * 1024 * 1024 * 2), "2.0 GiB");
    }

    #[test]
    fn test_file_entry_classification() {
        let _path = Path::new("/home/m/test.rs");
        let (cat, icon, _) = determine_category("test.rs", false, false, 0o644, "rs");
        assert_eq!(cat, FileCategory::Code);
        assert_eq!(icon, "󱘗 ");
    }
}
