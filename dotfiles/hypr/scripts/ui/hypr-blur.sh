#!/usr/bin/env bash
# hypr-blur: Zero-resident-RAM user controller for Hyprland 0.57+ blur shaders
set -e

BACKUP_DIR="$HOME/.config/hypr_backup_pre_blur_057"
CONFIG_FILE="$HOME/.config/hypr/modules/appearance.lua"
DOTFILES_CONFIG="$HOME/Projects/msui/dotfiles/hypr/modules/appearance.lua"

# 10 Options exactly as requested:
# 1) No blur, 2) drops, 3) fluid_jar, 4) prism, 5) frost,
# 6) acrylic, 7) aurora, 8) water, 9) heat_shimmer, 10) kawase
OPTIONS=(
    "No blur"
    "drops"
    "fluid_jar"
    "prism"
    "frost"
    "acrylic"
    "aurora"
    "water"
    "heat_shimmer"
    "kawase"
)

DESCRIPTIONS=(
    "Disable window & layer background blur (Lowest GPU / RAM)"
    "Rain droplets trickling & condensing on glass"
    "Viscous fluid container with turbulence & liquid physics"
    "Crystalline prism refraction with rainbow light dispersion"
    "Micro-textured winter frosted glass surface"
    "Modern acrylic glass with light diffusion & chromatic aberration"
    "Undulating Northern Lights / Aurora Borealis ambient color glow"
    "Fluid wave displacement equations rippling across background"
    "Atmospheric heat haze mirage distortion waves"
    "Standard dual-kawase pyramid blur (lightweight & clean)"
)

ICONS=(
    "🚫"
    "💧"
    "🧪"
    "💎"
    "❄️"
    "🪟"
    "🌌"
    "🌊"
    "♨️"
    "🌫️"
)

get_current_status() {
    local enabled var
    enabled=$(hyprctl -j getoption decoration:blur:enabled 2>/dev/null | jq -r '.bool // true')
    var=$(hyprctl -j getoption decoration:blur:variant 2>/dev/null | jq -r '.int // 0')
    if [ "$enabled" = "false" ]; then
        echo "0"
        return
    fi
    case "$var" in
        3) echo 2 ;;
        5) echo 3 ;;
        6) echo 4 ;;
        1) echo 5 ;;
        8) echo 6 ;;
        9) echo 7 ;;
        4) echo 8 ;;
        7) echo 9 ;;
        0) echo 10 ;;
        *) echo 6 ;;
    esac
}

apply_option() {
    local num="$1"
    local opt_name=""
    local lua_code=""
    local enabled="true"
    local variant="acrylic"
    local size=5
    local passes=2
    local prop=true

    case "$num" in
        1|"no-blur"|"none"|"off"|"disable")
            enabled="false"
            opt_name="No blur"
            lua_code='hl.config({ decoration = { blur = { enabled = false } } })'
            ;;
        2|"drops")
            opt_name="drops"
            variant="drops"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "drops", size = 5, passes = 2, proportional = true } } })'
            ;;
        3|"fluid_jar")
            opt_name="fluid_jar"
            variant="fluid_jar"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "fluid_jar", size = 5, passes = 2, proportional = true } } })'
            ;;
        4|"prism")
            opt_name="prism"
            variant="prism"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "prism", size = 5, passes = 2, proportional = true } } })'
            ;;
        5|"frost")
            opt_name="frost"
            variant="frost"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "frost", size = 5, passes = 2, proportional = true } } })'
            ;;
        6|"acrylic")
            opt_name="acrylic"
            variant="acrylic"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "acrylic", size = 5, passes = 2, proportional = true } } })'
            ;;
        7|"aurora")
            opt_name="aurora"
            variant="aurora"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "aurora", size = 5, passes = 2, proportional = true } } })'
            ;;
        8|"water")
            opt_name="water"
            variant="water"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "water", size = 5, passes = 2, proportional = true } } })'
            ;;
        9|"heat_shimmer")
            opt_name="heat_shimmer"
            variant="heat_shimmer"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "heat_shimmer", size = 5, passes = 2, proportional = true } } })'
            ;;
        10|"kawase")
            opt_name="kawase"
            variant="kawase"
            size=2
            passes=1
            prop=false
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "kawase", size = 2, passes = 1, proportional = false } } })'
            ;;
        *)
            echo "Error: Invalid option '$num'. Must be 1 to 10."
            echo "Run 'hypr-blur list' to see all 10 options."
            exit 1
            ;;
    esac

    hyprctl repl "$lua_code" >/dev/null 2>&1

    python3 -c "
import sys

def update_file(filepath):
    try:
        with open(filepath, 'r') as f:
            content = f.read()
        import re
        if '$enabled' == 'false':
            replacement = '''		blur = {
			enabled = false,
			variant = \"$variant\",
			size = 2,
			passes = 1,
			proportional = false,
		},'''
        else:
            replacement = '''		blur = {
			enabled = true,
			variant = \"$variant\", -- Active: $opt_name
			size = $size,
			passes = $passes,
			proportional = $prop,
			ignore_opacity = true,
			new_optimizations = true,
		},'''
        new_content = re.sub(r'blur\s*=\s*\{[^}]*\},', replacement, content, count=1)
        if new_content != content:
            with open(filepath, 'w') as f:
                f.write(new_content)
    except Exception as e:
        pass

