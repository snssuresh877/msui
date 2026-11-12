pub fn render_download_page(filename: &str, size_bytes: u64, dl_path: &str) -> String {
    let size_mb = size_bytes as f64 / (1024.0 * 1024.0);
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
    <meta name="color-scheme" content="dark">
    <meta name="theme-color" content="#11111b">
    <title>msUI FShare - {filename}</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        html, body {{
            width: 100%;
            height: 100%;
            min-height: 100vh;
            min-height: 100dvh;
        }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            background-color: #11111b;
            color: #cdd6f4;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 16px;
        }}
        .card {{
            background-color: #1e1e2e;
            border: 1px solid #313244;
            border-radius: 20px;
            padding: 36px 24px;
            width: 100%;
            max-width: 440px;
            text-align: center;
            box-shadow: 0 16px 40px rgba(0,0,0,0.5);
            margin: auto;
        }}
        .badge {{
            display: inline-block;
            background: rgba(137, 180, 250, 0.15);
            color: #89b4fa;
            padding: 6px 14px;
            border-radius: 12px;
            font-size: 13px;
            font-weight: 600;
            margin-bottom: 20px;
            letter-spacing: 0.5px;
        }}
        .icon {{
            font-size: 52px;
            margin-bottom: 16px;
        }}
        h2 {{
            font-size: 20px;
            color: #ffffff;
            margin-bottom: 8px;
            word-break: break-all;
            line-height: 1.4;
        }}
        .meta {{
            color: #a6adc8;
            font-size: 15px;
            margin-bottom: 28px;
        }}
        .btn {{
            display: block;
            width: 100%;
            background: #89b4fa;
            color: #11111b;
            text-decoration: none;
            padding: 16px;
            border-radius: 14px;
            font-size: 16px;
            font-weight: 700;
            transition: all 0.2s ease;
            box-shadow: 0 4px 14px rgba(137, 180, 250, 0.3);
            border: none;
            cursor: pointer;
        }}
        .btn:active {{
            transform: scale(0.98);
            background: #b4befe;
        }}
        .footer {{
            margin-top: 24px;
            font-size: 12px;
            color: #6c7086;
        }}
    </style>
</head>
<body>
    <div class="card">
        <span class="badge">msUI FShare • Direct Wi-Fi</span>
        <div class="icon">📦</div>
        <h2>{filename}</h2>
        <p class="meta">Size: {size_mb:.2} MB</p>
        <a href="{dl_path}" class="btn" id="dl-btn" download="{filename}" onclick="handleClick()">Download to Device</a>
        <div class="footer">Transferred directly over local network. No cloud involved.</div>
    </div>
    <script>
        function handleClick() {{
            const btn = document.getElementById('dl-btn');
            btn.innerText = '⏳ Starting Download...';
            btn.style.background = '#a6e3a1';
            setTimeout(() => {{
                btn.innerText = '✓ Download Triggered';
            }}, 2000);
        }}
    </script>
</body>
</html>"##
    )
}

