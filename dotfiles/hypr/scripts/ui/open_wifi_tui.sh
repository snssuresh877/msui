#!/usr/bin/env bash
# ==============================================================================
# msUI Wi-Fi Popdown TUI Launcher
# Supports: wifitui, nmtui-connect, impala, nmtui
# ==============================================================================

export PATH="$HOME/.local/bin:$HOME/.cargo/bin:/usr/local/bin:/usr/bin:$PATH"

# 1. Toggle: if an instance is already running, terminate it cleanly and exit
PIDS=$(pgrep -f "msui-wifi-tui" 2>/dev/null)
if [ -n "$PIDS" ]; then
    kill -TERM $PIDS 2>/dev/null
    exit 0
fi

# 2. Determine TUI binary to launch
TUI_CMD=""
if [ -n "$1" ] && command -v "$1" >/dev/null 2>&1; then
    TUI_CMD="$(command -v "$1")"
elif [ -x "$HOME/.local/bin/wifitui" ]; then
    TUI_CMD="$HOME/.local/bin/wifitui"
elif command -v wifitui >/dev/null 2>&1; then
    TUI_CMD="$(command -v wifitui)"
elif command -v nmtui-connect >/dev/null 2>&1; then
    TUI_CMD="$(command -v nmtui-connect)"
elif command -v impala >/dev/null 2>&1; then
    TUI_CMD="$(command -v impala)"
else
    TUI_CMD="$(command -v nmtui || echo "nmtui")"
fi

# 3. Launch inside terminal emulator with msui-wifi-tui class, detached from caller
if command -v foot >/dev/null 2>&1; then
    setsid -f foot --app-id=msui-wifi-tui -T "msui-wifi-tui" -e "$TUI_CMD" </dev/null >/dev/null 2>&1
elif command -v kitty >/dev/null 2>&1; then
    setsid -f kitty --class=msui-wifi-tui -T "msui-wifi-tui" -e "$TUI_CMD" </dev/null >/dev/null 2>&1
else
    setsid -f xterm -class msui-wifi-tui -e "$TUI_CMD" </dev/null >/dev/null 2>&1
fi

exit 0
