# msUI Dotfiles & Desktop Configuration

This directory contains configuration files for the **msUI** desktop environment on Arch Linux / Hyprland.

## Included Configurations

| Directory | Target Location | Description |
|-----------|-----------------|-------------|
| `aria2/` | `~/.config/aria2/` | Aria2 download accelerator configuration |
| `btop/` | `~/.config/btop/` | Btop system monitor theme and settings |
| `fish/` | `~/.config/fish/` | Fish shell functions, completions, and configuration |
| `flux/` | `~/.config/flux/` | Flux-FM modern GTK4/Rust file manager configuration and menus |
| `foot/` | `~/.config/foot/` | Foot terminal emulator configuration |
| `ghostty/` | `~/.config/ghostty/` | Ghostty terminal configuration and themes |
| `htop/` | `~/.config/htop/` | Htop settings and layout |
| `hypr/` | `~/.config/hypr/` | Hyprland compositor settings, keybinds, scripts, and themes |
| `kitty/` | `~/.config/kitty/` | Kitty terminal configuration |
| `msui/` | `~/.config/msui/` | Active runtime msUI bar and module toggles |
| `nvim/` | `~/.config/nvim/` | Neovim Lua setup, plugins, and keymaps |
| `Thunar/` | `~/.config/Thunar/` | Thunar file manager custom actions and accelerators |
| `yazi/` | `~/.config/yazi/` | Yazi terminal file manager (git status, chmod, ripdrag plugins, universal Neovim opener) |
| `yay/` | `~/.config/yay/` | Yay AUR helper preferences |

## Quick Restore / Installation

To symlink or restore these dotfiles to `~/.config/`:

```bash
# Example symlink command from the repository root:
for dir in dotfiles/*/; do
  target="$HOME/.config/$(basename "$dir")"
  echo "Linking $dir -> $target"
  # ln -sfn "$(realpath "$dir")" "$target"
done
```
