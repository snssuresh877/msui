#!/usr/bin/env bash
# ==============================================================================
# Clean GRUB Dual-Boot Menu (Arch Linux & Windows 11 only)
# ==============================================================================

set -euo pipefail

if [[ $EUID -ne 0 ]]; then
    echo "❌ Error: This script must be run as root (use sudo)." >&2
    exit 1
fi

echo "========================================================="
echo "🔧 Configuring GRUB for 2 Clean Options: Arch & Windows 11"
echo "========================================================="

# 1. Update /etc/default/grub settings
if [[ -f /etc/default/grub ]]; then
    # Disable OS prober (we use a direct, instant custom entry)
    if grep -q "GRUB_DISABLE_OS_PROBER=" /etc/default/grub; then
        sed -i 's/^GRUB_DISABLE_OS_PROBER=.*/GRUB_DISABLE_OS_PROBER=true/' /etc/default/grub
    else
        echo "GRUB_DISABLE_OS_PROBER=true" >> /etc/default/grub
    fi

    # Disable EFI BootNext scanner
    if grep -q "GRUB_DISABLE_BOOTNEXT=" /etc/default/grub; then
        sed -i 's/^GRUB_DISABLE_BOOTNEXT=.*/GRUB_DISABLE_BOOTNEXT=true/' /etc/default/grub
    else
        echo "GRUB_DISABLE_BOOTNEXT=true" >> /etc/default/grub
    fi

    # Ensure single top-level entry mode
    if ! grep -q "GRUB_SIMPLE_ONLY=" /etc/default/grub; then
        echo "GRUB_SIMPLE_ONLY=true" >> /etc/default/grub
    fi

    # Pin LTS kernel as top level if present
    if [[ -f /boot/vmlinuz-linux-lts ]] && ! grep -q "GRUB_TOP_LEVEL=" /etc/default/grub; then
        echo 'GRUB_TOP_LEVEL="/boot/vmlinuz-linux-lts"' >> /etc/default/grub
    fi
fi

# 2. Patch /etc/grub.d/10_linux for single clean Arch Linux entry
if [[ -f /etc/grub.d/10_linux ]]; then
    if ! grep -q "GRUB_SIMPLE_ONLY" /etc/grub.d/10_linux; then
        echo "  -> Enabling GRUB_SIMPLE_ONLY in /etc/grub.d/10_linux..."
        sed -i '/if \[ "x\$is_top_level" = xtrue \] && \[ "x\${GRUB_DISABLE_SUBMENU}" != xtrue \]; then/i \  if [ "x${GRUB_SIMPLE_ONLY}" = "xtrue" ]; then\n    linux_entry "${OS}" "${version}" simple "${GRUB_CMDLINE_LINUX} ${GRUB_CMDLINE_LINUX_DEFAULT}"\n    break\n  fi\n' /etc/grub.d/10_linux
    fi
fi

# 3. Locate EFI Partition UUID for Windows 11
EFI_UUID="$(findmnt -no UUID /boot/efi 2>/dev/null || true)"
if [[ -z "$EFI_UUID" ]]; then
    EFI_UUID="FEA2-57D3"
fi

# 4. Configure /etc/grub.d/40_custom with exact "Windows 11" entry
echo "  -> Setting up dedicated Windows 11 boot entry in /etc/grub.d/40_custom..."
cat << EOF > /etc/grub.d/40_custom
#!/bin/sh
exec tail -n +3 \$0
# Clean Dual-Boot Windows 11 Entry
menuentry 'Windows 11' --class windows --class os \$menuentry_id_option 'custom-windows11' {
	insmod part_gpt
	insmod fat
	search --no-floppy --fs-uuid --set=root ${EFI_UUID}
	chainloader /EFI/Microsoft/Boot/bootmgfw.efi
}
EOF
chmod +x /etc/grub.d/40_custom

# 5. Disable clutter generator scripts (UEFI firmware, BootNext, OS-prober)
echo "  -> Disabling redundant grub.d generator scripts..."
chmod -x /etc/grub.d/30_os-prober /etc/grub.d/30_uefi-firmware /etc/grub.d/31_efi_bootnext 2>/dev/null || true

# 6. Rebuild GRUB configuration
echo "  -> Regenerating /boot/grub/grub.cfg..."
grub-mkconfig -o /boot/grub/grub.cfg

echo "========================================================="
echo "✅ GRUB configuration successfully updated!"
echo "Active Menu Entries in /boot/grub/grub.cfg:"
grep -E "^menuentry " /boot/grub/grub.cfg | sed -E "s/menuentry '([^']+)'.*/ • \1/" || true
echo "========================================================="
