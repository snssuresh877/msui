use std::time::Duration;
use tracing::{debug, info, warn};
use iced::futures::StreamExt;

use crate::network::cache::CacheManager;
use crate::network::connection::{DbusConnection, NM_SERVICE};
use crate::network::service::NetworkService;

pub struct SignalManager;

impl SignalManager {
    /// Start the live D-Bus event listener task for NetworkManager.
    /// Listens for StateChanged, PropertiesChanged, DeviceAdded, DeviceRemoved,
    /// and reconnects automatically if NetworkManager restarts.
    pub fn start_listener() {
        tokio::spawn(async move {
            info!("Starting NetworkManager live D-Bus signal listener");

            loop {
                match DbusConnection::system().await {
                    Ok(dbus) => {
                        debug!("Subscribed to NetworkManager signals on system D-Bus");
                        if let Err(e) = Self::listen_loop(&dbus).await {
                            warn!("NetworkManager signal stream disconnected: {e}. Reconnecting in 3s...");
                        }
                    }
                    Err(e) => {
                        debug!("Cannot connect to system D-Bus ({e}). Retrying in 5s...");
                    }
                }

                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        });
    }

    /// Inner stream processing loop
    async fn listen_loop(dbus: &DbusConnection) -> Result<(), String> {
        // Initial refresh on connect
        NetworkService::refresh_async().await;

        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .sender(NM_SERVICE)
            .map_err(|e| format!("Invalid sender: {e}"))?
            .build();

        let raw_conn = dbus.raw();
        let mut stream = zbus::MessageStream::for_match_rule(rule, raw_conn, None)
            .await
            .map_err(|e| format!("Failed to create MessageStream: {e}"))?;

        while let Some(msg_res) = stream.next().await {
            match msg_res {
                Ok(msg) => {
                    let header = msg.header();
                    let member = header.member().map(|m| m.as_str()).unwrap_or("");
                    debug!("Received NetworkManager signal: {member}");

                    match member {
                        "PropertiesChanged" | "StateChanged" | "DeviceAdded" | "DeviceRemoved" => {
                            CacheManager::invalidate();
                            NetworkService::refresh_async().await;
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    return Err(format!("Stream error: {e}"));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_manager_build_rule() {
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .sender(NM_SERVICE)
            .expect("Valid sender")
            .build();
        assert_eq!(rule.sender().map(|s| s.as_str()), Some(NM_SERVICE));
    }
}
