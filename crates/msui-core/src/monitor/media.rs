use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaInfo {
    pub player_name: String,
    pub is_playing: bool,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_url: Option<String>,
    pub position_secs: u64,
    pub length_secs: u64,
    pub loop_status: Option<String>,
    pub shuffle: bool,
}

impl Default for MediaInfo {
    fn default() -> Self {
        Self {
            player_name: String::new(),
            is_playing: false,
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            art_url: None,
            position_secs: 0,
            length_secs: 0,
            loop_status: None,
            shuffle: false,
        }
    }
}

pub struct MediaMonitor;

impl MediaMonitor {
    pub fn read() -> Option<MediaInfo> {
        Self::read_player(None)
    }

    pub fn list_players() -> Vec<String> {
        let output = match Command::new("playerctl").arg("-l").output() {
            Ok(o) => o,
            Err(_) => return Vec::new(),
        };

        if !output.status.success() {
            return Vec::new();
        }

        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    pub fn read_player(player: Option<&str>) -> Option<MediaInfo> {
        let mut cmd = Command::new("playerctl");
        if let Some(p) = player {
            cmd.args(["-p", p]);
        }
        cmd.args([
            "metadata",
            "--format",
            "{{playerName}}@@@{{title}}@@@{{artist}}@@@{{album}}@@@{{mpris:artUrl}}@@@{{position}}@@@{{mpris:length}}@@@{{status}}",
        ]);

        let output = cmd.output().ok()?;
        if !output.status.success() {
            return None;
        }

        let line = String::from_utf8_lossy(&output.stdout);
        Self::parse_metadata_line(&line)
    }

    pub fn parse_metadata_line(raw: &str) -> Option<MediaInfo> {
        let parts: Vec<&str> = raw.trim().split("@@@").collect();
        if parts.len() < 8 {
            return None;
        }

        let player_name = parts[0].trim().to_string();
        let title = parts[1].trim().to_string();
        let artist = parts[2].trim().to_string();
        let album = parts[3].trim().to_string();
        let art_raw = parts[4].trim();
        let art_url = if art_raw.is_empty() {
            None
        } else {
            Some(art_raw.to_string())
        };

        // Position: can be microseconds or seconds with float
        let pos_str = parts[5].trim();
        let position_secs = if let Ok(val) = pos_str.parse::<u64>() {
            if val > 1_000_000 {
                val / 1_000_000
            } else {
                val
            }
        } else if let Ok(val) = pos_str.parse::<f64>() {
            val.round() as u64
        } else {
            0
        };

        // Length: in microseconds
        let len_str = parts[6].trim();
        let length_secs = if let Ok(val) = len_str.parse::<u64>() {
            if val > 1_000_000 {
                val / 1_000_000
            } else {
                val
            }
        } else if let Ok(val) = len_str.parse::<f64>() {
            val.round() as u64
        } else {
            0
        };

        let status = parts[7].trim().to_lowercase();
        let is_playing = status == "playing";

        if title.is_empty() && artist.is_empty() && !is_playing {
            return None;
        }

        Some(MediaInfo {
            player_name,
            is_playing,
            title,
            artist,
            album,
            art_url,
            position_secs,
            length_secs,
            loop_status: None,
            shuffle: false,
        })
    }

    pub fn play() {
        let _ = Command::new("playerctl").arg("play").spawn();
    }

    pub fn pause() {
        let _ = Command::new("playerctl").arg("pause").spawn();
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

    pub fn seek(position_secs: u64) {
        let _ = Command::new("playerctl")
            .args(["position", &position_secs.to_string()])
            .spawn();
    }

    pub fn seek_relative(offset_secs: i64) {
        let arg = if offset_secs >= 0 {
            format!("{}+", offset_secs)
        } else {
            format!("{}-", -offset_secs)
        };
        let _ = Command::new("playerctl")
            .args(["position", &arg])
            .spawn();
    }

    pub fn toggle_shuffle() {
        let _ = Command::new("playerctl")
            .args(["shuffle", "Toggle"])
            .spawn();
    }

    pub fn cycle_loop() {
        // Playerctl loop can be None, Track, Playlist
        let _ = Command::new("playerctl")
            .args(["loop", "Track"])
            .spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metadata_line() {
        let sample = "spotify@@@Blinding Lights@@@The Weeknd@@@After Hours@@@https://example.com/art.jpg@@@45000000@@@200000000@@@Playing";
        let info = MediaMonitor::parse_metadata_line(sample).unwrap();
        assert_eq!(info.player_name, "spotify");
        assert_eq!(info.title, "Blinding Lights");
        assert_eq!(info.artist, "The Weeknd");
        assert_eq!(info.album, "After Hours");
        assert_eq!(info.position_secs, 45);
        assert_eq!(info.length_secs, 200);
        assert!(info.is_playing);
        assert_eq!(info.art_url.as_deref(), Some("https://example.com/art.jpg"));
    }
}
