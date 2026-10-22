use std::path::{Path, PathBuf};
use std::process::Command;

pub struct FileOperations;

impl FileOperations {
    /// Open a file or directory with system default application (xdg-open)
    pub fn open_default(path: &Path) -> std::io::Result<()> {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(|_| ())
    }

    /// Open a terminal emulator in the specified directory
    pub fn open_terminal(dir: &Path) -> std::io::Result<()> {
        let terminals = [
            ("ghostty", vec!["--working-directory"]),
            ("foot", vec!["-D"]),
            ("kitty", vec!["--directory"]),
            ("alacritty", vec!["--working-directory"]),
            ("wezterm", vec!["start", "--cwd"]),
            ("xterm", vec![]),
        ];

        if let Ok(term) = std::env::var("TERMINAL") {
            if !term.is_empty() {
                if let Ok(child) = Command::new(&term).current_dir(dir).spawn() {
                    let _ = child.id();
                    return Ok(());
                }
            }
        }

        for (term, args) in terminals {
            if is_command_available(term) {
                let mut cmd = Command::new(term);
                for arg in args {
                    cmd.arg(arg);
                }
                cmd.arg(dir);
                cmd.current_dir(dir);
                if let Ok(child) = cmd.spawn() {
                    let _ = child.id();
                    return Ok(());
                }
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No compatible terminal emulator found",
        ))
    }

    /// Create a new subdirectory in parent
    pub fn create_directory(parent: &Path, name: &str) -> std::io::Result<PathBuf> {
        let clean_name = name.trim();
        if clean_name.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Folder name cannot be empty",
            ));
        }
        let target = parent.join(clean_name);
        std::fs::create_dir(&target)?;
        Ok(target)
    }

    /// Create an empty file in parent
    pub fn create_empty_file(parent: &Path, name: &str) -> std::io::Result<PathBuf> {
        let clean_name = name.trim();
        if clean_name.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "File name cannot be empty",
            ));
        }
        let target = parent.join(clean_name);
        std::fs::File::create(&target)?;
        Ok(target)
    }

    /// Rename an item
    pub fn rename_item(old_path: &Path, new_name: &str) -> std::io::Result<PathBuf> {
        let clean_name = new_name.trim();
        if clean_name.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "New name cannot be empty",
            ));
        }
        let parent = old_path.parent().unwrap_or(Path::new("/"));
        let target = parent.join(clean_name);
        std::fs::rename(old_path, &target)?;
        Ok(target)
    }

    /// Move file to trash using gio trash or FreeDesktop trash directory
    pub fn delete_or_trash(path: &Path) -> std::io::Result<()> {
        if is_command_available("gio") {
            let status = Command::new("gio")
                .arg("trash")
                .arg(path)
                .status();
            if let Ok(s) = status {
                if s.success() {
                    return Ok(());
                }
            }
        }

        // Fallback to standard FreeDesktop Trash
        if let Ok(home) = std::env::var("HOME") {
            let trash_files = PathBuf::from(home).join(".local/share/Trash/files");
            if std::fs::create_dir_all(&trash_files).is_ok() {
                if let Some(file_name) = path.file_name() {
                    let dest = trash_files.join(file_name);
                    if std::fs::rename(path, dest).is_ok() {
                        return Ok(());
                    }
                }
            }
        }

        // Final fallback: permanent removal
        if path.is_dir() {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_file(path)
        }
    }

    /// Copy path string to system clipboard (using wl-copy)
    pub fn copy_path_to_clipboard(path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_string();
        if is_command_available("wl-copy") {
            use std::io::Write;
            if let Ok(mut child) = Command::new("wl-copy")
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(path_str.as_bytes());
                }
                return child.wait().map(|s| s.success()).unwrap_or(false);
            }
        }
        false
    }
}

fn is_command_available(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
