---
name: ffkit
description: Help a user finish a local video or audio job. Chat about the outcome, propose a short plan, then run that plan with ffkit (pipeline of verbs, graph, or ffmpeg). Use when they mention a media file (mp4, mov, mkv, webm, wav, m4a, mp3, gif), footage, clip, Reel/Short/TikTok/YouTube, captions (mux or burn without libass), overlay, transcode, ffmpeg, rough cut, assembly, or an edit, export, or effect on files they have on disk. Requires ffmpeg, ffprobe, and ffkit on PATH matching this skill's version field.

version: 0.106.0
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

| trim / join | `cut`, `concat` (`--transition` any xfade, N clips, `--level -14` loudnorms each), `split` (`--every` story chunks, `--at` chapter points), `rough` (list speech islands, then `-o` to assemble, `--merge N` merge close keeps) |
| platform loudness | `loudnorm` (`--target`, `-I/--tp/--lra`; `--measure` report-only) |
| frame / size | `fit` (`--fit pad` / `crop` / `blur`), `zoom`, `--position` top/bottom/corners |

| export | `deliver`, `transcode` (`--copy-audio`), `compress` (`--size 10MB` two-pass), `audiogram` (`--progress`, `--mode`/`--color`), `slideshow` (`--motion kenburns`, `--transition`), `split`, `--subs` captions, `--preset prores`, `--target` platform sizes |
| captions / mute | `caption` (`--karaoke` word reveal, `--box-color` card, `--mode burn` social safe-zone, `--chunk N` word groups, or `--mode mux`), `--fade` |
| hook text | `title` |
| cover still | `cover` |

