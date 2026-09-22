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
| `concat` | Join; `--transition` crossfades between every clip (xfade + acrossfade, N inputs) |
| `fit` | Frame / rotate / flip (9:16, 1:1, 16:9, …); `--fit blur` fills with a blurred backdrop |
| `extract` | Audio, a frame, or subtitles from the output extension | `--width`
| `overlay` | Image/video overlay; position/scale/`--tile`, `--fade`, `--angle`, `--at`/`--dur`, `--mode`, `--opacity` blend composite | Logo/picture-in-picture; `--tile N` draft watermark | Logo, watermark, picture-in-picture |
| `broll` | Cutaway insert (`--insert` video, `--still` image, `--motion kenburns`) | Full-frame cutaway (`--insert --at --duration`); A-roll audio and length stay |
| `caption` | SRT burn (`--mode`, `--safe`, `--chunk`, `--shift`, `--color`, `--size`, `--position`) | Burn/mux `.srt`; `--chunk` word groups, `--shift` timing nudge | `--mode mux` soft subs; `burn` overlay raster (no libass); `--safe social` clears the bottom 20%; `--chunk N` splits cues into ≤N-word groups |
| `loudnorm` | EBU R128 two-pass (`-I -14` social, `-I -16` podcast) |
| `denoise` | Voice cleanup (fan / rumble / hiss); `--video` also degrains the picture |
| `transcode` | h264/webm/`--preset gif` (`--fps`/`--width`) | Presets `h264` / `webm` / `gif`; `--fps` retimes video too (`--preset hevc` for H.265/hvc1) |
| `compress` | Two-pass shrink to `--size 10MB` (Discord, WhatsApp 16MB, email ~25MB) |
| `deliver` | One-shot 9:16 social pack (Reels / TikTok / Shorts, −14 LUFS) |

| `audiogram` | Podcast audio → 9:16 waveform video over a cover still (`--image`, `--mode`/`--color`, `--bg`, `--size`, `--text`, `--position`) |

