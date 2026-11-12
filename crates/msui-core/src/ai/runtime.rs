use super::config::AiConfig;
use super::memory::MemorySentinel;
use super::model_store::ModelStore;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
#[cfg(unix)]
use std::os::unix::process::CommandExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub is_running: bool,
    pub pid: Option<u32>,
    pub model_name: Option<String>,
    pub port: u16,
    pub api_url: String,
    pub ram_usage_mb: Option<f64>,
    pub started_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RuntimeStateFile {
    pid: u32,
    model_name: String,
    port: u16,
    started_at: String,
    #[serde(default)]
    last_activity_epoch: u64,
}

pub struct LlamaRuntime;

impl LlamaRuntime {
    pub fn state_file_path() -> PathBuf {
        AiConfig::config_dir().parent().unwrap().join("runtime.json")
    }

    pub fn log_file_path() -> PathBuf {
        AiConfig::config_dir().parent().unwrap().join("logs/llama-server.log")
    }

    pub fn is_pid_alive(pid: u32) -> bool {
        let p = format!("/proc/{pid}");
        if !Path::new(&p).exists() {
            return false;
        }
        // Verify it is indeed llama-server (or transitioning child)
        if let Ok(cmdline) = std::fs::read_to_string(format!("/proc/{pid}/cmdline")) {
            if cmdline.is_empty() || cmdline.contains("llama-server") || cmdline.contains("msui") {
                return true;
            }
            return false;
        }
        true
    }

