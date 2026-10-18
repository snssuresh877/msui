use super::{TrayItem, GLOBAL_TRAY_ITEMS};
use iced::futures::StreamExt;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use zbus::message::Header;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;
use zbus::{interface, Connection};


pub const WATCHER_BUS_NAME: &str = "org.kde.StatusNotifierWatcher";
pub const WATCHER_OBJECT_PATH: &str = "/StatusNotifierWatcher";
pub const WATCHER_INTERFACE: &str = "org.kde.StatusNotifierWatcher";

pub struct StatusNotifierWatcherServer {
    conn: Connection,
}

impl StatusNotifierWatcherServer {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl StatusNotifierWatcherServer {
    async fn register_status_notifier_item(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        #[zbus(header)] header: Header<'_>,
        service: String,
    ) -> zbus::fdo::Result<()> {
        let (actual_service, actual_path) = if service.starts_with('/') {
            let sender = header
                .sender()
                .map(|s| s.as_str().to_string())
                .unwrap_or_default();
            (sender, service)
        } else {
            (service, "/StatusNotifierItem".to_string())
        };

        if actual_service.is_empty() {
            return Ok(());
        }

        info!(
            "StatusNotifierWatcher: RegisterStatusNotifierItem from service='{}', path='{}'",
            actual_service, actual_path
        );

        let reg_key = format!("{}{}", actual_service, actual_path);
        let initial_item = TrayItem {
            service: actual_service.clone(),
            path: actual_path.clone(),
            id: actual_service.clone(),
            title: actual_service.clone(),
            icon_name: String::new(),
            tooltip: String::new(),
        };

        {
            let mut items = GLOBAL_TRAY_ITEMS.write().unwrap();
            items.retain(|i| i.service != initial_item.service || i.path != initial_item.path);
            items.push(initial_item);
        }

        let _ = Self::status_notifier_item_registered(&emitter, &reg_key).await;

        // Query full properties asynchronously so client application is immediately unblocked
        let conn_clone = self.conn.clone();
        tokio::spawn(async move {
            if let Some(item) =
                query_tray_item_properties(&conn_clone, &actual_service, &actual_path).await
            {
                let mut items = GLOBAL_TRAY_ITEMS.write().unwrap();
                if let Some(pos) = items
                    .iter()
                    .position(|i| i.service == item.service && i.path == item.path)
                {
                    items[pos] = item;
                }
            }
        });

        Ok(())
    }

    async fn register_status_notifier_host(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        #[zbus(header)] _header: Header<'_>,
        service: String,
    ) -> zbus::fdo::Result<()> {
        debug!("StatusNotifierWatcher: RegisterStatusNotifierHost from {}", service);
        let _ = Self::status_notifier_host_registered(&emitter).await;
        Ok(())
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        let items = GLOBAL_TRAY_ITEMS.read().unwrap();
        items
            .iter()
            .map(|item| format!("{}{}", item.service, item.path))
            .collect()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        signal_ctxt: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        signal_ctxt: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(
        signal_ctxt: &SignalEmitter<'_>,
    ) -> zbus::Result<()>;
}

/// Query metadata properties of a StatusNotifierItem with timeout
pub async fn query_tray_item_properties(
    conn: &Connection,
    service: &str,
    path: &str,
) -> Option<TrayItem> {
    let actual_path = if path.is_empty() {
        "/StatusNotifierItem"
    } else {
        path
    };

    let mut id = service.to_string();
    let mut title = String::new();
    let mut icon_name = String::new();
    let mut tooltip = String::new();

    // 1. Try GetAll("org.kde.StatusNotifierItem") with 500ms timeout
    let reply = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        conn.call_method(
            Some(service),
            actual_path,
            Some("org.freedesktop.DBus.Properties"),
            "GetAll",
            &("org.kde.StatusNotifierItem",),
        ),
    )
    .await;

    if let Ok(Ok(msg)) = reply {

        if let Ok(dict) = msg.body().deserialize::<HashMap<String, OwnedValue>>() {
            if let Some(val) = dict.get("Id") {
                if let Ok(s) = <&str>::try_from(val) {
                    id = s.to_string();
                }
            }
            if let Some(val) = dict.get("Title") {
                if let Ok(s) = <&str>::try_from(val) {
                    title = s.to_string();
                }
            }
            if let Some(val) = dict.get("IconName") {
                if let Ok(s) = <&str>::try_from(val) {
                    icon_name = s.to_string();
                }
            }
            if let Some(val) = dict.get("IconAccessibleDesc") {
                if let Ok(s) = <&str>::try_from(val) {
                    tooltip = s.to_string();
                }
            }
        }
    } else {
        // Fallback: Individual property gets
        if let Ok(msg) = conn
            .call_method(
                Some(service),
                actual_path,
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.kde.StatusNotifierItem", "Id"),
            )
            .await
        {
            if let Ok(val) = msg.body().deserialize::<OwnedValue>() {
                if let Ok(s) = <&str>::try_from(&val) {
                    id = s.to_string();
                }
            }
        }
        if let Ok(msg) = conn
            .call_method(
                Some(service),
                actual_path,
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.kde.StatusNotifierItem", "Title"),
            )
            .await
        {
            if let Ok(val) = msg.body().deserialize::<OwnedValue>() {
                if let Ok(s) = <&str>::try_from(&val) {
                    title = s.to_string();
                }
            }
        }
        if let Ok(msg) = conn
            .call_method(
                Some(service),
                actual_path,
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &("org.kde.StatusNotifierItem", "IconName"),
            )
            .await
        {
            if let Ok(val) = msg.body().deserialize::<OwnedValue>() {
                if let Ok(s) = <&str>::try_from(&val) {
                    icon_name = s.to_string();
                }
            }
        }
    }

