# Changelog

Skill 与 CLI 共用一个 SemVer（`Cargo.toml` + `SKILL.md` 的 `version:`）。格式遵循 [Keep a Changelog](https://keepachangelog.com/)。

## [Unreleased]

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
