#!/usr/bin/env bash
set -euo pipefail
# Toggle DND natively via msUI
msui control dnd toggle 2>/dev/null || swaync-client -d -sw 2>/dev/null || true
