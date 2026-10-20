#!/usr/bin/env bash
# ==============================================================================
# msUI + Hyprland Complete System Installer & Setup Script
# Works on fresh Arch Linux installations or existing workstations
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOTFILES_DIR="$SCRIPT_DIR/dotfiles"
PACKAGES_DIR="$SCRIPT_DIR/packages"

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

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

banner() {
  cat <<'EOF'
======================================================================
                  msUI + Hyprland Complete Setup
          High-performance Wayland Shell & Window Manager
======================================================================
EOF
}

# ------------------------------------------------------------------------------
# 1. System & Dependency Check
# ------------------------------------------------------------------------------
install_system_packages() {
  local mode="${1:-minimal}"

  if ! command -v pacman >/dev/null 2>&1; then
    warn "Pacman not found. Skipping automated package installation."
    warn "Ensure Hyprland, Wayland, PipeWire, and Rust are installed on your distribution."
    return 0
  fi

  info "Installing system dependencies via pacman..."

  if [[ "$mode" == "minimal" ]]; then
    local pkg_file="$PACKAGES_DIR/pkglist-minimal.txt"
    if [[ -f "$pkg_file" ]]; then
      info "Installing minimal desktop requirements from $(basename "$pkg_file")..."
      # Filter out comments and blank lines
      local pkgs
      pkgs=$(grep -vE '^\s*#|^\s*$' "$pkg_file" | tr '\n' ' ')
      sudo pacman -S --needed --noconfirm $pkgs
      success "Minimal system dependencies installed."
    else
      warn "Package list $pkg_file not found. Installing base packages manually."
      sudo pacman -S --needed --noconfirm \
        base-devel git rustup hyprland hypridle hyprlock \
        xdg-desktop-portal-hyprland xdg-desktop-portal-gtk \
        pipewire pipewire-pulse wireplumber networkmanager \
        bluez bluez-utils grim slurp wl-clipboard kitty foot \
        noto-fonts awesome-terminal-fonts
    fi
  elif [[ "$mode" == "full" ]]; then
    local pkg_file="$PACKAGES_DIR/pkglist-pacman.txt"
    if [[ -f "$pkg_file" ]]; then
      info "Installing full pacman package snapshot..."
      sudo pacman -S --needed --noconfirm - < "$pkg_file"
      success "Full system package list restored."
    fi
  fi

  # Ensure Rust toolchain is configured
  if command -v rustup >/dev/null 2>&1; then
    if ! rustup default >/dev/null 2>&1; then
      info "Configuring default stable Rust toolchain..."
      rustup default stable
    fi
  fi
}

# ------------------------------------------------------------------------------
# 2. Build and Install msUI Binary
# ------------------------------------------------------------------------------
install_msui_binary() {
  info "Preparing msUI binary..."

  mkdir -p "$HOME/.local/bin"

  local binary_path=""

  # Check pre-compiled release binary in target/release/msui
  if [[ -f "$SCRIPT_DIR/target/release/msui" ]]; then
    info "Found pre-built release binary in target/release/msui"
    binary_path="$SCRIPT_DIR/target/release/msui"
  elif [[ -f "$SCRIPT_DIR/bin/msui" ]]; then
    info "Found packaged binary in bin/msui"
    binary_path="$SCRIPT_DIR/bin/msui"
  else
    info "Building msUI from source with Cargo (this may take a few minutes)..."
    if ! command -v cargo >/dev/null 2>&1; then
      error "Cargo not found! Please install Rust (rustup or rust package)."
      exit 1
    fi
    (cd "$SCRIPT_DIR" && cargo build --release)
    binary_path="$SCRIPT_DIR/target/release/msui"
  fi

  if [[ ! -f "$binary_path" ]]; then
    error "Failed to locate or build msui binary!"
    exit 1
  fi

  info "Installing msui to $HOME/.local/bin/msui..."
  cp -f "$binary_path" "$HOME/.local/bin/msui"
  chmod +x "$HOME/.local/bin/msui"

  # Also link to ~/.cargo/bin if that directory exists
  if [[ -d "$HOME/.cargo/bin" ]]; then
    ln -sf "$HOME/.local/bin/msui" "$HOME/.cargo/bin/msui"
  fi

  # Ensure ~/.local/bin is in PATH
  ensure_path_configured

  success "msUI binary installed: $($HOME/.local/bin/msui --version || echo 'msui ready')"
}

