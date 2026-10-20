#!/usr/bin/env bash
# ==============================================================================
# msUI Session Autostart Daemon Launcher
# ==============================================================================

MSUI_BIN="$HOME/.local/bin/msui"
if [ ! -x "$MSUI_BIN" ]; then
  MSUI_BIN="$(command -v msui || true)"
fi

if [ -z "$MSUI_BIN" ] || [ ! -x "$MSUI_BIN" ]; then
  echo "Error: msui binary not found in $HOME/.local/bin or PATH" >&2
  exit 1
fi

# Ensure clean slate: kill any stale processes
pkill -x msui 2>/dev/null || true
sleep 0.2

# 1. Start PolicyKit Authentication Agent
setsid "$MSUI_BIN" polkit >/tmp/msui-polkit.log 2>&1 &

# 2. Start Wayland Wallpaper Daemon
sleep 0.2
setsid "$MSUI_BIN" bg >/tmp/msui-bg.log 2>&1 &

# 3. Start Wayland Layer Shell Status Bar
sleep 0.3
setsid "$MSUI_BIN" bar >/tmp/msui-bar.log 2>&1 &

# 4. Record boot verification log
sleep 1.5
{
  date '+%Y-%m-%d %H:%M:%S msUI Desktop Shell Started'
  ps aux | grep "msui" | grep -v grep
} >/tmp/msui-boot.log 2>&1 &
