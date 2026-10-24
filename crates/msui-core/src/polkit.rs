use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::{error, info, warn};
use zbus::zvariant::{OwnedValue, Value};
use zbus::{connection::Connection, interface, DBusError};

const AGENT_HELPER_SOCKET: &str = "/run/polkit/agent-helper.socket";
const POLKIT_BUS: &str = "org.freedesktop.PolicyKit1";
const POLKIT_PATH: &str = "/org/freedesktop/PolicyKit1/Authority";
const POLKIT_INTERFACE: &str = "org.freedesktop.PolicyKit1.Authority";
const AGENT_PATH: &str = "/org/freedesktop/PolicyKit1/AuthenticationAgent";

#[derive(DBusError, Debug)]
#[zbus(prefix = "org.freedesktop.PolicyKit1.Error")]
pub enum PolkitError {
    #[zbus(error)]
    ZBus(zbus::Error),
    Failed(String),
    Cancelled(String),
    NotSupported(String),
    NotAuthorized(String),
}

#[derive(Debug, Clone)]
pub struct PolkitRequest {
    pub action_id: String,
    pub message: String,
    pub icon_name: String,
    pub user: String,
    pub cookie: String,
    pub command: String,
    pub responder: Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<bool>>>>,
}

static POLKIT_REQUEST_TX: std::sync::RwLock<Option<tokio::sync::mpsc::UnboundedSender<PolkitRequest>>> =
    std::sync::RwLock::new(None);

pub fn register_polkit_request_handler(tx: tokio::sync::mpsc::UnboundedSender<PolkitRequest>) {
    if let Ok(mut lock) = POLKIT_REQUEST_TX.write() {
        *lock = Some(tx);
    }
}

static POLKIT_CANCEL_TX: std::sync::RwLock<Option<tokio::sync::mpsc::UnboundedSender<String>>> =
    std::sync::RwLock::new(None);

pub fn register_polkit_cancel_handler(tx: tokio::sync::mpsc::UnboundedSender<String>) {
    if let Ok(mut lock) = POLKIT_CANCEL_TX.write() {
        *lock = Some(tx);
    }
}

/// Helper method to verify credentials against /run/polkit/agent-helper.socket
/// (or fallback to setuid fork-exec helper if socket is unavailable).
pub async fn verify_polkit_credentials(user: &str, cookie: &str, password: &str) -> anyhow::Result<bool> {
    if Path::new(AGENT_HELPER_SOCKET).exists() {
        match try_socket_helper(user, cookie, password).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                warn!("Helper socket failed: {:?}, attempting fork fallback", e);
            }
        }
    }

    try_fork_helper(user, cookie, password)
}

async fn try_socket_helper(user: &str, cookie: &str, password: &str) -> anyhow::Result<bool> {
    let stream = UnixStream::connect(AGENT_HELPER_SOCKET).await?;
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    // Write preamble: <username>\n<cookie>\n
    let preamble = format!("{}\n{}\n", user, cookie);
    writer.write_all(preamble.as_bytes()).await?;
    writer.flush().await?;

    let mut line = String::new();
    loop {
        line.clear();
        let bytes = buf_reader.read_line(&mut line).await?;
        if bytes == 0 {
            return Ok(false);
        }

        let trimmed = line.trim();
        if trimmed.starts_with("PAM_PROMPT_ECHO_OFF") || trimmed.starts_with("PAM_PROMPT_ECHO_ON") {
            let mut resp = password.to_string();
            resp.push('\n');
            writer.write_all(resp.as_bytes()).await?;
            writer.flush().await?;
        } else if trimmed == "SUCCESS" {
            return Ok(true);
        } else if trimmed == "FAILURE" {
            return Ok(false);
        }
    }
}

