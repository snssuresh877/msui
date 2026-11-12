use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use super::web_assets::{
    render_download_page, render_multi_download_page, render_pin_page, render_upload_page,
    urlencoding_encode,
};
use super::{generate_token, sanitize_filename, FShareEvent};

#[derive(Debug, Clone)]
pub struct FShareServerHandle {
    pub local_ip: String,
    pub port: u16,
    pub token: String,
    pub pin: String,
    shutdown_flag: Arc<AtomicBool>,
    shutdown_notify: Arc<tokio::sync::Notify>,
}

impl FShareServerHandle {
    pub fn stop(&self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        self.shutdown_notify.notify_waiters();
    }
}

/// Binds to a port allowed by local firewall (ports 43821..=43830), falling back to ephemeral port 0
async fn bind_listener() -> anyhow::Result<TcpListener> {
    for port in 43821..=43830 {
        if let Ok(listener) = TcpListener::bind(format!("0.0.0.0:{}", port)).await {
            info!("FShare micro-server bound to firewall-allowed port {}", port);
            return Ok(listener);
        }
    }
    warn!("All ports 43821-43830 occupied; falling back to ephemeral port.");
    Ok(TcpListener::bind("0.0.0.0:0").await?)
}

pub struct FShareServer;

impl FShareServer {
    /// Start server for Send mode (hosting one or more files)
    pub async fn start_send(
        file_paths: Vec<PathBuf>,
        local_ip: String,
        token: String,
        pin: String,
        event_tx: mpsc::UnboundedSender<FShareEvent>,
    ) -> anyhow::Result<FShareServerHandle> {
        if file_paths.is_empty() {
            anyhow::bail!("No files provided to share");
        }
        for f in &file_paths {
            if !f.exists() {
                anyhow::bail!("File not found: {:?}", f);
            }
        }

        let listener = bind_listener().await?;
        let port = listener.local_addr()?.port();
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let shutdown_notify = Arc::new(tokio::sync::Notify::new());

        let handle = FShareServerHandle {
            local_ip: local_ip.clone(),
            port,
            token: token.clone(),
            pin: pin.clone(),
            shutdown_flag: shutdown_flag.clone(),
            shutdown_notify: shutdown_notify.clone(),
        };

        let share_url = format!("http://{}:{}/d/{}", local_ip, port, token);
        let _ = event_tx.send(FShareEvent::Started {
            url: share_url,
            pin: pin.clone(),
        });

        let file_paths_clone = file_paths.clone();
        let token_clone = token.clone();
        let pin_clone = pin.clone();
        let flag_loop = shutdown_flag.clone();
        let notify_loop = shutdown_notify.clone();

        tokio::spawn(async move {
            info!("FShare Send server listening on 0.0.0.0:{} (accessible at {}:{})", port, local_ip, port);

            loop {
                if flag_loop.load(Ordering::SeqCst) {
                    break;
                }

                tokio::select! {
                    _ = notify_loop.notified() => {
                        info!("FShare Send server received stop signal; closing listener immediately.");
                        break;
                    }
                    accept_res = listener.accept() => {
                        match accept_res {
                            Ok((stream, peer_addr)) => {
                                let peer_ip = peer_addr.ip().to_string();
                                let file_paths = file_paths_clone.clone();
                                let token = token_clone.clone();
                                let pin = pin_clone.clone();
                                let event_tx = event_tx.clone();

                                tokio::spawn(async move {
                                    let _ = event_tx.send(FShareEvent::ClientConnected { peer_ip });
                                    if let Err(e) = handle_send_connection(stream, &file_paths, &token, &pin, event_tx.clone()).await {
                                        error!("FShare send connection error: {:?}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                warn!("FShare accept error: {:?}", e);
                                break;
                            }
                        }
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(300)) => {
                        info!("FShare server idle timeout (5m) reached; shutting down.");
                        let _ = event_tx.send(FShareEvent::Error("Transfer timed out after 5 minutes.".into()));
                        break;
                    }
                }
            }

            info!("FShare Send server closed.");
            let _ = event_tx.send(FShareEvent::Completed);
        });

        Ok(handle)
    }

    /// Start server for Receive mode (accepting incoming uploads)
    pub async fn start_recv(
        save_dir: PathBuf,
        local_ip: String,
        token: String,
        pin: String,
        event_tx: mpsc::UnboundedSender<FShareEvent>,
    ) -> anyhow::Result<FShareServerHandle> {
        std::fs::create_dir_all(&save_dir)?;

        let listener = bind_listener().await?;
        let port = listener.local_addr()?.port();
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let shutdown_notify = Arc::new(tokio::sync::Notify::new());

        let handle = FShareServerHandle {
            local_ip: local_ip.clone(),
            port,
            token: token.clone(),
            pin: pin.clone(),
            shutdown_flag: shutdown_flag.clone(),
            shutdown_notify: shutdown_notify.clone(),
        };

        let share_url = format!("http://{}:{}/r/{}", local_ip, port, token);
        let _ = event_tx.send(FShareEvent::Started {
            url: share_url,
            pin: pin.clone(),
        });

        let save_dir_clone = save_dir.clone();
        let token_clone = token.clone();
        let pin_clone = pin.clone();
        let flag_loop = shutdown_flag.clone();
        let notify_loop = shutdown_notify.clone();

        tokio::spawn(async move {
            info!("FShare Receive server listening on {}:{}", local_ip, port);

            loop {
                if flag_loop.load(Ordering::SeqCst) {
                    break;
                }

                tokio::select! {
                    _ = notify_loop.notified() => {
                        info!("FShare Receive server received stop signal; closing listener immediately.");
                        break;
                    }
                    accept_res = listener.accept() => {
                        match accept_res {
                            Ok((stream, peer_addr)) => {
                                let peer_ip = peer_addr.ip().to_string();
                                let save_dir = save_dir_clone.clone();
                                let token = token_clone.clone();
                                let pin = pin_clone.clone();
                                let event_tx = event_tx.clone();

                                tokio::spawn(async move {
                                    let _ = event_tx.send(FShareEvent::ClientConnected { peer_ip });
                                    if let Err(e) = handle_recv_connection(stream, &save_dir, &token, &pin, event_tx).await {
                                        error!("FShare recv connection error: {:?}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                warn!("FShare accept error: {:?}", e);
                                break;
                            }
                        }
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(600)) => {
                        info!("FShare receive server idle timeout (10m) reached; shutting down.");
                        break;
                    }
                }
            }

            info!("FShare Receive server closed.");
            let _ = event_tx.send(FShareEvent::Completed);
        });

        Ok(handle)
    }
}

async fn stream_file_response(
    stream: &mut TcpStream,
    file_path: &Path,
    filename: &str,
    is_head: bool,
    event_tx: mpsc::UnboundedSender<FShareEvent>,
) -> anyhow::Result<()> {
    let metadata = tokio::fs::metadata(file_path).await?;
    let total_bytes = metadata.len();

    let header = format!(
        "HTTP/1.1 200 OK\r\n\
        Content-Type: application/octet-stream\r\n\
        Content-Disposition: attachment; filename=\"{}\"; filename*=UTF-8''{}\r\n\
        Content-Length: {}\r\n\
        Access-Control-Allow-Origin: *\r\n\
        Cache-Control: no-cache, no-store, must-revalidate\r\n\
        Pragma: no-cache\r\n\
        Expires: 0\r\n\
        Accept-Ranges: none\r\n\
        Connection: close\r\n\r\n",
        filename, urlencoding_encode(filename), total_bytes
    );
    stream.write_all(header.as_bytes()).await?;
    if is_head {
        stream.flush().await?;
        return Ok(());
    }

    let mut file = tokio::fs::File::open(file_path).await?;
    let mut buffer = [0u8; 65536];
    let mut sent: u64 = 0;
    let start_time = std::time::Instant::now();

    loop {
        let read_bytes = file.read(&mut buffer).await?;
        if read_bytes == 0 {
            break;
        }
        stream.write_all(&buffer[..read_bytes]).await?;
        sent += read_bytes as u64;

        let elapsed = start_time.elapsed().as_secs_f32().max(0.001);
        let speed_mbps = (sent as f32 / (1024.0 * 1024.0)) / elapsed;

        let _ = event_tx.send(FShareEvent::Progress {
            transferred_bytes: sent,
            total_bytes,
            speed_mbps,
        });
    }
    stream.flush().await?;

    let _ = event_tx.send(FShareEvent::FileSent {
        filename: filename.to_string(),
        size_bytes: total_bytes,
    });

    Ok(())
}

async fn handle_send_connection(
    mut stream: TcpStream,
    file_paths: &[PathBuf],
    token: &str,
    pin: &str,
    event_tx: mpsc::UnboundedSender<FShareEvent>,
) -> anyhow::Result<()> {
    if file_paths.is_empty() {
        send_404(&mut stream).await?;
        return Ok(());
    }

    // Read full HTTP request headers until \r\n\r\n (prevent unread bytes in TCP buffer triggering RST)
    let mut header_buf = Vec::with_capacity(4096);
    let mut temp = [0u8; 1024];
    loop {
        let read_result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            stream.read(&mut temp),
        )
        .await;

        match read_result {
            Ok(Ok(n)) if n > 0 => {
                header_buf.extend_from_slice(&temp[..n]);
                if header_buf.windows(4).any(|w| w == b"\r\n\r\n") || header_buf.len() >= 8192 {
                    break;
                }
            }
            _ => break,
        }
    }

    if header_buf.is_empty() {
        return Ok(());
    }

    // Detect TLS ClientHello handshake (first byte 0x16, followed by TLS version 0x03)
    if header_buf[0] == 0x16 {
        warn!("Client attempted HTTPS/TLS connection. FShare operates over plain HTTP.");
        return Ok(());
    }

    let request_str = String::from_utf8_lossy(&header_buf);
    let first_line = request_str.lines().next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");

    let mut header_pin: Option<String> = None;
    for line in request_str.lines().skip(1) {
        let lower = line.to_lowercase();
        if let Some(val) = lower.strip_prefix("x-pin:") {
            header_pin = Some(val.trim().to_string());
            break;
        }
    }

    let (path_part, query_part) = match path.split_once('?') {
        Some((p, q)) => (p, q),
        None => (path, ""),
    };
    let path_clean = path_part.trim_end_matches('/');

    let mut query_pin: Option<String> = None;
    for pair in query_part.split('&') {
        if let Some(val) = pair.strip_prefix("pin=") {
            query_pin = Some(urlencoding_decode(val));
            break;
        }
    }
    let provided_pin = query_pin.or(header_pin);

    if method == "OPTIONS" {
        let resp = "HTTP/1.1 204 No Content\r\n\
                    Access-Control-Allow-Origin: *\r\n\
                    Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\n\
                    Access-Control-Allow-Headers: *\r\n\
                    Content-Length: 0\r\n\
                    Connection: close\r\n\r\n";
        stream.write_all(resp.as_bytes()).await?;
        stream.flush().await?;
        return Ok(());
    }

    if method != "GET" && method != "HEAD" {
        send_404(&mut stream).await?;
        return Ok(());
    }

    if path_clean == "/favicon.ico" {
        let resp = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        stream.write_all(resp.as_bytes()).await?;
        stream.flush().await?;
        return Ok(());
    }

    let download_file_prefix = format!("/dl/{}/", token);

    // If request is to download a file or all.zip:
    if let Some(rest) = path_clean.strip_prefix(&download_file_prefix) {
        if rest == "all.zip" {
            let temp_dir = std::env::temp_dir();
            let rand_suffix = generate_token();
            let zip_name = format!("msui_fshare_{}_{}_{}.zip", token, std::process::id(), &rand_suffix[..8]);
            let zip_path = temp_dir.join(&zip_name);

            let mut cmd = std::process::Command::new("zip");
            cmd.arg("-j").arg("-q").arg(&zip_path);
            for f in file_paths {
                cmd.arg(f);
            }

            match cmd.status() {
                Ok(status) if status.success() => {
                    let zip_download_name = format!("msui_fshare_{}_files.zip", file_paths.len());
                    let res = stream_file_response(&mut stream, &zip_path, &zip_download_name, method == "HEAD", event_tx).await;
                    let _ = tokio::fs::remove_file(&zip_path).await;
                    res?;
                }
                _ => {
                    error!("Failed to generate zip bundle for FShare download");
                    let _ = tokio::fs::remove_file(&zip_path).await;
                    send_404(&mut stream).await?;
                }
            }
        } else if let Some((idx_str, _fname_part)) = rest.split_once('/') {
            if let Ok(idx) = idx_str.parse::<usize>() {
                if idx < file_paths.len() {
                    let target_file = &file_paths[idx];
                    let fname = target_file
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| format!("file_{}", idx));
                    stream_file_response(&mut stream, target_file, &fname, method == "HEAD", event_tx).await?;
                    return Ok(());
                }
            }
            send_404(&mut stream).await?;
        } else {
            // Single file or direct match by filename
            let decoded_rest = urlencoding_decode(rest);
            let matched_file = file_paths.iter().find(|f| {
                if let Some(f_name) = f.file_name() {
                    let f_str = f_name.to_string_lossy();
                    f_str == rest || f_str == decoded_rest || urlencoding_encode(&f_str) == rest
                } else {
                    false
                }
            });

            if let Some(f) = matched_file {
                let fname = f.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "file".to_string());
                stream_file_response(&mut stream, f, &fname, method == "HEAD", event_tx).await?;
            } else if file_paths.len() == 1 {
                let f = &file_paths[0];
                let fname = f.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "file".to_string());
                stream_file_response(&mut stream, f, &fname, method == "HEAD", event_tx).await?;
            } else {
                send_404(&mut stream).await?;
            }
        }
    } else if path_clean.starts_with("/dl/") {
        // Enforce strict token checks on all /dl/{token}/... endpoints!
        // Unauthenticated access or wrong token is rejected immediately (no single-file bypass).
        send_404(&mut stream).await?;
    } else {
        let download_page_path = format!("/d/{}", token);
        let download_page_prefix = format!("/d/{}/", token);

        let is_authorized_by_token = path_clean == download_page_path || path_clean.starts_with(&download_page_prefix);
        let is_pin_route = path_clean.is_empty() || path_clean == "/d" || path_clean == "/auth";

        if is_authorized_by_token {
            serve_download_html(&mut stream, file_paths, token, method == "HEAD").await?;
        } else if is_pin_route {
            if let Some(p) = &provided_pin {
                if p == pin {
                    serve_download_html(&mut stream, file_paths, token, method == "HEAD").await?;
                } else {
                    let html = render_pin_page(Some("Invalid PIN. Please check your screen and try again."));
                    send_html_with_status(&mut stream, "401 Unauthorized", &html).await?;
                }
            } else {
                let html = render_pin_page(None);
                send_html_with_status(&mut stream, "200 OK", &html).await?;
            }
        } else {
            send_404(&mut stream).await?;
        }
    }

    Ok(())
}

async fn serve_download_html(
    stream: &mut TcpStream,
    file_paths: &[PathBuf],
    token: &str,
    is_head: bool,
) -> anyhow::Result<()> {
    let is_multi = file_paths.len() > 1;
    let html = if is_multi {
        let mut files_info = Vec::new();
        let mut total_size: u64 = 0;
        for f in file_paths {
            let size = std::fs::metadata(f).map(|m| m.len()).unwrap_or(0);
            let name = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "file".to_string());
            total_size += size;
            files_info.push((name, size));
        }
        let zip_dl_path = format!("/dl/{}/all.zip", token);
        render_multi_download_page(&files_info, total_size, &zip_dl_path, token)
    } else {
        let file_path = &file_paths[0];
        let filename = file_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        let size = std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
        let dl_url = format!("/dl/{}/{}", token, urlencoding_encode(&filename));
        render_download_page(&filename, size, &dl_url)
    };

    if is_head {
        let header = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Type: text/html; charset=utf-8\r\n\
            Content-Length: {}\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Cache-Control: no-cache, no-store, must-revalidate\r\n\
            Connection: close\r\n\r\n",
            html.len()
        );
        stream.write_all(header.as_bytes()).await?;
        stream.flush().await?;
    } else {
        send_html(stream, &html).await?;
    }
    Ok(())
}

async fn serve_upload_html(
    stream: &mut TcpStream,
    upload_endpoint: &str,
    is_head: bool,
) -> anyhow::Result<()> {
    let html = render_upload_page(upload_endpoint);
    if is_head {
        let header = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Type: text/html; charset=utf-8\r\n\
            Content-Length: {}\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Cache-Control: no-cache, no-store, must-revalidate\r\n\
            Connection: close\r\n\r\n",
            html.len()
        );
        stream.write_all(header.as_bytes()).await?;
        stream.flush().await?;
    } else {
        send_html(stream, &html).await?;
    }
    Ok(())
}

async fn handle_recv_connection(
    mut stream: TcpStream,
    save_dir: &Path,
    token: &str,
    pin: &str,
    event_tx: mpsc::UnboundedSender<FShareEvent>,
) -> anyhow::Result<()> {
    let mut reader = BufReader::new(&mut stream);
    let mut header_lines = Vec::new();

    // Read HTTP headers line-by-line with timeout and total length ceiling
    let mut content_length: u64 = 0;
    let mut raw_filename = String::new();
    let mut method = String::new();
    let mut path = String::new();
    let mut header_pin: Option<String> = None;

    let mut line_buf = String::new();
    let mut total_header_bytes: usize = 0;

    loop {
        line_buf.clear();
        let read_future = reader.read_line(&mut line_buf);
        let bytes = match tokio::time::timeout(std::time::Duration::from_secs(5), read_future).await {
            Ok(Ok(n)) => n,
            _ => break,
        };

        if bytes == 0 || line_buf == "\r\n" || line_buf == "\n" {
            break;
        }
        total_header_bytes += bytes;
        if total_header_bytes > 8192 {
            send_404(&mut stream).await?;
            return Ok(());
        }

        let line = line_buf.trim().to_string();
        if header_lines.is_empty() {
            let mut parts = line.split_whitespace();
            method = parts.next().unwrap_or("").to_string();
            path = parts.next().unwrap_or("").to_string();
        } else {
            let lower = line.to_lowercase();
            if lower.starts_with("content-length:") {
                if let Some(val) = line.split(':').nth(1) {
                    content_length = val.trim().parse().unwrap_or(0);
                }
            } else if lower.starts_with("x-filename:") {
                if let Some(val) = line.split(':').nth(1) {
                    raw_filename = urlencoding_decode(val.trim());
                }
            } else if lower.starts_with("x-pin:") {
                if let Some(val) = line.split(':').nth(1) {
                    header_pin = Some(val.trim().to_string());
                }
            }
        }
        header_lines.push(line);
    }

    // DoS protection: reject excessive upload sizes
    const MAX_UPLOAD_SIZE: u64 = 10 * 1024 * 1024 * 1024; // 10 GB limit
    if content_length > MAX_UPLOAD_SIZE {
        let resp = "HTTP/1.1 413 Payload Too Large\r\n\
                    Content-Type: text/plain\r\n\
                    Content-Length: 24\r\n\
                    Connection: close\r\n\r\nPayload Too Large (>10GB)";
        stream.write_all(resp.as_bytes()).await?;
        stream.flush().await?;
        return Ok(());
    }

    let (path_part, query_part) = match path.split_once('?') {
        Some((p, q)) => (p, q),
        None => (path.as_str(), ""),
    };
    let path_clean = path_part.trim_end_matches('/');

    let mut query_pin: Option<String> = None;
    for pair in query_part.split('&') {
        if let Some(val) = pair.strip_prefix("pin=") {
            query_pin = Some(urlencoding_decode(val));
            break;
        }
    }
    let provided_pin = query_pin.or(header_pin);

    let upload_page_path = format!("/r/{}", token);
    let upload_page_prefix = format!("/r/{}/", token);
    let upload_endpoint = format!("/upload/{}", token);

    if method == "OPTIONS" && (path_clean == upload_endpoint || path.starts_with(&upload_endpoint)) {
        let resp = "HTTP/1.1 204 No Content\r\n\
                    Access-Control-Allow-Origin: *\r\n\
                    Access-Control-Allow-Methods: POST, GET, OPTIONS\r\n\
                    Access-Control-Allow-Headers: *\r\n\
                    Content-Length: 0\r\n\
                    Connection: close\r\n\r\n";
        stream.write_all(resp.as_bytes()).await?;
        stream.flush().await?;
        return Ok(());
    }

    if path_clean == "/favicon.ico" {
        let resp = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        stream.write_all(resp.as_bytes()).await?;
        stream.flush().await?;
        return Ok(());
    }

    if method == "POST" && (path_clean == upload_endpoint || path.starts_with(&upload_endpoint)) {
        let clean_name = sanitize_filename(&raw_filename);
        let mut save_path = save_dir.join(&clean_name);

        // Deduplicate filename
        let file_stem = save_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "file".to_string());
        let ext = save_path.extension().map(|s| format!(".{}", s.to_string_lossy())).unwrap_or_default();
        let mut counter = 1;
        while save_path.exists() {
            save_path = save_dir.join(format!("{} ({}){}", file_stem, counter, ext));
            counter += 1;
        }

        // Canonical verification: ensure save_path stays strictly inside save_dir
        let save_dir_canonical = std::fs::canonicalize(save_dir).unwrap_or_else(|_| save_dir.to_path_buf());
        if let Some(parent) = save_path.parent() {
            let parent_canonical = std::fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
            if !parent_canonical.starts_with(&save_dir_canonical) {
                send_404(&mut stream).await?;
                anyhow::bail!("Security violation: Path traversal attempt prevented!");
            }
        }

        let mut out_file = tokio::fs::File::create(&save_path).await?;
        let mut remaining = content_length;
        let mut buffer = [0u8; 65536];
        let mut received: u64 = 0;
        let start_time = std::time::Instant::now();

        while remaining > 0 {
            let to_read = remaining.min(buffer.len() as u64) as usize;
            let read_future = reader.read(&mut buffer[..to_read]);
            let read_bytes = match tokio::time::timeout(std::time::Duration::from_secs(30), read_future).await {
                Ok(Ok(n)) if n > 0 => n,
                _ => {
                    drop(out_file);
                    let _ = tokio::fs::remove_file(&save_path).await;
                    let _ = event_tx.send(FShareEvent::Error(format!("Upload aborted or timed out for '{}'", clean_name)));
                    return Ok(());
                }
            };
            out_file.write_all(&buffer[..read_bytes]).await?;
            remaining -= read_bytes as u64;
            received += read_bytes as u64;

            let elapsed = start_time.elapsed().as_secs_f32().max(0.001);
            let speed_mbps = (received as f32 / (1024.0 * 1024.0)) / elapsed;

            let _ = event_tx.send(FShareEvent::Progress {
                transferred_bytes: received,
                total_bytes: content_length,
                speed_mbps,
            });
        }
        out_file.flush().await?;

        let _ = event_tx.send(FShareEvent::FileReceived {
            filename: clean_name,
            size_bytes: received,
            path: save_path,
        });

        let resp = "HTTP/1.1 200 OK\r\n\
                    Access-Control-Allow-Origin: *\r\n\
                    Content-Type: text/plain\r\n\
                    Content-Length: 2\r\n\
                    Connection: close\r\n\r\nOK";
        stream.write_all(resp.as_bytes()).await?;
        stream.flush().await?;
    } else if method == "POST" && path_clean.starts_with("/upload/") {
        // Enforce strict token checks on all upload endpoints
        send_404(&mut stream).await?;
    } else {
        let is_authorized_by_token = path_clean == upload_page_path || path_clean.starts_with(&upload_page_prefix);
        let is_pin_route = path_clean.is_empty() || path_clean == "/r" || path_clean == "/auth";

        if (method == "GET" || method == "HEAD") && is_authorized_by_token {
            serve_upload_html(&mut stream, &upload_endpoint, method == "HEAD").await?;
        } else if (method == "GET" || method == "HEAD") && is_pin_route {
            if let Some(p) = &provided_pin {
                if p == pin {
                    serve_upload_html(&mut stream, &upload_endpoint, method == "HEAD").await?;
                } else {
                    let html = render_pin_page(Some("Invalid PIN. Please check your screen and try again."));
                    send_html_with_status(&mut stream, "401 Unauthorized", &html).await?;
                }
            } else {
                let html = render_pin_page(None);
                send_html_with_status(&mut stream, "200 OK", &html).await?;
            }
        } else {
            send_404(&mut stream).await?;
        }
    }

