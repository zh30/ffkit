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
| `cut` | Trim; lossless copy by default, `--accurate`, `--ranges`, `--drop` for frame-exact |
| `concat` | Join N clips (any xfade `--transition`, `--audio-fade`); `--level -14` loudnorms each clip first |
| `fit` | Frame / rotate / flip (9:16, 1:1, 16:9, …); `--fit blur` fills with a blurred backdrop , `--position` anchors the picture in the bars , `--strength` sigma |
| `extract` | Still frame or `--gif` clip (`--bounce` palindrome) | `--at`, `--dur`, `--width`, `--fps` , `--loop` GIF repeat count, `--colors` palette size |
| `overlay` | Image/video overlay; position/scale/`--tile`, `--fade`, `--angle`, `--at`/`--dur`, `--mode`, `--opacity` blend composite, `--loop` repeat short clips | Logo/picture-in-picture; `--tile N` draft watermark | Logo, watermark, picture-in-picture; `--border` PiP ring |
| `broll` | Cutaway insert (`--insert` video, `--still` image, `--motion kenburns`) | Full-frame cutaway (`--insert --at --duration`); A-roll audio and length stay , `--audio` hear the insert (`--volume` its level) , `--position` PiP corner + `--scale` |
| `caption` | Burn subtitles (`--srt`, `--chunk`, `--karaoke`, `--box-color` card) , `--fade` soft in/out |
| `loudnorm` | EBU R128 two-pass normalization (`--target spotify|podcast|broadcast`, `-I/--tp/--lra`); `--measure` reports loudness without writing; `--dynamic` per-frame gain |
| `denoise` | Audio cleanup (`--strength`, `--highpass`, `--at/--dur` window) |
| `transcode` | h264/webm/`--preset gif` (`--fps`/`--width`/`--copy-audio`/`--colors`) | Presets `h264` / `webm` / `gif` / `hevc`; `--fps` retimes video too , `--preset prores` FCP delivery; `av1` preset |
| `compress` | Fit a size budget (`--size 10MB` two-pass, `--target discord|whatsapp|gmail`); `--crf` quality one-pass, `--res` downscale to free bitrate |
| `deliver` | One-shot 9:16 social pack (Reels / TikTok / Shorts, −14 LUFS) |

| `audiogram` | Waveform video | `--mode`, `--scale` amp scale, `--text`, `--bg`, `--progress` bar , `--subs` burn an .srt on it; `--mode spectrum` bars |

