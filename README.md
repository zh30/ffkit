# ffkit

[English](README.md) · [中文](README.zh.md)

FFmpeg hands for a local AI agent: the user talks about the finished piece, the agent proposes a scheme, then `ffkit` runs it. System `ffmpeg` / `ffprobe` are the engine. Media stays on disk.

The skill is not a menu of looks (blur / vignette / black-and-white). Multi-step jobs are a `pipeline` JSON, run once.

## Dependencies

- Rust 1.80+ (only if you install from source)
- `ffmpeg` and `ffprobe` on `PATH`  
  macOS: `brew install ffmpeg`

Capability equals **this ffmpeg's compile flags**. `ffkit doctor --json` lists the encoders / filters you actually have. `caption --mode burn` rasterizes overlays and does not need libass.

## Install

From [Releases](https://github.com/zh30/ffkit/releases/latest) download the **platform zip** (the asset that contains the `ffkit` binary — not Source code). Unzip:

```bash
./ffkit doctor --json          # runnable as-is
./install.sh                   # copy onto PATH and write the Agent skill
ffkit version --check
```

| System | Asset |
|--------|--------|
| macOS Apple Silicon | `ffkit-*-aarch64-apple-darwin.zip` |
| macOS Intel | `ffkit-*-x86_64-apple-darwin.zip` |
| Linux x86_64 | `ffkit-*-x86_64-unknown-linux-gnu.zip` |

If macOS blocks a browser download: `xattr -d com.apple.quarantine ffkit` then `./install.sh`.

From source (Rust 1.80+):

```bash
git clone https://github.com/zh30/ffkit && cd ffkit
cargo install --path .
ffkit install-skill
ffkit doctor --json
```

`install-skill` writes `SKILL.md` and `references/` into host skill dirs that already exist:

| Host | Path |
|------|------|
| Grok | `~/.grok/skills/ffkit` |
| Claude Code | `~/.claude/skills/ffkit` |
| Codex | `~/.codex/skills/ffkit` |
| Cursor | `~/.cursor/skills/ffkit` |

Skill and CLI share **one SemVer** (`Cargo.toml` and `SKILL.md` `version:`, checked at compile time). Inspect and verify:

```bash
ffkit version --json
ffkit version --check    # fails if an installed skill copy does not match this binary
```

`install-skill` writes a `VERSION` stamp in each skill dir. If `ffkit version --check` fails, run `ffkit install-skill` and reload the agent. Changes are recorded in [`CHANGELOG.md`](CHANGELOG.md).

Every merge to `main` publishes a GitHub Release (`vX.Y.Z`) whose assets are unzip-and-run zips. To ship a new version:

```bash
# In the PR: change code/docs → CHANGELOG [Unreleased] → bump
./scripts/bump-version.sh patch   # or minor / major / 0.2.0
cargo test
# After merge, Actions packs and creates the Release. Local fallback:
./scripts/pack-release.sh
./scripts/publish-release.sh
```

## For the agent

Once installed, speak naturally, for example:

- Cut the first 5 seconds of this mp4 and make it 9:16
- Extract wav and loudnorm to -16 LUFS
- Put logo.png in the top-right, then make a GIF

The agent should load the **ffkit** skill (`/ffkit` or auto-trigger): talk through the outcome, propose a scheme, run `ffkit pipeline` (or one verb), then report `--json` numbers against the **original goal**. The full loop is in [`SKILL.md`](SKILL.md).

## CLI

Flags: `ffkit <verb> --help`. Writing verbs share `--dry-run`, `--json`, `--json-brief`, `--overwrite`, `--timeout`, `--progress`. Trust numbers only from `--json`. Sources are never overwritten by default.

```bash
ffkit probe clip.mp4 --json
ffkit pipeline plan.json --json    # multi-step scheme ($src / $in / expect)
ffkit look branded.mp4 --at 1 -o frame.png
```

### Verbs

| Verb | What it does |
|------|----------------|
| `doctor` | Whether local ffmpeg works, which encoders/filters exist |
| `probe` | Duration, size, codecs, channels |
| `look` | Contact sheet (`--tiles`) or timestamps (`--at`, repeatable) |
| `cut` | Trim; lossless copy by default, `--accurate`, `--ranges`, `--drop` for frame-exact (bounds take `end`: `T-end` through the tail, `end-N` last N secs); `--fade N` softens the cut edges |
| `concat` | Join N clips (any xfade `--transition`, comma list picks one per joint); `--level -14` loudnorms each clip first; `--gap N` inserts black+silence between clips; `--audio-fade N` fades each joint's audio (boundary fades — duration and sync preserved) |
| `fit` | Frame / rotate / flip (9:16, 1:1, 16:9, …); `--fit blur` fills with a blurred backdrop , `--position` anchors the picture in the bars , `--strength` sigma |
| `extract` | Still frame or `--gif` clip (`--bounce` palindrome) | `--at` (`end` = last frame / last --dur sec, comma = one still — or GIF with `--gif` — per time), `--width`, `--fps` , `--loop` GIF repeat count, `--colors` palette size |
| `overlay` | Image/video overlay; position/scale/`--tile`, `--fade`, `--angle`, `--at`/`--dur`, `--mode`, `--opacity` blend composite, `--loop` repeat short clips | Logo/picture-in-picture; `--tile N` draft watermark | Logo, watermark, picture-in-picture; `--border` PiP ring |
| `broll` | Cutaway insert (`--insert` video, `--still` image, `--motion kenburns`) | Full-frame cutaway (`--insert --at --duration`); A-roll audio and length stay , `--audio` hear the insert (`--volume` its level) , `--position` PiP corner + `--scale`, `--border` ring the insert, `--opacity` ghost insert; `--at end` = tail cutaway, comma `--at` re-flashes it at several points |
| `caption` | Burn subtitles (`--srt`, `--chunk`, `--karaoke`, `--box-color` card, `--wrap` folds, `--from/--to` cue window, `end`/`end-N` ok) , `--fade` soft in/out, `--opacity` ghost captions |
| `broll` | Cutaway insert (`--insert` video, `--still` image, `--motion kenburns`) | Full-frame cutaway (`--insert --at --duration`); A-roll audio and length stay , `--audio` hear the insert (`--volume` its level) , `--position` PiP corner + `--scale`, `--border` ring the insert; `--at end` = tail cutaway, comma `--at` re-flashes it at several points |
| `caption` | Burn subtitles (`--srt`, `--chunk`, `--karaoke`, `--box-color` card, `--wrap` folds, `--from/--to` cue window, `end`/`end-N` ok) , `--fade` soft in/out, `--opacity` ghost captions |
| `broll` | Cutaway insert (`--insert` video, `--still` image, `--motion kenburns`) | Full-frame cutaway (`--insert --at --duration`); A-roll audio and length stay , `--audio` hear the insert (`--volume` its level) , `--position` PiP corner + `--scale`, `--border` ring the insert; `--at end` = tail cutaway, comma `--at` re-flashes it at several points |
| `caption` | Burn subtitles (`--srt`, `--chunk`, `--karaoke` (`--highlight` sung color), `--box-color` card, `--wrap` folds, `--from/--to` cue window, `end`/`end-N` ok) , `--fade` soft in/out, `--opacity` ghost captions |
| `loudnorm` | EBU R128 two-pass normalization (`--target spotify|podcast|broadcast`, `-I/--tp/--lra`); `--measure` reports loudness without writing (`--gate N` fails over N LUFS); `--dynamic` per-frame gain |
| `denoise` | Audio cleanup (`--strength`, `--highpass`, `--at/--dur` window); `end` ok, comma list = several windows |
| `transcode` | h264/webm/`--preset gif` (`--fps`/`--width`/`--copy-audio`/`--colors`) | Presets `h264` / `webm` / `gif` / `hevc` / `mp3` / `aac` / `wav` / `flac` / `opus` / `av1` / `prores`; `--fps` retimes video too; `--vbitrate` peak bitrate cap, `--abitrate` audio bitrate (voice → 64k) , `--preset prores` FCP delivery; `av1` preset; `--alpha` keeps transparency (webm/prores) |
| `compress` | Fit a size budget (`--size 10MB` two-pass, `--target discord|whatsapp|gmail`); `--crf` quality one-pass, `--res` downscale to free bitrate |
| `deliver` | One-shot platform pack (Reels / TikTok / Shorts 9:16, `square` 1:1 grid, `youtube` 16:9; −14 LUFS; `--fps 60` high-frame-rate uploads, `--crf` quality, `--subs file.srt` burns captions in one pass) |

| `audiogram` | Waveform video | `--mode`, `--scale`, `--split` channels, `--fscale` freq axis (spectrum), `--fps` rate, `--text`, `--bg`, `--progress` bar , `--subs` burn an .srt on it, `--from`/`--to` clip a segment (`end`/`end-N` ok), `--at a,b --dur N` one clip per point (`stem_N.mp4`); `--mode spectrum` bars, `--mode scope` lissajous vectorscope |

| `split` | Split by `--every`/`--at`/`--scenes`/`--size`/`--parts`/`--silence`/`--chapters`; `--subs` writes re-timed per-part .srt; `--fade N` softens each part's edges |
| `slideshow` | Still images → video montage (`--per` or `--dur` total runtime, `--fade`, `--transition`, `--motion kenburns`, `--audio` bed + `--volume`, `--size` canvas, `--bg` letterbox) |
| `speed` | Change playback speed (`--factor`, `--at/--dur` (comma list = several windows), `--ramp` FROM,TO); `end` ok |
| `music` | Bed under speech with ducking (`--track`, `--at`/`--dur` window, `end` ok; comma `--at` = multi-entrance bed) |
| `key` | Green-screen composite: `--color` keyed out over `--bg` image/video (`--similarity`, `--blend`, `--despill`, `--at`/`--dur` window — comma list ok) |
| `grid` | Multi-up collage (`--layout`, `--audio` pick, `--labels`, `--gap`, `--bg` gutter color, `--fill` crop instead of letterbox, `--time` mm:ss stamp on every tile) |
| `progress` | Progress bar on any edge, whole clip or a window (`--color`, `--height`, `--edge` bottom/top/left/right, `--at`, `--dur`, `--reverse` countdown-deplete, `--opacity` ghost) |
| `freeze` | Hold a frame (`--at`, comma list freezes at several points, `--dur`, `--end`, `--ease`, `--reverse`, `--zoom` push-in) |
| `censor` | Blur/mosaic a region (`--region x:y:w:h`, comma list for several spots; `--mode` pixel|blur|solid (black-bar redact), `--strength`, `--at`/`--dur` (comma list, needs `--dur`; `end` ok), `--shape circle` ellipse mask) |
| `bleep` | Tone over a word/segment: `--at`/`--dur` (comma list censors several spots; `end` ok), `--freq`, `--level` |
| `boomerang` | Forward + reversed replay (one loop, social trick) (`--times` repeat cycles) , `--at/--dur` bounces just that window — comma list for several spots (`end` ok) |
| `chapter` | Embed chapter marks at `TIME|TITLE` or `--import` a marks file (YouTube `H:MM:SS Title` lines ok); `--auto` / `--export` ffmeta / `--yt` description lines; `--list`; `--remove`; `--shift` re-times marks |
| `autocrop` | Detect & strip letterbox/pillarbox (`cropdetect` scan → `crop`; `--buffer N` keeps N px edge context) |
| `sheet` | Contact sheet grid (`--cols`x`--rows`, `--time` stamps, `--title` header, `--from`/`--to` window) |
| `sprite` | Seek-preview sprite sheets + WebVTT (`--every` secs, `--width` tile px, `--cols`x`--rows` per sheet, `--vtt` path, `--from`/`--to` bounds -- `end` ok) — hover thumbnails for video players |
| `pitch` | Shift pitch ±12 semitones, duration kept (`--at/--dur` window, comma list); `--formant` keeps the voice timbre (librubberband) |
| `cutsil` | Strip dead air at head+tail of an audio file (`--thresh` dB) |
| `channel` | Channel surgery: `--mode dualmono|mono|swap|invert|mix51|pan|widen|split` (stereo→`_L/_R.wav` stems); `--pan -1..1` pan |
| `eq` | Audio shelving EQ: `--bass`/`--treble`/`--presence`, `--preset` dB (`--at`/`--dur` window) , `--band` parametric F:G[:W], `--tilt` warm↔bright; `end` ok, comma list = several windows |
| `reverb` | Room ambience on a voice: `--size room\|hall\|cave`, `--wet` (`--at`/`--dur` window); `end` ok, comma list = several windows |
| `fx` | Audio FX rack: tremolo/vibrato/flanger/phaser/chorus/echo/lofi/radio (`--kind`, `--strength`, `--at`/`--dur`); `end` ok, comma list = several windows |
| `rotate` | 90/180/270 or mirror: `--deg`/`--flip`, free `--angle` tilt, `--at`/`--dur` windowed tilt (comma list) |
| `delogo` | Blend out a burned-in logo box: `--x --y --w --h` or `--regions x:y:w:h,...` for several spots; `--at`/`--dur` for a window, `--at end` the tail (`--soft` removelogo, `--shape circle` elliptical mask) |
| `meta` | Container tags (`--title`/`--artist`/`--album`/`--genre`/`--date`/`--track`/`--comment`) + `--rotate`, `--clear`, stream-copy |
| `subs` | Extract (`--stream`, `--all`)/burn/mux subtitles (`--shift` (±N; `--from`/`--to` bounds it)/`--merge`/`--rate`, burn style + `--outline`/`--box` plate/`--align`/`--margin` px/`--from`/`--to` window (`end`/`end-N` ok), `--safe`); `--convert` .srt↔.vtt; `--case` cue text; `--burn-si N` burns embedded track N; `--encoding gbk` legacy charsets |
| `thumb` | One-frame cover grab (`--at` — `end` = last frame, comma `--at` = one still per time / `--frame`, `--count` N stills (`--from`/`--to` bounds the spread, `end`/`end-N` ok), `--width`) → jpg/png/webp; `--scenes` stills at cuts |
| `solid` | Solid-color clip card (`--color`, `--size`, `--dur`, `--fps` rate; silent stereo optional) (`--gradient` animated, `--noise` grain) , `--text` end-card text (`--wrap` folds, `--align` lines) |
| `replace` | Swap the audio track (`--mix`, `--duck`, `--fade`, `--loop` short beds, `--at`/`--dur` window — comma `--at` several) |
| `jumpcut` | Cut silence inside a talking-head take |
| `rough` | Map speech islands in a long take (`--json`); `-o` assembles (`--merge N` merges keeps closer than N s, `--by-scene` splits keeps at scene changes) (default encodes only keeps; `--copy` is lossless/keyframe-sloppy) |
| `cover` | Cover still (`--at` — `end` = last frame, comma = one per beat, `--blur` ambient pad, `--size` canvas — default 1080x1920) |
| `fade` | Video and audio fade (`--in` / `--out`, `--color` e.g. white, `--dip T` scene-change dip — comma list dips at every mark; `--curve` audio fade shape) |
| `title` | Hook/title card (`--at` — `end` = end-card, comma list flashes at several marks, `--fade`, `--outline`, `--box` backplate, `--wrap` long hooks, `--align` lines, `--opacity` ghost, `--margin` px corner inset) |
| `loop` | `--times` or `--until` seconds | Repeat the clip N times (Shorts replay length) (`--from`/`--to` loops only a section, `end` ok, `--fade` seamless joints) |
| `stabilize` | Handheld deshake — `--rx`/`--ry` radius, `--edge` fill (blank|original|clamped|mirror) |
| `reverse` | Play picture and sound backwards |
| `grade` | `--preset` look, `--contrast/--saturation/--brightness/--gamma/--hue/--lut/--grain/--warm/--exposure` (EV stops) | Presets `cinematic`/`vivid`/`vintage`/`soft`/`sepia`/`teal`/`noir`/`bleach`/`neon` stack under the sliders; `--lut look.cube` applies a 3D LUT | `--at`/`--dur`
| `zoom` | Punch-in (`--factor 1.25`, `--center X,Y` target; `--at`/`--dur` window — comma list for several, `end` ok) | `--out`
| `sharpen` | Unsharp mask, whole clip or a window (`--amount`, `--at`, `--dur`) |
| `vignette` | Corner darkening, whole clip or a window (`--angle`, `--at`, `--dur`) |
| `bw` | Desaturate to B&W, whole clip or a window (`--at`, `--dur`, `--strength` keeps muted color) |
| `volume` | Gain ±dB; `--at/--dur` limits it to a window — comma list covers several spots (needs `--dur`) (platform loudness is `loudnorm`) |
| `blur` | Full-frame or windowed gaussian blur (`--sigma`, `--at`/`--dur`; `--at end` = tail) |
| `trail` | Motion trails: `--mode echo` ghost smear behind movement (`--frames` 2-16, `--at`/`--dur` window), `--mode light` bright-pixel persistence (`--decay` 0.5-0.99) |
| `glitch` | Datamosh-style glitch: `--strength` 0.5-20 drives RGB channel shift + temporal noise |
| `solarize` | Psychedelic partial invert: pixels above `--threshold` luma invert, `--at` windows |
| `pulse` | Breathing zoom bounce: `--rate` cycles/sec, `--depth` amplitude, `--at` windows |
| `deflicker` | Timelapse flicker fix: temporal luma smoothing `--size` frames |
| `emboss` | Relief emboss: convolution kernel, `--amount` mixes back with source, `--at` windows |
| `tilt` | Tilt-shift miniature: blurs top/bottom strips, `--band` sharp fraction, `--blur` sigma, `--at` windows |
| `sway` | Handheld drift: sine-wander crop on padded frame, `--rate`/`--px`, `--at` windows |
| `rack` | Rack-focus breathing blur: sine-mixes a blurred copy, `--rate`/`--blur` |
| `outline` | Ink detected edges black over footage: `--strength` threshold, `--at` windows |
| `night` | Night-vision look: green tint + grain + vignette, `--at` windows |
| `snow` | Falling snow overlay: scrolling noise keyed over video, `--density`/`--speed`, `--at` windows |
| `impact` | Beat-hit punch: white flash + decaying sine shake at `--at`, `--amp`/`--flash` |
| `wave` | Watery horizontal wave distortion (`geq` resample), `--amp`/`--speed`, `--at` windows |
| `spin` | Pendulum sway: frame rotates by a slow sine, `--deg`/`--rate`, `--at` windows |
| `iris` | Spotlight disc: dim + desat outside a hard circle at `--x`/`--y`/`--radius`, `--at` windows |
| `burst` | Radial zoom smear: blurred blown-up copy blended behind the sharp frame, `--strength` |
| `pick` | Dominant-color report at a timestamp: mean hex + 3x2 zone swatches (JSON only) |
| `diff` | Visual diff between two clips: amplified difference blend, `--side` shows reference |
| `selective` | Keep one color, desaturate the rest: `--color` + `--similarity`, `--at` windows |
| `cartoon` | Comic look: posterized base (`--levels` 2-16) + ink outlines from edge-detect, `--at` windows |
| `heat` | Thermal / false-color luma map: `--preset` magma|inferno|plasma|viridis|turbo|cividis|range1|range2|shadows|highlights, `--opacity`, `--at` windows |
| `kaleido` | 2x2 mirrored mandala from the top-left quadrant, `--at` windows |
| `strobe` | Music-video flash cuts: `--rate` flashes/sec, `--duty` on-fraction, `--color`, `--at` windows |
| `edge` | Neon edge-detect outlines: `--mode wires|colormix`, `--low`/`--high` thresholds, `--at` windows |
| `lens` | Lens distortion: `--k1`/`--k2` — negative values give a fisheye look, positive defish action cams; `--at` windows |
| `mirror` | Mirror half the frame across the center axis (`--axis x`/`y`, `--at`/`--dur` window) — dance/symmetry look |
| `pix` | Chunky retro pixelation: `--strength` 2-64 block divisor (`--at`/`--dur` window) |
| `flip` | Horizontal/vertical flip (`--axis x` unmirror selfie footage, `--at`/`--dur` window) |
| `poster` | Pop-art posterization: `--levels` 2-64 palette colors (`--at`/`--dur` window) |
| `duotone` | Two-tone color map: `--shadow`/`--highlight` ramp over luminance (`--at`/`--dur` window) |
| `glow` | Dreamy bloom: blurred copy screen-blended back (`--strength`, `--at`/`--dur` window) |
| `vhs` | Retro tape look: `--strength` 0-3 noise + chroma shift + scanlines (`--at`/`--dur` window) |
| `motionblur` | Shutter smear: `--frames` 2-8 temporal blend (`--at`/`--dur` window) |
| `vdenoise` | Spatial video denoise for grainy footage: `--strength` 0.5–30, `--at`/`--dur` window — comma list ok (nlmeans; slow on long clips), `end` ok |
| `crop` | Crop `--region x:y:w:h`, or `--aspect` reframe with `--anchor center|top|bottom|left|right` |
| `waveform` | Audio waveform → PNG (`--size`, `--color`, `--scale`, `--peak` transients, `--split` per-channel rows, `--full` dense draw, `--bg` opaque card) for podcast art/thumbnails (`--at/--dur` slice, `end` ok, comma `--at` renders `<stem>_N.png` per window, `--vertical` top→bottom wave) |
| `spectrogram` | Audio spectrogram → PNG (`--size`) — spot hum/noise before cleanup (`--color` magma/viridis…, `--scale` lin/sqrt…, `--no-legend`, `--separate` per-channel bands) (`--at/--dur` slice, `end` ok, comma `--at` renders `<stem>_N.png` per window) |
| `meter` | Live EBU R128 loudness meter video (`--size`, `--meter 9\|18`, `--at/--dur` slice) — watch I/TP/LRA while audio plays |
| `dehum` | Notch out mains hum (`--mains 50|60` or `--freq HZ` custom, `--harmonics`, `--at/--dur`, `end` ok, comma list = several windows) |
| `tempo` | Speed audio `--factor` 0.5–8, pitch held (`atempo` chain; use `speed` for video) , `--at/--dur` retempo just a window — comma list ok; `end` ok |
| `leveler` | Compress dynamics (`--preset`, `--at/--dur` window); `end` ok, comma list = several windows |
| `gate` | Noise gate — silence below `--threshold` dB (`agate`) (`--preset voice|podcast|studio`, `--at/--dur` window); `end` ok, comma list = several windows |
| `silence` | Insert `--dur` secs of silence at `--at` (comma list pads several points) or `--end`; `--detect` reports silence ranges as JSON |
| `vocal` | Remove/isolate center vocals (`--mode`, `--at/--dur` window, `--amount` strength); `end` ok, comma list = several windows |
| `remux` | Container swap, no re-encode (`-c copy` + faststart on mp4/mov); `--audio` rips the track, `--video` video-only repack, `--aspect 16:9` display-AR fix |
| `meme` | Top/bottom meme captions (`--outline`, `--at/--dur` window — comma list for several spots; `--at end` tail) , `--position` text block top/center/bottom; `--wrap` folds, `--align` line alignment, `--fade` edge fades with --at/--dur, `--opacity` ghost text |
| `voice` | Podcast voice one-shot: `agate`→`acompressor`→`loudnorm` (`--threshold`, `--lufs`, `--at`/`--dur` window, `end` ok, comma list = several windows) |
| `deinterlace` | Fix interlaced footage (`--mode`, `--parity` field order, `--engine` yadif/bwdif) |
| `crossfade` | Blend two audio files with `--dur`s overlap (`acrossfade`) |
| `strip` | Remove all metadata + chapters, lossless `-c copy` |
| `frames` | Still dump every `--every`, `--at` seconds (`end` = last frame), `--count` even-spread → `stem_001.png…` (`--width`) |
| `countdown` | Overlay a counting leader (`--from` up to 600, `--beep` + `--tone` Hz, `--text`, `--position`, `--bg` numeral plate, `--format` mm:ss/h:mm:ss, `--opacity` ghost) |
| `invert` | Full-frame or windowed color inversion (`--at`, `--dur` — comma list ok) |
| `mix` | Blend two sources (`--vol-a/--vol-b`, `--at/--dur` window — comma list for several entrances, `end` ok, `--loop`, `--duck` sidechain bed under voice) , `--normalize`, `--fade` bed edges |
| `mute` | Drop the audio track, stream-copy the rest , `--at/--dur` silences only that window (comma list covers several spots, needs `--dur`; `end` ok) |
| `timer` | On-screen running clock (`--position`, `--format ms`, `--box-color` card)  (`--format`, `--box-color`, `--down` countdown, `--start` seed the readout) — `--at` accepts `end`; `--opacity` ghost HUD |
| `hls` | Web-ready HLS (`--seg`, `--single`, `--copy`, `--ladder` ABR, `--audio-only` podcast streams, `--fmp4` CMAF, `--poster` writes poster.jpg, `--poster-at T` picks the frame, `--encrypt` AES-128 + `key.bin`/`key.info` (custom `--key HEX`, `--key-uri URI`) for private/paywalled streams) |

| `qa` | Measure quality loss vs a reference: PSNR + SSIM (`--metric`) |
| `conform` | Resize/fps/loudnorm to spec in one pass; `--size WxH`, `--fps 30`, `--lufs -14`, `--crf`, `--pad` letterbox color + `--anchor`, `--blur` blurred fill |
| `sync` | Shift audio ±ms to fix A/V sync (`--ms`) |
| `align` | Auto-sync a second recording to a reference by audio cross-correlation — multi-cam/external recorder (`--max-lag`) |
| `scroll` | Rolling end credits: text rolls bottom→top (`--text`/`--file`, `--at`, `--dur` or `--speed` px/s, `--size`, `--color`, `--font`, `--align` left/right, `--wrap` folds); `--mode ticker` news crawl with `--bg` opaque bar; `--at` comma list replays the roll, `end` ok; `--opacity` ghost credits |
| `insert` | Splice a clip mid-video (`--at`, comma list splices at several points, `end` appends; `--dur` cap; `--transition` any xfade `--duration` S crossfades both joints) |
| `multicam` | Two-camera angle switching across an aligned pair: `--at t1,t2,...` flips angle (`end` = tail switch); `--keep-audio` stays on cam A, `--transition` xfade switches |
| `art` | Attach embedded cover art to audio; `--extract` pulls it out to an image |
| `batch` | One verb on every media file in a directory |
| `pipeline` | Run a JSON scheme in order (`$src` / `$in` / `expect`) |
| `graph` | JSON filter graph, see [`references/graph.md`](references/graph.md) |
| `ffmpeg` | Guarded native ffmpeg; **requires** `--because REASON` |
| `install-skill` | Write the skill into host dirs |
| `version` | Binary / embedded skill / installed copies; `--check` verifies |

Talk through a scheme, then `pipeline`. One step: that verb. No verb: `graph`. Still uncovered: `ffmpeg --because` (say which verb or graph field was not enough).

## Develop

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

When you add a verb, add a Hands row in `SKILL.md`. When behavior or install changes, update `README.md` **and** `README.zh.md` in the same PR, plus `SKILL.md` / `references/` / CHANGELOG. Version only through `scripts/bump-version.sh`. Every merge to `main` must bump, and must get a GitHub Release.

SemVer: MAJOR = breaking CLI or skill workflow (dropped verb, JSON contract change); MINOR = new verb or capability; PATCH = fix and docs.
