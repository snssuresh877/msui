if status is-login
    if test (tty) = /dev/tty1
        if test -z "$SSH_CONNECTION"
            if test -z "$WAYLAND_DISPLAY"
                if test -z "$DISPLAY"
                    exec start-hyprland
                end
            end
        end
    end
end
