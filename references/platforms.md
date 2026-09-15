# Platforms

Destination implied by the user (Reels, Shorts, TikTok, YouTube, GIF) picks frame, loudness, and transcode preset. Say the defaults you used.

| Destination | Frame | Fit | Loudness | Export |
|-------------|-------|-----|----------|--------|
| Reels / TikTok / YouTube Shorts | 1080x1920, 30 or 60 fps | `--aspect 9:16` (`pad` keeps the whole picture; `crop` fills the frame) | `-I -14 --tp -1.5` | `transcode --preset h264` |
| YouTube landscape | 1920x1080 | `--aspect 16:9` | `-I -14` | `h264` (`+faststart` is already on that preset) |
| Square (feed) | 1080x1080 | `--aspect 1:1` | `-I -14` | `h264` |
| Podcast audio | no picture | — | `-I -16 --tp -1.5` | `extract` to wav/m4a, then `loudnorm` |
| GIF preview | short, ≤480px wide | — | no audio | `transcode --preset gif` |

Safe-area captions on 9:16: keep burn-in away from the top and bottom ~250 px (platform UI). `look` the result.

If the user says "export" / "post" / "deliver" without a platform, ask once where it goes. A plain cut or extract keeps the source format.
