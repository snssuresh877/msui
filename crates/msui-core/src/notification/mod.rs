pub mod service;

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: Option<String>,
    pub summary: String,
    pub body: String,
    pub actions: Vec<String>,
    pub expire_timeout_ms: i32,
    pub timestamp: String,
}

pub struct NotificationStore {
    next_id: u32,
    history: VecDeque<Notification>,
    active_toasts: Vec<Notification>,
    sender: broadcast::Sender<Notification>,
    cache_path: Option<PathBuf>,
    active_path: Option<PathBuf>,
}

pub static GLOBAL_NOTIFICATIONS: LazyLock<Arc<Mutex<NotificationStore>>> = LazyLock::new(|| {
    let mut store = NotificationStore::new();
    store.load();
    Arc::new(Mutex::new(store))
});

impl NotificationStore {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(32);
        let cache_dir = dirs_cache_dir();
        let cache_path = cache_dir.as_ref().map(|d| d.join("notifications_history.json"));
        let active_path = cache_dir.as_ref().map(|d| d.join("active_toasts.json"));

        Self {
            next_id: 1,
            history: VecDeque::with_capacity(100),
            active_toasts: Vec::with_capacity(10),
            sender,
            cache_path,
            active_path,
        }
    }

    pub fn new_in_memory() -> Self {
        let (sender, _) = broadcast::channel(32);
        Self {
            next_id: 1,
            history: VecDeque::with_capacity(100),
            active_toasts: Vec::with_capacity(10),
            sender,
            cache_path: None,
            active_path: None,
        }
    }

    pub fn load(&mut self) {
        if let Some(path) = &self.cache_path {
            if path.exists() {
                if let Ok(data) = fs::read_to_string(path) {
                    if let Ok(loaded) = serde_json::from_str::<Vec<Notification>>(&data) {
                        self.next_id = loaded.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                        self.history = VecDeque::from(loaded);
                    }
                }
            }
        }
    }

    pub fn save(&self) {
        if let Some(path) = &self.cache_path {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let list: Vec<&Notification> = self.history.iter().collect();
            if let Ok(json) = serde_json::to_string_pretty(&list) {
                let _ = fs::write(path, json);
            }
        }
    }

    pub fn add(
        &mut self,
        app_name: String,
        app_icon: Option<String>,
        summary: String,
        body: String,
        actions: Vec<String>,
        expire_timeout_ms: i32,
    ) -> Notification {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);

        let notif = Notification {
            id,
            app_name,
            app_icon,
            summary,
            body,
            actions,
            expire_timeout_ms: if expire_timeout_ms <= 0 { 5000 } else { expire_timeout_ms },
            timestamp: Local::now().format("%I:%M %p").to_string(),
        };

        if self.history.len() >= 100 {
            self.history.pop_front();
        }
        self.history.push_back(notif.clone());

        self.active_toasts.insert(0, notif.clone());
        if self.active_toasts.len() > 8 {
            self.active_toasts.truncate(8);
        }

        self.save();
        self.save_active_toasts();

        let _ = self.sender.send(notif.clone());
        notif
    }

    fn save_active_toasts(&self) {
        if let Some(path) = &self.active_path {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(&self.active_toasts) {
                let _ = fs::write(path, json);
            }
        }
    }

    pub fn delete_history(&mut self, id: u32) {
        self.history.retain(|n| n.id != id);
        self.save();
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
        self.save();
    }

    pub fn history(&self) -> Vec<Notification> {
        self.history.iter().cloned().rev().collect()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Notification> {
        self.sender.subscribe()
    }
}

pub fn dirs_cache_dir() -> Option<PathBuf> {
    if let Ok(cache) = std::env::var("XDG_CACHE_HOME") {
        return Some(PathBuf::from(cache).join("msui"));
    }
    if let Ok(home) = std::env::var("HOME") {
        return Some(PathBuf::from(home).join(".cache").join("msui"));
    }
    None
}

pub fn load_active_toasts() -> Vec<Notification> {
    if let Some(dir) = dirs_cache_dir() {
        let path = dir.join("active_toasts.json");
        if path.exists() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(toasts) = serde_json::from_str::<Vec<Notification>>(&data) {
                    return toasts;
                }
            }
        }
    }
    Vec::new()
}

pub fn dismiss_active_toast(id: u32) {
    if let Some(dir) = dirs_cache_dir() {
        let path = dir.join("active_toasts.json");
        let mut toasts = load_active_toasts();
        toasts.retain(|n| n.id != id);
        if let Ok(json) = serde_json::to_string_pretty(&toasts) {
            let _ = fs::write(&path, json);
        }
    }
}

pub fn clear_active_toasts() {
    if let Some(dir) = dirs_cache_dir() {
        let path = dir.join("active_toasts.json");
        let _ = fs::write(&path, "[]");
    }
}

pub fn is_toast_running() -> bool {
    if let Some(dir) = dirs_cache_dir() {
        let pid_path = dir.join("toast.pid");
        if pid_path.exists() {
            if let Ok(content) = fs::read_to_string(&pid_path) {
                if let Ok(pid) = content.trim().parse::<i32>() {
                    let proc_path = format!("/proc/{}", pid);
                    if std::path::Path::new(&proc_path).exists() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub fn record_toast_pid(pid: u32) {
    if let Some(dir) = dirs_cache_dir() {
        let _ = fs::create_dir_all(&dir);
        let pid_path = dir.join("toast.pid");
        let _ = fs::write(pid_path, pid.to_string());
    }
}

pub fn remove_toast_pid() {
    if let Some(dir) = dirs_cache_dir() {
        let pid_path = dir.join("toast.pid");
        let _ = fs::remove_file(pid_path);
    }
}