    if title.is_empty() || title.starts_with(':') {
        // Query D-Bus for the process PID to resolve human-readable process name
        let pid_reply = conn
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "GetConnectionUnixProcessID",
                &(service,),
            )
            .await;

        if let Ok(msg) = pid_reply {
            if let Ok((pid,)) = msg.body().deserialize::<(u32,)>() {
                if let Ok(comm) = std::fs::read_to_string(format!("/proc/{}/comm", pid)) {
                    let clean = comm.trim().to_string();
                    if !clean.is_empty() {
                        title = clean.clone();
                        if id.is_empty() || id.starts_with(':') {
                            id = clean;
                        }
                    }
                }
            }
        }
    }

    if title.is_empty() {
        title = id.clone();
    }
    if let Some(idx) = title.find("_status_icon_") {
        title = title[..idx].to_string();
    }
    if tooltip.is_empty() {
        tooltip = title.clone();
    }


    debug!(
        "Discovered tray item: title='{}', id='{}', icon='{}', service='{}'",
        title, id, icon_name, service
    );

    Some(TrayItem {
        service: service.to_string(),
        path: actual_path.to_string(),
        id,
        title,
        icon_name,
        tooltip,
    })
}

/// Poll and discover any pre-existing SNI services on the session bus
pub async fn discover_existing_tray_items(conn: &Connection) {
    let reply = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "ListNames",
            &(),
        )
        .await;

    if let Ok(msg) = reply {
        if let Ok((names,)) = msg.body().deserialize::<(Vec<String>,)>() {
            for name in names {
                if name.starts_with("org.kde.StatusNotifierItem-") {
                    if let Some(item) =
                        query_tray_item_properties(conn, &name, "/StatusNotifierItem").await
                    {
                        let mut items = GLOBAL_TRAY_ITEMS.write().unwrap();
                        items.retain(|i| i.service != item.service);
                        items.push(item);
                    }
                }
            }
        }
    }
}

/// Run the StatusNotifierWatcher service daemon
pub async fn run_tray_watcher() -> anyhow::Result<()> {
    info!("Starting msUI FreeDesktop StatusNotifierWatcher System Tray Daemon...");

    let conn = zbus::connection::Builder::session()?
        .name(WATCHER_BUS_NAME)?
        .build()
        .await?;

    let server = StatusNotifierWatcherServer::new(conn.clone());

    conn.object_server()
        .at(WATCHER_OBJECT_PATH, server)
        .await?;

    // Also register Host name so client applications recognize a tray host is present
    let host_bus_name = format!("org.kde.StatusNotifierHost-{}-0", std::process::id());
    if let Err(e) = conn.request_name(host_bus_name.as_str()).await {
        warn!("Failed to claim {}: {:?}", host_bus_name, e);
    }

    // Discover any already running SNI items
    discover_existing_tray_items(&conn).await;

    // Listen for D-Bus NameOwnerChanged to detect when tray items disconnect
    let conn_clone = conn.clone();
    tokio::spawn(async move {
        let dbus_proxy = match zbus::fdo::DBusProxy::new(&conn_clone).await {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to create DBusProxy for NameOwnerChanged: {:?}", e);
                return;
            }
        };

        if let Ok(mut stream) = dbus_proxy.receive_name_owner_changed().await {
            while let Some(signal) = stream.next().await {
                if let Ok(args) = signal.args() {
                    let name = args.name();
                    let new_owner = args.new_owner();

                    let name_str = name.as_str();
                    // If an application disconnects (new_owner is empty)
                    if new_owner.is_none() || new_owner.as_ref().map(|o| o.is_empty()).unwrap_or(true) {
                        let mut removed = false;
                        let mut reg_key = String::new();
                        {
                            let mut items = GLOBAL_TRAY_ITEMS.write().unwrap();
                            if let Some(pos) = items.iter().position(|i| i.service == name_str) {
                                let item = items.remove(pos);
                                reg_key = format!("{}{}", item.service, item.path);
                                removed = true;
                                info!("Purged disconnected tray item: {}", item.service);
                            }
                        }

                        if removed {
                            let _ = conn_clone
                                .emit_signal(
                                    None::<()>,
                                    WATCHER_OBJECT_PATH,
                                    WATCHER_INTERFACE,
                                    "StatusNotifierItemUnregistered",
                                    &(&reg_key,),
                                )
                                .await;
                        }
                    } else if name_str.starts_with("org.kde.StatusNotifierItem-") {
                        // New SNI service appeared
                        if let Some(item) =
                            query_tray_item_properties(&conn_clone, name_str, "/StatusNotifierItem")
                                .await
                        {
                            let reg_key = format!("{}{}", item.service, item.path);
                            {
                                let mut items = GLOBAL_TRAY_ITEMS.write().unwrap();
                                items.retain(|i| i.service != item.service);
                                items.push(item);
                            }

                            let _ = conn_clone
                                .emit_signal(
                                    None::<()>,
                                    WATCHER_OBJECT_PATH,
                                    WATCHER_INTERFACE,
                                    "StatusNotifierItemRegistered",
                                    &(&reg_key,),
                                )
                                .await;
                        }
                    }
                }
            }
        }
    });

    info!("msUI StatusNotifierWatcher initialized and listening on session bus");
    std::future::pending::<()>().await;
    Ok(())
}
