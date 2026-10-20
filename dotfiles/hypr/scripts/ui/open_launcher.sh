#!/usr/bin/env bash
set -euo pipefail

# Trigger msUI native Spotlight launcher directly
msui launcher 2>/dev/null || rofi -show drun