fn try_fork_helper(user: &str, cookie: &str, password: &str) -> anyhow::Result<bool> {
    let candidates = [
        "/usr/lib/polkit-1/polkit-agent-helper-1",
        "/usr/libexec/polkit-1/polkit-agent-helper-1",
        "/usr/lib64/polkit-1/polkit-agent-helper-1",
    ];

    let helper_path = candidates
        .iter()
        .find(|p| Path::new(p).exists())
        .ok_or_else(|| anyhow::anyhow!("No polkit-agent-helper-1 found"))?;

    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new(helper_path)
        .arg(user)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        let payload = format!("{}\n{}\n", cookie, password);
        let _ = stdin.write_all(payload.as_bytes());
    }

    let status = child.wait()?;
    Ok(status.success())
}

/// Active authentication agent handling requests from Polkit1
pub struct PolkitAgent {
    active_child_pid: Arc<AtomicU32>,
}

impl PolkitAgent {
    pub fn new() -> Self {
        Self {
            active_child_pid: Arc::new(AtomicU32::new(0)),
        }
    }
}

#[interface(name = "org.freedesktop.PolicyKit1.AuthenticationAgent")]
impl PolkitAgent {
    async fn begin_authentication(
        &self,
        action_id: String,
        message: String,
        icon_name: String,
        details: HashMap<String, String>,
        cookie: String,
        identities: Vec<(String, HashMap<String, OwnedValue>)>,
    ) -> Result<(), PolkitError> {
        info!("Polkit BeginAuthentication: action_id='{}', message='{}'", action_id, message);

        // Resolve target user
        let mut target_user = "root".to_string();
        for (kind, dict) in &identities {
            if kind == "unix-user" {
                if let Some(uid_val) = dict.get("uid") {
                    if let Ok(uid) = u32::try_from(uid_val) {
                        target_user = username_for_uid(uid);
                        break;
                    }
                }
            }
        }

        // Resolve command line
        let command = details
            .get("command_line")
            .or_else(|| details.get("cmdline"))
            .or_else(|| details.get("command"))
            .cloned()
            .unwrap_or_else(|| parse_command_from_message(&message));

        // If an in-process dialog handler is registered (daemon mode), send request and await response
        let in_process_tx = POLKIT_REQUEST_TX.read().ok().and_then(|r| r.clone());
        if let Some(tx) = in_process_tx {
            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
            let req = PolkitRequest {
                action_id: action_id.clone(),
                message: message.clone(),
                icon_name: icon_name.clone(),
                user: target_user.clone(),
                cookie: cookie.clone(),
                command: command.clone(),
                responder: Arc::new(std::sync::Mutex::new(Some(resp_tx))),
            };

            if tx.send(req).is_ok() {
                match resp_rx.await {
                    Ok(true) => {
                        info!("In-process polkit elevation dialog authenticated successfully");
                        return Ok(());
                    }
                    Ok(false) => {
                        warn!("In-process polkit authentication cancelled or failed by user");
                        return Err(PolkitError::Cancelled("Authentication cancelled".to_string()));
                    }
                    Err(_) => {
                        warn!("In-process polkit dialog responder dropped");
                        return Err(PolkitError::Cancelled("Authentication cancelled".to_string()));
                    }
                }
            }
        }

        // Fallback: Spawn msui-polkit elevation dialog
        let bin = env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("msui"));
        let mut cmd = tokio::process::Command::new(bin);
        cmd.arg("polkit-dialog");
        cmd.arg("--action-id").arg(&action_id);
        cmd.arg("--message").arg(&message);
        cmd.arg("--icon").arg(&icon_name);
        cmd.arg("--user").arg(&target_user);
        cmd.arg("--cookie").arg(&cookie);
        cmd.arg("--command").arg(&command);

