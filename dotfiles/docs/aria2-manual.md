# Aria2 & Aria2c Complete Practical Manual

A battle-tested, copy-paste ready guide for high-speed multi-connection downloading, torrenting, daemon management, and JSON-RPC automation on Arch Linux.

---

## 1. Quick Distinction: `aria2` vs `aria2c`

* **`aria2`**: The package name (`sudo pacman -S aria2`).
* **`aria2c`**: The actual executable command line tool (`/usr/bin/aria2c`). The **"c"** stands for client / command.
* **Why it is 5x–10x faster than standard downloaders (`curl`, `wget`, browser):**
  Aria2 splits a single file into up to 16 parallel segments, downloading them simultaneously from multiple connections or even multiple mirrors, saturating your internet bandwidth.

---

## 2. Two Modes of Operation

```
┌─────────────────────────────────────────────────────────────┐
│                       Aria2 Ecosystem                       │
├──────────────────────────────┬──────────────────────────────┤
│    Mode 1: Interactive CLI   │  Mode 2: Background Daemon   │
│       (aria2c <URL>)         │   (systemd: aria2.service)   │
├──────────────────────────────┼──────────────────────────────┤
│ • Run on-demand in terminal  │ • Runs 24/7 silently (<5MB)  │
│ • One-off file downloads     │ • Listens on port 6800 RPC   │
│ • Torrent downloads in shell │ • Receives browser downloads │
│ • Progress bar in console    │ • Managed via Web UI / Ext   │
└──────────────────────────────┴──────────────────────────────┘
```

---

## 3. Golden Rules & Best Practices

1. **Max Multi-Threading (`-s 16 -x 16`):**
   Always use 16 connections (`-x 16`) and 16 splits (`-s 16`) for maximum speed on large files.
2. **Always Auto-Resume (`-c`):**
   Never re-download interrupted files from 0%. The `-c` flag (`--continue=true`) picks up right where you left off.
3. **Avoid Port 6800 Conflicts in Standalone CLI:**
   If your background `aria2.service` daemon is running, ad-hoc terminal downloads that read `~/.config/aria2/aria2.conf` may complain that port 6800 is already in use. Pass `--enable-rpc=false` to run purely in the terminal:
   ```bash
   aria2c --enable-rpc=false -c -s 16 -x 16 "URL"
   ```
4. **Specify Output Directory (`-d`):**
   By default, aria2 saves to the current working directory unless configured in `aria2.conf` or overridden with `-d ~/Downloads`.

---

## 4. Everyday CLI Commands (Copy-Paste Ready)

### High-Speed Segmented Download (Max Bandwidth)
Downloads the file using 16 parallel chunks with auto-resume:
```bash
aria2c -c -s 16 -x 16 -k 1M -d ~/Downloads "https://example.com/largefile.iso"
```

### Download with Custom Output Filename (`-o`)
```bash
aria2c -c -s 16 -x 16 -d ~/Downloads -o "linux-kernel.tar.xz" "https://example.com/source.tar.xz"
```

### Batch Downloading from a List of URLs (`-i`)
Create a text file `urls.txt` with one URL per line:
```bash
# Download up to 4 files concurrently, each with 16 connections:
aria2c -c -j 4 -s 16 -x 16 -i urls.txt -d ~/Downloads
```

### Resuming Broken / Interrupted Downloads
If a download failed or you closed the terminal, re-run the same command with `-c` in the same directory:
```bash
aria2c -c "https://example.com/huge-archive.zip"
```

### Download Speed Throttling (Bandwidth Limiting)
Prevent aria2 from saturating your entire home network:
```bash
# Cap download speed at 5 MB/s:
aria2c --max-download-limit=5M "https://example.com/file.zip"

# Cap download at 2 MB/s and upload at 500 KB/s:
aria2c --max-download-limit=2M --max-overall-upload-limit=500K "file.torrent"
```

### Download with Custom User-Agent & Referer Headers
Bypasses anti-bot blocking on CDNs and download sites:
```bash
aria2c -c -s 16 -x 16 \
  --user-agent="Mozilla/5.0 (X11; Linux x86_64; rv:130.0) Gecko/20100101 Firefox/130.0" \
  --referer="https://website.com/download-page" \
  "https://cdn.website.com/file.zip"
```

---

## 5. BitTorrent & Magnet Downloads

`aria2c` is a full-featured BitTorrent client with DHT, PEX, and encryption.

### Download from a `.torrent` File
```bash
aria2c -d ~/Downloads "archlinux.iso.torrent"
```

### Download from a Magnet Link
```bash
# Always wrap magnet links in double quotes:
aria2c -d ~/Downloads "magnet:?xt=urn:btih:3b245504cf5f11bbdbe1201cea6a6bf45a004e35&dn=Arch+Linux"
```

### List Files Inside a Torrent (Without Downloading)
```bash
aria2c --show-files "movie.torrent"
```

