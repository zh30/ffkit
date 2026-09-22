# Creator multimedia needs → remaining ffkit gaps

Researched 2026-09-22 against **ffkit 0.25.1** (`main`). Previous pass 2026-09-15 (vs 0.22.1). This note is not a restatement of landed work — the tail lists what not to redo.

## What 2026 creators still trip on

**Platform size limits are the recurring hard wall.** Agents can edit and export, but "fit under N" is still hand-math:

- Discord free tier is **10 MB** now (nitro 500 MB); WhatsApp video caps ~**16 MB**; email attachments ~**20–25 MB**. Creators keep hitting "file too large" *after* finishing an edit.
- ffmpeg has no `--size` flag: the fix is bitrate math — `video_kbps = (target_bits × 0.98 − audio_bits) / duration` — then **two-pass** to actually land the size ([ffmpeg-cookbook, Apr 2026](https://ffmpeg-cookbook.com/en/articles/two-pass-encoding/); [bitrate math walkthrough, Jul 2026](https://dev.to/alexpua/how-i-make-ffmpeg-hit-an-exact-file-size-the-bitrate-math-nobody-explains-3o7o)). Agents reinvent this per job and overshoot.

**Podcast→clips is the #1 scaled workflow.** [Loopdesk, Jul 2026](https://loopdesk.ai/blog/video-workflows-for-creators): the four workflows that cover almost every creator are podcast-to-clips, weekly YouTube, archive mining, multi-platform publishing. Audio-only episodes need a **picture** before they can be a Reel/Short — the standard artifact is a cover still with an animated waveform (**audiogram**). ffmpeg `showwaves` renders it without any extra dependency.

**Audio cleanup comes before loudness.** [MSY Editor, Mar 2026](https://msyeditor.com/ai-video-editing-workflow-2026/): in every AI edit pipeline, noise reduction is applied *first*, before levels and B-roll — room rumble, laptop fan, hiss. Homebrew/apt ffmpeg ships `afwtdn`, `highpass`, `afftdn`; no model files needed.

**Landscape→vertical repurpose wants a blurred fill, not black bars.** The repurpose playbooks (CuteDyno Jun 2026, loopdesk multi-platform) assume a blurred pillarbox when the source is 16:9 — black `pad` bars read as unedited. `split + scale=increase,crop + gblur + overlay=centered fg` is a known-graph but error-prone raw.

**HDR iPhone footage washed out in SDR feeds** needs `zscale`+`tonemap`; Homebrew ffmpeg here has `tonemap` but **no `zscale`** (needs `--with-libzimg`). Gate behind `doctor` before promising it.

## Gaps vs ffkit 0.25.1

|| Creator request | Today | Gap |
||-----------------|-------|-----|
|| "压到 10MB 发 Discord / shrink for email" | `transcode` presets pick a codec/CRF, not a size | No size target; bitrate math + two-pass must be hand-built |
|| "把这段播客做成能发的视频" | audio-only input has no video verbs at all | No waveform/cover audiogram path to 9:16 |
|| "房间底噪 / fan noise / 降噪" | `loudnorm` / `volume` only | No voice denoise (queued last run) (queued last run) |
|| "竖屏但背景要模糊" | `fit --fit pad` = black bars; `crop` cuts the subject | No blurred-fill mode |
|| Swap camera audio for lav mic | `music` mixes a bed under; nothing replaces | Raw `ffmpeg -map` only; queue next |
|| Word-highlight karaoke captions | whole-cue raster burn | Needs per-word timing source; still deferred |
|| LUT / film look | `grade` sliders only | `lut3d` exists; queue if asked |
|| HDR→SDR for iPhone clips | — | needs libzimg (`zscale` absent on Homebrew/apt) |

## Ordered directions (this run)

1. **`denoise`** — `ffkit denoise IN -o OUT`: `highpass` + `afwtdn` voice cleanup, `--video` adds `hqdn3d` on the picture. Maps to "降噪 / 底噪 / room tone". Already queued by the previous run's ordered list.
2. **`compress`** — `ffkit compress IN -o OUT --size 10MB`: probe duration → bitrate budget (2% mux reserve, audio paid first at 96 kbps) → libx264 **two-pass**. Audio-only input single-passes `-b:a`. Fails fast when the math is impossible (<64 kbps video). Maps to "发不出去，太大了".
3. **`fit --fit blur`** — blurred-pillarbox fill behind the scaled foreground. Maps to "竖屏化不要黑边".
4. **`audiogram`** — `ffkit audiogram IN [-o reel.mp4 --image cover.png]`: `showwaves` over a cover still (or flat colour) → 1080×1920, audio kept. Maps to "播客做成 Reel".

Next (not this run): replace-audio mux (`--audio`), word-chunk caption highlight (needs word-timed source), `lut3d`, `audiogram` needs none — but HDR needs libzimg; slideshow (zoompan over N stills + bed) if agents keep hand-rolling it.

## Already landed (do not redo)

Deliver 1080×1920 −14 LUFS; caption burn without libass + `--safe social` bottom-20%; broll cutaway keeps A-roll audio/duration and plays B from its first frame; rough-cut speech islands (list, then encode only keeps); music duck (aformat dbl pin for sidechaincompress on apt ffmpeg); speed; jumpcut; cover; fade; title; loop; stabilize; reverse; grade/zoom/sharpen/vignette/bw/volume/blur; pipeline `$src`/`$in`/`expect`; GitHub Release zips; English + Chinese README.
