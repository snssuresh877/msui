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
        // Runs grim with slurp region selection
        let cmd = "grim -g \"$(slurp)\" \"$HOME/Pictures/Screenshot_$(date +%Y%m%d_%H%M%S).png\"";
        let _ = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .spawn();
    }
}
