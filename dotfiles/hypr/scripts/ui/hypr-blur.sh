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
    "Micro-textured winter frosted glass surface (crisp crystal grain)"
    "Modern acrylic glass with max light diffusion & chromatic aberration"
    "Undulating Northern Lights / Aurora Borealis neon ambient glow"
    "Fluid wave displacement equations rippling across background"
    "Atmospheric heat haze mirage distortion waves"
    "Standard dual-kawase pyramid blur (clean & lightweight baseline)"
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
        echo "1"
        return
    fi
    # Map variant index in Hyprland (0..10) to our 1..10 options
    # Hyprland indices:
    # 0: kawase -> option 10
    # 1: frost -> option 5
    # 2: ripple
    # 3: drops -> option 2
    # 4: water -> option 8
    # 5: fluid_jar -> option 3
    # 6: prism -> option 4
    # 7: heat_shimmer -> option 9
    # 8: acrylic -> option 6
    # 9: aurora -> option 7
    # 10: haze
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
    local snippet=""
    local enabled="true"

    case "$num" in
        1|"no-blur"|"none"|"off"|"disable")
            enabled="false"
            opt_name="No blur"
            lua_code='hl.config({ decoration = { blur = { enabled = false } } })'
            snippet='		blur = {
			enabled = false,
			variant = "kawase",
			size = 2,
			passes = 1,
			proportional = false,
		},'
            ;;
        2|"drops")
            opt_name="drops"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "drops", size = 8, passes = 3, vibrancy = 0.6, contrast = 1.1, proportional = true, ignore_opacity = true, new_optimizations = true, drops = { speed = 3.5 } } } })'
            snippet='		blur = {
			enabled = true,
			variant = "drops", -- Active: drops
			size = 8,
			passes = 3,
			vibrancy = 0.6,
			contrast = 1.1,
			proportional = true,
			ignore_opacity = true,
			new_optimizations = true,
			drops = {
				speed = 3.5,
			},
		},'
            ;;
        3|"fluid_jar")
            opt_name="fluid_jar"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "fluid_jar", size = 8, passes = 3, vibrancy = 0.6, contrast = 1.1, proportional = true, ignore_opacity = true, new_optimizations = true, fluid_jar = { fill_amount = 0.52, turbulence = 1.5, speed = 4.0 } } } })'
            snippet='		blur = {
			enabled = true,
			variant = "fluid_jar", -- Active: fluid_jar
			size = 8,
			passes = 3,
			vibrancy = 0.6,
			contrast = 1.1,
			proportional = true,
			ignore_opacity = true,
			new_optimizations = true,
			fluid_jar = {
				fill_amount = 0.52,
				turbulence = 1.5,
				speed = 4.0,
			},
		},'
            ;;
        4|"prism")
            opt_name="prism"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "prism", size = 8, passes = 3, vibrancy = 0.7, contrast = 1.15, proportional = true, ignore_opacity = true, new_optimizations = true, glass = { size = 45.0, refraction = 20.0, roughness = 1.0 } } } })'
            snippet='		blur = {
			enabled = true,
			variant = "prism", -- Active: prism
			size = 8,
			passes = 3,
			vibrancy = 0.7,
			contrast = 1.15,
			proportional = true,
			ignore_opacity = true,
			new_optimizations = true,
			glass = {
				size = 45.0,
				refraction = 20.0,
				roughness = 1.0,
			},
		},'
            ;;
        5|"frost")
            opt_name="frost"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "frost", size = 3, passes = 1, vibrancy = 0.8, contrast = 1.25, proportional = true, ignore_opacity = true, new_optimizations = true, glass = { size = 18.0, refraction = 20.0, roughness = 1.0 } } } })'
            snippet='		blur = {
			enabled = true,
			variant = "frost", -- Active: frost
			size = 3,
			passes = 1,
			vibrancy = 0.8,
			contrast = 1.25,
			proportional = true,
			ignore_opacity = true,
			new_optimizations = true,
			glass = {
				size = 18.0,
				refraction = 20.0,
				roughness = 1.0,
			},
		},'
            ;;
        6|"acrylic")
            opt_name="acrylic"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "acrylic", size = 8, passes = 3, vibrancy = 0.6, contrast = 1.1, proportional = true, ignore_opacity = true, new_optimizations = true, acrylic = { aberration = 0.25, refraction = 48.0, bulb = 120.0, clarity = 0.80 } } } })'
            snippet='		blur = {
			enabled = true,
			variant = "acrylic", -- Active: acrylic
			size = 8,
			passes = 3,
			vibrancy = 0.6,
			contrast = 1.1,
			proportional = true,
			ignore_opacity = true,
			new_optimizations = true,
			acrylic = {
				aberration = 0.25,
				refraction = 48.0,
				bulb = 120.0,
				clarity = 0.80,
			},
		},'
            ;;
        7|"aurora")
            opt_name="aurora"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "aurora", size = 6, passes = 2, vibrancy = 0.8, contrast = 1.15, proportional = true, ignore_opacity = true, new_optimizations = true, aurora = { intensity = 1.0, speed = 2.5, color1 = "rgba(00ff88ff)", color2 = "rgba(ff00aaff)" } } } })'
            snippet='		blur = {
			enabled = true,
			variant = "aurora", -- Active: aurora
			size = 6,
			passes = 2,
			vibrancy = 0.8,
			contrast = 1.15,
			proportional = true,
			ignore_opacity = true,
			new_optimizations = true,
			aurora = {
				intensity = 1.0,
				speed = 2.5,
				color1 = "rgba(00ff88ff)",
				color2 = "rgba(ff00aaff)",
			},
		},'
            ;;
        8|"water")
            opt_name="water"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "water", size = 4, passes = 2, vibrancy = 0.7, contrast = 1.15, proportional = true, ignore_opacity = true, new_optimizations = true, water = { strength = 32.0, radius = 80.0, speed = 2.0, damping = 0.98, duration = 15.0 } } } })'
            snippet='		blur = {
			enabled = true,
			variant = "water", -- Active: water
			size = 4,
			passes = 2,
			vibrancy = 0.7,
			contrast = 1.15,
			proportional = true,
			ignore_opacity = true,
			new_optimizations = true,
			water = {
				strength = 32.0,
				radius = 80.0,
				speed = 2.0,
				damping = 0.98,
				duration = 15.0,
			},
		},'
            ;;
        9|"heat_shimmer")
            opt_name="heat_shimmer"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "heat_shimmer", size = 4, passes = 2, vibrancy = 0.7, contrast = 1.2, proportional = true, ignore_opacity = true, new_optimizations = true, heat_shimmer = { speed = 3.5 }, glass = { size = 28.0, refraction = 20.0, roughness = 1.0 } } } })'
            snippet='		blur = {
			enabled = true,
			variant = "heat_shimmer", -- Active: heat_shimmer
			size = 4,
			passes = 2,
			vibrancy = 0.7,
			contrast = 1.2,
			proportional = true,
			ignore_opacity = true,
			new_optimizations = true,
			heat_shimmer = {
				speed = 3.5,
			},
			glass = {
				size = 28.0,
				refraction = 20.0,
				roughness = 1.0,
			},
		},'
            ;;
        10|"kawase")
            opt_name="kawase"
            lua_code='hl.config({ decoration = { blur = { enabled = true, variant = "kawase", size = 4, passes = 2, vibrancy = 0.5, contrast = 1.0, proportional = false, ignore_opacity = true, new_optimizations = true } } })'
            snippet='		blur = {
			enabled = true,
			variant = "kawase", -- Active: kawase
			size = 4,
			passes = 2,
			vibrancy = 0.5,
			contrast = 1.0,
			proportional = false,
			ignore_opacity = true,
			new_optimizations = true,
		},'
            ;;
        *)
            echo "Error: Invalid option '$num'. Must be 1 to 10."
            echo "Run 'hypr-blur list' to see all 10 options."
            exit 1
            ;;
    esac

    # Apply live immediately
    hyprctl repl "$lua_code" >/dev/null 2>&1

    # Persist in appearance.lua (both active config and dotfiles)
    export BLUR_SNIPPET="$snippet"
    python3 -c "
