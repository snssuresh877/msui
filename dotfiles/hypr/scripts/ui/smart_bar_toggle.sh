#!/usr/bin/env bash
set -euo pipefail

if pgrep -fx "msui bar" >/dev/null; then
    pkill -fx "msui bar"
else
    setsid msui bar >/tmp/msui-bar.log 2>&1 &
fi
