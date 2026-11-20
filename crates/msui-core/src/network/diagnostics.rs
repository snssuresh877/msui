use std::net::SocketAddr;
use std::time::Duration;
use tracing::warn;

use crate::network::connection::{DbusConnection, NM_IFACE, NM_PATH, NM_SERVICE};
use crate::network::types::InternetConnectivity;

pub struct Diagnostics;

impl Diagnostics {
    /// Fast native D-Bus connectivity check against NetworkManager
    pub async fn check_connectivity() -> InternetConnectivity {
        let dbus = match DbusConnection::system().await {
            Ok(d) => d,
            Err(_) => return InternetConnectivity::Unknown,
        };

        let call_fut = dbus.raw().call_method(
            Some(NM_SERVICE),
            NM_PATH,
            Some(NM_IFACE),
            "CheckConnectivity",
            &(),
        );

        match tokio::time::timeout(Duration::from_millis(1500), call_fut).await {
            Ok(Ok(reply)) => {
                let code: u32 = reply.body().deserialize().unwrap_or(0);
                InternetConnectivity::from(code)
            }
            Ok(Err(e)) => {
                warn!("CheckConnectivity D-Bus call failed: {e}");
                InternetConnectivity::Unknown
            }
            Err(_) => {
                warn!("CheckConnectivity timed out");
                InternetConnectivity::Unknown
            }
        }
    }

    /// Fast non-blocking TCP socket check to test external gateway reachability
    pub async fn test_tcp_reachability(addr: &str, timeout_dur: Duration) -> bool {
        if let Ok(socket_addr) = addr.parse::<SocketAddr>() {
            tokio::time::timeout(timeout_dur, tokio::net::TcpStream::connect(socket_addr))
                .await
                .map(|res| res.is_ok())
                .unwrap_or(false)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connectivity_check() {
        let conn = Diagnostics::check_connectivity().await;
        // Should not panic, returns valid variant
        assert!(matches!(
            conn,
            InternetConnectivity::Unknown
                | InternetConnectivity::None
                | InternetConnectivity::Portal
                | InternetConnectivity::Limited
                | InternetConnectivity::Full
        ));
    }
}
