# msUI + Hyprland Complete System Restore & Installation Guide

This guide describes how to replicate and restore your complete **msUI + Hyprland** desktop environment onto any new Arch Linux computer or freshly formatted system.

---

## ⚡ Method 1: Restore via Git (Recommended for Online Installs)

If your new system has internet access:

### 1. Install Base Git & Clone msUI
```bash
sudo pacman -S --needed base-devel git
git clone git@github.com:snssuresh877/msui.git ~/Projects/msui
cd ~/Projects/msui
```
*(If SSH keys are not yet configured on the new machine, you can clone via HTTPS:*
`git clone https://github.com/snssuresh877/msui.git ~/Projects/msui`*)*

### 2. Run the Turnkey Installer
```bash
./install.sh
```

The script will automatically:
1. Install all required Arch packages (`hyprland`, `hypridle`, `hyprlock`, `xdg-desktop-portal-hyprland`, audio, bluetooth, fonts, etc.).
2. Build or install the high-performance `msui` binary directly to `~/.local/bin/msui`.
3. Configure `PATH` across `.bashrc`, `.bash_profile`, `.profile`, and `fish`.
4. Deploy all desktop configurations (`~/.config/hypr`, `~/.config/msui`, `kitty`, `foot`, `fish`, `btop`, etc.) while creating safety backups of existing configurations.
5. Deploy wallpapers to `~/Pictures/Wallpapers/`.
6. Set correct executable permissions on all desktop scripts.

---

## 💾 Method 2: Offline / USB Pendrive Restore (Tarball Archive)

If you copied `msui-hyprland-complete-backup-YYYY-MM-DD.tar.zst` to a USB drive:

### 1. Mount USB Drive & Extract
```bash
# Example if your USB is mounted at /run/media/$USER/VENTOY:
tar --zstd -xvf /run/media/$USER/VENTOY/msui-hyprland-complete-backup-*.tar.zst -C ~
cd ~/Projects/msui
```

### 2. Run Installer
```bash
./install.sh
```
*(Because the archive includes the pre-compiled `msui` binary in `bin/msui`, installation takes under 10 seconds without needing to recompile Rust!)*

---

## 🚀 Starting Your Desktop Session

After running `./install.sh`:

1. **From Linux TTY Console**:
   ```bash
   exec Hyprland
   ```
2. **From Login Manager (SDDM / GDM / Ly)**:
   Select **Hyprland** and log in.
3. msUI Status Bar, Wallpaper Daemon, and PolicyKit Agent will automatically start via Hyprland's `autostart.lua`.

---

## 🛠️ Installer Command Line Options

The `install.sh` script supports modular flags if you only want to update specific parts:

| Command | Action |
|---|---|
| `./install.sh` or `./install.sh --all` | Complete end-to-end setup (packages, binary, configs, wallpapers) |
| `./install.sh --bin-only` | Compiles/installs `msui` binary only to `~/.local/bin/msui` |
| `./install.sh --configs-only` | Deploys desktop configs (`hypr`, `msui`, terminal, shell) |
| `./install.sh --packages-minimal` | Installs only the essential minimal desktop packages |
| `./install.sh --packages-full` | Restores the complete snapshot of all pacman packages |

---

## 📂 Backup & Config Structure

- **Repository**: `~/Projects/msui`
- **Active msUI Binary**: `~/.local/bin/msui`
- **Hyprland Configuration**: `~/.config/hypr/` (managed via Lua modules and direct socket dispatch)
- **msUI Configuration**: `~/.config/msui/msui.toml`
- **System Package Snapshot**: `packages/pkglist-pacman.txt` and `packages/pkglist-aur.txt`
- **Wallpapers**: `~/Pictures/Wallpapers/`
