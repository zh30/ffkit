---
name: ffkit
description: Help a user finish a local video or audio job. Chat about the outcome, propose a short plan, then run that plan with ffkit (pipeline of verbs, graph, or ffmpeg). Use when they mention a media file (mp4, mov, mkv, webm, wav, m4a, mp3, gif), footage, clip, Reel/Short/TikTok/YouTube, captions (mux or burn without libass), overlay, transcode, ffmpeg, rough cut, assembly, or an edit, export, or effect on files they have on disk. Requires ffmpeg, ffprobe, and ffkit on PATH matching this skill's version field.

version: 0.392.0



compatibility: Requires ffmpeg, ffprobe, and the ffkit binary on PATH.
---

# ffkit

The user talks to you. You propose a scheme. Then you use these hands to finish **that original task**.

`ffkit` is the hands. Flags: `ffkit <verb> --help`. Numbers: `--json` / `--json-brief`. Shared: `--dry-run`, `--json`, `--overwrite`, `--timeout`, `--progress`.

If `ffkit` is missing, install the GitHub Release zip that matches this skill's `version:` (not Source code), then `./install.sh`. If `ffkit version --check` fails, `ffkit install-skill` and reload. After a missing-tool failure, `ffkit doctor --json`.

## Workflow

1. **Chat** until you can restate the outcome in one sentence in their language (what they will post or keep). The original task is the **last agreed** outcome, not the first message.
2. **Propose a plan** of 3–7 steps in their language (not flag soup). If they asked to review first, stop here. If they change the plan, execute the new one.
3. **Probe** each input (`ffkit probe FILE --json`). Numbers in the plan come from that. A long take they want roughed: `ffkit rough FILE --json` first (speech islands, no write), then assemble with `-o` after they agree.
4. **Run the plan.** Two or more writing steps: pipeline JSON then `ffkit pipeline plan.json --json` — [pipeline.md](references/pipeline.md). `goal` is the restated outcome; `expect` is the numbers that make it true — shape (`aspect`, `fps`, `vcodec`, `max_mb`, `av_drift_max`) and, when the outcome is about sound or picture, measured signal (`lufs`, `true_peak_max`, `max_black`, `max_freeze`, `ssim_min` for re-encodes of the same timeline). Take target numbers from [platforms.md](references/platforms.md) or the user, never from the verb you ran; `$src` is `input`, `$in` is the previous output. Adapt a recipe only after they agreed to that scheme — [recipes.md](references/recipes.md). One writing step: that verb. No matching verb: `graph`, then `ffmpeg --because`.
5. Prefer **lossless** (`cut` copy, mux captions, `loop` concat) unless pixels or samples must change.
6. Picture changed: `ffkit look`. Overlay/`caption`/`title`: `--at` a time the graphic is on. `broll`: `--at` the cutaway and a time after it. Fade: `--at 0` vs mid. Cover: inspect the PNG.
7. Never write onto the user's source.

Done when the **original task** is true (`expect` / probe match, and look if the picture changed)—not when the last process exited 0. `expect` re-probes and decodes the file on disk with ffmpeg's own analyzers, so it is evidence independent of the step that wrote it. If `verified` is false, the file exists; quote the failing `checks` rows and revise the plan.

Ask one question only when it changes the file and probe cannot answer it. Which cut is interesting, and what looks cinematic, stay with the user.

## Hands

`ffkit --help` is the full list. Match an agreed plan step to a tool. Compose several rows when the job needs several steps.

