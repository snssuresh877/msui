use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReminderItem {
    pub id: u64,
    pub title: String,
    pub target_epoch_secs: u64,
    pub total_duration_secs: u64,
    pub triggered: bool,
}

impl ReminderItem {
    pub fn remaining_secs(&self) -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        (self.target_epoch_secs as i64) - (now as i64)
    }

    pub fn formatted_remaining(&self) -> String {
        let rem = self.remaining_secs();
        if rem <= 0 {
            return "Expired".to_string();
        }

        let hours = rem / 3600;
        let mins = (rem % 3600) / 60;
        let secs = rem % 60;

        if hours > 0 {
            format!("{:02}h {:02}m {:02}s", hours, mins, secs)
        } else if mins > 0 {
            format!("{:02}m {:02}s", mins, secs)
        } else {
            format!("{}s", secs)
        }
    }

    pub fn progress(&self) -> f32 {
        if self.total_duration_secs == 0 {
            return 1.0;
        }
        let rem = self.remaining_secs().max(0) as u64;
        let elapsed = self.total_duration_secs.saturating_sub(rem);
        (elapsed as f32 / self.total_duration_secs as f32).clamp(0.0, 1.0)
    }

    pub fn is_expired(&self) -> bool {
        self.remaining_secs() <= 0
    }
}

struct ReminderCache {
    last_mtime: Option<SystemTime>,
    items: Vec<ReminderItem>,
}

static REMINDER_CACHE: Mutex<ReminderCache> = Mutex::new(ReminderCache {
    last_mtime: None,
    items: Vec::new(),
});

static REMINDER_LOCK: Mutex<()> = Mutex::new(());

pub struct ReminderManager;

