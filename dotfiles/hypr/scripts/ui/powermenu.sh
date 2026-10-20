#!/usr/bin/env bash
set -euo pipefail

# Trigger msUI native Power & Session Menu directly
msui power 2>/dev/null && exit 0

# Fallback to Rofi menu
choice=$(printf " Lock\n Suspend\n󰍃 Logout\n Reboot\n⏻ Power Off" |
  rofi -dmenu -p "Power Menu")

[[ -z "$choice" ]] && exit 0

case "$choice" in
" Lock")
  "$HOME/.config/hypr/scripts/ui/lock_screen.sh"
  ;;
" Suspend")
  loginctl lock-session
  systemctl suspend
  ;;
"󰍃 Logout")
  "$HOME/.config/hypr/scripts/ui/logout_session.sh"
  ;;
" Reboot")
  "$HOME/.config/hypr/scripts/ui/reboot_system.sh"
  ;;
"⏻ Power Off")
  "$HOME/.config/hypr/scripts/ui/power_off.sh"
  ;;
esac
