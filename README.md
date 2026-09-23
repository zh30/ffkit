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
| `denoise` | Audio cleanup (`--strength`, `--highpass`, `--engine auto|wavel|fftdn` pick the denoiser, `--at/--dur` window); `end` ok, comma list = several windows | `--ref noise.wav` anlms adaptive cancel |
| `transcode` | h264/webm/`--preset gif` (`--fps`/`--width`/`--copy-audio`/`--colors`) | Presets `h264` / `webm` / `gif` / `hevc` / `mp3` / `aac` / `wav` / `flac` / `opus` / `av1` / `prores` / `dnxhd`; `--fps` retimes video too; `--vbitrate` peak bitrate cap, `--abitrate` audio bitrate (voice → 64k), `--preset prores`/`dnxhd` NLE delivery; `av1` preset; `--alpha` keeps transparency (webm/prores); `--range limited\|full` tags colour range |
| `compress` | Fit a size budget (`--size 10MB` two-pass, `--target discord|whatsapp|gmail`); `--crf` quality one-pass, `--res` downscale to free bitrate |
| `deliver` | One-shot platform pack (Reels / TikTok / Shorts 9:16, `square` 1:1 grid, `youtube` 16:9; −14 LUFS; `--fps 60` high-frame-rate uploads, `--crf` quality, `--subs file.srt` burns captions in one pass) |

| `audiogram` | Waveform video ，`--mode phase`（aphasemeter 相位表）| `--mode`, `--scale`, `--split` channels, `--fscale` freq axis (spectrum), `--fps` rate, `--text`, `--bg`, `--progress` bar , `--subs` burn an .srt on it, `--from`/`--to` clip a segment (`end`/`end-N` ok), `--at a,b --dur N` one clip per point (`stem_N.mp4`); `--mode spectrum` bars, `--mode scope` lissajous vectorscope, `--mode cqt` piano-roll spectrum, `--mode spectro` scrolling spectrogram | `--mode spatial|volume|bitscope` meter scopes | `--mode monitor` stats viz |

