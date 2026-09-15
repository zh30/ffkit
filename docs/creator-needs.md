# Creator multimedia needs → ffkit gaps

Researched 2026-09-15. This note is the source of improvement order for this run. It is not a restatement of the 0.1.0 verb list.

## What creators actually ship

Short-form is one canvas. Independent 2026 spec round-ups agree that TikTok, Instagram Reels, and YouTube Shorts all want **1080×1920, 9:16, H.264 + AAC in MP4**:

- [anfx.co, Jul 2026](https://anfx.co/blog/youtube-shorts-tiktok-reels-video-size-guide/) — 1080×1920 for Shorts / TikTok / Reels; UI safe-zones (TikTok bottom ~480 px, Reels bottom ~420 px). Union advice: keep critical text in a centered ~840×1300 block.
- [The Post Flow, Jul 2026](https://thepostflow.com/post-production/post-production-workflows/export-settings-youtube-instagram-tiktok/) — YouTube measured loudness **−14 LUFS integrated, −1 dBTP**; Shorts 1080×1920; Reels MP4, H.264, AAC 48 kHz, `moov` at front; TikTok SDR Rec.709 only.
- [CapKit export guide, Dec 2025](https://capkit.app/guides/export-settings/) — universal 9:16 / 1080×1920 / 30 fps / H.264 / AAC.
- [ClipSpeed, Apr 2026](https://clipspeed.ai/blog/video-resolution-guide-shorts-reels-tiktok.html) — same frame; ~10 Mbps VBR as a cross-post default.

**Mute viewing is the default, so captions are not optional:**

- [ShortSync, Feb 2026](https://www.shortsync.app/resources/add-captions-short-form-videos) — ~80% of short-form views start with sound off; YouTube indexes caption text.
- [Mark Studios, Mar 2026](https://www.markstudios.com/blog/tiktok-reels-shorts-cross-platform-strategy-2026) — burn captions into the file; platform-native CC is inconsistent on TikTok.
- [Blitzcut, Jun 2026](https://blitzcutai.com/blog/best-caption-placement-short-form-video) — place captions in a ~900×1400 centered safe rectangle so UI chrome does not cover them.

Creators say one sentence to a tool (“make this a Reel”, “add captions so it works on mute”) and expect **one deliverable**, not a three-step fit → transcode → loudnorm puzzle.

## Gaps in ffkit 0.1.0

| Creator request | Today | Gap |
|-----------------|-------|-----|
| “Export for Reels / TikTok / Shorts” | Agent must chain `fit --aspect 9:16`, `transcode --preset h264`, `loudnorm -I -14` (three encodes, easy to skip loudness) | No single delivery verb; `platforms.md` is a cheat sheet, not a command |
| “Burn captions, I watch on mute” | `caption --mode burn` needs ffmpeg **libass** (`subtitles` filter). Typical Homebrew ffmpeg 9 has neither libass nor `drawtext` | Burn fails on this class of machine; mux-only captions do not show in-feed |
| “Speeding up the take / slow-mo” | No verb; agent would invent `setpts`/`atempo` | Talking-head 1.1–2× and slow-mo are default short-form edits |
| “Add a track / trending audio under the voice” | No mix/duck verb | Mark Studios (2026): music under speech is the Reels/TikTok default mix |
| Karaoke / word-by-word ASS | Not offered | Deferred |
| “Cut the dead air / jump cuts” | No silence detector | Talking-head default edit |
| Multicam, HDR/LUT, Whisper, B-roll, publish APIs | Not offered | Deferred |

`fit` + `transcode` + `loudnorm` can *approximate* a social export if the agent never skips a step. That is not how creators speak, and it is not how this skill should route.

## Ordered directions (this run)

1. **Short-form delivery pack** — one verb `ffkit deliver` that outputs 1080×1920 9:16, 30 fps, H.264 + AAC + `faststart`, EBU-style **−14 LUFS / −1.5 dBTP**, in a single JSON contract. Maps to “做成 Reel / TikTok / Shorts / 导出到三平台”.
2. **Captions that work without libass** — burn SRT by rasterizing cues and `overlay` (always present). Maps to “烧字幕 / 静音也能看”. Do not depend on `subtitles` / `ass` / `drawtext`.

Landed as PRs to `main` (0.2.0 / 0.3.0). Next:

3. **Speed** — `ffkit speed --factor N` with `setpts` + chained `atempo` (pitch kept). Maps to “加速 / 慢动作 / 1.5x”.
4. **Music under speech** — `ffkit music --track BGM` with `sidechaincompress` ducking. Maps to “加 BGM / 配乐压人声”.
5. **Silence jump-cuts** — `ffkit jumpcut`. Maps to “剪掉停顿 / jump cut”.
6. **Social cover still** — `ffkit cover`. Maps to “封面 / 封面图 / thumbnail”.
7. **Fade in/out** — `ffkit fade`. Maps to “淡入淡出”.
8. **Hook title card** — `ffkit title --text`. Maps to “片头字 / hook”.
9. **Loop / replay length** — `ffkit loop --times`. Maps to “循环 / 加长 Shorts”.
10. **Stabilize handheld** — `ffkit stabilize` (`deshake`). Maps to “防抖 / 稳定”.
11. **Reverse** — `ffkit reverse`. Maps to “倒放”.
12. **Color grade / pop** — `ffkit grade`. Maps to “调色 / 更艳 / Reels 质感”.
