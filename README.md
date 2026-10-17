# msUI

**msUI** is a modern, ultra-lightweight, and memory-safe Wayland desktop environment shell (Top Bar, Application Launcher, and System Monitors) written in pure Rust using the [Iced](https://iced.rs) framework and Wayland layer-shell protocol (`zwlr_layer_shell_v1`).

Built as a high-performance alternative to traditional GObject/GTK shells, **msUI** features zero C widget trees, hardware-accelerated GPU rendering via `wgpu`/Vulkan, sub-30MB RAM footprint at idle, and microsecond IPC latency.

---

## Architecture Overview

msUI is structured as a modular Cargo workspace:

```
msui/
├── Cargo.toml                    # Workspace definition & dependencies
├── config/
│   └── msui.toml                 # Default configuration template
├── crates/
│   ├── msui-core/                # Shared domain logic & system monitors
│   │   ├── compositor/           # Hyprland Unix socket streaming IPC
│   │   ├── monitor/              # Direct /sys (battery) and /proc (CPU/RAM) readers
│   │   ├── desktop/              # XDG .desktop application scanner & icon lookup
│   │   └── config.rs             # TOML configuration parser
│   ├── msui-bar/                 # Wayland Layer Shell status bar
│   │   └── bar.rs                # Top bar with workspaces, clock, metrics & launchers
│   └── msui-launcher/            # Spotlight-style floating launcher overlay
│       └── launcher.rs           # Centered overlay with fuzzy search & keyboard navigation
└── apps/
    └── msui/                     # CLI dispatcher & unified binary
        └── main.rs
```

---

## Features

- **Blazing Fast & Zero Memory Leaks**: Pure Rust with immediate-mode / reactive state handling.
- **Top Status Bar (`msui bar`)**:
  - Wayland `ext-workspace-v1` + Hyprland workspace tags.
  - Formatted live clock & date.
  - Hardware resource monitors (CPU usage %, RAM used/total).
  - Battery capacity and charging status indicator.
  - Quick action launcher buttons.
- **Spotlight Application Launcher (`msui launcher`)**:
  - Modal overlay anchored to `Layer::Overlay` with exclusive keyboard focus.
  - XDG `.desktop` file scanning with icon path resolution.
  - Live fuzzy search filtering (powered by `fuzzy-matcher`).
  - Full keyboard navigation: <kbd>↑</kbd> / <kbd>↓</kbd> arrows to select, <kbd>Enter</kbd> to launch, <kbd>Esc</kbd> to dismiss.
- **Direct System Monitors (Zero Subprocesses)**:
  - Battery: Directly reads `/sys/class/power_supply/BAT*`.
  - System: Non-allocating parser for `/proc/stat` and `/proc/meminfo`.

---

## Building and Installation

### Prerequisites

Ensure you have Rust (edition 2024 / 1.85+) and a Wayland compositor (Hyprland, Sway, River, or COSMIC) running:

```bash
rustc --version
cargo --version
```

### Build

```bash
git clone https://github.com/your-username/msui.git
cd msui
cargo build --release
```

The compiled binary will be located at `target/release/msui`.

---

## Usage

### Run Top Bar

```bash
msui bar
```

### Open Application Launcher

Bind this command to your compositor shortcut (e.g., `SUPER + Space` or `SUPER + D`):

```bash
msui launcher
```

### Hyprland Keybind Example

Add the following to `~/.config/hypr/hyprland.conf`:

```ini
# Start msUI Top Bar on startup
exec-once = msui bar

# Bind launcher to Super + Space
bind = SUPER, Space, exec, msui launcher
```

---

## Configuration

Configuration is loaded from `~/.config/msui/msui.toml` (or `$XDG_CONFIG_HOME/msui/msui.toml`).

A sample configuration is available in `config/msui.toml`:

```toml
[bar]
height = 32
layer = "top"
anchor = "top"
time_format = "%a %b %d  %I:%M %p"
battery_refresh_seconds = 5
sysinfo_refresh_seconds = 2
show_workspaces = true
show_battery = true
show_cpu = true
show_memory = true

[launcher]
width = 680
height = 420
max_results = 8
layer = "overlay"
placeholder = "Search applications..."
terminal = "alacritty"
```

---

## License

MIT License.
