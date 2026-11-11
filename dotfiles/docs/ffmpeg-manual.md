# FFmpeg Complete Practical Manual & Hardware Tuning Guide

A battle-tested, copy-paste ready manual for video and audio conversion, compression, resolution scaling, and AMD GPU acceleration on Arch Linux.

---

## 1. Golden Rules & Recommendations

1. **Always use `-c copy` whenever possible:**
   If you only need to change the file format (container) or trim a video without changing resolution, **never re-encode**. `-c copy` takes **1 second**, causes **zero quality loss**, and uses virtually **0% CPU**.
2. **For downscaling, use `-vf "scale=-2:HEIGHT"`:**
   Using `-2` instead of `-1` guarantees the resulting width is divisible by 2 (mandatory for H.264/H.265 encoders).
3. **CRF (Constant Rate Factor) guide for H.264 (`-crf N`):**
   - `18` : Visually lossless (large file size).
   - `22` : Default sweet spot (excellent quality, balanced size).
   - `26` : Space saver (great for mobile, web sharing, small disk footprint).
   - `28+` : Aggressive compression.
4. **Use AMD VA-API for fast hardware encoding:**
   Your AMD Renoir/Vega graphics chip has dedicated VCN silicon. Using `h264_vaapi` renders videos 5x–10x faster than CPU while keeping your laptop cool and quiet.

---

## 2. Fast Container & Format Changing (Zero Loss · 1 Second)

Copies the audio and video streams directly into a new container without re-encoding:

