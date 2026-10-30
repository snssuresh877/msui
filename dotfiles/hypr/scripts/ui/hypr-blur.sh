#!/usr/bin/env bash
# hypr-blur: Interactive switcher and manager for Hyprland 0.57+ blur shaders and variants
set -e

BACKUP_DIR="$HOME/.config/hypr_backup_pre_blur_057"
CONFIG_FILE="$HOME/.config/hypr/modules/appearance.lua"

VARIANTS=(
    "kawase"
    "frost"
    "ripple"
    "drops"
    "water"
    "fluid_jar"
    "prism"
    "heat_shimmer"
    "acrylic"
    "aurora"
    "haze"
)

DESCRIPTIONS=(
    "Standard dual-kawase blur (default, lightweight & clean)"
    "Frosted glass effect with surface micro-texture"
    "Interactive ripple distortion wave effect"
    "Animated liquid droplets / rain on glass"
    "Animated water waves and fluid surface displacement"
    "Viscous fluid physics container"
    "Rainbow chromatic dispersion & prism refraction"
    "Animated heat waves & atmospheric mirage shimmer"
    "Modern acrylic with chromatic aberration & light diffusion"
    "Animated northern lights / aurora borealis ambient color glow"
    "Iridescent atmospheric color haze"
)

TOTAL=${#VARIANTS[@]}

get_current_index() {
    local cur
    cur=$(hyprctl -j getoption decoration:blur:variant 2>/dev/null | jq -r '.int // 0')
    if [[ "$cur" =~ ^[0-9]+$ ]] && [ "$cur" -ge 0 ] && [ "$cur" -lt "$TOTAL" ]; then
        echo "$cur"
    else
        echo 0
    fi
}

get_current_name() {
    local idx
    idx=$(get_current_index)
    echo "${VARIANTS[$idx]}"
}

apply_blur() {
    local var="$1"
    local size="${2:-5}"
    local passes="${3:-2}"
    local prop="${4:-true}"
    
    # If resetting to kawase default, we can use original minimal settings if requested
    if [ "$var" = "kawase" ] && [ -z "$2" ]; then
        size=2
        passes=1
        prop=false
    fi

    local lua_code="hl.config({ decoration = { blur = { enabled = true, variant = \"$var\", size = $size, passes = $passes, proportional = $prop } } })"
    hyprctl repl "$lua_code" >/dev/null 2>&1
    
    notify-send -a "Hyprland 0.57" -i preferences-desktop-theme \
        "Blur Variant: $var" \
        "Size: ${size} | Passes: ${passes} | Proportional: ${prop}"
    
    echo "✓ Set blur variant to: $var (size=$size, passes=$passes, proportional=$prop)"
}

cmd_status() {
    local idx
    idx=$(get_current_index)
    local name="${VARIANTS[$idx]}"
    local desc="${DESCRIPTIONS[$idx]}"
    local size passes prop enabled
    size=$(hyprctl -j getoption decoration:blur:size 2>/dev/null | jq -r '.int // 2')
    passes=$(hyprctl -j getoption decoration:blur:passes 2>/dev/null | jq -r '.int // 1')
    prop=$(hyprctl -j getoption decoration:blur:proportional 2>/dev/null | jq -r '.bool // false')
    enabled=$(hyprctl -j getoption decoration:blur:enabled 2>/dev/null | jq -r '.bool // true')

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo " Hyprland 0.57 Blur Status"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo " Enabled:      $enabled"
    echo " Current:      [$idx] $name"
    echo " Description:  $desc"
    echo " Size:         $size"
    echo " Passes:       $passes"
    echo " Proportional: $prop"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo " Undo point available at: $BACKUP_DIR"
}

cmd_list() {
    local cur_idx
    cur_idx=$(get_current_index)
    echo "Available Hyprland 0.57 Blur Variants:"
    echo "------------------------------------------------------------"
    for i in "${!VARIANTS[@]}"; do
        local marker="  "
        if [ "$i" -eq "$cur_idx" ]; then
            marker="► "
        fi
        printf "%s[%2d] %-14s - %s\n" "$marker" "$i" "${VARIANTS[$i]}" "${DESCRIPTIONS[$i]}"
    done
    echo "------------------------------------------------------------"
    echo "Usage: hypr-blur set <name_or_index> | next | prev | reset | boost | restore-backup"
}

cmd_next() {
    local cur_idx
    cur_idx=$(get_current_index)
    local next_idx=$(( (cur_idx + 1) % TOTAL ))
    local name="${VARIANTS[$next_idx]}"
    apply_blur "$name" 5 2 true
}

cmd_prev() {
    local cur_idx
    cur_idx=$(get_current_index)
    local prev_idx=$(( (cur_idx - 1 + TOTAL) % TOTAL ))
    local name="${VARIANTS[$prev_idx]}"
    apply_blur "$name" 5 2 true
}

cmd_set() {
    local target="$1"
    if [ -z "$target" ]; then
        echo "Error: Missing variant name or index."
        echo "Run 'hypr-blur list' to see all available variants."
        exit 1
    fi

    # Check if target is a number
    if [[ "$target" =~ ^[0-9]+$ ]] && [ "$target" -ge 0 ] && [ "$target" -lt "$TOTAL" ]; then
        apply_blur "${VARIANTS[$target]}" 5 2 true
        return
    fi

    # Check by name
    for i in "${!VARIANTS[@]}"; do
        if [ "${VARIANTS[$i]}" = "$target" ]; then
            apply_blur "$target" 5 2 true
            return
        fi
    done

    echo "Error: Unknown variant '$target'."
    echo "Run 'hypr-blur list' to see valid variants."
    exit 1
}

cmd_reset() {
    apply_blur "kawase" 2 1 false
    notify-send -a "Hyprland 0.57" -i preferences-desktop-theme "Blur Reset" "Restored baseline standard kawase blur (size=2, passes=1)"
    echo "✓ Reset to baseline kawase blur (size=2, passes=1, proportional=false)"
}

cmd_boost() {
    local cur_name
    cur_name=$(get_current_name)
    apply_blur "$cur_name" 6 3 true
    echo "✓ Boosted blur quality for '$cur_name' (size=6, passes=3, proportional=true)"
}

cmd_save() {
    local cur_idx
    cur_idx=$(get_current_index)
    local name="${VARIANTS[$cur_idx]}"
    echo "Saving '$name' into $CONFIG_FILE..."
    sed -i -E "s/variant = \"[^\"]*\"/variant = \"$name\"/" "$CONFIG_FILE" || true
    echo "✓ Saved into $CONFIG_FILE"
}

cmd_restore_backup() {
    if [ ! -d "$BACKUP_DIR" ]; then
        echo "Error: Backup directory $BACKUP_DIR not found!"
        exit 1
    fi
    echo "Restoring configuration from $BACKUP_DIR to $HOME/.config/hypr/..."
    cp -a "$BACKUP_DIR"/* "$HOME/.config/hypr/"
    hyprctl reload >/dev/null 2>&1
    notify-send -a "Hyprland 0.57" -i dialog-information "Undo Point Restored" "Restored original pre-blur configuration successfully"
    echo "✓ Restored original configuration from undo point."
}

case "${1:-status}" in
    status)
        cmd_status
        ;;
    list|ls)
        cmd_list
        ;;
    next|n)
        cmd_next
        ;;
    prev|p)
        cmd_prev
        ;;
    set|s)
        cmd_set "$2"
        ;;
    reset|r)
        cmd_reset
        ;;
    boost|b)
        cmd_boost
        ;;
    save)
        cmd_save
        ;;
    restore-backup|undo)
        cmd_restore_backup
        ;;
    help|-h|--help)
        echo "hypr-blur - Manage and test Hyprland 0.57 blur shaders live"
        echo ""
        echo "Commands:"
        echo "  status            Show current blur settings and variant"
        echo "  list              List all 11 blur variants"
        echo "  next              Cycle to next blur variant"
        echo "  prev              Cycle to previous blur variant"
        echo "  set <name|idx>    Switch directly to a variant (e.g. acrylic, aurora, water, drops...)"
        echo "  boost             Boost blur radius & passes (size=6, passes=3) for vivid shader testing"
        echo "  reset             Reset to lightweight baseline kawase blur (size=2, passes=1)"
        echo "  restore-backup    Undo everything: restore original ~/.config/hypr from backup point"
        ;;
    *)
        echo "Unknown command: $1"
        echo "Run 'hypr-blur help' for usage."
        exit 1
        ;;
esac
