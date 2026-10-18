pub mod service;

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotificationAction {
    pub key: String,
    pub label: String,
}

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
    #[serde(default = "default_unread")]
    pub unread: bool,
}

fn default_unread() -> bool {
    true
}

impl Notification {
    /// Parse FreeDesktop actions `[key1, label1, key2, label2]` into structured NotificationAction list
    pub fn parsed_actions(&self) -> Vec<NotificationAction> {
        let mut result = Vec::new();
        let mut i = 0;
        while i + 1 < self.actions.len() {
            let key = &self.actions[i];
            let label = &self.actions[i + 1];
            if !label.trim().is_empty() {
                result.push(NotificationAction {
                    key: key.clone(),
                    label: label.clone(),
                });
            }
            i += 2;
        }
        // Fallback for single actions like ["default"] with no separate label
        if result.is_empty() && self.actions.len() == 1 {
            result.push(NotificationAction {
                key: self.actions[0].clone(),
                label: "Open".to_string(),
            });
        }
        result
    }
}

pub struct NotificationStore {
    next_id: u32,
    history: VecDeque<Notification>,
    sender: broadcast::Sender<Notification>,
    cache_path: Option<PathBuf>,
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

        Self {
            next_id: 1,
            history: VecDeque::with_capacity(100),
            sender,
            cache_path,
        }
    }

    pub fn new_in_memory() -> Self {
        let (sender, _) = broadcast::channel(32);
        Self {
            next_id: 1,
            history: VecDeque::with_capacity(100),
            sender,
            cache_path: None,
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
            unread: true,
        };

        if self.history.len() >= 100 {
            self.history.pop_front();
        }
        self.history.push_back(notif.clone());

        self.save();
        add_active_toast(&notif);

        let _ = self.sender.send(notif.clone());
        notif
    }

    pub fn mark_all_read(&mut self) {
        let mut changed = false;
        for n in &mut self.history {
            if n.unread {
                n.unread = false;
                changed = true;
            }
        }
        if changed {
            self.save();
        }
    }

    pub fn unread_count(&self) -> usize {
        self.history.iter().filter(|n| n.unread).count()
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

pub fn save_active_toasts(toasts: &[Notification]) {
    if let Some(dir) = dirs_cache_dir() {
        let path = dir.join("active_toasts.json");
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(toasts) {
            let _ = fs::write(&path, json);
        }
    }
}

pub fn add_active_toast(notif: &Notification) {
    let mut toasts = load_active_toasts();
    toasts.retain(|n| n.id != notif.id);
    toasts.insert(0, notif.clone());
    if toasts.len() > 6 {
        toasts.truncate(6);
    }
    save_active_toasts(&toasts);
}

pub fn dismiss_active_toast(id: u32) {
    let mut toasts = load_active_toasts();
    let before_len = toasts.len();
    toasts.retain(|n| n.id != id);
    if toasts.len() != before_len {
        save_active_toasts(&toasts);
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

pub struct DndManager;

impl DndManager {
    pub fn dnd_file_path() -> PathBuf {
        let dir = dirs_cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        dir.join("dnd_enabled")
    }

    pub fn is_dnd_enabled() -> bool {
        Self::dnd_file_path().exists()
    }

    pub fn set_dnd(enabled: bool) {
        let path = Self::dnd_file_path();
        if enabled {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&path, "1");
        } else {
            let _ = fs::remove_file(&path);
        }
    }

    pub fn toggle_dnd() -> bool {
        let new_state = !Self::is_dnd_enabled();
        Self::set_dnd(new_state);
        new_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dnd_toggle() {
        let initial = DndManager::is_dnd_enabled();
        DndManager::set_dnd(true);
        assert!(DndManager::is_dnd_enabled());
        DndManager::set_dnd(false);
        assert!(!DndManager::is_dnd_enabled());
        DndManager::set_dnd(initial);
    }

    #[test]
    fn test_parsed_actions() {
        let notif = Notification {
            id: 1,
            app_name: "test".into(),
            app_icon: None,
            summary: "s".into(),
            body: "b".into(),
            actions: vec!["default".into(), "View".into(), "close".into(), "Dismiss".into()],
            expire_timeout_ms: 5000,
            timestamp: "12:00".into(),
            unread: true,
        };
        let actions = notif.parsed_actions();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].key, "default");
        assert_eq!(actions[0].label, "View");
        assert_eq!(actions[1].key, "close");
        assert_eq!(actions[1].label, "Dismiss");
    }
}
