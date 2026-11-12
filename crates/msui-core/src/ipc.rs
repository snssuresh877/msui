use std::env;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tracing::{error, info};

/// Path to the msUI Unix domain socket
pub fn get_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("msui.sock")
    } else {
        let uid = unsafe { libc::getuid() };
        let dir = PathBuf::from(format!("/tmp/msui-runtime-{}", uid));
        if !dir.exists() {
            let _ = std::fs::create_dir_all(&dir);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
            }
        }
        dir.join("msui.sock")
    }
}

/// IPC commands sent from CLI clients to the msUI daemon
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcCommand {
    Toggle(String),
    Open(String),
    Close,
    DndToggle,
    DndSet(bool),
    WallpaperCycle,
    NotificationAction { id: u32, action: String },
    NotificationClose { id: u32, reason: u32 },
    Osd { kind: String, action: String },
    ReloadConfig,
    Ping,
    ClipboardAdd(String),
    Unknown(String),
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

impl IpcCommand {
    pub fn parse(s: &str) -> Self {
        let parts: Vec<&str> = s.trim().split_whitespace().collect();
        match parts.as_slice() {
            ["toggle", target] => IpcCommand::Toggle(target.to_string()),
            ["bar", "toggle"] | ["bar"] => IpcCommand::Toggle("bar".to_string()),
            ["bar", "open"] => IpcCommand::Open("bar".to_string()),
            ["bar", "close"] => IpcCommand::Close,
            ["open", target] => IpcCommand::Open(target.to_string()),
            ["close"] => IpcCommand::Close,
            ["dnd", "toggle"] => IpcCommand::DndToggle,
            ["dnd", "on"] | ["dnd", "enable"] => IpcCommand::DndSet(true),
            ["dnd", "off"] | ["dnd", "disable"] => IpcCommand::DndSet(false),
            ["wallpaper", "cycle"] => IpcCommand::WallpaperCycle,
            ["reload"] | ["config", "reload"] => IpcCommand::ReloadConfig,
            ["osd", kind, action] => IpcCommand::Osd {
                kind: kind.to_string(),
                action: action.to_string(),
            },
            ["clip-add", hex] => {
                if let Some(bytes) = decode_hex(hex) {
                    if let Ok(text) = String::from_utf8(bytes) {
                        IpcCommand::ClipboardAdd(text)
                    } else {
                        IpcCommand::Unknown(s.trim().to_string())
                    }
                } else {
                    IpcCommand::Unknown(s.trim().to_string())
                }
            }
            ["notif-action", id_str, action] => {
                if let Ok(id) = id_str.parse::<u32>() {
                    IpcCommand::NotificationAction {
                        id,
                        action: action.to_string(),
                    }
                } else {
                    IpcCommand::Unknown(s.trim().to_string())
                }
            }
            ["notif-close", id_str, reason_str] => {
                if let (Ok(id), Ok(reason)) = (id_str.parse::<u32>(), reason_str.parse::<u32>()) {
                    IpcCommand::NotificationClose { id, reason }
                } else {
                    IpcCommand::Unknown(s.trim().to_string())
                }
            }
            ["ping"] => IpcCommand::Ping,
            _ => IpcCommand::Unknown(s.trim().to_string()),
        }
    }

    pub fn to_wire(&self) -> String {
        match self {
            IpcCommand::Toggle(target) => format!("toggle {}", target),
            IpcCommand::Open(target) => format!("open {}", target),
            IpcCommand::Close => "close".to_string(),
            IpcCommand::DndToggle => "dnd toggle".to_string(),
            IpcCommand::DndSet(true) => "dnd on".to_string(),
            IpcCommand::DndSet(false) => "dnd off".to_string(),
            IpcCommand::WallpaperCycle => "wallpaper cycle".to_string(),
            IpcCommand::ReloadConfig => "reload".to_string(),
            IpcCommand::Osd { kind, action } => format!("osd {} {}", kind, action),
            IpcCommand::NotificationAction { id, action } => {
                format!("notif-action {} {}", id, action)
            }
            IpcCommand::NotificationClose { id, reason } => {
                format!("notif-close {} {}", id, reason)
            }
            IpcCommand::ClipboardAdd(text) => format!("clip-add {}", encode_hex(text.as_bytes())),
            IpcCommand::Ping => "ping".to_string(),
            IpcCommand::Unknown(cmd) => cmd.clone(),
        }
    }
}

/// Lightweight client for sending single-shot IPC messages with sub-millisecond latency
pub struct IpcClient;

impl IpcClient {
    /// Send command to running msUI daemon. Returns Ok(response) or Err if daemon not running.
    pub async fn send(cmd: &IpcCommand) -> anyhow::Result<String> {
        let socket_path = get_socket_path();
        let timeout = std::time::Duration::from_millis(500);

        let stream = tokio::time::timeout(timeout, UnixStream::connect(&socket_path)).await??;
        let (reader, mut writer) = stream.into_split();

        let mut wire_msg = cmd.to_wire();
        wire_msg.push('\n');
        writer.write_all(wire_msg.as_bytes()).await?;
        writer.flush().await?;

        let mut buf_reader = BufReader::new(reader);
        let mut response = String::new();
        tokio::time::timeout(timeout, buf_reader.read_line(&mut response)).await??;

        Ok(response.trim().to_string())
    }

    /// Synchronous convenience helper for quick CLI execution
    pub fn send_blocking(cmd: &IpcCommand) -> anyhow::Result<String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(Self::send(cmd))
    }
}

