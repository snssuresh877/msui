use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[derive(Debug, Clone)]
pub enum HyprlandEvent {
    WorkspaceChanged(i32),
    FocusedWindow { title: String, class: String },
    WorkspaceCreated(i32),
    WorkspaceDestroyed(i32),
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HyprlandWorkspaceRef {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HyprlandClient {
    pub address: String,
    #[serde(default)]
    pub mapped: bool,
    #[serde(default)]
    pub hidden: bool,
    pub workspace: HyprlandWorkspaceRef,
    #[serde(default)]
    pub class: String,
    #[serde(default)]
    pub title: String,
    #[serde(default, rename = "initialClass")]
    pub initial_class: String,
    #[serde(default, rename = "initialTitle")]
    pub initial_title: String,
    #[serde(default)]
    pub pid: i32,
    #[serde(default, rename = "focusHistoryID")]
    pub focus_history_id: i32,
    #[serde(default)]
    pub floating: bool,
}

pub struct HyprlandIpc;

impl HyprlandIpc {
    pub fn socket2_path() -> Option<PathBuf> {
        let xdg = std::env::var("XDG_RUNTIME_DIR").ok()?;
        let his = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
        let path = PathBuf::from(xdg).join("hypr").join(his).join(".socket2.sock");
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn socket1_path() -> Option<PathBuf> {
        let xdg = std::env::var("XDG_RUNTIME_DIR").ok()?;
        let his = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
        let path = PathBuf::from(xdg).join("hypr").join(his).join(".socket.sock");
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn get_clients() -> Vec<HyprlandClient> {
        let output = std::process::Command::new("hyprctl")
            .args(["clients", "-j"])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout);
                Self::parse_clients_json(&text)
            }
            _ => Vec::new(),
        }
    }

    pub fn parse_clients_json(json_str: &str) -> Vec<HyprlandClient> {
        serde_json::from_str::<Vec<HyprlandClient>>(json_str).unwrap_or_default()
    }

    pub fn get_active_client() -> Option<HyprlandClient> {
        let output = std::process::Command::new("hyprctl")
            .args(["activewindow", "-j"])
            .output()
            .ok()?;

        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            serde_json::from_str::<HyprlandClient>(&text).ok()
        } else {
            None
        }
    }

