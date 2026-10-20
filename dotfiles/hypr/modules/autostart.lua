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
	-- POLKIT (MSUI NATIVE RUST + ICED AUTHENTICATION AGENT)
	--------------------------------------------------------

	hl.exec_cmd("pgrep -f 'msui polkit' >/dev/null || setsid $HOME/.local/bin/msui polkit >/tmp/msui-polkit.log 2>&1 &")

	--------------------------------------------------------
	-- WALLPAPER (MSUI NATIVE WAYLAND WALLPAPER DAEMON)
	--------------------------------------------------------

	hl.exec_cmd("pgrep -f 'msui bg' >/dev/null || (sleep 0.2 && setsid $HOME/.local/bin/msui bg >/tmp/msui-bg.log 2>&1) &")

	--------------------------------------------------------
	-- DESKTOP UI SHELL (MSUI NATIVE RUST + ICED)
	--------------------------------------------------------

	hl.exec_cmd("pgrep -f 'msui bar' >/dev/null || (sleep 0.3 && setsid $HOME/.local/bin/msui bar >/tmp/msui-bar.log 2>&1) &")
	hl.exec_cmd("(sleep 2 && date '+%Y-%m-%d %H:%M:%S Boot Verification' && pgrep -a msui) >/tmp/msui-boot.log 2>&1 &")

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
