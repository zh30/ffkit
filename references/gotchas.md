# Gotchas

Read this when a command looks right but the output is wrong, or ffmpeg errors on a filter.

## Rough cut vs jumpcut

`rough` is for a long take after recording: audio-only `silencedetect` (16 kHz mono, no video decode), then either list the islands or encode **only the keep windows** and concat. `jumpcut` re-encodes the whole filtergraph — fine for a short social clip, slow on a 20-minute 4K dump. `rough --copy` is stream-copy (fast, cuts sit on keyframes).

## Music duck / sidechaincompress

`music --duck` runs `aformat=sample_fmts=dbl` on **both** `sidechaincompress` inputs, after `volume`. That filter only accepts packed double; Ubuntu/apt ffmpeg will not insert the converter (Homebrew 9 often will). Do not aformat-then-volume — `volume` would convert away from dbl again.

## Stream copy vs re-encode

`cut` without `--accurate` is keyframe-accurate (`-c copy`). The first frame may sit a few hundred milliseconds before `--start`. Frame-exact cuts need `--accurate` (re-encodes).

Concat **copy** (the concat demuxer) requires identical codec, size, and fps. ffkit falls back to a concat filter and re-encodes when probes disagree.

## Even dimensions and yuv420p

libx264 + yuv420p rejects odd widths/heights. `fit` and `transcode --preset h264` even the size. Raw `ffmpeg` scale expressions should use `trunc(iw/2)*2`.

## GIF

A single-pass `ffmpeg -i in out.gif` looks dirty. `transcode --preset gif` runs palettegen + paletteuse. GIF has no audio; do not probe for it.

## Subtitles

`caption --mode burn` rasterizes SRT cues and `overlay`s them (works without libass / `drawtext`). Default `--safe social` sits the burn-in **above the bottom 20%** (TikTok/Reels chrome; top 15% is also dead). `--safe off` is the old 15% bottom margin. Pass `--font` if no Arial/DejaVu is on disk. `--mode mux` keeps toggleable soft subs and does not show in-feed on mute.

## iPhone VFR

`probe` sets `variable_frame_rate_suspected` when `r_frame_rate` and `avg_frame_rate` disagree. Re-encodes (`fit`, `transcode`, `--accurate` cuts) conform to constant fps. Stream copy keeps VFR.

## Audio-only files

`look`, `fit`, `overlay`, `caption`, `transcode --preset gif` need a video stream and will fail with `input has no video stream`. Extract/loudnorm/cut/concat on wav/m4a/mp3 are valid.

## Overwrite

Source path as `-o` is always refused. An existing output is refused unless `--overwrite`. Never point `-o` at the user's original.

## Web MP4

`transcode --preset h264` sets `+faststart` and yuv420p. A graph that writes mp4 for the browser should do the same.


## Target size

`compress --size` solves bitrate from probe duration: `usable = target×8×0.98`, audio paid first (`--audio-kbps`, default 96), the rest is `-b:v`. Video runs **two-pass** (`-passlogfile` in a tempdir; pass 1 sinks to `-f null -`, portable). Under 64 kbps video it refuses — raise the size or cut first. wav/flac outputs ignore bitrate and are refused for audio-only.

## Denoise

`denoise` chains `highpass` (rumble) + a noise stage: `afwtdn` (wavelet, `sigma` 0.02–0.08 from `--strength`) when the ffmpeg build has it (ffmpeg ≥5.1), else `afftdn` with the floor raised (`nf=-20`). Never bare `afftdn` — its default `nf=-50` measures ~0 dB of actual noise reduction; `anlmdn` segfaults on ffmpeg 9.0.1 builds. `--video` adds `hqdn3d` degrain and re-encodes the picture (default keeps `-c:v copy`).

## replace (audio swap)

`replace IN --audio NEW` keeps the picture on `-c:v copy` and pads/trims the new audio to the video length (`apad,atrim` — no `-shortest` needed, so a short bed gets silence and a long one is cut). `--audio-offset +S` delays the new track (`adelay`), `-S` trims its head (input `-ss` before `-i`). The `--audio` file may itself be a video — only its audio is read. `--mix G` (0..1) keeps the original track under the new one (`amix`, original attenuated by G) — refuses when the input has no audio.

## slideshow

`slideshow IMG...` normalizes every still onto the canvas (`scale=decrease,pad,setsar,fps,yuv420p`) then chains `xfade` (`transition k` starts at `k*(per - fade)`) or `concat` when `--fade 0`. An audio track is always written: `--audio` bed (padded, `afade` out the last 0.8s) or silent `anullsrc` — socials and players see a sound stream either way. `--fade` must be < `--per`; canvas `--size WxH` must be even.

### --transition and --motion kenburns
`--transition` picks the xfade flavor between stills (`fade` default; `wipe*`, `slide*`, `dissolve`, `radial`, `circleopen`). Ignored at `--fade 0` (concat path).

`--motion kenburns` switches the per-still chain to `scale=increase,crop` (fills the canvas — no bars inside the zoom window) + `zoompan` on a single input frame (`d=per*fps` output frames). Even stills push in (`min(zoom+0.001,1.25)`), odd stills pull out (`max(1.25-0.001*on,1.0)`), centered on `iw/2-(iw/zoom/2)`. Do NOT pass `-loop 1` for zoompan inputs — `d=` applies per input frame.

## concat --transition

`--transition` chains xfade across N clips: boundary i sits at `cumsum(d_0..d_i) − i*fade`, audio mirrors it with `acrossfade`. Every input must outlast the fade (`d_i > duration`); mixed audio presence disables the audio chain. Without `--transition` the old paths stand: `-c copy` when all probes match, else normalize+concat.

## split

`split IN --every S` re-encodes with `-force_key_frames expr:gte(t,n_forced*S)` so an IDR lands on every boundary, then lists the boundaries a hair early (`k*S − 0.01`) in `-segment_times` — the muxer cuts at the first keyframe *after* a listed time, so the on-boundary IDR is picked exactly. `-segment_time S` alone overshoots to the next IDR (measured: 4s part for a 2s ask). Output is `stem_%02d.ext` unless `-o` carries its own `%` pattern. Parts re-encode (h264+aac) — keyframes are the price of exact cuts. `--at t1,t2` cuts at explicit chapter points instead of an even grid (`--every` and `--at` are exclusive).

## progress / grid

`progress` draws a fill bar that moves with `t`. `drawbox` w/h are init-only, so it slides a `color` strip in from the left via `overlay x=-main_w+main_w*t/dur` — edge lands exactly at `t/dur`.

`grid A B ..` stacks tiles with `xstack`; each input is scaled+letterboxed into its cell (`--layout 2x2`, `--size` must divide evenly — tiles <16px refused). Audio mixes **all** inputs (`amix normalize=0`) only when every input has audio. `xstack` needs ffmpeg ≥ 4.4.

## key (green screen)

`key FG --bg BG` runs `colorkey` on the foreground and overlays it on the background normalized to the FG canvas (`scale=increase,crop` — fills, never bars). `--similarity` widens the keyed band (0.3 default; spill needs more), `--blend` feathers the edge. A still or too-short `--bg` is looped to the FG length; duration/audio follow the foreground. `--color` takes `0xRRGGBB` or `RRGGBB`.

## freeze / speed --at

`freeze --at T --dur D` rebuilds the clip as 3 concat segments with a `tpad stop_mode=clone` still in the middle; the freeze window's audio is silence (`anullsrc`), not held sound. `freeze --end D` is a simpler outro hold (tpad at the tail, `apad` silence under it).

`speed --factor F --at S --dur D` is a speed window: same 3-segment concat, middle `setpts=(PTS-STARTPTS)/F` + `atempo`. `speed --ramp FROM,TO` steps the change as an 8-segment constant-factor concat (also windowable with --at/--dur); `freeze --reverse N` replays the N s before the hold backwards (`reverse`/`areverse` inside the same concat skeleton) and holds the frame the rewind lands on. `censor --region x:y:w:h` mosaics with 16px `pixelize` cells (or `--mode blur` → `gblur sigma=30`).

## boomerang / chapter / key --despill

`boomerang` is fwd + `reverse` concat — duration doubles, ends where it starts. `chapter --at T|TITLE` writes an `ffmetadata` file and remuxes with `-map_chapters 1 -c copy` (lossless); the first mark snaps to t=0 if it lands late, titles with `=`/`\n` are sanitized. `key --despill` appends `despill=type=green` after `colorkey` for green fringe. `zoom --at/--dur` is the same 3-segment concat skeleton as `speed --at`: pixels only change inside the window.

## autocrop / sheet / title --at

`autocrop` runs `cropdetect` on up to the first 60 s and takes the **last** `crop=` line (it refines the box as it scans); refuses when detection equals the frame or is degenerate. Its probe pass must run at `-loglevel info` — the shared `ffmpeg_base` pins `error`, which silences cropdetect. `sheet` = `fps=N/dur,scale,tile=COLSxROWS` single PNG. `title --at` shifts the overlay's `enable='between(t,…)'` window — lower-thirds at any offset.

## eq / zoom --motion / broll --still

`eq` chains `bass=g=N`, `equalizer=f=3000:g=N`, `treble=g=N` — audio-only. `zoom --motion kenburns` swaps the static scale/crop punch for `zoompan=z='min(pzoom+STEP,F)':d=1:x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':s=WxH` where STEP=(F-1)/(dur*fps) — works whole-clip and inside an --at/--dur window. `-vf` output is NOT auto-mapped once any `-map` appears: use `-filter_complex …[vout]` + explicit `-map "[vout]"` whenever the same command also `-map`s audio. `broll --still` prepends `loop=loop=-1:size=1,fps=N` so a single-frame image streams through the cutaway window.

## overlay --at / caption styling / subs

`overlay --at T [--dur D]` adds `:enable='between(t,at,end)'` to the overlay — works on the single path and the `--tile` cascade (enable goes on the last overlay only). `caption --color RRGGBB` / `--size` reuse the same `render_text` params as `title`. `subs` extracts embedded subtitle tracks via `-map 0:s:N -c:s <srt|webvtt|ass>` picked from the output extension.

## title --size/--color / meta / broll --motion

`title --size` multiplies the auto px (video_w/8, clamped 8-512); `--color RRGGBB` sets glyph fill on the translucent-dark plate — `render_title_styled` threads fg+size through `render_text`, `render_caption` keeps the old defaults. `meta` is a `-map 0 -c copy` metadata pass (title/artist/comment) — lossless, no re-encode. `broll --motion kenburns` appends a `zoompan` push (1.0→1.3) after the `loop` when `--still` is set; refuses on video inserts.

## rotate / delogo / speed --interp

`rotate` maps `--deg` to `transpose=1` (90cw), `transpose=1,transpose=1` (180), `transpose=2` (270); `--flip h|v` is `hflip`/`vflip`. `delogo` blends a box (`x y w h`, >=4px, must fit the frame). `speed --interp` (factor<1) upsamples with `minterpolate=fps={src_fps/factor}:mi_mode=blend` before `setpts` — cheap smooth slow-mo without optical flow.

## channel / loop --until / title --tile

