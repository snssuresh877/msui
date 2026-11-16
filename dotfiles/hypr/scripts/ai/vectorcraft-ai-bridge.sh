#!/usr/bin/env bash
# ==============================================================================
# msUI VectorCraft / ArtCraft AI Bridge Launcher
# ==============================================================================
# Connects local AI engine (qwen2.5-coder:1.5b) with VectorCraft desktop.
# Modes:
#   - GUI Auto-Launch: Starts VectorCraft on port 7979 if not running
#   - MCP Server: Starts standard Model Context Protocol server (vectorcraft-cli mcp)
#   - Interactive Prompt: CLI interface to speak design instructions to VectorCraft
# ==============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VC_DIR="${HOME}/Projects/vectorcraft"

if [[ -x "${VC_DIR}/target/release/vectorcraft" ]]; then
    VC_BIN="${VC_DIR}/target/release/vectorcraft"
    VC_CLI="${VC_DIR}/target/release/vectorcraft-cli"
elif command -v vectorcraft &>/dev/null; then
    VC_BIN="$(command -v vectorcraft)"
    VC_CLI="$(command -v vectorcraft-cli)"
else
    echo "❌ Error: vectorcraft binary not found." >&2
    exit 1
fi

# 1. Ensure local AI engine is running
if command -v msui &>/dev/null; then
    msui ai start >/dev/null 2>&1 || true
elif [[ -x "${SCRIPT_DIR}/ai-control.sh" ]]; then
    "${SCRIPT_DIR}/ai-control.sh" start
fi

# 2. Check VectorCraft GUI
if ! ss -tuln | grep -q ":7979 "; then
    echo "VectorCraft control server not detected on port 7979."
    if [[ -x "${VC_BIN}" ]]; then
        echo "Starting VectorCraft with remote control enabled..."
        "${VC_BIN}" --control 7979 &
        sleep 2
    elif [[ -f "${VC_DIR}/Cargo.toml" ]]; then
        echo "Building or running VectorCraft via cargo..."
        cargo run --release -p vectorcraft --manifest-path "${VC_DIR}/Cargo.toml" -- --control 7979 &
        sleep 3
    fi
fi

# 3. Handle arguments
if [[ "${1:-}" == "--mcp" ]]; then
    echo "Starting VectorCraft Model Context Protocol (MCP) server..."
    exec "${VC_CLI}" mcp --connect 127.0.0.1:7979
elif [[ $# -gt 0 ]]; then
    python3 "${SCRIPT_DIR}/vectorcraft-bridge.py" "$@"
else
    # Interactive mode
    echo "=========================================================="
    echo "  🎨 VectorCraft AI Assistant Bridge"
    echo "  Speak natural language vector design instructions."
    echo "  Type 'quit' or 'exit' to exit."
    echo "=========================================================="
    while true; do
        read -r -p "VectorCraft AI > " user_prompt
        [[ -z "${user_prompt}" ]] && continue
        [[ "${user_prompt,,}" =~ ^(exit|quit|q)$ ]] && break
        python3 "${SCRIPT_DIR}/vectorcraft-bridge.py" "${user_prompt}"
        echo ""
    done
fi
