# msUI - High-Performance Wayland Desktop Shell

**msUI** is an ultra-lightweight, memory-safe, and visually stunning Wayland desktop environment shell written in pure **Rust (Edition 2024)** using the [Iced](https://iced.rs) GUI framework and Wayland Layer Shell protocol (`zwlr_layer_shell_v1`).

Built as a high-performance, modern replacement for legacy GObject/GTK-based shells (such as ZenithShell, Waybar, Rofi, and SwayNC), **msUI** features zero C widget trees, hardware-accelerated GPU rendering via `wgpu`/Vulkan, sub-25MB idle memory footprint, microsecond IPC latency, and native integration with the **Hyprland** (0.55+ Lua) ecosystem.

---

## Table of Contents

- [Key Highlights](#key-highlights)
- [Architecture & Workspace Structure](#architecture--workspace-structure)
- [Features & Services Overview](#features--services-overview)
  - [1. Status Bar (`msui bar`)](#1-status-bar-msui-bar)
  - [2. Quick Settings Control Center (`msui control`)](#2-quick-settings-control-center-msui-control)
  - [3. Application Launcher (`msui launcher`)](#3-application-launcher-msui-launcher)
  - [4. Clipboard History Manager (`msui clipboard`)](#4-clipboard-history-manager-msui-clipboard)
  - [5. App Drawer & Task Switcher (`msui drawer`)](#5-app-drawer--task-switcher-msui-drawer)
  - [6. Reminders & Countdown Timers (`msui reminder`)](#6-reminders--countdown-timers-msui-reminder)
  - [7. Storage & Removable USB Drives (`msui storage`)](#7-storage--removable-usb-drives-msui-storage)
  - [8. Audio Mixer & Routing (`msui audio`)](#8-audio-mixer--routing-msui-audio)
  - [9. Wi-Fi & Network Manager (`msui wifi`)](#9-wi-fi--network-manager-msui-wifi)
  - [10. Bluetooth Manager (`msui bluetooth`)](#10-bluetooth-manager-msui-bluetooth)
  - [11. Displays & Night Light (`msui display`)](#11-displays--night-light-msui-display)
  - [12. Battery & Power Profiles (`msui battery`)](#12-battery--power-profiles-msui-battery)
  - [13. MPRIS Media Player (`msui media`)](#13-mpris-media-player-msui-media)
  - [14. Wallpaper Daemon & Selector (`msui bg` & `msui wallpaper`)](#14-wallpaper-daemon--selector-msui-bg--msui-wallpaper)
  - [15. Dynamic Theme Engine (`msui theme`)](#15-dynamic-theme-engine-msui-theme)
  - [16. Power & Session Menu (`msui power`)](#16-power--session-menu-msui-power)
  - [17. Notifications & Toast System (`msui notifications` & `msui toast`)](#17-notifications--toast-system-msui-notifications--msui-toast)
  - [18. On-Screen Display HUD (`msui osd`)](#18-on-screen-display-hud-msui-osd)
- [CLI Reference & Subcommands](#cli-reference--subcommands)
- [Dependencies & Libraries](#dependencies--libraries)
  - [Rust Crates](#rust-crates)
  - [System Packages & Daemons](#system-packages--daemons)
- [Hyprland Integration](#hyprland-integration)
- [Building & Installation](#building--installation)
- [Configuration Reference](#configuration-reference)
- [License](#license)

---

## Key Highlights

- **Pure Rust & Memory Safety**: Zero GObject runtime overhead, zero memory leaks, and fearless multithreading with Tokio.
- **Hardware-Accelerated Rendering**: GPU-rendered with Vulkan/wgpu via Iced for buttery-smooth 120Hz/144Hz animations.
- **Microsecond Direct Telemetry**: Reads directly from Linux kernel interfaces (`/sys/class/power_supply`, `/sys/class/backlight`, `/sys/block`, `/proc/stat`, `/proc/meminfo`) with zero subprocess fork overhead.
- **Pure Rust D-Bus**: Powered by `zbus` for high-throughput, non-blocking asynchronous IPC (MPRIS, Notifications, NetworkManager, Udisks2).
- **Catppuccin Macchiato Design Language**: Cohesive 14px rounded cards, frosted glass layer blur, and rich Nerd Font typography.
- **Live Hot-Reloading**: Top bar dynamically adapts to disk modifications in `~/.config/msui/msui.toml` without restarting.

---

## Architecture & Workspace Structure

`msUI` is architected as a modular Cargo workspace:

```
msui/
├── Cargo.toml                          # Workspace definition & shared dependencies
├── config/
│   └── msui.toml                       # Default configuration template
├── crates/
│   ├── msui-core/                      # Core domain models, background monitors & IPC
│   │   ├── clipboard/                  # Wayland clipboard watcher & JSON cache
│   │   ├── compositor/                 # Hyprland Unix domain socket client
│   │   ├── desktop/                    # XDG .desktop scanner & icon locator
│   │   ├── monitor/                    # Low-overhead hardware & service monitors
│   │   │   ├── audio.rs                # PipeWire / wpctl sink & source controller
│   │   │   ├── battery.rs              # /sys battery diagnostics & power profiles
│   │   │   ├── bluetooth.rs            # bluetoothctl / BlueZ device monitor
│   │   │   ├── brightness.rs           # /sys/class/backlight controller
│   │   │   ├── display.rs              # Hyprland monitor manager & hyprsunset
│   │   │   ├── media.rs                # MPRIS org.mpris.MediaPlayer2 client
│   │   │   ├── network.rs              # NetworkManager D-Bus & nmcli client
│   │   │   ├── osd.rs                  # Volume/brightness OSD state trigger
│   │   │   ├── storage.rs              # Partition, mount, and udisks2 eject logic
│   │   │   └── system.rs               # /proc/stat (CPU) and /proc/meminfo (RAM)
│   │   ├── notification/               # D-Bus org.freedesktop.Notifications daemon
│   │   ├── reminder/                   # Countdown timer manager & alarm alerts
│   │   ├── config.rs                   # TOML configuration parser & persistence
│   │   ├── ipc.rs                      # Unix domain socket IPC client, server & protocol
│   │   ├── polkit.rs                   # Pure Rust D-Bus Polkit 1 authentication agent
│   │   ├── power.rs                    # Systemd session power management
│   │   ├── theme.rs                    # Catppuccin, Tokyo Night, Nord palettes
│   │   └── wallpaper.rs                # Wallpaper directory scanner & switcher
│   ├── msui-bar/                       # Status Bar (Wayland Layer Shell)
│   │   └── bar.rs                      # Top status bar with 22 modular widgets
│   └── msui-launcher/                  # Floating Overlays & Modals
│       ├── app_drawer.rs               # Window task switcher & running app drawer
│       ├── audio_menu.rs               # Audio mixer & device switcher modal
│       ├── battery_menu.rs             # Battery telemetry & power profile selector
│       ├── bluetooth_menu.rs           # Bluetooth device discovery & pairing
│       ├── clipboard_menu.rs           # Spotlight clipboard history modal
│       ├── control_center.rs           # Quick Settings panel & Top Bar Settings tab
│       ├── display_menu.rs             # Multi-monitor scaling & Night Light modal
│       ├── launcher.rs                 # Spotlight application search modal
│       ├── media_menu.rs               # MPRIS music playback overlay
│       ├── notification_center.rs      # Notification history overlay
│       ├── osd_window.rs               # Volume & screen brightness HUD
│       ├── polkit_dialog.rs            # Native Polkit 1 root elevation modal dialog
│       ├── power_menu.rs               # Lock, suspend, reboot, shutdown modal
│       ├── reminder_menu.rs            # Reminders & Pomodoro timer manager modal
│       ├── storage_menu.rs             # Disk drives & USB mount/eject modal
│       ├── theme_menu.rs               # Interactive theme selector modal
│       ├── toast.rs                    # Real-time toast notification popup HUD
│       ├── wallpaper_daemon.rs         # Background Layer Shell wallpaper renderer
│       ├── wallpaper_menu.rs           # Visual wallpaper picker grid
│       └── wifi_menu.rs                # Wi-Fi network scanner & connection modal
└── apps/
    └── msui/                           # Unified CLI dispatcher & entrypoint
        └── src/main.rs
```

---

## Features & Services Overview

### 1. Status Bar (`msui bar`)
- Wayland Top Layer Shell surface (`msui-bar`).
- Displays 22 granular, toggleable modules:
  - **Brand & Workspaces**: msUI brand button, Hyprland workspaces `1..8`.
  - **Hardware Metrics**: CPU percentage (` 9%`), RAM usage (`󰍛 2.3/7.1G`), Battery (`󰁹 87%`), Backlight (`󰃠 15%`), Volume (` 65%`).
  - **Live Badges**: Active Reminders & Pomodoro countdown ticker (`󰥔 24m Deep Work`), Removable USB Drive badge (`󱊞 USB`).
  - **Connectivity Indicators**: Wi-Fi status, Bluetooth status.
  - **Quick Action Buttons**: Wallpaper, Theme, Reminders, Screenshot, Clipboard, Notification counter (`󰂚 14`), Control Center gear, Terminal, Power.
- **Dynamic Hot-Reloading**: Automatically detects file modifications in `~/.config/msui/msui.toml` and updates its visual layout within 1 second.

### 2. Quick Settings Control Center (`msui control`)
- Consolidated control popup with dual-tab interface:
  - **`󰒓 Quick Settings`**: Single-click toggles for Wi-Fi, Bluetooth, Night Light, Do Not Disturb (DND), Audio Volume slider & sink selector, Brightness slider, Power Profiles (`Power Saver`, `Balanced`, `Performance`), System Telemetry, Media Controls, and Reminders summary.
  - **`󰍹 Top Bar Settings`**: Granular toggle switches for every individual widget on the status bar, complete with instant TOML persistence and presets (`Enable All`, `Minimal Bar`).

### 3. Application Launcher (`msui launcher`)
- Centered Spotlight modal overlay anchored to Wayland `Layer::Overlay`.
- Live fuzzy-search across all system and Flatpak `.desktop` applications (powered by `fuzzy-matcher` Skim algorithm).
- Displays resolved desktop application icons, categories, and executable comments.
- Full keyboard navigation (<kbd>↑</kbd> / <kbd>↓</kbd> / <kbd>Enter</kbd> / <kbd>Esc</kbd>).

### 4. Clipboard History Manager (`msui clipboard`)
- Background clipboard monitoring thread running seamlessly inside `msui bar`.
- Persistent local history cached at `~/.cache/msui/clipboard.json`.
- Live search filtering, metadata timestamping, line-count previews, and one-click copy to Wayland clipboard.

### 5. App Drawer & Task Switcher (`msui drawer`)
- Dedicated App Drawer & Window Switcher (<kbd>Alt + TAB</kbd>, <kbd>Super + TAB</kbd>, <kbd>Super + K</kbd>).
- Directly streams client windows from Hyprland IPC (`hyprctl clients -j`).
- Shows focused window, workspace numbers, application icons, window titles, and addresses.
- Interactive actions: Focus window, move workspace, or close window.

### 6. Reminders & Countdown Timers (`msui reminder`)
- Reminders and countdown timer manager with live status bar countdown badge.
- Quick timer presets: 5m, 10m, 15m, 25m (Pomodoro), 45m (Focus), 60m (Deep Work), or custom title + minutes.
- Triggers desktop toast notifications and auditory bell cues upon countdown expiry.

### 7. Storage & Removable USB Drives (`msui storage`)
- Auto-detects USB thumb drives, external HDDs/SSDs, SD cards, and internal partitions.
- Udisks2 and `lsblk` integration: Mount, unmount, and safely power off / eject removable media.

### 8. Audio Mixer & Routing (`msui audio`)
- PipeWire / PulseAudio output sink and input source manager.
- Interactive volume percentage sliders, mute toggles, and default sink routing.

### 9. Wi-Fi & Network Manager (`msui wifi`)
- High-speed Wi-Fi network scanner via NetworkManager D-Bus.
- Displays SSID, signal percentage, signal bars, security status, and active connection indicator.
- Interactive password connection prompt and disconnect actions.

### 10. Bluetooth Manager (`msui bluetooth`)
- Bluetooth discovery and management via `bluetoothctl` / BlueZ.
- Discover nearby devices, view battery status of connected peripherals, pair, connect, and disconnect.

### 11. Displays & Night Light (`msui display`)
- Multi-monitor management via Hyprland IPC (resolution, refresh rate, scaling, rotation).
- Integrated `hyprsunset` blue light filter slider (warmth from 3500K to 6500K) with instant toggle.

### 12. Battery & Power Profiles (`msui battery`)
- Direct kernel `/sys/class/power_supply` reader (wattage draw, voltage, design capacity, cycle count, battery health %).
- Power Profiles Daemon integration: Switch between `power-saver`, `balanced`, and `performance`.

### 13. MPRIS Media Player (`msui media`)
- Pure Rust MPRIS D-Bus client (`org.mpris.MediaPlayer2`).
- Track metadata (title, artist, album), live progress seek bar, play/pause, next, and previous buttons.

### 14. Wallpaper Daemon & Selector (`msui bg` & `msui wallpaper`)
- **Native Wallpaper Daemon (`msui bg`)**: Wayland Layer Shell surface rendered at `Layer::Background` on all connected outputs.
- **Wallpaper Selector (`msui wallpaper`)**: Visual grid modal displaying thumbnail previews of wallpapers in `~/Pictures/wallpapers`.

### 15. Dynamic Theme Engine (`msui theme`)
- Interactive theme picker featuring:
  - Catppuccin Macchiato, Catppuccin Mocha, Catppuccin Latte
  - Tokyo Night
  - Nord
  - Gruvbox Dark
  - Dracula
  - Cyberpunk
  - Rose Pine

### 16. Power & Session Menu (`msui power`)
- Clean modal with Lock Screen, Suspend, Hibernate, Reboot, Shutdown, and Logout actions.

### 17. Notifications & Toast System (`msui notifications` & `msui toast`)
- In-process D-Bus notification server implementing `org.freedesktop.Notifications`.
- Real-time toast popups with automatic dismissal timeout.
- Notification History modal with unread badge counter and clear-all action.

### 18. On-Screen Display HUD (`msui osd`)
- Floating volume and screen brightness indicator HUD that triggers upon hardware hotkeys.

### 19. Instantaneous IPC Socket & Modal Manager (`msui toggle` / `msui open` / `msui close`)
- Sub-5ms modal toggling via Unix Domain Socket (`$XDG_RUNTIME_DIR/msui.sock`).
- Re-executing modal commands (`msui launcher`, `msui control`, `msui clipboard`, etc.) cleanly toggles open/close state without redundant process overhead.
- Instant mutual exclusion: opening a new modal cleanly dismisses any active modal without overlapping layer shell surfaces.

### 20. Native Polkit 1 Authentication Agent (`msui polkit`)
- Pure Rust `org.freedesktop.PolicyKit1.AuthenticationAgent` implementation over system D-Bus via `zbus 5`.
- Directly replaces external C++ agents (`hyprpolkitagent`, `polkit-gnome`).
- Beautiful Catppuccin Macchiato elevation dialog with Wayland exclusive keyboard focus, shield indicator, bullet-masked password input, and PAM socket communication (`/run/polkit/agent-helper.socket`).

---

## CLI Reference & Subcommands

The unified `msui` binary can be run interactively or as a non-interactive CLI utility:

| Command | Subcommands / Usage | Description |
|---|---|---|
| `msui bar` | *(default)* | Run the Wayland status bar, IPC socket server & background daemons |
| `msui polkit` | — | Run the native PolicyKit 1 elevation agent daemon |
| `msui polkit-dialog` | `<action> <msg> <user> <cookie>` | Native Polkit elevation modal dialog |
| `msui toggle` | `<target>` | Toggle any modal (launcher, control, clipboard, etc.) via IPC |
| `msui open` | `<target>` | Open or focus modal via IPC |
| `msui close` | — | Dismiss any currently open modal via IPC |
| `msui ping` | — | Test IPC socket daemon connectivity and latency |
| `msui launcher` | — | Open the Spotlight application launcher overlay |
| `msui control` | `[dnd <on\|off\|toggle>]` / `[topbar]` | Open Quick Settings or toggle DND / Top Bar settings |
| `msui clipboard` | — | Open the Spotlight clipboard history manager |
| `msui drawer` | `[list]` / `[focus <addr>]` / `[close <addr>]` | Open App Drawer or manage active windows |
| `msui reminder` | `[list]` / `[add <title> <mins>]` / `[delete <id>]` / `[clear]` | Manage countdown timers and reminders |
| `msui storage` | `[list]` / `[mount <dev>]` / `[unmount <dev>]` / `[eject <dev>]` | Inspect and manage disk drives and USB media |
| `msui audio` | `[sinks]` / `[sources]` / `[set-sink <name>]` / `[volume <0-100>]` / `[mute]` | Audio mixer and PipeWire device switcher |
| `msui wifi` | `[list]` / `[rescan]` / `[connect <SSID> [pass]]` / `[disconnect]` | Wi-Fi network scanner and connection tool |
| `msui bluetooth` | — | Open Bluetooth settings manager overlay |
| `msui display` | `[list]` / `[scale <out> <val>]` / `[rotate <out> <deg>]` / `[nightlight <on\|off\|toggle\|temp>]` | Display resolution, scaling & Night Light controls |
| `msui battery` | `[status]` / `[profile set <power-saver\|balanced\|performance>]` | Battery diagnostics & power profile switcher |
| `msui media` | `[status]` / `[play-pause]` / `[next]` / `[prev]` / `[seek <secs>]` | MPRIS music controller and playback actions |
| `msui wallpaper` | `[cycle]` / `[random]` / `[current]` / `[set <path>]` | Wallpaper grid selector and cycling CLI |
| `msui bg` | — | Run the native Wayland background wallpaper daemon |
| `msui theme` | `[theme-name]` | Open theme picker or switch theme via CLI |
| `msui power` | — | Open the Power & Session menu overlay |
| `msui notifications` | `[clear]` | Open the Notification History Center |
| `msui toast` | — | Run the Toast notification HUD listener |
| `msui osd` | `volume <up\|down\|mute>` / `brightness <up\|down>` | Trigger volume or screen brightness OSD HUD |

---

## Dependencies & Libraries

### Rust Crates

| Crate | Version | Purpose |
|---|---|---|
| `iced` | `0.14` | High-performance reactive GUI framework with `wgpu` backend |
| `iced_runtime` | `0.14` | Runtime task executor and event loop integration |
| `iced_exwlshell` | `git` (waycrate) | Wayland layer-shell protocol binding (`zwlr_layer_shell_v1`) |
| `iced_wayland_subscriber` | `git` (waycrate) | Ext-workspace Wayland protocol event subscriber |
| `wayland-client` | `0.31.15` | Native Wayland protocol IPC bindings |
| `tokio` | `1.43` | Async runtime for clipboard watcher, D-Bus, and timers |
| `zbus` | `5.19` | Pure Rust async D-Bus client & server implementation |
| `serde` / `serde_json` | `1.0` | Serialization and deserialization for config and cache |
| `toml` | `0.8` | TOML configuration file parsing and formatting |
| `chrono` | `0.4` | System clock, date formatting, and countdown calculations |
| `fuzzy-matcher` | `0.3` | Skim fuzzy matching algorithm for launcher search |
| `freedesktop-desktop-entry` | `0.8` | XDG `.desktop` file parser for application discovery |
| `xdg` | `3.0` | XDG Base Directory specification paths resolver |
| `tracing` / `tracing-subscriber` | `0.1` / `0.3` | Structured diagnostics and logging framework |
| `anyhow` / `thiserror` | `1.0` / `2.0` | Idiomatic error handling and error definition |

### System Packages & Daemons

To run all msUI capabilities on Linux (Arch Linux / Fedora / Debian), ensure the following system components are present:

```bash
# Arch Linux
sudo pacman -S --needed \
  wayland \
  libxkbcommon \
  vulkan-icd-loader \
  pipewire \
  wireplumber \
  networkmanager \
  bluez \
  bluez-utils \
  udisks2 \
  brightnessctl \
  hyprsunset \
  ttf-jetbrains-mono-nerd
```

---

## Hyprland Integration

msUI integrates natively into **Hyprland** (0.55+ Lua configuration):

### 1. Autostart (`~/.config/hypr/modules/autostart.lua`)
```lua
hl.on("hyprland.start", function()
    -- Start native Polkit 1 authentication agent
    hl.exec_cmd("pgrep -f '[m]sui polkit' >/dev/null || setsid msui polkit >/tmp/msui-polkit.log 2>&1 &")

    -- Start native wallpaper daemon
    hl.exec_cmd("pgrep -f '[m]sui bg' >/dev/null || setsid msui bg >/tmp/msui-bg.log 2>&1 &")

    -- Start top status bar & background daemons (clipboard, notifications, and IPC socket)
    hl.exec_cmd("pgrep -f '[m]sui bar' >/dev/null || setsid msui bar >/tmp/msui-bar.log 2>&1 &")
end)
```

### 2. Keybindings (`~/.config/hypr/modules/binds.lua`)
```lua
-- Spotlight Launcher & Control Center
hl.bind("SUPER + SPACE", hl.dsp.exec_cmd("msui launcher"))
hl.bind("SUPER + A",     hl.dsp.exec_cmd("msui control"))

-- App Drawer & Task Switcher
hl.bind("ALT + TAB",     hl.dsp.exec_cmd("msui drawer"))
hl.bind("SUPER + TAB",   hl.dsp.exec_cmd("msui drawer"))
hl.bind("SUPER + K",     hl.dsp.exec_cmd("msui drawer"))

-- Productivity & Utilities
hl.bind("SUPER + V",     hl.dsp.exec_cmd("msui clipboard"))
hl.bind("SUPER + R",     hl.dsp.exec_cmd("msui reminder"))
hl.bind("SUPER + N",     hl.dsp.exec_cmd("msui notifications"))
hl.bind("SUPER + W",     hl.dsp.exec_cmd("$HOME/.config/hypr/scripts/ui/smart_bar_toggle.sh"))
hl.bind("SUPER + SHIFT + P", hl.dsp.exec_cmd("msui power"))

-- Hardware & Connectivity
hl.bind("SUPER + I",         hl.dsp.exec_cmd("msui wifi"))
hl.bind("SUPER + CTRL + A",  hl.dsp.exec_cmd("msui audio"))
hl.bind("SUPER + CTRL + B",  hl.dsp.exec_cmd("msui bluetooth"))
hl.bind("SUPER + CTRL + W",  hl.dsp.exec_cmd("msui wifi"))
hl.bind("SUPER + ALT + U",   hl.dsp.exec_cmd("msui theme"))
hl.bind("SUPER + SHIFT + W", hl.dsp.exec_cmd("msui wallpaper cycle"))

-- Audio & Brightness OSD Keys
hl.bind("XF86AudioRaiseVolume", hl.dsp.exec_cmd("msui osd volume up"))
hl.bind("XF86AudioLowerVolume", hl.dsp.exec_cmd("msui osd volume down"))
hl.bind("XF86AudioMute",        hl.dsp.exec_cmd("msui osd volume mute"))
hl.bind("XF86MonBrightnessUp",   hl.dsp.exec_cmd("msui osd brightness up"))
hl.bind("XF86MonBrightnessDown", hl.dsp.exec_cmd("msui osd brightness down"))
```

### 3. Layer Blur Rules (`~/.config/hypr/modules/rules.lua`)
```lua
-- Apply smooth frosted glass blur to all msUI overlays
hl.layer_rule({
    match = {
        namespace = "^(msui-.*)$",
    },
    blur = true,
    ignore_alpha = 0.25,
})
```

---

## Building & Installation

### Requirements
- **Rust Toolchain**: `1.85+` (Rust edition 2024).
- **Cargo**: Standard package manager.

### Compile
```bash
# Clone the repository
git clone https://github.com/your-username/msui.git
cd msui

# Build optimized release binary
cargo build --release
```

### Install
Link or copy the compiled binary into your `$PATH`:

```bash
mkdir -p ~/.local/bin
ln -sf $(pwd)/target/release/msui ~/.local/bin/msui
```

Verify installation:
```bash
msui --version
# Output: msUI v0.1.0
```

---

## Configuration Reference

Configuration is loaded from `~/.config/msui/msui.toml`. If the file does not exist, msUI generates default values automatically.

```toml
[bar]
height = 32
layer = "top"
anchor = "top"
time_format = "%a %b %d  %I:%M %p"
battery_refresh_seconds = 5
sysinfo_refresh_seconds = 2

# Status Bar Module Visibility Toggles (also editable live via msui control -> Top Bar Settings)
show_brand = true
show_workspaces = true
show_media = true
show_clock = true
show_reminders_badge = true
show_network = true
show_audio = true
show_brightness = true
show_cpu = true
show_memory = true
show_storage = true
show_battery = true
show_bluetooth = true
show_wallpaper = true
show_theme = true
show_reminders_btn = true
show_screenshot = true
show_clipboard = true
show_notifications = true
show_control_center = true
show_terminal = true
show_power = true

[launcher]
width = 680
height = 420
max_results = 8
layer = "overlay"
placeholder = "Search applications..."
terminal = "foot"
```

---

## License

This project is licensed under the **MIT License**.
