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

## loudnorm JSON

The first pass prints measured values on stderr. ffkit parses them for the second pass. Do not use `-loglevel quiet` on a hand-rolled loudnorm; you will lose the measurements.
