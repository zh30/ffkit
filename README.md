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
| `probe` | Duration, size, codecs, channels, `av_desync_ms` lip-sync offset, `timecode` container TC, `has_alpha`, `tags` metadata audit, `streams` per-track table (index/kind/codec/lang/default), `rotation` display-matrix QC, `start_time` earliest stream start (negative/odd capture starts — repair via `remux --offset`) |
| `look` | Contact sheet (`--tiles`) or timestamps (`--at`, repeatable) |
| `cut` | Trim; lossless copy by default, `--accurate`, `--ranges`, `--drop` for frame-exact (bounds take `end`: `T-end` through the tail, `end-N` last N secs); `--fade N` softens the cut edges; `--black` auto-excises blackdetect stretches ≥0.3s (dead-air trim, the video twin of cutsil) |
| `concat` | Join N clips (all 42 named 4.4 xfade `--transition`s, comma list picks one per joint); `--level -14` loudnorms each clip first; `--gap N` inserts black+silence between clips; `--audio-fade N` fades each joint's audio (boundary fades — duration and sync preserved); `--list manifest.txt` reads clip paths from a file (relative paths resolve against the list's dir); `--chapters` stamps each clip as a titled container chapter (audiobook/podcast assembly) |
| `fit` | Frame / rotate / flip (9:16, 1:1, 16:9, …); `--fit blur` fills with a blurred backdrop , `--position` anchors the picture in the bars , `--strength` sigma |
| `extract` | Still frame or `--gif` clip (`--bounce` palindrome) | `--at` (`end` = last frame / last --dur sec, comma = one still — or GIF with `--gif` — per time), `--width`, `--fps` , `--loop` GIF repeat count, `--colors` palette size, `--alpha` pull the alpha channel out as a grayscale PNG (alpha-capable inputs only), `--audio` rip the audio track losslessly (stream copy; -o extension picks the container — pull a music/dialog track for editing; `--track N` picks which, `--all` dumps every track to `stem_aN.<ext>`), `--lang jpn` pick one track by language tag on `--audio`/`--subs` (comma refused — use `--all` for every track), `--from SEC`/`--to SEC` rip just a window of the audio track (clip a cue, still lossless), `--attachment N` rip attachment stream N back out to a file (fonts/files a `remux --attach` put in — mkv/webm), `--subs` pull an embedded subtitle track to a text file (-o .srt/.ass/.vtt picks the caption container; `--track N` picks the language, `--all` dumps every track to `stem_sN.<ext>`), `--chapter N` pull the Nth embedded chapter as its own file (1-based — audiobook/lecture segment export; `chapter --list` shows the numbering), `--keyframes` dump every I-frame as an image (`stem_%03d` auto-suffix — GOP-boundary stills for QC/timelapse/scene scouting), `--gif --transparent` keeps alpha in the GIF (Discord/Telegram stickers — needs an alpha input like prores 4444/qtrle), `--webp` animated WebP clip (smaller than GIF, alpha kept natively; `--lossless`, `--bounce` ok) |
| `overlay` | Image/video overlay; position/scale/`--tile`, `--fade`, `--angle`, `--at`/`--dur`, `--mode`, `--opacity` blend composite, `--loop` repeat short clips | Logo/picture-in-picture; `--tile N` draft watermark | Logo, watermark, picture-in-picture; `--border` PiP ring; `--mode` blend composites (screen/multiply/softlight/dodge/burn/exclusion/hardmix/negation/grainmerge/multiply128/and/xor/freeze/heat… 31 modes: Photoshop blends + grain composites + channel logic) |
| `broll` | Cutaway insert (`--insert` video, `--still` image, `--motion kenburns`) | Full-frame cutaway (`--insert --at --duration`); A-roll audio and length stay , `--audio` hear the insert (`--volume` its level) , `--position` PiP corner + `--scale`, `--border` ring the insert, `--opacity` ghost insert; `--at end` = tail cutaway, comma `--at` re-flashes it at several points |
| `caption` | Burn subtitles (`--srt`, `--chunk`, `--karaoke`, `--box-color` card, `--wrap` folds, `--from/--to` cue window, `end`/`end-N` ok) , `--fade` soft in/out, `--opacity` ghost captions |
| `broll` | Cutaway insert (`--insert` video, `--still` image, `--motion kenburns`) | Full-frame cutaway (`--insert --at --duration`); A-roll audio and length stay , `--audio` hear the insert (`--volume` its level) , `--position` PiP corner + `--scale`, `--border` ring the insert; `--at end` = tail cutaway, comma `--at` re-flashes it at several points |
| `caption` | Burn subtitles (`--srt`, `--chunk`, `--karaoke`, `--box-color` card, `--wrap` folds, `--from/--to` cue window, `end`/`end-N` ok) , `--fade` soft in/out, `--opacity` ghost captions |
| `broll` | Cutaway insert (`--insert` video, `--still` image, `--motion kenburns`) | Full-frame cutaway (`--insert --at --duration`); A-roll audio and length stay , `--audio` hear the insert (`--volume` its level) , `--position` PiP corner + `--scale`, `--border` ring the insert; `--at end` = tail cutaway, comma `--at` re-flashes it at several points |
| `caption` | Burn subtitles (`--srt`, `--chunk`, `--karaoke` (`--highlight` sung color), `--box-color` card, `--wrap` folds, `--from/--to` cue window, `end`/`end-N` ok) , `--fade` soft in/out, `--opacity` ghost captions |
| `loudnorm` | EBU R128 two-pass normalization (`--target spotify|podcast|broadcast`, `-I/--tp/--lra`); `--measure` reports loudness without writing (`--gate N` fails over N LUFS); `--dynamic` per-frame gain |
| `denoise` | Audio cleanup (`--strength`, `--highpass`, `--engine auto|wavel|fftdn` pick the denoiser, `--at/--dur` window); `end` ok, comma list = several windows | `--ref noise.wav` anlms adaptive cancel |
| `transcode` | h264/webm/`--preset gif` (`--fps`/`--width`/`--copy-audio`/`--colors`) | Presets `h264` / `webm` / `gif` / `hevc` / `mp3` / `aac` / `wav` / `flac` / `opus` / `av1` / `prores` / `dnxhd` / `proxy`; `--fps` retimes video too; `--vbitrate` peak bitrate cap, `--abitrate` audio bitrate (voice → 64k), `--preset prores`/`dnxhd` NLE delivery; `av1` preset; `--preset proxy` ≤540p veryfast x264 edit proxies; `--alpha` keeps transparency (webm/prores); `--range limited\|full` tags colour range; `--interlaced` marks output interlaced (il weave + tff flag — broadcast masters); `--interlace-mode weave` real temporal interlacing (60p→30i via tinterlace); `--field-order tff|bff|prog` relabels a wrong parity flag without re-weaving; `--ar 48000`/`--channels 1|2` resample + channel count on re-encode (broadcast 48k stereo, podcast mono); `--copy-video` stream-copies the picture while re-encoding audio (fix bad audio / repack — video filter flags don't apply) |
| `compress` | Fit a size budget (`--size 10MB` two-pass, `--target discord|whatsapp|gmail`); `--crf` quality one-pass, `--res` downscale to free bitrate |
| `deliver` | One-shot platform pack (Reels / TikTok / Shorts 9:16, `square` 1:1 grid, `youtube` 16:9, `xhs` 小红书 3:4 1080x1440, `wechat` 视频号 6:7 1080x1260, `douyin`/`kuaishou` 9:16 1080x1920, `bilibili` 16:9 1920x1080, `pinterest` 2:3 1000x1500, `x` 16:9 1280x720, `linkedin`/`vimeo`/`bluesky` 16:9 1920x1080, `threads` 4:5 1080x1350, `mastodon` 16:9 1280x720, `circle` 1:1 640x640 mono — Telegram video notes, `canvas`/`snapchat` 9:16 1080x1920 — Spotify Canvas loop / Spotlight, `weibo` 微博 16:9 1920x1080, `twitch` 16:9 1920x1080, `discord` 16:9 1280x720 — pair with `compress --size discord` for the 10MB cap, `shopify` 1:1 1080x1080 — product-page video, `amazon` 16:9 1920x1080 — listing video, `etsy` 1:1 1080x1080 — product gallery, `rumble` 16:9 1920x1080, `instagram`/`facebook` 4:5 1080x1350 — Meta feed portrait, `kick`/`vk`/`dailymotion`/`odysee`/`trovo`/`substack` 16:9 1920x1080, `line`/`triller`/`likee`/`moj`/`josh` 9:16 1080x1920 (JP messaging / music-video posts / SEA+IN short-video apps), `lemon8` 3:4 1080x1440, `niconico`/`soop`/`xigua`/`peertube`/`floatplane`/`nebula`/`chzzk`/`douyu`/`huya` 16:9 1920x1080 (niconico JP / SOOP ex-AfreecaTV / 西瓜视频 / PeerTube / Floatplane / Nebula / CHZZK / 斗鱼 / 虎牙), `weverse`/`kwai`/`snackvideo` 9:16 1080x1920 (Weverse Media / Kwai international / SnackVideo), `udemy`/`coursera`/`teachable`/`kajabi`/`patreon` 16:9 1920x1080 (course lectures & membership posts); −14 LUFS; `--fps 60` high-frame-rate uploads, `--crf` quality, `--subs file.srt` burns captions in one pass, `--channels 1` mono voice packs, `--preview SEC` renders just the pack's head for approval QC, `--logo mark.png` corner watermark burn-in (`--logo-position`, `--logo-opacity`), `--intro/--outro clip.mp4` bakes a channel bumper + CTA card onto every export (normalized to the platform canvas)); `--platform podcast` audio-only feed pack (m4a AAC 128k/48k, −16 LUFS spec, `--cover art.png` embedded Apple/Spotify art) or `--platform audiobook` (m4b AAC 96k — Apple Books/Audible); audio packs take `--chapters marks.txt` (container chapters from the YouTube-format `mm:ss title` list `chapter --yt` exports) and feed tags `--title/--author/--album/--genre/--comment`; `--to rtmp://…`/`tcp://`/`udp://` pushes the rendered pack straight to ingest (premieres) |

| `audiogram` | Waveform video ，`--mode phase`（aphasemeter 相位表）| `--mode`, `--scale`, `--split` channels, `--fscale` freq axis (spectrum), `--fps` rate, `--text`, `--bg`, `--progress` bar , `--subs` burn an .srt on it, `--from`/`--to` clip a segment (`end`/`end-N` ok), `--at a,b --dur N` one clip per point (`stem_N.mp4`); `--mode spectrum` bars, `--mode scope` lissajous vectorscope, `--mode cqt` piano-roll spectrum, `--mode spectro` scrolling spectrogram | `--mode spatial|volume|bitscope` meter scopes | `--mode monitor` stats viz | `--mode hist` amplitude histogram (clip/headroom QC) |

| `split` | Split by `--every`/`--at`/`--scenes`/`--size`/`--parts`/`--silence`/`--chapters`/`--black` (blackdetect dead-air → one part per non-black keep); `--copy` lossless stream-copy split (no re-encode, boundaries snap to next keyframe); `--subs` writes re-timed per-part .srt; `--fade N` softens each part's edges |
| `slideshow` | Still images → video montage (`--per` or `--dur` total runtime, `--fade`, `--transition`, `--motion kenburns`, `--audio` bed + `--volume`, `--size` canvas, `--bg` letterbox, `--fit` montage ends on the song's end, `--shuffle SEED` deterministic photo order — same seed = same order, `--sort name|mtime` orders a camera dump by filename or shoot time, `--list manifest.txt` curated still order from a manifest, `--titles a,,c` bottom caption strip per still — comma slots in the final slide order, empty entries skip a slide, `--audio-fade SEC` music-bed tail fade length (default 0.8 — longer lets the song ring out under the last still)) |
| `speed` | Change playback speed (`--factor`, `--at/--dur` (comma list = several windows), `--ramp` FROM,TO); `end` ok; `--fit SEC` retimes the clip to an exact length (auto factor) |
| `music` | Bed under speech with ducking (`--track`, `--at`/`--dur` window, `end` ok; comma `--at` = multi-entrance bed) |
| `key` | Green-screen composite: `--color` keyed out over `--bg` image/video (`--similarity`, `--blend`, `--despill`, `--mode luma` keys a luma band around `--threshold` for white-sky/dark-backdrop shots, `--mode chroma` = YUV-domain chromakey for wrinkled/uneven screens, `--at`/`--dur` window — comma list ok) | `--mode matte --mask` external grayscale matte → alpha (prores 4444) |
| `grid` | Multi-up collage (`--layout`, `--audio` pick, `--labels`, `--gap`, `--bg` gutter color, `--fill` crop instead of letterbox, `--time` mm:ss stamp on every tile, `--focus` hero layout — first input big left ~2/3, rest stack right) |
| `progress` | Progress bar on any edge, whole clip or a window (`--color`, `--height`, `--edge` bottom/top/left/right, `--at`, `--dur`, `--reverse` countdown-deplete, `--opacity` ghost) |
| `freeze` | Hold a frame (`--at`, comma list freezes at several points, `--dur`, `--end`, `--ease`, `--reverse`, `--zoom` push-in) |
| `censor` | Blur/mosaic a region (`--region x:y:w:h`, comma list for several spots; `--mode` pixel|blur|solid (black-bar redact), `--strength`, `--at`/`--dur` (comma list, needs `--dur`; `end` ok), `--shape circle` ellipse mask) |
| `bleep` | Tone over a word/segment: `--at`/`--dur` (comma list censors several spots; `end` ok), `--freq`, `--level` |
| `boomerang` | Forward + reversed replay (one loop, social trick) (`--times` repeat cycles) , `--at/--dur` bounces just that window — comma list for several spots (`end` ok) |
| `chapter` | Embed chapter marks at `TIME|TITLE` or `--import` a marks file (YouTube `H:MM:SS Title` lines ok); `--auto` / `--export` ffmeta / `--yt` description lines / `--cue` CUE sheet / `--podcast` Podcasting 2.0 JSON chapters (audiobook/podcast players) / `--lrc` synced-lyrics cue file / `--vtt` WebVTT chapter track for web `<track kind="chapters">` nav; `--spread N` even-grid marks (`--titles a,b,c` names them — uniform TOC for long episodes); `--csv` exports `H:MM:SS.mmm,Title` lines (Resolve/Premiere markers, spreadsheet edits — quoted titles ok); `--import` auto-reads .json/.cue/.lrc/.vtt/.csv files too; `--scenes` scdet scene-cut detection — marks every cut like `split --scenes` finds (auto-TOC for unmarked masters); `--list`; `--remove`; `--shift` re-times marks |
| `autocrop` | Detect & strip letterbox/pillarbox (`cropdetect` scan → `crop`; `--buffer N` keeps N px edge context) |
| `sheet` | Contact sheet grid (`--cols`x`--rows`, `--time` stamps, `--title` header, `--from`/`--to` window) |
| `sprite` | Seek-preview sprite sheets + WebVTT (`--every` secs, `--width` tile px, `--cols`x`--rows` per sheet, `--vtt` path, `--from`/`--to` bounds -- `end` ok) — hover thumbnails for video players |
| `pitch` | Shift pitch ±12 semitones, duration kept (`--at/--dur` window, comma list); `--formant` keeps the voice timbre (librubberband) |
| `cutsil` | Strip dead air at head+tail of an audio file (`--thresh` dB) |
| `channel --mode ms` | Decode mid/side-recorded stereo back to L/R (stereotools ms>lr) |
| `channel` | Channel surgery: `--mode dualmono|mono|swap|invert|mix51|pan|widen|split|ambience|mid|side|haas|surround|base|bal|bands|sync|earwax|stereowiden|merge` (`stereowiden` M/S widener, `--amount` crossfeed) (stereo→`_L/_R.wav` stems, M/S extract, haas widening, stereo→5.1 upmix); `--pan -1..1` pan / `bal` rebalance lopsided stereo / `base` stereo base (-1 mono fold, +1 wide) | `bands --freqs 300,3000` → `<stem>_bandN.wav` frequency-band stems (acrossover) | `sync --side right --cm 34` delay one side by mic distance (two-mic comb-filter fix, compensationdelay) | `earwax` headphone-oriented stereo widening | `merge --with B` interleave two tracks into one multichannel file (amerge — mono+mono → stereo host-L/guest-R podcast, keeps channels discrete unlike mix) |
| `eq` | Audio shelving EQ: `--bass`/`--treble`/`--presence`, `--preset` dB (`--at`/`--dur` window) , `--band` parametric F:G[:W], `--curve` freehand F,G;F,G line (firequalizer), `--graphic` 18-band classic EQ, `--tilt` warm↔bright, `--deemph riaa/cd/fm50/fm75` undoes vinyl/FM/CD pre-emphasis; `--shelf low|high:FREQ:GAIN` shelves, `--notch FREQ[:WIDTH]` kills a resonance, `--brickwall LO,HI` FFT brick-wall bandpass, `--lowpass`/`--highpass`/`--bandpass FREQ[:W]` resonant Butterworth (`--linear` = linear-phase FIR mastering cuts, no `--at`), `--subcut`/`--supercut FREQ` rumble & ultrasonic cleanup, `--superpass FREQ[:Q]`/`--superstop FREQ` razor order-10 band isolate/kill, `--allpass FREQ:W` phase rotator for lopsided vocals; `end` ok, comma list = several windows |
| `reverb` | Room ambience on a voice: `--size room\|hall\|cave`, `--wet` (`--at`/`--dur` window); `end` ok, comma list = several windows. `--ir file.wav` = convolution reverb from impulse-response packs (cathedral/plate), `--tail` rings past the end |
| `fx` | Audio FX rack: tremolo/vibrato/flanger/phaser/chorus/echo/lofi/radio/saturate/excite/bass/muffled/crystal/sub/crossfeed/autopan (`--kind`, `--strength`, `--at`/`--dur`); `end` ok, comma list = several windows | `--kind ringmod` TRUE ring modulation (amultiply + sine carrier, `--strength` sweeps 25-500Hz) | `--kind crush` bitcrusher (bits+sample-rate destruction) | `--kind fshift` frequency shifter (metallic alien voice, 50→2000Hz) | `--kind contrast` dynamics tilt (>0.5 punch, <0.5 level) | `--kind wah` auto-wah (asendcmd sweeps a resonant equalizer peak 350→2700Hz, `--strength` sets the LFO rate) |
| `rotate` | 90/180/270 or mirror: `--deg`/`--flip`, free `--angle` tilt, `--at`/`--dur` windowed tilt (comma list) |
| `delogo` | Blend out a burned-in logo box: `--x --y --w --h` or `--regions x:y:w:h,...` for several spots; `--at`/`--dur` for a window, `--at end` the tail (`--soft` removelogo, `--shape circle` elliptical mask) | `--image mask.png` drawn-mask removal | `--find logo.png` auto-locates it (find_rect, first 15s) — no coordinates | `--find + --track` follows a MOVING mark every frame (cover_rect blur) |
| `meta` | Container tags (`--title`/`--artist`/`--album`/`--genre`/`--date`/`--track`/`--disc`/`--composer`/`--bpm`/`--lyrics file.lrc`/`--copyright`/`--comment` + `--album-artist`/`--show`/`--season`/`--episode`/`--network` TV/podcast feed set + `--creation-time`/`--location` archive & geo stamps + `--media-type` iTunes kind + `--gapless` + `--description`/`--synopsis` episode notes + blurb + `--hd` iTunes HD badge + `--lang-audio eng,jpn`/`--lang-subs eng,fra` per-track language tags in track order — blank slots skip + `--title-audio`/`--title-subs`/`--title-video` per-track display titles — players name the track, not "Track N" (multi-cam angle labels; stream titles need mkv — mp4 drops them)) + `--rotate`, `--clear`, stream-copy |
| `subs` | Extract (`--stream`, `--all`)/burn/mux subtitles (`--shift` (±N; `--from`/`--to` bounds it)/`--merge`/`--rate`, burn style + `--outline`/`--box` plate/`--align`/`--margin` px/`--from`/`--to` window (`end`/`end-N` ok), `--safe`); `--convert` .srt↔.vtt; `--case` cue text; `--burn-si N` burns embedded track N; `--encoding gbk` legacy charsets; `--sort`/`--fix-overlaps`/`--dedupe` .srt cue hygiene (reorder, clamp overlaps to the next start, drop repeats — broken-export repair) + `--min-dur SEC` flash-text floor (caps at next start) + `--cps N` caption-speed gate (`over_limit`/`worst_cps` — readability spec) + `--max-lines N` line-count report (broadcast spec is 2) + `--replace OLD,NEW` cue-text find/replace + `--strip-speakers` `[NAME]`/`ALL-CAPS:` label strip (auto-transcripts) + `--wrap N` rewrap at N chars/line + `--append b.srt` joins a second .srt where the first file's cues end (post-`concat` transcripts) + `--convert` to `.txt` plain-text transcript export or `.ass` minimal styled ASS (Aegisub/anime-pipeline handoff — `.ass` input works too, Format-line column order honoured) + `--split 3,8` cuts an .srt at comma timestamps into `stem_0.srt`/`stem_1.srt`… parts (cues re-timed per part — split a transcript to match `split`/`cut` parts) + `--resync O1,O2,N1,N2` two-point linear resync — old times O1,O2 land on new N1,N2 (offset+drift in one pass, for subs authored against a different cut) |
| `thumb` | One-frame cover grab (`--at` — `end` = last frame, comma `--at` = one still per time / `--frame`, `--count` N stills (`--from`/`--to` bounds the spread, `end`/`end-N` ok), `--width`) → jpg/png/webp; `--scenes` stills at cuts | `--best` representative still |
| `solid` | Solid-color clip card (`--color`, `--size`, `--dur`, `--fps` rate; silent stereo optional) (`--gradient` animated, `--noise` grain) , `--text` end-card text (`--wrap` folds, `--align` lines) |
| `replace` | Swap the audio track (`--mix`, `--duck`, `--fade`, `--loop` short beds, `--at`/`--dur` window — comma `--at` several) — or `--video` to swap the picture and keep the audio |
| `jumpcut` | Cut silence inside a talking-head take |
| `rough` | Map speech islands in a long take (`--json`); `-o` assembles (`--merge N` merges keeps closer than N s, `--by-scene` splits keeps at scene changes) (default encodes only keeps; `--copy` is lossless/keyframe-sloppy) |
| `cover` | Cover still (`--at` — `end` = last frame, comma = one per beat, `--blur` ambient pad, `--size` canvas — default 1080x1920) |
| `fade` | Video and audio fade (`--in` / `--out`, `--color` e.g. white, `--dip T` scene-change dip — comma list dips at every mark; `--curve` audio fade shape) |
| `title` | Hook/title card (`--text` or `--file notes.txt`, `--at` — `end` = end-card, comma list flashes at several marks, `--fade`, `--outline`, `--box` backplate, `--wrap` long hooks, `--align` lines, `--opacity` ghost, `--margin` px corner inset) |
| `loop` | `--times` or `--until` seconds | Repeat the clip N times (Shorts replay length) (`--from`/`--to` loops only a section, `end` ok, `--fade` seamless joints) |
| `stabilize` | Handheld deshake — `--rx`/`--ry` radius, `--edge` fill (blank|original|clamped|mirror) | `--engine vidstab` two-pass vid.stab (steadier on real shake), `--smoothing` frames |
| `reverse` | Play picture and sound backwards |
| `grade` | `--preset` look, `--contrast/--saturation/--brightness/--gamma/--hue/--lut/--grain/--warm/--exposure` (EV stops), `--skin` warmth ，含 `--kelvin` 开尔文白平衡、`--split` 青橙分调 | Presets `cinematic`/`vivid`/`vintage`/`soft`/`sepia`/`teal`/`noir`/`bleach`/`neon` stack under the sliders; `--lut look.cube` applies a 3D LUT, `--lut look.png` a HALD image LUT (haldclut — Darktable/RawTherapee exports); `--skin -1..1` warms faces only (selectivecolor reds), `--vibrance -1..1` smarter saturation (boosts muted, protects skin), `--curve "x/y …"` freeform master curve (matte fade, S-curve) | `--at`/`--dur` `--wash C` colour veil | `--match ref.mp4` histogram match | `--lut` accepts 1D LUTs | `--color-from ref` borrow chroma | `--mix "rr,rg,rb,…"` 3x3 channel matrix (colorchannelmixer) |
| `zoom` | Punch-in (`--factor 1.25`, `--center X,Y` target; `--at`/`--dur` window — comma list for several, `end` ok) | `--out`
| `sharpen` | Unsharp mask, whole clip or a window (`--amount`, `--at`, `--dur`) `--engine unsharp\|cas\|halo` (halo = maskedclamp, no overshoot) |
| `vignette` | Corner darkening, whole clip or a window (`--angle`, `--at`, `--dur`) |
| `bw` | Desaturate to B&W, whole clip or a window (`--at`, `--dur`, `--strength` keeps muted color) ，`--weights r,g,b` 胶片通道权重，`--cut 0-1` hard threshold (xerox/graphic B&W) |
| `volume` | Gain ±dB; `--at/--dur` limits it to a window — comma list covers several spots (needs `--dur`) (platform loudness is `loudnorm`) |
| `blur` | Full-frame or windowed gaussian blur (`--sigma`, `--at`/`--dur` comma list; `--at end` = tail) | `--engine gblur|directional|box|avg` (directional `--angle` streaks; box = fast blocky kernel; avg = lightest area-average) |
| `trail` | Motion trails: `--mode echo` ghost smear behind movement (`--frames` 2-16, `--at`/`--dur` window), `--mode light` bright-pixel persistence (`--decay` 0.5-0.99) | `--mode diff` motion ghost |
| `glitch` | Datamosh-style glitch: `--strength` 0.5-20 drives RGB channel shift + temporal noise | `--engine planes` channel rotation | `--engine swapuv` chroma flip | `--engine stutter` frame jitter | `--engine pixels` block scatter | `--engine swaprect` quadrant swap | `--engine random` frame-order scramble |
| `bars` | SMPTE test card: `--size`/`--dur`/`--hd`/`--tone` (1kHz bed), for QC slates and leader | `--kind sd|pal100|pal75|rgb|yuv|allrgb|allyuv|mptest|testsrc` other broadcast patterns (`allrgb`/`allyuv` = full color-cube QC sweeps, `mptest` = encoder-torture cycle, `testsrc` = all-in-one animated calibration card) |
| `scope --mode hist` | Rolling temporal histogram of luma — color/exposure drift QC over time |
| `scope` | QC scope overlay: `--mode vector|wave` in a corner (`--position`, `--size` fraction), `--at` windows | `--mode mvs` MV overlay | `--mode data --x/--y` hex readout | `--mode qp` macroblock QP overlay | `--mode pix` magnified pixel grid | `--mode osc` XY video oscilloscope | `--mode drift` luma-drift curve (exposure-ramp QC) | `--mode loud` loudness-over-time curve (ebur128+adrawgraph) | `--mode cie` CIE-1931 gamut map | `--mode palette` color-swatch grid (GIF/8-bit palette QC) | `--mode graph` live filtergraph stats card (graphmonitor — frames in/out + queue, encode-pipeline debug) | `--mode safe` full-frame broadcast safe-area guides (90% action yellow / 80% title red + center cross — composition QC) |
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
| `stack` | Median-stack 3+ locked-off videos of the same scene — removes objects present in <half the inputs (tourists, sensor noise); `--percentile`; audio from input 1 | `--mode median|max|min|mean` (max = star trails, min = darkest, mean = exposure averaging — `--weights` comma list per input, auto-normalized: `3,1` = 75%/25%) |
| `stereo` | 3D format convert — SBS↔anaglyph↔interleaved |
| `sonify` | Play an image/video as sound — spectrumsynth scans it like a spectrogram (bright pixels = loud harmonics); `--dur`/`--speed`/`--sample-rate` |
| `tmedian` | Temporal median — erase anything visible <half the window: moving people/cars on tripod shots, rain streaks (`--radius` history frames, `--percentile`, `--at` window; output loses 2*radius edge frames) |
| `dedup` | Drops near-duplicate frames via `mpdecimate` — shrinks static stretches, `--frac` sensitivity |
| `repair` | Swap bad/glitched frames for a frame from a reference take (`--ref`, `--at`/`--dur` damaged stretch, `--ref-at` clean frame — freezeframes) |
| `audiogram --mode cqt` | Constant-Q music spectrum (`showcqt`) — piano-roll spectrum look for music clips |
| `audiogram --mode spectro` | Scrolling spectrogram (`showspectrum`) — colour time/frequency roll |
| `scan` | QC report: black stretches, frozen frames, black-frame hits + strobe `flash_frames`/`flash_max_badness` + interlace verdict (idet) + stereo `phase_corr` (~-1 = mono-collapse) — JSON extras; writes no media , audio peak/mean dB (`audio_max_db`, `audio_mean_db`) | +blur QC | `--scenes` scene-cut timestamps | `luma_min/max` + `illegal_luma` (signalstats broadcast-range QC) | `noise_floor`/`noisy` bit-plane noise budget QC | `has_cc`/`cc_lines` EIA-608 closed captions | `crop_hint`/`letterboxed` cropdetect letterbox QC | `vfr`/`vfr_ratio`/`vfr_frames` variable-frame-rate QC | `--dupe REF` MPEG-7 signature duplicate/re-upload match | `--text` OCR burned-in text (`text`/`text_frames`/`text_confidence`) | `rg_gain_db`/`rg_peak` ReplayGain tags | `--motion` VMAF motion score (`motion_avg`/`motion_max` — bitrate-budget QC) | `--timecode` VITC timecode readout (`vitc`/`vitc_tc`/`vitc_frames` broadcast-master QC) | `--bbox` content bounding box (`content_detected`/`content_box`/`content_fill` — any uniform background, not just black) | `sat_mean`/`hue_mean`/`y_mean` signalstats tone QC (washed-out / color cast / programme brightness) | `--deadair DB` dead-air map (`deadair_secs`/`deadair_ranges` — podcast/talking-head silence QC) | `--loud` EBU R128 summary (`loud_i`/`loud_lra`/`loud_tp` — platform spec gate; works on audio-only files), `hdr`/`wide_gamut`/`color_space`/`color_primaries`/`color_transfer` HDR & gamut QC from container colour tags | `--gop` keyframe-interval QC (`keyframes`/`keyframe_times`/`gop_max_sec`/`gop_avg_sec`/`gop_max_frames` — packet flags, no decode) | `--hash` decoded-frame checksums to `<input>.framemd5` (archive-ingest integrity manifest) | `timecode` container timecode QC (no decode) | `--bitrate` video-rate curve from the packet map (`bitrate_mean_mbps`/`bitrate_peak_mbps`/`bitrate_spike_at` — peak-rate ingest specs, no decode) | `av_desync_ms` audio/video start_time offset (container-level lip-sync QC — no decode; pair with `remux --audio-delay`/`--video-delay` to fix) |
| `smooth` | Edge-preserving beauty/skin blur (`--engine` smartblur/bilateral; `--strength`, `--at`/`--dur`) | `--engine uspp` postproc deblock | `--engine pp7` light postproc | `--engine yaep` edge-preserving | `--engine spp/fspp` light deblock | `--engine sab` shape-adaptive blur |
| `upscale` | Up-res footage: `zscale` spline36 (better than lanczos) + light unsharp, `--factor` 1.05-4 (2 doubles dims), `--strength` edge acuity | `--engine spline|xbr|two-xsai` (pixel-art integer scalers) | `--engine hqx` hq2x/3x/4x pixel-art scaler | `--engine epx` EPX 2x/3x scaler |
| `v360` | Reframe 360 footage to flat (`--in` equirect/fisheye/dfisheye/cubemap/EAC/barrel/half-equirect, `--yaw`/`--pitch`/`--fov`, `--size`) |
| `perspective` | Deskew a filmed screen/whiteboard: `--points x0,y0,x1,y1,x2,y2,x3,y3` (TL,TR,BL,BR quad in source, px), `--interp linear|cubic` |
| `wb` | Auto white balance / cast removal: `--strength` 0..1, `--independence` 0 keeps the grade (contrast only), `--smooth` temporal frames, `--at`/`--dur` window | `--engine greyedge` grey-edge illuminant fix (gentler on graded footage) |
| `shear` | Italic-style picture slant: `--x`/`--y` shear factors -2..2, `--fill` edge color, `--interp nearest|bilinear`, `--at`/`--dur` window |
| `gen` | Generative animated backgrounds from lavfi sources (no input): `--pattern mandelbrot` (endless zoom) `|gradients` (drifting palette — `--colors` up to 8, `--seed`, `--speed`) `|life` (cellular automaton, `--rule`) `|sierpinski` (fractal), `--size`/`--fps`/`--dur` | `--pattern noise|tone` audio beds (`--color` white/pink/brown/blue/violet/velvet) | `--pattern sweep` 20Hz→`--freq` speaker-test chirp | `--pattern silence` digital-black bed (anullsrc — silent padding) | `--pattern hald` identity HALD LUT PNG (`--level`, default 8 — grade it in an editor then feed `grade --lut`) |
| `sharpen --engine cas` | Contrast-adaptive sharpening — crisper edges without unsharp halos, `--amount` |
| `equalize` | Auto-contrast via `histeq` for flat/washed footage, `--strength`/`--intensity`/`--at` window |
| `pick` | Dominant-color report at a timestamp: mean hex + 3x2 zone swatches (JSON only) |
| `diff` | Visual diff between two clips: amplified difference blend, `--side` shows reference | `--mode mask --threshold` bare change-mask QC |
| `selective` | Keep one color, desaturate the rest: `--color` + `--similarity`, `--blend` edge feather, `--engine chroma` chromahold for saturated hues, `--at` windows |
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
| `leveler` | Compress dynamics (`--preset`, `--engine speechnorm` adaptive speech normalize, `--engine limit` alimiter brickwall ceiling, `--engine compand` single-band transfer curve, `--at/--dur` window); `end` ok, comma list = several windows |
| `gate` | Noise gate — silence below `--threshold` dB (`agate`) (`--preset voice|podcast|studio`, `--at/--dur` window); `end` ok, comma list = several windows |
| `silence` | Insert `--dur` secs of silence at `--at` (comma list pads several points) or `--end`; `--detect` reports silence ranges as JSON |
| `vocal` | Remove/isolate center vocals (`--mode`, `--at/--dur` window, `--amount` strength); `end` ok, comma list = several windows |
| `remux` | Container swap, no re-encode (`-c copy` + faststart on mp4/mov); `--audio` rips the track, `--video` video-only repack, `--aspect 16:9` display-AR fix, `--frag` fragmented MP4 (moof/mfra — playable while still being written, HLS/DASH pipelines), `--no-subs` drops subtitle/data streams (clean deliverable), `--itsscale R` retime the whole container without re-encoding (re-stamps timestamps ×R — 1.042 PAL 25→24 pull-down, 0.96 film→PAL speed-up; audio pulls with the picture), `--offset SEC` set the container start_time (repairs negative/odd capture starts), `--from SEC`/`--to SEC` lossless trim (keyframe-accurate — repackage just a segment, no re-encode), `--lang jpn` keeps only audio tagged with that language (multi-language releases; with `--audio` rips just that track), `--default-audio N` makes audio track N the player default, `--cover pic` attaches feed art as an attached_pic stream (audio rips get cover art, videos get a thumbnail poster), `--chapters marks.txt` embeds container chapters from a YouTube-format list (the same file `chapter --yt` exports — Apple Podcasts/Books seek stops), `--title`/`--artist`/`--album`/`--genre`/`--comment`/`--date` writes library tags on the repack, `--lang eng,jpn` comma list keeps every listed dub, `--strip-meta` wipes inherited container tags (privacy — combine with tags to retag in one pass), `--no-cover` drops attached_pic cover-art streams, `--encrypt` CENC AES-CTR on the repack (mp4/mov only — ClearKey/Widevine DRM prep, `--key`/`--kid` 32-hex or auto-random, both reported in JSON), `--audio-delay SEC` shifts the audio track against the video — lip-sync repair without re-encoding (negative advances audio), `--video-delay SEC` the other half of the repair for capture cards that lag the picture (positive delays video, negative advances it; mutually exclusive with --audio-delay), `--tag hvc1` rewrites the codec tag so HEVC mp4s play in QuickTime/Safari (mp4/mov only), `--attach f` embeds a binary attachment stream into mkv/webm (subtitle fonts ship inside the file; repeatable), `--timecode HH:MM:SS[:FF]` writes a start TC (mov/mp4 tmcd track, mkv TIMECODE tag — dailies matching a slate), `--default-sub N` picks the default subtitle track), `--audio-order 1,0` keeps + reorders audio tracks (unlisted tracks drop — put the program mix on track 0 for players that only read the first track), `--sub-lang fra` keeps only subtitle tracks tagged with that language (comma list keeps several — multi-subtitle releases), `--sub-order 1,0` keeps + reorders subtitle tracks by index (audience captions first on multi-sub releases — unlisted tracks drop), `--keep 0,3` keeps ONLY the listed absolute stream indices (`probe.streams` lists indices — the escape hatch when the per-type orders can't express the pick), `--decrypt HEX` reads CENC-encrypted sources (32-hex key — round-trips with `--encrypt` for key rotation), `--copy-ts` preserves input timestamps verbatim (capture pipelines that need wall-clock pts kept — conflicts with the ts mutators), `--video-order 1,0` keeps + reorders video tracks by per-type index (multi-angle/multi-cam files — hero angle first, unlisted tracks drop; pairs with `--video` for the angle rip), `--forced-sub N` flags subtitle track N as FORCED (film-style forced captions — players auto-show them for the audience's language; mkv/webm only, mp4 can't express the flag), `--default-video N` picks the default video track on multi-angle/multi-cam files (`--default-audio`/`--default-sub` for the other kinds), `--no-video` drops the video streams but keeps audio/subtitles/cover/attachments (audio deliverable with its artwork intact — unlike `--audio` which rips only the track)) |
| `meme` | Top/bottom meme captions (`--outline`, `--at/--dur` window — comma list for several spots; `--at end` tail) , `--position` text block top/center/bottom; `--wrap` folds, `--align` line alignment, `--fade` edge fades with --at/--dur, `--opacity` ghost text |
| `voice` | Podcast voice one-shot: `agate`→`acompressor`→`loudnorm` (`--threshold`, `--lufs`, `--at`/`--dur` window, `end` ok, comma list = several windows) |
| `deinterlace` | Fix interlaced footage (`--mode`, `--parity` field order, `--engine` yadif/bwdif/estdif/kerndeint) ，`--engine` 含 `detelecine`（确定节奏反电视电影）、`mcdeint`（运动补偿）| `--engine w3fdif` Weston 3-field | `--engine separate` 50i→50p field-per-frame (smooth slow-mo source) | `--engine pullup` IVTC telecine reversal | `--engine phase` field-order swap (wrong-parity captures) | `--engine field` top-field extract (half-height, fastest preview) |
| `dedust` | Remove dust specks / hot pixels: `--size` 1-4, bright specks by default, `--dark` for dark ones; morphology (erosion/dilation), not a blur; `--engine temporal` tlut2 kills one-frame sparkles / VHS dropouts | `--at`/`--dur` |
| `extend` | Stretch edge pixels to fill border strips: `--left/--right/--top/--bottom` px, `--mode smear|mirror|fixed|reflect|wrap|fade` | - |
| `tonemap` | HDR → SDR: zscale → linear light → tonemap curve → bt709 (`--algo hable|reinhard|gamma|clip|linear`, `--peak` nits) | - |
| `telecine` | Pull 24p film up to interlaced NTSC fields (`--pattern 23` 3:2 pulldown, `--field tff|bff`) — inverse of fieldmatch | - |
| `premult` | Straight ↔ premultiplied alpha in place (`--mode premultiply|unpremultiply`); writes alpha-safe prores4444 | - |
| `dejudder` | Remove pullup judder (`--cycle 4` for 3:2 pulldown wobble) | - |
| `despill` | Remove green/blue screen spill from keyed edges (`--type`, `--mix`, `--expand`, `--at`/`--dur`) | - |
| `interp` | Motion-compensated interpolation: `--fps 60` upres, `--slow 0.5` smooth slow-mo | - | `--engine minterpolate|framerate` |
| `matrix` | Convert color matrices (`--from bt601 --to bt709` — fixes SD-gone-green; auto-detects source; `--engine colorspace` also maps primaries + transfer, the bt2020↔709 path) | - |
| `legalize` | Clamp luma to broadcast-safe 16-235 (`--min`/`--max`, `--at`/`--dur`); `--flash` damps photosensitive-epilepsy flash cuts (scan reports them as flash_frames) | - |
| `levels` | Photoshop levels: `--in-min/--in-max/--out-min/--out-max` (crush rescue, matte fade) | - |
| `aberrate` | Chromatic aberration fringe — `--amount` px (VHS / glitch edge look) | - |
| `displace` | Warp picture by a second clip's luma map (heat ripple, liquid glitch): `--edge` wrap/mirror/smear/blank, `--at`/`--dur` window | - |
| `eqviz` | Apply EQ bands and render the response curve as video: `--bands "f=200 w=100 g=10 t=h"` (t=h/l/p shelf/peak), `--size` | - |
| `crossfade` | Blend two audio files with `--dur`s overlap (`acrossfade`) |
| `strip` | Remove all metadata + chapters, lossless `-c copy` |
| `frames` | Still dump every `--every`, `--at` seconds (`end` = last frame), `--count` even-spread, `--nth N` every-Nth-frame grid → `stem_001.png…` (`--width`); `--untile CxR` splits each frame into tile stills (reverse a contact sheet) |
| `countdown` | Overlay a counting leader (`--from` up to 600, `--beep` + `--tone` Hz, `--text`, `--position`, `--bg` numeral plate, `--format` mm:ss/h:mm:ss, `--opacity` ghost, `--target HH:MM` counts down to a wall-clock premiere time — 1s per count, up to 10 min ahead, `--utc` reads the target in UTC for crews on a shared clock) |
| `invert` | Full-frame or windowed color inversion (`--at`, `--dur` — comma list ok) |
| `mix` | Blend two sources (`--vol-a/--vol-b`, `--at/--dur` window — comma list for several entrances, `end` ok, `--loop`, `--duck` sidechain bed under voice, `--gate` hard-mutes the bed instead — talk-show) , `--normalize`, `--fade` bed edges |
| `mute` | Drop the audio track, stream-copy the rest , `--at/--dur` silences only that window (comma list covers several spots, needs `--dur`; `end` ok) |
| `timer` | On-screen running clock (`--position`, `--format ms`, `--box-color` card)  (`--format`, `--box-color`, `--down` countdown, `--start` seed the readout) — `--at` accepts `end`; `--opacity` ghost HUD; `--tc HH:MM:SS:FF` burns a running timecode (dailies/review copies; `;` before FF = drop-frame display intent), `--clock` burns the local wall clock HH:MM:SS (event/sports overlays), `--date` burns the local calendar YYYY-MM-DD (air-date/archive overlay — static; prefixes the --clock readout), `--utc` reads the clock/date in UTC instead of local time (broadcast logs, cross-timezone crews) |
| `hls` | Web-ready HLS (`--seg`, `--single`, `--copy`, `--ladder` ABR, `--audio-only` podcast streams, `--fmp4` CMAF, `--poster` writes poster.jpg, `--poster-at T` picks the frame, `--encrypt` AES-128 + `key.bin`/`key.info` (custom `--key HEX`, `--key-uri URI`) for private/paywalled streams, `--live` sliding-window playlist — players join mid-write, newest `--live-window N` segments only, no endlist, `--start N` resumes segment numbering after a restart, `--epoch` seeds it from the epoch clock for 24/7 channels, `--date` stamps EXT-X-PROGRAM-DATE-TIME on segments, `--discontinuity` marks a feed restart, `--time-names` names segments by wall-clock time (archive recordings), `--independent` tags EXT-X-INDEPENDENT-SEGMENTS + forces a keyframe per segment (seek/trick-play VOD), `--base-url URL` prefixes every playlist segment entry — serve segments from a CDN while the manifest stays local) |
| `dash` | DASH package → manifest.mpd + init-/seg-*.m4s (`--seg`, `--copy` repack, `--single` one byte-range file per representation, `--webm` vp9+opus segments, `--window N` sliding manifest for live writes, `--ladder 1080,720,480` ABR — N video Representations at tiered bitrates in one AdaptationSet, `--streaming` moof-per-frame fragments (low-latency DASH prep), `--sidx` global SIDX index in the `--single` byte-range file — HTTP range seeking) |
| `live` | Push the clip live to an ingest endpoint: `--to rtmp://…`/`rtmps://`/`tcp://`/`udp://`/`srt://` (SRT/UDP ride MPEG-TS) (real-time `-re` pacing, x264/aac), `--codec hevc` contribution-grade HEVC (MPEG-TS transports only), `--subs file.srt` burns live captions in one pass, `--loop` forever (24/7 streams, premiere replays), `--vbitrate`/`--abitrate`, `--crf 0-51` constant-quality instead of -b:v (conflicts with --vbitrate/--maxrate/--bufsize), `--scale WxH` downscale a big master to ingest size, `--fps N` cap the output rate, `--record file.mp4` archive while streaming (tee — encode once, mux twice), `--until SEC` auto-stop, `--list` concat-manifest rotation (24/7 channel; `--loop` = infinite rotation), `--test` generated card + tone (verify the key before showtime), `--slate card.png --slate-dur SEC` starting-soon card ahead of the feed (premieres), `--card art.png` persistent still video for audio-only sources (24/7 lofi-radio streams), `--overlay bug.png` channel bug in a corner (`--overlay-position tl|tr|bl|br`, `--overlay-opacity`), `--restream url` pushes to a second ingest at the same time — multistream in one encode, `--gop N` keyframe interval for ingest specs (YouTube wants a keyframe ≤2s), `--preset` x264 speed/quality, `--vertical` letterboxes onto a 1080x1920 canvas — TikTok/Reels live, `--maxrate 4500k`/`--bufsize` CBR caps for ingest rate specs (Twitch ≤6000k, bufsize defaults to 2x maxrate), `--start T` begins streaming T seconds into the source — skip a long event's dead head on replay, `--title "Show"` writes the show name into the FLV/TS metadata + --record archive (ingest dashboards display it), `--audio-only` drops the video path — audio podcast/radio push from any source (aac-only feed), `--no-audio` drops the audio path — silent ambience/surveillance feeds, `--rw-timeout SEC` aborts the push when the ingest stalls (socket stall watchdog — distinct from the global `--timeout` kill timer; single-destination pushes only), URL input relays a live source — `live rtmp://…`/`udp://…`/`http://…` re-streams a pull feed to another ingest (--list/--start/--loop/--slate are file-only and refuse)) |

| `qa` | Measure quality loss vs a reference: PSNR + SSIM + MSAD + VIF (`--metric`) |
| `conform` | Resize/fps/loudnorm to spec in one pass; `--size WxH`, `--fps 30`, `--lufs -14`, `--crf`, `--pad` letterbox color + `--anchor`, `--blur` blurred fill, `--hold SEC` clone-last-frame end card + `--hold-start` pre-roll (audio silence-padded), `--even` floors odd pixel dims (phone captures → x264-safe); `--ar HZ` resample rate (48k broadcast default; 44100 podcast/CD, 96000 masters), `--channels 1` mono masters |
| `sync` | Shift audio ±ms to fix A/V sync (`--ms`) |
| `align` | Auto-sync a second recording to a reference by audio cross-correlation — multi-cam/external recorder (`--max-lag`) | `--check` reports `offset_ms`/`direction` without rendering (sync QC) |
| `scroll` | Rolling end credits: text rolls bottom→top (`--text`/`--file`, `--at`, `--dur` or `--speed` px/s, `--size`, `--color`, `--font`, `--align` left/right, `--wrap` folds); `--mode ticker` news crawl with `--bg` opaque bar; `--at` comma list replays the roll, `end` ok; `--opacity` ghost credits |
| `insert` | Splice a clip mid-video (`--at`, comma list splices at several points, `end` appends; `--dur` cap; `--transition` any xfade `--duration` S crossfades both joints; `--replace` overwrites the span under the clip — patch a flub, output keeps base duration; `--at chapterN` splices at embedded chapter N's start (`chapter --list` numbering — patch one section in a chaptered file)) |
| `multicam` | Two-camera angle switching across an aligned pair: `--at t1,t2,...` flips angle (`end` = tail switch); `--align` auto-syncs cam B to A by audio xcorr first (no separate `align` pass — needs broadband in-sync audio; pure tones don't correlate); `--keep-audio` stays on cam A, `--transition` xfade switches |
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