    pub fn get_process_rss_mb(pid: u32) -> Option<f64> {
        let statm_path = format!("/proc/{pid}/statm");
        if let Ok(content) = std::fs::read_to_string(statm_path) {
            let mut parts = content.split_whitespace();
            // second number in statm is resident pages
            let _ = parts.next();
            if let Some(rss_pages) = parts.next().and_then(|p| p.parse::<u64>().ok()) {
                #[cfg(unix)]
                let page_size_bytes = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 };
                #[cfg(not(unix))]
                let page_size_bytes = 4096u64;
                return Some((rss_pages * page_size_bytes) as f64 / (1024.0 * 1024.0));
            }
        }
        None
    }

    pub fn check_health(host: &str, port: u16) -> bool {
        let addr = format!("{host}:{port}");
        if let Ok(mut stream) = TcpStream::connect_timeout(
            &addr.parse().unwrap_or_else(|_| "127.0.0.1:8080".parse().unwrap()),
            Duration::from_millis(400),
        ) {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
            let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
            let req = format!("GET /health HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
            if stream.write_all(req.as_bytes()).is_ok() {
                let mut buf = [0u8; 512];
                if let Ok(n) = stream.read(&mut buf) {
                    let resp = String::from_utf8_lossy(&buf[..n]);
                    return resp.contains("200 OK") || resp.contains(r#""status":"ok""#);
                }
            }
        }
        false
    }

    pub fn get_status() -> RuntimeStatus {
        let cfg = AiConfig::load();
        let state_path = Self::state_file_path();

        if state_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&state_path) {
                if let Ok(state) = serde_json::from_str::<RuntimeStateFile>(&content) {
                    if Self::is_pid_alive(state.pid) {
                        let ram = Self::get_process_rss_mb(state.pid);
                        return RuntimeStatus {
                            is_running: true,
                            pid: Some(state.pid),
                            model_name: Some(state.model_name),
                            port: state.port,
                            api_url: format!("http://{}:{}", cfg.host, state.port),
                            ram_usage_mb: ram,
                            started_at: Some(state.started_at),
                        };
                    }
                }
            }
            let _ = std::fs::remove_file(&state_path);
        }

        // Fallback: check if an orphan llama-server is running
        if let Ok(output) = std::process::Command::new("pgrep")
            .arg("-x")
            .arg("llama-server")
            .output()
        {
            if let Ok(txt) = String::from_utf8(output.stdout) {
                if let Some(first_pid) = txt.lines().next().and_then(|p| p.parse::<u32>().ok()) {
                    let ram = Self::get_process_rss_mb(first_pid);
                    return RuntimeStatus {
                        is_running: true,
                        pid: Some(first_pid),
                        model_name: Some("unknown".into()),
                        port: cfg.port,
                        api_url: format!("http://{}:{}", cfg.host, cfg.port),
                        ram_usage_mb: ram,
                        started_at: None,
                    };
                }
            }
        }

        RuntimeStatus {
            is_running: false,
            pid: None,
            model_name: None,
            port: cfg.port,
            api_url: format!("http://{}:{}", cfg.host, cfg.port),
            ram_usage_mb: None,
            started_at: None,
        }
    }

    pub fn start(model_identifier: Option<&str>) -> Result<RuntimeStatus, String> {
        let status = Self::get_status();
        if status.is_running {
            return Ok(status);
        }

        let cfg = AiConfig::load();

        // 1. Locate model
        let model = if let Some(m_name) = model_identifier {
            ModelStore::find_model(m_name)
                .ok_or_else(|| format!("Model not found: '{m_name}' in ~/.local/share/msui/ai/models/"))?
        } else if let Some(ref def_name) = cfg.default_model {
            ModelStore::find_model(def_name)
                .ok_or_else(|| format!("Default model not found: '{def_name}'"))?
        } else {
            let available = ModelStore::list_models();
            if let Some(first) = available.into_iter().next() {
                first
            } else {
                return Err(
                    "No GGUF models installed. Place a .gguf model in ~/.local/share/msui/ai/models/".into(),
                );
            }
        };

        // 2. Check memory headroom (model file size in MB + context buffer)
        let est_model_mb = (model.size_bytes / (1024 * 1024)).max(500);
        MemorySentinel::can_start_inference(est_model_mb, cfg.min_free_ram_mb)?;

        // 3. Prepare logging
        let log_p = Self::log_file_path();
        if let Some(parent) = log_p.parent() {
            let _ = std::fs::create_dir_all(parent);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
            }
        }
        #[cfg(unix)]
        let log_file = {
            use std::os::unix::fs::OpenOptionsExt;
            let mut opts = std::fs::OpenOptions::new();
            opts.create(true).append(true).mode(0o600);
            opts.open(&log_p)
                .map_err(|e| format!("Cannot open log file: {e}"))?
        };
        #[cfg(not(unix))]
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_p)
            .map_err(|e| format!("Cannot open log file: {e}"))?;

        // 4. Construct execution arguments
        let mut cmd = std::process::Command::new("llama-server");
        cmd.arg("-m")
            .arg(&model.path)
            .arg("-c")
            .arg(cfg.context_size.to_string())
            .arg("-t")
            .arg(cfg.cpu_threads.to_string())
            .arg("--host")
            .arg(&cfg.host)
            .arg("--port")
            .arg(cfg.port.to_string());

        if cfg.quantize_kv {
            cmd.arg("-ctk").arg("q4_0").arg("-ctv").arg("q4_0");
        }
        if cfg.flash_attention {
            cmd.arg("--flash-attn").arg("on");
        }

        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(log_file.try_clone().map_err(|e| e.to_string())?);
        cmd.stderr(log_file);

        #[cfg(unix)]
        cmd.process_group(0);

        // Detach process
        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn llama-server: {e}. Ensure 'llama-cpp' is installed."))?;

        let pid = child.id();
        let now = Local::now();
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // 5. Save state
        let state = RuntimeStateFile {
            pid,
            model_name: model.name.clone(),
            port: cfg.port,
            started_at: now.format("%Y-%m-%d %H:%M:%S").to_string(),
            last_activity_epoch: epoch,
        };
        if let Ok(serialized) = serde_json::to_string_pretty(&state) {
            let state_p = Self::state_file_path();
            #[cfg(unix)]
            {
                use std::fs::OpenOptions;
                use std::io::Write;
                use std::os::unix::fs::OpenOptionsExt;
                let mut options = OpenOptions::new();
                options.write(true).create(true).truncate(true).mode(0o600);
                if let Ok(mut file) = options.open(&state_p) {
                    let _ = file.write_all(serialized.as_bytes());
                }
            }
            #[cfg(not(unix))]
            {
                let _ = std::fs::write(&state_p, serialized);
            }
        }

        // 6. Wait for health check (max 15 seconds)
        let start_time = Instant::now();
        let timeout = Duration::from_secs(15);
        let mut ready = false;
        std::thread::sleep(Duration::from_millis(150));
        while start_time.elapsed() < timeout {
            if !Self::is_pid_alive(pid) {
                return Err(format!(
                    "llama-server exited unexpectedly. Check logs at: {}",
                    log_p.display()
                ));
            }
            if Self::check_health(&cfg.host, cfg.port) {
                ready = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
        }

        if !ready {
            return Err(format!(
                "llama-server started (PID {pid}) but did not become ready within 15s. Check logs at: {}",
                log_p.display()
            ));
        }

        Ok(Self::get_status())
    }

    pub fn record_activity() {
        let state_path = Self::state_file_path();
        if let Ok(content) = std::fs::read_to_string(&state_path) {
            if let Ok(mut state) = serde_json::from_str::<RuntimeStateFile>(&content) {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                state.last_activity_epoch = now;
                if let Ok(serialized) = serde_json::to_string_pretty(&state) {
                    let _ = std::fs::write(&state_path, serialized);
                }
            }
        }
    }

    pub fn check_and_auto_unload(idle_timeout_mins: u64) -> bool {
        if idle_timeout_mins == 0 {
            return false;
        }
        let state_path = Self::state_file_path();
        if let Ok(content) = std::fs::read_to_string(&state_path) {
            if let Ok(state) = serde_json::from_str::<RuntimeStateFile>(&content) {
                if Self::is_pid_alive(state.pid) && state.last_activity_epoch > 0 {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let elapsed_mins = (now.saturating_sub(state.last_activity_epoch)) / 60;
                    if elapsed_mins >= idle_timeout_mins {
                        tracing::info!("Auto-unloading idle Local AI server (idle for {} mins)", elapsed_mins);
                        let _ = Self::stop();
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn stop() -> Result<(), String> {
        let status = Self::get_status();
        let _ = std::fs::remove_file(Self::state_file_path());

        if let Some(pid) = status.pid {
            #[cfg(unix)]
            unsafe {
                libc::kill(pid as libc::pid_t, libc::SIGTERM);
            }
            #[cfg(not(unix))]
            {
                let _ = std::process::Command::new("kill")
                    .arg("-15")
                    .arg(pid.to_string())
                    .output();
            }

            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(3) {
                if !Self::is_pid_alive(pid) {
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(150));
            }

            // Force kill if not stopped
            #[cfg(unix)]
            unsafe {
                libc::kill(pid as libc::pid_t, libc::SIGKILL);
            }
            #[cfg(not(unix))]
            {
                let _ = std::process::Command::new("kill")
                    .arg("-9")
                    .arg(pid.to_string())
                    .output();
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_status_when_stopped() {
        let status = LlamaRuntime::get_status();
        println!("Runtime status: {:?}", status);
    }
}