impl ReminderManager {
    pub fn storage_path() -> PathBuf {
        let base = if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(config_home).join("msui")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config").join("msui")
        } else {
            PathBuf::from("/tmp/msui")
        };
        let _ = fs::create_dir_all(&base);
        base.join("reminders.json")
    }

    pub fn load() -> Vec<ReminderItem> {
        let _guard = REMINDER_LOCK.lock().unwrap();
        Self::load_unlocked()
    }

    fn load_unlocked() -> Vec<ReminderItem> {
        let path = Self::storage_path();
        if !path.exists() {
            return Vec::new();
        }

        let mtime = fs::metadata(&path).ok().and_then(|m| m.modified().ok());
        {
            let cache = REMINDER_CACHE.lock().unwrap();
            if let (Some(cur_mtime), Some(cached_mtime)) = (mtime, cache.last_mtime) {
                if cur_mtime <= cached_mtime {
                    return cache.items.clone();
                }
            }
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let items: Vec<ReminderItem> = serde_json::from_str(&content).unwrap_or_default();
        if let Ok(mut cache) = REMINDER_CACHE.lock() {
            cache.last_mtime = mtime;
            cache.items = items.clone();
        }
        items
    }

    pub fn save(reminders: &[ReminderItem]) {
        let _guard = REMINDER_LOCK.lock().unwrap();
        Self::save_unlocked(reminders);
    }

    fn save_unlocked(reminders: &[ReminderItem]) {
        let path = Self::storage_path();
        if let Ok(json_str) = serde_json::to_string_pretty(reminders) {
            let _ = fs::write(&path, json_str);
            let mtime = fs::metadata(&path).ok().and_then(|m| m.modified().ok());
            if let Ok(mut cache) = REMINDER_CACHE.lock() {
                cache.last_mtime = mtime;
                cache.items = reminders.to_vec();
            }
        }
    }

    pub fn list_active() -> Vec<ReminderItem> {
        let all = Self::load();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();

        // Keep reminders that haven't been triggered or triggered within the last 10 minutes
        all.into_iter()
            .filter(|r| !r.triggered || (r.target_epoch_secs + 600 > now))
            .collect()
    }

    pub fn next_expiring() -> Option<ReminderItem> {
        let active = Self::list_active();
        // Look for the untriggered reminder with the earliest target time, or an expired reminder
        active
            .into_iter()
            .filter(|r| !r.triggered)
            .min_by_key(|r| r.target_epoch_secs)
    }

    pub fn add_reminder(title: &str, minutes: u64) -> ReminderItem {
        Self::add_reminder_seconds(title, minutes * 60)
    }

    pub fn add_reminder_seconds(title: &str, seconds: u64) -> ReminderItem {
        let _guard = REMINDER_LOCK.lock().unwrap();
        let mut list = Self::load_unlocked();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();

        let id = now * 1000 + (list.len() as u64) % 1000;
        let item = ReminderItem {
            id,
            title: title.trim().to_string(),
            target_epoch_secs: now + seconds,
            total_duration_secs: seconds,
            triggered: false,
        };

        list.push(item.clone());
        Self::save_unlocked(&list);
        item
    }

    pub fn remove_reminder(id: u64) -> bool {
        let _guard = REMINDER_LOCK.lock().unwrap();
        let mut list = Self::load_unlocked();
        let initial_len = list.len();
        list.retain(|r| r.id != id);
        let removed = list.len() < initial_len;
        if removed {
            Self::save_unlocked(&list);
        }
        removed
    }

    pub fn clear_all() {
        let _guard = REMINDER_LOCK.lock().unwrap();
        Self::save_unlocked(&[]);
    }

    pub fn check_and_trigger_expired() -> Vec<ReminderItem> {
        let _guard = REMINDER_LOCK.lock().unwrap();
        let mut list = Self::load_unlocked();
        let mut newly_expired = Vec::new();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();

        for item in list.iter_mut() {
            if !item.triggered && item.target_epoch_secs <= now {
                item.triggered = true;
                newly_expired.push(item.clone());
            }
        }

        if !newly_expired.is_empty() {
            Self::save_unlocked(&list);

            for item in &newly_expired {
                // 1. Dispatch desktop notification
                let mut store = crate::GLOBAL_NOTIFICATIONS.lock().unwrap();
                let mins = (item.total_duration_secs / 60).max(1);
                store.add(
                    "msui-timer".into(),
                    None,
                    format!("󰥔 Reminder: {}", item.title),
                    format!("Your scheduled {}m timer for \"{}\" has finished!", mins, item.title),
                    vec![],
                    10000,
                );

                // 2. Play audible chime via paplay / pw-play
                std::thread::spawn(|| {
                    let sounds = [
                        "/usr/share/sounds/freedesktop/stereo/alarm-clock-elapsed.oga",
                        "/usr/share/sounds/freedesktop/stereo/complete.oga",
                    ];
                    for s in sounds {
                        if std::path::Path::new(s).exists() {
                            let _ = Command::new("paplay").arg(s).output();
                            break;
                        }
                    }
                });
            }
        }

        newly_expired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reminder_calculations() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let item = ReminderItem {
            id: 1,
            title: "Test Task".into(),
            target_epoch_secs: now + 300,
            total_duration_secs: 600,
            triggered: false,
        };

        assert!(!item.is_expired());
        let rem = item.remaining_secs();
        assert!(rem > 290 && rem <= 300);
        let prog = item.progress();
        assert!(prog >= 0.49 && prog <= 0.51);

        let formatted = item.formatted_remaining();
        assert!(formatted.contains("04m") || formatted.contains("05m"));
    }

    #[test]
    fn test_reminder_serialization() {
        let item = ReminderItem {
            id: 42,
            title: "Coffee Break".into(),
            target_epoch_secs: 1700000000,
            total_duration_secs: 900,
            triggered: false,
        };

        let json = serde_json::to_string(&item).unwrap();
        let parsed: ReminderItem = serde_json::from_str(&json).unwrap();
        assert_eq!(item, parsed);
    }
}