    pub fn get_active_workspace_id() -> i32 {
        let output = std::process::Command::new("hyprctl")
            .args(["activeworkspace", "-j"])
            .output();

        if let Ok(out) = output {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                if let Some(id) = val.get("id").and_then(|v| v.as_i64()) {
                    return id as i32;
                }
            }
        }
        1
    }

    pub fn activate_client(client: &HyprlandClient) -> bool {
        let active_ws = Self::get_active_workspace_id();

        // 1. Unminimize / restore if on special/scratchpad workspace (id < 0 or special:*)
        if client.workspace.id < 0 || client.workspace.name.starts_with("special:") {
            let lua_move = format!(
                "hl.dsp.window.move({{ window = \"address:{}\", workspace = {} }})",
                client.address, active_ws
            );
            let res = std::process::Command::new("hyprctl")
                .args(["dispatch", &lua_move])
                .output();
            if res.is_err() || !res.as_ref().unwrap().status.success() {
                let _ = std::process::Command::new("hyprctl")
                    .args(["dispatch", "movetoworkspace", &format!("{},address:{}", active_ws, client.address)])
                    .output();
            }
        } else if client.workspace.id > 0 && client.workspace.id != active_ws {
            // 2. Switch to window's workspace if on another workspace
            let lua_ws = format!("hl.dsp.focus({{ workspace = {} }})", client.workspace.id);
            let res = std::process::Command::new("hyprctl")
                .args(["dispatch", &lua_ws])
                .output();
            if res.is_err() || !res.as_ref().unwrap().status.success() {
                let _ = std::process::Command::new("hyprctl")
                    .args(["dispatch", "workspace", &client.workspace.id.to_string()])
                    .output();
            }
        }

        // 3. Focus window
        let lua_focus = format!("hl.dsp.focus({{ window = \"address:{}\" }})", client.address);
        let res = std::process::Command::new("hyprctl")
            .args(["dispatch", &lua_focus])
            .output();
        if res.is_err() || !res.as_ref().unwrap().status.success() {
            let _ = std::process::Command::new("hyprctl")
                .args(["dispatch", "focuswindow", &format!("address:{}", client.address)])
                .output();
        }

        // 4. Bring window to the top (un-obscure from background)
        let lua_top = format!("hl.dsp.window.bring_to_top(\"address:{}\")", client.address);
        let res = std::process::Command::new("hyprctl")
            .args(["dispatch", &lua_top])
            .output();
        if res.is_err() || !res.as_ref().unwrap().status.success() {
            let _ = std::process::Command::new("hyprctl")
                .args(["dispatch", "bringactivetotop"])
                .output();
        }

        true
    }

    pub fn focus_window(target: &str) -> bool {
        let clients = Self::get_clients();
        let target_clean = target.strip_prefix("class:").unwrap_or(target);
        if let Some(client) = clients.iter().find(|c| {
            c.address == target
                || c.address == target_clean
                || c.class.eq_ignore_ascii_case(target_clean)
                || c.initial_class.eq_ignore_ascii_case(target_clean)
                || c.title.eq_ignore_ascii_case(target_clean)
        }) {
            return Self::activate_client(client);
        }

        // Direct focus attempt if client is not in listing
        let addr = if target.starts_with("0x") {
            format!("address:{}", target)
        } else if target.starts_with("address:") {
            target.to_string()
        } else {
            format!("class:{}", target_clean)
        };
        let lua_focus = format!("hl.dsp.focus({{ window = \"{}\" }})", addr);
        let res = std::process::Command::new("hyprctl")
            .args(["dispatch", &lua_focus])
            .output();
        if res.is_ok() && res.as_ref().unwrap().status.success() {
            let lua_top = format!("hl.dsp.window.bring_to_top(\"{}\")", addr);
            let _ = std::process::Command::new("hyprctl")
                .args(["dispatch", &lua_top])
                .output();
            return true;
        }

        std::process::Command::new("hyprctl")
            .args(["dispatch", "focuswindow", &addr])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn close_window(address: &str) -> bool {
        let lua_close = format!("hl.dsp.window.close(\"address:{}\")", address);
        let res = std::process::Command::new("hyprctl")
            .args(["dispatch", &lua_close])
            .output();
        if res.is_ok() && res.as_ref().unwrap().status.success() {
            return true;
        }

        let arg = format!("address:{}", address);
        std::process::Command::new("hyprctl")
            .args(["dispatch", "closewindow", &arg])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn switch_workspace_sync(workspace_id: i32) -> bool {
        // 1. Try direct Unix socket write with Lua dispatcher
        if let Some(path) = Self::socket1_path() {
            use std::io::Write;
            if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(path) {
                let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(100)));
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(100)));
                let lua_cmd = format!("dispatch hl.dsp.focus({{ workspace = {} }})\n", workspace_id);
                if stream.write_all(lua_cmd.as_bytes()).is_ok() {
                    return true;
                }
            }
        }

        // 2. Try hyprctl with Lua dispatch
        let lua_focus = format!("hl.dsp.focus({{ workspace = {} }})", workspace_id);
        if let Ok(res) = std::process::Command::new("hyprctl")
            .args(["dispatch", &lua_focus])
            .output()
        {
            if res.status.success() {
                return true;
            }
        }

        // 3. Fallback to standard hyprctl
        let _ = std::process::Command::new("hyprctl")
            .args(["dispatch", "workspace", &workspace_id.to_string()])
            .spawn();

        true
    }

    pub async fn switch_workspace(workspace_id: i32) -> anyhow::Result<()> {
        if let Some(path) = Self::socket1_path() {
            if let Ok(mut stream) = UnixStream::connect(path).await {
                let lua_cmd = format!("dispatch hl.dsp.focus({{ workspace = {} }})\n", workspace_id);
                if stream.write_all(lua_cmd.as_bytes()).await.is_ok() {
                    return Ok(());
                }
            }
        }
        let lua_focus = format!("hl.dsp.focus({{ workspace = {} }})", workspace_id);
        let res = tokio::process::Command::new("hyprctl")
            .args(["dispatch", &lua_focus])
            .output()
            .await;
        if res.is_ok() && res.as_ref().unwrap().status.success() {
            return Ok(());
        }
        // Fallback to standard hyprctl
        let _ = tokio::process::Command::new("hyprctl")
            .args(["dispatch", "workspace", &workspace_id.to_string()])
            .output()
            .await;
        Ok(())
    }


    pub async fn listen_events<F>(mut on_event: F) -> anyhow::Result<()>
    where
        F: FnMut(HyprlandEvent) + Send + 'static,
    {
        let path = Self::socket2_path().ok_or_else(|| anyhow::anyhow!("Hyprland socket2 not found"))?;
        let stream = UnixStream::connect(path).await?;
        let reader = BufReader::new(stream);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let event = Self::parse_event(&line);
            on_event(event);
        }

        Ok(())
    }

    pub fn parse_event(raw: &str) -> HyprlandEvent {
        if let Some((kind, payload)) = raw.split_once(">>") {
            match kind {
                "workspace" => {
                    if let Ok(id) = payload.parse::<i32>() {
                        return HyprlandEvent::WorkspaceChanged(id);
                    }
                }
                "createworkspace" => {
                    if let Ok(id) = payload.parse::<i32>() {
                        return HyprlandEvent::WorkspaceCreated(id);
                    }
                }
                "destroyworkspace" => {
                    if let Ok(id) = payload.parse::<i32>() {
                        return HyprlandEvent::WorkspaceDestroyed(id);
                    }
                }
                "activewindow" => {
                    if let Some((class, title)) = payload.split_once(',') {
                        return HyprlandEvent::FocusedWindow {
                            class: class.to_string(),
                            title: title.to_string(),
                        };
                    } else if payload.is_empty() || payload == "," {
                        return HyprlandEvent::FocusedWindow {
                            class: String::new(),
                            title: String::new(),
                        };
                    }
                }
                "activewindowv2" => {
                    if payload.is_empty() {
                        return HyprlandEvent::FocusedWindow {
                            class: String::new(),
                            title: String::new(),
                        };
                    }
                }
                _ => {}
            }
        }
        HyprlandEvent::Other(raw.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hyprland_clients() {
        let sample = r#"[{
            "address": "0x558f6cd46920",
            "mapped": true,
            "hidden": false,
            "workspace": {
                "id": 2,
                "name": "2"
            },
            "class": "firefox",
            "title": "Mozilla Firefox",
            "initialClass": "firefox",
            "initialTitle": "Mozilla Firefox",
            "pid": 2664,
            "focusHistoryID": 1,
            "floating": false
        }]"#;

        let clients = HyprlandIpc::parse_clients_json(sample);
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].address, "0x558f6cd46920");
        assert_eq!(clients[0].class, "firefox");
        assert_eq!(clients[0].workspace.id, 2);
        assert_eq!(clients[0].pid, 2664);
    }
}
