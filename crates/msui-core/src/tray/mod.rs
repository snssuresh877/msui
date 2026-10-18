pub mod watcher;

use std::sync::{Arc, LazyLock, RwLock};
use tracing::{debug, info};
use zbus::Connection;
use zbus::zvariant::OwnedValue;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TrayItem {
    pub service: String,
    pub path: String,
    pub id: String,
    pub title: String,
    pub icon_name: String,
    pub tooltip: String,
}

pub static GLOBAL_TRAY_ITEMS: LazyLock<Arc<RwLock<Vec<TrayItem>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));

/// Retrieve all currently registered system tray items
pub fn get_tray_items() -> Vec<TrayItem> {
    let in_mem = GLOBAL_TRAY_ITEMS.read().unwrap().clone();
    if !in_mem.is_empty() {
        return in_mem;
    }
    fetch_tray_items_blocking()
}

pub fn fetch_tray_items_blocking() -> Vec<TrayItem> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();
    if let Ok(rt) = rt {
        rt.block_on(fetch_tray_items_async())
    } else {
        Vec::new()
    }
}

pub async fn fetch_tray_items_async() -> Vec<TrayItem> {
    if let Ok(conn) = Connection::session().await {
        let reply = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            conn.call_method(
                Some("org.kde.StatusNotifierWatcher"),
                "/StatusNotifierWatcher",
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.kde.StatusNotifierWatcher", "RegisteredStatusNotifierItems"),
            ),
        )
        .await;

        if let Ok(Ok(msg)) = reply {
            if let Ok(val) = msg.body().deserialize::<zbus::zvariant::OwnedValue>() {
                if let Ok(items) = <Vec<String>>::try_from(val) {
                    let mut result = Vec::new();
                    for item_str in items {
                        let (service, path) = if let Some(idx) = item_str.find('/') {
                            (&item_str[..idx], &item_str[idx..])
                        } else {
                            (item_str.as_str(), "/StatusNotifierItem")
                        };

                        if let Some(tray_item) =
                            watcher::query_tray_item_properties(&conn, service, path).await
                        {
                            result.push(tray_item);
                        }
                    }
                    return result;
                }
            }
        }
    }
    Vec::new()
}


/// Activate a tray item over D-Bus (calls Activate(0, 0) and application activation)
pub fn activate_tray_item(service: &str, path: &str) {
    let s = service.to_string();
    let p = path.to_string();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        if let Ok(rt) = rt {
            rt.block_on(activate_tray_item_async(&s, &p));
        }
    });
}

pub async fn activate_tray_item_async(service: &str, path: &str) {
    if service.is_empty() {
        return;
    }
    let actual_path = if path.is_empty() {
        "/StatusNotifierItem"
    } else {
        path
    };

    info!(
        "Activating StatusNotifierItem: service='{}', path='{}'",
        service, actual_path
    );

    if let Ok(conn) = Connection::session().await {
        // 1. Standard SNI Activate(0, 0)
        let _ = conn
            .call_method(
                Some(service),
                actual_path,
                Some("org.kde.StatusNotifierItem"),
                "Activate",
                &(0i32, 0i32),
            )
            .await;

        // 2. Also try org.freedesktop.Application.Activate for Gtk/Qt desktop apps
        let mut app_service = service.to_string();
        if app_service.starts_with("org.kde.StatusNotifierItem-") {
            let parts: Vec<&str> = app_service.split('-').collect();
            if parts.len() >= 3 {
                app_service = parts[1].to_string();
            }
        }

        if app_service.contains('.') {
            let obj_path = format!("/{}", app_service.replace('.', "/"));
            let empty_dict: std::collections::HashMap<String, OwnedValue> =
                std::collections::HashMap::new();
            let _ = conn
                .call_method(
                    Some(app_service.as_str()),
                    obj_path.as_str(),
                    Some("org.freedesktop.Application"),
                    "Activate",
                    &(empty_dict,),
                )
                .await;
        }

        // 3. Focus corresponding window in Hyprland if it surfaces
        crate::HyprlandIpc::focus_window(&format!("class:{}", app_service));
        debug!("Dispatched activation and window focus for {}", app_service);
    }
}
