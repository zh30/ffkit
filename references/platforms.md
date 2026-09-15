# Platforms

Destination implied by the user (Reels, Shorts, TikTok, YouTube, GIF) picks frame, loudness, and transcode preset. Say the defaults you used.

| Destination | Frame | Fit | Loudness | Export |
|-------------|-------|-----|----------|--------|
| Reels / TikTok / YouTube Shorts | 1080x1920, 30 fps | one shot: `ffkit deliver IN --platform reels\|tiktok\|shorts -o OUT` | −14 LUFS / −1.5 dBTP (built in) | H.264 + AAC + faststart |
| YouTube landscape | 1920x1080 | `--aspect 16:9` | `-I -14` | `h264` (`+faststart` is already on that preset) |
| Square (feed) | 1080x1080 | `--aspect 1:1` | `-I -14` | `h264` |
| Podcast audio | no picture | — | `-I -16 --tp -1.5` | `extract` to wav/m4a, then `loudnorm` |
| GIF preview | short, ≤480px wide | — | no audio | `transcode --preset gif` |

Safe-area captions on 9:16: keep burn-in away from the top and bottom ~250 px (platform UI). `look` the result.

If the user says "export" / "post" / "deliver" without a platform, `ffkit deliver --platform social` (same 1080×1920 pack). A plain cut or extract keeps the source format.
