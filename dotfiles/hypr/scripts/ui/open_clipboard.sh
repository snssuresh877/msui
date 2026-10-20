#!/usr/bin/env bash
set -euo pipefail

# Trigger msUI native Clipboard Manager directly
msui clipboard 2>/dev/null || (cliphist list | rofi -dmenu -p "Clipboard" | cliphist decode | wl-copy 2>/dev/null)