| Plan step | Hands |
|-----------|-------|
| inspect | `doctor`, `probe`, `look` |
| trim / join | `cut`, `concat`, `split`, `rough` |
| platform loudness | `loudnorm` |
| frame / size | `fit`, `crop`, `blur`, `zoom` |
| export | `deliver`, `conform`, `transcode`, `remux`, `compress`, `audiogram`, `slideshow`, `split` |
| captions / mute | `caption` |
| hook text | `title` |
| dutch-angle tilt (full clip or windowed) | `rotate` |
| chapters already in the file | `chapter`, `split` |
| thumbnail candidates | `thumb` |
| spectrum-bar audiogram | `audiogram` |
| lissajous vectorscope video | `audiogram` |
| news-ticker crawl | `scroll` |
| AV1 delivery | `transcode` |
| podcast/voice → mp3/m4a/wav/flac/opus | `transcode` |
| podcast feed pack (−16 LUFS spec) | `deliver`, `chapter`, `conform` |
| resample/force channels on export | `transcode` |
| audiogram of just the best bit(s) | `audiogram` |
| cover still | `cover` |
| speech / music | `jumpcut`, `denoise`, `music`, `replace`, `loudnorm`, `volume` |
| grainy low-light footage | `vdenoise` |
| blocky re-uploaded/screen-rec footage | `deblock` |
| colored halo on tape/capture | `chromashift` |
| moving people/cars on a tripod shot, rain streaks | `tmedian` |
| tourists/noise across 3+ copies of the same locked shot | `stack` |
| waveform PNG of audio | `waveform` |
| audio spectrogram PNG | `spectrogram` |
| watch loudness while it plays | `meter` |
| mains hum / electrical buzz | `dehum` |
| faster/slower podcast | `tempo`, `speed` |
| voice all over the place | `leveler` |
| hiss between sentences | `gate` |
| pad in room tone / breath | `silence`, `freeze` |
| one-click look | `grade` |
| warm faces only | `grade` |
| HALD image LUT | `grade` |
| karaoke / keep only the vocal | `vocal` |
| container swap, no re-encode | `remux`, `chapter` |
| top/bottom caption meme | `meme` |
| fix my podcast voice | `voice` |
| slideshow that runs exactly N seconds | `slideshow` |
| old interlaced footage | `deinterlace` |
| fade to white | `fade` |
| blend two audio files | `crossfade` |
| strip location/device tags | `strip` |
| stills every N seconds | `frames` |
| 3-2-1 intro countdown | `countdown` |
| invert / negative look | `invert` |
| split to fit a size cap | `split` |
| merge two audio sources at full level | `mix` |
| captions on top instead of bottom | `caption` |
| lift/crush mid-tones | `grade` |
| drop the audio track entirely | `mute` |
| elapsed-time corner counter | `timer`, `countdown` |
| web-embed HLS package | `hls`, `dash` |
| go live / push a stream | `live`, `deliver` |
| check encode quality loss | `qa` |
| normalize mixed footage for concat | `conform` |
| light-leak / screen-blend overlay | `overlay` |
| fix audio/video sync drift | `sync` |
| sync a second take to the camera master | `align` |
| rolling end credits | `scroll` |
| splice a clip into the middle | `insert` |
| two-camera angle switching | `multicam` |
| reframe 9:16 keeping faces | `crop` |
| attach album cover art | `art` |
| grab a cover/thumbnail frame | `thumb`, `extract`, `cover` |
| burn an .srt/.ass into pixels | `subs`, `concat`, `split`, `cut` |
| split into exactly N parts | `split` |
| title that fades in/out | `title` |
| end-card title / tail-only effect | `thumb`, `cover`, `frames` |
| dip-to-black at a cut / at every scene mark | `fade` |
| fix white balance / color cast | `grade` |
| quiet tail on a podcast | `silence` |
| logo/watermark that eases in | `overlay` |
| subtitle file is early/late (all or just a stretch) | `subs` |
| full podcast/music tags | `meta` |
| keep only the good parts | `cut` |
| solid color card / backplate | `solid` |
| boost without clipping | `volume` |
| rip out a middle section | `cut` |
| keep only the tail / several parts | `cut` |
| text that survives busy frames | `title` |
| bars in brand color | `fit` |
| burned subs, your style | `subs` |
| audiogram in brand colors | `audiogram` |
| subtle watermark | `overlay` |
| split a podcast on pauses | `split` |
| music bed that eases in/out | `music` |
| one-word EQ curve | `eq` |
| soft b-roll cutaway edges | `broll` |
| stills at exact moments | `frames` |
| audiogram on any canvas | `audiogram` |
| swapped audio eases in/out | `replace` |
| audiogram with a title | `audiogram` |
| thumbnail at a given size | `thumb` |
| inverted flash/accent | `invert` |
| blur just a moment | `blur` |
| motion trails / ghost smears | `trail` |
| datamosh glitch | `glitch` |
| partial invert | `solarize` |
| breathing zoom | `pulse` |
| timelapse flicker fix | `deflicker` |
| relief emboss | `emboss` |
| tilt-shift miniature | `tilt` |
| handheld drift | `sway` |
| shaky walking / handheld shot | `stabilize` |
| rack-focus breathing | `rack` |
| ink the edges | `outline` |
| night-vision look | `night` |
| falling snow | `snow` |
| impact punch (flash+shake) | `impact` |
| watery wave distortion | `wave` |
| pendulum sway tilt | `spin` |
| spotlight circle mask | `iris` |
| radial zoom smear | `burst` |
| sub-bass drop | `thump` |
| rising chirp to a hit | `riser` |
| airy whoosh swell | `whoosh` |
| voice sibilance tamer | `deesser` |
| field-recorder mid/side stereo | `channel` |
| loud+quiet speech in one file | `leveler` |
| clipped/blown-out audio rescue | `declip` |
| magnify subtle motion | `amplify` |
| sky/gradient banding fix | `deband` |
| drop duplicate frames | `dedup` |
| audiogram CQT music spectrum | `audiogram` |
| audiogram scrolling spectrogram | `audiogram` |
| find black/frozen stretches (QC) | `scan`, `remux` |
| dust specks / hot pixels | `dedust` |
| inverse telecine | `deinterlace` |
| denoise without melting detail | `vdenoise` |
| fill border slivers | `extend` |
| HDR → SDR | `tonemap` |
| 24p → NTSC interlace | `telecine` |
| straight ↔ premult alpha | `premult` |
| star trails / light painting | `stack` |
| pullup judder wobble | `dejudder` |
| texture smooth, no blur | `smooth` |
| mono-collapse check | `scan` |
| grain/noise budget check | `scan` |
| keyed edge still green | `despill` |
| clean 3:2 cadence | `deinterlace` |
| butter-smooth 60fps | `interp` |
| archive interlaced | `deinterlace` |
| white balance by Kelvin | `grade` |
| SD looks green in HD edit | `matrix` |
| film-style B&W | `bw` |
| blockbuster split-tone | `grade` |
| two-zone cast repair | `grade` |
| freeform tone curve | `grade` |
| broadcast QC clamp | `legalize` |
| lifted web rip | `levels` |
| xerox / graphic B&W | `bw` |
| audio peak & mean | `scan` |
| loudness spec gate | `scan` |
| heat ripple / liquid warp | `displace` |
| sharpen without halos | `sharpen` |
| EQ applied + curve shown | `eqviz` |
| pixel-art / retro upscale | `upscale` |
| smarter saturation | `grade` |
| crunchy edge outlines | `edge` |
| real noise-cancel | `denoise` |
| cheap 60fps | `interp` |
| pick a good still | `thumb` |
| meter-bridge audiogram | `audiogram` |
| stereo-field audiogram | `audiogram` |
| blur QC | `scan` |
| connected line art | `edge` |
| rescue a crushed rip | `smooth` |
| motion-only ghost | `trail` |
| pipeline-health audiogram | `audiogram` |
| mood colour wash | `grade` |
| match a reference look | `grade` |
| speed-line blur | `blur` |
| fast rip rescue | `smooth` |
| analog capture cleanup | `vdenoise` |
| hex pixel readout | `scope` |
| false-color glitch | `glitch` |
| chroma borrow | `grade` |
| film-grain denoise | `vdenoise` |
| deblock twins | `smooth` |
| VHS stutter | `glitch` |
| 1D LUT support | `grade` |
| hard flicker kill | `deflicker` |
| audio beds | `gen` |
| chroma-swap look | `glitch` |
| edit-point map | `scan` |
| 3D format convert | `stereo` |
| play an image as sound | `sonify` |
| robot voice | `fx` |
| edge-preserving smooth | `smooth` |
| archive deinterlace | `deinterlace` |
| precise logo mask | `delogo` |
| compression QC | `scope` |
| cheap-lens fringe | `aberrate` |
| mono-compat visual | `audiogram` |
| beauty/skin smoothing | `smooth` |
| reframe 360/equirect footage | `v360` |
| noisy clip, pick denoiser | `vdenoise`, `denoise` |
| old footage is too low-res | `upscale` |
| filmed a screen/whiteboard at an angle | `perspective` |
| broadcast range tag | `transcode` |
| real temporal interlace | `transcode` |
| halo-free sharpening | `sharpen` |
| auto-contrast flat footage | `equalize` |
| dominant colors | `pick` |
| visual diff | `diff` |
| glitched/dropped frames | `repair` |
| magnify subtle motion | `amplify` |
| keep one color | `selective` |
| test card | `bars` |
| QC scope overlay | `scope` |
| anamorphic restore | `desqueeze` |
| comic look | `cartoon` |
| thermal luma map | `heat` |
| mandala mirror | `kaleido` |
| mirror symmetry (dance) | `mirror` |
| retro pixelation | `pix` |
| unmirror selfie / flip art | `flip` |
| pop-art posterize | `poster` |
| two-color grade | `duotone` |
| dreamy bloom | `glow` |
| retro tape look | `vhs` |
| shutter smear | `motionblur` |
| flash cut | `strobe` |
| neon outline | `edge` |
| fisheye / defish | `lens` |
| text in a corner | `title` |
| B&W only for a moment | `bw` |
| sharpen only the key shot | `sharpen` |
| wipe metadata before posting | `meta` |
| dark-edge only for a beat | `vignette` |
| grade only the dream sequence | `grade` |
| hear the b-roll under me | `broll` |
| captions in my brand font | `subs` |
| progress bar only in the back half | `progress` |
| waveform band at the top | `audiogram` |
| pull OUT of a shot (reveal) | `zoom` |
| title with a soft shadow | `title` |
| still at an exact width | `extract`, `remux` |
| countdown with tick beeps | `countdown` |
| one-word compressor curve | `leveler` |
| spectrogram in brand colors | `spectrogram` |
| music kicks in after the intro / sting at both ends | `music` |
| fix an out-of-phase mic | `channel` |
| rescue dark or blown footage | `grade` |
| contact-sheet breathing room | `sheet` |
| diagonal watermark | `overlay` |
| short overlay clip repeats | `overlay` |
| waveform showing quiet detail | `waveform` |
| split on longer pauses | `split` |
| wobble/sci-fi/echo/lofi/telephone voice | `fx` |
| effect only in the drop | `fx` |
| boomerang that loops 3x | `boomerang` |
| H.265 for Apple / smaller archive | `transcode` |
| edit proxies for the NLE | `transcode` |
| gate tuned for speech vs studio | `gate` |
| echo/reverb only on the hook | `reverb` |
| bass boost only on the drop | `eq` |
| repeat just the funny bit | `loop` |
| selectable soft subs in mp4 | `subs` |
| stroked TikTok captions | `caption` |
| branded audiogram title font | `audiogram` |
| name each tile in a grid | `grid` |
| gentle logo cleanup | `delogo` |
| animated gradient card | `solid` |
| reframe / crop out an edge | `crop` |
| motion / loop | `speed`, `reverse`, `loop`, `stabilize`, `fade` |
| picture | `grade`, `bw`, `vignette`, `sharpen`, `blur` |
| logo / PiP | `overlay` |
| green screen | `key` |
| hand-drawn matte → alpha | `key` |
| reaction / multi-cam grid | `grid` |
| watch-time progress bar | `progress` |
| freeze a beat / outro hold | `freeze` |
| blur a face / logo | `censor` |
| slow-mo punch-in | `speed` |
| boomerang replay | `boomerang` |
| YouTube/player chapters | `chapter`, `split` |
| punch-zoom a moment | `zoom` |
| strip letterbox/pillarbox | `autocrop` |
| contact sheet / preview grid | `sheet` |
| player seek-preview thumbnails | `sprite` |
| title card mid-clip | `title` |
| voice-over on video's own audio | `replace` |
| pitch-shift voice/music | `pitch` |
| film grain | `grade` |
| auto cut on scene changes | `split` |
| strip dead air head+tail (audio) | `cutsil` |
| grid with one input's audio | `grid` |
| draft/tiled watermark | `overlay` |
| shift subtitle timing | `caption` |
| gif tuning | `transcode`, `extract` |
| headphone fatigue on long audio | `fx` |
| draw a freehand EQ curve | `eq` |
| animated backdrop for a music/text card | `gen`, `grade`, `silence` |
| italic-style slant / dynamic tilt | `shear` |
| fix a color cast / white balance | `wb` |
| QC a clip for strobes before posting | `scan` |
| stereo too wide / phase issues | `channel` |
| one-ear voice fix / pan the mix | `channel`, `split`, `sync` |
| loop to a length | `loop` |
| text draft watermark | `title` |
| audio EQ polish | `eq` |
| animated push-in | `zoom` |
| still-image cutaway | `broll` |
| wrong-orientation phone clip | `rotate` |
| burned-in logo/watermark | `delogo` |
| smooth slow-mo | `speed` |
| styled title text | `title` |
| lower-third placement | `title` |
| container metadata tags | `meta` |
| fix display rotation flag | `meta` |
| room tone on a voice | `reverb` |
| wobble/sci-fi/echo/lofi/telephone/saturate/excite/crush audio | `fx` |
| Ken Burns on a photo cutaway | `broll` |
| styled captions | `caption` |
| logo only for part of the clip | `overlay` |
| rip embedded subtitles | `subs` |
| bleep out a word | `bleep` |
| warm/cool white balance | `grade` |
| B-roll cutaway | `broll` |
| extract | `extract` |
| many files | `batch` |
| no verb | `graph` — [graph.md](references/graph.md) |
| still uncovered | `ffmpeg` `--because "REASON"` `-- -i IN … OUT` |

`pipeline` runs the scheme. `install-skill` and `version` are setup, not plan steps.

## Report

User language for the prose; keep these labels in English. Numbers from `--json`. Quote the plan you ran.

```
Plan: 剪掉停顿 → 片头字 → 烧字幕 → 导出 Reel
Done: reel.mp4 — 58.2s 1080x1920 30fps h264 aac
Steps: probe -> jumpcut -> title -> caption burn -> deliver reels -> look --at 1
Check: matches “做成 Reel 并能静音看完” (verified: true)
Look: reel_frame.png (hook readable, captions clear of the bottom UI)
Notes: …
```

`Failed:` quote `error.message`. A pipeline failure names the step. `Look: not needed` when the picture did not change.

## References

- [pipeline.md](references/pipeline.md) — the plan file `ffkit pipeline` runs (`$src`, `$in`, `expect`)
- [recipes.md](references/recipes.md) — schemes to adapt after the user agrees
- [gotchas.md](references/gotchas.md) — VFR, even sizes, concat copy, GIF palette
- [graph.md](references/graph.md) — filter graph when verbs cannot express a step
- [platforms.md](references/platforms.md) — Reels / Shorts / YouTube / GIF
