#!/usr/bin/env bash
# =========================================================
# ⚡ Zenith Workstation - Battery & Power Auto-Hook
# Fast • Zero Background RAM • Hardware-Aware Power Tuning
# =========================================================

set -euo pipefail

# Determine current power source (1 = AC connected, 0 = On Battery)
is_ac_online() {
    if [[ -f /sys/class/power_supply/ACAD/online ]]; then
        cat /sys/class/power_supply/ACAD/online
    elif [[ -f /sys/class/power_supply/AC/online ]]; then
        cat /sys/class/power_supply/AC/online
    elif [[ -f /sys/class/power_supply/BAT1/status ]]; then
        local st
        st=$(cat /sys/class/power_supply/BAT1/status)
        if [[ "$st" == "Discharging" ]]; then
            echo 0
        else
            echo 1
        fi
    else
        echo 1
    fi
}

apply_battery_profile() {
    # 1. Switch Power Profiles via authoritative tlpctl / powerprofilesctl
    if command -v tlpctl >/dev/null 2>&1; then
        tlpctl set power-saver >/dev/null 2>&1 || true
    elif command -v powerprofilesctl >/dev/null 2>&1; then
        powerprofilesctl set power-saver >/dev/null 2>&1 || true
    fi

    # 2. Check Bluetooth: if 0 connected devices, soft-block to save power
    if command -v bluetoothctl >/dev/null 2>&1 && command -v rfkill >/dev/null 2>&1; then
        local connected
        connected=$(bluetoothctl devices Connected 2>/dev/null || true)
        if [[ -z "$connected" ]]; then
            rfkill block bluetooth 2>/dev/null || true
        fi
    fi

    # 3. Notification
    export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-1}"
    export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=/run/user/$(id -u)/bus}"
    if command -v notify-send >/dev/null 2>&1; then
        notify-send -a "msUI Power" -u low -i "battery-good-symbolic" \
            "Battery Saver Active" "Switched to Power Saver profile." >/dev/null 2>&1 || true
    fi
}

apply_ac_profile() {
    # 1. Switch Power Profiles to balanced
    if command -v tlpctl >/dev/null 2>&1; then
        tlpctl set balanced >/dev/null 2>&1 || true
    elif command -v powerprofilesctl >/dev/null 2>&1; then
        powerprofilesctl set balanced >/dev/null 2>&1 || true
    fi

    # 2. Unblock Bluetooth radio
    if command -v rfkill >/dev/null 2>&1; then
        rfkill unblock bluetooth 2>/dev/null || true
    fi

    # 3. Notification
    export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-1}"
    export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=/run/user/$(id -u)/bus}"
    if command -v notify-send >/dev/null 2>&1; then
        notify-send -a "msUI Power" -u low -i "ac-adapter-symbolic" \
            "AC Power Connected" "Switched to Balanced performance profile." >/dev/null 2>&1 || true
    fi
}

print_status() {
    local ac_state
    ac_state=$(is_ac_online)

    echo "========================================================="
    echo "⚡ Zenith Laptop Power Diagnostics"
    echo "========================================================="
    if [[ "$ac_state" -eq 1 ]]; then
        echo "🔌 Power Source     : AC Power (Plugged in)"
    else
        echo "🔋 Power Source     : Battery (Discharging)"
    fi

    if [[ -f /sys/class/power_supply/BAT1/capacity ]]; then
        echo "🔋 Battery Charge   : $(cat /sys/class/power_supply/BAT1/capacity)%"
    fi

    if [[ -f /sys/class/power_supply/BAT1/power_now ]]; then
        local pw
        pw=$(cat /sys/class/power_supply/BAT1/power_now)
        local watts
        watts=$(awk "BEGIN {printf \"%.2f\", $pw / 1000000}")
        echo "⚡ Realtime Draw    : ${watts} W"
    fi

    if [[ -f /sys/firmware/acpi/platform_profile ]]; then
        echo "💻 Platform Profile : $(cat /sys/firmware/acpi/platform_profile)"
    fi

    if [[ -f /sys/class/drm/card1-eDP-1/amdgpu/panel_power_savings ]]; then
        echo "🖥️ AMD Panel ABM    : Level $(cat /sys/class/drm/card1-eDP-1/amdgpu/panel_power_savings)"
    fi

    if command -v rfkill >/dev/null 2>&1; then
        local bt_state
        bt_state=$(rfkill list bluetooth | grep -m1 "Soft blocked:" | awk '{print $3}')
        echo "📶 Bluetooth Soft-Off: ${bt_state:-unknown}"
    fi
    echo "========================================================="
}

# ---------------------------------------------------------
# Action Dispatcher
# ---------------------------------------------------------
MODE="${1:-auto}"

case "$MODE" in
    bat|battery|power-saver)
        apply_battery_profile
        ;;
    ac|balanced|performance)
        apply_ac_profile
        ;;
    status)
        print_status
        ;;
    auto|*)
        if [[ "$(is_ac_online)" -eq 0 ]]; then
            apply_battery_profile
        else
            apply_ac_profile
        fi
        ;;
esac
