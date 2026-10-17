pub mod watcher;

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, LazyLock, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClipboardEntry {
    pub id: usize,
    pub text: String,
    pub preview: String,
    pub char_count: usize,
    pub timestamp: String,
}

pub struct ClipboardStore {
    next_id: usize,
    entries: Vec<ClipboardEntry>,
    cache_path: Option<PathBuf>,
}

pub static GLOBAL_CLIPBOARD: LazyLock<Arc<Mutex<ClipboardStore>>> = LazyLock::new(|| {
    let mut store = ClipboardStore::new();
    store.load();
    Arc::new(Mutex::new(store))
});

impl ClipboardStore {
    pub fn new() -> Self {
        let cache_path = dirs_cache_path();
        Self {
            next_id: 1,
            entries: Vec::with_capacity(100),
            cache_path,
        }
    }

    pub fn new_in_memory() -> Self {
        Self {
            next_id: 1,
            entries: Vec::with_capacity(100),
            cache_path: None,
        }
    }

    pub fn add(&mut self, text: String) -> bool {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return false;
        }

        // Check if identical to the most recent entry
        if let Some(first) = self.entries.first() {
            if first.text == text {
                return false;
            }
        }

        // Remove if previously existed elsewhere in history (move to top)
        self.entries.retain(|e| e.text != text);

        let preview = if trimmed.len() > 80 {
            let mut s: String = trimmed.chars().take(77).collect();
            s.push_str("...");
            s.replace('\n', " ")
        } else {
            trimmed.replace('\n', " ")
        };

        let entry = ClipboardEntry {
            id: self.next_id,
            char_count: text.chars().count(),
            timestamp: Local::now().format("%I:%M %p").to_string(),
            preview,
            text,
        };

        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.entries.insert(0, entry);

        if self.entries.len() > 100 {
            self.entries.truncate(100);
        }

        self.save();
        true
    }

    pub fn delete(&mut self, id: usize) {
        self.entries.retain(|e| e.id != id);
        self.save();
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.save();
    }

    pub fn entries(&self) -> Vec<ClipboardEntry> {
        self.entries.clone()
    }

    pub fn copy_to_system(text: &str) {
        use std::io::Write;
        if let Ok(mut child) = Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
        }
    }

    pub fn read_system() -> Option<String> {
        let out = Command::new("wl-paste")
            .arg("-n")
            .output()
            .ok()?;
        if out.status.success() {
            let text = String::from_utf8(out.stdout).ok()?;
            Some(text)
        } else {
            None
        }
    }

    fn load(&mut self) {
        let Some(path) = &self.cache_path else { return; };
        if path.exists() {
            if let Ok(data) = fs::read_to_string(path) {
                if let Ok(loaded) = serde_json::from_str::<Vec<ClipboardEntry>>(&data) {
                    self.next_id = loaded.iter().map(|e| e.id).max().unwrap_or(0) + 1;
                    self.entries = loaded;
                }
            }
        }
    }

    fn save(&self) {
        let Some(path) = &self.cache_path else { return; };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.entries) {
            let _ = fs::write(path, json);
        }
    }
}

fn dirs_cache_path() -> Option<PathBuf> {
    if let Ok(cache) = std::env::var("XDG_CACHE_HOME") {
        return Some(PathBuf::from(cache).join("msui").join("clipboard_history.json"));
    }
    if let Ok(home) = std::env::var("HOME") {
        return Some(PathBuf::from(home).join(".cache").join("msui").join("clipboard_history.json"));
    }
    None
}
