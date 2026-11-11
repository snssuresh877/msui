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
| `bin/` | `~/.local/bin/` | Custom CLI utilities (`dl-stream`, `firefox-optimize`, `systemcontrol`, `vconv`) |

## Custom Utilities

| Command | Description |
|---------|-------------|
| `dl-stream` | High-speed HLS (`.m3u8`) & video stream downloader. Automatically grabs stream links from Wayland clipboard and downloads with 8 parallel connections. |
| `firefox-optimize` | One-shot Firefox performance, low-RAM (70% reduction), AMD GPU VA-API, and SQLite database vacuum optimizer. |
| `vconv` | Simple video/audio converter & downscaler wrapper for FFmpeg (`vconv in.mp4 720p`, `vconv in.mkv mp4`, `vconv in.mp4 mp3`). |

## Documentation & Guides

- [FFmpeg Complete Practical Manual](docs/ffmpeg-manual.md) — Practical guide for container conversion, resolution downscaling, AMD VA-API hardware acceleration, and audio extraction. Accessible via terminal with `vconv --manual`.
- [yt-dlp Complete Practical Manual](docs/yt-dlp-manual.md) — Comprehensive cheat sheet for stream extraction, formats, audio, and speed tuning. Accessible via terminal with `dl-stream --manual`.

## Quick Restore / Installation

To symlink or restore these dotfiles to `~/.config/`:

```bash
# Example symlink command from the repository root:
for dir in dotfiles/*/; do
  target="$HOME/.config/$(basename "$dir")"
  echo "Linking $dir -> $target"
  # ln -sfn "$(realpath "$dir")" "$target"
done
```
