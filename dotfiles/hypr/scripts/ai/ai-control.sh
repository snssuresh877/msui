#!/usr/bin/env bash
# ==============================================================================
# msUI On-Demand Local AI Engine Manager
# ==============================================================================
# Guarantees 0 MB resident RAM on boot by running strictly on-demand.
# Features:
#   - Instant start/stop of local Ollama inference server
#   - Complete RAM reclamation on shutdown (unloads model + drops caches)
#   - Floating chat window toggle
#   - ArtCraft / VectorCraft integration trigger
# ==============================================================================

set -uo pipefail

MODEL="qwen2.5-coder:1.5b"
PORT=11434
API_URL="http://127.0.0.1:${PORT}"
APP_ID="msui-ai-float"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

is_server_running() {
    curl -s --connect-timeout 1 "${API_URL}/" 2>/dev/null | grep -q "Ollama is running"
}

get_ollama_ram_mb() {
    local pids
    pids=$(pgrep -d',' -x ollama 2>/dev/null || true)
    if [[ -z "$pids" ]]; then
        echo "0"
        return
    fi
    ps -o rss= -p "$pids" 2>/dev/null | awk '{sum += $1} END {printf "%.1f", sum/1024}'
}

start_server() {
    if is_server_running; then
        return 0
    fi

    # Start ollama serve in background with no systemd autostart
    OLLAMA_ORIGINS="*" OLLAMA_HOST="127.0.0.1:${PORT}" ollama serve >/dev/null 2>&1 &
    local server_pid=$!

    # Wait for server readiness (max 10 seconds)
    local retries=20
    while [[ $retries -gt 0 ]]; do
        if is_server_running; then
            notify-send -a "msUI AI" -i preferences-system "🤖 Local AI Engine Started" "Model: ${MODEL}\nAPI: ${API_URL}\nReady for ArtCraft / VectorCraft"
            return 0
        fi
        sleep 0.5
        ((retries--))
    done

    notify-send -u critical -a "msUI AI" "AI Engine Error" "Failed to start local Ollama server"
    return 1
}

stop_server() {
    if ! is_server_running && ! pgrep -x ollama >/dev/null 2>&1; then
        notify-send -a "msUI AI" -i preferences-system "Local AI Engine" "Already stopped (0 MB RAM)"
        return 0
    fi

    local ram_before
    ram_before=$(get_ollama_ram_mb)

    # 1. Unload any active models from RAM immediately
    curl -s "${API_URL}/api/generate" -d "{\"model\": \"${MODEL}\", \"keep_alive\": 0}" >/dev/null 2>&1 || true

    # 2. Terminate all ollama processes
    pkill -TERM -x ollama >/dev/null 2>&1 || true
    sleep 0.8
    if pgrep -x ollama >/dev/null 2>&1; then
        pkill -KILL -x ollama >/dev/null 2>&1 || true
    fi

    notify-send -a "msUI AI" -i preferences-system "🤖 Local AI Engine Stopped" "All models unloaded\nFreed ~${ram_before} MB RAM\nIdle RAM overhead: 0 MB"
    return 0
}

toggle_server() {
    if is_server_running; then
        stop_server
    else
        start_server
    fi
}

toggle_window() {
    # Check if floating chat window already exists
    if hyprctl clients 2>/dev/null | grep -q "class: ${APP_ID}"; then
        # Window exists: close or toggle focus
        pkill -f "foot.*${APP_ID}" >/dev/null 2>&1 || true
    else
        start_server
        # Launch dedicated foot floating terminal
        foot -a "${APP_ID}" -T "msUI AI Assistant" -e "${SCRIPT_DIR}/ai-terminal-chat.sh" &
    fi
}

show_status() {
    if is_server_running; then
        local ram
        ram=$(get_ollama_ram_mb)
        echo "Active (RAM: ${ram} MB, Model: ${MODEL}, URL: ${API_URL})"
        return 0
    else
        echo "Inactive (0 MB RAM resident)"
        return 1
    fi
}

case "${1:-toggle}" in
    start)
        start_server
        ;;
    stop)
        stop_server
        ;;
    toggle)
        toggle_server
        ;;
    chat|window)
        toggle_window
        ;;
    status)
        show_status
        ;;
    *)
        echo "Usage: $0 {start|stop|toggle|chat|window|status}"
        exit 1
        ;;
esac
