#!/usr/bin/env bash
# ==============================================================================
# msUI Bar Toggle Script (Super + W)
# ==============================================================================
set -euo pipefail

MSUI_BIN="$HOME/.local/bin/msui"
if [ ! -x "$MSUI_BIN" ]; then
    MSUI_BIN="$(command -v msui || echo "msui")"
fi

if pgrep -f "(^|/)msui bar" >/dev/null; then
    pkill -f "(^|/)msui bar" || true
else
    setsid "$MSUI_BIN" bar >/tmp/msui-bar.log 2>&1 &
fi
