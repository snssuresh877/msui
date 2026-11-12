#!/usr/bin/env bash
# ==============================================================================
# msUI Desktop - Automated UFW Firewall Setup & Hardening Script
# ==============================================================================
# Configures a hardened, workstation-grade firewall for Arch Linux / Hyprland.
# - Default INCOMING: DROP (blocks unsolicited scans and public attacks)
# - Default OUTGOING: ALLOW (web browsing, updates, downloads)
# - Default ROUTED:   DROP (prevents unwanted packet routing)
# - SSH: Rate-limited protection on port 22
# - File Sharing: Restricts msui-fshare (43821-43830) & LocalSend (53317) to LAN
# - Closes unneeded web ports (80/443) to prevent accidental server exposure
# ==============================================================================

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

info() {
  printf "${BLUE}${BOLD}[*]${NC} %s\n" "$1"
}

success() {
  printf "${GREEN}${BOLD}[✔]${NC} %s\n" "$1"
}

warn() {
  printf "${YELLOW}${BOLD}[!]${NC} %s\n" "$1"
}

error() {
  printf "${RED}${BOLD}[✘]${NC} %s\n" "$1" >&2
}

# Determine privilege elevation method (sudo or pkexec)
run_privileged() {
  if [[ $EUID -eq 0 ]]; then
    "$@"
  elif command -v sudo >/dev/null 2>&1 && sudo -n true 2>/dev/null; then
    sudo "$@"
  elif command -v pkexec >/dev/null 2>&1; then
    pkexec "$@"
  elif command -v sudo >/dev/null 2>&1; then
    sudo "$@"
  else
    error "Privilege elevation required. Please run as root or install sudo/pkexec."
    exit 1
  fi
}

# ------------------------------------------------------------------------------
# 1. Package & Service Verification
# ------------------------------------------------------------------------------
verify_ufw_installed() {
  if ! command -v ufw >/dev/null 2>&1; then
    info "UFW not found. Installing via pacman..."
    if command -v pacman >/dev/null 2>&1; then
      run_privileged pacman -S --needed --noconfirm ufw
      success "UFW package installed."
    else
      error "Pacman not found. Please install 'ufw' manually."
      exit 1
    fi
  else
    info "UFW package already installed."
  fi

  info "Ensuring ufw.service is enabled and running..."
  if command -v systemctl >/dev/null 2>&1; then
    run_privileged systemctl enable --now ufw.service 2>/dev/null || true
  fi
}

# ------------------------------------------------------------------------------
# 2. Detect Local Network Subnet
# ------------------------------------------------------------------------------
detect_lan_subnet() {
  local default_iface
  default_iface="$(ip route 2>/dev/null | awk '/default/ {print $5; exit}' || true)"

  if [[ -n "$default_iface" ]]; then
    local ip_cidr
    ip_cidr="$(ip -o -f inet addr show "$default_iface" 2>/dev/null | awk '{print $4; exit}' || true)"

    if [[ -n "$ip_cidr" ]]; then
      # Calculate network CIDR via python if available
      if command -v python3 >/dev/null 2>&1; then
        local net
        net="$(python3 -c "import ipaddress; print(ipaddress.IPv4Interface('$ip_cidr').network)" 2>/dev/null || true)"
        if [[ -n "$net" ]]; then
          echo "$net"
          return 0
        fi
      fi

      # Fallback: calculate /24 from IP
      local raw_ip="${ip_cidr%%/*}"
      echo "$raw_ip" | awk -F. '{printf "%d.%d.%d.0/24\n", $1, $2, $3}'
      return 0
    fi
  fi

  # Default fallback if offline or interface not yet configured
  echo "192.168.1.0/24"
}

# ------------------------------------------------------------------------------
# 3. Apply Hardened Firewall Rules
# ------------------------------------------------------------------------------
apply_firewall_rules() {
  local lan_subnet
  lan_subnet="$(detect_lan_subnet)"

  info "Detected local network subnet: ${CYAN}${lan_subnet}${NC}"
  info "Configuring default policies (Incoming: DROP, Outgoing: ALLOW, Routed: DROP)..."

  # Set default policies
  run_privileged ufw default deny incoming >/dev/null
  run_privileged ufw default allow outgoing >/dev/null
  run_privileged ufw default deny routed >/dev/null

  # Enable logging (low)
  run_privileged ufw logging low >/dev/null

  info "Cleaning up unneeded public exposure (ports 80 & 443)..."
  run_privileged ufw delete allow 80/tcp 2>/dev/null || true
  run_privileged ufw delete allow 443/tcp 2>/dev/null || true
  run_privileged ufw delete allow 43821:43830/tcp 2>/dev/null || true
  run_privileged ufw delete allow 53317/tcp 2>/dev/null || true
  run_privileged ufw delete allow 53317/udp 2>/dev/null || true

  info "Configuring rate-limited SSH protection (port 22)..."
  run_privileged ufw limit 22/tcp comment 'SSH brute-force protection' >/dev/null

  info "Configuring LAN-only rules for msUI file share (ports 43821-43830)..."
  run_privileged ufw allow from "$lan_subnet" to any port 43821:43830 proto tcp comment 'msui fshare LAN only' >/dev/null

  info "Configuring LAN-only rules for LocalSend (port 53317 TCP & UDP)..."
  run_privileged ufw allow from "$lan_subnet" to any port 53317 proto tcp comment 'LocalSend transfer LAN only' >/dev/null
  run_privileged ufw allow from "$lan_subnet" to any port 53317 proto udp comment 'LocalSend discovery LAN only' >/dev/null

  info "Enabling and reloading firewall..."
  run_privileged ufw --force enable >/dev/null
  run_privileged ufw reload >/dev/null

  success "Firewall successfully hardened and active!"
}

# ------------------------------------------------------------------------------
# 4. Print Verification Status
# ------------------------------------------------------------------------------
print_status() {
  echo
  printf "${BOLD}%s${NC}\n" "================================================================"
  printf "${BOLD}%s${NC}\n" "                 Active Firewall Status                         "
  printf "${BOLD}%s${NC}\n" "================================================================"
  run_privileged ufw status verbose
  printf "${BOLD}%s${NC}\n" "================================================================"
  echo
  printf "${GREEN}%s${NC}\n" "Security Summary:"
  echo "  • Public inbound scans:   DROPPED (Zero unsolicited internet access)"
  echo "  • IPv6 external attacks:  BLOCKED (Global IPv6 inbound traffic dropped)"
  echo "  • Local file transfers:   PERMITTED strictly from local LAN subnet"
  echo "  • SSH port (22):          Rate-limited (Blocks brute force attempts)"
  echo
  echo "To view full firewall guide, run: firewall-manual"
  echo
}

main() {
  printf "${CYAN}${BOLD}%s${NC}\n" "================================================================"
  printf "${CYAN}${BOLD}%s${NC}\n" "         msUI Workstation - UFW Firewall Setup                  "
  printf "${CYAN}${BOLD}%s${NC}\n" "================================================================"
  verify_ufw_installed
  apply_firewall_rules
  print_status
}

main "$@"
