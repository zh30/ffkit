---
name: ffkit
description: Help a user finish a local video or audio job. Chat about the outcome, propose a short plan, then run that plan with ffkit (pipeline of verbs, graph, or ffmpeg). Use when they mention a media file (mp4, mov, mkv, webm, wav, m4a, mp3, gif), footage, clip, Reel/Short/TikTok/YouTube, captions (mux or burn without libass), overlay, transcode, ffmpeg, rough cut, assembly, or an edit, export, or effect on files they have on disk. Requires ffmpeg, ffprobe, and ffkit on PATH matching this skill's version field.
version: 0.38.0
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
4. **Run the plan.** Two or more writing steps: pipeline JSON then `ffkit pipeline plan.json --json` — [pipeline.md](references/pipeline.md). `goal` is the restated outcome; `expect` is the numbers that make it true; `$src` is `input`, `$in` is the previous output. Adapt a recipe only after they agreed to that scheme — [recipes.md](references/recipes.md). One writing step: that verb. No matching verb: `graph`, then `ffmpeg --because`.
5. Prefer **lossless** (`cut` copy, mux captions, `loop` concat) unless pixels or samples must change.
6. Picture changed: `ffkit look`. Overlay/`caption`/`title`: `--at` a time the graphic is on. `broll`: `--at` the cutaway and a time after it. Fade: `--at 0` vs mid. Cover: inspect the PNG.
7. Never write onto the user's source.

Done when the **original task** is true (`expect` / probe match, and look if the picture changed)—not when the last process exited 0. If `verified` is false, the file exists; revise the plan.

Ask one question only when it changes the file and probe cannot answer it. Which cut is interesting, and what looks cinematic, stay with the user.

## Hands

`ffkit --help` is the full list. Match an agreed plan step to a tool. Compose several rows when the job needs several steps.

| Plan step | Hands |
|-----------|-------|
| inspect | `doctor`, `probe`, `look` (`--tiles` / `--at`) |
| trim / join | `cut`, `concat` (`--transition` any xfade, N clips), `split` (`--every` story chunks, `--at` chapter points), `rough` (list speech islands, then `-o` to assemble) |
| frame / size | `fit` (`--fit pad` / `crop` / `blur`), `zoom` |
| export | `deliver`, `transcode`, `compress` (`--size 10MB` two-pass), `audiogram`, `slideshow` (`--motion kenburns`, `--transition`), `split` |
| captions / mute | `caption` (`--mode burn` social safe-zone, `--chunk N` word groups, or `--mode mux`) |
| hook text | `title` |
| cover still | `cover` |
| speech / music | `jumpcut`, `denoise`, `music`, `replace` (`--audio` swap the track, `--mix` keep the original under it), `loudnorm`, `volume` |
| motion / loop | `speed`, `reverse`, `loop`, `stabilize`, `fade` |
| picture | `grade` (+ `--lut` .cube), `bw`, `vignette`, `sharpen`, `blur` |
| logo / PiP | `overlay` |
| green screen | `key` (`--bg`, `--color`/`--similarity`/`--blend`, `--despill` for fringe) |
| reaction / multi-cam grid | `grid` (`--layout 2x2`, `--size`) |
| watch-time progress bar | `progress` (`--color`, `--height`, `--edge`) |
| freeze a beat / outro hold | `freeze` (`--at T --dur D`, or `--end D`) |
| blur a face / logo | `censor` (`--region x:y:w:h`, `--mode pixel|blur`) |
| slow-mo punch-in | `speed` (`--factor`, `--at`/`--dur` for just one window) |
| boomerang replay | `boomerang` (forward then reversed, one loop) |
| YouTube/player chapters | `chapter` (`--at T|TITLE`, repeatable; lossless) |
| punch-zoom a moment | `zoom` (`--factor`, `--at`/`--dur`) |
| strip letterbox/pillarbox | `autocrop` (cropdetect scan → crop) |
| contact sheet / preview grid | `sheet` (`--cols`/`--rows`/`--tile` → PNG) |
| title card mid-clip | `title` (`--text`, `--at` S for lower-third timing) |
| voice-over on video's own audio | `replace --audio V --mix G --duck` (sidechain) |
| pitch-shift voice/music | `pitch` (`--semitones N`, duration preserved) |
| film grain | `grade --grain N` |
| auto cut on scene changes | `split --scenes 0.3` |
| strip dead air head+tail (audio) | `cutsil` (`--thresh -45`) |
| grid with one input's audio | `grid --audio N` |
| draft/tiled watermark | `overlay --tile N` (diagonal watermark pass) |
| shift subtitle timing | `caption --shift SEC` |
| gif tuning | `transcode --preset gif --fps --width` |
| one-ear voice fix | `channel` (`--mode dualmono`/`mono`/`swap`) |
| loop to a length | `loop --until SEC` |
| text draft watermark | `title --tile N` |
| B-roll cutaway | `broll` (`--insert --at --duration`; A-roll audio stays) |
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
