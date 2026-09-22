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
| `cut` | Trim; lossless copy by default, `--accurate` for frame-exact |
| `concat` | Join; `--transition` crossfades between every clip (xfade + acrossfade, N inputs) |
| `fit` | Frame / rotate / flip (9:16, 1:1, 16:9, …); `--fit blur` fills with a blurred backdrop |
| `extract` | Audio, a frame, or subtitles from the output extension |
| `overlay` | Logo, watermark, picture-in-picture |
| `broll` | Full-frame cutaway (`--insert --at --duration`); A-roll audio and length stay |
| `caption` | `--mode mux` soft subs; `burn` overlay raster (no libass); `--safe social` clears the bottom 20%; `--chunk N` splits cues into ≤N-word groups |
| `loudnorm` | EBU R128 two-pass (`-I -14` social, `-I -16` podcast) |
| `denoise` | Voice cleanup (fan / rumble / hiss); `--video` also degrains the picture |
| `transcode` | Presets `h264` / `webm` / `gif` |
| `compress` | Two-pass shrink to `--size 10MB` (Discord, WhatsApp 16MB, email ~25MB) |
| `deliver` | One-shot 9:16 social pack (Reels / TikTok / Shorts, −14 LUFS) |
| `audiogram` | Podcast audio → 9:16 waveform video over a cover still (`--image`) |
| `split` | Chop into fixed-length parts (`--every 30` → `stem_00..` via forced keyframes + segment muxer) |
| `slideshow` | Still images → video montage (`--per`, `--fade`, `--transition`, `--motion kenburns`, `--audio` bed, `--size` canvas) |
| `speed` | `--factor 2` is 2×; talking-head keeps pitch |
| `music` | Bed under speech with ducking (`--track`) |
| `replace` | Swap the video's audio track for `--audio` (lav mic, clean voice, new music); `--audio-offset` for sync, `--mix` to keep the original under it |
| `jumpcut` | Cut silence inside a talking-head take |
| `rough` | Map speech islands in a long take (`--json`); `-o` assembles (default encodes only keeps; `--copy` is lossless/keyframe-sloppy) |
| `cover` | 9:16 cover still (1080×1920) |
| `fade` | Video and audio fade (`--in` / `--out`) |
| `title` | First-second hook card (`--text`, no libass) |
| `loop` | Repeat the clip N times (Shorts replay length) |
| `stabilize` | Handheld deshake |
| `reverse` | Play picture and sound backwards |
| `grade` | Contrast / saturation / brightness (mild Reels pop); `--lut look.cube` applies a 3D LUT |
| `zoom` | Center punch-in (`--factor 1.25`) |
| `sharpen` | Unsharp |
| `vignette` | Darken corners |
| `bw` | Black and white |
| `volume` | Gain ±dB (platform loudness is `loudnorm`) |
| `blur` | Gaussian blur (`--sigma`) |
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