        match cmd.spawn() {
            Ok(mut child) => {
                if let Some(pid) = child.id() {
                    self.active_child_pid.store(pid, Ordering::SeqCst);
                }

                let status = child.wait().await;
                self.active_child_pid.store(0, Ordering::SeqCst);

                match status {
                    Ok(exit_status) if exit_status.success() => {
                        info!("Polkit elevation dialog authenticated successfully");
                        Ok(())
                    }
                    Ok(_) => {
                        warn!("Polkit authentication cancelled or failed by user");
                        Err(PolkitError::Cancelled("Authentication cancelled".to_string()))
                    }
                    Err(e) => {
                        error!("Failed to wait for polkit dialog: {:?}", e);
                        Err(PolkitError::Failed(format!("Dialog execution error: {:?}", e)))
                    }
                }
            }
            Err(e) => {
                error!("Failed to spawn polkit-dialog: {:?}", e);
                Err(PolkitError::Failed(format!("Failed to spawn dialog: {:?}", e)))
            }
        }
    }

    async fn cancel_authentication(&self, cookie: String) -> Result<(), PolkitError> {
        info!("Polkit CancelAuthentication for cookie {}", cookie);
        if let Some(cancel_tx) = POLKIT_CANCEL_TX.read().ok().and_then(|r| r.clone()) {
            let _ = cancel_tx.send(cookie.clone());
        }
        let pid = self.active_child_pid.swap(0, Ordering::SeqCst);
        if pid > 0 {
            let _ = Command::new("kill").arg("-TERM").arg(pid.to_string()).status();
        }
        Ok(())
    }
}

pub fn parse_command_from_message(msg: &str) -> String {
    if let Some(idx) = msg.find("to run `") {
        let rest = &msg[idx + 8..];
        if let Some(end) = rest.find('`') {
            return rest[..end].to_string();
        }
    }
    if let Some(idx) = msg.find("to run '") {
        let rest = &msg[idx + 8..];
        if let Some(end) = rest.find('\'') {
            return rest[..end].to_string();
        }
    }
    String::new()
}

pub fn username_for_uid(uid: u32) -> String {
    if uid == 0 {
        return "root".to_string();
    }
    if let Ok(user) = env::var("USER") {
        return user;
    }
    uid.to_string()
}

/// Retrieve the current logind session ID
pub async fn get_logind_session_id(conn: &Connection) -> Option<String> {
    if let Ok(id) = env::var("XDG_SESSION_ID") {
        if !id.is_empty() {
            return Some(id);
        }
    }

    // Fallback: Query systemd-logind over D-Bus for this process
    let pid = std::process::id();
    let reply = conn
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "GetSessionByPID",
            &(pid),
        )
        .await;

    if let Ok(msg) = reply {
        if let Ok(session_path) = msg.body().deserialize::<(zbus::zvariant::OwnedObjectPath,)>() {
            let path_str = session_path.0.to_string();
            if let Some(id) = path_str.split('/').last() {
                return Some(id.to_string());
            }
        }
    }

    None
}

/// Run the Polkit 1 authentication agent daemon
pub async fn run_polkit_agent() -> anyhow::Result<()> {
    info!("Starting msUI Native Polkit 1 Authentication Agent...");

    let conn = Connection::system().await?;
    let agent = PolkitAgent::new();

    conn.object_server()
        .at(AGENT_PATH, agent)
        .await?;

    let session_id = get_logind_session_id(&conn)
        .await
        .ok_or_else(|| anyhow::anyhow!("Could not determine logind session ID"))?;

    info!("Registering msUI Polkit Agent for logind session '{}'...", session_id);

    let locale = env::var("LANG").unwrap_or_default();
    let mut subject_details: HashMap<String, Value> = HashMap::new();
    subject_details.insert("session-id".to_string(), Value::from(session_id.as_str()));

    let subject = ("unix-session", subject_details);

    conn.call_method(
        Some(POLKIT_BUS),
        POLKIT_PATH,
        Some(POLKIT_INTERFACE),
        "RegisterAuthenticationAgent",
        &(subject, locale, AGENT_PATH),
    )
    .await?;

    info!("msUI Polkit 1 Authentication Agent registered successfully!");

    // Keep running until terminated
    std::future::pending::<()>().await;

    Ok(())
}
