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

# Optimize glibc memory arenas to prevent heap fragmentation in multi-threaded runtime
export MALLOC_ARENA_MAX=2

# Start unified msUI desktop daemon (Bar + Wallpaper + Toasts + Polkit + Clipboard + Tray + IPC)
setsid "$MSUI_BIN" daemon >/tmp/msui.log 2>&1 &

# Record boot verification log
sleep 1.5
{
  date '+%Y-%m-%d %H:%M:%S msUI Desktop Shell Started'
  ps aux | grep "msui" | grep -v grep
} >/tmp/msui-boot.log 2>&1 &
