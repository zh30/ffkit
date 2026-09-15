# Recipes

Copy a recipe into a pipeline JSON **only after the user agreed to that scheme**. Change `input`, times, text, and `expect` from probe. These are schemes, not a verb menu.

## Mute-friendly Reel

User: 做成 Reel，静音也能看，开头有句 hook.

```json
{
  "goal": "做成 Reel，静音也能看，开头有 hook",
  "input": "talk.mp4",
  "expect": { "aspect": "9:16", "width": 1080, "height": 1920, "has_audio": true, "ext": "mp4" },
  "steps": [
    { "tool": "jumpcut", "label": "剪掉停顿", "argv": ["$src", "-o", "01.mp4"] },
    { "tool": "title", "label": "片头字", "argv": ["$in", "--text", "WAIT", "--duration", "0.8", "-o", "02.mp4"] },
    { "tool": "caption", "label": "烧字幕", "argv": ["$in", "--srt", "talk.srt", "--mode", "burn", "-o", "03.mp4"] },
    { "tool": "deliver", "label": "导出 Reel", "argv": ["$in", "--platform", "reels", "-o", "reel.mp4"] }
  ]
}
```

Then `ffkit look reel.mp4 --at 1` (and `--at` a caption time).

## Trim to 9:16

User: 剪前 5 秒，做成竖屏.

```json
{
  "goal": "前 5 秒的 9:16 成片",
  "input": "clip.mp4",
  "expect": { "aspect": "9:16", "duration_lt": 5.3, "has_video": true, "ext": "mp4" },
  "steps": [
    { "tool": "cut", "label": "剪前 5 秒", "argv": ["$src", "--start", "0", "--end", "5", "-o", "01.mp4"] },
    { "tool": "fit", "label": "竖屏", "argv": ["$in", "--aspect", "9:16", "--fit", "pad", "-o", "reel.mp4"] }
  ]
}
```

Need Reels loudness / 1080×1920 pack: last step `deliver` instead of `fit`.

## Podcast clip

User: 抽出声音，对齐到播客响度.

```json
{
  "goal": "wav，响度约 −16 LUFS",
  "input": "talk.mp4",
  "expect": { "has_audio": true, "has_video": false, "ext": "wav" },
  "steps": [
    { "tool": "extract", "label": "抽音频", "argv": ["$src", "-o", "01.wav"] },
    { "tool": "loudnorm", "label": "−16 LUFS", "argv": ["$in", "-I", "-16", "-o", "talk.wav"] }
  ]
}
```

`loudnorm` probe does not include LUFS; say the `-I` you used in Notes.

## Cover still

User: 给这段做张 9:16 封面.

```json
{
  "goal": "9:16 封面图",
  "input": "talk.mp4",
  "expect": { "aspect": "9:16", "width": 1080, "height": 1920, "ext": "png" },
  "steps": [
    { "tool": "cover", "label": "封面", "argv": ["$src", "--at", "1", "-o", "cover.png"] }
  ]
}
```

Inspect the PNG (it is the output; no extra `look` unless they asked to compare times).

## B-roll under speech

User: 口播切一段 B-roll，声音继续.

```json
{
  "goal": "0.5s 切走 B-roll，口播声音不断",
  "input": "talk.mp4",
  "expect": { "has_audio": true, "has_video": true, "ext": "mp4" },
  "steps": [
    { "tool": "broll", "label": "切 B-roll", "argv": ["$src", "--insert", "broll.mp4", "--at", "1", "--duration", "0.5", "-o", "with-broll.mp4"] }
  ]
}
```

Do not write `-o` onto the user's source; pick a new path. Then `ffkit look` at `--at` and after the window.

## Rough cut a long take

User: 刚录完，帮我粗剪。

First `ffkit rough take.mp4 --json` (no `-o`) and quote `extra.keeps` / `speech_seconds`. Then assemble:

```json
{
  "goal": "去掉长停顿，拼成一条能看的粗剪",
  "input": "take.mp4",
  "expect": { "has_audio": true, "has_video": true, "ext": "mp4" },
  "steps": [
    { "tool": "rough", "label": "粗剪", "argv": ["$src", "--min-duration", "0.5", "-o", "rough.mp4"] }
  ]
}
```

`jumpcut` is the wrong hand for a 20-minute dump (it re-encodes the whole timeline). `rough --copy` is faster and keyframe-sloppy.
