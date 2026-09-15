# Changelog

Skill 与 CLI 共用一个 SemVer（`Cargo.toml` + `SKILL.md` 的 `version:`）。格式遵循 [Keep a Changelog](https://keepachangelog.com/)。

## [Unreleased]

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
