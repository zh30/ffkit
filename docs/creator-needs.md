# Creator multimedia needs → remaining ffkit gaps

Researched 2026-09-15 against **ffkit 0.22.1** (`main`). This note is not a restatement of landed work (deliver, caption-without-libass, jumpcut, pipeline, Release zips). Those already cover “做成 Reel / 静音也能看 / 剪停顿 / 方案一次跑完”.

## What 2026 creators still trip on

**Burned captions that sit under the app chrome are wasted work.** Mute viewing is still the default, but placement now has a measured safe rectangle:

- [CuteDyno, Apr 2026](https://cutedyno.com/blog/short-form-video-safe-zones-reels-shorts-2026) — top 10–15% (status / nav), **bottom 15–20%** (captions, audio chip, CTAs), right rail for like/share. Cross-post: keep critical text off those edges.
- [Blitzcut, Jun 2026](https://blitzcutai.com/blog/best-caption-placement-short-form-video) — TikTok bottom dead zone **320–350 px** on 1080×1920; Reels bottom **310–450 px**. Union advice: keep all burn-in inside a **~900×1400 centered** block. Lower-middle captions (about Y=1200–1550) clear a typical talking-head face.
- [ShortSync, Feb 2026](https://www.shortsync.app/resources/add-captions-short-form-videos) — ~80% of short-form views start muted; place captions in the **middle 60%** vertically, not the very bottom.
- [HeyGen tool test, Aug 2026](https://www.heygen.com/blog/best-ai-video-generator-tiktok-reels) — caption quality scoring includes whether burn-in **clears TikTok username and Reels caption bar**.
- [Shortzly, Jun 2026](https://shortzly.com/blog/animated-captions-style-guide-short-form-video) — CapCut-style 2–4 word chunks + current-word highlight is the feed default; **middle-third** is the safest band.

**Talking-head edits insert B-roll while the voice keeps going:**

- [Captions (Mirage) review, Sep 2026](https://aidemos.com/tools/captions) — the winning talking-head first pass is: trim dead air, **add captions, insert B-roll**, style titles. B-roll is a first-class step, not a logo PiP.
- CuteDyno’s [repurpose playbook, Jun 2026](https://cutedyno.com/blog/repurpose-short-form-video-multi-platform-2026) — shoot B-roll with the A-roll so the same 9:16 master can cut away.
- Comparable agent tool: ffmpeg-skill `broll.py` — cut away to an insert for `--at`/`--duration`, **A’s length and audio untouched**.

**Voice denoise** (room rumble, laptop fan) is the other frequent “make this usable” ask. Homebrew ffmpeg here has `afftdn` / `highpass` / `hqdn3d`. Deferred this run (see ordered list).

Word-by-word karaoke highlight, Whisper/TTS, stock B-roll search, multicam, HDR/LUT, beat-sync, and publish APIs stay out of scope.

## Gaps vs ffkit 0.22.1

| Creator request | Today | Gap |
|-----------------|-------|-----|
| “Burn captions so mute viewers can read them in-feed” | `caption --mode burn` overlays at `y=H-h-15%H` | **15% of 1920 = 288 px**, under TikTok’s ~320–350 px bottom chrome. `gotchas.md` still says “~15% from the bottom”. Cross-post needs ~**20% bottom** (and clear the top 15%). |
| “Cut to this clip while I keep talking” | `overlay --video` is a **PiP** for the whole timeline | No timed **full-frame cutaway** that keeps A-roll audio and duration. Agent would invent `enable='between(t,…)'` graphs. |
| “Clean up the room noise” | `loudnorm` / `volume` only | No `afftdn` / highpass voice denoise verb. |
| Karaoke / current-word highlight | Raster burn is whole-cue | Needs word-timed SRT or libass; deferred. |

## Ordered directions (this run)

1. **Social caption safe-zone** — `caption --mode burn` default `--safe social`: place burn-in above the bottom **20%** (and not into the top 15%). `--safe none` keeps the old 15% bottom margin. Maps to “烧字幕 / 静音也能看 / 不要挡住 TikTok 底栏”. Not a new look verb; the existing caption hand was placing text in the dead zone.
2. **B-roll cutaway** — `ffkit broll A --insert B --at T --duration D`: replace the picture for a window, **keep A’s audio and duration**. Maps to “口播切 B-roll / 画面切走声音继续”. Justified as a verb: the enable+scale+crop graph is repeatedly error-prone as raw ffmpeg.

Next (not this run): `denoise` via `afftdn` (+ optional `hqdn3d`), then word-chunk caption highlight without libass if a plan step keeps failing.

## Already landed (do not redo)

Deliver 1080×1920 −14 LUFS; caption burn without libass; speed; music duck; jumpcut; cover; fade; title; loop; stabilize; reverse; grade/zoom/sharpen/vignette/bw/volume/blur; pipeline `$src`/`$in`/`expect`; GitHub Release zips; English + Chinese README.
