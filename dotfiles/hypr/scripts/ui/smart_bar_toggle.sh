#!/usr/bin/env bash
# ==============================================================================
# msUI Bar Toggle Script (Super + W)
# ==============================================================================
set -euo pipefail

MSUI_BIN="$HOME/.local/bin/msui"
if [ ! -x "$MSUI_BIN" ]; then
    MSUI_BIN="$(command -v msui || echo "msui")"
fi

# Toggle top bar via instant IPC (or launch unified daemon if not running)
if ! "$MSUI_BIN" toggle bar >/dev/null 2>&1; then
    setsid "$MSUI_BIN" daemon >/tmp/msui.log 2>&1 &
fi

