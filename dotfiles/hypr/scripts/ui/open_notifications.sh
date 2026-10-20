#!/usr/bin/env bash
set -euo pipefail

# Trigger msUI native Notification History Center directly
msui notifications 2>/dev/null || swaync-client -t -sw 2>/dev/null || true
