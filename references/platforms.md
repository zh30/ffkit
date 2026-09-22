# Platforms

Destination implied by the user (Reels, Shorts, TikTok, YouTube, GIF) picks frame, loudness, and transcode preset. Say the defaults you used.

| Destination | Frame | Fit | Loudness | Export |
|-------------|-------|-----|----------|--------|
| Reels / TikTok / YouTube Shorts | 1080x1920, 30 fps | one shot: `ffkit deliver IN --platform reels\|tiktok\|shorts -o OUT` | −14 LUFS / −1.5 dBTP (built in) | H.264 + AAC + faststart |
| YouTube landscape | 1920x1080 | `--aspect 16:9` | `-I -14` | `h264` (`+faststart` is already on that preset) |
| Square (feed) | 1080x1080 | `--aspect 1:1` | `-I -14` | `h264` |
| Podcast audio | no picture | — | `-I -16 --tp -1.5` | `extract` to wav/m4a, then `loudnorm` |
| Podcast clip → social | 1080x1920 waveform | `ffkit audiogram IN --image cover.png -o OUT` | audio kept | H.264+AAC |
| GIF preview | short, ≤480px wide | — | no audio | `transcode --preset gif` |

| Size-capped upload (Discord 10MB / WhatsApp 16MB / email ~25MB) | source frame kept | `ffkit compress IN --size 10MB -o OUT` | − | two-pass H.264+AAC |
| Photo montage (anniversary / listing / event recap) | `--size 1920x1080` or `720x1280` | `ffkit slideshow a.jpg b.jpg c.jpg --audio bed.mp3 -o OUT` | bed faded at end | H.264+AAC |
| Synced lav-mic audio onto camera footage | source frame kept | `ffkit replace cam.mp4 --audio lav.wav -o OUT` | trimmed/padded to video | H.264+AAC |


Safe-area captions on 9:16: `caption --mode burn` defaults to `--safe social` (above the bottom 20%). Union rectangle is ~900×1400 centered in 1080×1920. `look` the result.

If the user says "export" / "post" / "deliver" without a platform, `ffkit deliver --platform social` (same 1080×1920 pack). A plain cut or extract keeps the source format.
