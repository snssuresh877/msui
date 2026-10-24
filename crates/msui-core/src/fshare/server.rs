use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use super::web_assets::{render_download_page, render_upload_page};
use super::{sanitize_filename, FShareEvent};

#[derive(Debug, Clone)]
pub struct FShareServerHandle {
    pub local_ip: String,
    pub port: u16,
    pub token: String,
    pub pin: String,
    shutdown_trigger: Arc<AtomicBool>,
}

impl FShareServerHandle {
    pub fn stop(&self) {
        self.shutdown_trigger.store(true, Ordering::SeqCst);
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
    /// Start server for Send mode (hosting a specific file)
    pub async fn start_send(
        file_path: PathBuf,
        local_ip: String,
        token: String,
        pin: String,
        event_tx: mpsc::UnboundedSender<FShareEvent>,
    ) -> anyhow::Result<FShareServerHandle> {
        if !file_path.exists() {
            anyhow::bail!("File not found: {:?}", file_path);
        }

        let listener = bind_listener().await?;
        let port = listener.local_addr()?.port();
        let shutdown = Arc::new(AtomicBool::new(false));

        let handle = FShareServerHandle {
            local_ip: local_ip.clone(),
            port,
            token: token.clone(),
            pin: pin.clone(),
            shutdown_trigger: shutdown.clone(),
        };

        let share_url = format!("http://{}:{}/d/{}", local_ip, port, token);
        let _ = event_tx.send(FShareEvent::Started {
            url: share_url,
            pin: pin.clone(),
        });

        let file_path_clone = file_path.clone();
        let token_clone = token.clone();
        let shutdown_loop = shutdown.clone();

        tokio::spawn(async move {
            info!("FShare Send server listening on 0.0.0.0:{} (accessible at {}:{})", port, local_ip, port);

            loop {
                if shutdown_loop.load(Ordering::SeqCst) {
                    break;
                }

                tokio::select! {
                    accept_res = listener.accept() => {
                        match accept_res {
                            Ok((stream, peer_addr)) => {
                                let peer_ip = peer_addr.ip().to_string();
                                let file_path = file_path_clone.clone();
                                let token = token_clone.clone();
                                let event_tx = event_tx.clone();

                                tokio::spawn(async move {
                                    let _ = event_tx.send(FShareEvent::ClientConnected { peer_ip });
                                    if let Err(e) = handle_send_connection(stream, &file_path, &token, event_tx.clone()).await {
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
        let shutdown = Arc::new(AtomicBool::new(false));

        let handle = FShareServerHandle {
            local_ip: local_ip.clone(),
            port,
            token: token.clone(),
            pin: pin.clone(),
            shutdown_trigger: shutdown.clone(),
        };

        let share_url = format!("http://{}:{}/r/{}", local_ip, port, token);
        let _ = event_tx.send(FShareEvent::Started {
            url: share_url,
            pin: pin.clone(),
        });

        let save_dir_clone = save_dir.clone();
        let token_clone = token.clone();
        let shutdown_loop = shutdown.clone();

        tokio::spawn(async move {
            info!("FShare Receive server listening on {}:{}", local_ip, port);

            loop {
                if shutdown_loop.load(Ordering::SeqCst) {
                    break;
                }

                tokio::select! {
                    accept_res = listener.accept() => {
                        match accept_res {
                            Ok((stream, peer_addr)) => {
                                let peer_ip = peer_addr.ip().to_string();
                                let save_dir = save_dir_clone.clone();
                                let token = token_clone.clone();
                                let event_tx = event_tx.clone();

                                tokio::spawn(async move {
                                    let _ = event_tx.send(FShareEvent::ClientConnected { peer_ip });
                                    if let Err(e) = handle_recv_connection(stream, &save_dir, &token, event_tx).await {
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

async fn handle_send_connection(
    mut stream: TcpStream,
    file_path: &Path,
    token: &str,
    event_tx: mpsc::UnboundedSender<FShareEvent>,
) -> anyhow::Result<()> {
    let mut header_buf = vec![0u8; 4096];
    let n = stream.read(&mut header_buf).await?;
    if n == 0 {
        return Ok(());
    }

    let request_str = String::from_utf8_lossy(&header_buf[..n]);
    let first_line = request_str.lines().next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");

    let path_clean = path.split('?').next().unwrap_or("").trim_end_matches('/');

    if method == "OPTIONS" {
        let resp = "HTTP/1.1 204 No Content\r\n\
                    Access-Control-Allow-Origin: *\r\n\
                    Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\n\
                    Access-Control-Allow-Headers: *\r\n\
                    Content-Length: 0\r\n\
                    Connection: close\r\n\r\n";
        stream.write_all(resp.as_bytes()).await?;
        return Ok(());
    }

    if method != "GET" && method != "HEAD" {
        send_404(&mut stream).await?;
        return Ok(());
    }

    let filename = file_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());

    if path_clean == "/favicon.ico" {
        let resp = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        stream.write_all(resp.as_bytes()).await?;
        return Ok(());
    }

    let download_page_path = format!("/d/{}", token);
    let download_file_prefix = format!("/dl/{}/", token);

    if path_clean == download_page_path {
        let size = std::fs::metadata(file_path)?.len();
        let dl_url = format!("/dl/{}/{}", token, urlencoding_encode(&filename));
        let html = render_download_page(&filename, size, &dl_url);
        if method == "HEAD" {
            let header = format!(
                "HTTP/1.1 200 OK\r\n\
                Content-Type: text/html; charset=utf-8\r\n\
                Content-Length: {}\r\n\
                Access-Control-Allow-Origin: *\r\n\
                Connection: close\r\n\r\n",
                html.len()
            );
            stream.write_all(header.as_bytes()).await?;
        } else {
            send_html(&mut stream, &html).await?;
        }
    } else if path.starts_with(&download_file_prefix) {
        let metadata = tokio::fs::metadata(file_path).await?;
        let total_bytes = metadata.len();
        let mut file = tokio::fs::File::open(file_path).await?;

        let header = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Type: application/octet-stream\r\n\
            Content-Disposition: attachment; filename=\"{}\"; filename*=UTF-8''{}\r\n\
            Content-Length: {}\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Accept-Ranges: none\r\n\
            Connection: close\r\n\r\n",
            filename, urlencoding_encode(&filename), total_bytes
        );
        stream.write_all(header.as_bytes()).await?;
        if method == "HEAD" {
            return Ok(());
        }

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
            filename,
            size_bytes: total_bytes,
        });
    } else {
        send_404(&mut stream).await?;
    }

    Ok(())
}

async fn handle_recv_connection(
    mut stream: TcpStream,
    save_dir: &Path,
    token: &str,
    event_tx: mpsc::UnboundedSender<FShareEvent>,
) -> anyhow::Result<()> {
    let mut reader = BufReader::new(&mut stream);
    let mut header_lines = Vec::new();

    // Read HTTP headers line-by-line
    let mut content_length: u64 = 0;
    let mut raw_filename = String::new();
    let mut method = String::new();
    let mut path = String::new();

    let mut line_buf = String::new();
    while let Ok(bytes) = reader.read_line(&mut line_buf).await {
        if bytes == 0 || line_buf == "\r\n" || line_buf == "\n" {
            break;
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
            }
        }
        header_lines.push(line);
        line_buf.clear();
    }

    let upload_page_path = format!("/r/{}", token);
    let upload_endpoint = format!("/upload/{}", token);
    let path_clean = path.split('?').next().unwrap_or("").trim_end_matches('/');

    if method == "OPTIONS" && (path_clean == upload_endpoint || path.starts_with(&upload_endpoint)) {
        let resp = "HTTP/1.1 204 No Content\r\n\
                    Access-Control-Allow-Origin: *\r\n\
                    Access-Control-Allow-Methods: POST, GET, OPTIONS\r\n\
                    Access-Control-Allow-Headers: *\r\n\
                    Content-Length: 0\r\n\
                    Connection: close\r\n\r\n";
        stream.write_all(resp.as_bytes()).await?;
        return Ok(());
    }

    if path_clean == "/favicon.ico" {
        let resp = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        stream.write_all(resp.as_bytes()).await?;
        return Ok(());
    }

    if (method == "GET" || method == "HEAD") && path_clean == upload_page_path {
        let html = render_upload_page(&upload_endpoint);
        if method == "HEAD" {
            let header = format!(
                "HTTP/1.1 200 OK\r\n\
                Content-Type: text/html; charset=utf-8\r\n\
                Content-Length: {}\r\n\
                Access-Control-Allow-Origin: *\r\n\
                Connection: close\r\n\r\n",
                html.len()
            );
            stream.write_all(header.as_bytes()).await?;
        } else {
            send_html(&mut stream, &html).await?;
        }
    } else if method == "POST" && (path_clean == upload_endpoint || path.starts_with(&upload_endpoint)) {
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
            let read_bytes = reader.read(&mut buffer[..to_read]).await?;
            if read_bytes == 0 {
                break;
            }
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
    } else {
        send_404(&mut stream).await?;
    }

    Ok(())
}

async fn send_html(stream: &mut TcpStream, html: &str) -> anyhow::Result<()> {
    let bytes = html.as_bytes();
    let header = format!(
        "HTTP/1.1 200 OK\r\n\
        Content-Type: text/html; charset=utf-8\r\n\
        Content-Length: {}\r\n\
        Connection: close\r\n\r\n",
        bytes.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(bytes).await?;
    stream.flush().await?;
    Ok(())
}

async fn send_404(stream: &mut TcpStream) -> anyhow::Result<()> {
    let body = b"404 Not Found";
    let header = format!(
        "HTTP/1.1 404 Not Found\r\n\
        Content-Type: text/plain\r\n\
        Content-Length: {}\r\n\
        Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await?;
    Ok(())
}

fn urlencoding_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
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
            test_file.clone(),
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
            test_file.clone(),
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

        // 4. Test OPTIONS preflight on receive server
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
        let _ = std::fs::remove_dir_all(&test_dir);
    }
}

