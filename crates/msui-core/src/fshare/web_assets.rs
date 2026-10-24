pub fn render_download_page(filename: &str, size_bytes: u64, dl_path: &str) -> String {
    let size_mb = size_bytes as f64 / (1024.0 * 1024.0);
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>msUI FShare - {filename}</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            background: #11111b;
            color: #cdd6f4;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            padding: 20px;
        }}
        .card {{
            background: #1e1e2e;
            border: 1px solid #313244;
            border-radius: 20px;
            padding: 36px 28px;
            width: 100%;
            max-width: 440px;
            text-align: center;
            box-shadow: 0 16px 40px rgba(0,0,0,0.4);
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
        <a href="{dl_path}" class="btn" download="{filename}">Download to Device</a>
        <div class="footer">Transferred directly over local network. No cloud involved.</div>
    </div>
</body>
</html>"#
    )
}

pub fn render_upload_page(upload_path: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>msUI FShare - Send to PC</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            background: #11111b;
            color: #cdd6f4;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            padding: 20px;
        }}
        .card {{
            background: #1e1e2e;
            border: 1px solid #313244;
            border-radius: 20px;
            padding: 36px 28px;
            width: 100%;
            max-width: 440px;
            text-align: center;
            box-shadow: 0 16px 40px rgba(0,0,0,0.4);
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
</html>"#
    )
}
