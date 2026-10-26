use std::process::Command;

pub struct SystemControl;

impl SystemControl {
    pub fn lock() {
        let _ = Command::new("hyprlock").spawn();
    }

    pub fn suspend() {
        let _ = Command::new("systemctl").arg("suspend").spawn();
    }

    pub fn reboot() {
        let _ = Command::new("systemctl").arg("reboot").spawn();
    }

    pub fn poweroff() {
        let _ = Command::new("systemctl").arg("poweroff").spawn();
    }

    pub fn logout() {
        let _ = Command::new("hyprctl")
            .args(["dispatch", "exit"])
            .spawn();
    }

    pub fn screenshot() {
        let script = r##"
sleep 0.15
DIR="$HOME/Pictures/Screenshots"
mkdir -p "$DIR"
FILE="$DIR/Screenshot_$(date +%Y%m%d_%H%M%S).png"

if command -v slurp >/dev/null 2>&1; then
    GEOM=$(slurp -d -b "#00000044" -c "#00d4ffaa" -w 2 2>/dev/null)
    STATUS=$?
    if [ $STATUS -eq 0 ] && [ -n "$GEOM" ]; then
        grim -g "$GEOM" "$FILE"
    else
        exit 0
    fi
else
    grim "$FILE"
fi

if [ -f "$FILE" ]; then
    command -v wl-copy >/dev/null 2>&1 && wl-copy < "$FILE"
    command -v notify-send >/dev/null 2>&1 && notify-send -a msUI -i "$FILE" "Screenshot Saved" "Copied to clipboard and saved to ~/Pictures/Screenshots"
fi
"##;
        let _ = Command::new("sh")
            .arg("-c")
            .arg(script)
            .spawn();
    }

    pub fn launch_terminal(preferred: &str) {
        let preferred = preferred.trim();
        if !preferred.is_empty() {
            if Self::spawn_terminal_cmd(preferred) {
                return;
            }
        }

        const FALLBACK_TERMINALS: &[&str] = &[
            "kitty",
            "foot",
            "alacritty",
            "ghostty",
            "wezterm",
            "xterm",
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
            "terminator",
        ];

        for term in FALLBACK_TERMINALS {
            if *term != preferred && Self::spawn_terminal_cmd(term) {
                tracing::info!("Launched fallback terminal: {}", term);
                return;
            }
        }

        let _ = Command::new("notify-send")
            .args([
                "-a",
                "msUI",
                "-i",
                "utilities-terminal",
                "Terminal Launch Error",
                &format!("Could not launch terminal '{}' or any fallback (kitty, foot, etc.)", preferred),
            ])
            .spawn();
    }

    fn spawn_terminal_cmd(cmd_name: &str) -> bool {
        match Command::new(cmd_name).spawn() {
            Ok(mut child) => {
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                true
            }
            Err(_) => false,
        }
    }
}
