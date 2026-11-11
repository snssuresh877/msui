# Firefox Extensions & Aria2 Download System Setup Guide

Complete reference and setup guide for the bundled Firefox extensions and the background Aria2 RPC service in **msUI**.

---

## 1. Overview & Architecture

msUI includes a zero-friction, pre-configured download and media capture stack designed for maximum speed and minimal resource usage:

```
┌─────────────────────────────────────────────────────────────┐
│                       Firefox Browser                       │
├──────────────────────────────┬──────────────────────────────┤
│      uBlock Origin           │ Blocks ads, trackers, bloat  │
│      The Stream Detector     │ Captures HLS (.m3u8) streams │
│      Aria2 Integration       │ Pure JSON-RPC download hook  │
└──────────────┬───────────────┴──────────────┬───────────────┘
               │                              │
               │ (Stream URL / wl-copy)       │ (JSON-RPC :6800)
               ▼                              ▼
     ┌───────────────────┐          ┌───────────────────┐
     │  dl-stream / CLI  │          │   aria2.service   │
     │  (yt-dlp -N 8)    │          │  (systemd daemon) │
     └─────────┬─────────┘          └─────────┬─────────┘
               │                              │
               ▼                              ▼
     ┌──────────────────────────────────────────────────┐
     │              ~/Downloads/ Directory              │
     └──────────────────────────────────────────────────┘
```

All extension archives (`.xpi`) are stored directly in `dotfiles/firefox/extensions/` and deployed across all Firefox profiles using the `firefox-optimize` utility.

---

## 2. Bundled Extensions

