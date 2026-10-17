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