`channel --mode dualmono` = `pan=stereo|FL<c0|FR<c0` (one-ear voice → both), `mono` = `aformat=channel_layouts=mono`, `swap` = `channelmap=map=FR-FL|FL-FR`. `loop --until` derives `times = ceil(SECS/dur)` then `-t SECS` trims the concat. `title --tile N` chains N overlay passes on a diagonal (same 4.4-safe pattern — a reused `[ov]` pad is consumed once on ffmpeg ≤5, so split=N first or chain distinct labels).

## overlay --tile / caption --shift / gif flags

`overlay --tile N` chains N `overlay` passes on a diagonal cascade at 50% alpha — draft watermark without a font. `caption --shift` adds SEC to every cue's start/end after `srt::parse_srt` (clamped ≥0, min 50ms). `transcode --preset gif` takes `--fps` (1–30) and `--width` (16–1920).

## split --scenes / cutsil / grid --audio

`split --scenes T` = probe pass `select='gt(scene,T)',showinfo` (info level, like autocrop) → pts_time list → same segment-muxer recipe. `cutsil` = `silenceremove=start_periods=1:stop_periods=-1` strips dead air at both ends — AUDIO ONLY, on video it desyncs (use `jumpcut`). `grid --audio N` maps `[N:a]` instead of amix.

## replace --duck / pitch / grade --grain

`replace --mix G --duck` duckes the ORIGINAL track under the new audio: `[old][new_sc]sidechaincompress` — pin `aformat=sample_fmts=dbl` on both pads (same requirement as `music`, gotcha above). `pitch --semitones N` = `asetrate=sr*2^(N/12),aresample,atempo=2^(-N/12)` — pitch up/down while duration holds. `grade --grain` appends `noise=alls=N:allf=t+u` after eq/lut.

## volume --at/--dur

`volume --db -60 --at 12.5 --dur 1.5` mutes just that window (`volume=…:enable='between(t,a,b)'`) — the bleep/mute-a-swear ask. `--at` alone runs to the end; `--dur` without `--at` is refused.

## caption --chunk
`--chunk N` (burn mode) splits each cue into ≤N-word groups that share the cue's span evenly — the chunked-caption look without word timing. The 80-cue cap applies after chunking, so long transcripts hit it sooner.

## grade --lut

`--lut file.cube` appends `lut3d` after the `eq` sliders, so the LUT rides on top of any slider correction. Quote-escape the path (`file='...'`).

## fit --fit blur

`--fit blur` pillarboxes with a zoomed, gblur'd copy of the frame (the repurpose look) instead of black bars. Same trick on `broll --fit blur` for the insert. Costs an overlay graph + re-encode; `pad`/`crop` stay the cheap defaults.


## loudnorm JSON

The first pass prints measured values on stderr. ffkit parses them for the second pass. Do not use `-loglevel quiet` on a hand-rolled loudnorm; you will lose the measurements.

## display rotation vs ffmpeg version

`-metadata:s:v rotate=N` writes the mp4 display matrix on ffmpeg ≤6 but is silently ignored on ≥7 — there `-display_rotation:v:0 -N` as an INPUT option (before `-i`) carries the same matrix through stream-copy. `meta --rotate` picks by `ffmpeg -version` major; `--rotate 0` clears. Sign is counter-clockwise in the matrix, so clockwise N maps to `-N`.

## RGB channel checks

`signalstats` only reports YUV stats (YAVG, SATAVG…) — there is no RAVG/BAVG. For colour assertions (`audiogram --color`, title/caption colours) dump the crop as `-f rawvideo -pix_fmt rgb24` and average the bytes. `showwaves` also needs ~0.5s of stream before the strip has content — seek before frame-checking it.

## Generated audio sources start at t=0

`sine`/`anoisesrc` inside a `filter_complex` always produce from timestamp 0 — they ignore the `--at` moment of the effect. To land a generated tone on a window, `adelay=<ms>:all=1` it before the mix (`bleep`). The windowed `enable='between(t,a,b)'` on the SOURCE-side filter is still what silences/marks the window; the generator just needs to be slid there.

## Denoiser strength varies wildly

`hqdn3d` barely dents strong synthetic noise (variance −5% at strength 8); `nlmeans=s=8` cuts ~87% on the same clip and is the usable engine for `vdenoise`. Keep `--strength` modest: nlmeans is per-frame non-local-means — slow on long clips and softens texture at high values.

## Audio-only retiming vs video inputs

Any `-af` chain that changes duration (`atempo`, `atrim` crops) leaves `-c:v copy` video at its original length — the container ends up at the LONGER stream and the two desync. `tempo` refuses video inputs for exactly this; `speed` retimes both.

## Negative values as clap args

`--threshold -30` fails clap parsing ("unexpected argument '-3'") unless the arg carries `allow_hyphen_values = true` — required on every dB-style flag (`--db`, `--threshold`, `--warm`).

## colorbalance midtones on old ffmpeg

`colorbalance` midtone options are `rm`/`gm`/`bm` on ffmpeg ≤5 — the `ms` alias only exists on newer builds (verified crash "Option 'ms' not found" on 4.4). Use `bm`/`rm`/`gm` for midtone shifts.
## `Path::parent()` returns `Some("")` for bare filenames
`-o part.mp4` (no directory) gives `.parent() == Some("")` — `read_dir("")` is
ENOENT. `split`/`frames`/`countdown` part-globs now `.filter(|p| !p.is_empty())`
before the `.unwrap_or(".")` fallback, else every bare-stem output failed after
the parts were already written.
## Infinite overlay inputs need `shortest=1` — and `-loop 1`, not `loop`
Chaining `overlay` over a synthesized infinite stream (a `-loop 1 -i png` input or
`loop=` filter): ffmpeg 4.4 does NOT end the graph on main-input EOF — every
stage needs `overlay=…:shortest=1`, and every still secondary must be looped at
INPUT level (`-loop 1 -framerate N -i`). The `loop` video filter repeats stored
frames at their ORIGINAL pts — `fps` after it cannot invent timestamps, so the
branch stalls at ~1 frame and `shortest=1` then ends the whole output.
`timer` uses this to drive `crop x='mod(floor(t),60)*cell'` digit sprites.
## `loudnorm` upsamples internally
One-pass `loudnorm` outputs at its internal rate (seen: 96 kHz output from 48 kHz
input). Any conform/normalize chain must put `aresample` AFTER loudnorm, not
before. `conform` does `loudnorm=…,aresample=48000,aformat=…` in that order.

## karaoke / audio windows

`caption --karaoke` renders one PNG per word step (cap 200 steps) — each shows
words 1..=k, enabled over an even slice of the cue. `denoise --at`/`leveler --at`
route through `engine::audio_window` like eq/reverb/fx: dry feed ducked to 0 in
the window, FX trim+delayed into place — required on ffmpeg 4.4 where audio
filters ignore `enable`.

## timer box / replace loop

`timer --box-color` draws a fixed card PNG behind the readout (total_w now
counts the centisecond field). `replace --loop` is `-stream_loop -1` on the
replacement — the existing `atrim=duration=<video>` still caps it at the video
edge, so a 3s jingle fills a 60s vlog.

## audio window family — complete