pub fn urlencoding_encode(s: &str) -> String {
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

pub fn render_multi_download_page(
    files: &[(String, u64)],
    total_size_bytes: u64,
    zip_dl_path: &str,
    token: &str,
) -> String {
    let total_size_mb = total_size_bytes as f64 / (1024.0 * 1024.0);
    let count = files.len();

    let mut items_html = String::new();
    for (idx, (name, size)) in files.iter().enumerate() {
        let size_str = if *size < 1024 {
            format!("{} B", size)
        } else if *size < 1024 * 1024 {
            format!("{:.1} KB", *size as f64 / 1024.0)
        } else {
            format!("{:.2} MB", *size as f64 / (1024.0 * 1024.0))
        };

        let encoded_name = urlencoding_encode(name);
        let item_url = format!("/dl/{}/{}/{}", token, idx, encoded_name);

        items_html.push_str(&format!(
            r#"<div class="file-item">
                <div class="file-info">
                    <span class="file-name">{}</span>
                    <span class="file-size">{}</span>
                </div>
                <a href="{}" class="item-dl" download="{}">Download</a>
            </div>"#,
            name, size_str, item_url, name
        ));
    }

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
    <meta name="color-scheme" content="dark">
    <meta name="theme-color" content="#11111b">
    <title>msUI FShare - {count} Files</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        html, body {{
            width: 100%;
            height: 100%;
            min-height: 100vh;
            min-height: 100dvh;
        }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            background-color: #11111b;
            color: #cdd6f4;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 16px;
        }}
        .card {{
            background-color: #1e1e2e;
            border: 1px solid #313244;
            border-radius: 20px;
            padding: 32px 24px;
            width: 100%;
            max-width: 480px;
            text-align: center;
            box-shadow: 0 16px 40px rgba(0,0,0,0.5);
            margin: auto;
        }}
        .badge {{
            display: inline-block;
            background: rgba(137, 180, 250, 0.15);
            color: #89b4fa;
            padding: 6px 14px;
            border-radius: 12px;
            font-size: 13px;
            font-weight: 600;
            margin-bottom: 16px;
            letter-spacing: 0.5px;
        }}
        .icon {{
            font-size: 46px;
            margin-bottom: 12px;
        }}
        h2 {{
            font-size: 20px;
            color: #ffffff;
            margin-bottom: 6px;
        }}
        .meta {{
            color: #a6adc8;
            font-size: 14px;
            margin-bottom: 20px;
        }}
        .btn {{
            display: block;
            width: 100%;
            background: #89b4fa;
            color: #11111b;
            text-decoration: none;
            padding: 14px;
            border-radius: 14px;
            font-size: 15px;
            font-weight: 700;
            transition: all 0.2s ease;
            box-shadow: 0 4px 14px rgba(137, 180, 250, 0.3);
            margin-bottom: 20px;
            border: none;
            cursor: pointer;
        }}
        .btn:active {{
            transform: scale(0.98);
            background: #b4befe;
        }}
        .file-list {{
            background: #181825;
            border: 1px solid #313244;
            border-radius: 14px;
            max-height: 260px;
            overflow-y: auto;
            text-align: left;
            margin-bottom: 16px;
        }}
        .file-item {{
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 12px 14px;
            border-bottom: 1px solid #262738;
        }}
        .file-item:last-child {{
            border-bottom: none;
        }}
        .file-info {{
            display: flex;
            flex-direction: column;
            overflow: hidden;
            padding-right: 10px;
        }}
        .file-name {{
            font-size: 14px;
            color: #cdd6f4;
            font-weight: 500;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }}
        .file-size {{
            font-size: 12px;
            color: #a6adc8;
            margin-top: 2px;
        }}
        .item-dl {{
            background: rgba(137, 180, 250, 0.15);
            color: #89b4fa;
            text-decoration: none;
            font-size: 12px;
            font-weight: 600;
            padding: 6px 12px;
            border-radius: 8px;
            flex-shrink: 0;
            transition: background 0.15s;
        }}
        .item-dl:active {{
            background: rgba(137, 180, 250, 0.35);
        }}
        .footer {{
            margin-top: 14px;
            font-size: 12px;
            color: #6c7086;
        }}
    </style>
</head>
<body>
    <div class="card">
        <span class="badge">msUI FShare • Direct Wi-Fi</span>
        <div class="icon">📦</div>
        <h2>{count} Files Shared</h2>
        <p class="meta">Total size: {total_size_mb:.2} MB</p>
        <a href="{zip_dl_path}" class="btn" id="zip-btn" download="msui_fshare_{count}_files.zip" onclick="handleZipClick()">📥 Download All as Zip</a>
        <div class="file-list">
            {items_html}
        </div>
        <div class="footer">Transferred directly over local network. No cloud involved.</div>
    </div>
    <script>
        function handleZipClick() {{
            const btn = document.getElementById('zip-btn');
            btn.innerText = '⏳ Packaging & Downloading Zip...';
            btn.style.background = '#a6e3a1';
            setTimeout(() => {{
                btn.innerText = '✓ Zip Download Started';
            }}, 2500);
        }}
    </script>
</body>
</html>"##
    )
}

