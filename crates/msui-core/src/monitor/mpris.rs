use super::media::{MediaInfo, update_media_cache};
use std::collections::HashMap;
use std::sync::RwLock;
use tracing::{debug, error, info};
use zbus::zvariant::OwnedValue;
use zbus::Connection;

pub const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
pub const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
pub const MPRIS_PLAYER_INTERFACE: &str = "org.mpris.MediaPlayer2.Player";

static ACTIVE_PLAYER: RwLock<Option<String>> = RwLock::new(None);
static MPRIS_CONN: RwLock<Option<Connection>> = RwLock::new(None);

pub fn get_active_player() -> Option<String> {
    ACTIVE_PLAYER.read().ok().and_then(|guard| guard.clone())
}

pub fn set_active_player(player: Option<String>) {
    if let Ok(mut guard) = ACTIVE_PLAYER.write() {
        *guard = player;
    }
}

/// Send a native D-Bus method call to the active MPRIS player
pub async fn send_mpris_command(method: &str) -> anyhow::Result<()> {
    let player = match get_active_player() {
        Some(p) => p,
        None => return Err(anyhow::anyhow!("No active MPRIS player")),
    };

    let existing_conn = MPRIS_CONN.read().ok().and_then(|guard| (*guard).clone());

    let conn = match existing_conn {
        Some(c) => c,
        None => zbus::connection::Builder::session()?.build().await?,
    };

    conn.call_method(
        Some(player.as_str()),
        MPRIS_PATH,
        Some(MPRIS_PLAYER_INTERFACE),
        method,
        &(),
    )
    .await?;

    Ok(())
}

/// Parse MPRIS metadata dictionary into MediaInfo
pub fn parse_mpris_properties(player_name: &str, props: &HashMap<String, OwnedValue>) -> Option<MediaInfo> {
    let status_str = props
        .get("PlaybackStatus")
        .and_then(|v| <&str>::try_from(v).ok())
        .unwrap_or("Stopped");
    let is_playing = status_str.eq_ignore_ascii_case("Playing");

    let mut title = String::new();
    let mut artist = String::new();
    let mut album = String::new();
    let mut art_url = None;
    let mut length_secs = 0u64;

    if let Some(meta_val) = props.get("Metadata") {
        let meta_dict: Result<HashMap<String, OwnedValue>, _> = meta_val.clone().try_into();
        if let Ok(meta_dict) = meta_dict {
            if let Some(t) = meta_dict.get("xesam:title").and_then(|v| <&str>::try_from(v).ok()) {
                title = t.to_string();
            }
            if let Some(a) = meta_dict.get("xesam:album").and_then(|v| <&str>::try_from(v).ok()) {
                album = a.to_string();
            }
            if let Some(art) = meta_dict.get("mpris:artUrl").and_then(|v| <&str>::try_from(v).ok())
                && !art.is_empty() {
                    art_url = Some(art.to_string());
                }

            // Artist can be Vec<String> or a String
            if let Some(art_val) = meta_dict.get("xesam:artist") {
                let artists_res: Result<Vec<String>, _> = art_val.clone().try_into();
                if let Ok(artists) = artists_res {
                    artist = artists.join(", ");
                } else if let Ok(s) = <&str>::try_from(art_val) {
                    artist = s.to_string();
                }
            }

            // Length is microseconds
            if let Some(len_val) = meta_dict.get("mpris:length") {
                if let Ok(micros) = <i64>::try_from(len_val) {
                    length_secs = (micros.max(0) as u64) / 1_000_000;
                } else if let Ok(micros) = <u64>::try_from(len_val) {
                    length_secs = micros / 1_000_000;
                }
            }
        }
    }

    let mut position_secs = 0u64;
    if let Some(pos_val) = props.get("Position") {
        if let Ok(micros) = <i64>::try_from(pos_val) {
            position_secs = (micros.max(0) as u64) / 1_000_000;
        } else if let Ok(micros) = <u64>::try_from(pos_val) {
            position_secs = micros / 1_000_000;
        }
    }

    let loop_status = props
        .get("LoopStatus")
        .and_then(|v| <&str>::try_from(v).ok())
        .map(|s| s.to_string());

    let shuffle = props
        .get("Shuffle")
        .and_then(|v| <bool>::try_from(v).ok())
        .unwrap_or(false);

    let clean_player = player_name
        .trim_start_matches(MPRIS_PREFIX)
        .split('.')
        .next()
        .unwrap_or(player_name)
        .to_string();

    if title.is_empty() && artist.is_empty() && !is_playing {
        return None;
    }

    Some(MediaInfo {
        player_name: clean_player,
        is_playing,
        title,
        artist,
        album,
        art_url,
        position_secs,
        length_secs,
        loop_status,
        shuffle,
    })
}

/// Query an individual player's full properties over D-Bus
pub async fn query_player(conn: &Connection, player: &str) -> Option<MediaInfo> {
    let reply = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        conn.call_method(
            Some(player),
            MPRIS_PATH,
            Some("org.freedesktop.DBus.Properties"),
            "GetAll",
            &(MPRIS_PLAYER_INTERFACE,),
        ),
    )
    .await
    .ok()?
    .ok()?;

    let props = reply.body().deserialize::<HashMap<String, OwnedValue>>().ok()?;
    parse_mpris_properties(player, &props)
}

