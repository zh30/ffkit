# Creator multimedia needs → remaining ffkit gaps

Researched 2026-09-22 against **ffkit 0.45.0** (`main` + round-20 branch). Same-day research base as the 0.26.0/0.27.0 rounds below — refreshed priorities only, no new sources needed. This note is not a restatement of landed work — the tail lists what not to redo.

## What 2026 creators still trip on

**Platform size limits are the recurring hard wall.** Agents can edit and export, but "fit under N" is still hand-math:

- Discord free tier is **10 MB** now (nitro 500 MB); WhatsApp video caps ~**16 MB**; email attachments ~**20–25 MB**. Creators keep hitting "file too large" *after* finishing an edit.
- ffmpeg has no `--size` flag: the fix is bitrate math — `video_kbps = (target_bits × 0.98 − audio_bits) / duration` — then **two-pass** to actually land the size ([ffmpeg-cookbook, Apr 2026](https://ffmpeg-cookbook.com/en/articles/two-pass-encoding/); [bitrate math walkthrough, Jul 2026](https://dev.to/alexpua/how-i-make-ffmpeg-hit-an-exact-file-size-the-bitrate-math-nobody-explains-3o7o)). Agents reinvent this per job and overshoot.

**Podcast→clips is the #1 scaled workflow.** [Loopdesk, Jul 2026](https://loopdesk.ai/blog/video-workflows-for-creators): the four workflows that cover almost every creator are podcast-to-clips, weekly YouTube, archive mining, multi-platform publishing. Audio-only episodes need a **picture** before they can be a Reel/Short — the standard artifact is a cover still with an animated waveform (**audiogram**). ffmpeg `showwaves` renders it without any extra dependency.

**Audio cleanup comes before loudness.** [MSY Editor, Mar 2026](https://msyeditor.com/ai-video-editing-workflow-2026/): in every AI edit pipeline, noise reduction is applied *first*, before levels and B-roll — room rumble, laptop fan, hiss. Homebrew/apt ffmpeg ships `afwtdn`, `highpass`, `afftdn`; no model files needed.

**Multi-source audio is the other half of "my audio is bad".** Creators record on the camera AND a lav mic; "swap in the clean track" is a daily ask. `music` mixes a bed *under* the original; nothing *replaces* it. Raw `-map 0:v -map 1:a` is simple but error-prone (sync offset, length mismatch, codec copy of an incompatible container).

**Photo montages are the second-largest audio-only adjacent case.** "Turn these 12 photos into a 30-second Reel with music" is the classic slideshow ask: normalize stills onto a canvas, crossfade, lay a bed that ends cleanly. Doable in raw ffmpeg (`-loop 1 -t` + xfade chain) but the offset math (`k*(per-fade)`) is exactly the kind of thing agents get wrong.

**Landscape→vertical repurpose wants a blurred fill, not black bars.** The repurpose playbooks (CuteDyno Jun 2026, loopdesk multi-platform) assume a blurred pillarbox when the source is 16:9 — black `pad` bars read as unedited. `split + scale=increase,crop + gblur + overlay=centered fg` is a known-graph but error-prone raw.

**HDR iPhone footage washed out in SDR feeds** needs `zscale`+`tonemap`; Homebrew ffmpeg here has `tonemap` but **no `zscale`** (needs `--with-libzimg`). Gate behind `doctor` before promising it.

## Gaps vs ffkit 0.39.0

|| Creator request | Today | Gap |
||-----------------|-------|-----|
|| Swap camera audio for lav mic / new voice / new music | `music` mixes a bed under; nothing replaces | Raw `ffmpeg -map` only — landed this run as `replace` |
|| Photo dump → montage video ("把照片做成视频") | nothing builds video from stills | Landed this run as `slideshow` |
|| "套我的 LUT / film look" | `grade` sliders only | Landed this run as `grade --lut` (`lut3d`) |
|| Word-group "karaoke" captions (chunked, no ASR) | whole-cue raster burn | Landed this run as `caption --chunk N` (approximation — true word highlight still needs a timing source) |
|| Transition variety on montage | `slideshow`/`concat` fade only | Landed this run as `slideshow --transition` + `--motion kenburns` |
|| "Keep room tone under the lav" | `replace` drops the original | Landed this run as `replace --mix G` |
|| Multi-clip montage with transitions | `concat --transition` 2-clip fade only | Landed this run: N-clip xfade/acrossfade chain |
|| "Split into 30s chunks for Status/Stories" | `cut` one range at a time | Landed this run as `split --every` |
|| "把我 P 到绿幕背景上" | `overlay` needs a mask; nothing keys | Landed this run as `key` (`colorkey`) |
|| Chapter/explicit-point splits | `split --every` grid only | Landed this run as `split --at` |
|| HDR→SDR for iPhone clips | — | needs libzimg (`zscale` absent on Homebrew/apt) |

## Ordered directions (this run)

*(round 21: `waveform`, `spectrogram`, `dehum`. round 20: `vdenoise`, `crop`, `title --position bottom`. round 19: `bleep`, `censor --at/--dur`, `grade --warm`. round 18: `reverb`, `audiogram --mode/--color`, `delogo --at/--dur`, `meta --rotate`)*

1. **`waveform`** — waveform PNG export for podcast art/thumbnails/embeds (`showwavespic`; complements `audiogram`'s animated video).
2. **`spectrogram`** — `showspectrumpic` PNG so creators/agents can *see* the noise or hum before picking cleanup settings.
3. **`dehum`** — mains hum (50/60 Hz + harmonics) via `highpass` + Q=12 `equalizer` notch chain; the "电流声/嗡鸣" complaint `denoise`'s broadband path doesn't target.

*(earlier rounds below)*

1. **`vdenoise`** — "暗光素材全是噪点": `nlmeans` spatial denoise (`--strength`); `hqdn3d` was tried first and barely moves strong noise — nlmeans is the usable engine.
2. **`crop`** — manual reframe/crop-out-edge (`--region x:y:w:h`, or `--aspect W:H` center box) — `autocrop` only detects letterbox; creators crop for recompose and watermark-edge removal.
3. **`title --position bottom`** — lower-third placement joins `top`/`center` (one match arm).

*(earlier rounds below)*

1. **`bleep`** — the classic TV-safe beep over a word: silences the source inside `--at`/`--dur` and mixes a delayed `sine` tone (`--freq`, `--level`). `volume --at` mutes but leaves dead air; creators want the audible cover.
2. **`censor --at/--dur`** — faces/plates that only appear for part of the clip (the window pattern, third verb after `delogo`/`volume`).
3. **`grade --warm`** — "my footage is too blue/orange": white-balance warmth −1..1 via `colortemperature` (the `colorbalance` midtone term is a no-op on mid-gray — verified).

*(earlier rounds below)*

1. **`reverb`** — flat phone-mic voiceover sounds dead; `aecho` room/hall/cave presets give it space (one `--wet` knob).
2. **`audiogram --mode`/`--color`** — brand-styled waveform strips (line/p2p/point + colour), the recurring "match my brand" ask on podcast clips.
3. **`delogo --at/--dur`** — watermark/logo boxes that appear mid-clip only (intro cards, end screens).
4. **`meta --rotate`** — "shot fine but plays sideways": fix the display-rotation flag losslessly (version-gated: `-display_rotation` on ffmpeg ≥7, `rotate` metadata below).

*(earlier rounds below)*

1. **`autocrop`** — `cropdetect` scan pass + crop; letterbox removal is a top rewatch ask.
2. **`sheet`** — contact-sheet thumbnails for preview/selection (`fps=N/dur,tile`).
3. **`title --at`** — lower-thirds mid-clip, one flag on an existing verb.

*(round 16: `title --size/--color`, `meta`, `broll --motion`. round 15: `rotate`, `delogo`, `speed --interp`. round 14: `eq`, `zoom --motion`, `broll --still`. round 13: `channel`, `loop --until`, `title --tile`. round 12: `overlay --tile`, `caption --shift`, gif flags. round 11: `split --scenes`, `cutsil`, `grid --audio`. round 10: `replace --duck`, `pitch`, `grade --grain`. round 9: `autocrop`, `sheet`, `title --at`. round 8: `boomerang`, `chapter`, `zoom --at/--dur`, `key --despill`)*

1. **`boomerang`** — social fwd+rev replay (`split`+`reverse`/`areverse`+`concat`).
2. **`chapter`** — named marks for players/YouTube via `ffmetadata` `-c copy` (lossless).
3. **`zoom --at/--dur`**, **`key --despill`** — windowed punch and edge cleanup, cheap flags on existing verbs.

*(round 7: `freeze`, `censor`, `speed --at/--dur`. round 6: `grid`, `progress`, `volume --at/--dur`)*

1. **`freeze`** — hold-frame mid-clip or outro (`tpad` clone; silence under the hold).
2. **`censor`** — "把脸/车牌打码": crop→`pixelize`/`gblur`→overlay on a pixel box.
3. **`speed --at/--dur`** — slow-mo punch / fast-forward one window (3-segment trim/concat).

*(round 6: `grid`, `progress`, `volume --at/--dur`. round 5: `key`, `split --at`)*

1. **`grid`** — reaction / comparison / multi-angle tiles (`xstack`, scale+pad per cell, `amix` when all inputs carry audio).
2. **`progress`** — the engagement trick of a bottom bar filling across the video (drawbox w is init-only → sliding-strip `overlay` recipe).
3. **`volume --at/--dur`** — mute/bleep just a moment (`volume=…:enable='between(t,a,b)'`).

*(round 5: `key`, `split --at`)*

1. **`key`** — "把我 P 到绿幕背景/把产品 P 进场景": `colorkey` the foreground over a `--bg` image/video normalized to the FG canvas; the compositing ask `overlay` alone can't answer (it needs a mask).
2. **`split --at t1,t2`** — chapter splits: same forced-keyframe + `-segment_times` machinery, explicit cut list instead of an even grid.

*(round 4: `split --every`, N-clip `concat --transition` chains. round 3: `caption --chunk`, `slideshow --transition`/`--motion kenburns`, `replace --mix`)*

Next (not this run): true word-highlight karaoke (needs a word-timed source — whisper export or `align`; none in repo), HDR→SDR (needs libzimg — absent on Homebrew/apt), `key` despill (green fringe on edges), `grid` audio-follow-one-input option.

## Already landed (do not redo)

Deliver 1080×1920 −14 LUFS; caption burn without libass + `--safe social` bottom-20%; broll cutaway keeps A-roll audio/duration and plays B from its first frame; rough-cut speech islands (list, then encode only keeps); music duck (aformat dbl pin for sidechaincompress on apt ffmpeg); speed; jumpcut; cover; fade; title; loop; stabilize; reverse; grade/zoom/sharpen/vignette/bw/volume/blur; pipeline `$src`/`$in`/`expect`; GitHub Release zips; English + Chinese README; **0.26.0**: `denoise` (afwtdn/afftdn fallback), `compress --size` two-pass budget, `fit`/`broll --fit blur`, `audiogram`. **0.27.0**: `replace`, `slideshow`, `grade --lut`. **0.28.0**: `caption --chunk`, `slideshow --transition`/`--motion kenburns`, `replace --mix`. **0.29.0**: `split --every`, N-clip `concat --transition` chains. **0.30.0**: `key`, `split --at`. **0.31.0**: `grid`, `progress`, `volume --at/--dur`. **0.32.0**: `freeze`, `censor`, `speed --at/--dur`. **0.33.0**: `boomerang`, `chapter`, `zoom --at/--dur`, `key --despill`. **0.34.0**: `autocrop`, `sheet`, `title --at`. **0.35.0**: `replace --duck`, `pitch`, `grade --grain`. **0.36.0**: `split --scenes`, `cutsil`, `grid --audio`. **0.37.0**: `overlay --tile`, `caption --shift`, gif `--fps/--width`. **0.38.0**: `channel`, `loop --until`, `title --tile`. **0.39.0**: `eq`, `zoom --motion`, `broll --still`. **0.40.0**: `rotate`, `delogo`, `speed --interp`. **0.41.0**: `title --size/--color`, `meta`, `broll --motion`. **0.42.0**: `overlay --at`, `caption --color/--size`, `subs`. **0.43.0**: `reverb`, `audiogram --mode/--color`, `delogo --at/--dur`, `meta --rotate`. **0.44.0**: `bleep`, `censor --at/--dur`, `grade --warm`. **0.45.0**: `vdenoise`, `crop`, `title --position bottom`. **0.46.0**: `waveform`, `spectrogram`, `dehum`.

Deliver 1080×1920 −14 LUFS; caption burn without libass + `--safe social` bottom-20%; broll cutaway keeps A-roll audio/duration and plays B from its first frame; rough-cut speech islands (list, then encode only keeps); music duck (aformat dbl pin for sidechaincompress on apt ffmpeg); speed; jumpcut; cover; fade; title; loop; stabilize; reverse; grade/zoom/sharpen/vignette/bw/volume/blur; pipeline `$src`/`$in`/`expect`; GitHub Release zips; English + Chinese README; **0.26.0**: `denoise` (afwtdn/afftdn fallback), `compress --size` two-pass budget, `fit`/`broll --fit blur`, `audiogram`. **0.27.0**: `replace`, `slideshow`, `grade --lut`. **0.28.0**: `caption --chunk`, `slideshow --transition`/`--motion kenburns`, `replace --mix`. **0.29.0**: `split --every`, N-clip `concat --transition` chains. **0.30.0**: `key`, `split --at`. **0.31.0**: `grid`, `progress`, `volume --at/--dur`. **0.32.0**: `freeze`, `censor`, `speed --at/--dur`. **0.33.0**: `boomerang`, `chapter`, `zoom --at/--dur`, `key --despill`. **0.34.0**: `autocrop`, `sheet`, `title --at`. **0.35.0**: `replace --duck`, `pitch`, `grade --grain`. **0.36.0**: `split --scenes`, `cutsil`, `grid --audio`. **0.37.0**: `overlay --tile`, `caption --shift`, gif `--fps/--width`. **0.38.0**: `channel`, `loop --until`, `title --tile`. **0.39.0**: `eq`, `zoom --motion`, `broll --still`. **0.40.0**: `rotate`, `delogo`, `speed --interp`. **0.41.0**: `title --size/--color`, `meta`, `broll --motion`.
