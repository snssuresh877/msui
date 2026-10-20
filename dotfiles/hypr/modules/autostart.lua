-- ~/.config/hypr/modules/autostart.lua

------------------------------------------------------------
-- HYPRLAND START
------------------------------------------------------------

hl.on("hyprland.start", function()
	--------------------------------------------------------
	-- DBUS / SYSTEMD ENVIRONMENT
	--------------------------------------------------------

	hl.exec_cmd(
		"dbus-update-activation-environment --systemd "
			.. "WAYLAND_DISPLAY DISPLAY XDG_CURRENT_DESKTOP=Hyprland "
			.. "XDG_SESSION_TYPE XDG_SESSION_DESKTOP XDG_DATA_DIRS SSH_AUTH_SOCK"
	)

	--------------------------------------------------------
	-- FLATPAK DESKTOP APPLICATION INDEXER
	--------------------------------------------------------

	hl.exec_cmd("$HOME/.config/hypr/scripts/ui/sync_flatpak_desktop.sh &")

	--------------------------------------------------------
	-- MSUI DESKTOP SUITE (POLKIT, WALLPAPER, BAR)
	--------------------------------------------------------

	hl.exec_cmd("$HOME/.config/hypr/scripts/ui/start_msui.sh &")

	--------------------------------------------------------
	-- SYSTEM UPDATES NOTIFIER
	--------------------------------------------------------

	hl.exec_cmd("(sleep 5 && $HOME/.config/hypr/scripts/ui/check_updates.sh) &")

	--------------------------------------------------------
	-- HYPRIDLE
	--------------------------------------------------------

	hl.exec_cmd("pgrep -x hypridle >/dev/null || hypridle &")

	--------------------------------------------------------
	-- KEYBINDINGS DESKTOP ENTRIES SYNC
	--------------------------------------------------------

	hl.exec_cmd("$HOME/.config/hypr/scripts/ui/sync_keybind_desktop_entries.sh &")

	--------------------------------------------------------
	-- CLIPBOARD & NOTIFICATIONS (Managed natively by msUI)
	--------------------------------------------------------
	-- msUI automatically monitors Wayland clipboard in-process and runs D-Bus notification service.

	--------------------------------------------------------
	--------------------------------------------------------
end)