update_file('$CONFIG_FILE')
update_file('$DOTFILES_CONFIG')
"

    if [ "$enabled" = "false" ]; then
        notify-send -a "Hyprland Blur" -i dialog-information "Blur Disabled" "Window & layer blur turned off (Option 1)"
        echo "✓ Disabled blur (Option 1: No blur)"
    else
        notify-send -a "Hyprland Blur" -i preferences-desktop-theme "Blur: $opt_name" "Applied option $num ($opt_name) with size=$size, passes=$passes"
        echo "✓ Applied option $num: $opt_name"
    fi
}

cmd_status() {
    local enabled var size passes prop
    enabled=$(hyprctl -j getoption decoration:blur:enabled 2>/dev/null | jq -r '.bool // true')
    var=$(hyprctl -j getoption decoration:blur:variant 2>/dev/null | jq -r '.int // 0')
    size=$(hyprctl -j getoption decoration:blur:size 2>/dev/null | jq -r '.int // 2')
    passes=$(hyprctl -j getoption decoration:blur:passes 2>/dev/null | jq -r '.int // 1')
    prop=$(hyprctl -j getoption decoration:blur:proportional 2>/dev/null | jq -r '.bool // false')

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo " Hyprland 0.57 Blur Status (0 MB Resident RAM)"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo " Blur Enabled: $enabled"
    echo " Active Option: $([ "$enabled" = "false" ] && echo "1) No blur" || echo "Variant $var")"
    echo " Size / Passes: ${size}px / ${passes} passes"
    echo " Proportional: $prop"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

cmd_list() {
    local cur
    cur=$(get_current_status)

    echo ""
    echo "  ╔════════════════════════════════════════════════════════════════════════╗"
    echo "  ║         󰔎 Hyprland Blur & Shader Options (0 MB Resident RAM)           ║"
    echo "  ╚════════════════════════════════════════════════════════════════════════╝"
    echo ""
    for i in {1..10}; do
        local idx=$(( i - 1 ))
        local marker="  "
        if [ "$i" -eq "$cur" ]; then
            marker="► "
        fi
        printf " %s[%2d] %s %-14s - %s\n" "$marker" "$i" "${ICONS[$idx]}" "${OPTIONS[$idx]}" "${DESCRIPTIONS[$idx]}"
    done
    echo ""
    echo "  Usage:"
    echo "    hypr-blur <1-10>       Switch immediately (e.g. 'hypr-blur 2' for drops)"
    echo "    hypr-blur menu         Open interactive picker menu"
    echo "    hypr-blur status       Display current settings"
    echo "    hypr-blur reset        Reset to standard kawase blur"
    echo ""
}

cmd_menu() {
    local cur
    cur=$(get_current_status)

    if command -v fzf >/dev/null 2>&1 && [ -t 0 ]; then
        local menu_items=()
        for i in {1..10}; do
            local idx=$(( i - 1 ))
            local prefix="  "
            if [ "$i" -eq "$cur" ]; then
                prefix="* "
            fi
            menu_items+=("$(printf "%s%2d) %s %-13s │ %s" "$prefix" "$i" "${ICONS[$idx]}" "${OPTIONS[$idx]}" "${DESCRIPTIONS[$idx]}")")
        done

        local chosen
        chosen=$(printf "%s\n" "${menu_items[@]}" | fzf \
            --ansi \
            --height=15 \
            --reverse \
            --prompt="Select Blur Option (1-10) > " \
            --header="Hyprland 0.57 Blur Shaders (0 MB Resident RAM)" \
            --header-first) || true

        if [ -n "$chosen" ]; then
            local num
            num=$(echo "$chosen" | grep -oE '[0-9]+' | head -n 1)
            if [ -n "$num" ]; then
                apply_option "$num"
                sleep 0.3
                return
            fi
        fi
        return
    fi

    cmd_list
    echo -n "  Enter choice (1-10) or 'q' to quit: "
    read -r choice
    if [[ "$choice" =~ ^[0-9]+$ ]] && [ "$choice" -ge 1 ] && [ "$choice" -le 10 ]; then
        apply_option "$choice"
    else
        echo "Exiting."
    fi
}

cmd_restore_backup() {
    if [ ! -d "$BACKUP_DIR" ]; then
        echo "Error: Backup directory $BACKUP_DIR not found!"
        exit 1
    fi
    echo "Restoring configuration from $BACKUP_DIR to $HOME/.config/hypr/..."
    cp -a "$BACKUP_DIR"/* "$HOME/.config/hypr/"
    hyprctl reload >/dev/null 2>&1
    notify-send -a "Hyprland Blur" -i dialog-information "Undo Point Restored" "Restored original pre-blur configuration"
    echo "✓ Restored original configuration from undo point."
}

if [[ "$1" =~ ^[0-9]+$ ]] && [ "$1" -ge 1 ] && [ "$1" -le 10 ]; then
    apply_option "$1"
    exit 0
fi

case "${1:-menu}" in
    menu|ui)
        cmd_menu
        ;;
    list|ls)
        cmd_list
        ;;
    status)
        cmd_status
        ;;
    no-blur|none|off|disable)
        apply_option 1
        ;;
    drops)
        apply_option 2
        ;;
    fluid_jar|fluid)
        apply_option 3
        ;;
    prism)
        apply_option 4
        ;;
    frost)
        apply_option 5
        ;;
    acrylic)
        apply_option 6
        ;;
    aurora)
        apply_option 7
        ;;
    water)
        apply_option 8
        ;;
    heat_shimmer|shimmer)
        apply_option 9
        ;;
    kawase)
        apply_option 10
        ;;
    reset)
        apply_option 10
        ;;
    restore-backup|undo)
        cmd_restore_backup
        ;;
    help|-h|--help)
        cmd_list
        ;;
    *)
        echo "Unknown option: $1"
        cmd_list
        exit 1
        ;;
esac
