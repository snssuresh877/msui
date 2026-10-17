use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaInfo {
    pub is_playing: bool,
    pub title: String,
    pub artist: String,
}

impl Default for MediaInfo {
    fn default() -> Self {
        Self {
            is_playing: false,
            title: String::new(),
            artist: String::new(),
        }
    }
}

pub struct MediaMonitor;

impl MediaMonitor {
    pub fn read() -> Option<MediaInfo> {
        let status_out = Command::new("playerctl")
            .arg("status")
            .output()
            .ok()?;

        let status_str = String::from_utf8_lossy(&status_out.stdout).trim().to_lowercase();
        if status_str.is_empty() || status_str == "stopped" {
            return None;
        }

        let is_playing = status_str == "playing";

        let title_out = Command::new("playerctl")
            .args(["metadata", "title"])
            .output()
            .ok();
        let artist_out = Command::new("playerctl")
            .args(["metadata", "artist"])
            .output()
            .ok();

        let title = title_out
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        let artist = artist_out
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();

        if title.is_empty() && artist.is_empty() {
            return None;
        }

        Some(MediaInfo {
            is_playing,
            title,
            artist,
        })
    }

    pub fn play_pause() {
        let _ = Command::new("playerctl").arg("play-pause").spawn();
    }

    pub fn next() {
        let _ = Command::new("playerctl").arg("next").spawn();
    }

    pub fn previous() {
        let _ = Command::new("playerctl").arg("previous").spawn();
    }
}
