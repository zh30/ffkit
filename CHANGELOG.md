# Changelog

## [Unreleased]

## [0.109.0] — 2026-09-22

### Added

- `remux --audio` — audio-only stream copy (`-map 0:a`); re-encodes to mp3/ogg/wav/aac when the container can't hold the source codec
- `hls --fmp4` — CMAF fragmented-MP4 segments (`seg_*.m4s` + `init.mp4`) for flat and `--ladder` HLS packages; Safari/AirPlay-ready
- `loop --fade SECS` — seamless loop: copies joined by `xfade`/`acrossfade` joints so the loop point is invisible (re-encode path)
- `compress --res HEIGHT` — downscale before encoding (folded into both `--size` two-pass and `--crf` one-pass chains) to free bitrate at small size budgets

## [0.108.0] — 2026-09-22

### Added
- `subs --all` — extract every subtitle stream in one ffmpeg call (`stem_0.srt`, `stem_1.srt`, …); `probe` gains `subtitle_streams`.
- `mix --normalize` — amix `normalize=1` halves the summed level for two hot tracks.
- `fit --strength` — gblur sigma knob for `--fit blur` (default 30).
- `compress --crf N` — quality mode: single-pass libx264 crf (0–51) instead of the two-pass size budget; `--size`/`--target`/`--crf` is now an any-of requirement.

## [0.107.0] — 2026-09-22

### Added
- `silence --detect [--threshold --min]` — report-only silence ranges in JSON extras (no `-o`), for planning cuts before `split --silence`/`jumpcut`.
- `thumb --count N` — write N evenly-spaced stills (`stem_01..NN.ext`); `fps=(n-0.5)/dur` keeps every pick before EOF.
- `sheet --from/--to` — sample the contact sheet only within a window; `--time` stamps re-anchor to the window.
- `loudnorm --dynamic` — per-frame dynamic normalization (`linear=false` on the two-pass apply) for speech that a linear offset pumps on.

## [0.106.0] — 2026-09-22

### Added
- `vocal --amount 0..1` — partial center-channel cancel keeps backing bleed; partial isolate blends toward the center mix.
- `eq --tilt -10..10` — one-knob warm↔bright tone tilt mapped onto the bass/treble shelves (explicit `--bass`/`--treble` still win).
- `deinterlace --engine yadif|bwdif` — choose the smoother motion-compensated bwdif. (nnedi intentionally omitted: it aborts without an external weights file on every ffmpeg tested.)
- `insert --dur N` — splice only the first N seconds of the clip, in both the hard-cut and `--transition` xfade paths.

## [0.105.0] - 2026-09-23

### Added
- `mix --duck` — sidechain compression: the B bed ducks whenever A (voice) is loud (asplit + aformat so it works on ffmpeg <7).
- `compress --target discord|whatsapp|gmail` — platform size presets (8/16/25 MB) in place of `--size`.
- `subs --burn --outline N` — burned-caption stroke width via ASS `Outline`.


## [0.105.0] — 2026-09-22

## [0.104.0] - 2026-09-23

### Added
- `insert --transition T --duration D` — the splice crossfades into and out of the clip (two xfade joints + acrossfade), instead of a hard cut.
- `hls --audio-only` — `-vn` audio-only HLS packaging for podcasts and voice-over streams.
- `grid --fill` — tiles scale up and crop to fill each cell instead of letterboxing.


## [0.104.0] — 2026-09-22

## [0.103.0] - 2026-09-23

### Added
- `loudnorm --measure` — report-only loudness: measured I/TP/LRA in extras, no `-o` needed.
- `subs --convert` — .srt ↔ .vtt conversion (parses vtt cue settings/WEBVTT blocks, emits clean files).
- `art --extract` — pull the embedded cover stream out to an image (`--image` no longer required).
- `countdown --position` — place the counting digits anywhere on the 9-position map.


## [0.103.0] — 2026-09-22

## [0.102.0] - 2026-09-23

### Added
- `hls --ladder 1080,720,480` — ABR ladder: variant playlists + `master.m3u8`, tiered bitrates per height, one aac encode per variant (ffmpeg <7 rejects shared streams across groups).
- `concat --level LUFS` — one-pass loudnorm on every input before joining (mixed-source loudness).


## [0.102.0] — 2026-09-22

## [0.101.0] - 2026-09-23

