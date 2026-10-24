use super::{dismiss_active_toast, is_toast_running, GLOBAL_NOTIFICATIONS};
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;
use zbus::{interface, Connection};

pub static IS_UNIFIED_DAEMON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub static NOTIFICATION_BUS_CONN: LazyLock<RwLock<Option<Connection>>> =
    LazyLock::new(|| RwLock::new(None));

pub async fn emit_notification_closed(id: u32, reason: u32) -> anyhow::Result<()> {
    let conn_opt = NOTIFICATION_BUS_CONN.read().unwrap().clone();
    if let Some(conn) = conn_opt {
        let _ = conn
            .emit_signal(
                Option::<&str>::None,
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                "NotificationClosed",
                &(id, reason),
            )
            .await;
    }
    Ok(())
}

pub async fn emit_action_invoked(id: u32, action_key: &str) -> anyhow::Result<()> {
    let conn_opt = NOTIFICATION_BUS_CONN.read().unwrap().clone();
    if let Some(conn) = conn_opt {
        let _ = conn
            .emit_signal(
                Option::<&str>::None,
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                "ActionInvoked",
                &(id, action_key),
            )
            .await;
    }
    Ok(())
}

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

        // Spawn msui toast popup window only if not running in unified daemon mode and DND is OFF
        if !IS_UNIFIED_DAEMON.load(std::sync::atomic::Ordering::Relaxed)
            && !crate::notification::DndManager::is_dnd_enabled()
            && !is_toast_running()
        {
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
        let _ = emit_notification_closed(id, 3).await; // 3 = closed by call to CloseNotification
    }

    async fn get_capabilities(&self) -> Vec<&'static str> {
        vec!["body", "actions", "icon-static"]
    }

    async fn get_server_information(&self) -> (&'static str, &'static str, &'static str, &'static str) {
        ("msUI Notification Daemon", "msUI", "0.1.0", "1.2")
    }

    #[zbus(signal)]
    async fn notification_closed(
        signal_ctxt: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn action_invoked(
        signal_ctxt: &SignalEmitter<'_>,
        id: u32,
        action_key: &str,
    ) -> zbus::Result<()>;
}

pub async fn run_notification_service() -> anyhow::Result<()> {
    let server = NotificationServer;
    let conn = zbus::connection::Builder::session()?
        .name("org.freedesktop.Notifications")?
        .serve_at("/org/freedesktop/Notifications", server)?
        .build()
        .await?;

    *NOTIFICATION_BUS_CONN.write().unwrap() = Some(conn.clone());

    tracing::info!("msUI Notification D-Bus service running on org.freedesktop.Notifications");
    std::future::pending::<()>().await;
    Ok(())
}
