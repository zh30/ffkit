# Changelog

Skill 与 CLI 共用一个 SemVer（`Cargo.toml` + `SKILL.md` 的 `version:`）。格式遵循 [Keep a Changelog](https://keepachangelog.com/)。

## [Unreleased]

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
