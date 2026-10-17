use super::{dismiss_active_toast, is_toast_running, GLOBAL_NOTIFICATIONS};
use std::collections::HashMap;
use zbus::interface;
use zbus::zvariant::OwnedValue;

pub struct NotificationServer;

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationServer {
    #[allow(clippy::too_many_arguments)]
    async fn notify(
        &mut self,
        app_name: String,
        _replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        _hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        let icon_opt = if app_icon.is_empty() { None } else { Some(app_icon) };
        let notif_id = {
            let mut store = GLOBAL_NOTIFICATIONS.lock().unwrap();
            let notif = store.add(app_name, icon_opt, summary, body, actions, expire_timeout);
            notif.id
        };

        // Spawn msui toast popup window if not currently running
        if !is_toast_running() {
            if let Ok(exe) = std::env::current_exe() {
                if let Ok(mut child) = std::process::Command::new(exe).arg("toast").spawn() {
                    std::thread::spawn(move || {
                        let _ = child.wait();
                    });
                }
            }
        }

        notif_id
    }

    async fn close_notification(&self, id: u32) {
        dismiss_active_toast(id);
    }

    async fn get_capabilities(&self) -> Vec<&'static str> {
        vec!["body", "actions", "icon-static"]
    }

    async fn get_server_information(&self) -> (&'static str, &'static str, &'static str, &'static str) {
        ("msUI Notification Daemon", "msUI", "0.1.0", "1.2")
    }
}

pub async fn run_notification_service() -> anyhow::Result<()> {
    let server = NotificationServer;
    let _conn = zbus::connection::Builder::session()?
        .name("org.freedesktop.Notifications")?
        .serve_at("/org/freedesktop/Notifications", server)?
        .build()
        .await?;

    tracing::info!("msUI Notification D-Bus service running on org.freedesktop.Notifications");
    // Keep connection alive in background task
    std::future::pending::<()>().await;
    Ok(())
}
