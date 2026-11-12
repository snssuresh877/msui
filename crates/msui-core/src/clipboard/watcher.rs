use super::{is_password_hint_active, ClipboardStore, GLOBAL_CLIPBOARD};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::time::sleep;

/// Starts the Wayland clipboard watcher.
///
/// Uses `wl-paste --watch` for true zero-CPU event-driven selection updates.
/// If `wl-paste` is not present or terminates unexpectedly, it seamlessly falls
/// back to low-frequency adaptive polling with idle backoff.
pub async fn start_clipboard_watcher() {
    // Initial read of current clipboard content (skips passwords)
    if !is_password_hint_active() {
        if let Ok(Some(current)) = tokio::task::spawn_blocking(ClipboardStore::read_system).await {
            if !current.trim().is_empty() {
                let mut store = GLOBAL_CLIPBOARD.lock().unwrap();
                store.add(current);
            }
        }
    }

    // Attempt event-driven watcher via `wl-paste --watch <msui_exe> clipboard-ingest`
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("msui"));
    let mut cmd = tokio::process::Command::new("wl-paste");
    cmd.args(["--watch", exe.to_str().unwrap_or("msui"), "clipboard-ingest"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);

    match cmd.spawn() {
        Ok(mut child) => {
            tracing::info!("Started event-driven Wayland clipboard watcher (wl-paste --watch)");
            let status = child.wait().await;
            tracing::warn!("wl-paste --watch exited ({:?}); falling back to adaptive polling", status);
        }
        Err(e) => {
            tracing::warn!("Failed to spawn wl-paste --watch: {:?}; falling back to adaptive polling", e);
        }
    }

    // Fallback: Adaptive polling (2s-5s) with password hint checking
    let mut last_seen = tokio::task::spawn_blocking(ClipboardStore::read_system)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let mut poll_interval = Duration::from_secs(2);

    loop {
        sleep(poll_interval).await;
        if !is_password_hint_active() {
            if let Ok(Some(current)) = tokio::task::spawn_blocking(ClipboardStore::read_system).await {
                if !current.trim().is_empty() && current != last_seen {
                    last_seen = current.clone();
                    let mut store = GLOBAL_CLIPBOARD.lock().unwrap();
                    store.add(current);
                    poll_interval = Duration::from_secs(2);
                } else {
                    poll_interval = (poll_interval + Duration::from_millis(500)).min(Duration::from_secs(5));
                }
            }
        }
    }
}