# ------------------------------------------------------------------------------
# 3. Ensure PATH Configuration
# ------------------------------------------------------------------------------
ensure_path_configured() {
  local path_line='export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"'

  # ~/.bashrc
  if [[ -f "$HOME/.bashrc" ]] && ! grep -q 'HOME/.local/bin' "$HOME/.bashrc"; then
    info "Adding ~/.local/bin to ~/.bashrc..."
    echo -e "\n# Added by msUI installer\n$path_line" >> "$HOME/.bashrc"
  fi

  # ~/.bash_profile
  if [[ -f "$HOME/.bash_profile" ]] && ! grep -q 'HOME/.local/bin' "$HOME/.bash_profile"; then
    info "Adding ~/.local/bin to ~/.bash_profile..."
    echo -e "\n# Added by msUI installer\n$path_line" >> "$HOME/.bash_profile"
  fi

  # ~/.profile
  if [[ -f "$HOME/.profile" ]] && ! grep -q 'HOME/.local/bin' "$HOME/.profile"; then
    info "Adding ~/.local/bin to ~/.profile..."
    echo -e "\n# Added by msUI installer\n$path_line" >> "$HOME/.profile"
  fi

  # Fish shell
  local fish_config="$HOME/.config/fish/config.fish"
  if [[ -d "$HOME/.config/fish" ]] || command -v fish >/dev/null 2>&1; then
    mkdir -p "$HOME/.config/fish"
    if [[ -f "$fish_config" ]]; then
      if ! grep -q '.local/bin' "$fish_config"; then
        echo -e '\n# Added by msUI installer\nfish_add_path -a $HOME/.local/bin $HOME/.cargo/bin' >> "$fish_config"
      fi
    else
      echo -e '# Fish shell config\nfish_add_path -a $HOME/.local/bin $HOME/.cargo/bin' > "$fish_config"
    fi
  fi
}

# ------------------------------------------------------------------------------
# 4. Deploy Dotfiles (~/.config)
# ------------------------------------------------------------------------------
deploy_dotfiles() {
  local deploy_all="${1:-true}"
  local timestamp
  timestamp="$(date +%Y%m%d_%H%M%S)"
  local backup_dir="$HOME/.config/backup_$timestamp"

  info "Deploying configurations..."
  mkdir -p "$HOME/.config"

  # Core configs to deploy
  local configs=("hypr" "msui")

  if [[ "$deploy_all" == "true" ]]; then
    # Full suite of desktop applications
    configs+=("fish" "foot" "kitty" "ghostty" "nvim" "btop" "htop" "Thunar" "yay" "aria2")
  fi

  for cfg in "${configs[@]}"; do
    local src="$DOTFILES_DIR/$cfg"
    local dst="$HOME/.config/$cfg"

    if [[ -d "$src" ]]; then
      if [[ -d "$dst" ]]; then
        mkdir -p "$backup_dir"
        info "Backing up existing $dst -> $backup_dir/$cfg..."
        cp -r "$dst" "$backup_dir/$cfg"
        rm -rf "$dst"
      fi
      info "Installing configuration for $cfg -> $dst..."
      cp -r "$src" "$dst"
    fi
  done

  # Make all shell scripts in hypr executable
  if [[ -d "$HOME/.config/hypr/scripts" ]]; then
    info "Setting executable permissions on Hyprland scripts..."
    find "$HOME/.config/hypr/scripts" -type f -name "*.sh" -exec chmod +x {} +
  fi

  if [[ -d "$backup_dir" ]]; then
    success "Previous configurations backed up to: $backup_dir"
  fi
  success "Desktop configurations deployed successfully."
}

# ------------------------------------------------------------------------------
# 5. Deploy Wallpapers
# ------------------------------------------------------------------------------
deploy_wallpapers() {
  info "Checking desktop wallpapers..."
  mkdir -p "$HOME/Pictures/Wallpapers"

  if [[ -d "$SCRIPT_DIR/wallpapers" ]]; then
    info "Copying wallpapers from repository to ~/Pictures/Wallpapers/..."
    cp -rn "$SCRIPT_DIR/wallpapers/"* "$HOME/Pictures/Wallpapers/" 2>/dev/null || true
    success "Wallpapers installed."
  fi
}

# ------------------------------------------------------------------------------
# Main Flow
# ------------------------------------------------------------------------------
main() {
  banner

  local action="all"
  if [[ $# -gt 0 ]]; then
    case "$1" in
      --bin-only) action="bin" ;;
      --configs-only) action="configs" ;;
      --packages-minimal) action="pkgs-min" ;;
      --packages-full) action="pkgs-full" ;;
      --all) action="all" ;;
      *)
        echo "Usage: $0 [--all | --bin-only | --configs-only | --packages-minimal | --packages-full]"
        exit 0
        ;;
    esac
  fi

  case "$action" in
    pkgs-min)
      install_system_packages "minimal"
      ;;
    pkgs-full)
      install_system_packages "full"
      ;;
    bin)
      install_msui_binary
      ;;
    configs)
      deploy_dotfiles "true"
      deploy_wallpapers
      ;;
    all)
      info "Starting full installation..."
      install_system_packages "minimal"
      install_msui_binary
      deploy_dotfiles "true"
      deploy_wallpapers
      ;;
  esac

  echo
  success "================================================================"
  success " Installation completed successfully!"
  success " msUI binary: $HOME/.local/bin/msui"
  success " Hyprland config: $HOME/.config/hypr"
  success " msUI config: $HOME/.config/msui"
  success "================================================================"
  echo
  info "To start your desktop session:"
  echo "  1. If running inside a TTY:  exec Hyprland"
  echo "  2. If already in Hyprland:   hyprctl reload"
  echo "  3. To launch or test msUI:   msui bar"
  echo
}

main "$@"
