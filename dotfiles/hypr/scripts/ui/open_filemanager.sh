#!/usr/bin/env bash
# ==============================================================================
# 📂 Dynamic High-End File Manager Launcher
# ==============================================================================
# - Launches GUI file manager (flea / thunar / dolphin / cosmic-files) or TUI (yazi / flea)
# - Opens in the focused window's directory if available
# ==============================================================================

set -euo pipefail

export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"

MODE="${1:-gui}"
export EDITOR="${EDITOR:-nvim}"
export VISUAL="${VISUAL:-nvim}"


# Resolve target directory from focused window
TARGET_DIR="$HOME"
if command -v hyprctl >/dev/null 2>&1 && command -v jq >/dev/null 2>&1; then
    ACTIVE_PID=$(hyprctl activewindow -j 2>/dev/null | jq -r '.pid // empty')
    if [ -n "$ACTIVE_PID" ] && [ "$ACTIVE_PID" != "null" ] && [ -d "/proc/$ACTIVE_PID" ]; then
        CHILD_PID=$(pgrep -P "$ACTIVE_PID" 2>/dev/null | tail -n 1 || true)
        if [ -n "$CHILD_PID" ] && [ -d "/proc/$CHILD_PID/cwd" ]; then
            RESOLVED_DIR=$(readlink -f "/proc/$CHILD_PID/cwd" 2>/dev/null || true)
        else
            RESOLVED_DIR=$(readlink -f "/proc/$ACTIVE_PID/cwd" 2>/dev/null || true)
        fi
        if [ -n "$RESOLVED_DIR" ] && [ -d "$RESOLVED_DIR" ]; then
            TARGET_DIR="$RESOLVED_DIR"
        fi
    fi
fi

# TUI launch helper
launch_terminal_app() {
    local cmd=("$@")
    if command -v foot >/dev/null 2>&1; then
        exec foot -D "$TARGET_DIR" -e "${cmd[@]}"
    elif command -v kitty >/dev/null 2>&1; then
        exec kitty --directory "$TARGET_DIR" -e "${cmd[@]}"
    else
        cd "$TARGET_DIR" && exec "${cmd[@]}"
    fi
}

# Optional environment override
if [ -n "${FILEMANAGER:-}" ]; then
    case "$FILEMANAGER" in
        flea)
            command -v flea >/dev/null 2>&1 && exec flea --gui "$TARGET_DIR"
            ;;
        thunar)
            command -v thunar >/dev/null 2>&1 && exec thunar --window "$TARGET_DIR"
            ;;
        yazi)
            command -v yazi >/dev/null 2>&1 && launch_terminal_app yazi "$TARGET_DIR"
            ;;
    esac
fi

# Explicit mode handling
case "$MODE" in
    flea|flea-gui)
        if command -v flea >/dev/null 2>&1; then
            exec flea --gui "$TARGET_DIR"
        fi
        ;;
    flea-tui)
        if command -v flea >/dev/null 2>&1; then
            launch_terminal_app flea --tui "$TARGET_DIR"
        fi
        ;;
    yazi)
        if command -v yazi >/dev/null 2>&1; then
            launch_terminal_app yazi "$TARGET_DIR"
        fi
        ;;
    thunar)
        if command -v thunar >/dev/null 2>&1; then
            exec thunar --window "$TARGET_DIR"
        fi
        ;;
    tui)
        if command -v yazi >/dev/null 2>&1; then
            launch_terminal_app yazi "$TARGET_DIR"
        elif command -v flea >/dev/null 2>&1; then
            launch_terminal_app flea --tui "$TARGET_DIR"
        fi
        ;;
esac

# GUI Mode / Default Cascade
if command -v flea >/dev/null 2>&1; then
    exec flea --gui "$TARGET_DIR"
elif command -v thunar >/dev/null 2>&1; then
    exec thunar --window "$TARGET_DIR"
elif command -v dolphin >/dev/null 2>&1; then
    exec dolphin "$TARGET_DIR"
elif command -v cosmic-files >/dev/null 2>&1; then
    exec cosmic-files "$TARGET_DIR"
elif command -v yazi >/dev/null 2>&1; then
    launch_terminal_app yazi "$TARGET_DIR"
else
    xdg-open "$TARGET_DIR"
fi