| `split` | Split by `--every`/`--at`/`--scenes`/`--size`/`--parts`/`--silence`/`--chapters`; `--subs` writes re-timed per-part .srt; `--fade N` softens each part's edges |
| `slideshow` | Still images → video montage (`--per` or `--dur` total runtime, `--fade`, `--transition`, `--motion kenburns`, `--audio` bed + `--volume`, `--size` canvas, `--bg` letterbox) |
| `speed` | Change playback speed (`--factor`, `--at/--dur` (comma list = several windows), `--ramp` FROM,TO); `end` ok |
| `music` | Bed under speech with ducking (`--track`, `--at`/`--dur` window, `end` ok; comma `--at` = multi-entrance bed) |
| `key` | Green-screen composite: `--color` keyed out over `--bg` image/video (`--similarity`, `--blend`, `--despill`, `--mode luma` keys a luma band around `--threshold` for white-sky/dark-backdrop shots, `--at`/`--dur` window — comma list ok) | `--mode matte --mask` external grayscale matte → alpha (prores 4444) |
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
| `channel --mode ms` | Decode mid/side-recorded stereo back to L/R (stereotools ms>lr) |
| `channel` | Channel surgery: `--mode dualmono|mono|swap|invert|mix51|pan|widen|split|ambience|mid|side|haas|surround|base|bal|bands|sync|earwax` (stereo→`_L/_R.wav` stems, M/S extract, haas widening, stereo→5.1 upmix); `--pan -1..1` pan / `bal` rebalance lopsided stereo / `base` stereo base (-1 mono fold, +1 wide) | `bands --freqs 300,3000` → `<stem>_bandN.wav` frequency-band stems (acrossover) | `sync --side right --cm 34` delay one side by mic distance (two-mic comb-filter fix, compensationdelay) | `earwax` headphone-oriented stereo widening |
| `eq` | Audio shelving EQ: `--bass`/`--treble`/`--presence`, `--preset` dB (`--at`/`--dur` window) , `--band` parametric F:G[:W], `--curve` freehand F,G;F,G line (firequalizer), `--graphic` 18-band classic EQ, `--tilt` warm↔bright; `end` ok, comma list = several windows |
| `reverb` | Room ambience on a voice: `--size room\|hall\|cave`, `--wet` (`--at`/`--dur` window); `end` ok, comma list = several windows. `--ir file.wav` = convolution reverb from impulse-response packs (cathedral/plate), `--tail` rings past the end |
| `fx` | Audio FX rack: tremolo/vibrato/flanger/phaser/chorus/echo/lofi/radio/saturate/excite/bass/muffled/crystal/sub/crossfeed/autopan (`--kind`, `--strength`, `--at`/`--dur`); `end` ok, comma list = several windows | `--kind ringmod` TRUE ring modulation (amultiply + sine carrier, `--strength` sweeps 25-500Hz) | `--kind crush` bitcrusher (bits+sample-rate destruction) | `--kind fshift` frequency shifter (metallic alien voice, 50→2000Hz) | `--kind contrast` dynamics tilt (>0.5 punch, <0.5 level) |
| `rotate` | 90/180/270 or mirror: `--deg`/`--flip`, free `--angle` tilt, `--at`/`--dur` windowed tilt (comma list) |
| `delogo` | Blend out a burned-in logo box: `--x --y --w --h` or `--regions x:y:w:h,...` for several spots; `--at`/`--dur` for a window, `--at end` the tail (`--soft` removelogo, `--shape circle` elliptical mask) | `--image mask.png` drawn-mask removal | `--find logo.png` auto-locates it (find_rect, first 15s) — no coordinates |
| `meta` | Container tags (`--title`/`--artist`/`--album`/`--genre`/`--date`/`--track`/`--comment`) + `--rotate`, `--clear`, stream-copy |
| `subs` | Extract (`--stream`, `--all`)/burn/mux subtitles (`--shift` (±N; `--from`/`--to` bounds it)/`--merge`/`--rate`, burn style + `--outline`/`--box` plate/`--align`/`--margin` px/`--from`/`--to` window (`end`/`end-N` ok), `--safe`); `--convert` .srt↔.vtt; `--case` cue text; `--burn-si N` burns embedded track N; `--encoding gbk` legacy charsets |
| `thumb` | One-frame cover grab (`--at` — `end` = last frame, comma `--at` = one still per time / `--frame`, `--count` N stills (`--from`/`--to` bounds the spread, `end`/`end-N` ok), `--width`) → jpg/png/webp; `--scenes` stills at cuts | `--best` representative still |
| `solid` | Solid-color clip card (`--color`, `--size`, `--dur`, `--fps` rate; silent stereo optional) (`--gradient` animated, `--noise` grain) , `--text` end-card text (`--wrap` folds, `--align` lines) |
| `replace` | Swap the audio track (`--mix`, `--duck`, `--fade`, `--loop` short beds, `--at`/`--dur` window — comma `--at` several) — or `--video` to swap the picture and keep the audio |
| `jumpcut` | Cut silence inside a talking-head take |
| `rough` | Map speech islands in a long take (`--json`); `-o` assembles (`--merge N` merges keeps closer than N s, `--by-scene` splits keeps at scene changes) (default encodes only keeps; `--copy` is lossless/keyframe-sloppy) |
| `cover` | Cover still (`--at` — `end` = last frame, comma = one per beat, `--blur` ambient pad, `--size` canvas — default 1080x1920) |
| `fade` | Video and audio fade (`--in` / `--out`, `--color` e.g. white, `--dip T` scene-change dip — comma list dips at every mark; `--curve` audio fade shape) |
| `title` | Hook/title card (`--at` — `end` = end-card, comma list flashes at several marks, `--fade`, `--outline`, `--box` backplate, `--wrap` long hooks, `--align` lines, `--opacity` ghost, `--margin` px corner inset) |
| `loop` | `--times` or `--until` seconds | Repeat the clip N times (Shorts replay length) (`--from`/`--to` loops only a section, `end` ok, `--fade` seamless joints) |
| `stabilize` | Handheld deshake — `--rx`/`--ry` radius, `--edge` fill (blank|original|clamped|mirror) | `--engine vidstab` two-pass vid.stab (steadier on real shake), `--smoothing` frames |
| `reverse` | Play picture and sound backwards |
| `grade` | `--preset` look, `--contrast/--saturation/--brightness/--gamma/--hue/--lut/--grain/--warm/--exposure` (EV stops), `--skin` warmth ，含 `--kelvin` 开尔文白平衡、`--split` 青橙分调 | Presets `cinematic`/`vivid`/`vintage`/`soft`/`sepia`/`teal`/`noir`/`bleach`/`neon` stack under the sliders; `--lut look.cube` applies a 3D LUT, `--lut look.png` a HALD image LUT (haldclut — Darktable/RawTherapee exports); `--skin -1..1` warms faces only (selectivecolor reds), `--vibrance -1..1` smarter saturation (boosts muted, protects skin), `--curve "x/y …"` freeform master curve (matte fade, S-curve) | `--at`/`--dur` `--wash C` colour veil | `--match ref.mp4` histogram match | `--lut` accepts 1D LUTs | `--color-from ref` borrow chroma | `--mix "rr,rg,rb,…"` 3x3 channel matrix (colorchannelmixer) |
| `zoom` | Punch-in (`--factor 1.25`, `--center X,Y` target; `--at`/`--dur` window — comma list for several, `end` ok) | `--out`
| `sharpen` | Unsharp mask, whole clip or a window (`--amount`, `--at`, `--dur`) `--engine unsharp\|cas\|halo` (halo = maskedclamp, no overshoot) |
| `vignette` | Corner darkening, whole clip or a window (`--angle`, `--at`, `--dur`) |
| `bw` | Desaturate to B&W, whole clip or a window (`--at`, `--dur`, `--strength` keeps muted color) ，`--weights r,g,b` 胶片通道权重，`--cut 0-1` hard threshold (xerox/graphic B&W) |
| `volume` | Gain ±dB; `--at/--dur` limits it to a window — comma list covers several spots (needs `--dur`) (platform loudness is `loudnorm`) |
| `blur` | Full-frame or windowed gaussian blur (`--sigma`, `--at`/`--dur`; `--at end` = tail) | `--engine directional --angle` streaks |
| `trail` | Motion trails: `--mode echo` ghost smear behind movement (`--frames` 2-16, `--at`/`--dur` window), `--mode light` bright-pixel persistence (`--decay` 0.5-0.99) | `--mode diff` motion ghost |
| `glitch` | Datamosh-style glitch: `--strength` 0.5-20 drives RGB channel shift + temporal noise | `--engine planes` channel rotation | `--engine swapuv` chroma flip | `--engine stutter` frame jitter | `--engine pixels` block scatter | `--engine swaprect` quadrant swap | `--engine random` frame-order scramble |
| `bars` | SMPTE test card: `--size`/`--dur`/`--hd`/`--tone` (1kHz bed), for QC slates and leader |
| `scope --mode hist` | Rolling temporal histogram of luma — color/exposure drift QC over time |
| `scope` | QC scope overlay: `--mode vector|wave` in a corner (`--position`, `--size` fraction), `--at` windows | `--mode mvs` MV overlay | `--mode data --x/--y` hex readout | `--mode qp` macroblock QP overlay | `--mode pix` magnified pixel grid | `--mode osc` XY video oscilloscope | `--mode drift` luma-drift curve (exposure-ramp QC) | `--mode loud` loudness-over-time curve (ebur128+adrawgraph) |
| `desqueeze` | Anamorphic restore: `--factor` lens ratio (1.33/1.5/1.8/2.0), `--axis y|x` |
| `solarize` | Psychedelic partial invert: pixels above `--threshold` luma invert, `--at` windows |
| `pulse` | Breathing zoom bounce: `--rate` cycles/sec, `--depth` amplitude, `--at` windows |
| `deflicker` | Timelapse flicker fix: temporal luma smoothing `--size` frames | `--engine tmide` temporal equalization |
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
| `thump` | Sub-bass drop at `--at`: 55Hz sine with fast decay mixed under the track, `--freq`/`--gain`/`--dur` |
| `riser` | Tonal chirp sweep (200→2000Hz) that lands on `--at`, `--dur` rise length, `--gain` |
| `whoosh` | Airy brown-noise swell that lands on `--at` (transition accent), `--dur`/`--gain` |
| `deesser` | Voice de-essing: tames the 4-8kHz sibilance band, `--amount`/`--freq`/`--at` window |
| `declip` | Repairs clipped/blown-out audio (`--engine clip` = `adeclip` peak interpolation, `--engine click` = `adeclick` vinyl pops/dropouts; `--window` ms, `--threshold` 1-100, `--overlap-save`, `--at` window) |
| `deband` | Smooths gradient banding (sky/backdrop steps) via `gradfun`, `--strength`/`--radius`/`--at` window |
| `deblock` | Removes DCT block edges from heavy compression (`--strength` scales detection, `--at` window) |
| `chromashift` | Shift chroma planes by px to fix misregistration halos (`--x`/`--y`, `--edge` wrap/smear, `--at` window) |
| `stack` | Median-stack 3+ locked-off videos of the same scene — removes objects present in <half the inputs (tourists, sensor noise); `--percentile`; audio from input 1 ，`--mode median|max|min`（max=星轨/光绘，min=最暗合成）|
| `stereo` | 3D format convert — SBS↔anaglyph↔interleaved |
| `sonify` | Play an image/video as sound — spectrumsynth scans it like a spectrogram (bright pixels = loud harmonics); `--dur`/`--speed`/`--sample-rate` |
| `tmedian` | Temporal median — erase anything visible <half the window: moving people/cars on tripod shots, rain streaks (`--radius` history frames, `--percentile`, `--at` window; output loses 2*radius edge frames) |
| `dedup` | Drops near-duplicate frames via `mpdecimate` — shrinks static stretches, `--frac` sensitivity |
| `repair` | Swap bad/glitched frames for a frame from a reference take (`--ref`, `--at`/`--dur` damaged stretch, `--ref-at` clean frame — freezeframes) |
| `audiogram --mode cqt` | Constant-Q music spectrum (`showcqt`) — piano-roll spectrum look for music clips |
| `audiogram --mode spectro` | Scrolling spectrogram (`showspectrum`) — colour time/frequency roll |
| `scan` | QC report: black stretches, frozen frames, black-frame hits + strobe `flash_frames`/`flash_max_badness` + interlace verdict (idet) + stereo `phase_corr` (~-1 = mono-collapse) — JSON extras; writes no media , audio peak/mean dB (`audio_max_db`, `audio_mean_db`) | +blur QC | `--scenes` scene-cut timestamps | `luma_min/max` + `illegal_luma` (signalstats broadcast-range QC) | `noise_floor`/`noisy` bit-plane noise budget QC | `has_cc`/`cc_lines` EIA-608 closed captions | `crop_hint`/`letterboxed` cropdetect letterbox QC | `vfr`/`vfr_ratio`/`vfr_frames` variable-frame-rate QC |
| `smooth` | Edge-preserving beauty/skin blur (`--engine` smartblur/bilateral; `--strength`, `--at`/`--dur`) | `--engine uspp` postproc deblock | `--engine pp7` light postproc | `--engine yaep` edge-preserving | `--engine spp/fspp` light deblock |
| `upscale` | Up-res footage: `zscale` spline36 (better than lanczos) + light unsharp, `--factor` 1.05-4 (2 doubles dims), `--strength` edge acuity | `--engine spline|xbr|two-xsai` (pixel-art integer scalers) | `--engine hqx` hq2x/3x/4x pixel-art scaler | `--engine epx` EPX 2x/3x scaler |
| `v360` | Reframe 360 footage to flat (`--in` equirect/fisheye/dfisheye/cubemap/EAC/barrel/half-equirect, `--yaw`/`--pitch`/`--fov`, `--size`) |
| `perspective` | Deskew a filmed screen/whiteboard: `--points x0,y0,x1,y1,x2,y2,x3,y3` (TL,TR,BL,BR quad in source, px), `--interp linear|cubic` |
| `wb` | Auto white balance / cast removal: `--strength` 0..1, `--independence` 0 keeps the grade (contrast only), `--smooth` temporal frames, `--at`/`--dur` window | `--engine greyedge` grey-edge illuminant fix (gentler on graded footage) |
| `shear` | Italic-style picture slant: `--x`/`--y` shear factors -2..2, `--fill` edge color, `--interp nearest|bilinear`, `--at`/`--dur` window |
| `gen` | Generative animated backgrounds from lavfi sources (no input): `--pattern mandelbrot` (endless zoom) `|gradients` (drifting palette — `--colors` up to 8, `--seed`, `--speed`) `|life` (cellular automaton, `--rule`) `|sierpinski` (fractal), `--size`/`--fps`/`--dur` | `--pattern noise|tone` audio beds (`--color` white/pink/brown/blue/violet/velvet) | `--pattern sweep` 20Hz→`--freq` speaker-test chirp |
| `sharpen --engine cas` | Contrast-adaptive sharpening — crisper edges without unsharp halos, `--amount` |
| `equalize` | Auto-contrast via `histeq` for flat/washed footage, `--strength`/`--intensity`/`--at` window |
| `pick` | Dominant-color report at a timestamp: mean hex + 3x2 zone swatches (JSON only) |
| `diff` | Visual diff between two clips: amplified difference blend, `--side` shows reference | `--mode mask --threshold` bare change-mask QC |
| `selective` | Keep one color, desaturate the rest: `--color` + `--similarity`, `--blend` edge feather, `--at` windows |
| `amplify` | Motion magnification — subtle change becomes visible (`--amount` factor, `--radius` frames, `--threshold` diff cap, `--at` windows) |
| `cartoon` | Comic look: posterized base (`--levels` 2-16) + ink outlines from edge-detect, `--at` windows |
| `heat` | Thermal / false-color luma map: `--preset` magma|inferno|plasma|viridis|turbo|cividis|range1|range2|shadows|highlights, `--opacity`, `--at` windows |
| `kaleido` | 2x2 mirrored mandala from the top-left quadrant, `--at` windows |
| `strobe` | Music-video flash cuts: `--rate` flashes/sec, `--duty` on-fraction, `--color`, `--at` windows |
| `edge` | Neon edge-detect outlines: `--mode wires|colormix`, `--low`/`--high` thresholds, `--at` windows | `--engine edgedetect|sobel|kirsch|roberts|prewitt` | `--engine link` hysteresis-linked edges |
| `lens` | Lens distortion: `--k1`/`--k2` — negative values give a fisheye look, positive defish action cams; `--at` windows |
| `mirror` | Mirror half the frame across the center axis (`--axis x`/`y`, `--at`/`--dur` window) — dance/symmetry look |
| `pix` | Chunky retro pixelation: `--strength` 2-64 block divisor (`--at`/`--dur` window) |
| `flip` | Horizontal/vertical flip (`--axis x` unmirror selfie footage, `--at`/`--dur` window) |
| `poster` | Pop-art posterization: `--levels` 2-64 palette colors (`--at`/`--dur` window) |
| `duotone` | Two-tone color map: `--shadow`/`--highlight` ramp over luminance (`--at`/`--dur` window) |
| `glow` | Dreamy bloom: blurred copy screen-blended back (`--strength`, `--at`/`--dur` window) |
| `vhs` | Retro tape look: `--strength` 0-3 noise + chroma shift + scanlines (`--at`/`--dur` window) |
| `motionblur` | Shutter smear: `--frames` 2-8 temporal blend (`--at`/`--dur` window) |
| `vdenoise` | Spatial video denoise for grainy footage: `--strength` 0.5–30, `--engine nlmeans` (quality default) `|hqdn3d|`atadenoise|`vaguedenoise|`bm3d`/`dctdnoiz`/`owdenoise` (strongest three — no `--at` on those), `median` (salt&pepper), `chroma` (color speckle), `--at`/`--dur` window — comma list ok, `end` ok | `--engine dotcrawl` analog artifact removal | `--engine fftdnoiz` FFT film grain | `--engine rg` removegrain fast per-plane modes |
| `crop` | Crop `--region x:y:w:h`, or `--aspect` reframe with `--anchor center|top|bottom|left|right` |
| `waveform` | Audio waveform → PNG (`--size`, `--color`, `--scale`, `--peak` transients, `--split` per-channel rows, `--full` dense draw, `--bg` opaque card) for podcast art/thumbnails (`--at/--dur` slice, `end` ok, comma `--at` renders `<stem>_N.png` per window, `--vertical` top→bottom wave) |
| `spectrogram` | Audio spectrogram → PNG (`--size`) — spot hum/noise before cleanup (`--color` magma/viridis…, `--scale` lin/sqrt…, `--no-legend`, `--separate` per-channel bands) (`--at/--dur` slice, `end` ok, comma `--at` renders `<stem>_N.png` per window) |
| `meter` | Live EBU R128 loudness meter video (`--size`, `--meter 9\|18`, `--at/--dur` slice) — watch I/TP/LRA while audio plays |
| `dehum` | Notch out mains hum (`--mains 50|60` or `--freq HZ` custom, `--harmonics`, `--at/--dur`, `end` ok, comma list = several windows) |
| `tempo` | Speed audio `--factor` 0.5–8, pitch held (`atempo` chain; use `speed` for video) , `--at/--dur` retempo just a window — comma list ok; `end` ok |
| `leveler --engine mcompand` | Multiband compression preset — lifts quiet speech, caps peaks across rumble/body/air bands |
| `leveler` | Compress dynamics (`--preset`, `--engine speechnorm` adaptive speech normalize, `--engine limit` alimiter brickwall ceiling, `--at/--dur` window); `end` ok, comma list = several windows |
| `gate` | Noise gate — silence below `--threshold` dB (`agate`) (`--preset voice|podcast|studio`, `--at/--dur` window); `end` ok, comma list = several windows |
| `silence` | Insert `--dur` secs of silence at `--at` (comma list pads several points) or `--end`; `--detect` reports silence ranges as JSON |
| `vocal` | Remove/isolate center vocals (`--mode`, `--at/--dur` window, `--amount` strength); `end` ok, comma list = several windows |
| `remux` | Container swap, no re-encode (`-c copy` + faststart on mp4/mov); `--audio` rips the track, `--video` video-only repack, `--aspect 16:9` display-AR fix |
| `meme` | Top/bottom meme captions (`--outline`, `--at/--dur` window — comma list for several spots; `--at end` tail) , `--position` text block top/center/bottom; `--wrap` folds, `--align` line alignment, `--fade` edge fades with --at/--dur, `--opacity` ghost text |
| `voice` | Podcast voice one-shot: `agate`→`acompressor`→`loudnorm` (`--threshold`, `--lufs`, `--at`/`--dur` window, `end` ok, comma list = several windows) |
| `deinterlace` | Fix interlaced footage (`--mode`, `--parity` field order, `--engine` yadif/bwdif/estdif/kerndeint) ，`--engine` 含 `detelecine`（确定节奏反电视电影）、`mcdeint`（运动补偿）| `--engine w3fdif` Weston 3-field | `--engine separate` 50i→50p field-per-frame (smooth slow-mo source) | `--engine pullup` IVTC telecine reversal | `--engine phase` field-order swap (wrong-parity captures) |
| `dedust` | Remove dust specks / hot pixels: `--size` 1-4, bright specks by default, `--dark` for dark ones; morphology (erosion/dilation), not a blur | `--at`/`--dur` |
| `extend` | Stretch edge pixels to fill border strips: `--left/--right/--top/--bottom` px, `--mode smear|mirror|fixed|reflect|wrap|fade` | - |
| `tonemap` | HDR → SDR: zscale → linear light → tonemap curve → bt709 (`--algo hable|reinhard|gamma|clip|linear`, `--peak` nits) | - |
| `telecine` | Pull 24p film up to interlaced NTSC fields (`--pattern 23` 3:2 pulldown, `--field tff|bff`) — inverse of fieldmatch | - |
| `premult` | Straight ↔ premultiplied alpha in place (`--mode premultiply|unpremultiply`); writes alpha-safe prores4444 | - |
| `dejudder` | Remove pullup judder (`--cycle 4` for 3:2 pulldown wobble) | - |
| `despill` | Remove green/blue screen spill from keyed edges (`--type`, `--mix`, `--expand`, `--at`/`--dur`) | - |
| `interp` | Motion-compensated interpolation: `--fps 60` upres, `--slow 0.5` smooth slow-mo | - | `--engine minterpolate|framerate` |
| `matrix` | Convert color matrices (`--from bt601 --to bt709` — fixes SD-gone-green; auto-detects source) | - |
| `legalize` | Clamp luma to broadcast-safe 16-235 (`--min`/`--max`, `--at`/`--dur`) | - |
| `levels` | Photoshop levels: `--in-min/--in-max/--out-min/--out-max` (crush rescue, matte fade) | - |
| `aberrate` | Chromatic aberration fringe — `--amount` px (VHS / glitch edge look) | - |
| `displace` | Warp picture by a second clip's luma map (heat ripple, liquid glitch): `--edge` wrap/mirror/smear/blank, `--at`/`--dur` window | - |
| `eqviz` | Apply EQ bands and render the response curve as video: `--bands "f=200 w=100 g=10 t=h"` (t=h/l/p shelf/peak), `--size` | - |
| `crossfade` | Blend two audio files with `--dur`s overlap (`acrossfade`) |
| `strip` | Remove all metadata + chapters, lossless `-c copy` |
| `frames` | Still dump every `--every`, `--at` seconds (`end` = last frame), `--count` even-spread → `stem_001.png…` (`--width`); `--untile CxR` splits each frame into tile stills (reverse a contact sheet) |
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
