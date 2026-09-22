# Changelog

Skill 与 CLI 共用一个 SemVer（`Cargo.toml` + `SKILL.md` 的 `version:`）。格式遵循 [Keep a Changelog](https://keepachangelog.com/)。

## [Unreleased]

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
