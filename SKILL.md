---
name: ffkit
description: Operate local video and audio with the ffkit CLI wrapping FFmpeg: probe, cut, concat, fit, overlay, caption (mux or burn-in), extract, transcode, deliver (9:16 social export), speed, music, jumpcut, cover stills, fade in/out, loudness, batch, filter graphs, and raw ffmpeg with --because. Use when the user mentions a media file (mp4, mov, mkv, webm, wav, m4a, mp3, gif), footage, clip, captions, overlay, BGM, cover, thumbnail, fade, transcode, ffmpeg, Reel/Short/TikTok/YouTube, or asks to trim, join, resize, speed up, add a track, cut silence, extract a cover, burn or mux subtitles, export a Reel, or produce a visual/audio effect. Requires ffmpeg, ffprobe, and ffkit on PATH matching this skill's version field.
version: 0.8.0
compatibility: Requires ffmpeg, ffprobe, and the ffkit binary on PATH.
---

# ffkit

Hands: run `ffkit <verb>`. Flags live in `ffkit <verb> --help` (do not guess flags). Numbers come from `--json` or `--json-brief`, never from memory.

Shared flags on every writing verb: `--dry-run`, `--json`, `--json-brief`, `--overwrite`, `--timeout SECONDS`, `--progress`.

If `ffkit version --check` fails, or `ffkit --version` is not this file's `version`, run `ffkit install-skill` and reload this skill. After a missing-tool failure, run `ffkit doctor --json`.

## Workflow

1. **Probe** each input you will plan from (`ffkit probe INPUT --json`). Plan from those numbers.
2. Prefer **lossless**. `cut` and mux-only work stream-copy unless `--accurate` is required.
3. Route: a **verb** if the table names one; else **graph**.
4. `ffkit ffmpeg --because REASON -- …` only when REASON names the missing verb or graph field in one line. `--because` is required.
5. Picture changed (fit, overlay, caption, crop, colour, gif, cover, fade): `ffkit look OUTPUT`. Overlay and caption: `look --at T` at a time the graphic is on screen (repeat `--at` for in/mid/out). A tile sheet alone does not confirm a short overlay. Cover: inspect the PNG. Fade: `look --at 0` should be darker than mid-clip.
6. Never write onto the user's source. `--overwrite` only replaces an output this job created or the user named.

A write step is done when the process exits 0 and the output probe matches the request. A picture step is done when `Look:` names a real PNG you inspected (or `Look: PATH (pixels not inspected; agent has no image view)`). Overlay/caption is done when that PNG is from `--at`, not only `--tiles`.

## Request → verb

| User says | Do |
|-----------|----|
| what's in this file / how long | `ffkit probe FILE --json` |
| cut / trim this range | `ffkit cut IN --start T --end T -o OUT` |
| frame-exact cut | `ffkit cut IN --start T --end T --accurate -o OUT` |
| stitch these clips | `ffkit concat A B C -o OUT` |
| crossfade two clips | `ffkit concat A B --transition fade --duration 0.5 -o OUT` |
| make it 9:16 / square / 16:9 | `ffkit fit IN --aspect 9:16 --fit pad -o OUT` (or `--fit crop`) |
| resize to a width/height | `ffkit fit IN --width 1080 -o OUT` |
| rotate / flip | `ffkit fit IN --rotate 90 --flip h -o OUT` |
| logo / watermark / PiP | `ffkit overlay IN --image logo.png --position top-right -o OUT` then `look --at` a visible time |
| mux captions (toggleable) | `ffkit caption IN --srt subs.srt --mode mux -o OUT` |
| burn captions / mute viewing | `ffkit caption IN --srt subs.srt --mode burn -o OUT` then `look --at` a cue time (raster overlay; no libass) |
| extract audio / a frame / subs | `ffkit extract IN -o OUT.wav` (extension picks the stream; `--at T` for a still) |
| louder / match LUFS | `ffkit loudnorm IN -o OUT` (`-I -16` podcast, `-I -14` social) |
| web mp4 / webm / gif | `ffkit transcode IN --preset h264 -o OUT.mp4` (`webm`, `gif`) |
| make this a Reel / TikTok / Short / 9:16 social export | `ffkit deliver IN --platform reels -o OUT` (1080x1920, 30fps, −14 LUFS, h264+aac+faststart) |
| speed up / slow-mo / 1.5x | `ffkit speed IN --factor 1.5 -o OUT` (2 = twice as fast; pitch kept) |
| add BGM / 配乐 / duck music under speech | `ffkit music IN --track bed.mp3 -o OUT` (ducks the bed when voice is present) |
| cut silence / jump cuts / 剪掉停顿 | `ffkit jumpcut IN -o OUT` |
| cover / thumbnail / 封面 | `ffkit cover IN --at T -o cover.png` (1080x1920 still) |
| fade in / fade out / 淡入淡出 | `ffkit fade IN --in 0.3 --out 0.3 -o OUT` |
| show me the picture | `ffkit look OUT --tiles 3x2` and/or `--at T` (repeat `--at`) |
| every file in this folder | `ffkit batch DIR -o OUTDIR -- transcode --preset h264` |
| filter chain no verb covers | `ffkit graph plan.json` — see [references/graph.md](references/graph.md) |
| a flag no graph field covers | `ffkit ffmpeg --because "REASON" -- -i IN … OUT` |
| machine missing tools | `ffkit doctor --json` |
| skill/binary version | `ffkit version --json` (`--check` fails on stale installed copies) |

Ask one question only when it changes the file and probe cannot answer it (destination platform, caption source). Otherwise pick a default, say it, run.

This skill executes an edit. It does not decide which cut is interesting, whether a face should be cropped, or what looks cinematic.

## Report

Reply in the user's language; keep these labels in English. Numbers from `--json`.

```
Done: OUT — 59.98s 1080x1920 30fps h264 aac
Steps: probe -> cut 0:00-0:05 (lossless) -> fit 9:16 pad -> overlay logo -> look --at 1
Check: probe matches the request (verified: true)
Look: OUT_frame.png (logo top-right at 1s)
Notes: …
```

A raw ffmpeg step puts the `--because` text in `Notes:`. Failure: `Failed:` with the contract `error.message` quoted, not paraphrased. `Look: not needed` when the picture did not change.

## References

- [gotchas.md](references/gotchas.md) — VFR, even sizes, concat copy, GIF palette, libass / mux
- [graph.md](references/graph.md) — JSON filter graph when verbs are not enough
- [platforms.md](references/platforms.md) — Reels / Shorts / YouTube / GIF delivery