pub fn render_upload_page(upload_path: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
    <meta name="color-scheme" content="dark">
    <meta name="theme-color" content="#11111b">
    <title>msUI FShare - Send to PC</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        html, body {{
            width: 100%;
            height: 100%;
            min-height: 100vh;
            min-height: 100dvh;
        }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            background-color: #11111b;
            color: #cdd6f4;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 16px;
        }}
        .card {{
            background-color: #1e1e2e;
            border: 1px solid #313244;
            border-radius: 20px;
            padding: 36px 24px;
            width: 100%;
            max-width: 440px;
            text-align: center;
            box-shadow: 0 16px 40px rgba(0,0,0,0.5);
            margin: auto;
        }}
        .badge {{
            display: inline-block;
            background: rgba(166, 227, 161, 0.15);
            color: #a6e3a1;
            padding: 6px 14px;
            border-radius: 12px;
            font-size: 13px;
            font-weight: 600;
            margin-bottom: 20px;
        }}
        .icon {{
            font-size: 48px;
            margin-bottom: 12px;
        }}
        h2 {{
            font-size: 20px;
            color: #ffffff;
            margin-bottom: 8px;
        }}
        p.sub {{
            color: #a6adc8;
            font-size: 14px;
            margin-bottom: 24px;
        }}
        .dropzone {{
            border: 2px dashed #45475a;
            border-radius: 16px;
            padding: 28px 16px;
            margin-bottom: 20px;
            position: relative;
            background: rgba(49, 50, 68, 0.3);
            cursor: pointer;
        }}
        .dropzone input[type=file] {{
            position: absolute;
            top: 0; left: 0; width: 100%; height: 100%;
            opacity: 0;
            cursor: pointer;
        }}
        .select-btn {{
            display: inline-block;
            background: #313244;
            color: #cdd6f4;
            padding: 10px 20px;
            border-radius: 10px;
            font-size: 14px;
            font-weight: 600;
            margin-top: 8px;
            pointer-events: none;
        }}
        .btn-upload {{
            display: none;
            width: 100%;
            background: #a6e3a1;
            color: #11111b;
            border: none;
            padding: 16px;
            border-radius: 14px;
            font-size: 16px;
            font-weight: 700;
            cursor: pointer;
            box-shadow: 0 4px 14px rgba(166, 227, 161, 0.3);
        }}
        .btn-upload:active {{ transform: scale(0.98); }}
        #file-list {{
            margin: 16px 0;
            font-size: 13px;
            color: #89b4fa;
            text-align: left;
            max-height: 120px;
            overflow-y: auto;
        }}
        .progress-bar-container {{
            display: none;
            width: 100%;
            background: #313244;
            height: 10px;
            border-radius: 5px;
            overflow: hidden;
            margin: 16px 0;
        }}
        .progress-fill {{
            background: #a6e3a1;
            width: 0%;
            height: 100%;
            transition: width 0.1s linear;
        }}
        #status {{
            margin-top: 14px;
            font-size: 14px;
            font-weight: 600;
            color: #fab387;
        }}
    </style>
</head>
<body>
    <div class="card">
        <span class="badge">msUI FShare • Direct Receive</span>
        <div class="icon">📥</div>
        <h2>Send Files to PC</h2>
        <p class="sub">Files are saved to your PC's Downloads folder</p>

        <div class="dropzone" id="dz">
            <input type="file" id="filePicker" multiple onchange="onFilesSelected()">
            <div>Tap to select photos, videos, or documents</div>
            <div class="select-btn">Choose Files</div>
        </div>

        <div id="file-list"></div>

        <div class="progress-bar-container" id="prog-wrap">
            <div class="progress-fill" id="prog-fill"></div>
        </div>

        <button class="btn-upload" id="uploadBtn" onclick="startUpload()">Upload to PC</button>
        <div id="status"></div>
    </div>

    <script>
        const uploadEndpoint = '{upload_path}';
        let selectedFiles = [];

        function onFilesSelected() {{
            const picker = document.getElementById('filePicker');
            selectedFiles = Array.from(picker.files);
            const list = document.getElementById('file-list');
            list.innerHTML = '';
            if (selectedFiles.length > 0) {{
                selectedFiles.forEach(f => {{
                    const d = document.createElement('div');
                    d.textContent = '• ' + f.name + ' (' + (f.size / (1024*1024)).toFixed(2) + ' MB)';
                    list.appendChild(d);
                }});
                document.getElementById('uploadBtn').style.display = 'block';
                document.getElementById('status').innerText = selectedFiles.length + ' file(s) ready to send.';
            }} else {{
                document.getElementById('uploadBtn').style.display = 'none';
            }}
        }}

        async function startUpload() {{
            if (selectedFiles.length === 0) return;
            const btn = document.getElementById('uploadBtn');
            const status = document.getElementById('status');
            const progWrap = document.getElementById('prog-wrap');
            const progFill = document.getElementById('prog-fill');

            btn.style.display = 'none';
            progWrap.style.display = 'block';

            try {{
                for (let i = 0; i < selectedFiles.length; i++) {{
                    const file = selectedFiles[i];
                    status.style.color = '#fab387';
                    status.innerText = 'Sending ' + (i+1) + '/' + selectedFiles.length + ': ' + file.name + '...';
                    
                    await new Promise((resolve, reject) => {{
                        const xhr = new XMLHttpRequest();
                        xhr.open('POST', uploadEndpoint);
                        xhr.setRequestHeader('X-Filename', encodeURIComponent(file.name));
                        xhr.upload.onprogress = (e) => {{
                            if (e.lengthComputable) {{
                                const pct = Math.round((e.loaded / e.total) * 100);
                                progFill.style.width = pct + '%';
                            }}
                        }};
                        xhr.onload = () => {{
                            if (xhr.status === 200) resolve();
                            else reject(new Error('Upload failed with status ' + xhr.status));
                        }};
                        xhr.onerror = () => reject(new Error('Network error during transfer'));
                        xhr.ontimeout = () => reject(new Error('Transfer timed out'));
                        xhr.send(file);
                    }});
                }}

                progFill.style.width = '100%';
                status.style.color = '#a6e3a1';
                status.innerText = '✅ All ' + selectedFiles.length + ' file(s) sent successfully to PC!';
                selectedFiles = [];
                document.getElementById('file-list').innerHTML = '';
            }} catch (err) {{
                status.style.color = '#f38ba8';
                status.innerText = '❌ ' + (err.message || 'Transfer failed');
                btn.style.display = 'block';
                btn.innerText = 'Retry Upload';
            }}
        }}
    </script>