### Download Only Specific Files from a Torrent
After checking index numbers with `--show-files`:
```bash
# Only download files 1, 3, and 5:
aria2c --select-file=1,3,5 "movie.torrent"
```

### Set Seeding Ratio and Seeding Time Limits
```bash
# Seed until upload ratio reaches 1.0 (100%) or for 30 minutes, then exit:
aria2c --seed-ratio=1.0 --seed-time=30 "file.torrent"
```

---

## 6. Background Daemon & JSON-RPC Management

In **msUI**, aria2 runs silently as a systemd user daemon:

```bash
# Check service status (uses ~4.7 MB RAM)
systemctl --user status aria2.service

# Restart daemon
systemctl --user restart aria2.service

# Stop / Start daemon
systemctl --user stop aria2.service
systemctl --user start aria2.service
```

### Interacting with the Daemon via `curl`

#### 1. Check Version & Active Features
```bash
curl -s http://127.0.0.1:6800/jsonrpc -d '{"jsonrpc":"2.0","id":"q","method":"aria2.getVersion"}'
```

#### 2. Push a New Download to the Background Daemon
```bash
curl -s http://127.0.0.1:6800/jsonrpc -d '{
  "jsonrpc": "2.0",
  "id": "q",
  "method": "aria2.addUri",
  "params": [["https://example.com/file.iso"]]
}'
```

#### 3. View All Currently Active Downloads
```bash
curl -s http://127.0.0.1:6800/jsonrpc -d '{
  "jsonrpc": "2.0",
  "id": "q",
  "method": "aria2.tellActive"
}'
```

#### 4. Pause or Resume All Downloads
```bash
# Pause all:
curl -s http://127.0.0.1:6800/jsonrpc -d '{"jsonrpc":"2.0","id":"q","method":"aria2.pauseAll"}'

# Resume all:
curl -s http://127.0.0.1:6800/jsonrpc -d '{"jsonrpc":"2.0","id":"q","method":"aria2.unpauseAll"}'
```

---

## 7. Web UIs (Graphical Dashboards)

You don't need to use the command line to manage background downloads. You can connect a browser dashboard directly to your local daemon:

1. **AriaNg Web Dashboard:**
   * Open the open-source web UI: **[https://ariang.mayswind.net/](https://ariang.mayswind.net/)**
   * Go to **AriaNg Settings** → **RPC**:
     * **Aria2 RPC Address:** `127.0.0.1`
     * **Aria2 RPC Port:** `6800`
     * **Aria2 RPC Protocol:** `Http`
     * **Aria2 Secret Token:** *(Leave empty)*
   * Status will turn **Connected**! You can pause, reorder, view live speed graphs, and manage files visually.
2. **Firefox Extension (Aria2 Integration):**
   * Right-click any web file in Firefox → Click **Download with Aria2**.
   * It sends the file directly to your daemon and saves it to `~/Downloads/`.

---

## 8. Configuration Reference (`~/.config/aria2/aria2.conf`)

Here is msUI's optimized configuration:

```ini
# Daemon & RPC Settings
enable-rpc=true
rpc-listen-all=false
rpc-listen-port=6800
rpc-allow-origin-all=true

# Speed & Parallelism
max-connection-per-server=16
split=16
min-split-size=1M
max-concurrent-downloads=5

# Directory & Disk Allocation
dir=${HOME}/Downloads
continue=true
file-allocation=falloc

# State & Session Persistence
save-session=${HOME}/.config/aria2/session.dat
save-session-interval=60

# BitTorrent Settings
follow-torrent=mem
enable-dht=true
enable-peer-exchange=true
peer-id-prefix=-TR2770-
user-agent=Transmission/2.77
```

---

## 9. Troubleshooting & FAQ

#### Q: `[ERROR] IPv4 RPC: failed to bind TCP port 6800. Cause: Address already in use`
* **Cause:** The background `aria2.service` is already listening on port 6800. When you run `aria2c URL` in the terminal, it reads `aria2.conf` (which has `enable-rpc=true`) and tries to bind port 6800 again.
* **Fix:** Pass `--enable-rpc=false` to your CLI command:
  ```bash
  aria2c --enable-rpc=false -c "URL"
  ```

#### Q: `No files to download` immediately after running `aria2c`
* **Cause:** If `input-file=~/.config/aria2/session.dat` is specified in `aria2.conf` when `session.dat` is empty, aria2 terminates because it thinks you passed an empty list of tasks.
* **Fix:** Remove `input-file` from `aria2.conf` and only keep `save-session`.

#### Q: How to monitor active downloads in the terminal?
* You can view active tasks using `aria2c` RPC client or a simple one-liner:
  ```bash
  curl -s http://127.0.0.1:6800/jsonrpc -d '{"jsonrpc":"2.0","id":"q","method":"aria2.tellActive"}' | jq .
  ```