| `split` | `--every` regular parts, `--at` explicit, `--scenes` shot-detection, `--size` target-MB, `--parts N` equal | Chop into parts (`--every 30` even chunks, or `--at 30,90` chapter points) → `stem_00..` |
| `slideshow` | Still images → video montage (`--per`, `--fade`, `--transition`, `--motion kenburns`, `--audio` bed, `--size` canvas) |
| `speed` | `--factor 2` is 2×; talking-head keeps pitch |
| `music` | Bed under speech with ducking (`--track`) |
| `key` | Green-screen composite: `--color` keyed out over `--bg` image/video (`--similarity`, `--blend`, `--despill`) |
| `grid` | N-input mosaic; `--audio N` keeps one input's track | N inputs into a `--layout CxR` tile wall (`--size 1920x1080`); audios mix when all inputs have one |
| `progress` | Bottom/top progress bar, whole clip or a window (`--color`, `--height`, `--edge`, `--at`, `--dur`) |
| `freeze` | Hold the frame at `--at T` for `--dur D` (mid-clip), or clone the last frame with `--end D` (outro) |
| `censor` | Mosaic/blur a face/logo region `--region x:y:w:h` (`--mode pixel|blur`; `--at`/`--dur` window) |
| `bleep` | Tone over a word/segment: `--at`/`--dur`, `--freq`, `--level` |
| `boomerang` | Forward + reversed replay (one loop, social trick) (`--times` repeat cycles) |
| `chapter` | Embed `--at T|TITLE` chapter markers (lossless `-c copy` metadata pass) |
| `autocrop` | Detect & strip letterbox/pillarbox (`cropdetect` scan → `crop`) |
| `sheet` | Contact sheet: `--cols`/`--rows` thumbnails → one PNG | `--pad`/`--margin`
| `pitch` | `--semitones N` voice/music shift, duration preserved |
| `cutsil` | Strip dead air at head+tail of an audio file (`--thresh` dB) |
| `channel` | Channel surgery: `--mode dualmono|mono|swap` |
| `eq` | Audio shelving EQ: `--bass`/`--treble`/`--presence`, `--preset` dB (`--at`/`--dur` window) |
| `reverb` | Room ambience on a voice: `--size room\|hall\|cave`, `--wet` (`--at`/`--dur` window) |
| `fx` | Audio FX rack: tremolo/vibrato/flanger/phaser/chorus/echo/lofi/radio (`--kind`, `--strength`, `--at`/`--dur`) |
| `rotate` | 90/180/270 or mirror: `--deg`/`--flip` |
| `delogo` | Blend out a burned-in logo box: `--x --y --w --h`; `--at`/`--dur` for a window |
| `meta` | Container tags (`--title`/`--artist`/`--album`/`--genre`/`--date`/`--track`/`--comment`) + `--rotate`, `--clear`, stream-copy |
| `subs` | Extract embedded subtitles (`--stream`), `--burn` hardsubs, `--shift ±N` retime an .srt |
| `thumb` | One-frame cover grab (`--at` / `--frame`, `--width`) → jpg/png/webp |
| `solid` | Solid-color clip card (`--color`, `--size`, `--dur`; silent stereo optional) |
| `replace` | Swap the video's audio track for `--audio` (lav mic, clean voice, new music); `--audio-offset` for sync, `--mix` to keep the original under it |
| `jumpcut` | Cut silence inside a talking-head take |
| `rough` | Map speech islands in a long take (`--json`); `-o` assembles (default encodes only keeps; `--copy` is lossless/keyframe-sloppy) |
| `cover` | 9:16 cover still (1080×1920) |
| `fade` | Video and audio fade (`--in` / `--out`, `--color` e.g. white) |
| `title` | On-screen hook/caption PNG (`--at`, `--position` incl. corners, `--fade`, `--outline`, `--shadow`) |
| `loop` | `--times` or `--until` seconds | Repeat the clip N times (Shorts replay length) (`--from`/`--to` loops only a section) |
| `stabilize` | Handheld deshake |
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
| `waveform` | Audio waveform → PNG (`--size`, `--color`, `--scale`) for podcast art/thumbnails |
| `spectrogram` | Audio spectrogram → PNG (`--size`) — spot hum/noise before cleanup (`--color` magma/viridis…) |
| `dehum` | Notch mains hum `--mains 50|60` + `--harmonics` (Q=12 `equalizer` chain) |
| `tempo` | Speed audio `--factor` 0.5–8, pitch held (`atempo` chain; use `speed` for video) |
| `leveler` | Voice dynamic-range compressor (`--threshold`/`--ratio`/`--makeup`) (`--preset` voice/podcast/master) |
| `gate` | Noise gate — silence below `--threshold` dB (`agate`) (`--preset voice|podcast|studio`) |
| `silence` | Insert `--dur` secs of silence at `--at`, or append with `--end` |
| `vocal` | `--mode karaoke` drops centered vocals; `isolate` keeps the center (stereo) |
| `remux` | Container swap, no re-encode (`-c copy` + faststart on mp4/mov) |
| `meme` | `--top`/`--bottom` caption text burned in (`--color`, `--size`) |
| `voice` | Podcast voice one-shot: `agate`→`acompressor`→`loudnorm` (`--threshold`, `--lufs`) |
| `deinterlace` | `yadif` for DV/interlaced sources (`--mode frame`/`field`) |
| `crossfade` | Blend two audio files with `--dur`s overlap (`acrossfade`) |
| `strip` | Remove all metadata + chapters, lossless `-c copy` |
| `frames` | Still dump every `--every`, `--at` seconds → `stem_001.png…` (`--width`) |
| `countdown` | 3-2-1(-GO) intro overlay (`--from`, `--each`, `--go`, `--at`) (`--beep` tick tones) |
| `invert` | Full-frame or windowed color inversion (`--at`, `--dur`) |
| `mix` | Sum two audio sources (`--vol-a`/`--vol-b` linear, `--longest`) |
| `mute` | Drop the audio track, stream-copy the rest |
| `timer` | Burn a running MM:SS(/H:) counter (`--position`, `--at`, `--dur`) |
| `hls` | Package to `index.m3u8` + `seg_*.ts` (`--seg` seconds) |
| `qa` | Measure quality loss vs a reference: PSNR + SSIM (`--metric`) |
| `conform` | Normalize mixed footage for concat (`--size`, `--fps`, `--lufs`) |
| `sync` | Shift audio ±ms to fix A/V sync (`--ms`) |
| `art` | Attach a cover image (`--image`) → mp3/m4a/mp4/mkv |
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
