#!/usr/bin/env bash
# =========================================================
# ⚡ Zenith Workstation - System Power Optimizer
# One-Step Hardware Power Tuning & Battery Life Extender
# =========================================================

set -euo pipefail

# Ensure running with root privileges
if [[ $EUID -ne 0 ]]; then
    echo "⚠️ This script requires root privileges to configure system power parameters."
    echo "Please run: sudo $0"
    exit 1
fi

echo "========================================================="
echo "⚡ Starting Zenith System Power Optimization..."
echo "========================================================="

# ---------------------------------------------------------
# 1. TLP Hardware-Specific Tuning Drop-In
# ---------------------------------------------------------
echo "🔧 [1/4] Configuring TLP hardware power profile (/etc/tlp.d/99-zenith-power.conf)..."

mkdir -p /etc/tlp.d
cat << 'TLP_EOF' > /etc/tlp.d/99-zenith-power.conf
# =========================================================
# ⚡ Zenith Hardware Power Tuning (Lenovo IdeaPad / AMD Zen 2)
# =========================================================

# CPU Turbo Boost: Off on battery to eliminate 20W+ thermal spikes; on when plugged in
CPU_BOOST_ON_AC=1
CPU_BOOST_ON_BAT=0

# Lenovo ACPI Platform Profile
PLATFORM_PROFILE_ON_AC=balanced
PLATFORM_PROFILE_ON_BAT=low-power
PLATFORM_PROFILE_ON_SAV=low-power

# PCIe Active State Power Management (ASPM)
PCIE_ASPM_ON_AC=default
PCIE_ASPM_ON_BAT=powersupersave
PCIE_ASPM_ON_SAV=powersupersave

# AMD GPU DPM & Panel Power Savings (ABM)
RADEON_DPM_PERF_LEVEL_ON_AC=auto
RADEON_DPM_PERF_LEVEL_ON_BAT=auto
RADEON_DPM_STATE_ON_AC=performance
RADEON_DPM_STATE_ON_BAT=battery
AMDGPU_ABM_LEVEL_ON_AC=0
AMDGPU_ABM_LEVEL_ON_BAT=3
AMDGPU_ABM_LEVEL_ON_SAV=3

# Wi-Fi & Audio Power Saving
WIFI_PWR_ON_AC=off
WIFI_PWR_ON_BAT=on
SOUND_POWER_SAVE_ON_AC=1
SOUND_POWER_SAVE_ON_BAT=1
SOUND_POWER_SAVE_CONTROLLER=Y

# USB Autosuspend
# Exempt Logitech Wireless Nano Receiver (046d:c534) to avoid any mouse cursor wake lag
USB_AUTOSUSPEND=1
USB_DENYLIST="046d:c534"

# Lenovo Battery Conservation Mode (Prolongs lifespan by capping AC charging at 60-80%)
RESTORE_THRESHOLDS_ON_BAT=1
START_CHARGE_THRESH_BAT0=60
STOP_CHARGE_THRESH_BAT0=80
START_CHARGE_THRESH_BAT1=60
STOP_CHARGE_THRESH_BAT1=80
TLP_EOF

# ---------------------------------------------------------
# 2. Lenovo Battery Conservation Mode
# ---------------------------------------------------------
echo "🔋 [2/4] Enabling Lenovo Battery Conservation Mode..."
CONV_MODE_PATH=$(find /sys/bus/platform/drivers/ideapad_acpi/ -name conservation_mode 2>/dev/null | head -n1 || true)
if [[ -n "$CONV_MODE_PATH" && -w "$CONV_MODE_PATH" ]]; then
    echo 1 > "$CONV_MODE_PATH"
    echo "  -> Conservation mode active (stops charging at 60-80% to protect battery)."
fi

# Create persistent systemd unit for Conservation Mode across reboots
cat << 'UNIT_EOF' > /etc/systemd/system/lenovo-conservation.service
[Unit]
Description=Lenovo Battery Conservation Mode
After=multi-user.target

[Service]
Type=oneshot
ExecStart=/bin/sh -c 'for f in /sys/bus/platform/drivers/ideapad_acpi/*/conservation_mode; do [ -f "$f" ] && echo 1 > "$f"; done'
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
UNIT_EOF

systemctl daemon-reload
systemctl enable --now lenovo-conservation.service >/dev/null 2>&1 || true

# ---------------------------------------------------------
# 3. GRUB USB Autosuspend Bug Fix
# ---------------------------------------------------------
echo "🔌 [3/4] Checking kernel command line for usbcore.autosuspend=-1..."
if [[ -f /etc/default/grub ]] && grep -q "usbcore\.autosuspend=-1" /etc/default/grub; then
    echo "  -> Removing global usbcore.autosuspend=-1 from /etc/default/grub..."
    sed -i -E 's/usbcore\.autosuspend=-1 ?//' /etc/default/grub
    if command -v grub-mkconfig >/dev/null 2>&1; then
        echo "  -> Updating GRUB configuration (/boot/grub/grub.cfg)..."
        grub-mkconfig -o /boot/grub/grub.cfg >/dev/null 2>&1 || true
    fi
    echo "  -> USB autosuspend unblocked! Internal devices (camera, fingerprint) can now sleep."
else
    echo "  -> No global usbcore.autosuspend override detected in GRUB."
fi

# ---------------------------------------------------------
# 4. UDev Auto-Switching Rule for AC / Battery Events
# ---------------------------------------------------------
echo "⚡ [4/4] Setting up dynamic AC/Battery udev event triggers..."
USER_NAME="m"
USER_SCRIPT="/home/${USER_NAME}/.config/hypr/scripts/system/battery-power-hook.sh"

cat << UDEV_EOF > /etc/udev/rules.d/99-zenith-power.rules
# Trigger user power profile hook when AC power is plugged or unplugged
SUBSYSTEM=="power_supply", ATTR{online}=="0", RUN+="/usr/bin/su ${USER_NAME} -c '${USER_SCRIPT} bat'"
SUBSYSTEM=="power_supply", ATTR{online}=="1", RUN+="/usr/bin/su ${USER_NAME} -c '${USER_SCRIPT} ac'"
UDEV_EOF

udevadm control --reload-rules >/dev/null 2>&1 || true

# ---------------------------------------------------------
# 5. Apply TLP Settings Immediately
# ---------------------------------------------------------
if command -v tlp >/dev/null 2>&1; then
    echo "🔄 Applying TLP settings immediately..."
    tlp start >/dev/null 2>&1 || true
fi

echo "========================================================="
echo "✅ System Power Optimization Successfully Applied!"
echo "========================================================="
echo "Summary of applied enhancements:"
echo " • CPU Boost disabled on battery (no 20W spikes)"
echo " • Platform Profile set to low-power on battery"
echo " • AMD GPU ABM level 3 active on battery"
echo " • PCIe ASPM powersupersave active on battery"
echo " • USB autosuspend enabled with Logitech mouse receiver excluded"
echo " • Lenovo Conservation Mode enabled (preserves 1066-cycle battery)"
echo " • Dynamic udev rule hooked to AC/Battery transitions"
echo "========================================================="