```bash
# MKV to MP4:
ffmpeg -i input.mkv -c copy output.mp4

# WebM to MP4:
ffmpeg -i input.webm -c copy output.mp4

# MOV to MP4:
ffmpeg -i input.mov -c copy output.mp4

# TS stream to MP4:
ffmpeg -i input.ts -c copy output.mp4
```
*(Note: If the source video uses VP9/AV1 or Opus audio that your target player doesn't support, re-encode using Section 3).*

---

## 3. Resolution Scaling (Resizing Video)

### Scale to 1080p (Full HD)
```bash
ffmpeg -i input.mp4 -vf "scale=-2:1080" -c:v libx264 -crf 22 -c:a copy output_1080p.mp4
```

### Scale to 720p (HD · Ideal for sharing & mobile)
```bash
ffmpeg -i input.mp4 -vf "scale=-2:720" -c:v libx264 -crf 23 -c:a copy output_720p.mp4
```

### Scale to 480p (SD · Minimal file size)
```bash
ffmpeg -i input.mp4 -vf "scale=-2:480" -c:v libx264 -crf 24 -c:a aac -b:a 128k output_480p.mp4
```

---

## 4. Hardware-Accelerated Encoding (AMD Vega / Renoir GPU)

Encodes on your AMD GPU hardware silicon via VA-API instead of burning CPU:

### Fast 720p GPU Scaling & Encoding
```bash
ffmpeg -hwaccel vaapi -hwaccel_device /dev/dri/renderD128 -hwaccel_output_format vaapi \
  -i input.mp4 \
  -vf "scale_vaapi=w=-2:h=720" \
  -c:v h264_vaapi -b:v 2500k -c:a copy output_720p.mp4
```

### Fast 1080p GPU Encoding
```bash
ffmpeg -hwaccel vaapi -hwaccel_device /dev/dri/renderD128 -hwaccel_output_format vaapi \
  -i input.mp4 \
  -vf "scale_vaapi=w=-2:h=1080" \
  -c:v h264_vaapi -b:v 4500k -c:a copy output_1080p.mp4
```

---

## 5. Video Compression (Shrinking File Size)

### Balanced High-Quality Compression (Saves 60–80% Disk Space)
```bash
ffmpeg -i input.mp4 -c:v libx264 -crf 26 -preset slow -c:a aac -b:a 128k compressed.mp4
```

### Target File Size for Discord / WhatsApp / Email
To compress a video to fit under Discord's 25 MB or WhatsApp's 64 MB limit:

```bash
# Calculate video bitrate: Bitrate = (Target_MB * 8192 / Duration_in_seconds) - 128k
# Example for a 60-second video targeting 20 MB:
ffmpeg -i input.mp4 -c:v libx264 -b:v 2500k -maxrate 2800k -bufsize 5000k -c:a aac -b:a 128k target_20mb.mp4
```

---

## 6. Fast Trimming & Cutting

### Instant Lossless Cut (No Re-encoding · 1 Second)
```bash
# Cut from 01:30 to 03:00:
ffmpeg -ss 00:01:30 -to 00:03:00 -i input.mp4 -c copy trimmed.mp4

# Cut starting at 00:45 for a duration of 30 seconds:
ffmpeg -ss 00:00:45 -i input.mp4 -t 00:00:30 -c copy trimmed.mp4
```

### Frame-Accurate Cut (Re-encodes only if cutting on non-keyframes)
```bash
ffmpeg -ss 00:01:30.500 -to 00:02:45.200 -i input.mp4 -c:v libx264 -crf 22 -c:a copy accurate_cut.mp4
```

---

## 7. Audio Extraction & Conversion

### Extract Original Audio (Instant · No Re-encoding)
```bash
ffmpeg -i input.mp4 -vn -c:a copy audio.m4a
```

### Convert Video to High-Quality MP3 (320kbps / V0)
```bash
ffmpeg -i input.mp4 -vn -q:a 0 audio.mp3
```

### Convert Any Audio (FLAC / WAV / OGG to MP3)
```bash
ffmpeg -i input.flac -q:a 0 output.mp3
```

### Remove / Mute Audio from Video
```bash
ffmpeg -i input.mp4 -an -c:v copy muted_video.mp4
```

### Replace Audio in Video with a New Audio Track
```bash
ffmpeg -i video.mp4 -i soundtrack.mp3 -c:v copy -c:a aac -map 0:v:0 -map 1:a:0 final.mp4
```

---

## 8. High-Quality Animated GIF Creation

Standard FFmpeg GIF commands create pixelated, discolored results. Use palette generation for crisp 60fps / 30fps GIFs:

```bash
# Create a high-quality 480p GIF from second 5 to 10:
ffmpeg -ss 00:00:05 -t 5 -i input.mp4 \
  -filter_complex "[0:v] fps=15,scale=480:-1:flags=lanczos,split [a][b];[a] palettegen [p];[b][p] paletteuse" \
  animation.gif
```

---

## 9. Video Speed Manipulation

```bash
# Speed up video 2x:
ffmpeg -i input.mp4 -filter_complex "[0:v]setpts=0.5*PTS[v];[0:a]atempo=2.0[a]" -map "[v]" -map "[a]" fast_2x.mp4

# Slow down video to 0.5x (Slow-Motion):
ffmpeg -i input.mp4 -filter_complex "[0:v]setpts=2.0*PTS[v];[0:a]atempo=0.5[a]" -map "[v]" -map "[a]" slow_0.5x.mp4
```

---

## 10. Extracting Thumbnails & Screenshots

```bash
# Extract high-res image at exactly 01:25:
ffmpeg -ss 00:01:25 -i input.mp4 -frames:v 1 -q:v 2 frame.jpg

# Extract 1 screenshot every second:
ffmpeg -i input.mp4 -vf fps=1 "frame_%04d.jpg"
```

---

## 11. Quick Reference Cheat Sheet Table

| Goal | Command | Speed |
| :--- | :--- | :--- |
| **Instant format change** | `ffmpeg -i in.mkv -c copy out.mp4` | **Instant (< 1s)** |
| **Downscale to 720p** | `ffmpeg -i in.mp4 -vf "scale=-2:720" -c:v libx264 -crf 23 -c:a copy out.mp4` | Fast |
| **Downscale to 1080p** | `ffmpeg -i in.mp4 -vf "scale=-2:1080" -c:v libx264 -crf 22 -c:a copy out.mp4` | Fast |
| **AMD GPU 720p** | `ffmpeg -hwaccel vaapi -i in.mp4 -vf scale_vaapi=-2:720 -c:v h264_vaapi out.mp4` | **5x Faster (Hardware)** |
| **Extract 320k MP3** | `ffmpeg -i in.mp4 -vn -q:a 0 out.mp3` | Fast |
| **Lossless cut / trim** | `ffmpeg -ss 01:00 -to 02:30 -i in.mp4 -c copy out.mp4` | **Instant (< 1s)** |
| **Compress video (CRF 26)** | `ffmpeg -i in.mp4 -c:v libx264 -crf 26 -preset slow -c:a aac out.mp4` | High Compression |
| **Create HD GIF** | `ffmpeg -ss 5 -t 3 -i in.mp4 -vf "fps=15,scale=480:-1" out.gif` | Medium |
| **Remove audio track** | `ffmpeg -i in.mp4 -an -c:v copy muted.mp4` | **Instant (< 1s)** |
