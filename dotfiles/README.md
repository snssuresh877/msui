# msUI Dotfiles & Desktop Configuration

This directory contains configuration files for the **msUI** desktop environment on Arch Linux / Hyprland.

## Included Configurations

| Directory | Target Location | Description |
|-----------|-----------------|-------------|
| `aria2/` | `~/.config/aria2/` | Aria2 download accelerator configuration |
| `btop/` | `~/.config/btop/` | Btop system monitor theme and settings |
| `fish/` | `~/.config/fish/` | Fish shell functions, completions, and configuration |
| `flux/` | `~/.config/flux/` | Flux-FM modern GTK4/Rust file manager configuration and menus |
| `foot/` | `~/.config/foot/` | Foot terminal emulator configuration |
| `ghostty/` | `~/.config/ghostty/` | Ghostty terminal configuration and themes |
| `htop/` | `~/.config/htop/` | Htop settings and layout |
| `hypr/` | `~/.config/hypr/` | Hyprland compositor settings, keybinds, scripts, and themes |
| `kitty/` | `~/.config/kitty/` | Kitty terminal configuration |
| `msui/` | `~/.config/msui/` | Active runtime msUI bar and module toggles |
| `nvim/` | `~/.config/nvim/` | Neovim Lua setup, plugins, and keymaps |
| `Thunar/` | `~/.config/Thunar/` | Thunar file manager custom actions and accelerators |
| `yazi/` | `~/.config/yazi/` | Yazi terminal file manager (git status, chmod, ripdrag plugins, universal Neovim opener) |
| `systemd/` | `~/.config/systemd/user/` | Systemd user units (`aria2.service` background RPC daemon) |
| `firefox/` | `~/.mozilla/firefox/` | Firefox low-RAM / AMD VA-API profile (`user.js`) & bundled extensions |
| `bin/` | `~/.local/bin/` | Custom CLI utilities (`dl-stream`, `firefox-optimize`, `systemcontrol`, `vconv`) |

## Custom Utilities

| Command | Description |
|---------|-------------|
| `dl-stream` | High-speed HLS (`.m3u8`) & video stream downloader. Automatically grabs stream links from Wayland clipboard and downloads with 8 parallel connections. |
| `firefox-optimize` | One-shot Firefox performance, low-RAM (70% reduction), AMD GPU VA-API, SQLite vacuum, and automated extension installer (`uBlock Origin`, `Stream Detector`, `Aria2 Integration`). |
| `vconv` | Simple video/audio converter & downscaler wrapper for FFmpeg (`vconv in.mp4 720p`, `vconv in.mkv mp4`, `vconv in.mp4 mp3`). |

## Bundled Firefox Extensions

Located in `dotfiles/firefox/extensions/`:
* **Aria2 Integration (Alex)** (`{e2488817-3d73-4013-850d-b66c5e42d505}.xpi`): Pure JSON-RPC browser extension connecting directly to the local `aria2.service` daemon (`http://127.0.0.1:6800/jsonrpc`). Needs **zero** NodeJS runtime or external companion scripts.
* **The Stream Detector** (`@m3u8link.xpi`): Parses and intercepts HLS (`.m3u8`) and DASH stream URLs from web players for one-click downloading with `dl-stream`.
* **uBlock Origin** (`uBlock0@raymondhill.net.xpi`): Lightweight content & ad blocker reducing memory and CPU usage on Wayland.

## Documentation & Guides

- [Firefox Extensions & Aria2 Download System Setup Guide](docs/firefox-extensions-manual.md) — Complete setup guide for browser extensions and background Aria2 RPC daemon. Accessible via terminal with `firefox-optimize --manual`.
- [FFmpeg Complete Practical Manual](docs/ffmpeg-manual.md) — Practical guide for container conversion, resolution downscaling, AMD VA-API hardware acceleration, and audio extraction. Accessible via terminal with `vconv --manual`.
- [yt-dlp Complete Practical Manual](docs/yt-dlp-manual.md) — Comprehensive cheat sheet for stream extraction, formats, audio, and speed tuning. Accessible via terminal with `dl-stream --manual`.

## New System Setup (Quick Restore)

To set up everything on a fresh machine:

```bash
# 1. Clone repository
git clone git@github.com:snssuresh877/msui.git ~/Projects/msui
cd ~/Projects/msui

# 2. Run the automated installer (installs packages, binaries, configs, and systemd units)
./install.sh

# 3. Enable Aria2 background RPC daemon (if not already enabled)
systemctl --user enable --now aria2.service

# 4. Deploy Firefox performance profile & extensions across all profiles
firefox-optimize
```

