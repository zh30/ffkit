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

`speed --factor F --at S --dur D` is a speed ramp: same 3-segment concat, middle `setpts=(PTS-STARTPTS)/F` + `atempo`. `censor --region x:y:w:h` mosaics with 16px `pixelize` cells (or `--mode blur` → `gblur sigma=30`).

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
