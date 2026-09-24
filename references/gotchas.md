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
  Its box lands in `lavfi.rect.{x,y,w,h}` frame metadata.
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
  `find_rect=object=x,cover_rect=mode=blur` tracks a moving mark live;
  the object bitmap must be grayscale (color errors out).
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