/// Discover active MPRIS players and refresh media cache
pub async fn refresh_mpris_players(conn: &Connection) -> Option<MediaInfo> {
    let reply = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "ListNames",
            &(),
        )
        .await
        .ok()?;

    let (names,): (Vec<String>,) = reply.body().deserialize().ok()?;
    let mut candidate_info: Option<MediaInfo> = None;
    let mut candidate_player: Option<String> = None;

    for name in names {
        if name.starts_with(MPRIS_PREFIX)
            && let Some(info) = query_player(conn, &name).await {
                if info.is_playing {
                    set_active_player(Some(name));
                    update_media_cache(Some(info.clone()));
                    return Some(info);
                } else if candidate_info.is_none() {
                    candidate_info = Some(info);
                    candidate_player = Some(name);
                }
            }
    }

    if let Some(info) = candidate_info {
        set_active_player(candidate_player);
        update_media_cache(Some(info.clone()));
        Some(info)
    } else {
        set_active_player(None);
        update_media_cache(None);
        None
    }
}

/// Run native MPRIS D-Bus listener daemon
pub async fn run_mpris_listener() -> anyhow::Result<()> {
    info!("Starting msUI Native MPRIS D-Bus Event Listener...");

    let conn = zbus::connection::Builder::session()?
        .build()
        .await?;

    if let Ok(mut guard) = MPRIS_CONN.write() {
        *guard = Some(conn.clone());
    }

    // 1. Initial scan of running players
    refresh_mpris_players(&conn).await;

    // 2. Listen to NameOwnerChanged to detect player launch / exit
    let conn_clone = conn.clone();
    tokio::spawn(async move {
        let dbus_proxy = match zbus::fdo::DBusProxy::new(&conn_clone).await {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to create DBusProxy for MPRIS NameOwnerChanged: {:?}", e);
                return;
            }
        };

        if let Ok(mut stream) = dbus_proxy.receive_name_owner_changed().await {
            use iced::futures::StreamExt;
            while let Some(signal) = stream.next().await {
                if let Ok(args) = signal.args() {
                    let name = args.name().as_str();
                    if name.starts_with(MPRIS_PREFIX) {
                        let new_owner = args.new_owner();
                        if new_owner.is_none() || new_owner.as_ref().map(|o| o.is_empty()).unwrap_or(true) {
                            debug!("MPRIS player exited: {}", name);
                            refresh_mpris_players(&conn_clone).await;
                        } else {
                            debug!("MPRIS player appeared: {}", name);
                            refresh_mpris_players(&conn_clone).await;
                        }
                    }
                }
            }
        }
    });

    // 3. Listen to PropertiesChanged on /org/mpris/MediaPlayer2
    use zbus::MatchRule;
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .path(MPRIS_PATH)?
        .build();

    let mut stream = zbus::MessageStream::for_match_rule(
        rule,
        &conn,
        None,
    )
    .await?;

    use iced::futures::StreamExt;
    while let Some(msg) = stream.next().await {
        if let Ok(msg) = msg {
            let sender = msg.header().sender().map(|s| s.as_str().to_string()).unwrap_or_default();
            // Deserializing PropertiesChanged args: (interface_name, changed_properties, invalidated_properties)
            if let Ok((iface, changed, _)) = msg.body().deserialize::<(String, HashMap<String, OwnedValue>, Vec<String>)>()
                && iface == MPRIS_PLAYER_INTERFACE {
                    let player_name = if sender.starts_with(':') {
                        get_active_player().unwrap_or(sender)
                    } else {
                        sender
                    };

                    if let Some(info) = parse_mpris_properties(&player_name, &changed) {
                        update_media_cache(Some(info));
                    } else if let Some(active) = get_active_player()
                        && let Some(full_info) = query_player(&conn, &active).await {
                            update_media_cache(Some(full_info));
                        }
                }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mpris_properties() {
        let mut props = HashMap::new();
        props.insert("PlaybackStatus".to_string(), OwnedValue::try_from(zbus::zvariant::Value::from("Playing")).unwrap());

        let mut meta = HashMap::new();
        meta.insert("xesam:title".to_string(), OwnedValue::try_from(zbus::zvariant::Value::from("Starboy")).unwrap());
        meta.insert("xesam:album".to_string(), OwnedValue::try_from(zbus::zvariant::Value::from("Starboy")).unwrap());
        meta.insert("mpris:artUrl".to_string(), OwnedValue::try_from(zbus::zvariant::Value::from("https://example.com/art.png")).unwrap());
        meta.insert("mpris:length".to_string(), OwnedValue::from(230_000_000i64));
        let artists = vec!["The Weeknd".to_string(), "Daft Punk".to_string()];
        meta.insert("xesam:artist".to_string(), OwnedValue::try_from(zbus::zvariant::Value::from(artists)).unwrap());

        props.insert("Metadata".to_string(), OwnedValue::try_from(zbus::zvariant::Value::from(meta)).unwrap());
        props.insert("Position".to_string(), OwnedValue::from(45_000_000i64));

        let info = parse_mpris_properties("org.mpris.MediaPlayer2.spotify", &props).unwrap();
        assert_eq!(info.player_name, "spotify");
        assert_eq!(info.title, "Starboy");
        assert_eq!(info.artist, "The Weeknd, Daft Punk");
        assert_eq!(info.album, "Starboy");
        assert_eq!(info.art_url.as_deref(), Some("https://example.com/art.png"));
        assert_eq!(info.position_secs, 45);
        assert_eq!(info.length_secs, 230);
        assert!(info.is_playing);
    }

    #[test]
    fn test_parse_mpris_empty() {
        let props = HashMap::new();
        assert!(parse_mpris_properties("org.mpris.MediaPlayer2.mpv", &props).is_none());
    }
}

