#!/usr/bin/env bash
# ==============================================================================
# Zenith Minimal RAM Optimizer
# Reduces idle boot memory from ~1.5 GB down to ~1.0 GB
# ==============================================================================

set -euo pipefail

if [[ $EUID -ne 0 ]]; then
    echo "❌ Error: This script must be run as root (use sudo)." >&2
    exit 1
fi

echo "========================================================="
echo "⚡ Starting Zenith System RAM Optimization..."
echo "========================================================="

# 1. Disable Proton VPN Daemon from auto-starting on boot (~386 MB savings)
if systemctl is-enabled proton.VPN.service >/dev/null 2>&1; then
    echo "🛑 [1/5] Disabling Proton VPN auto-start daemon (~386 MB RAM)..."
    systemctl disable --now proton.VPN.service 2>/dev/null || true
    echo "  -> Note: Proton VPN remains fully functional on demand via app or 'systemctl start proton.VPN'."
else
    echo "✓ [1/5] Proton VPN daemon auto-start already disabled."
fi

# 2. Keep Ollama active as requested by user


# 3. Disable TLP Python Power Daemon API (~24 MB savings)
# TLP's core hardware power profile (tlp.service) remains 100% active at 0 MB RAM.
if systemctl is-enabled tlp-pd.service >/dev/null 2>&1; then
    echo "🛑 [3/5] Disabling redundant TLP Python daemon (~24 MB RAM)..."
    systemctl disable --now tlp-pd.service 2>/dev/null || true
    echo "  -> Hardware power saving (CPU boost, ABM, ASPM) stays active via tlp.service."
else
    echo "✓ [3/5] tlp-pd.service already disabled."
fi

# 4. Stop lingering Flatpak system helper (~68 MB savings)
if systemctl is-active flatpak-system-helper.service >/dev/null 2>&1; then
    echo "🛑 [4/5] Stopping idle Flatpak system helper (~68 MB RAM)..."
    systemctl stop flatpak-system-helper.service 2>/dev/null || true
fi

# 5. Cap journald in-memory buffer to prevent RAM bloat
echo "🔧 [5/5] Capping journald runtime RAM buffer to 30 MB..."
mkdir -p /etc/systemd/journald.conf.d
cat << 'EOF' > /etc/systemd/journald.conf.d/00-ram-limit.conf
[Journal]
SystemMaxUse=50M
RuntimeMaxUse=30M
EOF
systemctl restart systemd-journald 2>/dev/null || true

# 6. Drop filesystem cache to release unreferenced buffers
echo 3 > /proc/sys/vm/drop_caches 2>/dev/null || true

echo "========================================================="
echo "✅ RAM Optimization Successfully Applied!"
echo "========================================================="
free -h
echo "========================================================="