    Ok(())
}

async fn send_html_with_status(stream: &mut TcpStream, status: &str, html: &str) -> anyhow::Result<()> {
    let bytes = html.as_bytes();
    let header = format!(
        "HTTP/1.1 {}\r\n\
        Content-Type: text/html; charset=utf-8\r\n\
        Content-Length: {}\r\n\
        Access-Control-Allow-Origin: *\r\n\
        Cache-Control: no-cache, no-store, must-revalidate\r\n\
        Pragma: no-cache\r\n\
        Expires: 0\r\n\
        Connection: close\r\n\r\n",
        status,
        bytes.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(bytes).await?;
    stream.flush().await?;
    Ok(())
}

async fn send_html(stream: &mut TcpStream, html: &str) -> anyhow::Result<()> {
    send_html_with_status(stream, "200 OK", html).await
}

async fn send_404(stream: &mut TcpStream) -> anyhow::Result<()> {
    let body = b"404 Not Found";
    let header = format!(
        "HTTP/1.1 404 Not Found\r\n\
        Content-Type: text/plain\r\n\
        Content-Length: {}\r\n\
        Access-Control-Allow-Origin: *\r\n\
        Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await?;
    Ok(())
}

fn urlencoding_decode(s: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().unwrap_or('0');
            let h2 = chars.next().unwrap_or('0');
            let hex_str = format!("{}{}", h1, h2);
            if let Ok(b) = u8::from_str_radix(&hex_str, 16) {
                bytes.push(b);
            }
        } else if c == '+' {
            bytes.push(b' ');
        } else {
            let mut buf = [0u8; 4];
            let encoded = c.encode_utf8(&mut buf);
            bytes.extend_from_slice(encoded.as_bytes());
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fshare_send_and_download() {
        let test_dir = std::env::temp_dir().join(format!("msui_test_send_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&test_dir);
        let test_file = test_dir.join("test_file.txt");
        let content = b"Hello from msUI FShare automated streaming test!";
        std::fs::write(&test_file, content).unwrap();

        let (tx, mut rx) = mpsc::unbounded_channel();
        let token = "testtoken123".to_string();
        let pin = "1234".to_string();

        let handle = FShareServer::start_send(
            vec![test_file.clone()],
            "127.0.0.1".into(),
            token.clone(),
            pin.clone(),
            tx,
        )
        .await
        .unwrap();

        // Check started event
        if let Some(event) = rx.recv().await {
            match event {
                FShareEvent::Started { url, .. } => {
                    assert!(url.contains(&format!(":{}", handle.port)));
                }
                _ => panic!("Expected Started event"),
            }
        }

        // Test downloading the file directly via raw HTTP GET over TcpStream
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", handle.port)).await.unwrap();
        let request = format!(
            "GET /dl/{}/test_file.txt HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
            token
        );
        stream.write_all(request.as_bytes()).await.unwrap();

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(response_str.contains("HTTP/1.1 200 OK"));
        assert!(response_str.contains("Hello from msUI FShare automated streaming test!"));

        handle.stop();
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_fshare_upload_and_jail_protection() {
        let test_dir = std::env::temp_dir().join(format!("msui_test_recv_{}", std::process::id()));
        let save_dir = test_dir.join("inbox");
        let _ = std::fs::create_dir_all(&save_dir);

        let (tx, _rx) = mpsc::unbounded_channel();
        let token = "recvtoken456".to_string();
        let pin = "5678".to_string();

        let handle = FShareServer::start_recv(
            save_dir.clone(),
            "127.0.0.1".into(),
            token.clone(),
            pin.clone(),
            tx,
        )
        .await
        .unwrap();

        // Test normal upload via raw HTTP POST
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", handle.port)).await.unwrap();
        let payload = b"Important document payload from mobile";
        let request = format!(
            "POST /upload/{} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Filename: my_vacation.jpg\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            token, payload.len()
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        stream.write_all(payload).await.unwrap();

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(response_str.contains("HTTP/1.1 200 OK"));

        let saved_file = save_dir.join("my_vacation.jpg");
        assert!(saved_file.exists());
        let saved_content = std::fs::read(&saved_file).unwrap();
        assert_eq!(saved_content, payload);

        // Test malicious path traversal upload: attempt to write to ../outside.txt
        let mut stream_evil = TcpStream::connect(format!("127.0.0.1:{}", handle.port)).await.unwrap();
        let evil_payload = b"echo pwned";
        let evil_request = format!(
            "POST /upload/{} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Filename: ../../../evil_script.sh\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            token, evil_payload.len()
        );
        stream_evil.write_all(evil_request.as_bytes()).await.unwrap();
        stream_evil.write_all(evil_payload).await.unwrap();

        let mut evil_response = Vec::new();
        stream_evil.read_to_end(&mut evil_response).await.unwrap();

        // Filename MUST be sanitized and kept strictly inside save_dir as "evil_script.sh"
        let evil_target = save_dir.join("evil_script.sh");
        assert!(evil_target.exists());
        assert!(!test_dir.join("evil_script.sh").exists());

        handle.stop();
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_fshare_trailing_slash_and_options_preflight() {
        let test_dir = std::env::temp_dir().join(format!("msui_test_robust_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&test_dir);
        let test_file = test_dir.join("photo.png");
        std::fs::write(&test_file, b"FAKE_PNG_BINARY_DATA").unwrap();

        let (tx, _rx) = mpsc::unbounded_channel();
        let token = "tok999".to_string();
        let pin = "9999".to_string();

        let handle = FShareServer::start_send(
            vec![test_file.clone()],
            "127.0.0.1".into(),
            token.clone(),
            pin.clone(),
            tx,
        )
        .await
        .unwrap();

        // 1. Test trailing slash on download page
        let mut s1 = TcpStream::connect(format!("127.0.0.1:{}", handle.port)).await.unwrap();
        s1.write_all(format!("GET /d/{}/ HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n", token).as_bytes()).await.unwrap();
        let mut resp1 = Vec::new();
        s1.read_to_end(&mut resp1).await.unwrap();
        let s_resp1 = String::from_utf8_lossy(&resp1);
        assert!(s_resp1.contains("HTTP/1.1 200 OK"));
        assert!(s_resp1.contains("msUI FShare"));

        // 2. Test query parameter on download page
        let mut s2 = TcpStream::connect(format!("127.0.0.1:{}", handle.port)).await.unwrap();
        s2.write_all(format!("GET /d/{}?source=qr HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n", token).as_bytes()).await.unwrap();
        let mut resp2 = Vec::new();
        s2.read_to_end(&mut resp2).await.unwrap();
        let s_resp2 = String::from_utf8_lossy(&resp2);
        assert!(s_resp2.contains("HTTP/1.1 200 OK"));

        // 3. Test favicon.ico returns 204 No Content
        let mut s3 = TcpStream::connect(format!("127.0.0.1:{}", handle.port)).await.unwrap();
        s3.write_all(b"GET /favicon.ico HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").await.unwrap();
        let mut resp3 = Vec::new();
        s3.read_to_end(&mut resp3).await.unwrap();
        let s_resp3 = String::from_utf8_lossy(&resp3);
        assert!(s_resp3.contains("HTTP/1.1 204 No Content"));

        handle.stop();

        // 4. Test multi-file send server and zip download
        let test_file2 = test_dir.join("photo2.png");
        std::fs::write(&test_file2, b"FAKE_PNG_BINARY_DATA_2").unwrap();

        let (tx_multi, _rx_multi) = mpsc::unbounded_channel();
        let multi_token = "tokmulti".to_string();
        let multi_handle = FShareServer::start_send(
            vec![test_file.clone(), test_file2.clone()],
            "127.0.0.1".into(),
            multi_token.clone(),
            "5678".into(),
            tx_multi,
        )
        .await
        .unwrap();

        let mut s_multi = TcpStream::connect(format!("127.0.0.1:{}", multi_handle.port)).await.unwrap();
        s_multi.write_all(format!("GET /d/{} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n", multi_token).as_bytes()).await.unwrap();
        let mut resp_multi = Vec::new();
        s_multi.read_to_end(&mut resp_multi).await.unwrap();
        let s_resp_multi = String::from_utf8_lossy(&resp_multi);
        assert!(s_resp_multi.contains("HTTP/1.1 200 OK"));
        assert!(s_resp_multi.contains("2 Files Shared"));
        assert!(s_resp_multi.contains("Download All as Zip"));
        assert!(s_resp_multi.contains("photo.png"));
        assert!(s_resp_multi.contains("photo2.png"));

        // Test downloading one individual file by index
        let mut s_file0 = TcpStream::connect(format!("127.0.0.1:{}", multi_handle.port)).await.unwrap();
        s_file0.write_all(format!("GET /dl/{}/0/photo.png HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n", multi_token).as_bytes()).await.unwrap();
        let mut resp_file0 = Vec::new();
        s_file0.read_to_end(&mut resp_file0).await.unwrap();
        let s_resp_file0 = String::from_utf8_lossy(&resp_file0);
        assert!(s_resp_file0.contains("HTTP/1.1 200 OK"));
        assert!(s_resp_file0.contains("FAKE_PNG_BINARY_DATA"));

        // Test zip download
        let mut s_zip = TcpStream::connect(format!("127.0.0.1:{}", multi_handle.port)).await.unwrap();
        s_zip.write_all(format!("GET /dl/{}/all.zip HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n", multi_token).as_bytes()).await.unwrap();
        let mut resp_zip = Vec::new();
        s_zip.read_to_end(&mut resp_zip).await.unwrap();
        let s_resp_zip = String::from_utf8_lossy(&resp_zip);
        assert!(s_resp_zip.contains("HTTP/1.1 200 OK"));
        assert!(s_resp_zip.contains("Content-Type: application/octet-stream"));

        multi_handle.stop();

        // 5. Test OPTIONS preflight on receive server
        let (tx_recv, _rx_recv) = mpsc::unbounded_channel();
        let recv_handle = FShareServer::start_recv(
            test_dir.clone(),
            "127.0.0.1".into(),
            "recv888".into(),
            "8888".into(),
            tx_recv,
        )
        .await
        .unwrap();

        let mut s_opt = TcpStream::connect(format!("127.0.0.1:{}", recv_handle.port)).await.unwrap();
        s_opt.write_all(b"OPTIONS /upload/recv888 HTTP/1.1\r\nHost: 127.0.0.1\r\nAccess-Control-Request-Method: POST\r\nConnection: close\r\n\r\n").await.unwrap();
        let mut resp_opt = Vec::new();
        s_opt.read_to_end(&mut resp_opt).await.unwrap();
        let s_resp_opt = String::from_utf8_lossy(&resp_opt);
        assert!(s_resp_opt.contains("HTTP/1.1 204 No Content"));
        assert!(s_resp_opt.contains("Access-Control-Allow-Methods"));

        recv_handle.stop();

        // 6. Test universal page routing and caching headers
        let (tx_univ, _rx_univ) = mpsc::unbounded_channel();
        let univ_token = "univtok123".to_string();
        let univ_handle = FShareServer::start_send(
            vec![test_file.clone()],
            "127.0.0.1".into(),
            univ_token.clone(),
            "1111".into(),
            tx_univ,
        )
        .await
        .unwrap();

        // 6. Test universal PIN challenge and PIN-authenticated access
        // Unauthenticated root / should serve PIN challenge and MUST NOT leak secrets or filenames
        let mut s_root = TcpStream::connect(format!("127.0.0.1:{}", univ_handle.port)).await.unwrap();
        s_root.write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").await.unwrap();
        let mut resp_root = Vec::new();
        s_root.read_to_end(&mut resp_root).await.unwrap();
        let s_resp_root = String::from_utf8_lossy(&resp_root);
        assert!(s_resp_root.contains("HTTP/1.1 200 OK"), "Root path / must return 200 OK with PIN challenge");
        assert!(s_resp_root.contains("PIN Verification"), "Root path must display PIN challenge");
        assert!(!s_resp_root.contains("photo.png"), "Unauthenticated request must not leak shared filenames");
        assert!(!s_resp_root.contains("univtok123"), "Unauthenticated request must not leak session token");

        // Entering invalid PIN must return 401 Unauthorized
        let mut s_bad_pin = TcpStream::connect(format!("127.0.0.1:{}", univ_handle.port)).await.unwrap();
        s_bad_pin.write_all(b"GET /?pin=9999 HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").await.unwrap();
        let mut resp_bad = Vec::new();
        s_bad_pin.read_to_end(&mut resp_bad).await.unwrap();
        let s_resp_bad = String::from_utf8_lossy(&resp_bad);
        assert!(s_resp_bad.contains("HTTP/1.1 401 Unauthorized"));
        assert!(s_resp_bad.contains("Invalid PIN"));

        // Entering valid PIN (?pin=1111) must serve download page with shared files
        let mut s_pin = TcpStream::connect(format!("127.0.0.1:{}", univ_handle.port)).await.unwrap();
        s_pin.write_all(b"GET /?pin=1111 HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").await.unwrap();
        let mut resp_pin = Vec::new();
        s_pin.read_to_end(&mut resp_pin).await.unwrap();
        let s_resp_pin = String::from_utf8_lossy(&resp_pin);
        assert!(s_resp_pin.contains("HTTP/1.1 200 OK"));
        assert!(s_resp_pin.contains("photo.png"));

        // Path with valid token /d/{token} must serve download page directly
        let mut s_d = TcpStream::connect(format!("127.0.0.1:{}", univ_handle.port)).await.unwrap();
        s_d.write_all(format!("GET /d/{} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n", univ_token).as_bytes()).await.unwrap();
        let mut resp_d = Vec::new();
        s_d.read_to_end(&mut resp_d).await.unwrap();
        let s_resp_d = String::from_utf8_lossy(&resp_d);
        assert!(s_resp_d.contains("HTTP/1.1 200 OK"), "/d/{token} must return 200 OK");
        assert!(s_resp_d.contains("photo.png"));

        univ_handle.stop();
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_fshare_blocks_unauthorized_single_file_download_and_upload() {
        let test_dir = std::env::temp_dir().join(format!("msui_test_auth_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&test_dir);
        let test_file = test_dir.join("secret_document.pdf");
        std::fs::write(&test_file, b"%PDF-1.4 confidential content").unwrap();

        let (tx, _rx) = mpsc::unbounded_channel();
        let token = "realtoken777".to_string();
        let pin = "7777".to_string();

        let handle = FShareServer::start_send(
            vec![test_file.clone()],
            "127.0.0.1".into(),
            token.clone(),
            pin.clone(),
            tx,
        )
        .await
        .unwrap();

        // 1. Attempting /dl/ with wrong token MUST return 404 (no single file auth bypass!)
        let mut s_bypass = TcpStream::connect(format!("127.0.0.1:{}", handle.port)).await.unwrap();
        s_bypass.write_all(b"GET /dl/wrongtoken/secret_document.pdf HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").await.unwrap();
        let mut resp_bypass = Vec::new();
        s_bypass.read_to_end(&mut resp_bypass).await.unwrap();
        let s_resp_bypass = String::from_utf8_lossy(&resp_bypass);
        assert!(s_resp_bypass.contains("HTTP/1.1 404 Not Found"));
        assert!(!s_resp_bypass.contains("confidential content"));

        // 2. Direct /dl/ or /dl/secret_document.pdf without token prefix MUST return 404
        let mut s_direct = TcpStream::connect(format!("127.0.0.1:{}", handle.port)).await.unwrap();
        s_direct.write_all(b"GET /dl/secret_document.pdf HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").await.unwrap();
        let mut resp_direct = Vec::new();
        s_direct.read_to_end(&mut resp_direct).await.unwrap();
        let s_resp_direct = String::from_utf8_lossy(&resp_direct);
        assert!(s_resp_direct.contains("HTTP/1.1 404 Not Found"));
        assert!(!s_resp_direct.contains("confidential content"));

        // 3. Valid token download MUST succeed
        let mut s_valid = TcpStream::connect(format!("127.0.0.1:{}", handle.port)).await.unwrap();
        s_valid.write_all(format!("GET /dl/{}/secret_document.pdf HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n", token).as_bytes()).await.unwrap();
        let mut resp_valid = Vec::new();
        s_valid.read_to_end(&mut resp_valid).await.unwrap();
        let s_resp_valid = String::from_utf8_lossy(&resp_valid);
        assert!(s_resp_valid.contains("HTTP/1.1 200 OK"));
        assert!(s_resp_valid.contains("%PDF-1.4 confidential content"));

        handle.stop();
        let _ = std::fs::remove_dir_all(&test_dir);
    }
}

