use super::{ClipboardStore, GLOBAL_CLIPBOARD};
use std::time::Duration;
use tokio::time::sleep;

pub async fn start_clipboard_watcher() {
    let mut last_seen = tokio::task::spawn_blocking(ClipboardStore::read_system)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    loop {
        sleep(Duration::from_millis(500)).await;
        if let Ok(Some(current)) = tokio::task::spawn_blocking(ClipboardStore::read_system).await {
            if !current.trim().is_empty() && current != last_seen {
                last_seen = current.clone();
                let mut store = GLOBAL_CLIPBOARD.lock().unwrap();
                store.add(current);
            }
        }
    }
}