/// IPC server running in background daemon (e.g. inside `msui bar`)
pub struct IpcServer {
    socket_path: PathBuf,
    is_running: Arc<AtomicBool>,
}

impl IpcServer {
    pub fn new() -> Self {
        Self {
            socket_path: get_socket_path(),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start(&self, tx: mpsc::Sender<IpcCommand>) -> anyhow::Result<()> {
        // Clean up stale socket file if it exists
        if self.socket_path.exists() {
            // Check if socket is active
            if UnixStream::connect(&self.socket_path).await.is_err() {
                let _ = std::fs::remove_file(&self.socket_path);
            } else {
                anyhow::bail!("Another msUI daemon instance is already listening on {:?}", self.socket_path);
            }
        }

        let listener = UnixListener::bind(&self.socket_path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.socket_path, std::fs::Permissions::from_mode(0o600));
        }
        info!("msUI IPC socket listening on {:?}", self.socket_path);
        self.is_running.store(true, Ordering::SeqCst);

        let socket_path_clone = self.socket_path.clone();

        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    let (reader, mut writer) = stream.into_split();
                    let mut buf_reader = BufReader::new(reader);
                    let mut line = String::new();

                    if let Ok(bytes) = buf_reader.read_line(&mut line).await {
                        if bytes > 0 {
                            let cmd = IpcCommand::parse(&line);
                            let resp = match &cmd {
                                IpcCommand::Ping => "PONG\n".to_string(),
                                IpcCommand::ClipboardAdd(text) => {
                                    {
                                        let mut store = crate::clipboard::GLOBAL_CLIPBOARD.lock().unwrap();
                                        store.add(text.clone());
                                    }
                                    let _ = tx_clone.send(cmd.clone()).await;
                                    "OK\n".to_string()
                                }
                                other => {
                                    if let Err(e) = tx_clone.send(other.clone()).await {
                                        error!("Failed to dispatch IPC command: {:?}", e);
                                        "ERR dispatch_failed\n".to_string()
                                    } else {
                                        "OK\n".to_string()
                                    }
                                }
                            };
                            let _ = writer.write_all(resp.as_bytes()).await;
                            let _ = writer.flush().await;
                        }
                    }
                });
            }
            let _ = std::fs::remove_file(socket_path_clone);
        });

        Ok(())
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        if self.is_running.load(Ordering::SeqCst) {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }
}