</body>
</html>"##
    )
}

pub fn render_pin_page(error_msg: Option<&str>) -> String {
    let err_html = if let Some(err) = error_msg {
        format!(
            r#"<div style="background: rgba(243, 139, 168, 0.15); border: 1px solid #f38ba8; color: #f38ba8; padding: 12px; border-radius: 12px; margin-bottom: 20px; font-size: 14px; font-weight: 500;">{}</div>"#,
            err
        )
    } else {
        String::new()
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
    <meta name="color-scheme" content="dark">
    <meta name="theme-color" content="#11111b">
    <title>msUI FShare - PIN Verification</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        html, body {{
            width: 100%;
            height: 100%;
            min-height: 100vh;
            min-height: 100dvh;
        }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            background-color: #11111b;
            color: #cdd6f4;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 16px;
        }}
        .card {{
            background-color: #1e1e2e;
            border: 1px solid #313244;
            border-radius: 20px;
            padding: 36px 24px;
            width: 100%;
            max-width: 420px;
            text-align: center;
            box-shadow: 0 16px 40px rgba(0,0,0,0.5);
            margin: auto;
        }}
        .badge {{
            display: inline-block;
            background: rgba(137, 180, 250, 0.15);
            color: #89b4fa;
            padding: 6px 14px;
            border-radius: 12px;
            font-size: 13px;
            font-weight: 600;
            margin-bottom: 20px;
            letter-spacing: 0.5px;
        }}
        .icon {{
            font-size: 52px;
            margin-bottom: 16px;
        }}
        h2 {{
            font-size: 20px;
            color: #ffffff;
            margin-bottom: 8px;
            line-height: 1.4;
        }}
        .meta {{
            color: #a6adc8;
            font-size: 14px;
            margin-bottom: 24px;
            line-height: 1.5;
        }}
        .pin-input {{
            width: 100%;
            background: #181825;
            border: 2px solid #313244;
            color: #cdd6f4;
            border-radius: 12px;
            padding: 14px;
            font-size: 24px;
            text-align: center;
            letter-spacing: 8px;
            font-weight: 700;
            outline: none;
            margin-bottom: 20px;
            transition: border-color 0.2s;
        }}
        .pin-input:focus {{
            border-color: #89b4fa;
        }}
        .btn {{
            display: block;
            width: 100%;
            background: #89b4fa;
            color: #11111b;
            text-decoration: none;
            padding: 16px;
            border-radius: 14px;
            font-size: 16px;
            font-weight: 700;
            transition: all 0.2s ease;
            box-shadow: 0 4px 14px rgba(137, 180, 250, 0.3);
            border: none;
            cursor: pointer;
        }}
        .btn:active {{
            transform: scale(0.98);
            background: #b4befe;
        }}
        .footer {{
            margin-top: 24px;
            font-size: 12px;
            color: #6c7086;
        }}
    </style>
</head>
<body>
    <div class="card">
        <span class="badge">msUI FShare • Direct Wi-Fi</span>
        <div class="icon">🔒</div>
        <h2>PIN Verification</h2>
        <p class="meta">Enter the 4-digit PIN displayed on your computer screen to access this file share.</p>
        {err_html}
        <form method="GET" action="/">
            <input type="password" inputmode="numeric" pattern="[0-9]*" maxlength="4" name="pin" class="pin-input" placeholder="••••" autofocus required>
            <button type="submit" class="btn">Unlock File Transfer</button>
        </form>
        <div class="footer">Transferred directly over local network. End-to-end local security.</div>
    </div>
</body>
</html>"##
    )
}