### Added
- `multicam` — two-camera angle switching across an aligned pair: `--at t1,t2,...` flips angle per cut (trim/atrim + concat chain; pair with `align` when takes aren't synced).
- `insert` — splice a whole clip into the middle of a video at `--at T` (scaled/letterboxed to the base size).
- `split --subs in.srt` — write a per-part `.srt` next to each split file with cues re-timed and clamped to the part window.


## [0.101.0] — 2026-09-22

## [0.100.0] - 2026-09-23

### Added
- `align` — auto-sync a second recording to a reference via audio cross-correlation (pure-Rust radix-2 FFT over 16 kHz PCM, ±`--max-lag` window, reports `offset_ms`); shifts the target's audio onto the reference timeline.
- `scroll` — rolling end credits: text/file renders once and rises bottom→top across `--dur` (`--at`, `--size`, `--color`, `--font`).
- `countdown --text` — static label above the digits for the whole count window.


## [0.100.0] — 2026-09-22


## [0.98.0] - 2026-09-23

### Added
- `conform --crf` — x264 quality knob (0–51, default 18) for delivery exports.
- `grid --gap` — uniform pixel border around every tile.
- `chapter --import` — read chapter marks from a text file (`TIME|TITLE` or `TIME,TITLE` lines, `#` comments skipped).


## [0.98.0] — 2026-09-22


## [0.96.0] — 2026-09-22

### Added

- `eq --band FREQ:GAIN[:WIDTH_OCT]` — repeatable parametric bands on top of the shelf/curve (e.g. `--band 800:-3 --band 5200:2:0.7`); validation rejects out-of-range freq/gain/width.
- `transcode --preset prores` — ProRes 422 HQ (`prores_ks -profile:v 3`, `yuv422p10le`) + `pcm_s16le` in `.mov`, the FCP/Premiere edit-delivery format.
- `chapter --export` — write the resolved marks as an `.ffmetadata` text file at `-o` instead of embedding (hand chapters to an editor or DAW).


## [0.95.0] — 2026-09-22

### Added

- `tempo --at/--dur` — pitch-preserving retempo inside a window only (head/mid/tail `atrim`+`concat`); the rest plays at 1x. Audio-only inputs, as before.
- `broll --position` + `--scale` + `--margin` — PiP mode: the insert scales to a fraction of the frame (default 0.30) and pins to any corner/edge instead of full-screen cutaway. Same `--at`/`--duration`, `--fade`, `--audio` handling.
- `subs --burn --safe` — raises `MarginV` to clear the bottom 20% (or top 15% with `--top`) so burned captions sit above TikTok/Reels UI chrome.


## [0.94.0] — 2026-09-22

### Added

- `extract --gif --loop N` — GIF repeat count passthrough: `-1` plays once, `0`/unset loops forever, `N` loops N times.
- `meme --position top|center|bottom` — move the caption block: `center` stacks both texts mid-screen, `bottom` parks the pair low.
- `solid --text` / `--text-color` / `--font` — centered card text on the generated clip (end-cards, section titles) — one call instead of solid + title.


## [0.93.0] — 2026-09-22

### Added

- `fit --position` — anchor the picture inside the padded frame: top/bottom/left/right/corners, works for `--fit pad` and `--fit blur` (e.g. a bottom bar reserved for captions).
- `mute --at/--dur` — silence only inside a window (`volume=0` gate) instead of dropping the whole track; the rest of the audio and the video pass through copied.
- `boomerang --at/--dur` — forward-backward bounce only the chosen window; head and tail play straight (`--times` still loops the window).


## [0.92.0] — 2026-09-22

### Added

- `audiogram --subs` — burn an .srt's cues along the audiogram bottom strip (≤60 raster PNG overlays, same font pipeline as `--text`).
- `caption --fade` — soft caption in/out: each PNG loops as a 30 fps input with alpha `fade` in/out inside its window.
- `subs --rate` — rescale every cue timestamp by a factor (25↔23.976 fps drift, e.g. `--rate 0.959`).


## [0.91.0] — 2026-09-22

### Added

- `timer --down` — count down to the window end instead of up from `--at`.
- `broll --volume` — linear gain (0..=4) on the insert audio (needs `--audio`).
- `mix --loop` — `-stream_loop -1` on the B input so a short bed repeats under A.


## [0.90.0] — 2026-09-22

### Added

- `loudnorm --target` — platform loudness presets (spotify/youtube −14 LUFS, podcast −16, broadcast −23/LRA 7); explicit `--i`/`--tp`/`--lra` still win.
- `stabilize --edge` — deshake edge fill (blank|original|clamped|mirror).
- `extract --gif --bounce` — palindrome GIF: forward frames + reversed (`split→reverse→concat` before `paletteuse`).


## [0.89.0] — 2026-09-22

### Added

- `gate --at/--dur` — windowed noise gate (same dry/wet graph as denoise/leveler/pitch/dehum/vocal).
- `mix --at/--dur` — the B track enters only inside the window (`volume=between(t,…)` gate on the B feed).
- `waveform --at/--dur` — render just a slice: crops the rendered wave to the window then stretches to `--size`.
- `spectrogram --at/--dur` — render just a slice (`atrim` before `showspectrumpic`).

## [0.88.0] — 2026-09-22

### Added

- `pitch --at/--dur` — windowed pitch shift.
- `dehum --at/--dur` — windowed mains-hum notches.
- `vocal --at/--dur` — windowed karaoke/isolate.


## [Unreleased]

## [0.87.0] — 2026-09-22

### Added

- `timer --box-color C` / `caption --box-color C` — card behind the clock or captions.
- `replace --loop` — loop a short replacement track to fill the video.
- Fix: `--format ms` position math now counts the centisecond field.


## [Unreleased]

## [0.86.0] — 2026-09-22

### Added

- `caption --karaoke` — word-by-word caption reveal inside each cue.
- `denoise --at/--dur` — windowed noise cleanup.
- `leveler --at/--dur` — windowed compression.


## [Unreleased]

## [0.85.0] — 2026-09-22

### Added

- `freeze --reverse SEC` — rewinds the SEC before the hold, then freezes.
- `speed --ramp FROM,TO` — linear speed ramp (whole clip or --at/--dur window).
- `subs --merge FILE` — merge two .srt files into one, cues sorted by start.


## [Unreleased]

## [0.84.0] — 2026-09-22

### Added

- `freeze --ease SEC` — last SEC before the hold plays at half-speed (swoop-in).
- `sheet --time` — timestamp label under every tile.
- `meme --at/--dur` — time-windowed meme captions.


## [Unreleased]

## [0.83.0] — 2026-09-22

### Added

- `extract --gif --at --dur --fps` — 2-pass palette GIF clip straight out of a video.
- `cover --blur` — ambient blurred pad behind the 9:16 cover still.
- `audiogram --progress` — moving progress bar along the bottom edge.


## [Unreleased]

## [0.82.0] — 2026-09-22

### Added

- `title --box COLOR` — filled backplate behind title text (lower-third legibility).
- `censor --strength N` — mosaic block px / blur sigma.
- `chapter --auto MIN_GAP` — auto chapter marks after silences (podcast segments).


## [Unreleased]

## [0.81.0] — 2026-09-22

### Added

- `deinterlace --parity auto|tff|bff` — field-order override for DV/HDV/archival sources.
- `transcode --copy-audio` — keep the original audio bitstream while re-encoding video.
- `meme --outline N` — classic white-on-black-outline meme text.


## [Unreleased]

## [0.80.0] — 2026-09-22

### Added

- Shared `src/color.rs`: every `--color`-family flag now accepts names (`red`, `black`, `gold`, …) as well as `#RGB`/hex — fixes the `0xred` mangling in `solid`/`fit`/gradient paths.
- `hls --single` — byte-range single-`.ts` package (one file to upload).
- `hls --copy` — stream-copy repack (instant when input is already h264/aac).


## [Unreleased]

## [0.79.0] — 2026-09-22

### Added

- `timer --format ms` — mm:ss.cc centisecond timer field for sports/review overlays.
- `autocrop --buffer N` — expand the detected crop box by N px per side, clamped to frame.
- `rough --merge N` — merge keep ranges separated by less than N seconds.


Skill 与 CLI 共用一个 SemVer（`Cargo.toml` + `SKILL.md` 的 `version:`）。格式遵循 [Keep a Changelog](https://keepachangelog.com/)。

## [Unreleased]

## [0.78.0] - 2026-09-22

- `grid --labels "a,b,c"` — raster caption PNGs overlaid at the bottom of each tile (drawtext is absent on some ffmpeg builds, so labels share the raster pipeline).
- `delogo --soft` — generates a PNG mask and uses `removelogo` interpolation instead of the hard delogo box.
- `solid --gradient RRGGBB:RRGGBB` — animated gradient card via the `gradients` lavfi source.

## [0.77.0] - 2026-09-22

- `subs --mux file.srt --lang spa` — muxes a subtitle file in as a selectable stream (mov_text in mp4, srt in mkv) with a language tag; videos stay stream-copied.
- `caption --outline RRGGBB` — strokes each glyph (`render_caption_outlined` in raster) for readability on busy frames.
- `audiogram --font` — custom ttf/otf for the `--text` title card.

## [0.76.0] - 2026-09-22

- `reverb --at/--dur`, `eq --at/--dur` — windowed audio FX via the new shared `engine::audio_window` (dry-duck + wet-delay + amix), extracted from `fx`.
- `loop --from/--to` — loops only the middle section and keeps head/tail once (three concat arms; reencodes since it can't stream-copy).
- Fix: `loop=...:size=0` is a silent no-op — the filter needs an explicit frame/sample buffer. `boomerang --times` was silently emitting one cycle; buffer sizes now derive from probe fps/sample_rate.

## [0.75.0] - 2026-09-22

- `fx --kind` +`echo` (aecho), `lofi` (acrusher + lowpass), `radio` (telephone bandpass + compressor).
- `fx --at/--dur` reworked: ffmpeg 4.4 gives none of the FX filters timeline support, so the window is now a split → duck-dry / delay-wet → amix graph (works for every kind, chorus included).
- `transcode --preset hevc` — libx265 at crf 28 with the `hvc1` tag so QuickTime/Safari play it.
- `gate --preset voice|podcast|studio` — tuned threshold/ratio/attack/release curves.

## [0.74.0] — 2026-09-22

## [0.74.0] - 2026-09-22

- `fx` — audio FX rack: `--kind tremolo|vibrato|flanger|phaser|chorus` with `--strength` scaling depth and `--at/--dur` enable windows (chorus lacks timeline support and refuses `--at` cleanly).
- `boomerang --times N` — `loop`/`aloop` the fwd+rev cycle N times (duration ×N).

## [0.73.0] - 2026-09-22

- `overlay --angle DEG` — `format=rgba,rotate=a=rad:c=none` pre-chain ahead of any fade/opacity chain (diagonal watermarks).
- `waveform --scale lin|log|sqrt|cbrt` — showwavespic amplitude scale.
- `split --min-silence N` — silence-gap minimum for `--silence` mode (default 0.4s); standalone refuses.

## [0.72.0] - 2026-09-22

- `music --at/--dur` — delayed/trimmed bed window: `atrim` + `adelay=at*1000` before the ducking mix; fades stay relative to the bed.
- `channel --mode invert --side left|right|both` — `aeval='-val(0)|val(1)':c=stereo` polarity flip (pan rejects `-c0`; aeval needs `aformat` to restore the layout for aac).
- `sheet --pad/--margin` — tile spacing/outer margin in px.

## [0.71.0] - 2026-09-22

- `countdown --beep` — one `aevalsrc` source: `sin(2*PI*880*t)*lt(mod(t-at,each),0.12)` gated tick tone, amix'd with input audio (commas must stay inside the quoted expr — aevalsrc splits channels on them).
- `leveler --preset voice|podcast|master` — one-click acompressor curves.
- `spectrogram --color NAME` — showspectrumpic color scheme (validated against the ffmpeg list).

## [0.70.0] - 2026-09-22

- `zoom --out` — zoompan `z='max(factor-on*step,1.0)'`: starts at --factor, settles to 1x (reveal shot).
- `title --shadow N` — raster: blurred darkened copy of the title card offset down-right under the card (soft drop shadow).
- `extract --width` — `-vf scale=w:-2` on the still path (exact-width frames).

## [0.69.0] - 2026-09-22

- `subs --burn --font NAME` — font family in the libass force_style (brand captions).
- `progress --at/--dur` — windowed progress bar via overlay `enable=`.
- `audiogram --position top|center|bottom` — wave band y factor (0.18/0.50/0.62 of frame height).

## [0.68.0] - 2026-09-22

- `vignette --at/--dur` — windowed vignette (`enable='between(...)'` on the angle filter).
- `grade --at/--dur` — windowed grade: `:enable='...'` appended to every filter in the chain (eq/curves/hue/lut3d/colortemperature/colorbalance/noise all accept timeline on ffmpeg 4.4+).
- `broll --audio` — mixes the insert's audio (`atrim`+`adelay`+`amix`) into its window so b-roll sound is heard; requires a video insert with an audio stream.

## [0.67.0] - 2026-09-22

- `bw --at/--dur` — windowed desaturation (`hue=s=0:enable='between(...)'`).
- `sharpen --at/--dur` — windowed `unsharp` (same enable pattern; `--dur` alone refuses).
- `meta --clear` — `-map_metadata -1`, strips every container tag before publishing; refuses to combine with tag flags.

## [0.66.0] - 2026-09-22

- `invert --at/--dur` — windowed negation via `negate=enable='between(...)'` (flashback accents).
- `blur --at/--dur` — windowed `gblur` (same enable pattern; `--dur` alone refuses).
- `title --position` gains corners: `top-left|top-right|bottom-left|bottom-right` (6% margins).

## [0.65.0] - 2026-09-22

- `replace --fade N` — `afade` in/out on the swapped audio (fade-out pinned to the video's end; all three mix/duck/plain paths).
- `audiogram --text` — raster title composited near the top (`render_caption` PNG → second `overlay` stage, `shortest=1` on the looped input).
- `thumb --width` — scale the grabbed still (`scale=W:-2`).

## [0.64.0] — 2026-09-22

- `broll --fade N` — alpha fade in/out on the insert (`format=rgba` + `fade alpha=1` on the shifted B chain; stays rgba through `overlay`).
- `frames --at t1,t2,...` — grab stills at listed timestamps (N `-ss` seek jobs → `stem_NNN.ext`).
- `audiogram --size WxH` — canvas for YouTube (1920x1080) or square (1080x1080); waveform band scales with it.

## [0.63.0] - 2026-09-22

- `split --silence dB` — cuts at silence midpoints (`silencedetect` → `-f segment`; podcast → episode segments).
- `music --fade N` — `afade` in/out on the bed (`-stream_loop` is pinned to the talk's length for the fade-out point).
- `eq --preset voice|podcast|bright|bass` — one-shot curves; each band still overridable (preset only fills bands left at 0).

## [0.62.0] - 2026-09-22

- `subs --burn` style overrides: `--size` (FontSize), `--color` (RRGGBB → ASS `&H00BBGGRR`), `--top` (Alignment 8 vs 2).
- `audiogram --bg` — backdrop color when no `--image` cover (name or hex, default `0x101418`).
- `overlay --opacity` now works on `--image` stills too — `format=rgba,colorchannelmixer=aa=N` pre-chain for subtle watermarks (was `--mode`-only).

## [0.61.0] - 2026-09-22

- `cut --drop "a-b,c-d"` — remove middle sections, keep the rest joined (complement of `--ranges`; same N-trim + concat path).
- `title --outline RRGGBB` — stroke around every glyph (`render_title_outlined`: 16-offset ring blits under the fill) for readable text on busy frames.
- `fit --color RRGGBB` — pad bars in a brand color instead of black (`pad` filter color arg).

## [0.60.0] - 2026-09-22

- `cut --ranges "a-b,c-d"` — keep several ranges joined into one file (N `trim`/`atrim` + `concat`; always re-encodes for frame-exact, aligned pts).
- `solid` — new verb: standalone solid-color clip (`lavfi color`), `--color`/`--size`/`--dur`, optional silent stereo (`--audio=false`). For intro cards, lyric backplates, b-roll spacers.
- `volume --limit dBTP` — `alimiter` brickwall after the gain (linear ceiling from dBTP, `level=false`).

## [0.59.0] - 2026-09-22

- `overlay --fade` — alpha-fades any overlay (`--image` still or `--video`) in/out over its window; stills are `-loop 1` + `shortest=1` so the composite ends with the main stream.
- `subs --shift ±N` — retimes every cue of an .srt (clamped at 0), e.g. "字幕慢了 2 秒" → `subs in.srt --shift 2 -o out.srt`.
- `meta --album/--genre/--date/--track` — podcast & music library tags alongside title/artist/comment.

## [0.58.0] - 2026-09-22

- `title --fade` — fades the title in/out over N s (looped still + `fade alpha=1`; clamps to half the window).
- `grade --hue` — `hue=h=N` rotate (-180..180) for white-balance rescue or color FX.
- `silence --end` — append the `--dur` silence to the tail without doing the math.

## [0.57.0] - 2026-09-22

- `thumb` — single-frame cover grab: `--at 2.5` (fast `-ss` seek) or `--frame N` (exact `select`); writes jpg/png/webp.
- `subs --burn <file.srt|ass>` — libass `subtitles` burn-in with a creator-safe force_style (white text, outline, bottom margin). Extract mode unchanged.
- `split --parts N` — equal-length N-way split (derives `--every` from duration).

## [0.56.0] - 2026-09-22

- `sync` — fix constant A/V offset: `--ms +N` pads the audio start (`adelay`), `--ms -N` trims it (`atrim`); video stream-copied.
- `crop --anchor` — `--aspect` reframes at `center|top|bottom|left|right` instead of always centering (keep faces in 9:16 crops of landscape masters).
- `art` — attach a cover image (`--image`): `mjpeg` art stream + `id3v2`/attached_pic for mp3/m4a/mp4/mov/mkv; refuses containers that can't hold art.

## [0.55.0] - 2026-09-22

- `qa` — measure encode quality loss: `psnr`/`ssim` via `scale2ref` + metric filter, parsed into `extra` (psnr dB, ssim 0..1).
- `conform` — normalize a clip to a shared spec for concat/assembly: `--size WxH` (even-dim fit), `--fps`, `--lufs` one-pass loudnorm (resampled back to 48k after — loudnorm upsamples internally).
- `overlay --mode` — full-frame blend composite (`screen`/`addition`/`multiply`/`lighten`/`darken`/`overlay`/`difference`) with `--opacity` — light leaks, particles, film textures; refuses `--x/--y/--scale` since blending is full-frame.

## [0.54.0] - 2026-09-22

- `timer` — running MM:SS (H:MM:SS past the hour) counter burned into a corner, no drawtext needed: a 60-cell digit sprite + `crop x='mod(floor(t),60)*cell'` driven by `-loop 1` inputs; `--position`, `--at`, `--dur`, `--size`, `--color`, `--font`.
- `mute` — drop the audio stream, everything else stream-copied (`-map 0 -map -0:a -c copy`).
- `hls` — package for web embeds: `dir/index.m3u8` + `seg_*.ts`, `--seg` segment seconds, h264+aac transcode for player compat.

## [0.53.0] - 2026-09-22

- `mix` — sum two audio sources at full level (`amix normalize=0` + `aresample`), `--vol-a`/`--vol-b` linear trims, `--longest` to run to the longer input; keeps input A's video as-is.
- `caption --position top` — burn captions in the upper safe zone (top ~15% social / 10% off).
- `grade --gamma` — mid-tone `eq=gamma` slider (verified on ffmpeg 4.4).

## [0.52.0] - 2026-09-22

- `split --size 9MB` — aim chunks at a byte cap (Discord/WhatsApp uploads) by deriving the even grid from input size; also fixes bare-stem outputs (`-o part.mp4`) failing in `split` and `frames` with a bogus ENOENT after the parts were already written.
- `countdown` — 3-2-1(-GO!) intro overlay (`--from`, `--each`, `--at`, `--go`, styled via `--size`/`--color`/`--font`); rendered by the title rasterizer so no drawtext fontfile wrangling.
- `invert` — negative colors via `negate` (clone-VFX base, flash frames).

## [0.51.0] — 2026-09-22

- `ffkit crossfade`: `acrossfade` blend of two audio files (`--second`, `--dur`)
- `ffkit strip`: drop all metadata + chapters, lossless stream copy
- `ffkit frames`: still dump every `--every` seconds (`stem_%03d.png`, `--width`)

## [0.50.0] — 2026-09-22

## [0.49.0] — 2026-09-22

- `ffkit vocal`: `--mode karaoke` removes center-panned vocals; `isolate` keeps only the center (stereo sources)
- `ffkit remux`: container swap without re-encoding (`-c copy`, faststart on mp4/mov)
- `ffkit meme`: `--top`/`--bottom` caption text burned onto video

## [0.48.0] — 2026-09-22

- `ffkit silence`: insert `--dur` seconds of quiet at `--at` in audio files (video holds stay with `freeze`)
- `grade --preset cinematic|vivid|vintage|soft`: one-click looks applied before the sliders
- `transcode --fps`: retimes h264/webm video too, not only GIF

## [0.47.0] — 2026-09-22

- `ffkit tempo`: audio speed `--factor` 0.5–8, pitch preserved (`atempo` chain; refuses video — use `speed`)
- `ffkit leveler`: voice dynamic-range compressor (`acompressor`)
- `ffkit gate`: noise gate below `--threshold` dB (`agate`)

## [0.46.0] — 2026-09-22

- `ffkit waveform`: audio waveform → PNG (`--size`, `--color`) via `showwavespic`
- `ffkit spectrogram`: audio spectrogram → PNG (`--size`) via `showspectrumpic`
- `ffkit dehum`: notch mains hum `--mains 50|60` + `--harmonics` (`equalizer` Q=12 chain)

## [0.45.0] — 2026-09-22

- `ffkit vdenoise`: spatial video denoise for grainy footage (`--strength` 0.5–30) via `nlmeans`
- `ffkit crop`: crop a `--region x:y:w:h` box or `--aspect W:H` center-reframe
- `title --position bottom`: lower-third placement (joins `top`/`center`)

## [0.44.0] — 2026-09-22

- `ffkit bleep`: censor-tone over a window (`--at`/`--dur`, `--freq`, `--level`) — silences the source and mixes a delayed `sine`
- `censor --at`/`--dur`: mosaic/blur only inside a window
- `grade --warm -1..1`: white-balance warmth via `colortemperature`

## [0.43.0] — 2026-09-22

- `ffkit reverb`: room/hall/cave ambience on a voice (`--size`, `--wet`) via `aecho`
- `audiogram --mode`/`--color`: waveform style (point/line/p2p/cline) and colour
- `delogo --at`/`--dur`: blur the logo box only inside a window
- `meta --rotate 0/90/180/270`: fix the display-rotation flag losslessly (uses `-display_rotation` on ffmpeg ≥7, `rotate` metadata below)

## [0.42.0] — 2026-09-22

- `overlay --at`/`--dur`: windowed logo/overlay (single and `--tile` paths)
- `caption --color`/`--size`: styled caption raster (same knobs as `title`)
- `ffkit subs`: extract embedded subtitle tracks to `.srt`/`.vtt`/`.ass`


## [0.41.0] — 2026-09-22

- `title --size`/`--color`: text styling on the burned-in title
- `ffkit meta`: write `title`/`artist`/`comment` container tags (lossless copy)
- `broll --still --motion kenburns`: animated push on an image cutaway


## [0.40.0] — 2026-09-22

- `ffkit rotate`: `--deg 90/180/270` or `--flip h|v` for mis-oriented phone clips
- `ffkit delogo`: blend out a burned-in logo/watermark box (`--x --y --w --h`)
- `speed --interp`: `minterpolate` blend upsampling for smooth slow-mo (factor < 1)


## [0.39.0] — 2026-09-22

- `ffkit eq`: `--bass`/`--treble`/`--presence` dB shelving on audio
- `zoom --motion kenburns`: animated zoompan push to `--factor`, whole clip or inside `--at/--dur`
- `broll --still`: cut away to a still image (looped over the window)


## [0.38.0] — 2026-09-22

- `ffkit channel`: `--mode dualmono` (one-ear fix), `mono` (fold-down), `swap` (L/R flip)
- `loop --until SEC`: repeat+trim to a target length
- `title --tile N`: text tiled diagonally as a draft watermark
- fix: `overlay --tile` splits the overlay pad for ffmpeg ≤5 (labels consumed once)


## [0.37.0] — 2026-09-22

- `overlay --tile N`: tiled semi-transparent watermark pass (draft protection)
- `caption --shift SEC`: nudge every cue (negative pulls earlier)
- `transcode --preset gif --fps/--width`: gif tuning flags


## [0.36.0] — 2026-09-22

- `split --scenes T`: auto-detect shot changes and cut there (select scene score + segment muxer)
- `ffkit cutsil`: `silenceremove` head+tail dead air (audio-only; video → jumpcut)
- `grid --audio N`: keep one input's audio instead of the amix


## [0.35.0] — 2026-09-22

- `replace --duck`: sidechain-duck the original track under the replacement audio
- `ffkit pitch`: `--semitones` shift with preserved duration (asetrate+atempo)
- `grade --grain`: film-grain `noise=alls:allf=t+u` pass


## [0.34.0] — 2026-09-22

- `ffkit autocrop`: cropdetect scan → crop out letterbox/pillarbox (refuses when nothing detected)
- `ffkit sheet`: `--cols`/`--rows`/`--tile` contact-sheet PNG
- `title --at S`: title/lower-third at any offset, not just t=0


## [0.33.0] — 2026-09-22

- `ffkit boomerang`: forward + reversed replay loop
- `ffkit chapter`: embed `--at T|TITLE` markers via ffmetadata + `-c copy` (lossless)
- `zoom --at S [--dur D]`: punch-zoom only inside a window
- `key --despill`: `despill=type=green` cleanup pass on keyed edges


## [0.32.0] — 2026-09-22

- `ffkit freeze`: mid-clip hold (`--at T --dur D`) or outro hold (`--end D`) via `tpad` clone; the held window's audio is silence
- `ffkit censor`: mosaic/blur a region `--region x:y:w:h` (`--mode pixel|blur`)
- `speed --at S [--dur D]`: speed-ramp only one window (3-segment trim/concat)


## [0.31.0] — 2026-09-22

- `ffkit grid`: N inputs into a `--layout CxR` tile wall via `xstack`; audio mixes when every input has it
- `ffkit progress`: bottom/top fill bar over the duration (`--color`, `--height`, `--edge`)
- `volume --at S [--dur D]`: apply `--db` gain only inside a window (mute a moment)


## [0.30.0] — 2026-09-22

## [0.29.0] — 2026-09-22

## [0.28.0] — 2026-09-22

- `caption --chunk N`: split each cue into ≤N-word groups sharing its span — the chunked-caption look without word timing (burn mode)
- `slideshow --transition <name>`: xfade flavor between stills (`wipe*`, `slide*`, `dissolve`, `radial`, `circleopen`); `--motion kenburns`: zoompan push-in/pull-out per still
- `replace --mix G`: keep the original track under the new one at linear gain G

## [0.27.0] — 2026-09-22

- `ffkit replace`: swap a video's audio track (`--audio`, `--audio-offset`); video stream-copied, new audio padded/trimmed to length
- `ffkit slideshow`: still images → montage (`--per`, `--fade` xfade chain, `--audio` bed, `--size` canvas); silent track written when no bed
- `grade --lut file.cube`: apply a 3D LUT after the slider correction

## [0.26.0] — 2026-09-22

- `ffkit denoise`: `highpass` + voice denoise (`afwtdn` where ffmpeg ≥5.1 ships it, else `afftdn` with raised floor); `--video` adds `hqdn3d` degrain
- `ffkit compress --size 10MB`: two-pass bitrate budget that lands under a platform cap (audio-only inputs single-pass `-b:a`)
- `fit --fit blur`: blurred-pillarbox fill for repurpose; `broll --fit blur` gets the same mode
- `ffkit audiogram`: podcast audio → 1080×1920 `showwaves` video over a cover still (`--image`) or flat colour
- Creator-gap research refreshed in `docs/creator-needs.md` (size caps, podcast→clips, denoise, blur fill)


## [0.25.1] — 2026-09-16

- `music --duck` pins both legs to `aformat=dbl` so Ubuntu/apt ffmpeg can negotiate `sidechaincompress`

## [0.25.0] — 2026-09-15

- `ffkit rough`: list speech islands (audio-only detect), then assemble only those windows — not a full-timeline re-encode

## [0.24.1] — 2026-09-15

- `broll` delays the insert with `setpts` so `--at` plays B-roll from its first frame (short clips no longer freeze)

## [0.24.0] — 2026-09-15

- `ffkit broll --insert --at --duration`: cut away to B-roll; A-roll audio and duration stay

## [0.23.0] — 2026-09-15

- `caption --mode burn` defaults to `--safe social` (above the bottom 20% of the frame)
- Remaining 2026 creator-gap research in `docs/creator-needs.md`

## [0.22.1] — 2026-09-15

- README.md is English by default; Chinese lives in README.zh.md (edit both together)

## [0.22.0] — 2026-09-15

- GitHub Release zips (binary + skill + `install.sh`) on every versioned merge to `main`
- README install path is the Release zip; Source code zip is not the bundle

## [0.21.0] — 2026-09-15

- Pipeline plans take `input` (`$src`), previous output (`$in`), step `label`, and `expect` (sets `verified`)
- Recipe schemes in `references/recipes.md` to adapt after the user agrees

## [0.20.0] — 2026-09-15

- Skill loop is chat → proposed scheme → hands finish the original task (not a verb menu)
- `ffkit pipeline plan.json` runs that scheme (stops on first failure)

## [0.19.0] — 2026-09-15

- `ffkit blur --sigma`: gaussian blur (`gblur`)

## [0.18.0] — 2026-09-15

- `ffkit volume --db`: simple gain (not LUFS)

## [0.17.0] — 2026-09-15

- `ffkit bw`: strip color (`hue=s=0`)

## [0.16.0] — 2026-09-15

- `ffkit vignette`: darken corners for a Reels look

## [0.15.0] — 2026-09-15

- `ffkit sharpen`: unsharp after social re-encode

## [0.14.0] — 2026-09-15

- `ffkit zoom --factor`: center punch-in (talking-head crop)

## [0.13.0] — 2026-09-15

- `ffkit grade`: Reels-style contrast/saturation/brightness pop (`eq`)

## [0.12.0] — 2026-09-15

- `ffkit reverse`: play picture and sound backwards

## [0.11.0] — 2026-09-15

- `ffkit stabilize`: deshake handheld footage

## [0.10.0] — 2026-09-15

- `ffkit loop --times`: lossless concat-repeat for Shorts replay length

## [0.9.0] — 2026-09-15

- `ffkit title --text`: first-second hook card via raster overlay (no libass)

## [0.8.0] — 2026-09-15

- `ffkit fade --in/--out`: video + audio fade

## [0.7.0] — 2026-09-15

- `ffkit cover`: 1080×1920 still for Reels / TikTok / Shorts thumbnails

## [0.6.0] — 2026-09-15

- `ffkit jumpcut`: drop internal silence for talking-head jump cuts

## [0.5.0] — 2026-09-15

- `ffkit music --track`: loop a bed under speech with sidechain ducking

## [0.4.0] — 2026-09-15

- `ffkit speed --factor`: setpts + chained atempo (0.25×–8×, pitch kept)

## [0.3.0] — 2026-09-15

- `caption --mode burn` rasterizes SRT and overlays PNGs; no libass/`drawtext` required

## [0.2.0] — 2026-09-15

- `ffkit deliver`: one-shot 9:16 social export (1080×1920, 30 fps, −14 LUFS, H.264+AAC+faststart)
- Cited creator-needs research in `docs/creator-needs.md`

## [0.1.0] — 2026-09-15

- 首个可安装 skill：probe / cut / concat / fit / extract / overlay / caption / loudnorm / transcode / look / batch / graph / ffmpeg / doctor / install-skill
- `ffkit ffmpeg` 必须 `--because`
- `look --at` 可重复；caption burn 依赖 libass
- `ffkit version` 报告二进制、嵌入 skill、已安装拷贝
