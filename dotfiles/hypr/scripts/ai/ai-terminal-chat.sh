#!/usr/bin/env bash
# ==============================================================================
# msUI AI Interactive Terminal Chat
# ==============================================================================
# Runs inside floating foot window, connected to local Ollama inference engine.
# Features:
#   - ArtCraft / VectorCraft shortcuts and tips
#   - Memory cleanup prompt upon exiting
# ==============================================================================

set -uo pipefail

MODEL="qwen2.5-coder:1.5b"
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
echo -e "${PURPLE}║${RESET}  ${CYAN}${BOLD}🤖 msUI Local AI Assistant${RESET} ${GRAY}(On-Demand, 0 MB Boot RAM)${RESET}             ${PURPLE}║${RESET}"
echo -e "${PURPLE}╠══════════════════════════════════════════════════════════════════════╣${RESET}"
echo -e "${PURPLE}║${RESET}  ${GREEN}• Model:${RESET}    ${BOLD}${MODEL}${RESET} (Qwen 2.5 Coder 1.5B)                           ${PURPLE}║${RESET}"
echo -e "${PURPLE}║${RESET}  ${GREEN}• Engine:${RESET}   http://127.0.0.1:11434 (OpenAI-compatible)               ${PURPLE}║${RESET}"
echo -e "${PURPLE}║${RESET}  ${GREEN}• ArtCraft:${RESET} Ready for VectorCraft MCP & JSON-RPC automation           ${PURPLE}║${RESET}"
echo -e "${PURPLE}║${RESET}  ${YELLOW}• Commands:${RESET} Type ${BOLD}/bye${RESET} or ${BOLD}Ctrl+D${RESET} to finish chat session                 ${PURPLE}║${RESET}"
echo -e "${PURPLE}╚══════════════════════════════════════════════════════════════════════╝${RESET}"
echo ""

# Run the model interactively
ollama run "${MODEL}"

echo ""
echo -e "${CYAN}Chat session closed.${RESET}"
echo -n -e "Stop local AI engine to free RAM to 0 MB? [Y/n] (Auto-stops in 5s): "
read -t 5 -r choice || choice="y"

if [[ "${choice,,}" =~ ^(n|no)$ ]]; then
    echo -e "${GREEN}Engine remains active in background.${RESET}"
    sleep 1
else
    echo -e "${YELLOW}Freeing RAM and stopping engine...${RESET}"
    "${SCRIPT_DIR}/ai-control.sh" stop
    sleep 0.8
fi
