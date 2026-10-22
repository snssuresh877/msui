-- ~/.config/hypr/modules/variables.lua
--
-- Central Hyprland variables
--
-- Design:
--   • Namespaced configuration
--   • Applications in Apps
--   • Launchers in Launcher
--   • UI components in UI
--   • Commands grouped by function
--   • msUI (Rust + Iced) is the native desktop shell
--
-- Other modules should consume these values.
-- This file contains configuration values only.

------------------------------------------------------------
-- MOD KEY
------------------------------------------------------------

MOD = "SUPER"

------------------------------------------------------------
-- APPLICATIONS
------------------------------------------------------------

Apps = {

	-- Core applications
	terminal = "foot",

	secondTerminal = "kitty",

	browser = "firefox",

	fileManager = "msui-fm",

	codeEditor = "foot -e nvim",

	textEditor = "foot -e nvim",

	officeSoftware = "onlyoffice",

	settingsApp = "nwg-look",

	volumeMixer = "pavucontrol",

	taskManager = "foot -e btop",

	-- Productivity
	notesApp = "obsidian",

	taskApp = "flatpak run com.super_productivity.SuperProductivity",

	-- Communication
	wechatApp = "flatpak run com.tencent.WeChat",
}

------------------------------------------------------------
-- UI BACKENDS
------------------------------------------------------------

UI = {

	launcher = "msui",

	-- Keep Rofi available as fallback
	rofi = "rofi",

	bar = "msui",

	notifications = "msui",

	wallpaper = "msui",
}

------------------------------------------------------------
-- LAUNCHERS
------------------------------------------------------------

Launchers = {

	--------------------------------------------------------
	-- MSUI (NATIVE RUST + ICED WAYLAND SHELL)
	--------------------------------------------------------

	msui = {
		app = "msui launcher",
		run = "msui launcher",
		window = "msui launcher",
		clipboard = "msui clipboard",
		controlCenter = "msui control",
		keybinds = "msui drawer",
		network = "msui wifi",
		reminders = "msui reminder",
		activeApps = "msui drawer",
		audio = "msui audio",
		bluetooth = "msui bluetooth",
		storage = "msui storage",
		display = "msui display",
		media = "msui media",
		theme = "msui theme",
		wallpaper = "msui wallpaper",
		power = "msui power",
	},

	--------------------------------------------------------
	-- ROFI (FALLBACK)
	--------------------------------------------------------

	rofi = {
		app = "rofi -show drun",
		run = "rofi -show run",
		window = "rofi -show window",
		clipboard = "cliphist list | rofi -dmenu | cliphist decode | wl-copy",
		emoji = "rofimoji",
	},

	--------------------------------------------------------
	-- POWER
	--------------------------------------------------------

	power = "msui power 2>/dev/null || wlogout",
}

------------------------------------------------------------
-- NOTIFICATIONS
------------------------------------------------------------

Notifications = {

	daemon = "msui",

	toggleDND = "msui control dnd toggle 2>/dev/null || swaync-client -d -sw",

	dismissAll = "msui notifs clear 2>/dev/null || swaync-client -C",

	restore = "msui notifications 2>/dev/null || swaync-client -t -sw",
}

------------------------------------------------------------
-- SCREENSHOTS
------------------------------------------------------------

Screenshots = {

	region = "hyprshot --freeze --clipboard-only --mode region --silent",

	fullscreen = "grim - | wl-copy",

	save = "grim ~/Pictures/Screenshots/Screenshot-$(date +%F-%T).png",
}

------------------------------------------------------------
-- WALLPAPER
------------------------------------------------------------

Wallpaper = {

	directory = "$HOME/Pictures/wallpapers",

	script = "msui wallpaper cycle",

	pywalCache = "$HOME/.cache/wal",
}

------------------------------------------------------------
-- BAR
------------------------------------------------------------

Waybar = {
	toggle = "$HOME/.config/hypr/scripts/ui/smart_bar_toggle.sh",
}

------------------------------------------------------------
-- SESSION
------------------------------------------------------------

Session = {

	lock = "$HOME/.config/hypr/scripts/ui/lock_screen.sh",

	logout = "msui power 2>/dev/null || $HOME/.config/hypr/scripts/ui/logout_session.sh",

	suspend = "loginctl lock-session; systemctl suspend",

	reboot = "$HOME/.config/hypr/scripts/ui/reboot_system.sh",

	shutdown = "$HOME/.config/hypr/scripts/ui/power_off.sh",
}

------------------------------------------------------------
-- MEDIA
------------------------------------------------------------

Media = {

	playPause = "msui media play-pause 2>/dev/null || playerctl play-pause",

	next = "msui media next 2>/dev/null || playerctl next",

	previous = "msui media prev 2>/dev/null || playerctl previous",
}

------------------------------------------------------------
-- AUDIO
------------------------------------------------------------

Audio = {

	volumeUp = "msui osd volume up 2>/dev/null || wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+",

	volumeDown = "msui osd volume down 2>/dev/null || wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-",

	mute = "msui osd volume mute 2>/dev/null || wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle",

	micMute = "msui audio mute 2>/dev/null || wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle",
}

------------------------------------------------------------
-- BRIGHTNESS
------------------------------------------------------------

Brightness = {

	up = "msui osd brightness up 2>/dev/null || brightnessctl set 5%+",

	down = "msui osd brightness down 2>/dev/null || brightnessctl set 5%-",
}

------------------------------------------------------------
-- SYSTEM
------------------------------------------------------------

System = {

	update = "$HOME/.config/hypr/scripts/system/system.sh",

	clean = "$HOME/.config/hypr/scripts/system/clean.sh",

	memory = "$HOME/.config/hypr/scripts/system/optimize-memory.sh",

	monitor = "foot -e btop",

	bluetooth = "msui bluetooth 2>/dev/null || blueman-manager",

	network = "msui wifi 2>/dev/null || nm-connection-editor",
}

------------------------------------------------------------
-- HYPRLAND
------------------------------------------------------------

Hyprland = {

	reload = "hyprctl reload",

	exit = "msui power 2>/dev/null || hyprctl dispatch exit",
}

------------------------------------------------------------
-- CURSOR
------------------------------------------------------------

Cursor = {

	theme = "Bibata-Modern-Ice",

	size = 24,
}
