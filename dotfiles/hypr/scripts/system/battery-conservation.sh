#!/usr/bin/env bash
# =========================================================
# 🔋 Lenovo IdeaPad Battery Conservation Mode Controller
# Manages Desk Mode (55–60% charge limit) vs Full Charge (100%)
# =========================================================

set -euo pipefail

find_conservation_file() {
    local candidates=(
        "/sys/bus/platform/drivers/ideapad_acpi/VPC2004:00/conservation_mode"
        "/sys/bus/platform/drivers/ideapad_laptop/VPC2004:00/conservation_mode"
        "/sys/devices/platform/ideapad_acpi/conservation_mode"
        "/sys/devices/platform/ideapad_laptop/conservation_mode"
        "/sys/devices/pci0000:00/0000:00:14.3/PNP0C09:00/VPC2004:00/conservation_mode"
    )
    for c in "${candidates[@]}"; do
        if [[ -f "$c" ]]; then
            echo "$c"
            return 0
        fi
    done
    return 1
}

TARGET=$(find_conservation_file || true)
if [[ -z "$TARGET" ]]; then
    echo "❌ No Lenovo conservation_mode sysfs interface found on this hardware." >&2
    exit 1
fi

write_val() {
    local val="$1"
    if [[ -w "$TARGET" ]]; then
        echo "$val" > "$TARGET"
    else
        pkexec /bin/sh -c "echo $val > '$TARGET' && chmod 0666 '$TARGET' 2>/dev/null; mkdir -p /etc/tmpfiles.d && echo 'z /sys/bus/platform/drivers/ideapad_acpi/*/conservation_mode 0666 root root - -' > /etc/tmpfiles.d/lenovo-conservation.conf 2>/dev/null; systemd-tmpfiles --create /etc/tmpfiles.d/lenovo-conservation.conf 2>/dev/null || true; echo 'ACTION==\"add|change\", SUBSYSTEM==\"platform\", RUN+=\"/bin/chmod 0666 /sys/bus/platform/drivers/ideapad_acpi/*/conservation_mode\"' > /etc/udev/rules.d/99-lenovo-conservation.rules 2>/dev/null || true"
    fi
}

send_notify() {
    local title="$1"
    local body="$2"
    if command -v notify-send >/dev/null 2>&1; then
        export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-1}"
        export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=/run/user/$(id -u)/bus}"
        notify-send -a "msUI Battery" -u low -i "battery-good-symbolic" "$title" "$body" >/dev/null 2>&1 || true
    fi
}

ACTION="${1:-status}"
case "$ACTION" in
    1|on|enable|desk)
        write_val 1
        echo "✓ Conservation Mode active: Desk Mode (55–60% charge limit)."
        send_notify "Desk Mode Enabled" "Battery charge capped at ~60% for maximum longevity."
        ;;
    0|off|disable|full)
        write_val 0
        echo "✓ Conservation Mode disabled: Full Charge Mode (charging up to 100%)."
        send_notify "Full Charge Mode Enabled" "Charging battery to 100% capacity."
        ;;
    toggle)
        CURRENT=$(cat "$TARGET" 2>/dev/null || echo 0)
        if [[ "$CURRENT" == "1" ]]; then
            "$0" off
        else
            "$0" on
        fi
        ;;
    status|get)
        CURRENT=$(cat "$TARGET" 2>/dev/null || echo "unknown")
        if [[ "$CURRENT" == "1" ]]; then
            echo "Desk Mode (Active: 55–60% limit)"
        elif [[ "$CURRENT" == "0" ]]; then
            echo "Full Charge Mode (100% limit)"
        else
            echo "Unknown ($CURRENT)"
        fi
        ;;
    *)
        echo "Usage: $0 {desk|full|toggle|status|on|off}" >&2
        exit 1
        ;;
esac