`eq`/`reverb`/`fx`/`denoise`/`leveler`/`pitch`/`dehum`/`vocal` all share
`engine::audio_window` for `--at`/`--dur` (dry ducked to 0 in-window, FX
trim+adelay'ed into place). Skip it only for duration-changing chains —
`speed`/`tempo` must keep their own trim/concat structure, since atrim slicing
an atempo'd whole-file render would shift the window.
`amplify` threshold is a CEILING: pixel diffs BELOW it get magnified (threshold=1 ≈ no-op; 30 catches real motion). `kerndeint` has no parity/mode flags — always same-rate. `selective` now uses native `colorhold` (one filter); `--blend` feathers the edge.
`lumakey` keys the luma BAND [threshold±tolerance], not "everything brighter/darker" — for "remove white sky" use a high pivot (0.9) + wide similarity. `deblock` stock thresholds are near no-op; `--strength` scales alpha/beta/gamma together (weak filter mode does nothing on x264 output). `entropy` filter verified a pass-through on noise fixtures — do not wire it in. `tmedian` drops `2*radius` output frames (history buffering at both edges) and its box-removal needs the intruder visible <half the window. `adeclick` detects *impulse* outliers (~1-sample spikes repair cleanly); flat plateau bursts look like DC steps to its AR model and barely change. `sidechaingate` is a verified pass-through on 4.4 (silent key → full-volume output); `anlms` crushes the desired signal and keeps the reference — do not wire either in. `aeval` works as a filter on 4.4 but not as a lavfi source (no `s` option); `aclick` does not exist on 4.4 — synthesize clicks with `aeval` gated expressions instead.

- `blurdetect` does not exist on ffmpeg 4.4 (5.x+). `idet` prints its verdict as a stderr summary ("Single frame detection: ..."), not per-frame metadata — parse the `TFF/BFF/Progressive/Undetermined` counts.
- `haldclut` accepts PNG/JPG HALD images (Darktable/RawTherapee exports) — LUT identity passthrough is within ~±5 encode error.

- `erosion`/`dilation` thresholds are per-plane (`threshold0..3`), not a bare `threshold` — set threshold1/2/3=0 to leave chroma untouched.
- `maskedmerge` graph pads are consumed once: split the source into 3 branches (orig → merge base, orig → denoise leg, orig → edgedetect mask leg).
- `fieldmatch`+`decimate` drops duplicate frames so output fps falls (25→20 on uniform testsrc, 29.97→23.976 on real telecine) — assert fps < input in tests, not a fixed rate.

- `asdr` does not exist on ffmpeg 4.4. `tonemap` needs `format=gbrpf32le` before it; the zscale pre/post legs only work when input carries HDR tags (PQ/HLG) — SDR input with explicit tin/pin fails ("Generic error in an external library").

- `maskedmax`/`maskedmin` take NO `inputs=` option (framesync auto-detects) — only xmedian wants the count.
- `spawn::run` waits for process exit BEFORE draining stdout — any filter emitting >64KB on stdout deadlocks until timeout. Route large raw output (PCM extraction) through a temp file instead.

- `aphasemeter` outputs TWO pads (audio out0, video out1) — both must be consumed or the graph stalls; map the audio pad as the output audio instead of anullsink (which hangs). dither/morpho are absent on ffmpeg 4.4.

- `earwax` DOES exist on ffmpeg 4.4 (earlier note said absent — checked on ffmpeg 9). It hard-requires 44.1kHz STEREO input: pin `aformat=channel_layouts=stereo:sample_rates=44100` before it or it fails silently inside a graph.

- `vfrdet` prints ONE line at EOF on stderr: `VFR:<ratio> (<n>/<N>)` — no lavfi.* metadata keys; parse the stderr line (a `select`+`-vsync vfr` clip scores ~0.97; concat-joined CFR segments score ~0.01 — flag at >0.05).

- `vidstabdetect` writes a BINARY `.trf` file (`TRF1` header + packed records) — not text; a tempfile between the two passes is the way (NamedTempFile kept alive until the transform job returns).

- `drawgraph`/`adrawgraph` render their own canvas (default 900x256) — the input picture is discarded, so run it on the scaled corner branch. `fgN` takes `0xRRGGBB`/`0xAARRGGBB` — a bare color name like `red` errors "Undefined constant".

- `mptestsrc` has NO `size` option on 4.4 (fixed-canvas MPEG encoder test pattern; test enum names are `dc_luma`, `freq_luma`… not `dc`) — skipped for `gen` since it can't be sized.

- `removegrain` modes are categorical algorithms, not a smooth strength dial (1 = softest neighbour-avg, 11 = median-of-8 strongest). `deconvolve`/`freezeframes`/`maskedthreshold`/`replaygain`/`alphamerge`/`signature`/`untile`/`stereowiden`/`acontrast`/`asupercut`/`hdcd`/`bbox` all exist on 4.4 — verified present, not yet wired.

- `limiter` without `planes=` applies luma numbers to chroma too — pass `planes=1` for luma-only legalization. Codec ringing can push encoded luma a few points past the clamp; that's inherent, not a clamp failure.

- `drmeter`, `astats` print nothing on ffmpeg 4.4 even at clean EOF (metadata filters stay silent) — `meter --mode drm` was cut; use `scan`'s `audio_max_db`/`audio_mean_db` (volumedetect does print).
- `displace`, `maskedclamp` each take THREE inputs (src + two map/ref pads) — a labeled link can be consumed only once, so `split` the map before feeding two pads.
- `maskfun`/`geq` per-plane semantics on 4.4 don't match docs (threshold scale/plane resolution) — luma-band isolation was cut this round; use `key --mode luma` or `levels` for band work.
- `lumakey` keys the [threshold±tolerance] band to alpha=0 (not single-side); outside pixels keep partial alpha with softness — not a hard binary mask.
- `ffmpeg -h filter=NAME` exits 0 even for filters that don't exist ("Unknown filter" on stdout) — verify by actually running the filter. `scharr` is missing on ffmpeg 4.4 (sobel/kirsch/roberts/prewitt exist).
- `vibrance` barely moves on fully-saturated or gray content — test with muted sources (mandelbrot) where its protect-saturated-skin band actually bites.
- `anlms` learns input0→input1 (system ID), NOT "voice in 0, noise in 1" — feed the noise reference as input 0, the noisy mix as input 1, then subtract the estimate; feeding voice/ref directly cancels the voice too (measured −37dB on the tone).
- Multiple `-af` flags don't chain (last wins) — join filters with commas in one `-af`.
- `showcwt` is missing on ffmpeg 4.4 (audiogram wavelet mode deferred); `scharr` also missing.
- `signalstats` reports nothing on stderr — pair it with `metadata=print` (or `file=-`) to read YAVG/SATAVG etc.
- Leaf labels in a filter_complex DO count as consumed when `-map`ped; but a label consumed only inside the graph and never mapped OR fed onward is "unconnected" — every output pad must reach either a filter input or a `-map`.

- `spectrumsynth` needs TWO video inputs (magnitude + phase); feed a `nullsrc` of matching WxH/fps as the phase leg, `slide=scroll` + a `scroll`-animated magnitude = continuous left→right scan. For still images `-loop 1` before `-i`.
- `shufflepixels` block width must stay positive — scale it off the strength range (glitch uses 0.5..20, not 0..1).
- When resolving merge conflicts in CHANGELOG.md, taking "ours" on `## [Unreleased]` drops the just-bumped version heading — entries stay under Unreleased forever. Always keep the `## [x.y.z]` heading line.

- **pixscope needs ≥640x480 input.** Feeding it a small tile errors with
  "min supported resolution is 640x480". For corner-overlay use, upscale the
  leg first with `scale='max(iw,640)':'max(ih,480)':flags=neighbor`, run
  pixscope at full-res, then scale the viz back down to the tile.
- **`separatefields` halves the height, doubles the rate** — 25i→50p frames
  at half vertical res; that is the point (each field becomes a frame).

- **`spawn::run` must drain pipes on threads.** It used to `wait_timeout`
  before reading stdout/stderr — any filter emitting >64KB (macOS pipe
  buffer) deadlocked the child on write (signalstats metadata output
  tripped it; PCM decode hit the same trap earlier via a temp-file
  workaround). Both pipes now have reader threads — do not regress this.
- **`swaprect` takes literal pixel ints on 4.x** — `w=iw/2` errors with
  "Undefined constant". Compute w/h/x/y from probed dims in Rust.
- **`oscilloscope` is a VIDEO scope on 4.x** (XY plot of video input),
  not an audio viz — it belongs on `scope`, not `audiogram`.

- **frei0r filters are dead on this toolchain.** ffmpeg@4's `frei0r`
  wrapper reports "Could not find module" for every plugin under every
  discovery path (`FREI0R_PATH`, `~/.frei0r-1/lib`, absolute
  filter_name); ffmpeg 9 lacks the filter entirely. Do not build a vfx
  verb on it.
- **Verified-dead filters on ffmpeg 4.4** (tested, do not retry):
  `anlmdn` = zero measurable effect at any `s`; `asoftclip` output pins
  ~-21dB regardless of params (broken); `arnndn` needs a model file;
  `colorcontrast` shows no measurable change; `aspectralstats`/`adrc`
  absent; the flanger is named just `flanger` (no `aflanger`).
- **`alimiter` pumps on pure sine fixtures** — `limit=0.5` drops a sine
  to peak 0.177. The limiting is real (peak holds under ceiling), so
  assert `peak <= ceiling`, never an exact level.
- **`bitplanenoise` metadata keys** are `lavfi.bitplanenoise.{plane}.1`
  (LSB); mean >0.8 flags a noisy source (grain ≈0.9, clean ≈0.3).
- **`stereotools balance_in=+0.5` attenuates LEFT ~6dB** (image shifts
  right) — maps `channel --mode bal --pan` directly.
- **`nnedi` is a dead end on ffmpeg 4.4** — exists in the filter list but
  aborts with "No weights file provided, aborting!" (needs an external
  nnedi weights blob none of the builds ship). ffmpeg 9 dropped it.
- **`colorcorrect` on 4.4 has no `analyze` mode** — only manual rl/bl/rh/bh
  region sliders (auto-WB arrived later); `wb`/`grade --split` cover it.
- **`find_rect` wants a GRAYSCALE object image** — a color PNG errors
  "object image is not a grayscale image"; run `format=gray` on the ref.
  Its box lands in `lavfi.rect.{x,y,w,h}` frame metadata — in theory:
  on BOTH ffmpeg 4.4 and 9.0.1 the filter emits NO `lavfi.rect.*` keys
  at all (checked via metadata=print / showinfo / -show_frames with a
  pixel-exact grayscale crop at thresholds 0.3-0.9), so detection is
  dead on these builds and `delogo --find`/`--track` always errors
  "not found"; don't build reporting on it.
- **`readeia608`/`cropdetect` ride the scan pass cheaply** — keys are
  `lavfi.readeia608.*` (any = a CC line decoded) and
  `lavfi.cropdetect.{x1,x2,y1,y2}` (w = x2-x1+1, h = y2-y1+1).
- **`acrossover` pad order is low→high** — output pads follow the split
  list order; `split=500 2000` → b1<500, b2=500-2000, b3>2000 (24dB/oct).
- **`freezeframes` wants frame INDICES, not seconds** — `first`/`last` are
  on the clip, `replace` on the ref input; `repair` converts --at/--dur
  seconds via each clip's own fps. Verified frame-exact on 4.4.
- **`alphamerge` keeps alpha only if the encoder does** — libx264 yuv420p
  silently drops it; `key --mode matte` must encode prores_ks
  `yuva444p10le` (same as `premult`).
- **`untile` turns EVERY frame into cols*rows frames** — a 20-frame
  4x3-tiled clip → 240 outputs; chain it before any fps cap.
- **`asupercut` is ultrasonic-only** on 4.4 (cutoff range 20000–192000Hz)
  — useless below 192kHz sources; skipped, not a hiss remover.
- **`acontrast` >33 expands dynamics AND lifts level** (mean -21→-14dB at
  contrast=80) — it is a tilt, not transparent expansion.
- **`maskedthreshold` threshold is 0..1 of full-scale** (0.2 works as a
  change mask) — both inputs must share dims (`scale2ref` first).
- **`deconvolve` is broken on ffmpeg 4.4** — even a pure impulse PSF
  (identity case) scores PSNR ~5.9 vs input; all deblur attempts are
  worse than leaving the blur. Skipped, not a blur-rescue path.
- **`headphone` needs external HRIR files on 4.4** — no
  `hrir=multichan` option (that syntax is newer); with only `map` it
  expects HRIR impulse-response streams as extra inputs. No bundled
  HRIRs → binaural verb skipped.
- **`signature` needs a few seconds of footage** — ~2s clips report
  "no matching" even against themselves; 4s+ builds enough words for
  "matching of video 0 at T and 1 at T2, N frames matching" +
  "whole video matching" verdicts.
- **`ciescope` `size`/`s` is an INT (256-8192), not WxH** — it renders
  its own square scope ignoring input dims; clamp tile size to >=256.
- **`cover_rect` reads `find_rect` metadata in-chain** —
  `find_rect=object=x,cover_rect=mode=blur` tracks a moving mark live
  (subject to the dead-detection caveat above). Its `cover=` bitmap
  wants a YUV420 image for mode=cover ("cover image is not a YUV420
  image"), while mode=blur accepts the grayscale find template.
- **`replaygain` prints at EOF to stderr** — `track_gain = +N.NN dB` /
  `track_peak = 0.NNN`; chains after volumedetect in one pass.
- **`ocr` finds tessdata by itself** (no datapath needed when tesseract
  is installed under the prefix); each frame costs ~0.5-1s so gate it
  behind `fps=2`.
- **`aemphasis` `type` is a named enum, not `de`** — valid values:
  col/emi/bsi/riaa/cd/50fm/75fm/50kf/75kf; `mode=reproduction`
  (de-emphasis) is the default, `mode=production` re-applies the curve.
- **`il` uses `luma_mode`/`chroma_mode`, not positional `i:i`** —
  `il=luma_mode=i:chroma_mode=i` interleaves fields; pair with
  `setfield=tff` or containers keep reporting progressive. mp4/h264
  ffprobe still shows `unknown` (the flag lives in SPS/VUI) — verify on
  .mov (prores → `field_order=tb`).
- **`vmafmotion` needs no libvmaf** — per-frame
  `lavfi.vmafmotion.score=` metadata; score is resolution-dependent
  (tiny clips ≈0.3, 320x240 testsrc2 ≈5-8, static = 0.00 exactly).
  Costs a per-frame pass — gate it behind a flag.
- **`readvitc` only finds codes on broadcast masters** — synthetic
  clips always read `lavfi.readvitc.found=0`; the found=1 path can't be
  exercised without real VITC lines.
- **`msad` prints the same `average:` key as psnr** — final line
  `msad Y:.. average:0.000000 min:.. max:..`; extend the metric loop,
  reuse the parser.
- **`grid --focus` ignores `--layout`** — hero mode computes per-tile
  rects itself (tile 0 = left ~2/3 column); share the rect list with
  labels/`--time` overlays or they land on grid math.
- **`sidechaingate` hard-mutes vs `sidechaincompress` smooth-dip** —
  same asplit key+mix wiring (ffmpeg <7 needs the explicit split);
  gate is the talk-show bed, compressor the music bed.
- **`json!` macro recursion limit** — serde_json's json! hits its
  internal recursion cap around ~40 keys; once extras get that big,
  assign the tail keys after construction (`extra["k"] = json!(v)`).
- **`allrgb`/`allyuv` take no `size=` option** — the color-cube sources
  only accept rate/duration/sar and render 4096x4096 natively; put the
  target `scale=` in the output chain instead (and keep `rate` low —
  the card is static).
- **`-vf` is an output-side option** — placed between `-f lavfi -i`
  inputs it binds to the NEXT input and ffmpeg errors "cannot be
  applied to input url". Bars appends the cube scale after all inputs.
- **`vif` prints one line per scale (0-3)** — the last line is
  `VIF scale=3 average:X`; `last_metric` (rev find) parses it with the
  same `average` key as psnr/msad. Identical clips score ~1.0.
- **`compand` attacks/decays are SECONDS** — not the ms-style values
  acompressor presets carry. Feeding 8s attack into a short clip leaves
  the gain envelope closed (output drops instead of lifting); keep the
  doc-default 0.3/0.8 envelope.
- **`haldclutsrc` `level=N` → (N³)x(N³) image** — level 4 = 64x64,
  8 = standard 512x512 HALD-8. It renders a still; `-frames:v 1`.
- **`alphaextract` hard-fails on alpha-less input** — "Requested planes
  not available". ffkit pre-checks `pix_fmt` for the alpha fmts
  (rgba/argb/yuva*/gbrap/ayuv/…) and errors with a readable message.
- **`field` outputs HALF-HEIGHT frames** — it extracts one field, so
  320x240 → 320x120 progressive, no interpolation. Cheapest
  deinterlace; pair with an upscale if full height is needed.
- **`-af` appears twice → second overrides first** — when measuring with
  volumedetect, chain it into the SAME `-af` (`-af "filter,volumedetect"`);
  two `-af` args don't concatenate.
- **bandreject on broadband looks like a no-op** — it only carves a
  narrow slice; on pink-noise-like material mean_volume barely moves.
  Verify on a sine at the notch frequency instead (-11dB on 440Hz).
- **`afftfilt` has no `f` variable on ffmpeg 4.x** — frequency must be
  computed as bin index: `b ∈ [lo*win_size/rate, hi*win_size/rate]`, and
  the negative-frequency mirror `[nb-hi, nb-lo]` must be kept too or the
  band passes at half amplitude / distorts.
- **`chromakey` keys in YUV chroma space** — better than RGB colorkey on
  wrinkled greenscreens; output goes through the same despill tail.
- **`ahistogram` stamps one column per frame like showspectrum** —
  `slide=scroll` keeps it scrolling; no rate arg needed.
- **`boxblur` radius is px (0-23), not sigma** — ffkit maps sigma→radius
  /3 clamped; power = kernel passes (2 = near-gaussian).
- **`ahistogram` emits `yuva444p` (alpha)** — append `format=rgb24`
  before `colorkey`/overlaying its output (same fix as aphasemeter).
- **`ahistogram` heap-crashes on Ubuntu's ffmpeg 4.4.2**
  (`malloc_consolidate(): invalid chunk size` — distro build bug).
  Homebrew ffmpeg 4.4.8 is fine; the cli test probes with a real render
  before asserting (`ahistogram_works`).

- **ffmpeg's `weave` stacks full frames vertically** (320x240 → 320x480
  output, fps halved): it expects FIELD input, not progressive frames.
  Real temporal interlacing (60p→30i) is
  `tinterlace=mode=interleave_top` + `setfield=tff` — used by
  `transcode --interlaced --interlace-mode weave`.
- **`hilbert` on ffmpeg 4.x only generates FIR coefficients** (source-
  style |->A output), not a transform — SSB modulation can't be wired
  from it. Dropped `fx --kind ssb`.
- **`mptestsrc` takes `rate`/`duration`/`test`/`max_frames`, no `size=`**
  — like allrgb/allyuv it needs a downstream `scale` (bars verb handles).
- **`showpalette` requires `format=pal8` input** and emits a
  `16s x 16s` swatch grid; its `s` option is swatch pixels, not WxH.
- **`avgblur` `sizeX` is kernel half-width, planes=15 hits Y/U/V** —
  sigma≈2·sizeX gives comparable spread to gblur.

- **`monochrome`'s cb/cr tint params do nothing on 4.4** — U/V planes
  stay at 128 even at cr=1/cb=-1/size=10/high=1 (only luma moves).
  Tinted-B&W stays with `duotone`; don't wire `bw --tint`.
- **`fieldorder` is a passthrough on demuxed input** — the filter needs
  frames that arrive flagged interlaced; demuxed prores/.mov loses the
  flag on 4.4 so nothing gets reordered (idet still reads TFF after
  fieldorder=bff). Field-order fixing needs a different path.
- **`afireqsrc` doesn't exist on 4.4** (5.0+); the linear-phase EQ path
  is `sinc`+`afir` instead — `eq --linear` convolves a kaiser-windowed
  impulse (att up to 180dB, phase response knob via `phase=`).
- **`asubcut`/`asupercut`/`asuperpass`/`asuperstop` are edge-band
  specialists**: asubcut cutoff is capped at 200Hz (rumble below the
  voice), asupercut only accepts ≥20kHz (ultrasonic — needs hi-res
  sample rates to matter). asuperpass/asuperstop are the matching
  ultrasonic band pass/stop — unwired for now.
- **AAC-in-.wav now auto-retargets**: `engine::write_job` rewrites
  `-c:a aac` to pcm_s16le/flac/libmp3lame/libvorbis/libopus by output
  extension — AAC-in-WAV misdecoded on some ffmpeg 4.x builds.

- **`asuperpass`/`asuperstop` are band ops at ANY center frequency** —
  despite the "super" name they're not ultrasonic-only: asuperpass
  keeps the band around centerf (steep order-4..20 skirts, qfactor for
  width), asuperstop notches it out ~50dB deep. Note asuperstop leaves
  the above-band slightly hot (level param overshoot ~+6dB at the edge).
- **`allpass` width can exceed its center frequency** — `w` is a
  broadness knob, not a strict band edge: f=110:w=600 is valid and
  rotates phase across more of the spectrum. Verify via crest shift
  (volumedetect max_volume moves, mean_volume stays).
- **`aderivative`/`aintegral` are raw math ops, not creator FX** —
  derivative = +6dB/oct HF tilt at −24dB level on 440Hz, integral =
  huge bass swell with DC wander; neither maps to a shippable effect.
- **`floodfill` takes per-plane component values, not colors** —
  s0..s3/d0..d3 are raw ints (YUV planes for yuv420p), so a hex
  "green" never parses; wiring a color pick would need RGB→plane
  conversion per pix_fmt. Skipped.
- **`setparams=field_mode` fixes parity labels demux loses** — the
  fieldorder filter passthroughs on unflagged input (round 230), but
  setparams just WRITES the flag: `transcode --field-order tff|bff|prog`
  is the relabel repair. setparams=range works the same way (--range).
- `interleave` (video) is broken on ffmpeg 4.x: emits ~1M fps output (900001 frames/0.9s), `settb=AVTB` does not fix it, and after `fps=N` only the second stream's frames survive — alternation lost. Do not ship it.
- `sendcmd` is video-only; the audio twin is `asendcmd`. Multi-command syntax: `commands='0.0 filter_name option value;1.0 filter_name option value'` (semicolon-separated, single quotes REQUIRED or `;` splits the filter chain). `equalizer` accepts `frequency` and `gain` commands — this is the runtime-param automation class.
- `mix` video filter at `scale=0` auto-normalizes by weight sum — `weights` just needs per-input numbers, no manual scale. Weights are SPACE-separated inside quotes.
- `testsrc2` takes `size`/`rate`/`duration` params (fits the normal bars.rs branch); `mptestsrc` does NOT take `size=`.
- `anullsrc=channel_layout=stereo:sample_rate=44100` produces true digital-black silence (−91dB) — the silence bed generator.
- Equalizer `w` (width) is the full band width in Hz — `w=300` covers ±150 around center frequency.
- `ainterleave` interleaves samples temporally — mono+mono becomes ONE mono stream of alternating samples (2s, 1 channel), NOT stereo. For dual-mic → stereo use `amerge` (input 0 lands on ch0/L, verified by level split).
- `graphmonitor` consumes the stream and emits a stats card — overlay it via `split[a][b];[b]graphmonitor[sc];[a][sc]overlay` like other sink-renderers, never in the main path.
- Pure tones (sine) defeat `detect_lag`/multicam `--align`: a 600Hz sine correlates at every 1.67ms period → 35ms false lag vs the true 400ms. Alignment needs broadband audio (speech, room tone, pink noise).
- `channelsplit` errors on unmapped pads ("unconnected output"): extracting one side of stereo with `[L]` mapped and `[R]` dangling fails — use `pan=mono|c0=c0` / `c0=c1` to pick one channel instead.
- `lut2` is a per-pixel two-input EXPRESSION filter (c0..c3 over x=input0 px, y=input1 px), not a LUT-map — gradient-map colorize via a ramp image does NOT work that way. Skipped.
- clap `Option<f64>` rejects `-35`-style leading-dash values ("unexpected argument") unless the arg declares `allow_negative_numbers = true` — needed on every dB-threshold flag like `scan --deadair`.
- `siti`, `outlier`, `aspectralstats`, `ydiff`, `adiff`, `asubtract`, `grayworld` are NOT in ffmpeg 4.4 (all added in 5.x) — skipped as scan/diff legs.
- CUE `FILE` media types only cover WAVE/MP3/AIFF/BINARY/MOTOROLA — a video input still writes a playable sheet but the tag falls back to `BINARY`.
- `write_job`'s verification (exists → metadata → probe) is for FILE outputs — URL/stream outputs (`rtmp://`, `tcp://`, `udp://`) always fail it. For `live` do `ensure_input` + `ensure_output_allowed` + `run_argvs` + `Contract::ok(tool, Some(url), None)` manually.
- `drawbox`/`drawgrid` take timeline `enable` — a full-frame guide overlay (safe areas, grid) can still honor `--at` windows; `drawgrid w=iw/2:h=ih/2` draws just a center cross.
- clap derives the flag name from the field name — `pub loop_: bool` becomes `--loop_`. For reserved words use `#[arg(long = "loop")]`.
- yuv420p halves chroma resolution — a 2px colored box line lands blended with the neighboring pixel's hue after x264 encode. Pixel-assert hue dominance (r > g+60), never exact channel values.
- `timer` sprite TC: cells = `fps.round()` (NTSC 29.97 → 30-cell nominal numbering), frame field = `mod(floor(tv*fps),cells)` — use the REAL fps in the multiplier, not the rounded cell count (30 vs 29.97 drifts ~3.6s/hour).
- `-movflags` is an MP4-muxer-only option — a `--to`/`live` FLV output must drop `+faststart` (moov doesn't exist in FLV) and add `-tune zerolatency` instead.
- `&[m]` on an owned `Argv` MOVES it (array literal by value) — `engine::commands_of(&[m])` then `argvs.push(m)` is E0382. Borrow with `std::slice::from_ref(&m)` or clone.
- ebur128's summary `Peak:` prints `dBFS` on ffmpeg 4.x (`dBTP` on 5.x+) — parse both suffixes; per-frame `M:`/`S:`/`I:` lines never match a `strip_prefix("I:")` on the trimmed line.
- `scan` used to hard-require video — audio-only inputs (podcast m4a) now run the audio legs (`--loud`, `--deadair`, volumedetect) with video extras null; explicitly-requested video legs (`--scenes`/`--motion`/`--timecode`/`--bbox`/`--text`/`--dupe`) still error so a mistyped file can't silently pass QC.
- `blend=all_mode` on ffmpeg 4.x has 33 named modes — the overlay --mode list was extended to 19 Photoshop-style names (burn/dodge/softlight/hardlight/vividlight/linearlight/pinlight/hardmix/exclusion/negation/subtract/divide/glow/phoenix/reflect on top of the classic seven). Dodge self-blend is the easiest pixel-assert: same clip over itself lifts YAVG ~120 → ~181.
- A closure `|n| bytes.windows(n).any(...)` captures the OUTER `bytes` — a later shadowed `let bytes = read(other)` doesn't rebind it; byte-content checks in tests must take the buffer as a parameter, not capture it.
- ffmpeg `-vf` after `-i` applies to the OUTPUT file — for the `live` verb the scale/-r must go between the input and the encode args (before `-f flv`), not at the front.
- `-movflags frag_keyframe+empty_moov+default_base_moof` writes `moof` fragments + `mfra` index — assert those boxes exist (a plain `+faststart` remux has neither).
- The `tee` muxer skips ffmpeg's default stream selection — without an explicit `-map 0:v -map 0:a` it errors "Output file #0 does not contain any stream". Syntax: `-f tee "[f=flv]url|[f=mp4]archive.mp4"` (encode once, mux twice).
- `Argv::extend` takes `S: AsRef<OsStr>` — an array of bare `"x".into()` can't infer S (E0283). Use plain `&str` elements, or `.to_string()` when mixing with `format!`.
- clap `required_unless_present = "other"` must become `required_unless_present_any = ["a","b"]` when a THIRD flag also satisfies the requirement — adding `--fit` beside `--factor`/`--ramp` breaks parsing unless fit is added to the exempt list.
- `-map -0:s` drops subtitle streams, `-map -0:d` drops data — a positive `-map 0` must come first (negative maps subtract from the already-mapped set).
- `-ss` BEFORE `-i` rewinds input timestamps to 0 — a following output `-to` no longer means the original timeline. For lossless trims translate `--to SEC` into `-t (to - from)` as an OUTPUT option (keyframe-accurate: seeks to the last keyframe ≤ `from`, never frame-exact).
- Multi-input lavfi graphs (e.g. `live --test` with `-f lavfi -i testsrc2` + `-f lavfi -i sine`) get no automatic stream selection — always add explicit `-map 0:v -map 1:a` or ffmpeg picks the first streams it sees.
- The x264 default GOP is ~250 frames: a <9s fixture has ONE keyframe at t=0, so input-side `-ss` can only seek to 0. Trim tests need `-g 30` (or `-force_key_frames`) to create real GOP structure.
- `-af` / `-vf` cannot feed a stream that comes OUT of `-filter_complex` ("specified through -vf/-af/-filter option for output stream 0:1, which is fed from a complex filtergraph") — post-filters like loudnorm must be appended INSIDE the graph (`;[ac]loudnorm=…[aout]`), and when a filter_complex string can end with `;` the trailing empty chain element errors "No such filter: ''" — `trim_end_matches(';')` it, and never append `;x` to a string already ending in `;` (yields `;;`).
- `.m4a` resolves to the `ipod` muxer, which rejects EVERY video codec on ffmpeg 4.x (mjpeg AND png both "not currently supported") — embedded cover art needs `-f mp4` forced (m4a IS mp4, just named differently).
- concat-joining clips needs every segment normalized to ONE canvas first — scale=fw:fh:decrease,pad,setsar=1,fps + aresample/aformat for audio — or concat desyncs/switches SAR mid-stream.
- `0:a:m:language:L` positive map fails "matches no streams" when the tag is absent — that IS the loud failure path for a missing language; add `?` only if you want a soft pass-through.
- `Contract::with_extra` REPLACES the whole extra object — collect every key in one `json!` call (adding a key later in a second with_extra drops the earlier keys).
- `assert!(o.status.success())` on a Command::output loses the ffmpeg stderr — `assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr))` turns fixture-build failures readable.
- `live --slate`: the still card rides `-loop 1 -t D -i img` as input 0 — `-stream_loop -1` must be pushed AFTER it (it binds to the NEXT `-i`, i.e. the content); swap the order and the slate loops forever while the content plays once.
- Container chapters on mp4/m4b: build an ffmetadata `[CHAPTER]` table (TIMEBASE=1/1000, START/END in ms) and feed it as a `-f ffmetadata -i` input, then `-map_metadata IDX -map_chapters IDX`; chapters beyond `duration` get clamped by the muxer — use `END=dur` on the last one.
- `-disposition:a -default` clears ALL audio defaults, `-disposition:a:N +default` sets one — both are output-stream specifiers and need a repack (`-map 0`), not a filtered `-map 0:a:…` subset.

- **chapter marks at/past the input end** → ffmpeg dies inside ffmetadata parsing ("Chapter end time X before start Y / Cannot allocate memory"), far from the real cause. `chapter::check_marks` rejects `t >= duration` up front with a clean error — call it after `parse_yt_list` on every `--chapters` path (remux/deliver).
- **any `-map` drops every unmapped stream** — once `-filter_complex` output is mapped, the audio side must be mapped too (`0:a`/`1:a`/a graph label); "video in the graph, audio by default" is not a thing.
- **`-hls_flags` is single-valued** — combine flags with `+` (`delete_segments+omit_endlist`), never push a second `-hls_flags` (last one silently wins).
- **sliding-window HLS ≠ VOD** — `--live` swaps `-hls_playlist_type vod` for `event` and adds `-hls_list_size N`; `single_file` + `delete_segments` are mutually exclusive (validated out).

- **the tee muxer takes `+`-joined output specs, not extra `-f`** — multistream is ONE output: `-f tee '[f=flv]to|[f=flv]restream|[f=mp4]archive'`. Per-destination format goes inside the `[f=…]` bracket; `tee` needs explicit `-map` for every stream (no default selection).
- **`timer --clock` reads local TZ via `date +%z`** — Rust std has no local-time API without a chrono dep; a `Command::new("date")` spawn gets the offset (UTC fallback). The sprite-cell readout then just seeds `tv` with seconds-of-day — no drawtext needed.
- **`-start_number`/`hls_start_number_source` are independent of `-hls_flags`** — segment numbering options live next to the muxer, not in the flags string; epoch seeds `seg_<epoch>` names AND the media sequence.
- **`live --vertical` must letterbox on BOTH paths** — the plain `-vf scale=W:H` branch distorts the picture; the vertical preset routes the `scale=…:force_original_aspect_ratio=decrease,pad=…` chain on the non-graph path AND joins the same `vchain` inside `-filter_complex` (slate/overlay modes already normalize through it).
- **`-g N` keys land at GOP *boundaries*, not every N** — x264 treats `-g` as max GOP size; on a 31-frame clip `-g 15` yields keys at 0 and 16. Assert key spacing `<= N+2` in tests, never exact `N` cadence.
- **`-map_metadata -1` still leaves the muxer `encoder` tag** — stripping wipes inherited tags (title/comment/GPS) but Lavf writes its own `encoder`; don't assert "no tags at all", probe a specific key. `-map_metadata` flags compose in order: `-1` then `-metadata k=v` = wipe + retag in one pass.
- **`-hls_flags` is single-valued — collect then join once** — `-hls_flags a -hls_flags b` keeps only the LAST set; gather every enabled flag into one `Vec` and push `-hls_flags a+b+c` once. Same story for `+`-joined tee `[f=…]` output specs.
- **`Argv::extend` can't infer `["lit".into(), var.into()]`** — mixing a literal `.into()` with a value `.into()` leaves `S` ambiguous (E0283); use `["lit".to_string(), val]` so both arms are `String`.
- **2x-CBR default must split the suffix first** — deriving `bufsize = 2*maxrate` naively (`r.split_at(len-1)`) turns `6000` into `120000`; check `tail.is_alphabetic()` before assuming a `k`/`m` suffix exists.
- **chapter marks ≥ duration die in check_marks, not ffmpeg** — importing marks at/past the input end returns a clean `chapter at Xs lands at/past the end` error; test fixtures need marks *inside* the clip (1s fixture → `0:00`/`0:00.5`, not `0:01`).
- **`-map -0:v:m:attached_pic` does NOT drop cover art on ffmpeg 4.4** — `m:` is a stream-METADATA matcher, but `attached_pic` is a stream DISPOSITION flag; the negative map silently matches nothing and the mjpeg stream survives the repack. Drop it per-index instead: probe `disposition.attached_pic==1` for each stream's `index` and emit `-map -0:{i}` for each (after every positive map).
- **`cmd1 && cmd2 &` backgrounds the whole AND-list** — `cd /x && python lsn.py &` runs the cd+python in a background subshell; later `&&` commands then run in the ORIGINAL cwd and `cat log.txt` finds nothing. Use absolute paths everywhere instead of `cd` before `&`.
- **GIF alpha lives in the palette, not RGBA** — `extract --gif --transparent` = `palettegen=stats_mode=full:reserve_transparent=1` + `paletteuse=dither=bayer:alpha_threshold=128`; the transparent bg becomes a palette index. Decode-check by `ffmpeg -i t.gif -vf format=rgba,crop=1:1:x:y -f rawvideo` and read the alpha byte.

- **`-init_seg_name`/`-media_seg_name`/`-single_file_name` resolve against the manifest's directory** — the DASH muxer prepends the .mpd's dirname; passing `dir/seg-…` produces `dir/dir/seg-…` and "Could not write header … No such file or directory". Pass BARE filenames (`seg-$RepresentationID$-$Number%05d$.m4s`) and let the muxer place them next to the manifest.
- **ffmpeg 4.4's webp decoder can't read ANIMATED webp** — `ffprobe`/`ffmpeg -i` on a libwebp ANIM/ANMF file fails with "image data not found" even though the file is spec-valid (browsers, Discord, Telegram all play it). Verify structure instead: `RIFF`+`WEBP` magic, an `ANIM` chunk, and count `ANMF` frames (plus `ALPH` subchunks for alpha) — never assert pix_fmt/duration from ffprobe on .webp.
- **A `-lavfi` graph can't end in a dangling `[label]`** — `fps,scale[x]` errors "unconnected output" when nothing consumes `[x]`; only label intermediate pads. Bare chains auto-connect input 0 → first filter → output.
- **CENC on 4.4 = `-encryption_scheme cenc-aes-ctr -encryption_key <32hex> -encryption_kid <32hex>`**, ISOBMFF outputs only; ffprobe can't decode the essence afterward (profile-level ok but stream fails) — verify by grepping the file for `encv` + `senc` boxes instead.
- **FLV can't carry HEVC on ffmpeg 4.x** — `live --codec hevc` must pair
  with an MPEG-TS transport (`srt://`/`udp://`); SRT URLs ride the same
  mpegts path (`?mode=listener` receiver side, caller is default).
- **DASH `-adaptation_sets streams=` indexes OUTPUT stream order** — `id=0,streams=0,1 id=1,streams=2` groups output streams 0,1 (the two `-map [lvN]` video rungs) vs stream 2 (`-map 0:a` audio). The indexes are NOT input stream indexes; count them in `-map` emission order.
- **`-itsoffset` shifts ONE input's timestamps pre-demux** — audio-delay on a lossless repack = read the file twice (`-i f -itsoffset D -i f`), take non-audio from the first read and audio from the shifted one (`-map 0 -map -0:a -map 1:a`). Negative delay just puts the offset on the video-side input instead. Combine with `-ss` by repeating it before EACH `-i`.
- **hls subtitle groups / `iframe_playlist` / `byte_range` flags don't exist on ffmpeg 4.4** — `-var_stream_map "… s:0,sgroup:sub"` fails with "No streams to mux", and the two `-hls_flags` values are unknown constants (5.x+). `single_file` already emits `EXT-X-BYTERANGE` entries — a `--byterange` flag would be a dupe.
- **ffprobe omits `color_*` fields on untagged video** — `color_space`/`color_primaries`/`color_transfer` only appear when the bitstream/container actually carries them; treat `None` as "unknown", not an error. Synth an HDR fixture with `-color_primaries bt2020 -color_trc smpte2084 -colorspace bt2020nc`.
- **`-hls_flags low_latency` doesn't exist on ffmpeg 4.4** — "Undefined constant" (LL-HLS partial segments need 5.x+). Verified dead same as `iframe_playlist`/`byte_range`.
- **ffprobe `packet=dts_time` can start negative** — B-frame reorder puts display-delayed packets at dts < 0; GOP intervals should be diffs between consecutive K-flagged packets in stream order, never absolute pts.
- **`conform`'s "nothing to conform" guard must list every new flag** — adding a conform flag without extending the guard leaves it unreachable (hit by `--even`).
- **`chapter --at` marks can't sit past the input's duration** — export formats (--cue/--lrc/--podcast) inherit the same bounds check as embedding; keep test marks inside the fixture's 1s.

- **mp4/mov muxers silently drop unknown metadata keys** — `bpm`/`tmpo` never land (ffmpeg movenc has a fixed tag whitelist; mp3/mkv/flac keep them). If a tag must survive, deliver in an open container or test the probe yourself.
- **framemd5 with `-c copy` hashes compressed packets, not frames** — packet checksums differ from decoded-frame checksums; archive manifests should decode (no `-c copy`).
- **LRC `[offset:+500]` headers parse as bare numbers** — a naive timestamp parser turns the offset tag into a mark at +500s; only treat `[mm:ss]`-style brackets (containing ':') as marks.
- `publisher` IS in the mp4/mov tag whitelist and lands; `copyright` also lands — but `bpm`/`tmpo` don't (movenc fixed whitelist). For podcast tags check per-key with ffprobe, don't assume "some tags land".
- `transcode --copy-video` can't combine with any video filter flag (filters never see a copied stream) — it errors; it's for audio-only re-encodes and repacks.
- ffprobe packet sizes measure compressed payload only — `scan --bitrate` reports codec bitrate, not container bitrate (container overhead isn't in packet.size).
- **`-v error` kills blackdetect/freezedetect's interval report** — the `[blackdetect @ …] black_start:… black_end:…` lines ride stderr at info level; a quiet detector gets zero ranges. Run the detection pass at default level and parse stderr+stdout together (same as scan).
- **A `-f lavfi` `filter_complex` concat can silently truncate to the first segment** — mixing `testsrc`/`color` sources through `[v][a]concat=n=3` output only segment 0's video (2.0s of a 4.7s graph); synthesize multi-segment fixtures as separate files joined by the concat DEMUXER instead.
- **`--video-delay` maps like `--audio-delay` for BOTH signs** — non-shifted-track streams always come from input 0 and the shifted track from input 1; only `-itsoffset` placement differs (negative = the offset goes on input 0, delaying everything except the shifted stream). Do not flip the map by sign.
- **mp4/m4a accepts `show`/`season_number`/`episode_id`/`network`/`album_artist`/`track` tags** (movenc whitelist) — unlike `bpm`/`tmpo`. TV/podcast feed metadata lands fine in ISOBMFF.
- **Bare `-metadata:s:t` stamps EVERY attachment stream** — for multiple `-attach` files, index the specifier (`-metadata:s:t:0 mimetype=…`) or the last mime wins for all.
- **movenc normalizes `location` to `+DD.DDDD+DDD.DDDD/`** and writes both `location` and `location-eng` — assert on the normalized value.
- **`-tag:v hvc1` retags, it doesn't convert** — codec stays the same, only the container brand changes; h264 tagged hvc1 lies to the player. Restrict to mp4/mov outputs.
- **`-ss` before `-i` + re-encode = frame-accurate split parts** — needed because black boundaries aren't keyframes (segment muxer would cut GOP-late).
- **`-timecode` is an output option on 4.x too** — mov/mp4 write a `tmcd` data stream + `timecode` stream tag; mkv writes the `TIMECODE` format tag. It needs a video repack (audio-only repacks error).
- **`meta --creation-time auto` uses UTC civil conversion** — `SystemTime` → ISO needs no chrono: Hinnant's civil-from-days algorithm; movenc re-normalizes to `…Z` form.
- **`-disposition:s` modifier form mirrors audio** — `-disposition:s -default` clears all sub defaults, then `+default` on the chosen index (same pattern as `--default-audio`).

- **`media_type` lands on mp4/m4a as the stik-relevant tag; sort_* tags are dropped.** movenc's whitelist keeps `media_type`/`gapless_playback` but silently drops `sort_artist`/`sort_name` — don't ship iTunes sort tags via ffmpeg 4.4.
- **mkv TIMECODE lives in format tags, mov tmcd in the video-stream `timecode` tag.** `probe`/`scan` check both (`timecode` key) — format tag is uppercased `TIMECODE`, stream tag lowercased.

- **`split --copy` snaps boundaries to the next keyframe** — the segment muxer with `-c copy` can only cut where a keyframe exists, so parts are approximate; the re-encode path (`-force_key_frames`) is the frame-exact one. Short fixtures need `-g N` to have any interior keyframe at all.
- **WebVTT import takes the cue's first text line only** — chapter titles are single-line by design; cue identifiers and `-->` timing settings are skipped.
- **`has_alpha` is decided from `pix_fmt` naming** (yuva*, rgba/bgra/argb/abgr, ya*, gbrap*, ayuv…) — vp9 alpha needs a yuva input *and* the encoder to keep it; a `color=` lavfi source silently encodes plain yuv420p.

- **`extract --subs` re-muxes through the text encoders** — `-c:s copy` would write the source codec's binary layout (mov_text isn't srt); the .srt/.ass/.vtt extension picks `srt`/`ass`/`webvtt` so the container and codec always agree.
- **`chapter --spread` puts mark i at `duration*i/N`** — last chapter ends exactly at EOF; ask for `--spread 3` on a 3s file and titles land 0/1/2s, none past the end (the at-end guard still rejects).
- **`probe.tags` groups by `format` and `stream:N` with the *absolute* stream index** — the same index `attached_pic_indices` uses, so tag lookups and stream selection stay consistent on multi-track files.

- **movenc drops `rating` but keeps `description`/`synopsis`/`episode_id`/`hd_video`.** `-metadata rating=600` never lands on mp4/m4a on ffmpeg 4.4 — iTunes content-rating has to come from elsewhere; the other publishing tags are all in the whitelist.
- **`live --audio-only` still writes FLV/MPEG-TS, not mp3/aac.** The container is chosen by transport (`rtmp/tcp`→flv, `udp/srt`→mpegts); aac-in-FLV is a valid audio-only stream — don't expect an `.mp3`-style file on the wire.
- **`probe.streams[].index` is the absolute ffprobe index**, not the per-type ordinal — `extract --subs --track N` counts subtitle streams (`0:s:N`) while `remux` stream maps count absolute; keep them distinct when scripting.

- **Rotation lands as a display matrix only through demux→mux.** `-metadata:s:v rotate=90` inside a lavfi-made file is silently dropped on 4.4; `-i in.mp4 -c copy -metadata:s:v rotate=90` carries the demuxer-synthesized displaymatrix through and IS what `meta --rotate` emits. `probe.rotation` reads the side data, not the `rotate` tag.
- **`concat --chapters` marks come from probed durations** — cumulative `sum(d_i)` at 1ms timebase; transitions (overlap drift) and gaps (padding) would mis-title the joins, so they're refused rather than silently wrong.
- **`live --no-audio` still needs the video guard first** — a no-video input fails `input has no video` before flag conflicts are checked; ordering matters when scripting around the error kinds.

- **`select` + image2 needs `-vsync 0`** — dropping frames via `select='not(mod(n,N))'` into the image muxer under default cfr sync re-duplicates the survivors back into almost every slot; pass `-vsync 0` (passthrough) so only the selected frames land on disk (`frames --nth` does this for you).
- **`-itsscale R` direction** — factor multiplies the *duration* (1.042 stretches PAL 25 to film 24 timing; 0.96 speeds it back up). It re-stamps input timestamps, so `-ss`/`-itsoffset` window arguments then read the scaled timeline — refuse them together rather than silently double-shift.
- **`extract --chapter` re-encodes** — chapter boundaries rarely sit on keyframes; stream-copy would snap the start to the chapter's GOP head and include leading frames from the previous chapter. `-ss`+re-encode lands the start frame-accurate at the cost of one encode.

- **Empty concat segments hang ffmpeg** — a `trim` window past EOF yields zero frames and the `concat` filter waits on the pad forever; when a splice/replace tail could land past EOF, drop that segment from `n` instead of feeding an empty pin (insert --replace hit this).
- **`-output_ts_offset` is an output option** — it must sit with the output URL (after `-i`/`-map`), not on the input side like `-itsscale`/`-itsoffset`; placement decides whether ffmpeg honours it.

- **A single-frame overlay input repeats its last frame at EOF** — `overlay`'s default eof_action is `repeat`, so feeding a rendered PNG as a bare `-i` (no `-loop`) still covers the whole target segment. That's exactly a per-slide caption sticker — no loop needed (slideshow --titles relies on it).
- **`-metadata title=` ahead of `-f tee` lands on every destination** — the tee muxer forwards global metadata to each child, so one flag titles the pushed stream AND the `--record` archive (verified: FLV onMetaData + mp4 `title` tag both carried it).
- **blend's 33 modes include no-op traps** — `normal` is just the top input passthrough (worthless as a choice); the useful stragglers are grainmerge/grainextract (film-grain composite), the `*128` neutral-128 variants, and/or/xor (channel logic), average/extremity/freeze/heat. `extract --keyframes` needs the same `-vsync 0` as `frames --nth` (select + image2 — the cfr re-duplication trap).

- **`metadata=print` writes `key=value`, not `key:value`** — scdet timestamps come out as `lavfi.scd.time=1.2` (equals sign). A parser anchored on `key:` never matches — scan --scenes shipped that latent bug (tested stderr? no: file=- goes to STDOUT while detect logs hit STDERR). Match `=` (and keep `:` as a fallback).
- **scdet scores texture-change, not color-change** — a flat red→flat blue hard cut reads score 0 (the per-pixel difference is uniform → zero variance). Scene detection needs textured content; test fixtures must be testsrc2/smptebars-style, not `color=` flats.
- **scdet reports a cut on both boundary frames** — `lavfi.scd.time` appears twice ~1 frame apart for one cut; dedupe marks within ~50ms or every scene generates a doubled entry (scene_cuts showed `[1.2, 1.2]`).

- **`live` URL inputs must bypass `ensure_input`** — it stat()s the path and would refuse `rtmp://…`/`udp://…`/`http://…` inputs. Skip the file-exists check whenever the path string contains `://` (probe.rs, live stream_out, and the verb's own flag validation all needed the bypass).
- **URL inputs can't seek or loop** — `-ss`/`-stream_loop` on a live pull feed are meaningless (and concat slates can't wrap them). Refuse those flags up front instead of letting ffmpeg fail opaquely mid-stream.
- **Stream-copy `-ss`/`-t` are packet-accurate, not frame-accurate** — `extract --audio --from/--to` lands on demuxer cue boundaries (an aac rip asked for 1.0s came out 1.509s); assert loose windows in tests.
- **xfade name list ≠ your guess** — ffmpeg 4.4's vertical squeeze is `squeezev` (not `squeezeev`); check `ffmpeg -h filter=xfade` before wiring an enum variant — an unrecognized name parses as an expression and fails with "Undefined constant".
- **`-hls_base_url` only prefixes URIs in the playlist** — segments still write next to the manifest dir; serving them from the CDN is your deploy's job.
- **`live --crf` replaces `-b:v` entirely** — a CRF stream has no rate envelope; pairing it with `--vbitrate`/`--maxrate`/`--bufsize` is a mode conflict, not a cap.
- **`-rw_timeout` is microseconds and socket-scoped** — it guards each output's network socket; the tee muxer opens its children internally so a top-level `-rw_timeout` can't reach them (live --rw-timeout refuses `--record`/`--restream`). Name collision: the global `--timeout` is ffkit's process kill timer (seconds) — the ingest-stall flag had to be `--rw-timeout`.
- **`-decryption_key` is a mov-demuxer option** — input-side only, binds to the next `-i` (like `-itsscale`/`-itsoffset`); put it before every encrypted input. It decrypts while demuxing so the repack still stream-copies out — `--encrypt`'s mirror for ClearKey receipts.
- **`-copyts` keeps input timestamps verbatim** — a 5s-offset capture stays at start_time=5 instead of re-zeroing (that's the point). It fights every ts mutator (`-itsscale`, `-itsoffset`, `-output_ts_offset`, delay paths) — refuse the combination.
- **`meta --lang-{audio,subs}` list position is the track index** — `eng,,jpn` tags a:0 and a:2, leaves a:1's `und` alone; ffprobe reports untouched tracks as `und`, not missing.

- **`-global_sidx` is mp4-single-file only** — dash's global SIDX index lives at the tail of the `--single` byte-range file; it conflicts with `--streaming` (per-frame moof) and is meaningless in `--webm` (webm packages don't carry sidx). Guard all three in the verb, not in ffmpeg.
- **`-hls_playlist_type vod` is already the default** — ffkit's `hls` writes `vod` on every non-`--live` manifest; a `--vod` flag would be a no-op. Check what the muxer flag already emits before wiring a flag for it.
- **CSV `TIME,TITLE` splits on the FIRST comma** — `split_once(',')` keeps commas inside the title; quote-wrap titles containing `,`/`"` on export and unquote+unescape (`""` → `"`) on import. A header row ("Timecode,Name") is only a header when the time column fails to parse on line 1 — parse first, then skip.

- **Overlap-clamping can cascade a cue to zero** — `subs --fix-overlaps` clamps each end to the next start; when a cue is fully swallowed (next cue starts at-or-before this one), the clamp zeroes it and it's dropped from the output. Run `--dedupe`/`--fix-overlaps` AFTER `--sort`: a duplicate cue still clamps (and drops) against its twin, which is why the report counts `clamped` separately from `dropped`.

- **`subs --min-dur` caps at the next cue's start** — extending a flash cue past the next start would re-create the overlap `--fix-overlaps` just removed; the extension is `min(start+min_dur, next.start)`. A cue that can't reach the floor (adjacent cues) still reports in `extended` with a partial extension.

- **Speaker labels need digits + underscore, not just caps** — transcript exports write `SPEAKER 1:`/`SPEAKER_1:`; an all-caps check that rejects digits misses the commonest label form. ffkit's strip requires ≥1 uppercase letter AND only [A-Z0-9_ .'-] before the colon, so `Note:`/`Monday:` survive.

- **`subs --append` offsets by the first file's last CUE end, not the clip's duration** — if the A-side video runs past its last subtitle (credits, outro silence), the appended cues land early. Verify the first clip's length before joining, or shift the joined file afterwards with `--shift`.

- **ffprobe 的 disposition 键表 ≠ ffmpeg 4.4 能写的键**：`karaoke`/`lyrics`/`clean_effects` 设上 mkv 静默丢（写不出），`timed_thumbnails` 连选项解析都过不了（Undefined constant）；4.4 实际可写的只有 default/dub/original/comment/hearing_impaired/visual_impaired/forced/attached_pic。
- **A repeated `-disposition:a:N` / `-disposition:s:N` option REPLACES, not merges** — `+flag` only accumulates within one option: `-disposition:a:0 +visual_impaired -disposition:a:0 +dub` lands dub only. ffkit concatenates every flag for the same track into a single `+visual_impaired+dub` option.
- **mp4/mov can't express the FORCED subtitle flag** — `-disposition:s:0 forced` is accepted silently and writes `forced:0` on tx3g/mov_text; only matroska-family containers carry the flag. `remux --forced-sub` refuses mp4/mov/m4a outright rather than ship a silent no-op — verify the flag with `ffprobe -show_entries stream_disposition=forced`, not by exit code.
- **Track-order indices are PER-TYPE, not absolute** — `--video-order 1,0` indexes the video-track list (`0:v:N`), matching `--audio-order`/`--sub-order`; absolute stream indices are `--keep`'s job. Mixing both errors out (a video index means nothing in the audio list).

- **`-dump_attachment:t` is an INPUT option and a bare `t` dumps ALL attachments** — it goes BEFORE `-i` and the next arg is the dump filename. `-dump_attachment:t` (no index) writes every attachment to that one name sequentially (overwrite prompt); `-dump_attachment:t:N` picks the Nth. The run still needs an output — sink it to `-f null -`. The dumped file is the raw payload, not a media file: skip the output-probe (`write_job_raw`).
- **ASS `Format:` fixes the column order — parse it, don't guess** — `[Events]` `Format: Layer, Start, End, Style, …, Text` names the columns; Dialogue lines then split at `n_cols-1` commas so the Text field (always last) keeps its own commas. `{}` blocks are override markup — strip them on import and escape `{`/`}` to parens on export.
- **`ffprobe csv` prints one row per matching stream, not a joined list** — `-select_streams v -of csv=p=0` on a dual-video file prints `0\n1`, not `0,1`.

- **mp4/mov drop stream-level `title=` for EVERY kind** — `-metadata:s:v:0 title=`/`:a:`/`:s:` never lands in movenc's output (the format-level whitelist is separate and does keep `title`/`artist`/…). `--title-audio`/`--title-subs`/`--title-video` need an mkv/webm output; verify with `probe` `tags.stream:N.title`, not exit code.
- **dashenc on ffmpeg 4.4 has no CENC options** — `-encryption_scheme`/`-encryption_key` are movenc-only (dashenc gained them in 5.x). For encrypted DASH, `remux --encrypt` the mp4 first, then `dash --copy` the encrypted file.

- **`-map -0:t` hits attachment streams, NOT attached_pic cover art** — cover art is a *video* stream with an `attached_pic` disposition, so `--no-cover` (index-based negative maps) and `--no-attachments` (`-0:t` specifier) are different jobs. Check kinds with `probe` `streams[].kind == "attachment"` vs `attached_pic_indices`.
- **ffmpeg 4.4's mkv AND mp4 muxers refuse raw data streams** — "Only audio, video, and subtitles are supported for Matroska" / "codec not currently supported". Camera telemetry/GPMF tracks can only be dropped (`-map -0:d`), never remuxed on this version.

- **`-af`/`-filter:a` can't sit on a `-filter_complex` output** — stream-specifier filters attach to mapped *input* streams; when the audio comes OUT of a graph (a concat label), the gain must ride inside the graph chain itself (`live --volume` injects `,volume=` into the audio chain, or builds a bare `[0:a]volume[r][avol]` graph when nothing else needs one). An audio-only graph leaves the video map empty — fall back to the raw `-map 0:v`, not the graph's (missing) video label.
- **`chapter --rate` applies before `--shift`** — retiming is fixed scale-then-offset (the same order `subs --resync` solves). Marks authored for a sped-up cut divide by the rate; check a mark past EOF isn't created — the end-of-file guard still rejects at the scaled time.

- **clap won't take a bare negative for an `f64` flag** — `--audio-offset -1` parses `-1` as a new flag (exit 2, no JSON). Negative values only pass in `--flag=-1` equals-form, which reaches the verb's own range check.

- **A short `--audio` bed silently pads with `apad`, not music** — the slideshow bed chain is `apad,atrim=duration=…`: past the bed's end the montage gets digital silence (−91dB tail). `-stream_loop -1` is an input option BEFORE `-i` (`--audio-loop`) and repeats the bed instead; the `afade` tail then fades real audio, not silence.

- **`.srt` load already strips `<…>` markup** — `parse_srt` runs every cue line through the tag-stripper, so `subs --strip-tags` only ever *counts* cues carrying `{\…}` ASS override blocks in `tags_stripped` (the `<i>`/`<b>` cues were already clean at ingest). The flag still handles both — the count is "cues whose text changed", not "tags seen".
- **Chapter exports write *defined* marks, not embedded ones** — `--srt`/`--csv`/`--vtt`/… serialize the marks you set with `--at`/`--import`/`--spread`/`--auto`/`--scenes` in the same run; embedded container chapters only come back via `--list`. `chapter file --srt` alone fails "needs --at …" by design.

- **`subs --strip-sdh` treats every `(…)` span as an annotation** — SDH convention uses both `[SOUND]` and `(door creaks)`; a parenthetical that's real dialogue ("(whispered) yes") is stripped too. The flag is for broadcast/transcript files where parens are annotation-only.
- **ffprobe `profile` is a per-stream string, not a number** — h264 reports "High 4:4:4 Predictive"/"Baseline"/"Main", aac reports "LC"/"HE-AAC". Match it as text (case-sensitive), never parse an int.

- **`-fflags +genpts` is an input-side flag** — it must precede `-i` to rebuild timestamps at demux; placing it after does nothing. It only repairs missing pts — it does NOT renumber gaps (a 2s pts hole still plays as a 2s stall; re-encode for that instead).
- **`subs --clip` rebases the window to 0** — cue `1.5–2.0` clipped at `F=1.5` lands at `0–0.5`; `end` resolves to the last cue's end (an .srt has no container duration to probe).

- **`streams[].duration`/`bit_rate` can be absent on some containers** — ffprobe reports them only when the muxer recorded per-stream duration/rate (mp4/mkv yes, some raw streams no); treat missing as unknown, not zero.
- **EDL frame numbers use the clip's probed fps** — a VFR file exports marks at its average rate; NLE timelines running the same rate line up, a mismatched-timeline import lands off by drift.
- **`--no-audio` reads as "no audio" inside flag guards** — order conflict checks before has_audio checks or the wrong error fires (`live --channels --no-audio` should say "conflicts", not "input has no audio").

- **`subs --join` joins text with a space, not a newline** — merged fragments read as one sentence; run `--wrap` after if portrait captions need re-breaking.
- **`extract --cover` writes the embedded codec's bytes verbatim** — a mjpeg cover named `.png` still contains JPEG bytes (extension should match; the stream's codec is in the same probe's `streams[]`).
- **Multiple attached pics: `--cover` takes the first** — `probe.attached_pic_indices` lists them all; pull a later one with `ffmpeg -map 0:<idx> -c:v copy` when needed.

- **`streams[].color_space` is absent when untagged** — most encodes never write the tag (ffprobe reports nothing, not "unknown"); verify color metadata presence first, don't read absent as bt709.
- **`subs --drop` can't split one cue's text** — a cue spanning the whole cut keeps only its head (text covers both sides, so the surviving head carries it all); clips landing inside the window drop entirely.
## live --audio-delay rides the audio filter chain
adelay must run inside `-filter_complex` (it pads with silence — `-itsoffset`
can't pad inside one graph). It chains after `volume` on `[amap]` and inside the
slate arm's aformat chain. `--no-audio`/no-audio input rejects upfront — a bare
adelay flag needs the pushed aac path.

## .lrc has no per-line end time
LRC lines carry only a start `[mm:ss.xx]`; each line's end is the next line's
timestamp (cue end data is dropped on export — re-importing an .lrc won't
rebuild cue durations). Multi-line cues join with a space, one line per cue.
## graph-only live flags must join use_fc
`live` builds `-filter_complex` only when slate/overlay/hold asks for it —
a flag that feeds the graph (like --hold's tpad) but doesn't flip `use_fc`
either silently drops out (no graph emitted) or collides `-vf` with
`-filter_complex`. New graph flags: add to `use_fc` AND emit the content
chain when `vout` is still empty.

## subs --move counts the input file, not the result
Cue N is the index in the .srt as written (1-based) — --move runs first
inside tidy before --clip/--drop/--sort filters renumber or drop cues.
A re-seated cue can overlap its neighbor; chain `--fix-overlaps` to clamp.
## field_order progressive vs absent
ffprobe reports `field_order=progressive` on tagged progressive tracks,
`unknown` (filtered) on untagged, `tt`/`tb`/`tff`/`bff` on interlaced —
absent ≠ progressive, just means the codec/container didn't tag it.
## select frame indices need the escaped comma
`select='eq(n\,15)'` inside a filter string escapes the option-comma —
in a Rust format! that's `\\,` in source. `eq(n,15)` unescaped splits
the filter args and ffmpeg fails with Undefined constant.
## dynamic loudnorm stderr noise
One-pass `loudnorm` on the live push chain prints its loudness stats to
stderr each interval — cosmetic progress noise, not a failure. The
normalized spec is the default −23 LUFS broadcast target; measured
two-pass normalization stays the `loudnorm` verb's job.
## itsoffset measures approximate
`-itsoffset 0.4` lands ~0.377s on the stream's start_time — timescale
rounding (the demuxer quantizes to the codec's tbn). Assert per-stream
start_time with a window (>0.2), never an exact equality.

## ffprobe sentinels and movenc drops
Codec `level` reads `-99` for untagged streams (ffprobe's "no level") —
`streams[].level` filters it out rather than reporting a nonsense value.
`-metadata itunes_advisory=1` is silently dropped by the mp4/mov muxer:
the iTunes explicit/clean tag can't be written via ffmpeg 4.4.
`chapter --at 0.3` anchors the FIRST mark at t=0 (container chapters
start at 0) — later marks keep their times.

## TTML/DFXP is muxer-only in ffmpeg 4.4
ffmpeg 4.4 can WRITE ttml but has no demuxer — `subs --convert x.ttml`
round-trips through ffkit's own `<p begin end>` parser, not libavformat.
The parser reads begin/end attributes (H:MM:SS.mmm or HH:MM:SS:FF) and
`<br>` line breaks; styling, regions, and `<div>` markup are ignored.
`r_frame_rate` is the codec's declared base rate — on CFR files
`r_fps == fps`; a mismatch per track means VFR footage, not a bug.

## x264's SPS num_ref_frames is always 1
ffprobe `refs` reads the SPS field, and libx264 writes 1 there no
matter what `-refs`/`-x264-params refs=N` you pass — it can't QC the
encoder's reference-frame setting (probe the encode log, not the file).
`is_avc`/`nal_length_size` are the reliable payload-shape check instead:
mp4/mov h264 is length-prefixed avcc (`is_avc` true, `nal_length_size`
4), mpegts/raw is annex-b (`is_avc` false, size 0) — mpegtsenc inserts
h264_mp4toannexb automatically on remux.
`.csv` subtitle rows keep real newlines inside a quoted cell — Sheets/
Excel render them as in-cell line breaks, and `parse_csv_subs` only
treats a row as a cue when the first two fields both parse as times
(the header row and malformed lines drop quietly).

## nb_programs 0 vs "no programs"
Program-less containers (mp4/mov/mkv/webm) report `nb_programs: 0` in
ffprobe — only multiplexed ts/spts carry a real count. `program_count`
filters the 0 to absent rather than claiming a file "has no programs"
(it never had the concept). `dash --name PFX` prefixes segment AND
init AND single-file names — the written-segment check counts against
the same prefix, so a custom name can't fake "no segments written".

## -map 0:p:N picks by program NUMBER, and Vec contract fields need serde(default)
`ffmpeg -map 0:p:2` selects the program whose program_num is 2 — not the
second program in the list. `-show_programs` reports num/service_name
(`tags.service_name`)/member stream indices, but member streams carry NO
program_num field — stream-level program attribution is impossible in 4.4.
Probe `Vec` contract fields must carry `#[serde(default)]` alongside
`skip_serializing_if = "Vec::is_empty"` — an empty vec serializes without
the key, and pipeline re-deserializes each step's contract: a missing key
fails the whole parse ("no probe" on every expect check).

## A program map can't compose with a media-type specifier
`ffmpeg -map 0:p:2` takes the WHOLE program — `0:p:2:a` or `0:p:2:v`
aren't valid stream specifiers. `hls`/`dash --program` therefore map
differently per path: the plain path emits one `0:p:N` map, but
`--ladder` resolves `probe.programs[]` member indices and maps them by
absolute index (`[0:IDX]` for the ladder's video input, `0:IDX` for the
service's first audio member). Member streams carry no program_num in
ffprobe 4.4 — attribution is only reachable through programs[].streams.

## `--dedupe` needs identical timing AND text; `--dedupe-text` drops same-text neighbours

`subs --dedupe` removes a cue only when the previous cue's start AND text match exactly (broken exporters that double-emit a cue). Whisper-style transcripts print the same sentence at *different* times — `--dedupe` misses those; `--dedupe-text` compares normalized text (case/space-insensitive) against the previous KEPT cue and drops repeats regardless of timing. Both count into extras `dropped`. `chapter --min-gap` is the chapter-side analogue: it filters marks by spacing after the dedup/sort passes, so the first mark is always kept.

## `concat` silently re-encodes when input specs differ

`concat` picks the stream-copy path whenever the inputs match codec/size/rate — and quietly falls back to a full filter re-encode when they don't. There's no warning: a "lossless join" on mismatched inputs isn't. `concat --copy` turns the auto-pick into a gate — it refuses mismatched inputs (and the re-encode flags `--transition`/`--level`/`--gap`/`--audio-fade`) instead of silently losing the copy.

`chapter --snap` re-seats marks on the nearest keyframe: two marks can snap onto the SAME keyframe, so a second dedup pass runs after snapping (the `snapped` extra counts moves before dedup).

## `probe.encrypted` is a byte-scan, not an ffprobe field

ffprobe 4.4 never reports CENC encryption — `remux --encrypt` output still shows `codec_tag: avc1`/`mp4a` and no encryption side data. `probe.encrypted` instead scans the file's first and last 1MB for the `sinf`/`encv`/`enca`/`schi` box signatures those files carry. URLs skip the scan (remote files can't be byte-read) and report `false`.

## `concat --repeat` expands the input list before validation

`--repeat 3` multiplies `inputs` before the two-input minimum runs, so a single file loops cleanly (`concat a.mp4 --repeat 4` → 4 plays). Transitions/fades/gaps all see the expanded list — every repeat seam gets the joint treatment, which is what loop-intended assemblies want.

## `subs --mux` takes the video as INPUT, the .srt as the flag value

`subs VIDEO --mux subs.srt -o out.mkv`. Swapping them (`subs subs.srt --mux video.mp4`) does NOT error: ffmpeg demuxes the mp4 as the "subtitle" input, `-map 1` then just picks its streams, and you get a clean lossless repack with zero subtitles — it even probes ok. Only `probe.has_subs` tells you the mux actually landed.

## `-muxrate` pads, it does not throttle

`remux --muxrate` on a .ts target inserts null packets to reach the constant mux rate — the file gets LARGER than the sum of its streams. It's for broadcast ingest specs that require a fixed-rate transport stream; it won't shave a bitrate-heavy file down.

## `-level:v` is profile-shaped, not a free cap

x264's `-level:v` constrains the encode budget (macroblock rate, buffer size) — it can only tighten what the profile allows, and it reports as a NUMBER on probe (`3.1` → `level: 31`, `4.1` → `41`). Pair it with `--profile`: `--level` alone on a High encode still leaves High features in.
- **`-maxrate`/`-bufsize` unqualified hits the audio codec too** — ffmpeg's codec-option matching applies them per-stream, and aac rejects a bufsize it can't parse. Always emit `-maxrate:v`/`-bufsize:v` (deliver does). Related: deriving bufsize as `format!("{m}x2")` is a literal string, not arithmetic — parse the number and double it, preserving the k/M suffix.

## `.sub` MicroDVD times are FRAME numbers, not seconds

`{100}{150}line` means frames 100→150 — seconds = frame/fps. A `{1}{1}fps` declaration line inside the file wins over `--fps`; without either the convert refuses rather than guessing 25.

## `conform --maxrate` alone counts as a conform op

The "nothing to conform" gate lists every transform flag — forgetting `--maxrate`/`--bufsize` there makes a rate-cap-only run report "nothing to conform" even though it would re-encode.

## Digit-leading platform names need `#[value(name = …)]`

clap derives value names from the Rust variant (`Live17` → `live17`), so `--platform 17live` is rejected unless the variant carries `#[value(name = "17live")]` — same trick as `NineGag` → `9gag`.
