---
name: testing-ffkit
description: How to end-to-end test ffkit verbs on this machine — toolchain PATH, fixture synthesis with ffmpeg lavfi, JSON-contract assertions, and objective audio/video effect verification (volumedetect, band analysis, frame pixel checks). Use when testing ffkit CLI changes.
---

# Testing ffkit

ffkit is a Rust CLI wrapping system ffmpeg; every verb emits a JSON contract on stdout.

## Build & invoke

- cargo is NOT on PATH. Prepend:
  `export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`
- `cargo build`, then run `target/debug/ffkit <verb> ...` (faster than `cargo run`).
- ffmpeg/ffprobe come from Homebrew and are on PATH (ffmpeg 9.x). Check filter availability with `ffmpeg -filters | grep <name>` — this machine has afftdn/hqdn3d/gblur/showwaves/colorkey but LACKS libass/zscale.
- Assert on the contract: `{status, tool, output, probe{duration,width,height,vcodec,acodec,has_video,has_audio,size_bytes,format}, commands[][], extra, verified}`. `status` is `ok`/`failed`/`dry_run`. `commands` shows the exact ffmpeg argv — use it to prove stream-copy vs re-encode (`-c:v copy`) or which filters ran.
- Error contracts return `status:"failed"`, `error.kind:"input"|...`, `error.message` — and must NOT leave an output file. `--overwrite` is required to reuse an output path.

## Fixture synthesis (lavfi)

Synthesize media instead of hunting for real files. Keep distinct colors for multi-clip tests so cutaway windows are pixel-verifiable:

- Noisy voice: `sine` + `anoisesrc=color=white:amplitude=A` + low-freq `sine` rumble, mixed via `amix=inputs=N:normalize=0`, `-ac 1 -ar 48000` wav.
- Grainy video: `testsrc2=size=WxH:rate=R:duration=D,noise=alls=25:allf=t` + sine/noise audio → h264. NOTE: noisy video barely compresses — a 20s 720p grainy clip lands ~140MB at crf23; great for `compress` squeeze tests.
- Solid-color insert (`color=c=red:s=1280x720`) makes `broll`/`fit` windows distinguishable from a testsrc2 A-roll — without distinct colors the two are statistically identical.
- No-audio video: `-an` for error-path fixtures.

## Objective effect verification

- **Audio level**: `ffmpeg -i F -af volumedetect -f null -` → mean/max_volume. Band-isolate before measuring: `-af "lowpass=f=80,volumedetect"` (rumble band), `highpass=f=2000` (hiss band), `atrim=A:B` (time window). `astats=metadata=1` gives RMS/min/max.
  - GOTCHA (ffmpeg 9.x): `afftdn=nr=X` alone is nearly a NO-OP — `nf` (noise floor) defaults -50 dB and `tn` (track_noise) defaults false; raising `nf=-20` is what makes it actually cut (~12 dB). `afwtdn=sigma=0.02-0.08` is stronger (~16 dB hiss / <1-3 dB voice) but only exists in ffmpeg ≥5.1 — the verb prefers it and falls back to `afftdn:nf=-20`. `anlmdn` SEGFAULTS on this build (ffmpeg 9.0.1). Verify any denoise claim with band volumedetect numbers, not just contract status. Methodology sanity: `volume=0.5` must read ~-6 dB.
- **Visual bars/fills**: extract a frame (`-vf select=eq(n\,N) -frames:v 1 f.png`), dump rawvideo rgb24 into python3, and measure band mean luma + adjacent-pixel gradient. Blurred fill ⇒ mean>20 AND low |dx| vs sharp center; black pad ⇒ mean≈0. Compare `--fit pad` vs `--fit blur` on the SAME frame.
- **Overlay/keying**: for `audiogram`-style keyed strips, flat-bg variant should be pixel-exact background color outside the strip; waveform pixels are achromatic-gray (|r-g|,|g-b|<15). NOTE: `showwaves` defaults `draw=scale` which renders the trace at ~55-60% intensity (~145 gray, never white) — `draw=full` gives full 255. A "white waveform" claim needs `draw=full`.
- **Size budgets**: two-pass `compress` lands ~1-6% under `--size` (0.98 mux reserve). `size_bytes ≤ target` in the output probe. Impossible targets refuse BEFORE encoding (fast fail), e.g. video bitrate would be <64 kbps.
- **Duration preservation**: probe.duration in the output contract should match input within ~0.5s; audiogram should end near-exact at audio EOF (overlay `shortest=1` semantics, NOT -shortest alone).

## Devin secrets needed

None — everything is local CLI + Homebrew ffmpeg.
