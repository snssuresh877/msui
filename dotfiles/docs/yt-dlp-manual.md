# yt-dlp Complete Practical Manual & Cheat Sheet

Comprehensive guide for downloading videos, streams, playlists, and audio using `yt-dlp` on Linux.

---

## 1. Quick Start & Speed Optimization

### Basic Download (Highest Quality)
```bash
yt-dlp "URL"
```

### Fast Multi-Threaded Download (8 Concurrent Connections)
By default, `yt-dlp` downloads single-threaded. Adding `-N 8` enables 8 parallel chunk connections, maximizing your bandwidth on HLS, YouTube, and generic web streams:
```bash
yt-dlp -N 8 "URL"
```

### Save to Custom Directory with Clean Filename
```bash
yt-dlp -N 8 -o "~/Downloads/%(title)s.%(ext)s" "URL"
```

---

## 2. HLS & .m3u8 Streams (The Stream Detector Workflow)

When capturing streams from custom players or web streaming sites:

### Basic Stream Download
```bash
yt-dlp -N 8 "https://example.com/playlist.m3u8"
```

### With Referer Header (Required if the server returns 403 Forbidden)
```bash
yt-dlp -N 8 --referer "https://website.com/watch/123" "https://example.com/playlist.m3u8"
```

### With Custom User-Agent & Headers
```bash
yt-dlp -N 8 \
  --user-agent "Mozilla/5.0 (X11; Linux x86_64; rv:130.0) Gecko/20100101 Firefox/130.0" \
  --referer "https://website.com/" \
  "https://example.com/playlist.m3u8"
```

### Live Stream Recording (Download from Start or Live)
```bash
# Record live stream from the very beginning (if supported by stream buffer):
yt-dlp --live-from-start "STREAM_URL"

# Record live stream continuously until stopped (Ctrl+C):
yt-dlp "STREAM_URL"
```

---

## 3. Video Quality & Format Selection

### List All Available Formats & Resolutions
Always check available video/audio streams first with `-F`:
```bash
yt-dlp -F "URL"
```
*Note the format IDs in the first column (e.g., `137` for 1080p video, `140` for m4a audio).*

### Download Specific Format by ID
```bash
# Combine video ID 137 and audio ID 140 into an MP4:
yt-dlp -f 137+140 "URL"
```

### Best Quality Capped at 1080p (Saves Bandwidth & Disk Space)
```bash
yt-dlp -f "bv*[height<=1080]+ba/b[height<=1080]" "URL"
```

### Best Quality Capped at 720p
```bash
yt-dlp -f "bv*[height<=720]+ba/b[height<=720]" "URL"
```

### Best MP4 Compatibility (Universal playback on TV/Mobile/QuickTime)
Forces H.264 video + AAC audio inside an MP4 container:
```bash
yt-dlp -f "bv*[ext=mp4]+ba[ext=m4a]/b[ext=mp4]" --merge-output-format mp4 "URL"
```

---

## 4. Audio Extraction & Music Downloads

### Extract Best Quality MP3 (320kbps)
```bash
yt-dlp -x --audio-format mp3 --audio-quality 0 "URL"
```

### Extract Lossless Audio (FLAC or WAV)
```bash
yt-dlp -x --audio-format flac "URL"
```

### Download Music with Embedded Thumbnail & Metadata
Embeds artist, title, album, and high-res cover art directly into the MP3 tags:
```bash
yt-dlp -x --audio-format mp3 --audio-quality 0 \
  --embed-thumbnail \
  --add-metadata \
  -o "~/Music/%(artist,uploader)s - %(title)s.%(ext)s" \
  "URL"
```

---

## 5. Playlists, Channels & Series

### Download Entire Playlist into Organized Folder
```bash
yt-dlp -o "~/Downloads/%(playlist)s/%(playlist_index)02d - %(title)s.%(ext)s" "PLAYLIST_URL"
```

### Download Specific Playlist Items
```bash
# Download only videos 1, 3, and 5 through 10:
yt-dlp --playlist-items 1,3,5-10 "PLAYLIST_URL"

# Download last 5 videos from a channel:
yt-dlp --playlist-end 5 "CHANNEL_URL"
```

### Channel Archiving (Never Re-download Already Downloaded Videos)
Creates a tracking file `archive.txt`. Running the command again will only download brand-new uploads:
```bash
yt-dlp --download-archive "~/Downloads/archive.txt" \
  -o "~/Downloads/%(uploader)s/%(upload_date)s - %(title)s.%(ext)s" \
  "CHANNEL_URL"
```

---

## 6. Subtitles & Closed Captions

### List Available Subtitles
```bash
yt-dlp --list-subs "URL"
```

### Download English Subtitles as Separate File (.srt / .vtt)
```bash
yt-dlp --write-sub --sub-lang en --convert-subs srt "URL"
```

### Download Auto-Generated Subtitles
```bash
yt-dlp --write-auto-sub --sub-lang en --convert-subs srt "URL"
```

### Embed Subtitles Directly Inside the Video File (Soft Subtitles)
```bash
yt-dlp --write-sub --sub-lang en --embed-subs "URL"
```

---

## 7. Video Trimming & Chapters

### Download Only a Specific Time Range (No Full Download Needed!)
Downloads only the specified segment without fetching the whole file:
```bash
# Download from 1 minute 30 seconds to 4 minutes 15 seconds:
yt-dlp --download-sections "*01:30-04:15" "URL"
```

### Split Video by Official Chapters
Automatically slices the video into individual MP4 files based on video timestamps/chapters:
```bash
yt-dlp --split-chapters -o "chapter:%(title)s - %(section_number)02d %(section_title)s.%(ext)s" "URL"
```

---

## 8. Authenticated & Members-Only Videos (Firefox Cookies)

If a video requires login, age confirmation, or a paid subscription that you have access to in Firefox:

### Borrow Active Session Cookies from Firefox
```bash
yt-dlp --cookies-from-browser firefox "URL"
```
*`yt-dlp` securely reads your active Firefox session cookies, allowing you to download private/members-only content effortlessly.*

---

## 9. Performance & Network Throttling

### Use Aria2 as External Multi-Connection Accelerator
```bash
yt-dlp --downloader aria2c --downloader-args "aria2c:-s 8 -x 8 -k 1M" "URL"
```

### Cap Download Speed (Prevent bandwidth saturation on shared networks)
```bash
yt-dlp --limit-rate 5M "URL"
```

---

## 10. Cheat Sheet Reference Table

| Goal | Command |
| :--- | :--- |
| **Fast 8-thread download** | `yt-dlp -N 8 "URL"` |
| **HLS / m3u8 stream** | `yt-dlp -N 8 --referer "PAGE_URL" "STREAM_URL"` |
| **Inspect formats** | `yt-dlp -F "URL"` |
| **Cap at 1080p** | `yt-dlp -f "bv*[height<=1080]+ba/b[height<=1080]" "URL"` |
| **Extract 320k MP3** | `yt-dlp -x --audio-format mp3 --audio-quality 0 "URL"` |
| **MP3 + Cover Art** | `yt-dlp -x --audio-format mp3 --embed-thumbnail --add-metadata "URL"` |
| **Download section** | `yt-dlp --download-sections "*02:00-05:00" "URL"` |
| **Embed subtitles** | `yt-dlp --write-sub --sub-lang en --embed-subs "URL"` |
| **Private / Login video** | `yt-dlp --cookies-from-browser firefox "URL"` |
| **Channel archive** | `yt-dlp --download-archive archive.txt "URL"` |
| **Our dotfiles helper** | `dl-stream` (auto-reads clipboard stream & downloads) |
