#!/usr/bin/env bash
# ==============================================================================
# msUI AI Interactive Terminal Chat
# ==============================================================================
# Runs inside floating foot window, connected to local llama-server engine.
# Features:
#   - Direct integration with native msUI Local AI (llama-server)
#   - ArtCraft / VectorCraft shortcuts and tips
#   - Memory cleanup prompt upon exiting
# ==============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ANSI Color codes matching msUI theme
CYAN="\033[1;36m"
BLUE="\033[1;34m"
PURPLE="\033[1;35m"
GREEN="\033[1;32m"
YELLOW="\033[1;33m"
GRAY="\033[0;90m"
BOLD="\033[1m"
RESET="\033[0m"

clear
echo -e "${PURPLE}╔══════════════════════════════════════════════════════════════════════╗${RESET}"
echo -e "${PURPLE}║${RESET}  ${CYAN}${BOLD}🤖 msUI Local AI Assistant${RESET} ${GRAY}(Native llama.cpp, 0 MB Boot RAM)${RESET}     ${PURPLE}║${RESET}"
echo -e "${PURPLE}╠══════════════════════════════════════════════════════════════════════╣${RESET}"
echo -e "${PURPLE}║${RESET}  ${GREEN}• Engine:${RESET}   http://127.0.0.1:8080/v1 (OpenAI-compatible)             ${PURPLE}║${RESET}"
echo -e "${PURPLE}║${RESET}  ${GREEN}• Context:${RESET}  2048 tokens, 4-bit KV Cache (Quantized)                   ${PURPLE}║${RESET}"
echo -e "${PURPLE}║${RESET}  ${GREEN}• ArtCraft:${RESET} Ready for VectorCraft MCP & JSON-RPC automation           ${PURPLE}║${RESET}"
echo -e "${PURPLE}║${RESET}  ${YELLOW}• Commands:${RESET} Type ${BOLD}exit${RESET}, ${BOLD}/bye${RESET}, or ${BOLD}Ctrl+D${RESET} to finish chat session            ${PURPLE}║${RESET}"
echo -e "${PURPLE}╚══════════════════════════════════════════════════════════════════════╝${RESET}"
echo ""

# Ensure runtime is started
if ! msui ai status 2>/dev/null | grep -q "RUNNING"; then
    echo -e "${YELLOW}Starting Local AI runtime (llama-server)...${RESET}"
    msui ai start
    echo ""
fi

# Interactive chat loop
while true; do
    echo -ne "${CYAN}${BOLD}msui-ai ❯ ${RESET}"
    if ! IFS= read -r user_input; then
        echo ""
        break
    fi

    trimmed=$(echo "${user_input}" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')
    if [[ -z "${trimmed}" ]]; then
        continue
    fi

    if [[ "${trimmed}" == "/bye" || "${trimmed}" == "exit" || "${trimmed}" == "quit" || "${trimmed}" == ":q" ]]; then
        break
    fi

    if [[ "${trimmed}" == "/status" ]]; then
        msui ai status
        echo ""
        continue
    fi

    if [[ "${trimmed}" == "/clear" ]]; then
        clear
        continue
    fi

    # Query via msui ai chat
    msui ai chat "${user_input}"
    echo ""
done

echo ""
echo -e "${CYAN}Chat session closed.${RESET}"
echo -n -e "Stop local AI engine to free RAM to 0 MB? [Y/n] (Auto-stops in 5s): "
read -t 5 -r choice || choice="y"

if [[ "${choice,,}" =~ ^(n|no)$ ]]; then
    echo -e "${GREEN}Engine remains active in background.${RESET}"
    sleep 1
else
    echo -e "${YELLOW}Freeing RAM and stopping engine...${RESET}"
    msui ai stop
    sleep 0.8
fi