import os

snippet = os.environ.get('BLUR_SNIPPET', '')
if not snippet:
    exit(0)

def replace_blur(filepath):
    try:
        if not os.path.exists(filepath):
            return
        with open(filepath, 'r') as f:
            text = f.read()
        start = text.find('blur = {')
        if start == -1:
            return
        depth = 0
        end = start
        for i in range(start, len(text)):
            if text[i] == '{':
                depth += 1
            elif text[i] == '}':
                depth -= 1
                if depth == 0:
                    end = i + 1
                    if end < len(text) and text[end] == ',':
                        end += 1
                    break
        new_text = text[:start] + snippet.strip() + text[end:]
        with open(filepath, 'w') as f:
            f.write(new_text)
    except Exception:
        pass

replace_blur('$CONFIG_FILE')
replace_blur('$DOTFILES_CONFIG')
"

    # Send notification
    if [ "$enabled" = "false" ]; then
        notify-send -a "Hyprland Blur" -i dialog-information "Blur Disabled" "Window & layer blur turned off (Option 1)"
        echo "✓ Disabled blur (Option 1: No blur)"
    else
        notify-send -a "Hyprland Blur" -i preferences-desktop-theme "Blur: $opt_name" "Applied option $num ($opt_name) with boosted parameters"
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

    local cur
    cur=$(get_current_status)
    local cur_name="Unknown"
    if [ "$cur" -ge 1 ] && [ "$cur" -le 10 ]; then
        cur_name="${OPTIONS[$(( cur - 1 ))]}"
    fi

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo " Hyprland 0.57 Blur Status (0 MB Resident RAM)"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo " Blur Enabled: $enabled"
    echo " Active Option: [$cur] $cur_name (Internal Variant ID: $var)"
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

    # If fzf is available and interactive terminal, provide sleek fuzzy menu
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

    # Fallback to fast interactive bash read
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

# Direct numeric argument handling: hypr-blur 1..10
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