| speech / music | `jumpcut`, `denoise`, `music`, `replace` (`--loop` short beds, `--audio` swap the track, `--mix` keep the original under it), `loudnorm` (`--target` platform preset), `volume` |
| grainy low-light footage | `vdenoise` (`--strength`, nlmeans — slow on long clips) |
| waveform PNG of audio | `waveform` (`--size`, `--color`, `--scale`, `--at/--dur`) — podcast art, thumbnails |
| audio spectrogram PNG | `spectrogram` (`--size`, `--color`, `--at/--dur`) — inspect hum/noise before cleanup |
| mains hum / electrical buzz | `dehum` (`--at`/`--dur` window, `--mains 50|60`, `--harmonics`) — notches the fundamental + harmonics |
| faster/slower podcast | `tempo` (`--factor 1.5` — pitch held; video inputs: use `speed`), `--at/--dur` window |
| voice all over the place | `leveler` (`--at`/`--dur` window, `--threshold`/`--ratio`/`--makeup` — `acompressor`) |
| hiss between sentences | `gate` (`--threshold`, `--preset`, `--at/--dur` — `agate` closes on quiet parts) |
| pad in room tone / breath | `silence` (`--at`, `--dur` — inserts quiet into audio files; video holds: `freeze`) |
| one-click look | `grade --preset cinematic|vivid|vintage|soft` (stacks under the sliders) |
| karaoke / keep only the vocal | `vocal` (`--at`/`--dur` window, `--mode karaoke` drops the center, `isolate` keeps it — stereo only; `--amount` partial) |
| container swap, no re-encode | `remux` (mkv→mp4 etc., `-c copy` + faststart) |
| top/bottom caption meme | `meme` (`--top`/`--bottom` text, `--color`, `--size`, `--outline`, `--at/--dur` window), `--position` center/bottom |
| fix my podcast voice | `voice` — one-shot chain: gate hiss → compress swings → loudnorm `--lufs` (default −16) |
| old interlaced footage | `deinterlace` (`--mode field` doubles the rate, `frame` same rate, `--parity` field order, `--engine` yadif/bwdif) |
| fade to white | `fade --color white` (`--in`/`--out` seconds as usual) |
| blend two audio files | `crossfade` (`--second`, `--dur` overlap — acrossfade) |
| strip location/device tags | `strip` — drops all container metadata + chapters, stream copy |
| stills every N seconds | `frames` (`--every`, `--width`) → `stem_001.png…` |
| 3-2-1 intro countdown | `countdown` (`--from`, `--each`, `--go`, `--at`, `--text`, `--position`) |
| invert / negative look | `invert` — `negate` the picture |
| split to fit a size cap | `split --size 9MB` — even grid aimed at Discord/WhatsApp caps |
| merge two audio sources at full level | `mix` `A B` (`--vol-a/--vol-b`, `--longest`, `--at/--dur`), `--duck` bed dips under voice |
| captions on top instead of bottom | `caption --position top` |
| lift/crush mid-tones | `grade --gamma` |
| drop the audio track entirely | `mute` (stream-copy video, no re-encode), `--at/--dur` window |
| elapsed-time corner counter | `timer` (`--box-color` card, `--position`, `--at`, `--dur`, `--size`, `--color`, `--format ms` centiseconds), `--down` countdown |
| web-embed HLS package | `hls` (`--seg` seconds, `--single` one-file, `--copy` repack) → dir/`index.m3u8` + `seg_*.ts`; `--ladder 1080,720,480` → ABR variant playlists + `master.m3u8`; `--audio-only` podcast HLS |
| check encode quality loss | `qa` `ref.mp4 test.mp4` → psnr/ssim numbers |
| normalize mixed footage for concat | `conform` (`--size WxH`, `--fps`, `--lufs`) |
| light-leak / screen-blend overlay | `overlay --video leak.mp4 --mode screen` |
| fix audio/video sync drift | `sync` (`--ms ±N` — pad or trim audio start) |
| sync a second take to the camera master | `align` (`ref target -o out` — auto-detects offset by audio cross-correlation; multi-cam, external recorder) |
| rolling end credits | `scroll` (`--text`/`--file`, `--at`, `--dur` — text rolls bottom→top) |
| splice a clip into the middle | `insert` (`--clip x.mp4 --at T` — b-roll/ad read without manual split+concat; `--dur N` first N sec only), `--transition` xfade both joints |
| two-camera angle switching | `multicam` (`A B --at t1,t2,...` — run `align` first if the takes aren't synced) |
| reframe 9:16 keeping faces | `crop` (`--aspect 9:16 --anchor top` keeps the face) |
| attach album cover art | `art` (`--image cover.png`) → mp3/m4a/mp4/mkv, `--extract` pull cover out |
| grab a cover/thumbnail frame | `thumb` (`--at` / `--frame`) → jpg/png |
| burn an .srt/.ass into pixels | `subs` (`--burn subs.srt` — libass), `--rate` drift fix, `--safe` social zone; `--convert` srt↔vtt |
| split into exactly N parts | `split` (`--parts N` — equal-length grid) |
| title that fades in/out | `title` (`--fade` secs — soft entry/exit, `--box` card) |
| fix white balance / color cast | `grade` (`--hue` deg — rotates the hue) |
| quiet tail on a podcast | `silence` (`--end --dur` secs — appended) |
| logo/watermark that eases in | `overlay` (`--fade` secs — alpha in/out) |
| subtitle file is early/late | `subs` (`--shift ±N` — retimes every cue) |
| full podcast/music tags | `meta` (`--album`/`--genre`/`--date`/`--track`) |
| keep only the good parts | `cut` (`--ranges "10-20,40-50"` — joined) |
| solid color card / backplate | `solid` (`--color`/`--size`/`--dur`, optional silent track), `--text` card text |
| boost without clipping | `volume` (`--limit` dBTP — brickwall after the gain) |
| rip out a middle section | `cut` (`--drop "30-45"` — keeps the rest joined) |
| text that survives busy frames | `title` (`--outline` — stroke around every glyph) |
| bars in brand color | `fit` (`--color` — pad fill instead of black) |
| burned subs, your style | `subs` (`--size`/`--color`/`--top`/`--outline`/`--font`) |
| audiogram in brand colors | `audiogram` (`--bg` backdrop) |
| subtle watermark | `overlay` (`--opacity` on `--image`) |
| split a podcast on pauses | `split` (`--silence=-35` — cuts at gap midpoints) |
| music bed that eases in/out | `music` (`--fade` on the bed) |
| one-word EQ curve | `eq` (`--preset voice/podcast/bright/bass`), `--band` parametric, `--tilt` |
| soft b-roll cutaway edges | `broll` (`--fade`), `--position` pip |
| stills at exact moments | `frames` (`--at 12,45,90`) |
| audiogram on any canvas | `audiogram` (`--size` — 1080x1920, 1920x1080, 1080x1080) |
| swapped audio eases in/out | `replace` (`--fade`) |
| audiogram with a title | `audiogram` (`--text "EP 12"` near the top) |
| thumbnail at a given size | `thumb` (`--width`) |
| inverted flash/accent | `invert` (`--at`/`--dur`) |
| blur just a moment | `blur` (`--at`/`--dur`) |
| text in a corner | `title` (`--position top-right` …) |
| B&W only for a moment | `bw` (`--at`/`--dur`) |
| sharpen only the key shot | `sharpen` (`--at`/`--dur`) |
| wipe metadata before posting | `meta` (`--clear`) |
| dark-edge only for a beat | `vignette` (`--at`/`--dur`) |
| grade only the dream sequence | `grade` (`--at`/`--dur`) |
| hear the b-roll under me | `broll` (`--audio`) |
| captions in my brand font | `subs` (`--burn --font`) |
| progress bar only in the back half | `progress` (`--at`/`--dur`) |
| waveform band at the top | `audiogram` (`--position`) |
| pull OUT of a shot (reveal) | `zoom` (`--out`) |
| title with a soft shadow | `title` (`--shadow`) |
| still at an exact width | `extract` (`--gif` clip, `--width`), `--loop` gif repeats |
| countdown with tick beeps | `countdown` (`--beep`, `--text` label during the count) |
| one-word compressor curve | `leveler` (`--preset`) |
| spectrogram in brand colors | `spectrogram` (`--color`) |
| music kicks in after the intro | `music` (`--at`/`--dur`) |
| fix an out-of-phase mic | `channel` (`--mode invert --side`) |
| contact-sheet breathing room | `sheet` (`--pad`/`--margin`) |
| diagonal watermark | `overlay` (`--angle`) |
| waveform showing quiet detail | `waveform` (`--scale log`) |
| split on longer pauses | `split` (`--silence --min-silence`) |
| wobble/sci-fi/echo/lofi/telephone voice | `fx` (`--kind` 8 effects) |
| effect only in the drop | `fx` (`--at`/`--dur`) |
| boomerang that loops 3x | `boomerang` (`--times`), `--at/--dur` window |
| H.265 for Apple / smaller archive | `transcode` (`--preset hevc`) |
| gate tuned for speech vs studio | `gate` (`--preset voice|podcast|studio`) |
| echo/reverb only on the hook | `reverb` (`--at`/`--dur`) |
| bass boost only on the drop | `eq` (`--at`/`--dur`) |
| repeat just the funny bit | `loop` (`--from`/`--to`/`--times`) |
| selectable soft subs in mp4 | `subs` (`--mux file.srt --lang spa`) |
| stroked TikTok captions | `caption` (`--outline RRGGBB`) |
| branded audiogram title font | `audiogram` (`--font`) |
| name each tile in a grid | `grid` (`--labels "a,b"`), `--fill` crop-fill cells |
| gentle logo cleanup | `delogo` (`--soft`) |
| animated gradient card | `solid` (`--gradient ff0000:0000ff`) |
| reframe / crop out an edge | `crop` (`--region x:y:w:h` or `--aspect 1:1`/`9:16` centered) |
| motion / loop | `speed`, `reverse`, `loop`, `stabilize` (`--edge` fill), `fade` |
| picture | `grade` (+ `--lut` .cube), `bw`, `vignette`, `sharpen`, `blur` |
| logo / PiP | `overlay` |
| green screen | `key` (`--bg`, `--color`/`--similarity`/`--blend`, `--despill` for fringe) |
| reaction / multi-cam grid | `grid` (`--layout 2x2`, `--size`) |
| watch-time progress bar | `progress` (`--color`, `--height`, `--edge`) |
| freeze a beat / outro hold | `freeze` (`--ease`/`--reverse` swoop, `--at T --dur D`, or `--end D`) |
| blur a face / logo | `censor` (`--strength`, `--region x:y:w:h`, `--mode pixel|blur`; `--at`/`--dur` limits the window) |
| slow-mo punch-in | `speed` (`--factor`/`--ramp`, `--at`/`--dur` for just one window) |
| boomerang replay | `boomerang` (forward then reversed, one loop) |
| YouTube/player chapters | `chapter` (`--at T|TITLE` repeatable, `--auto` silence gaps; lossless) |
| punch-zoom a moment | `zoom` (`--factor`, `--at`/`--dur`) |
| strip letterbox/pillarbox | `autocrop` (cropdetect scan → crop, `--buffer N` keeps N px edge) |
| contact sheet / preview grid | `sheet` (`--cols`/`--rows`/`--tile` → PNG, `--time` stamps) |
| title card mid-clip | `title` (`--text`, `--at` S for lower-third timing) |
| voice-over on video's own audio | `replace --audio V --mix G --duck` (sidechain) |
| pitch-shift voice/music | `pitch` (`--at`/`--dur` window, `--semitones N`, duration preserved) |
| film grain | `grade --grain N` |
| auto cut on scene changes | `split --scenes 0.3` |
| strip dead air head+tail (audio) | `cutsil` (`--thresh -45`) |
| grid with one input's audio | `grid --audio N` |
| draft/tiled watermark | `overlay --tile N` (diagonal watermark pass) |
| shift subtitle timing | `caption --shift SEC` |
| gif tuning | `transcode --preset gif --fps --width`, `extract --gif --bounce` (palindrome loop) |
| one-ear voice fix | `channel` (`--mode dualmono`/`mono`/`swap`) |
| loop to a length | `loop --until SEC` |
| text draft watermark | `title --tile N` |
| audio EQ polish | `eq` (`--bass`/`--treble`/`--presence` dB) |
| animated push-in | `zoom --motion kenburns` |
| still-image cutaway | `broll --insert img.png --still` |
| wrong-orientation phone clip | `rotate` (`--deg`/`--flip`) |
| burned-in logo/watermark | `delogo` (`--x --y --w --h`; `--at`/`--dur` only some of the time) |
| smooth slow-mo | `speed --factor 0.5 --interp` |
| styled title text | `title --size 2 --color ff0000` |
| lower-third placement | `title --position bottom` (or `top`/`center`) |
| container metadata tags | `meta` (`--title`/`--artist`/`--comment`) |
| fix display rotation flag | `meta --rotate 90` (lossless; clears with `--rotate 0`) |
| room tone on a voice | `reverb` (`--size room|hall|cave`, `--wet 0..0.9`) |
| wobble/sci-fi/echo/lofi/telephone audio | `fx` (`--kind`, `--strength`, `--at`/`--dur`) |
| Ken Burns on a photo cutaway | `broll --insert img.png --still --motion kenburns` |
| styled captions | `caption --color ff0000 --size 1.5` |
| logo only for part of the clip | `overlay --at 2 --dur 5` |
| rip embedded subtitles | `subs` (`--stream N`) |
| bleep out a word | `bleep` (`--at`/`--dur`; `--freq`/`--level`) |
| warm/cool white balance | `grade --warm -1..1` |
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