### 1. Aria2 Integration (Alex)
* **XPI Identifier:** `{e2488817-3d73-4013-850d-b66c5e42d505}.xpi`
* **AMO Page:** [Aria2 Integration on Mozilla Add-ons](https://addons.mozilla.org/en-US/firefox/addon/aria2-integration/)
* **Protocol:** Pure JSON-RPC over HTTP/WebSocket.
* **Why this extension?**
  * Unlike legacy extensions (such as InBasic's "Download with aria2"), this extension communicates directly with Aria2's built-in RPC server.
  * **Zero external dependencies:** Requires **NO** NodeJS runtime, **NO** wrapper shell scripts (`install.sh`), and spawns **NO** terminal windows.
* **Default Connection Settings:**
  * **RPC URL:** `http://127.0.0.1:6800/jsonrpc`
  * **Secret Token:** *(Leave blank — local daemon operates securely on localhost)*
  * **Download Directory:** Handled by `~/.config/aria2/aria2.conf` (`~/Downloads`)
* **Usage:**
  * **Right-Click:** Right-click any downloadable link or file → Select **Download with Aria2**.
  * **Auto-Capture:** Open extension Options → Enable **Capture downloads** to intercept standard browser file downloads automatically.

---

### 2. The Stream Detector (by rowrawer)
* **XPI Identifier:** `@m3u8link.xpi`
* **AMO Page:** [The Stream Detector on Mozilla Add-ons](https://addons.mozilla.org/en-US/firefox/addon/hls-stream-detector/)
* **Purpose:** Seamlessly detects HLS (`.m3u8`), MPEG-DASH (`.mpd`), and VOD media streams embedded in web pages.
* **Usage:**
  1. Navigate to any video stream in Firefox and begin playback.
  2. The Stream Detector icon in the toolbar lights up with a badge count.
  3. Click the icon to view detected streams.
  4. Click the copy icon next to the desired resolution to copy the stream URL or yt-dlp command.
  5. In your terminal, run `dl-stream` (which reads directly from your Wayland clipboard) or pass the URL to `yt-dlp -N 8 "<URL>"`.

---

### 3. uBlock Origin (by Raymond Hill)
* **XPI Identifier:** `uBlock0@raymondhill.net.xpi`
* **AMO Page:** [uBlock Origin on Mozilla Add-ons](https://addons.mozilla.org/en-US/firefox/addon/ublock-origin/)
* **Purpose:** High-efficiency, memory-conscious content blocker. Removes advertising, tracking scripts, and cryptominers, drastically reducing Firefox's RAM footprint.

---

## 3. New System Setup (Step-by-Step)

When restoring msUI on a fresh Arch Linux system, follow these steps to set up Firefox and the Aria2 download service:

### Step 1: Install Required System Packages
```bash
sudo pacman -S --needed aria2 firefox yt-dlp ffmpeg wl-clipboard
```

### Step 2: Deploy Configurations & CLI Utilities
If you ran `./install.sh`, configurations and binaries are deployed automatically. To deploy manually:
```bash
# 1. Deploy Aria2 configuration
mkdir -p ~/.config/aria2
cp dotfiles/aria2/aria2.conf ~/.config/aria2/
touch ~/.config/aria2/session.dat

# 2. Deploy systemd user service
mkdir -p ~/.config/systemd/user
cp dotfiles/systemd/user/aria2.service ~/.config/systemd/user/

# 3. Deploy CLI tools
mkdir -p ~/.local/bin
cp dotfiles/bin/* ~/.local/bin/
chmod +x ~/.local/bin/*
```

### Step 3: Enable the Aria2 Background Daemon
```bash
systemctl --user daemon-reload
systemctl --user enable --now aria2.service
```

Verify that the daemon is active and listening:
```bash
# Check service status
systemctl --user status aria2.service

# Test JSON-RPC endpoint
curl -s http://127.0.0.1:6800/jsonrpc -d '{"jsonrpc":"2.0","id":"q","method":"aria2.getVersion"}'
```
Expected output:
```json
{"id":"q","jsonrpc":"2.0","result":{"enabledFeatures":[...],"version":"1.37.0"}}
```

### Step 4: Deploy Extensions to Firefox Profiles
Launch Firefox once so it generates your default profile directory (`~/.mozilla/firefox/`), then close it and run:
```bash
firefox-optimize
```
This automated command:
1. Deploys `user.js` (AMD GPU VA-API hardware acceleration, 70% lower RAM footprint, Fission process caps).
2. Copies all 3 extensions (`Aria2 Integration`, `The Stream Detector`, and `uBlock Origin`) into every profile's `extensions/` directory.
3. Automatically vacuums and reindexes all SQLite databases for fast browser startup.

### Step 5: Verify in Firefox
1. Open Firefox.
2. Navigate to `about:addons` (`Ctrl+Shift+A`) and ensure **Aria2 Integration**, **The Stream Detector**, and **uBlock Origin** are enabled.
3. Click the **Aria2 Integration** toolbar icon — the status indicator will show **connected (green)**.

---

## 4. Configuration Reference

### Aria2 Configuration File (`~/.config/aria2/aria2.conf`)
Key settings tuned for high performance:
```ini
# RPC Daemon Mode
enable-rpc=true
rpc-listen-all=false
rpc-listen-port=6800
rpc-allow-origin-all=true

# Speed & Connections
max-connection-per-server=16
split=16
min-split-size=1M
max-concurrent-downloads=5

# Storage & Session Persistence
dir=${HOME}/Downloads
continue=true
save-session=${HOME}/.config/aria2/session.dat
save-session-interval=60
file-allocation=falloc
```

### Systemd User Unit (`~/.config/systemd/user/aria2.service`)
```ini
[Unit]
Description=Aria2 High-Speed Download Daemon (RPC)
After=network.target

[Service]
Type=simple
ExecStart=/usr/bin/aria2c --conf-path=%h/.config/aria2/aria2.conf
Restart=on-failure
RestartSec=3s

[Install]
WantedBy=default.target
```

---

## 5. Troubleshooting & FAQ

#### Q: Extension says "Connection error" or status is red
* Check if the systemd daemon is active:
  ```bash
  systemctl --user status aria2.service
  ```
* If inactive, restart it:
  ```bash
  systemctl --user restart aria2.service
  ```
* Ensure no conflicting process is hogging port 6800:
  ```bash
  ss -tulpn | grep 6800
  ```

#### Q: "IPv4 RPC: failed to bind TCP port 6800" error when running `aria2c` in terminal
* Standalone `aria2c` CLI commands should not attempt to start another RPC server on port 6800 while the daemon is running.
* To download from CLI without touching the background daemon, pass `--enable-rpc=false`:
  ```bash
  aria2c --enable-rpc=false "URL"
  ```
* Note: `dl-stream` already includes this flag automatically.

#### Q: Firefox warns about unverified add-on
* Official Mozilla signed `.xpi` packages are used. If Firefox prompts on first launch, simply click **Enable Extension**.
