#!/bin/bash
pgrep -x rofi >/dev/null && exit 0

if ~/.config/hypr/scripts/ai/ai-control.sh status >/dev/null 2>&1; then
  ai_option="🤖 Local AI Engine [Active - Click to Stop & Free RAM]"
else
  ai_option="🤖 Local AI Engine [Stopped - Click to Start]"
fi

choice=$(printf \
"$ai_option
🎨 VectorCraft / ArtCraft AI Studio
󰓃 Reload Audio (PipeWire / WirePlumber)
󰂯 Reload Bluetooth Stack (BlueZ)
󰤨 Reload Wi-Fi Driver & NetworkManager
󰟸 Reload Trackpad / Input Drivers
󰍹 Reload Monitors & Hyprland Config
🔋 Battery & Power Diagnostics
⚡ Run Full Hardware Diagnostics" | rofi -dmenu -i -p "Hardware & System Controls")

[ -z "$choice" ] && exit 0

case "$choice" in
  *Local\ AI\ Engine*)
    ~/.config/hypr/scripts/ai/ai-control.sh toggle
    ;;
  *VectorCraft*)
    foot -a ai_assistant_float -T "VectorCraft AI Studio" -e ~/.config/hypr/scripts/ai/vectorcraft-ai-bridge.sh &
    ;;
  *Battery*|*Power*)
    foot -T "Power Diagnostics" -e bash -c "~/.config/hypr/scripts/system/battery-power-hook.sh status; echo ''; read -n1 -r -p 'Press any key to exit...'"
    ;;
  *Audio*)
    systemctl --user restart pipewire wireplumber pipewire-pulse 2>/dev/null || true
    notify-send "Hardware" "Audio stack (PipeWire) reloaded"
    ;;
  *Bluetooth*)
    sudo systemctl restart bluetooth 2>/dev/null || true
    notify-send "Hardware" "Bluetooth service reloaded"
    ;;
  *Wi-Fi*)
    sudo systemctl restart NetworkManager 2>/dev/null || true
    notify-send "Hardware" "NetworkManager reloaded"
    ;;
  *Trackpad*)
    sudo modprobe -r psmouse 2>/dev/null && sudo modprobe psmouse 2>/dev/null || true
    notify-send "Hardware" "Trackpad driver reset"
    ;;
  *Monitors*)
    hyprctl reload
    notify-send "Hyprland" "Monitors and configuration reloaded"
    ;;
  *Full\ Hardware*)
    if command -v inxi >/dev/null 2>&1; then
      foot -T "Hardware Diagnostics" -e inxi -Fazy
    elif command -v lshw >/dev/null 2>&1; then
      foot -T "Hardware Diagnostics" -e sudo lshw -short
    fi
    ;;
esac