| `split` | Split by `--every`/`--at`/`--scenes`/`--size`/`--parts`/`--silence`/`--chapters`; `--subs` writes re-timed per-part .srt |
| `slideshow` | Still images → video montage (`--per`, `--fade`, `--transition`, `--motion kenburns`, `--audio` bed, `--size` canvas) |
| `speed` | Change playback speed (`--factor`, `--at/--dur`, `--ramp` FROM,TO) |
| `music` | Bed under speech with ducking (`--track`) |
| `key` | Green-screen composite: `--color` keyed out over `--bg` image/video (`--similarity`, `--blend`, `--despill`) |
| `grid` | Multi-up collage (`--layout`, `--audio` pick, `--labels`, `--gap`, `--fill` crop instead of letterbox) |
| `progress` | Bottom/top progress bar, whole clip or a window (`--color`, `--height`, `--edge`, `--at`, `--dur`) |
| `freeze` | Hold a frame (`--at`, `--dur`, `--end`, `--ease`, `--reverse`) |
| `censor` | Blur/mosaic a region (`--mode`, `--strength`, `--at`) |
| `bleep` | Tone over a word/segment: `--at`/`--dur`, `--freq`, `--level` |
| `boomerang` | Forward + reversed replay (one loop, social trick) (`--times` repeat cycles) , `--at/--dur` bounces just that window |
| `chapter` | Embed chapter marks at `TIME|TITLE` or `--import` a marks file; `--auto` / `--export`; `--list` dumps embedded marks |
| `autocrop` | Detect & strip letterbox/pillarbox (`cropdetect` scan → `crop`; `--buffer N` keeps N px edge context) |
| `sheet` | Contact sheet grid (`--cols`x`--rows`, `--time` stamps, `--from`/`--to` window) |
| `pitch` | Shift pitch ±12 semitones, duration kept (`--at/--dur` window) |
| `cutsil` | Strip dead air at head+tail of an audio file (`--thresh` dB) |
| `channel` | Channel surgery: `--mode dualmono|mono|swap|invert|mix51`; `widen` stereo |
| `eq` | Audio shelving EQ: `--bass`/`--treble`/`--presence`, `--preset` dB (`--at`/`--dur` window) , `--band` parametric F:G[:W], `--tilt` warm↔bright |
| `reverb` | Room ambience on a voice: `--size room\|hall\|cave`, `--wet` (`--at`/`--dur` window) |
| `fx` | Audio FX rack: tremolo/vibrato/flanger/phaser/chorus/echo/lofi/radio (`--kind`, `--strength`, `--at`/`--dur`) |
| `rotate` | 90/180/270 or mirror: `--deg`/`--flip`, free `--angle` tilt |
| `delogo` | Blend out a burned-in logo box: `--x --y --w --h`; `--at`/`--dur` for a window (`--soft` removelogo) |
| `meta` | Container tags (`--title`/`--artist`/`--album`/`--genre`/`--date`/`--track`/`--comment`) + `--rotate`, `--clear`, stream-copy |
| `subs` | Extract (`--stream`, `--all`)/burn/mux subtitles (`--shift/--merge/--rate`, burn style + `--outline`/`--box` plate/`--align`, `--safe`); `--convert` .srt↔.vtt; `--case` cue text |
| `thumb` | One-frame cover grab (`--at` / `--frame`, `--count` N stills, `--width`) → jpg/png/webp; `--scenes` stills at cuts |
| `solid` | Solid-color clip card (`--color`, `--size`, `--dur`; silent stereo optional) (`--gradient` animated) , `--text` end-card text (`--wrap` fold long text) |
| `replace` | Swap the audio track (`--mix`, `--duck`, `--fade`, `--loop` short beds, `--at`/`--dur` window) |
| `jumpcut` | Cut silence inside a talking-head take |
| `rough` | Map speech islands in a long take (`--json`); `-o` assembles (`--merge N` merges keeps closer than N s) (default encodes only keeps; `--copy` is lossless/keyframe-sloppy) |
| `cover` | 9:16 cover still (`--at`, `--blur` ambient pad) |
| `fade` | Video and audio fade (`--in` / `--out`, `--color` e.g. white, `--dip T` scene-change dip) |
| `title` | Hook/title card (`--at`, `--fade`, `--outline`, `--box` backplate, `--wrap` long hooks) |
| `loop` | `--times` or `--until` seconds | Repeat the clip N times (Shorts replay length) (`--from`/`--to` loops only a section, `--fade` seamless joints) |
| `stabilize` | Handheld deshake — `--rx`/`--ry` radius, `--edge` fill (blank|original|clamped|mirror) |
| `reverse` | Play picture and sound backwards |
| `grade` | `--preset` look, `--contrast/--saturation/--brightness/--gamma/--hue/--lut/--grain/--warm` | Presets `cinematic`/`vivid`/`vintage`/`soft` stack under the sliders; `--lut look.cube` applies a 3D LUT | `--at`/`--dur`
| `zoom` | Center punch-in (`--factor 1.25`) | `--out`
| `sharpen` | Unsharp mask, whole clip or a window (`--amount`, `--at`, `--dur`) |
| `vignette` | Corner darkening, whole clip or a window (`--angle`, `--at`, `--dur`) |
| `bw` | Desaturate to B&W, whole clip or a window (`--at`, `--dur`) |
| `volume` | Gain ±dB (platform loudness is `loudnorm`) |
| `blur` | Full-frame or windowed gaussian blur (`--sigma`, `--at`, `--dur`) |
| `vdenoise` | Spatial video denoise for grainy footage: `--strength` 0.5–30 (nlmeans; slow on long clips) |
| `crop` | Crop `--region x:y:w:h`, or `--aspect` reframe with `--anchor center|top|bottom|left|right` |
| `waveform` | Audio waveform → PNG (`--size`, `--color`, `--scale`, `--peak` transients, `--split` per-channel rows) for podcast art/thumbnails (`--at/--dur` slice) |
| `spectrogram` | Audio spectrogram → PNG (`--size`) — spot hum/noise before cleanup (`--color` magma/viridis…, `--scale` lin/sqrt…, `--no-legend`) (`--at/--dur` slice) |
| `dehum` | Notch out mains hum (`--mains 50|60` or `--freq HZ` custom, `--harmonics`, `--at/--dur`) |
| `tempo` | Speed audio `--factor` 0.5–8, pitch held (`atempo` chain; use `speed` for video) , `--at/--dur` retempo just a window |
| `leveler` | Compress dynamics (`--preset`, `--at/--dur` window) |
| `gate` | Noise gate — silence below `--threshold` dB (`agate`) (`--preset voice|podcast|studio`, `--at/--dur` window) |
| `silence` | Insert `--dur` secs of silence at `--at` or `--end`; `--detect` reports silence ranges as JSON |
| `vocal` | Remove/isolate center vocals (`--mode`, `--at/--dur` window, `--amount` strength) |
| `remux` | Container swap, no re-encode (`-c copy` + faststart on mp4/mov); `--audio` rips the track only |
| `meme` | Top/bottom meme captions (`--outline`, `--at/--dur` window) , `--position` text block top/center/bottom; `--wrap` folds, `--align` line alignment |
| `voice` | Podcast voice one-shot: `agate`→`acompressor`→`loudnorm` (`--threshold`, `--lufs`) |
| `deinterlace` | Fix interlaced footage (`--mode`, `--parity` field order, `--engine` yadif/bwdif) |
| `crossfade` | Blend two audio files with `--dur`s overlap (`acrossfade`) |
| `strip` | Remove all metadata + chapters, lossless `-c copy` |
| `frames` | Still dump every `--every`, `--at` seconds → `stem_001.png…` (`--width`) |
| `countdown` | Overlay a counting leader (`--from`, `--beep`, `--text`, `--position`) |
| `invert` | Full-frame or windowed color inversion (`--at`, `--dur`) |
| `mix` | Blend two sources (`--vol-a/--vol-b`, `--at/--dur`, `--loop`, `--duck` sidechain bed under voice) , `--normalize`, `--fade` bed edges |
| `mute` | Drop the audio track, stream-copy the rest , `--at/--dur` silences only that window |
| `timer` | On-screen running clock (`--position`, `--format ms`, `--box-color` card)  (`--format`, `--box-color`, `--down` countdown, `--start` seed the readout) |
| `hls` | Web-ready HLS (`--seg`, `--single`, `--copy`, `--ladder` ABR, `--audio-only` podcast streams, `--fmp4` CMAF) |
| `qa` | Measure quality loss vs a reference: PSNR + SSIM (`--metric`) |
| `conform` | Resize/fps/loudnorm to spec in one pass; `--size WxH`, `--fps 30`, `--lufs -14`, `--crf`, `--pad` letterbox color + `--anchor`, `--blur` blurred fill |
| `sync` | Shift audio ±ms to fix A/V sync (`--ms`) |
| `align` | Auto-sync a second recording to a reference by audio cross-correlation — multi-cam/external recorder (`--max-lag`) |
| `scroll` | Rolling end credits: text rolls bottom→top (`--text`/`--file`, `--at`, `--dur`, `--size`, `--color`, `--font`); `--mode ticker` news crawl |
| `insert` | Splice a clip mid-video (`--at`; `--dur` cap the insert; `--transition` any xfade `--duration` S crossfades both joints) |
| `multicam` | Two-camera angle switching across an aligned pair: `--at t1,t2,...` flips angle; `--keep-audio` stays on cam A, `--transition` xfade switches |
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
