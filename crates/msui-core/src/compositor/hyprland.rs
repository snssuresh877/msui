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

    pub fn focus_window(address: &str) -> bool {
        let arg = format!("address:{}", address);
        std::process::Command::new("hyprctl")
            .args(["dispatch", "focuswindow", &arg])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn close_window(address: &str) -> bool {
        let arg = format!("address:{}", address);
        std::process::Command::new("hyprctl")
            .args(["dispatch", "closewindow", &arg])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub async fn switch_workspace(workspace_id: i32) -> anyhow::Result<()> {
        if let Some(path) = Self::socket1_path() {
            if let Ok(mut stream) = UnixStream::connect(path).await {
                let cmd = format!("dispatch workspace {}\n", workspace_id);
                stream.write_all(cmd.as_bytes()).await?;
                return Ok(());
            }
        }
        // Fallback to hyprctl
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
