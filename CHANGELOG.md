# Changelog

## [Unreleased]

## [0.285.0] — 2026-09-24

### Added
- `extract --subs` — pull an embedded subtitle track out to a text file: `-o` .srt/.ass/.vtt picks the caption container, `--track N` picks the language in multi-sub files (captions out of a finished export for re-timing or re-burning)
- `chapter --spread N` — generate N evenly-spaced marks on a `duration*i/N` grid (uniform TOC for long episodes/lectures); `--titles a,b,c` supplies the names (must match N), default `Chapter 1..N`; works with every export flag and the embed path
- `probe`/`scan` report `tags` — every container + stream metadata tag grouped by source (`format`, `stream:0`, …); metadata audit: what did the last export actually stamp

## [0.284.0] — 2026-09-24

### Added
- `split --copy` — lossless stream-copy splitting through the segment muxer: instant for long recordings, boundaries snap forward to the next keyframe (not frame-exact); works with `--every`/`--at`/`--parts`/`--size`/`--scenes`/`--chapters`/`--silence`; `--fade` is refused
- `chapter --vtt` — export chapter marks as a WebVTT chapter track (web `<track kind="chapters">` click-to-seek); `--import` now auto-reads `.vtt` files too (round-trips)
- `probe`/`scan` report `has_alpha` — the video stream's pixel format carries an alpha plane (yuva*/rgba family); sticker/overlay asset QC

## [0.283.0] — 2026-09-24

### Added
- `meta --media-type music|musicvideo|tvshow|movie|audiobook` — iTunes `stik` atom (movenc `media_type` tag, verified on ffmpeg 4.4 m4a: `media_type=10`); podcast/music app library sorting
- `meta --gapless` — iTunes `pgap` atom (`gapless_playback=1`): continuous albums, live-split tracks and DJ mixes shouldn't gap between tracks
- `scan`/`probe` now report `timecode` — container timecode QC: mov tmcd → video-stream `timecode` tag, mkv `TIMECODE` format tag; zero decode, key absent when the master carries none (verify a slate-matched TC on deliverables; pairs with `remux --timecode`)

## [0.282.0] — 2026-09-24

### Added
- `remux --timecode HH:MM:SS[:FF]` — write a start timecode into the repack: mov/mp4 get a real `tmcd` track plus the video-stream tag, mkv gets the `TIMECODE` format tag — dailies matching a camera slate
- `remux --default-sub N` — pick the default subtitle track on multi-language sub deliverables (bounds-checked against the input's subtitle count)
- `extract --audio --track N` — pull a specific audio track losslessly (commentary/stem out of a multi-track file; default first track unchanged)
- `meta --creation-time auto` — stamp the input file's own mtime as ISO-8601 creation_time (shoot/download day without typing a date; civil-from-days conversion, no chrono dep)

## [0.281.0] — 2026-09-24

### Added
- `split --black` — blackdetect (≥0.3s at 98% black) finds the dead stretches, then every non-black keep segment renders to its own `stem_NN.ext` part (frame-accurate re-encode): dead-air chapterization for event/talking-head footage — the video twin of `split --silence`
- `remux --tag TAG` — rewrite the video codec tag without re-encoding (`--tag hvc1` makes HEVC mp4s play in QuickTime/Safari; mp4/mov only)
- `remux --attach FILE` — embed a binary attachment stream into mkv/webm repacks (subtitle fonts travel inside the file with styled subs; repeatable, mimetype stamped per file)
- `meta --creation-time`, `--location` — archive shoot-day stamp + ISO 6709 geo stamp; both land in mp4/mkv (movenc normalizes location to +DD.DDDD+DDD.DDDD/)

## [0.280.0] — 2026-09-24

### Added
- `cut --black` — auto-detect and excise black stretches (blackdetect ≥0.3s at 98% black) joined via the drop path: dead-air trim for talking-head/event footage, the video twin of cutsil
- `remux --video-delay SEC` — the other half of `--audio-delay`: shift the video track against the audio without re-encoding (capture cards that lag the picture; negative advances it by delaying the rest)
- `probe`/`scan` report `av_desync_ms` — |audio start − video start| in ms read from container stream start_times: lip-sync QC with zero decoding, paired with `remux --audio-delay`/`--video-delay` to repair
- `meta --album-artist`, `--show`, `--season`, `--episode`, `--network` — TV-series and podcast-feed tag set (verified landing in mp4/m4a/mkv: `album_artist`/`show`/`season_number`/`episode_id`/`network`)

## [0.279.0] — 2026-09-24

### Added
- `slideshow --list FILE` — read the still order from a manifest (one path per line, `#` comments, relative paths resolve against the list's directory): curated order beyond `--sort`
- `scan --bitrate` — video-bitrate curve from the packet map without decoding: `bitrate_mean_mbps`, `bitrate_peak_mbps` (worst 0.5s window), `bitrate_spike_at` for platform peak-rate ingest specs
- `meta --copyright` — copyright/license tag (lands in mp4 too)
- `conform --channels 1|2` — force mono/stereo inside conform (mono podcast masters), alongside `--ar`
- `transcode --copy-video` — stream-copy the picture while re-encoding only the audio (fix bad audio / repack); video filter flags are rejected with it

## [0.278.0] — 2026-09-24

### Added
- `concat --list manifest.txt` — read clip paths from a manifest file (one per line, '#' comments, relative paths resolve against the list's directory): script-generated assemblies without positional args
- `scan --hash` — writes `<input>.framemd5` decoded-frame checksums of every stream (archive-ingest integrity manifest; `hash_file`/`hash_frames` in the report)
- `chapter --import` now reads `.lrc` synced-lyrics files — `[mm:ss.xx]title` marks import alongside .json/.cue (round-trips with `chapter --lrc`)
- `meta --composer`, `meta --bpm`, `meta --lyrics file.lrc` — music-release tags; lyrics files get LRC timestamps stripped into the unsynced-lyrics tag (bpm lands in mp3/mkv/flac; mp4 drops it)
- `conform --ar HZ` — resample rate override inside conform (48k broadcast stays the default; 44100 for podcast/CD deliverables, 96000 for masters)

## [0.277.0] — 2026-09-24

### Added
- `scan --gop` — keyframe-interval QC straight from packet flags (zero decode): `keyframes`, `gop_max_sec`, `gop_avg_sec`, `gop_max_frames` — ingest specs like "keyframe every ≤2s" verified on any master, plus the tail stretch after the last key
- `conform --even` — floors odd pixel dims to even: phone/screen captures at odd px can't encode yuv420p x264; one flag fixes it inside the normal conform pass
- `countdown --target HH:MM[:SS]` — real-time count to a local wall-clock time (premiere/stream-start overlays): 1s per count up to 10 min ahead, a passed time rolls to tomorrow
- `chapter --lrc` — exports marks as synced-lyrics `[mm:ss.xx]title` lines (music players show them as seekable verse/track cues); also lets bare `chapter --podcast`/`--lrc` skip `--at` (the required-flags list now includes both)
- `meta --disc N[/total]` — disc-number tag for multi-disc album releases

## [0.276.0] — 2026-09-24

### Added
- `dash --ladder 1080,720,480` — ABR packaging: one encode per rung → N video Representations at tiered bitrates (4500/2800/1400/800/500k) in a single AdaptationSet, audio in its own group; players switch rungs with bandwidth. 2..=6 heights, rejects `--copy`/`--audio-only`
- `live --start T` — input-side seek before `-re` pacing begins: start the stream T seconds into the source (replay archives mid-way, skip a long event's dead head); rejected on `--list`/`--test`
- `remux --audio-delay SEC` — shifts the audio track against the video without re-encoding (second `-itsoffset` read of the same file; positive delays audio, negative advances it). Lip-sync repair on a lossless repack; works with `--no-subs`/`--cover`/`--chapters`, rejects `--audio`/`--video`/`--lang` stream-picks
- `scan` — HDR & wide-gamut QC straight from container colour tags (zero extra decode): `hdr` (PQ/HLG transfer), `wide_gamut` (BT.2020 primaries), plus raw `color_space`/`color_primaries`/`color_transfer` fields on the probe

## [0.275.0] — 2026-09-23

### Added
- `live --to srt://…` — SRT ingest transport: SRT and UDP URLs both ride the MPEG-TS muxer (contribution-grade links)
- `live --codec hevc` — HEVC stream encode via libx265 for contribution/spec ingest; gated to MPEG-TS transports (the FLV muxer can't carry HEVC on ffmpeg 4.x)
- `live --subs file.srt` — burn an .srt/.ass/.vtt caption file into the live picture in one pass (live-captioned broadcasts without a captioning rig); rides the slate/overlay graph paths too
- `deliver --platform circle` — Telegram video-note (кружок) pack: 640x640 1:1 canvas + mono audio

## [0.274.0] — 2026-09-23

### Added
- `dash` — DASH packaging: input → `manifest.mpd` + `init-`/`seg-*.m4s` segments (`--seg` seconds, `--copy` repack, `--single` one byte-range file per representation, `--webm` vp9+opus segments for open players, `--window N` sliding manifest for live writes, `--audio_only`)
- `remux --encrypt` — CENC AES-CTR (cenc-aes-ctr) on the repack for ClearKey/Widevine DRM prep; ISOBMFF outputs only (mp4/mov/m4a/m4b). `--key`/`--kid` take 32-hex, auto-random otherwise; both echoed in `extra`
- `extract --webp` — animated WebP clip via libwebp (`-pix_fmt yuva420p` keeps alpha natively — sticker/creator exports smaller than GIF); `--lossless` bit-exact, `--bounce` palindrome, `--dur`/`--fps`/`--width`/`--at` all apply
- `deliver --platform x` (1280x720 16:9 feed) and `--platform linkedin` (1920x1080) canvases on the same −14 LUFS pack pipeline

## [0.273.0] — 2026-09-23

### Added
- `hls --time-names` — segments named by wall-clock time (`seg_YYYYmmdd-HHMMSS.ts`) so archived recordings carry their airtime in the filename; conflicts with `--single`
- `hls --independent` — tags `EXT-X-INDEPENDENT-SEGMENTS` and forces a keyframe at every segment boundary (`-force_key_frames` per `--seg`); rejected on `--copy`/`--ladder`
- `extract --gif --transparent` — keeps alpha in the exported GIF (Discord/Telegram stickers) via `palettegen reserve_transparent` + `paletteuse alpha_threshold`; requires an alpha-capable input (prores 4444/qtrle), fails cleanly otherwise
- `remux --no-cover` — drops attached_pic cover-art streams on the repack (per-index negative maps from probe disposition data; `-0:v:m:attached_pic` doesn't work — it's a disposition flag, not stream metadata)
- `chapter --import` now auto-detects `.json` (Podcasting 2.0 `chapters[]`) and `.cue` (TRACK/INDEX mm:ss:ff) by extension, round-tripping with `--podcast`/`--cue` exports
- `deliver --platform pinterest` — 1000x1500 (2:3) idea-pin canvas, same −14 LUFS pack pipeline


## [0.272.0] — 2026-09-23

- `live --maxrate`/`--bufsize` — CBR rate caps on the stream encode (Twitch ≤6000k, YouTube ingest specs); `--bufsize` alone also works, and when unset it auto-fills to 2× `--maxrate` (suffix-aware: `4500k` → `9000k`)
- `hls --date` — stamps EXT-X-PROGRAM-DATE-TIME on every segment (players show real wall-clock times; required by some CDNs/origins)
- `hls --discontinuity` — EXT-X-DISCONTINUITY restart marker; combines with `--live` and other hls flags in one `-hls_flags` set
- `chapter --podcast` — exports Podcasting 2.0 JSON chapters (`{"chapters":[{startTime,title}]}`) alongside the ffmeta/YouTube/CUE exporters
- `slideshow --sort name|mtime` — orders a camera/photo dump by filename or shoot time before `--shuffle` applies

## [0.271.0] — 2026-09-23

- `live --gop N` — keyframe interval on the ingest encode (YouTube spec wants a keyframe every ≤2s ≈ 60f at 30fps; short GOPs also make live restarts reconnect faster)
- `live --preset` — x264 speed/quality on the stream encode (default veryfast; drop to `fast`/`medium` for quality headroom, `ultrafast` for weak machines)
- `live --vertical` — vertical-stream preset: the feed is letterboxed onto a 1080x1920 canvas (TikTok/Reels/抖音 live; conflicts with `--scale`, works with `--overlay`/`--slate`/`--record`/`--restream`)
- `remux --lang eng,jpn` — `--lang` now takes a comma list: every listed audio track is kept (multi-dub releases in one repack)
- `remux --strip-meta` — privacy wipe: `-map_metadata -1` drops every inherited container tag (camera/GPS/app metadata) on the repack; combines with `--title` etc to retag in one pass

## [0.270.0] — 2026-09-23

- `live --restream url` — multistream in one encode: the tee muxer fans the same x264/aac encode out to a second ingest URL (`rtmp/rtmps/tcp/udp`, scheme-checked like `--to`; combines with `--record` for a 3-way push)
- `deliver --platform douyin|kuaishou|bilibili` — Chinese feed presets: 抖音/快手 9:16 1080x1920 and B站 16:9 1920x1080 packs on the same −14 LUFS pipeline as every other platform
- `remux --title/--artist/--album/--genre/--comment/--date` — container library tags written on the repack (music/audiobook metadata fix without a re-encode; `extra.tags` counts what landed)
- `timer --clock` — wall-clock burn-in: readout seeded from the local system clock (TZ offset via `date +%z`, UTC fallback) so the overlay reads HH:MM:SS of day — event/sports overlays, premiere countdowns
- `hls --start N` / `--epoch` — media-sequence continuity: `--start` resumes segment numbering at N after a stream restart (seg_NNN names + `EXT-X-MEDIA-SEQUENCE`), `--epoch` seeds it from the epoch clock so 24/7 channels stay continuous without tracking N

## [0.269.0] — 2026-09-23

- `live --overlay bug.png` — channel bug burned onto the stream: logo auto-scaled to ~10% of the stream width and pinned to a corner (`--overlay-position tl|tr|bl|br` default br, `--overlay-opacity` 0..1 ghost) — broadcast corner branding without a pre-render
- `live --card art.png` — 24/7 lofi-radio mode: an audio-only source gets a persistent still as its video track (looped card + music to any ingest URL); rejects video sources (use `--overlay`) and `--test`/`--slate`
- `remux --cover pic` — attach feed art on the repack: an `attached_pic` mjpeg stream joins the copy — audio rips get cover art, video files get a thumbnail poster (forces `-f mp4` on m4a/m4b/mp4/mov so ffmpeg 4.x writes it)
- `remux --chapters marks.txt` — container chapters on any repack from the YouTube-format list `chapter --yt` exports (shares `chapter.rs`'s ffmetadata helpers with `deliver --chapters`); marks at/past the input end are rejected with a clear error instead of ffmpeg's cryptic ffmetadata failure
- `hls --live` — sliding-window live playlist: `EXT-X-PLAYLIST-TYPE:EVENT` + `delete_segments+omit_endlist` keeps only the newest `--live-window N` segments (default 6) so players can join mid-write

## [0.268.0] — 2026-09-23

- `deliver --platform audiobook` — audiobook feed pack (m4b, AAC 96k/48k, −16 LUFS): Apple Books/Audible-ready single file; forced `-f mp4` so chapters + cover write past the ipod-muxer limits
- `deliver --chapters marks.txt` — embed real container chapters on podcast/audiobook packs from the YouTube-format list `chapter --yt` exports (`mm:ss title` → ffmetadata `[CHAPTER]` table → mp4 chpl); Apple Podcasts/Apple Books show them as seek stops
- `deliver --title/--author/--album/--genre/--comment` — feed metadata tags written into every pack (author → artist): episode title, show name, audiobook synopsis ride inside the file
- `live --slate card.png [--slate-dur SEC]` — starting-soon card before the content: normalized to the stream canvas with a silent bed, concat-joined ahead of the feed (premiere/scheduled-start countdown card)
- `remux --default-audio N` — make audio track N the default on multi-track files (clear every audio default flag, set the pick): players lead with the language/edit you choose

## [0.267.0] — 2026-09-23

- `remux --lang LANG` — keep only audio tracks tagged with a language (ISO-639: `eng`, `jpn`, …): multi-language releases down to one track, dub extraction; positive `-map 0:a:m:language:L` fails loudly when nothing matches; with `--audio` rips just that track
- `slideshow --shuffle SEED` — deterministic photo order: same seed → same order across runs (xorshift Fisher-Yates), reroll photo-dump montages without re-listing files; `extra.order` reports the resolved order
- `deliver --intro clip.mp4` / `--outro clip.mp4` — channel bumper + CTA card baked into every platform export: each segment normalized to the platform canvas (scale/pad/setsar/fps + 48k stereo), concat-joined, loudnorm applied to the whole deliverable (folded into filter_complex — `-af` can't touch a complex-fed stream); subs ride the main segment, `--logo` overlays post-concat so it covers all three
- `deliver --platform podcast --cover art.png` — embedded feed art: cover becomes an `attached_pic` mjpeg stream on the m4a (Apple/Spotify show art); forced `-f mp4` since the `.m4a` ipod muxer rejects video streams on ffmpeg 4.x

## [0.266.0] — 2026-09-23

- `live --list` — concat-manifest rotation channel: input becomes a text file of `file 'x'` lines, `-f concat -safe 0` streams them back-to-back; `--loop` loops the whole list (24/7 channel, rotation replays)
- `live --test` — no input needed: built-in lavfi testsrc2 card + 1kHz tone pushed to the ingest URL (verify the stream key and measure the pipe before showtime)
- `deliver --logo mark.png` — corner watermark burned in during the pack render (second input → filter_complex overlay at 18% frame width); `--logo-position tl/tr/bl/br` (default br), `--logo-opacity 0..1` (ghost marks)
- `slideshow --fit` — solves `--per` from the audio bed's probed duration: the montage ends exactly on the song's outro (exclusive with `--dur`)
- `remux --from SEC` / `--to SEC` — lossless segment repack: input-side `-ss` keyframe seek + `-t` limit, no re-encode (frame-exact trims stay with `cut`/`split`)

## [0.265.0] — 2026-09-23

- `speed --fit SEC` — retime the clip to an exact length: factor = input duration / SEC is derived automatically (a 90s take `--fit 15` becomes 6x; same 0.25..8 factor range)
- `deliver --preview SEC` — render only the pack's first SEC seconds for approval QC (canvas + loudnorm chain identical, just shorter; works on every platform incl. podcast)
- `live --record file.mp4` — tee muxer: encode once, mux twice — the ingest URL gets the stream AND a local archive lands on disk (`.mp4/.mov/.mkv/.ts/.flv` picks the container)
- `live --until SEC` — stop the stream automatically after SEC seconds (premiere windows, timed replays)
- `remux --no-subs` — drop subtitle/data streams in a full repack (mkv with embedded subs → clean deliverable mp4; `-map 0 -map -0:s -map -0:d`)

## [0.264.0] — 2026-09-23

- `overlay --mode` — 12 more Photoshop-style blend modes on the full-frame composite (burn, dodge, exclusion, hardlight, softlight, pinlight, vividlight, linearlight, hardmix, negation, subtract, divide, plus glow/phoenix/reflect — 19 total `blend=all_mode` names)
- `deliver --channels N` — force channel count on the pack's AAC track (`--channels 1` mono voice feed, `--channels 2` stereo; works on the video platforms and the podcast feed pack)
- `live --scale WxH` / `--fps N` — downscale + rate-cap the live ingest encode (push a 4K master as 720p, 60fps capture as 30fps — no separate transcode pass)
- `remux --frag` — fragmented MP4 output (`-movflags frag_keyframe+empty_moov+default_base_moof` → moof fragments + mfra footer; playable/streamable while still being written — HLS/DASH/live-ingest pipelines; mp4/mov only)

## [0.263.0] — 2026-09-23

- `deliver --platform podcast` — audio-only feed pack: loudnorm to the podcast spec (−16 LUFS) → m4a AAC 128k/48k (accepts audio-only sources; measured I lands within ±1 LU)
- `deliver --to rtmp://…`/`rtmps://`/`tcp://`/`udp://` — push the rendered platform pack straight to ingest (premiere/replay in one pass; FLV + zerolatency, loudnorm still measured+applied)
- `transcode --ar HZ` / `--channels N` — resample + channel count on every re-encode path (broadcast 48k stereo, podcast mono; `--copy-audio` skips as there's no encoder)
- `scan --loud` — EBU R128 summary in extras: `loud_i`/`loud_lra`/`loud_tp` (platform loudness-spec gate — podcast ≈−16 I, broadcast ≈−23 I); scan now also accepts audio-only inputs for the audio QC legs

## [0.262.0] — 2026-09-23

- `live` — push the clip live to an ingest endpoint: `--to rtmp://…` / `rtmps://` / `tcp://` / `udp://` (real-time `-re` pacing, x264/aac encode; udp → mpegts automatically), `--loop` forever for 24/7 streams and premiere replays, `--vbitrate`/`--abitrate` caps
- `scope --mode safe` — broadcast safe-area guides drawn full-frame: 90% action-safe yellow box, 80% title-safe red box, center cross (composition QC overlay, `--at` windows still work)
- `timer --tc HH:MM:SS:FF` — burn a running timecode readout for dailies/review copies (`;` before FF = drop-frame display intent, counting stays straight)
- `extract --audio` — rip the audio track losslessly (stream copy, no decode/re-encode; -o extension picks the container — pull a music/dialog track for editing)

## [0.261.0] — 2026-09-23

- `deliver --platform xhs|wechat` — 小红书 3:4 (1080x1440) + 微信视频号 6:7 (1080x1260) one-shot packs, same −14 LUFS loudnorm pipeline as the other platforms
- `scan --deadair DB` — dead-air QC leg: silent stretches ≥1s at/below the threshold land in `deadair_secs`/`deadair_ranges` (podcast/talking-head pause map before publish; errors on audio-less input)
- `scan` tone QC — `sat_mean`/`hue_mean`/`y_mean` parsed from the same signalstats pass (washed-out / color-cast / programme-brightness numbers at zero extra decode cost)
- `chapter --cue` — export marks as a .cue sheet (TRACK/INDEX mm:ss:ff at 75fps — audiobook/podcast player chapters; `FILE` media tag follows the input extension, mp3→MP3, wav→WAVE, else BINARY)

## [0.260.0] — 2026-09-23

- `multicam --align` — auto-sync camera B to A by audio cross-correlation inside the switch (no separate `align` pass; two takes that started at different wall times line up in one step. Needs broadband in-sync audio — pure tones don't correlate)
- `channel --mode merge --with B` — amerge: interleave two tracks into one multichannel file (two mono lav mics → stereo host-L/guest-R podcast; two stereo stems → quad). Unlike `mix` it keeps every channel discrete
- `scope --mode graph` — graphmonitor live filtergraph stats card in the corner (frames in/out + queue + pts per filter — encode-pipeline debug viz)
- `title --file notes.txt` — hook text from a file (long cards / generated copy)

## [0.259.0] — 2026-09-23

- `fx --kind wah` — auto-wah: asendcmd sweeps a resonant equalizer peak 350→2700Hz (`--strength` sets the LFO rate) — funk/EDU wobble without a pedal
- `stack --mode mean` — exposure averaging across inputs (long-exposure water/cloud smoothing, HDR-look merges); `--weights` comma list per input, auto-normalized (`3,1` = 75%/25%)
- `transcode --preset proxy` — NLE edit proxies: ≤960x540 veryfast x264 + aac 96k (smooth scrubbing on long takes / multicam dailies — never a delivery format)
- `align --check` — reports `offset_ms`/`direction` without rendering (sync QC gate before a multicam assembly)
- `bars --kind testsrc` — testsrc2 all-in-one animated calibration card
- `gen --pattern silence` — anullsrc digital-black audio bed (silent padding)

## [0.258.0] — 2026-09-23

- `eq --superpass FREQ[:Q]` / `eq --superstop FREQ` — order-10 razor band pass/stop (isolate or kill a whine/whistle/tone anywhere on the spectrum; ~50dB deeper notch than --notch)
- `eq --allpass FREQ:WIDTH` — two-pole allpass phase rotator: symmetrizes lopsided vocal waveforms for free headroom (spectrum untouched — mean stays, peak moves)
- `legalize --flash` (+`--flash-threshold`) — damps photosensitive-epilepsy flash cuts; the fix half of `scan`'s flash_frames QC
- `conform --hold SEC` / `--hold-start SEC` — tpad clones the edge frame: end-card hold & pre-roll without a title card (audio silence-padded to match)
- `transcode --field-order tff|bff|prog` — setparams field_mode relabel: fixes masters tagged with the wrong parity WITHOUT re-weaving frames (the repair the fieldorder filter can't do on demuxed input)

## [0.257.0] — 2026-09-23

- `eq --linear` — linear-phase FIR mode for `--lowpass`/`--highpass`/`--bandpass` (sinc+afir: flat passband, ~60dB stopband, zero phase smear — mastering-safe cuts; no `--at`)
- `eq --subcut FREQ` / `eq --supercut FREQ` — order-10 sub-bass & ultrasonic cuts (mic-stand rumble below the voice band; 20kHz+ hiss/pilot tones on hi-res masters)
- `selective --engine chroma` — chromahold keeps saturated hues in YUV chroma space (uneven subjects the RGB colorhold misses)
- `matrix --engine colorspace` — full conversion incl. primaries + transfer curve (the bt2020↔bt709 gamut path, not just the matrix coeff)
- `dedust --engine temporal` — tlut2 temporal min/max removes one-frame sparkles & VHS dropouts the morphology pass misses
- fix: `-o .wav`/`flac`/`mp3`/`ogg`/`opus` no longer writes an AAC payload — `engine::write_job` retargets `-c:a aac` to a codec the container carries (aac-in-.wav misdecoded on ffmpeg 4.x)

## [0.256.0] — 2026-09-23

- `eq --lowpass`/`--highpass`/`--bandpass FREQ[:W]` — resonant Butterworth filters (synth-style sweeps, rumble/hiss roll-off sharper than shelves; Q width for LP/HP, half-band Hz for bandpass)
- `scan --bbox` — content bounding box QC: `content_detected`, `content_box` (x,y,w,h union over frames), `content_fill` — works on any uniform background, not just black borders
- `transcode --interlaced --interlace-mode weave` — real temporal interlacing: consecutive frames woven into top/bottom fields via `tinterlace=interleave_top` (60p→30i broadcast conversion; default `il` mode unchanged)
- `bars --kind mptest` — mptestsrc encoder-torture cycle (fine detail/ringing zones for codec-preset stress QC)
- `scope --mode palette` — frame palette swatch grid overlay (GIF/8-bit palette QC, showpalette on pal8)
- `blur --engine avg` — avgblur area-average kernel (lightest blur; huge-radius washes)

## [0.255.0] — 2026-09-23

### Added

- `key --mode chroma` — YUV-domain chromakey (broadcast keyer): same
  --color/--similarity/--blend/--despill knobs, tolerates wrinkled and
  unevenly lit screens better than RGB colorkey.
- `eq --shelf low|high:FREQ:GAIN` — shelving EQ via lowshelf/highshelf
  (rumble shelf cut, air shelf; repeatable).
- `eq --notch FREQ[:WIDTH_HZ]` — two-pole Butterworth bandreject kills a
  resonance or ring (extends dehum beyond the fixed mains list).
- `eq --brickwall LO,HI` — afftfilt zero-phase FFT bandpass; keeps only
  the band (telephone FX, speech-band isolation).
- `blur --engine box` — boxblur, the classic fast box kernel (blockier
  texture than gblur; sigma maps to luma radius).
- `audiogram --mode hist` — ahistogram amplitude-distribution video:
  sample-level histogram scrolling over time (clip/headroom QC).

## [0.254.0] — 2026-09-23

### Added

- `qa --metric vif` — Video Multi-method Assessment Fusion score
  (perceptual quality metric; identical clips score ~1.0). Parsed off
  the last `VIF scale=3 average:` line.
- `bars --kind allrgb|allyuv` — full color-cube sweeps: every RGB/YUV
  combination in one card (encoder color-bleed QC).
- `gen --pattern hald` — `haldclutsrc` identity HALD LUT image
  (`--level` 3..12, default 8 = 512x512): grade it in any image editor,
  then feed it back via `grade --lut`.
- `channel --mode stereowiden` — dedicated M/S widener
  (pre-delay + feedback echo + crossfeed on the side signal);
  `--amount` scales crossfeed 0.05..0.8.
- `smooth --engine sab` — shape-adaptive blur: flattens flat regions
  without crossing contours (matte-style cleanup, skin).
- `leveler --engine compand` — single-band transfer-curve leveler:
  quiet program material lifted along one continuous knee.
- `deinterlace --engine field` — top-field extract: half-height
  progressive output, the fastest possible deinterlace (previews).
- `extract --alpha` — `alphaextract`: the alpha channel as a grayscale
  PNG (matte export/QC); rejects inputs with no alpha channel.

## [0.253.0] — 2026-09-23

### Added

- `scan --motion` — `vmafmotion` leg: `motion_avg`/`motion_max` extras
  (bitrate-budget QC — static content ≈0, busy action ≈5+; slower leg,
  so flag-gated).
- `scan --timecode` — `readvitc` leg: `vitc`/`vitc_tc`/`vitc_frames`
  extras (embedded VITC timecode on broadcast masters).
- `qa --metric msad` — third difference metric (mean of absolute
  differences; same `average:` output key as PSNR).
- `grid --focus` — hero layout via `xstack` per-tile rects: first input
  fills the left ~2/3 column, the rest stack down the right
  (podcast/interview); ignores `--layout`.
- `mix --gate` — `sidechaingate` duck variant: the bed hard-mutes under
  speech instead of smoothly dipping (talk-show bed); implies `--duck`.
- `eq --deemph riaa|cd|fm50|fm75` — `aemphasis` reproduction curve:
  undoes the pre-emphasis HF boost baked into vinyl rips, FM captures
  and early CDs.
- `transcode --interlaced` — `il` field interleave + `setfield tff`:
  marks the output interlaced for broadcast/air-master delivery
  (field_order=tb on .mov).

## [0.252.0] — 2026-09-23

### Added

- `scan --dupe REF` — MPEG-7 `signature` match: reports `dupe`,
  `dupe_segments`, `dupe_frames` — is this clip inside REF (re-upload /
  library-duplicate QC; needs a few seconds of footage).
- `scan` — `rg_gain_db`/`rg_peak` extras from a `replaygain` leg in the
  audio pass (the gain a player applies for reference loudness).
- `scan --text` — `ocr` leg (tesseract, 2fps pass): `text` (first hit),
  `text_frames`, `text_confidence` — burned-in text QC.
- `scope --mode cie` — `ciescope` CIE-1931 horseshoe vs Rec.709 triangle
  (out-of-gamut spills past — wide-gamut QC).
- `bars --kind sd|pal100|pal75|rgb|yuv` — other broadcast test-card
  generators (PAL 100/75, rgbtestsrc, yuvtestsrc).
- `delogo --find ref.png --track` — `find_rect`+`cover_rect` re-detects
  the mark EVERY frame and blurs wherever it lands (moving watermarks).

## [0.251.0] — 2026-09-23

### Added

- `repair` — swap a stretch of bad/glitched frames for one clean frame
  from a reference take (`freezeframes`): `--ref` the other take, `--at`/
  `--dur` mark the damage, `--ref-at` picks the clean frame (seconds
  mapped to frame indices per clip fps; verified frame-exact swap).
- `diff --mode mask --threshold` — `maskedthreshold` change-mask view:
  keeps only pixels whose |diff| beats the threshold (QC overlays, spot
  the one corrupt region in a re-render).
- `key --mode matte --mask` — `alphamerge`: an external grayscale matte's
  luma becomes the alpha channel; encodes prores_ks `yuva444p10le` so the
  channel survives to an editor (hand-drawn masks, roto handoffs).
- `fx --kind contrast` — `acontrast` dynamics tilt: `--strength` >0.5
  expands punch, <0.5 compresses toward level (measured: mean -21→-14 dB
  at 0.8).
- `scope --mode loud` — loudness-over-time corner tile via
  `ebur128`+`adrawgraph` (momentary LUFS curve; dips = quiet stretches,
  pinned top = ceiling drive).
- `frames --untile CxR` — `untile` splits every frame into a COLSxROWS
  tile sequence (contact-sheet/mosaic → per-tile stills).

## [0.250.0] — 2026-09-23

### Added

- `stabilize --engine vidstab` — two-pass vid.stab stabilization (the
  pro-grade engine NLEs wrap): pass 1 analyzes motion into a temp `.trf`,
  pass 2 renders. `--smoothing` frames (default 15) widens the history it
  averages; measurably steadier than single-pass deshake on real shake.
- `vdenoise --engine rg` — `removegrain` per-plane modes (VLC/AviSynth
  style); `--strength` maps to mode 2..11 (flat-noise luma stdev ~halved
  at default) — a fast middle ground between hqdn3d and nlmeans.
- `wb --engine greyedge` — grey-edge illuminant estimation removes a color
  cast while keeping more of an existing grade than histogram stretch
  (green-cast frame → neutral r≈g≈b).
- `scan` — new `vfrdet` leg: `vfr` (ratio > 0.05), `vfr_ratio`,
  `vfr_frames` extras — screen-recording / edit-joined variable-frame-rate
  sources flag in the QC pass.
- `upscale --engine epx` — EPX pixel scaler (2x/3x, softer diagonals than
  hqx — dithered retro captures).
- `channel --mode earwax` — headphone-oriented stereo widening (earwax
  crossfeed; aformat pins 44.1kHz stereo first — it refuses anything else).
- `scope --mode drift` — `signalstats`+`drawgraph` luma-drift curve in the
  corner tile: flat = locked exposure, slope = ramp/flicker source.

## [0.249.0] — 2026-09-23

### Added

- `channel --mode bands --freqs` — `acrossover` frequency-band stems:
  `--freqs 500,2000` writes `<stem>_band1..3.wav` (low/mid/high splits
  for remixes); measured 60dB rejection outside each band.
- `channel --mode sync --side --cm` — `compensationdelay` per-channel
  mic-distance delay: two mics on one source at different distances
  comb-filter when summed; 34cm ≈ 1ms at 20°C.
- `delogo --find logo.png` — `find_rect` auto-locates the watermark in
  the first 15s and removes the found box (no coordinates needed).
- `scan` — `readeia608` leg adds `has_cc`/`cc_lines` (EIA-608 closed
  captions for broadcast QC); `cropdetect` leg adds `crop_hint` +
  `letterboxed` (black-border detection with the inner crop box).
- `gen --pattern noise --color` — noise colour selectable:
  white/pink/brown/blue/violet/velvet (`anoisesrc`).
- `glitch --engine random` — `random` frame-order scramble inside a
  rolling frame cache; `--strength` scales the cache depth.

## [0.248.0] — 2026-09-23

### Added

- `fx --kind fshift` — `afreqshift` frequency shifter: metallic
  alien/robot voice; `--strength` sweeps the shift 50→2000Hz.
- `deinterlace --engine phase` — field-order swap for captures whose
  parity is wrong (jerky pans that no real deinterlacer fixes).
- `gen --pattern sweep` — speaker-test chirp: linear 20Hz→`--freq`
  sweep bed (`aevalsrc`), goes through the audio-only output path.
- `leveler --engine limit` — `alimiter` lookahead brickwall ceiling:
  `--threshold` is the ceiling dB, `--makeup` pushes into it.
- `channel --mode bal --pan -1..1` — `stereotools` balance correction
  for lopsided stereo (tape drift, mismatched mics).
- `scan` — bitplanenoise leg adds `noise_floor`/`noisy` extras:
  LSB-plane occupancy flags grainy sources that devour bitrate.

## [0.247.0] — 2026-09-23

### Added

- `upscale --engine hqx` — hq2x/hq3x/hq4x pixel-art scaler (`--factor`
  snaps to 2/3/4); the cleanest sprite/text upscale of the set.
- `deinterlace --engine pullup` — inverse-telecine (3:2 pulldown IVTC);
  on a progressive source it passes through harmlessly.
- `glitch --engine swaprect` — swaps the frame's quadrants (surreal
  mirror-shuffle).
- `scope --mode osc` — oscilloscope XY plot of the video signal.
- `scan` — signalstats pass adds `luma_min`/`luma_max` + `illegal_luma`
  broadcast-range QC (16-235 legal).

### Fixed

- `spawn::run` drained the child's pipes only AFTER waiting for exit —
  any ffmpeg writing >64KB (macOS pipe buffer) deadlocked. The new
  signalstats scan leg tripped it immediately. Both pipes now drain on
  reader threads while the child runs; this fixes every current and
  future heavy-output filter.

## [0.246.0] — 2026-09-23

### Added

- `fx --kind ringmod` now TRUE ring modulation — `amultiply` multiplies the
  signal by a lavfi `sine` carrier (`--strength` sweeps 25→500Hz); replaces
  the tremolo approximation. `--at` windows gate the modulated leg.
- `scope --mode qp` — per-macroblock QP overlay (encoding QC).
- `scope --mode pix` — pixscope magnified pixel grid at `--x`/`--y`.
- `grade --mix "rr,rg,rb,gr,gg,gb,br,bg,bb"` — colorchannelmixer 3x3
  channel matrix (channel swaps, custom split-tones).
- `deinterlace --engine separate` — separatefields: every field becomes a
  frame (25i→50p, 29.97i→59.94p), a smooth slow-mo/sports source.

## [0.245.0] — 2026-09-23

### Added
- `sonify` — play a picture as sound: spectrumsynth scans the image like a spectrogram (bright pixels = loud harmonics); `--dur`/`--speed`/`--sample-rate`
- `fx --kind crush` — acrusher bitcrusher: bit-depth + sample-rate destruction for digital lo-fi dirt
- `gen --pattern sierpinski` — fractal animated background (type=triangle, `--speed` scales jump, `--seed`)
- `glitch --engine pixels` — shufflepixels block-scatter corruption bursts (`--strength` scales block size)

## [0.244.0] — 2026-09-23

### Added
- `grade --color-from ref.mp4` — mergeplanes chroma borrow: keep your luma, take the reference's U/V colour grade (scale2ref-sized, SAR-normalized; no --at support)
- `vdenoise --engine fftdnoiz` — FFT-domain denoise for film grain (temporal prev/next context)
- `smooth --engine spp|fspp` — simple/fast postproc deblock for the blockiest sources
- `glitch --engine stutter` — shuffleframes frame-drop stutter (VHS skip/jitter)

## [0.243.0] — 2026-09-23

### Added
- `grade --lut` now auto-routes 1D `.cube` files (LUT_1D_SIZE) to `lut1d` — lut3d rejects them outright, so tonal 1D LUTs previously failed
- `deflicker --engine tmide` — tmidequalizer temporal histogram midpoint for stubborn timelapse/strobe flicker
- `gen --pattern noise|tone` — audio beds without input files: pink-noise roomtone/dither bed, `--freq` sine reference tone
- `glitch --engine swapuv` — U/V chroma swap (magenta↔green weird-color look)

## [0.242.0] — 2026-09-23

### Added
- `stereo` — stereo3d packed-format conversion: side-by-side/above-below/interleaved ↔ anaglyph (red-cyan, green-magenta) for 3D creators; `--in`/`--out` take stereo3d format names (anaglyphs are output-only)
- `scan --scenes` — same QC pass also reports `scene_cuts`: hard-cut timestamps from scdet (edit-point map)
- `fx --kind ringmod` — fast-AM tremolo robot voice (Dalek / sci-fi comm channel)
- `smooth --engine yaep` — yaepblur edge-preserving smoothing (bilateral-class)
- `deinterlace --engine w3fdif` — Martin Weston three-field deinterlacer (sharp diagonals on SD archives)

## [0.241.0] — 2026-09-23

### Added
- `vdenoise --engine dotcrawl` — dedot removes composite/analog dot-crawl and rainbow edges (VHS rips, capture cards)
- `scope --mode data --x/--y` — datascope hex pixel-value readout centered on a point (full-frame; verify clipped values, find a pixel's exact luma)
- `glitch --engine planes` — shuffleplanes RGB rotation: clean false-color acid look without noise grain
- `delogo --image mask.png` — removelogo from a drawn bitmap mask (white = inpaint): precise non-rectangular logos, watermarks with irregular shapes

## [0.240.0] — 2026-09-23

### Added
- `grade --match ref.mp4` — midequalizer histogram matching: pull your footage's colour distribution toward a reference clip (camera matching, "grade it like that film"); the reference is scaled to fit via scale2ref
- `blur --engine directional --angle` — dblur directed streaks (speed-line / fake-motion look)
- `smooth --engine pp7` — lighter/faster postproc deblock for when uspp is too slow
- `scope --mode mvs` — codecview motion-vector overlay (compression QC: coherent arrows on pans, jittery arrows on noisy blocks); runs on the full frame since MV side-data doesn't survive scaling

## [0.239.0] — 2026-09-23

### Added
- `edge --engine link` — hysteresis edge linking: blurred strong edges grow into the weak map so connected contours survive and specks drop
- `smooth --engine uspp` — MPEG post-processor deblock+dering for over-compressed rips (`--strength` scales postproc quality)
- `trail --mode diff` — tblend frame difference: only moving pixels survive, static background fades to black
- `audiogram --mode monitor` — agraphmonitor filtergraph-stats visualization
- `grade --wash COLOR` (+ `--wash-amount`) — colorize mood veil that keeps luma

## [0.238.0] — 2026-09-23

### Added
- `audiogram --mode spatial` — showspatial stereo-field spectrogram (the field drawn as a moving image over time)
- `audiogram --mode volume` — showvolume per-channel VU bars (broadcast meter-bridge look)
- `audiogram --mode bitscope` — abitscope bit-pattern scope (audio bit-depth visualiser)
- `scan` blur QC — `blur_frames`/`blur_mean`/`blur_min` from diff-mode frame entropy: out-of-focus stretches read well under `blur_threshold` (default 0.45, tunable with `--blur`)

## [0.237.0] — 2026-09-23

### Added
- `denoise --ref noise.wav` — anlms adaptive noise cancellation with a reference recording (room-tone mic, second recorder): learns ref→mix then subtracts the estimate; ~9dB broadband cut while the voice survives (the naive single-graph reading cancels the voice too — documented in gotchas)
- `interp --engine framerate` — scene-aware frame blending for high-fps delivery: ~10x faster than minterpolate's motion estimation at the cost of slight ghosting on fast motion
- `thumb --best` — let ffmpeg pick the most representative frame (`thumbnail` scores each 100-frame batch by average similarity): a clean typical still from shaky footage, no manual timestamp needed

## [0.236.0] — 2026-09-23

### Added
- `upscale --engine xbr|two-xsai` — integer-scale pixel-art upscalers (retro game captures, sprite sheets): xbr snaps `--factor` to 2/3/4, super2xsai is fixed 2x
- `grade --vibrance -1..1` — smarter saturation that boosts muted colors while protecting saturated skin tones (mandelbrot pastel chroma +84% at 0.8 vs flat response on saturated testsrc)
- `edge --engine sobel|kirsch|roberts|prewitt` — classic convolution edge kernels (bright edges on black, no thresholds — cruder, crunchier look than edgedetect's Canny pass)

## [0.235.0] — 2026-09-23

### Added
- `displace` — warp the picture by a second clip's luma displacement map (displace + scale2ref; `--edge` wrap/mirror/smear/blank, timeline `--at`/`--dur`): heat ripple, liquid glitch, water reflections
- `sharpen --engine halo` — unsharp clamped to a blurred base via maskedclamp: strongest sharpening available, zero overshoot halos
- `eqviz` — apply EQ bands (anequalizer params) and render the frequency-response curve as the video: `--bands "f=200 w=100 g=10 t=h"` low-shelf, " | "-separated per-channel; mix QC card

## [0.234.0] — 2026-09-23

### Added
- `grade --curve "x/y x/y …"` — freeform master tone curve (curves master): matte fade `0/0.08 1/1`, S-curve `0/0 0.25/0.18 0.75/0.82 1/1`
- `bw --cut 0-1` — hard luma threshold instead of grayscale (lutyuv: xerox / high-contrast graphic B&W)
- `scan` — new extras `audio_max_db`/`audio_mean_db` (volumedetect pass: clip check + cheap loudness read on any audio-bearing input)

## [0.233.0] — 2026-09-23

### Added
- `legalize` — clamp luma to broadcast-safe levels (limiter, planes=1 luma-only; `--min`/`--max`, timeline `--at`/`--dur`)
- `levels` — Photoshop-style levels (colorlevels): `--in-min/--in-max` input points, `--out-min/--out-max` output range — crush rescue, matte film fade
- `aberrate` — chromatic aberration fringe (rgbashift): `--amount` px splits red left / blue right (VHS, cheap-lens, glitch edge)

## [0.232.0] — 2026-09-23

### Added
- `matrix` — convert between color matrices (colormatrix): `--from bt601 --to bt709` fixes SD-601 footage gone green in a 709 timeline; `--from auto` reads the stream tag; timeline `--at`/`--dur`
- `bw --weights r,g,b` — film-photographer channel weights for B&W (1.5,0.3,0.1 darkens blue skies like a red filter); replaces the 601 luma mix
- `grade --split -1..1` — split-tone: teal shadows + orange highlights (blockbuster grade); negative flips to warm shadows / cool highlights

## [0.231.0] — 2026-09-23

### Added
- `interp` — motion-compensated frame interpolation (minterpolate): `--fps 60` upres for high-refresh delivery, `--slow 0.5` smooth slow-mo from normal-rate footage (`--mode mci|blend|dup`)
- `deinterlace --engine mcdeint` — motion-compensated deinterlacer (archive quality, slow)
- `grade --kelvin 1000-40000` — direct white-balance dial in Kelvin (tungsten 2700 / daylight 5500 / cool 9000); exclusive with `--warm`

## [0.230.0] — 2026-09-23

### Added
- `despill` — remove green/blue screen spill from a keyed edge without keying (`--type`, `--mix`, `--expand`; timeline `--at`/`--dur`)
- `deinterlace --engine detelecine` — deterministic inverse telecine for a known 3:2 cadence (pattern=23; frame-exact when the cadence is clean, vs fieldmatch's per-frame comb analysis)
- `audiogram --mode phase` — aphasemeter mono-compat scope (thin line = mono, wide cloud = decorrelated); output video carries the audio track

## [0.229.0] — 2026-09-23

### Added
- `stack --mode max|min` — maskedmax composites every input's brightest pixels (star trails, light painting); maskedmin = darkest composite. median keeps the object-removal default
- `dejudder` — remove pullup judder from fps-converted footage (`--cycle 4` for 3:2 telecine)
- `smooth --engine deflate|inflate` — morphological texture smoothing (pore/grain), zero blur halo; `--strength` maps to pass count
- `scan` — stereo mono-compat QC: `phase_corr` extra (Pearson L/R); ~-1 means the mix cancels on mono speakers (phone/podcast playback)

## [0.228.0] — 2026-09-23

### Added
- `tonemap` — HDR → SDR: zscale to linear light, tonemap curve (`--algo hable|reinhard|gamma|clip|linear`, `--peak` nits), back to bt709. PQ/HLG phone footage for SDR platforms
- `telecine` — pull 24p film up to interlaced NTSC fields (`--pattern 23` 3:2 pulldown, `--field`) — inverse of `deinterlace --engine fieldmatch`
- `premult` — straight ↔ premultiplied alpha in place (`--mode premultiply|unpremultiply`); alpha-safe prores4444 output for AE/Motion handoffs

## [0.227.0] — 2026-09-23

### Added
- `dedust` — remove dust specks / hot pixels by morphology (luma erosion/dilation); `--size` 1-4, `--dark` for dark specks, `--at`/`--dur` window
- `deinterlace --engine fieldmatch` — inverse telecine: fieldmatch+decimate reconstructs 23.976p film frames from 29.97i transfers
- `vdenoise --engine edge` — nlmeans masked to flat areas via edgedetect+negate+maskedmerge; denoise without melting detail
- `extend` — stretch edge pixels into border strips (fillborders: `--left/--right/--top/--bottom`, `--mode smear|mirror|...`) for chroma-key rims and leftover letterbox slivers

## [0.226.0] — 2026-09-23

### Added
- `grade --lut look.png` — HALD image LUTs via `haldclut` (PNG/JPG; Darktable/RawTherapee exports); `.cube` keeps `lut3d`. `lut_engine` extra reports which path ran
- `grade --skin -1..1` — selectivecolor on the reds channel only: warms faces without touching the rest of the grade
- `scan` — interlace QC: `interlaced` verdict + `frames_tff`/`frames_bff`/`frames_progressive`/`frames_undetermined` (idet, same pass)

## [0.225.0] - 2026-09-23

### Added
- `stack` — median-composite 3+ locked-off inputs (`xmedian`): removes objects present in fewer than half the inputs (tourists crossing a museum shot, per-file sensor noise). `--percentile`; audio taken from input 1.
- `channel --mode ms` — decode mid/side-recorded stereo back to L/R (`stereotools ms>lr`) for field recorders.
- `leveler --engine mcompand` — multiband compressor preset (rumble/body/air bands): lifts quiet speech, caps peaks.
- `scope --mode hist` — rolling temporal histogram (`thistogram`) for exposure/color drift QC.


## [0.224.0] - 2026-09-23

### Added
- `tmedian` verb — temporal median filter removes anything visible for less than half the window (people/cars crossing tripod shots, timelapse intruders, rain streaks). `--radius` sets frames of history each side, `--percentile` shifts the pick, `--at`/`--dur` windowing supported; output drops `2*radius` edge frames.
- `declip --engine clip|click` — new `adeclick` engine repairs impulsive damage (vinyl pops, mouth clicks, digital dropouts) alongside the existing `adeclip` peak interpolation.
- `smooth --engine smartblur|bilateral` — edge-aware bilateral alternative: keeps skin texture edges, drops luma noise.


## [0.223.0] — RSI round 196

- `deblock` — DCT block-edge removal for heavily compressed sources (phone screen recordings, re-uploads): `deblock=filter=strong` with `--strength` scaling all three detection thresholds (stock defaults are a near no-op; measured HF block energy −7%). `--at`/`--dur` windows.
- `chromashift` — shift both chroma planes by whole pixels (`--x`/`--y` -255..255, `--edge` wrap/smear): fixes the colored halo on tape captures and misregistered encodes. `--at` window.
- `key --mode luma` — luminance keying via `lumakey`: keys the luma band `[threshold−similarity, threshold+similarity]` with `--blend` feathering. White-sky or dark-backdrop removal with no green screen. (`lumakey` semantics verified: it keys a luma band around the pivot, not a directional cutoff.)


## [0.222.0] — RSI round 195

- `amplify` — motion magnification: per-pixel diffs below `--threshold` across `--radius` frames get multiplied by `--amount`. Subtle breathing, pulses, machine shake become visible (measured stdev 66 → 80 at amount=8/threshold=30; +19% on a slow drift fixture). `--at`/`--dur` windows.
- `deinterlace --engine estdif|kerndeint` — two more deinterlacers: estdif edge-slope tracing (diagonals stay clean) maps `--mode`/`--parity`; kerndeint adaptive kernel (sharp+twoway, the classic high-quality mplayer mode).
- `selective` — rewritten on native `colorhold` (replaces the split/colorkey/maskedmerge chain; same pop-color result, fewer moving parts) + new `--blend` 0-1 feather so the kept color dissolves into the gray field instead of hard-cutting.


## [0.221.0] — RSI round 194

- `wb` — auto white balance via `normalize`: per-channel histogram stretch neutralizes a cast (verified: green-cast RGB [123,190,129] → [123,127,129]). `--strength` blends, `--independence 0` keeps the grade (contrast only), `--smooth` eases frame-to-frame, `--at`/`--dur` windows.
- `vdenoise --engine median` — salt&pepper / hot-pixel removal (radius s/3; timeline-capable so `--at` works). Flat stdev 16.1 → 3.9.
- `vdenoise --engine chroma` — chroma-only denoise (chromanr, thres s×12): phone-sensor color speckle. U-plane stdev 14.3 → 2.6.
- `scan` — now also flags strobe frames for photosensitive-epilepsy QC: same pass adds `photosensitivity=bypass=1` + metadata; reports `flash_frames`/`flash_max_badness` (badness>2 threshold separates real strobes from smooth motion).

## [0.220.0] — RSI round 193

- `shear` — italic-style picture slant (`shear` filter): `--x`/`--y` shear factors -2..2, `--fill` edge color, `--interp nearest|bilinear`, `--at`/`--dur` windows. Dynamic-tilt look without full rotate.
- `vdenoise --engine dctdnoiz|owdenoise` — two more denoisers: DCT-domain (σ=s×5, edge-sharp) and overcomplete wavelet (s×4, smoothest). Verified: flat-region stdev 16.1 → 2.5 / 1.4. Like bm3d they have no timeline → `--at` rejected.
- `channel --mode base --pan -1..1` — stereotools stereo base: -1 folds an over-wide/phasey recording to mono, +1 widens. Measured L-R diff 2.5 → 5.2 (wide) / 0 (narrow).

## [0.219.0] — RSI round 192

- `gen` — generative animated backgrounds from lavfi sources, no input file: `--pattern mandelbrot` (endless fractal zoom, `--zoom` depth), `gradients` (drifting palette — `--colors` up to 8, `--seed`, `--speed`), `life` (cellular automaton, `--rule` 0-255). `--size`/`--fps`/`--dur`. For music-visualizer backdrops, VJ loops, text cards.
- `vdenoise --engine bm3d` — strongest spatial denoiser ffmpeg ships (patch-stack Wiener); flat-region luma stdev 16.1 → 2.7 on a noise=25 fixture. No timeline support → `--at` is rejected with a clear error.
- `eq --graphic "dB,dB,..."` — classic 18-slider graphic EQ (`superequalizer`, 65Hz..20kHz); measured -18dB cut at 1kHz on band 9.

## [0.218.0] — RSI round 191

- `perspective` — deskew a filmed screen/whiteboard: `--points` takes the 4 corners of the skewed quad in the source (TL,TR,BL,BR, px) and `perspective` stretches it onto the output rectangle (`sense=0`); `--interp linear|cubic`.
- `eq --curve "F,G;F,G"` — freehand EQ line through freq,gain points via `firequalizer=gain_entry` (points interpolate — draw a tilt, a smile, a notch shelf).
- `fx --kind autopan` — `apulsator` L/R autopan sweep (hz 0.4-3 by strength); measured R-channel swing ~10dB over a cycle.


## [0.217.0] — RSI round 190

- `upscale` — up-res old/phone footage: `zscale` spline36 kernel (reconstructs detail better than bilinear/lanczos) + light `unsharp` for edge acuity. `--factor` 1.05-4 (2 doubles width AND height), `--strength` 0-1.
- `fx --kind sub` — synthesized sub-bass octave under the mix (`asubboost`, wet scaled by strength) — drop/trap low-end weight.
- `fx --kind crossfeed` — headphone crossfeed: bleeds each ear slightly into the other so long listening feels speaker-like, not hard-panned (L−R difference −11dB measured).


## [0.216.0] — RSI round 189

- `declip` — clipped/blown-out audio repair via `adeclip`: interpolates flattened peaks back into waveforms (`--window` ms analysis slice, `--threshold` 1-100 clip fraction, `--overlap-save`, `--at`/`--dur` windowed).
- `reverb --ir` — convolution reverb: convolve against an impulse-response WAV from any IR pack (cathedral/plate/room — real spaces, not synthetic echo taps). `--tail` pads the dry side so the tail rings past the source end (default: the IR's own length).
- `channel --mode surround` — stereo→5.1 upmix (`surround` soundfield transform + derived LFE) for TV / cinema-side delivery.


## [0.215.0] — RSI round 188

- `replace --video` — the converse swap: keep this video's audio, show another file's frames (retake/音乐换画面). The audio is the master clock; a shorter picture needs `--loop`, a longer one is trimmed.
- `channel --mode haas` — Haas-effect stereo widening (micro L/R delays + polarity flip, mono-safe); `--amount` scales side gain 0.5..3.0.
- `denoise --engine` — explicit denoiser pick: `auto` (afwtdn if built, else afftdn — old default), `wavel` (errors when afwtdn isn't in this build), `fftdn` (works everywhere).

## [0.214.0] — RSI round 187

- `transcode --preset dnxhd` — DNxHR HQ + PCM in .mov, the Avid/Resolve-side edit handoff (ProRes covers the Apple side).
- `channel --mode mid|side` — mid/side extraction: `mid` keeps the correlated center (dry voice), `side` keeps the difference (room tone, ambience — a correlated signal cancels to silence).
- `eq --preset warm|air` — warm = +3dB lows with a slight presence dip (de-harshens); air = +6dB top shelf + small presence lift (dull-recording sparkle).

## [0.213.0] — RSI round 186

- `v360 --in` — input projection for non-equirect 360 sources: `fisheye`, `dfisheye` (GoPro Max / Insta360 / Ricoh Theta dual-fisheye), `c3x2`/`eac` cubemaps, `barrel`, `hequirect` (180 VR).
- `fx --kind bass` — one-knob low-end weight (`bass=g:110Hz`, strength 2..12dB).
- `fx --kind muffled` — next-room/underwater voice (strength sweeps the lowpass 2400→500Hz).
- `fx --kind crystal` — transient sharpening for dull recordings (crystalizer, verified +12dB >8kHz on noise).

## [0.212.0] — RSI round 185

- `v360` — reframe equirect/360 footage to a flat viewport
  (`--yaw`/`--pitch` look direction, `--fov` lens, `--size` canvas)
- `grade --preset crossprocess|strongcontrast|linearcontrast` — film curves
  presets (cross-processed, hard S-curve, gentle contrast stretch)
- `fx --kind excite` — harmonic exciter (aexciter): adds air/presence above
  ~7.5kHz, `--strength` scales the amount

## [0.211.0] — RSI round 184

- `smooth` — edge-preserving beauty/skin blur (smartblur in `ls>0` flat-region mode;
  `--strength` scales radius+threshold, `--at/--dur` windows it)
- `vdenoise --engine vaguedenoise` — wavelet denoiser (strong spatial cut);
  `hqdn3d` strength mapping raised so it actually bites
- `channel --mode ambience --amount` — stereotools side-level cut that dries
  out room echo under voice (mid survives, side drops)


## [0.210.0] — 2026-09-23

### Added

- `scan` (QC report: black/frozen stretches + black frames, JSON extras)
- `vdenoise --engine` (nlmeans | hqdn3d | atadenoise)
- `transcode --range` (limited/full colour-range tag)


## [0.209.0] — 2026-09-23

### Added

- `audiogram --mode spectro` (scrolling `showspectrum` audiogram look)
- `leveler --engine` (`compressor` default, `speechnorm` adaptive speech normalize)
- `fx --kind saturate` (`asoftclip` tape-style soft clip saturation)


## [0.208.0] — 2026-09-23

### Added
- `audiogram --mode cqt`, `sharpen --engine cas`, `equalize`.

## [0.207.0] — 2026-09-23

### Added
- `deesser`, `deband`, `dedup`.

## [0.206.0] — 2026-09-23

### Added
- `thump`, `riser`, `whoosh`.

## [0.199.0] — 2026-09-23

### Added

- `bars` — SMPTE test card (`smptebars`/`smptehdbars`, `--size`/`--dur`/`--hd`) with optional 1kHz `--tone` bed, for QC slates and tape leaders.
- `scope` — QC scope overlay (`--mode vector|wave`, corner `--position`, `--size` fraction), `--at`/`--dur` windows via overlay enable.
- `desqueeze` — anamorphic restore: `--factor` lens ratio stretches one axis (`--axis y|x`).

## [0.205.0] — 2026-09-23

### Added
- `iris`, `burst`, `grade --preset bleach`/`neon`.

## [0.204.0] — 2026-09-23

### Added
- `impact`, `wave`, `spin` verbs.

## [0.203.0] — 2026-09-23

### Added

- `pick` — dominant-color report: decodes one frame at `--at` (default midpoint), scales to a 3x2 grid and reports mean + per-zone hex colors as JSON extras.
- `diff` — visual diff between two clips: `scale2ref` conforms then `blend=all_mode=difference` + gamma boost; `--side` stacks the reference beside the diff; output stops at the shorter duration.
- `selective` — keep-one-color look: `colorkey` mask + `maskedmerge` paints a desaturated copy everywhere except matching pixels; `--color`/`--similarity`, `--at`/`--dur` windows.

## [0.202.0] — 2026-09-23

### Added

- `outline` — inks detected edges black over footage: `edgedetect=mode=wires`+`negate` multiply-blended; `--strength`, `--at`/`--dur` windows.
- `night` — night-vision look: desaturate + green mids + `noise` grain + `vignette`; `--grain`, windowed via blend branch.
- `snow` — falling snow: a tall `noise` field cropped by a `mod(t*speed,2h)` scrolling window, `colorkey`ed over the video; `--density`/`--speed`, `--at`/`--dur` windows.

## [0.201.0] — 2026-09-23

### Added

- `emboss` — relief look via `convolution` kernel; `--amount` blends back with the source, `--at`/`--dur` windows.
- `tilt` — tilt-shift miniature: `crop`+`gblur` top/bottom strips recomposited with `overlay`; `--band` sharp fraction, `--blur` sigma, `--at`/`--dur` windows.
- `sway` — handheld drift: `pad` margin then per-frame `crop` x/y on slow sine; `--rate`/`--px`, windowed via blend branch.
- `rack` — rack-focus breathing blur: `gblur` copy sine-mixed with the source; `--rate`/`--blur`.

## [0.200.0] — 2026-09-23

### Added

- `solarize` — psychedelic partial invert: `lutrgb` inverts pixels above `--threshold` luma, `--at`/`--dur` windows.
- `pulse` — breathing zoom bounce: `zoompan d=1` sine zoom (`--rate` cycles/sec, `--depth` amplitude), `--at`/`--dur` windows via blend branch.
- `deflicker` — timelapse flicker removal: `deflicker=size=N:mode=am` temporal luma smoothing.

## [0.198.0] — 2026-09-23

### Added

- `cartoon` — comic look: posterized base (`elbg --levels`) with dark ink outlines (`edgedetect=mode=wires` + `blend=multiply`), `--at`/`--dur` windows.
- `heat` — false-color thermal luma map (`pseudocolor --preset`, `--opacity`), `--at`/`--dur` windows.
- `kaleido` — 2x2 mirrored mandala from the top-left quadrant (crop + hflip/vflip + hstack/vstack), `--at`/`--dur` windows.

## [0.197.0] — 2026-09-23

### Added

- `strobe` — music-video flash cuts: periodic opaque color flashes (`--rate` flashes/sec, `--duty` on-fraction, `--color`), gated by `--at`/`--dur` windows.
- `edge` — neon edge-detect outlines (`--mode wires|colormix`, `--low`/`--high`), `--at`/`--dur` windows.
- `lens` — lens distortion (`--k1`/`--k2`): negative values produce a fisheye look, positive values defish action-cam footage; `--at`/`--dur` windows.

## [0.194.0] — 2026-09-23

### Added

- `mirror` — half-frame mirror across the center axis (`--axis x`/`y`, `--at`/`--dur` windowed via blend T-expr)
- `pix` — full-frame retro pixelation (`--strength` 2-64 block divisor, `--at`/`--dur` window)
- `grade --preset sepia` — classic sepia `colorchannelmixer` matrix

## [0.195.0] — 2026-09-23

### Added

- `flip` — horizontal/vertical flip (`--axis x` unmirror selfie footage, `--at`/`--dur` windowed)
- `poster` — pop-art posterization (`elbg=l=N`, `--levels` 2-64, `--at`/`--dur` via blend branch)
- `duotone` — two-color luminance ramp (`--shadow`/`--highlight`, `format=gray` + per-channel `lutrgb`)

## [0.193.0] — 2026-09-23

### Added

- `censor --mode solid` — black-bar redact (drawbox=t=fill; combines with --shape circle)
- `caption --margin N` — pixel offset from the caption edge
- `grade --preset teal` / `noir` — orange-and-teal look; true B&W (tail desaturation)

## [0.196.0] — 2026-09-23

### Added

- `glow` — dreamy bloom (gblur + screen blend, `--at`/`--dur` window)
- `vhs` — retro tape look (`--strength` 0-3: noise + rgbashift + scanlines, `--at`/`--dur` via blend branch)
- `motionblur` — shutter smear (`tblend`/`tmix` temporal average, `--frames` 2-8, `--at`/`--dur` window)

## [0.192.0] — 2026-09-23

### Added

- `waveform --vertical` — transpose the rendered wave (top→bottom, h×w PNG)
- `broll --opacity` — ghost inserts (colorchannelmixer=aa on the overlay branch)
- `title --margin N` — pixel corner insets (platform safe-zone margins)

## [0.191.0] — 2026-09-23

### Added

- `caption --karaoke --highlight` — sung-word color over the dim full cue (two-layer PNG composite)
- `audiogram --mode scope` — lissajous vectorscope video (avectorscope)
- `delogo --shape circle` — elliptical removal mask via the removelogo path

## [0.190.0] — 2026-09-22

### Added

- `censor --shape circle` — elliptical mask inside each `--region` (circular face censor; geq alpha clip)
- `progress --opacity` — ghost progress bar (`colorchannelmixer=aa=N` on the bar source)

## [0.189.0] — 2026-09-22

### Added

- `trail` — motion trails: `--mode echo` trailing ghost smear (`tmix` + delayed overlay composite, `--frames` 2-16, `--at`/`--dur` window), `--mode light` bright-pixel persistence via `lagfun` (`--decay` 0.5-0.99)
- `glitch --strength` — datamosh-style look (RGB channel shift + temporal noise)
- `fade --curve` — audio fade curve shape (tri/qsin/esin/hsin/log/qua/cub/exp)


## [0.188.0] — 2026-09-22

### Added

- `timer --opacity`, `countdown --opacity`, `scroll --opacity` — semi-transparent HUD clock, leader countdown, and rolling credits via a shared `raster::alpha_scale` pass; every raster text overlay now takes `--opacity` (title, meme, caption, timer, countdown, scroll)

## [0.187.0] — 2026-09-22

### Added
- `meme --opacity` / `caption --opacity` — semi-transparent ghost/watermark text (1..=100).

## [0.186.0] — 2026-09-22

### Added
- `subs --encoding LABEL` — decode .srt/.vtt in legacy charsets (gbk/big5/sjis/latin1) instead of UTF-8.
- `deliver --subs FILE` — burn captions during the platform packaging pass.


## [0.185.0] — 2026-09-22

### Added
- `pitch --formant` — timbre-preserving pitch shift via librubberband (natural voice, not chipmunk).
- `transcode --abitrate RATE` — audio bitrate on every encode path (voice posts → 64k frees video bitrate).


## [0.184.0] — 2026-09-22

### Added
- `scroll --speed N` — px/s scroll pacing (each window's duration = travel/N; conflicts `--dur`).
- `countdown --format` — `s` (default), `mm:ss`, `h:mm:ss`; `--from` cap raised to 600.


## [0.183.0] — 2026-09-22

### Added
- `loudnorm --gate N` — `--measure` fails when input loudness tops N LUFS (delivery QC gate).
- `transcode --vbitrate RATE` — peak bitrate cap (`-maxrate R -bufsize 2R`) on h264/hevc/webm/av1.



## [0.182.0] — 2026-09-22

### Added
- `delogo --regions x:y:w:h,...` — remove several logos/watermarks in one pass; `--soft` masks every box. Single `--x/--y/--w/--h` still works.


## [0.181.0] — 2026-09-22

### Added
- `censor --region` comma list — censor several regions in one pass (per-region crop+effect+overlay arms).
- `subs --burn-si N` — burn the input's own subtitle stream N (multi-track inputs).


## [0.180.0] — 2026-09-22

### Added
- `progress --reverse` — bar starts full and depletes to zero ("time left" overlays).
- `meme --fade` — fade each text card in/out at the `--at/--dur` window edges.



## [0.179.0] — 2026-09-22

### Added
- `hls --encrypt` — AES-128 segment encryption writing `key.bin` + `key.info` (`#EXT-X-KEY` in playlists); `--key HEX` custom key, `--key-uri URI` playlist key URL.



## [0.178.1] — 2026-09-22

### Fixed
- `scroll_comma_at_and_end_replays_windows` test: compile fix (`{w:?}`) + `--text` flag (the test was authored on rsi-round150 but never committed to its branch; landed with the merge).

## [0.178.0] — 2026-09-22

### Added
- `scroll --at` comma list + `end` — replay the credits/ticker at several marks (`extra.windows` lists each window).


## [0.176.0] — 2026-09-22

### Added
- `timer`/`countdown`/`meter --at` accept `end` — last numeric `--at` args moved onto `resolve_frame_at` anchors.


## [0.175.0] — 2026-09-22

### Added
- `hls --poster-at` — pick the poster.jpg frame time (sec or `end`, clamped inside the stream); errors without `--poster`.
- `deliver --crf` — H.264 quality level for platform delivery (default 20).

## [0.174.0] — 2026-09-22

### Added
- `extract --gif --at` comma lists — one GIF per timepoint (`<stem>_N.gif`, `extra.files`); 2-pass palette per clip, `--dur`/`--width`/`--bounce`/`--loop` honored.

## [0.173.0] — 2026-09-22

### Added
- `audiogram --at a,b,...` + `--dur` — one N-second waveform clip per start point (`<stem>_N.mp4`, `extra.files`); single `--at`/`--from` + `--dur` bounds one clip. Rendering refactored to `render_clip(from, to, out)` per window.



## [0.172.0] — 2026-09-22

### Added
- `extract`/`cover --at` comma lists — one still/cover per timepoint (`<stem>_N.<ext>`); cover keeps the same canvas/`--blur`/`--size` per output, `extra.files` lists them.

## [0.171.0] — 2026-09-22

### Added
- `thumb --at` comma lists — one still per timepoint (`<stem>_N.<ext>`); input-seeks each point, `--width` rescales all, `extra.files` lists them.

## [0.170.0] — 2026-09-22

### Added
- `grid --time` — stamps the same mm:ss readout on every tile's bottom-right (one shared digit sprite; no libass/drawtext needed) for multi-cam/review grids.

## [0.169.0] — 2026-09-22

### Added
- `concat --audio-fade N` — boundary `afade` at every joint: each non-last clip's tail fades out, each non-first clip's head fades in. Duration and lip sync preserved (no overlap drift); forces the re-encode path since stream-copy can't fade.

## [0.168.0] — 2026-09-22

### Added
- `waveform`/`spectrogram --at` comma lists — one PNG per window (`<stem>_N.png`); spectrogram slices the audio per window, waveform crops each rendered strip. The written paths land in `extra.outputs`.

## [0.167.0] — 2026-09-22

### Added
- `rotate --at/--dur` — `--angle` tilt (and `--flip`) only inside a window via `enable='{enable_expr}'`: comma `--at` lists several tilts. `--deg` turns error out with `--at` — a 90° rotate changes the canvas mid-clip.

## [0.166.0] — 2026-09-22

### Added
- `concat --gap N` — inserts N seconds of black + silence between every pair of clips (beat gap between montage sections): generated `color=black` + `anullsrc` pads sit inside the filter-concat chain, so all outputs share one canvas. Exclusive with `--transition`.

## [0.165.0] — 2026-09-22

### Added
- `replace --at a,b,...` — comma list swaps the audio track inside several windows (needs `--dur`); the replacement track is laid across the windows **in order** — window i plays the slice that continues where window i−1 left off. `--fade` eases each window edge. Overlapping windows error out.

## [0.164.0] — 2026-09-22

### Added
- `key --at/--dur` — green-screen composite only inside a window: `overlay` gains `enable='{enable_expr}'`, so comma `--at` lists work too; outside the window the background shows through.
- `subs --shift --from/--to` — bounds the retiming to cues overlapping the window (`end` ok) for when only part of the track is late.

## [0.163.0] — 2026-09-22

### Added
- `freeze --at a,b,...` — comma list holds a cloned frame (`--dur`) at several points in one pass; `--ease`/`--reverse`/`--zoom` still need a single `--at`.
- `fade --dip a,b,...` — comma list dips to `--color` at every point (a dip per scene mark); `--dip` now takes a string list.

## [0.162.0] — 2026-09-22

### Added
- `deliver --fps N` — output frame-rate override (60 for gameplay/sport uploads; default stays 30).
- `subs --burn --margin N` — exact `MarginV` in pixels; overrides `--safe`'s computed safe-zone margin.
- `hls --poster` — also writes `poster.jpg` (a mid-video frame) next to the playlist, ready to use as the web player's poster frame.

## [0.161.0] — 2026-09-22

### Added
- `music --at a,b,...` — multi-entrance bed: each comma `--at` point gets its own delayed wet branch (`end` ok), amixed into one bed before ducking — intro sting + outro sting in one command.
- `silence --at a,b,...` — comma list pads `--dur` of quiet at several points (pause beats between scenes); `--at` now takes a string list (still accepts a bare number).

## [0.160.0] — 2026-09-22

### Added
- `insert --at a,b,...` — comma list splices the clip at several points in one pass (a sponsor sting at every chapter mark). Plain splice only; `--transition` still needs a single `--at`.
- `broll --at a,b,...` — comma list re-flashes the same cutaway at several points: per-window `split` overlay branches restart the insert each time, and `--fade`/`--audio` apply to every window.

## [0.159.0] — 2026-09-22

### Added
- `concat --transition` accepts a comma list (`fade,wipeleft,...`) — picks a different xfade per joint; a single value still applies to every joint.
- `channel --mode split` — splits a stereo track into `<stem>_L.wav` and `<stem>_R.wav` mono files (host/guest mic separation for per-voice cleanup).
- `remux --aspect 16:9` — rewrites the display aspect ratio while stream-copying (fix anamorphic/wrong-AR files without re-encoding).

## [0.158.0] — 2026-09-22

### Added
- Comma-list `--at` on `boomerang` — boomerang several windows in one pass (each gets its own split/reverse/`--times` loop chain inside an alternating trim+concat graph).
- Comma-list `--at` on `mix` — the bed enters at several spots (OR'd `between` gate + per-window `afade` when `--fade` is set). Requires `--dur`.

## [0.157.0] — 2026-09-22

### Added
- Comma-list `--at` on `speed`, `tempo`, `zoom`, `vdenoise` — several timed windows per pass (requires `--dur`).

### Changed
- New `time::window_list` resolves comma `--at` lists and merges overlapping/touching windows; `speed`/`tempo`/`zoom` now emit alternating normal/FX trim+concat segments per merged window (the single-window graph is unchanged).

## [0.156.0] — 2026-09-22

### Added
- Comma-list `--at` on every windowed audio-FX verb — `reverb`, `eq`, `dehum`, `denoise`, `leveler`, `gate`, `vocal`, `pitch`, `voice`, `fx`: the effect hits at several marks in one pass (requires `--dur`).

### Changed
- `engine::audio_window_for` resolves `--at` (single or comma list) and builds one `atrim`+`adelay` wet branch per window with AND'd `1-between` dry gates; replaces per-verb `resolve_at`+`audio_window` plumbing.

## [0.155.0] — 2026-09-22

### Added
- Comma-list `--at` (`t1,t2,...`) on every windowed look verb — `invert`, `blur`, `bw`, `sharpen`, `vignette`, `delogo`, `grade`, `progress`, `overlay`: the effect appears at several marks in one pass (requires `--dur`).

### Changed
- Window resolution now goes through shared `time::enable_windows`/`enable_expr` helpers instead of each verb hand-rolling `between(t,..)`/`gte(t,..)`.

## [0.154.0] — 2026-09-22

### Added
- `title --at` comma list — flash the same card at several marks.
- `meme --at` comma list — show the caption at several windows (requires `--dur`).


## [0.153.0] — 2026-09-22

### Added
- `mute --at` comma list — silence several windows (requires `--dur`).
- `volume --at` comma list — gain several windows (requires `--dur`).

## [0.152.0] — 2026-09-22

### Added
- `bleep --at` comma list — censor several windows in one pass.
- `censor --at` comma list — cover several windows (requires `--dur`).

## [0.151.0] — 2026-09-22

### Added
- `audiogram --from end` / `end-N` — tail-anchored segment clip.
- `caption --from end` / `end-N`, `--to end` — tail-anchored cue window.

## [0.150.0] — 2026-09-22

### Added
- `thumb --from end` / `end-N` — tail-anchored `--count` spread window.
- `multicam --at end` — switch back to the A angle at the tail.
- `subs --from end` / `end-N` — tail-anchored cue window for `--burn`.

## [0.149.0] — 2026-09-22

### Added
- `split --fade N` — fade video+audio around every cut boundary (each part reads as its own clip).
- `spectrogram --separate` — one band per channel (spot a hum living in only one side).

## [0.148.0] — 2026-09-22

### Added
- `cut --fade N` — fade in/out at the trimmed edges (re-encode).
- `deliver --platform square` — 1080x1080 feed-grid pack.

## [0.147.0] — 2026-09-22

### Added
- `subs --burn --to end`, `audiogram --to end` — tail-bound windows without probing.
- `bleep --at end --dur` — beep the last N seconds.

## [0.146.0] — 2026-09-22

### Added
- `channel --mode pan --pan -1..1` — stereo pan.
- `grade --exposure` — EV stops (-3..3) via the `exposure` filter.

## [0.145.0] — 2026-09-22

### Added
- `sprite --from/--to` — bound the thumbnail window (`--to end` ok); VTT cues stay on absolute media times.
- `loop --from/--to` accept `end`.
- `frames --count N` — N evenly-spaced stills across the clip.

## [0.144.0] — 2026-09-22

### Added
- `cut --ranges`/`--drop` accept `end` bounds — `T-end` through the tail, `end-N` the last N seconds.
- `thumb --count --from/--to` — bound the even-spread still window (`--to end` ok).

## [0.143.0] — 2026-09-22

### Added
- `--at end --dur N` on `boomerang`, `mute`, `dehum`, `voice`, `vdenoise`, `zoom` — completing `time::resolve_at` across every windowed verb.
- `--at end` on `thumb`, `cover`, `frames` — grabs the last frame (new `time::resolve_frame_at`, no `--dur` needed).

## [0.142.0] — 2026-09-22

### Added
- `chapter --yt` — write marks in YouTube description format ("0:00 Intro") for paste-in seek chapters; `--import` accepts the same `H:MM:SS Title` lines.
- `waveform`/`spectrogram --at end --dur N` — tail-anchored slices via `time::resolve_at`.

## [0.141.0] — 2026-09-22

### Added
- `sprite` — seek-preview thumbnails for video players: `<stem>-N.jpg` tile sheets (`--every` secs, `--width` px, `--cols`x`--rows` per sheet) plus a WebVTT cue file with `#xywh` coordinates (`--vtt`).

## [0.140.0] — 2026-09-22

### Added
- `--at end --dur N` on the remaining windowed verbs — `speed`, `tempo`, `mix`, `music`, `eq`, `reverb`, `fx`, `denoise`, `leveler`, `gate`, `vocal`, `pitch` — every `--at/--dur` verb now tail-anchors via `time::resolve_at`.

## [0.139.0] — 2026-09-22

### Added
- `--at end --dur N` on every windowed look/gain verb — `title` (end-cards), `blur`, `bw`, `invert`, `sharpen`, `vignette`, `grade`, `delogo`, `volume`, `progress` — via shared `time::resolve_at`.

## [0.138.0] — 2026-09-22

### Added
- `zoom --center X,Y` — punch target point in % of frame (default 50,50).
- `censor --at end --dur`, `meme --at end --dur`, `overlay --at end --dur` — tail-anchored windows via shared `time::resolve_at`.

## [0.137.0] — 2026-09-22

### Added
- `deliver --platform youtube` — 16:9 pack (1920x1080, −14 LUFS).
- `slideshow --bg` — letterbox color behind stills (name or hex).
- `extract --at end` — last-frame still / last-N-seconds GIF.

## [0.136.0] — 2026-09-22

### Added
- `vdenoise --at/--dur` — window the denoise pass (nlmeans timeline enable).
- `cover --size` — poster canvas WxH (default 1080x1920).
- `grid --bg` — gutter / letterbox color behind tiles (name or hex).

## [0.135.0] — 2026-09-22

### Added

- `slideshow --dur N` — total montage runtime spread across the stills (`per = (dur + (n-1)*fade)/n`); overrides `--per`
- `voice --at` / `--dur` — window the voice-polish chain through the shared `engine::audio_window` dry/wet graph
- `freeze --zoom Z` — slow push-in on the held frame via `zoompan` (end scale 1.0–2; mid-clip `--at` freezes)

## [0.134.0] — 2026-09-22

### Added

- `transcode --preset wav` / `flac` / `opus` — audio-only lossless + compact delivery: `pcm_s16le` WAV for DAW/edit handoff, FLAC archival, libopus 128k smallest voice/music; same `-vn` path and `--copy-audio` passthrough as mp3/aac
- `audiogram --from` / `--to` — render just a segment of the episode (`atrim` + `asplit` shares the window between the waveform and the mapped audio); podcast→clips no longer needs a separate `cut`

## [0.133.0] — 2026-09-22

### Added

- `transcode --preset mp3` / `--preset aac` — audio-only delivery (`-vn` + libmp3lame/aac at 192k); `--copy-audio` stream-copies when the input already holds the codec

## [0.132.0] — 2026-09-22

### Added

- `chapter --shift` — slide all marks by SEC (re-time chapters after adding an intro)
- `broll --at end` — cutaway over the tail without probing duration

## [0.131.0] — 2026-09-22

### Added

- `transcode --alpha` — keep the alpha channel: yuva420p on webm, ProRes 4444 on .mov (overlays/lower thirds for editors)
- `slideshow --volume` — linear gain on the music bed (0..=4)

## [0.130.0] — 2026-09-22

### Added

- `progress --edge left|right` — vertical progress bars (bottom-up fill) for vertical video
- `sheet --title` — big header label above the contact sheet grid

## [0.129.0] — 2026-09-22

### Added

- `bw --strength` — partial desaturation, keeps muted color
- `insert --at end` — append a clip just before the tail

## [0.128.0] — 2026-09-22

### Added

- `countdown --bg` — card plate behind numerals
- `audiogram --fps` — render frame rate (`rate=`; spectrum mode needs ffmpeg ≥7)

## [0.127.0] — 2026-09-22

### Added

- `subs --burn --from/--to` — burn only cues overlapping the time window
- `caption --from/--to` — same cue-window filter on raster captions

## [0.126.0] — 2026-09-22

### Added

- `waveform --bg` — opaque background card under the wave PNG (thumbnails)
- `broll --border [--border-color]` — pad ring around the PiP insert

## [0.125.0] — 2026-09-22

### Added

- `remux --video` — repack keeping only the video stream (mute, no re-encode)
- `chapter --remove` — strip all embedded chapter marks on remux (`-map_chapters -1`)
- `scroll --bg` — opaque bar behind `--mode ticker` crawl text

## [0.124.0] — 2026-09-22

### Added

- `rough --by-scene` — split speech keeps at scene changes (`src/scene.rs`: `select=gt(scene,0.4)`+`metadata=print` detection)
- `meter --at/--dur` — meter one slice for QC

## [0.123.0] — 2026-09-22

### Added

- `audiogram --fscale lin|log|rlog` — frequency axis on `--mode spectrum` bars
- `solid --fps N` — card frame rate (filmic 24 fps grain)
- `waveform --full` — `draw=full` dense waveform rendering
- `scroll --wrap N` — word-wrap credit lines at N columns

## [0.122.0] — 2026-09-22

### Added

- `meter` — live EBU R128 loudness meter video (`ebur128 video=1`; `--size`, `--meter 9|18`)
- `caption --wrap N` — word-wrap burned cue lines at N columns
- `title --opacity PCT` — ghost/watermark titles (1–100)
- `solid --align` — per-line `--text` alignment on cards

## [0.121.0] — 2026-09-22

### Added

- `solid --noise N` — animated film grain on solid/gradient cards
- `countdown --tone HZ` — beep frequency (default 880 Hz)
- `audiogram --split` — per-channel waveform rows (stereo split view)
- `scroll --align left|center|right` — per-line alignment in the credit block

## [0.120.0] — 2026-09-22

### Added

- `waveform --split` — per-channel rows (stereo L/R split view)
- `spectrogram --no-legend` — drop the axis strip for clean thumbnails
- `dehum --freq HZ` — custom hum fundamental 20–500 Hz, overrides `--mains`
- `meme --align left|center|right` — per-line alignment in wrapped meme cards

## [0.119.0] — 2026-09-22

### Added

- `caption --align left|center|right` — per-line alignment in burned caption cards
- `spectrogram --scale lin|sqrt|cbrt|log|4thrt|5thrt` — display scale knob
- `audiogram --scale lin|log|sqrt|cbrt` — wave amplitude scale (waveform modes)
- `subs --burn --shadow N` — libass drop-shadow depth on burned subs

## [0.118.0] — 2026-09-22

### Added

- `title --align left|center|right` — per-line text alignment in the rendered card (plain titles; conflicts `--outline`/`--shadow`)
- `timer --start N` — seed the countdown/counter readout at N seconds
- `waveform --peak` — peak-sample waveform rendering (`showwavespic filter=peak`)
- `solid --wrap N` — word-wrap long `--text` on solid/gradient cards

## [0.117.0] — 2026-09-22

### Added

- `meme --wrap N` — word-wrap long meme captions
- `channel --mode widen` — `extrastereo` stereo widening
- `transcode --preset av1` — AV1 delivery (libsvtav1 ≥7 / libaom 4.x)
- `scroll --mode ticker` — bottom news ticker crawl

## [0.116.0] — 2026-09-22

### Added

- `thumb --scenes` — still at frame 0 + every scene change (thumbnail mining)
- `overlay --border PX` / `--border-color` — ring around the overlay picture
- `audiogram --mode spectrum` — `showfreqs` frequency bars
- `subs --case upper|lower|title` — cue-text case rewrite (`--burn`/`--convert`)

## [0.115.0] — 2026-09-22

### Added

- `rotate --angle N` — free-angle dutch tilt (`rotate=a=`, canvas preserved)
- `subs --burn --align left|center|right` — ASS Alignment row for burned captions
- `chapter --list` — dump embedded chapter marks (time + title) as JSON
- `conform --anchor` — letterbox anchor on the `--pad` path

## [0.114.0] — 2026-09-22

### Added

- `multicam --transition N` — xfade/acrossfade at every camera switch instead of hard cuts
- `title --wrap N` — greedy word-wrap at N chars per line before rasterization
- `fade --dip T --dur N` — dip-to-color at T for scene-change transitions (video + audio)
- `conform --blur` — blurred-video letterbox fill instead of `--pad` color

## [0.113.0] — 2026-09-22

### Added

- `split --chapters` — cut at the input's embedded chapter marks
- `replace --at T --dur N` — windowed audio swap; original track plays outside the window, `--fade` on the joints
- `overlay --loop` — repeat a short `--video` overlay to cover the base (`shortest=1` bounds the composite)
- `subs --burn --box` — opaque plate behind each burned line (`BorderStyle=3`)

## [0.112.0] — 2026-09-22

### Added

- `progress --bg COLOR` — static full-width track bar behind the sliding fill
- `mix --fade N` — `afade` in/out on the B bed inside the `--at/--dur` window
- `insert --volume N` — scale the spliced clip's audio 0–4 in both concat and `--transition` paths
- `solid --fade N` — `fade` in/out on the generated card (works with `--text` and `--gradient`)

## [0.111.0] — 2026-09-22

### Added

- `multicam --keep-audio` — video switches angles at `--at` cuts while audio stays on camera A end-to-end (interview workflow)
- `conform --pad COLOR` — `pad=W:H:(ow-iw)/2:(oh-ih)/2:c` after the decrease-scale so `--size` output is exactly WxH with a chosen letterbox color
- `channel --mode mix51` — ITU 5.1→stereo fold-down (`FL<FL+0.707*FC+0.707*BL+0.5*LFE`)
- `transcode --colors N` — `palettegen max_colors=` knob on `transcode --preset gif` (same as `extract --colors`)

## [0.110.0] — 2026-09-22

### Added

- `meta --copy SRC` — carry every metadata tag + chapter mark from a sibling export (`-map_metadata 1 -map_chapters 1`)
- `extract --colors N` — GIF palette size via `palettegen max_colors=` (2–256; small palettes shrink files)
- `broll --loop` — replay a video insert shorter than the cutaway window (`loop=loop=-1:size=F` + restamp) instead of freezing its last frame
- `align --window SECS` — cap the PCM decode behind the cross-correlation search so long multicam takes align fast

## [0.109.0] — 2026-09-22

### Added

- `remux --audio` — audio-only stream copy (`-map 0:a`); re-encodes to mp3/ogg/wav/aac when the container can't hold the source codec
- `hls --fmp4` — CMAF fragmented-MP4 segments (`seg_*.m4s` + `init.mp4`) for flat and `--ladder` HLS packages; Safari/AirPlay-ready
- `loop --fade SECS` — seamless loop: copies joined by `xfade`/`acrossfade` joints so the loop point is invisible (re-encode path)
- `compress --res HEIGHT` — downscale before encoding (folded into both `--size` two-pass and `--crf` one-pass chains) to free bitrate at small size budgets

## [0.108.0] — 2026-09-22

### Added
- `subs --all` — extract every subtitle stream in one ffmpeg call (`stem_0.srt`, `stem_1.srt`, …); `probe` gains `subtitle_streams`.
- `mix --normalize` — amix `normalize=1` halves the summed level for two hot tracks.
- `fit --strength` — gblur sigma knob for `--fit blur` (default 30).
- `compress --crf N` — quality mode: single-pass libx264 crf (0–51) instead of the two-pass size budget; `--size`/`--target`/`--crf` is now an any-of requirement.

## [0.107.0] — 2026-09-22

### Added
- `silence --detect [--threshold --min]` — report-only silence ranges in JSON extras (no `-o`), for planning cuts before `split --silence`/`jumpcut`.
- `thumb --count N` — write N evenly-spaced stills (`stem_01..NN.ext`); `fps=(n-0.5)/dur` keeps every pick before EOF.
- `sheet --from/--to` — sample the contact sheet only within a window; `--time` stamps re-anchor to the window.
- `loudnorm --dynamic` — per-frame dynamic normalization (`linear=false` on the two-pass apply) for speech that a linear offset pumps on.

## [0.106.0] — 2026-09-22

### Added
- `vocal --amount 0..1` — partial center-channel cancel keeps backing bleed; partial isolate blends toward the center mix.
- `eq --tilt -10..10` — one-knob warm↔bright tone tilt mapped onto the bass/treble shelves (explicit `--bass`/`--treble` still win).
- `deinterlace --engine yadif|bwdif` — choose the smoother motion-compensated bwdif. (nnedi intentionally omitted: it aborts without an external weights file on every ffmpeg tested.)
- `insert --dur N` — splice only the first N seconds of the clip, in both the hard-cut and `--transition` xfade paths.

## [0.105.0] - 2026-09-23

### Added
- `mix --duck` — sidechain compression: the B bed ducks whenever A (voice) is loud (asplit + aformat so it works on ffmpeg <7).
- `compress --target discord|whatsapp|gmail` — platform size presets (8/16/25 MB) in place of `--size`.
- `subs --burn --outline N` — burned-caption stroke width via ASS `Outline`.


## [0.105.0] — 2026-09-22

## [0.104.0] - 2026-09-23

### Added
- `insert --transition T --duration D` — the splice crossfades into and out of the clip (two xfade joints + acrossfade), instead of a hard cut.
- `hls --audio-only` — `-vn` audio-only HLS packaging for podcasts and voice-over streams.
- `grid --fill` — tiles scale up and crop to fill each cell instead of letterboxing.


## [0.104.0] — 2026-09-22

## [0.103.0] - 2026-09-23

### Added
- `loudnorm --measure` — report-only loudness: measured I/TP/LRA in extras, no `-o` needed.
- `subs --convert` — .srt ↔ .vtt conversion (parses vtt cue settings/WEBVTT blocks, emits clean files).
- `art --extract` — pull the embedded cover stream out to an image (`--image` no longer required).
- `countdown --position` — place the counting digits anywhere on the 9-position map.


## [0.103.0] — 2026-09-22

## [0.102.0] - 2026-09-23

### Added
- `hls --ladder 1080,720,480` — ABR ladder: variant playlists + `master.m3u8`, tiered bitrates per height, one aac encode per variant (ffmpeg <7 rejects shared streams across groups).
- `concat --level LUFS` — one-pass loudnorm on every input before joining (mixed-source loudness).


## [0.102.0] — 2026-09-22

## [0.101.0] - 2026-09-23

### Added
- `multicam` — two-camera angle switching across an aligned pair: `--at t1,t2,...` flips angle per cut (trim/atrim + concat chain; pair with `align` when takes aren't synced).
- `insert` — splice a whole clip into the middle of a video at `--at T` (scaled/letterboxed to the base size).
- `split --subs in.srt` — write a per-part `.srt` next to each split file with cues re-timed and clamped to the part window.


## [0.101.0] — 2026-09-22

## [0.100.0] - 2026-09-23

### Added
- `align` — auto-sync a second recording to a reference via audio cross-correlation (pure-Rust radix-2 FFT over 16 kHz PCM, ±`--max-lag` window, reports `offset_ms`); shifts the target's audio onto the reference timeline.
- `scroll` — rolling end credits: text/file renders once and rises bottom→top across `--dur` (`--at`, `--size`, `--color`, `--font`).
- `countdown --text` — static label above the digits for the whole count window.


## [0.100.0] — 2026-09-22


## [0.98.0] - 2026-09-23

### Added
- `conform --crf` — x264 quality knob (0–51, default 18) for delivery exports.
- `grid --gap` — uniform pixel border around every tile.
- `chapter --import` — read chapter marks from a text file (`TIME|TITLE` or `TIME,TITLE` lines, `#` comments skipped).


## [0.98.0] — 2026-09-22


## [0.96.0] — 2026-09-22

### Added

- `eq --band FREQ:GAIN[:WIDTH_OCT]` — repeatable parametric bands on top of the shelf/curve (e.g. `--band 800:-3 --band 5200:2:0.7`); validation rejects out-of-range freq/gain/width.
- `transcode --preset prores` — ProRes 422 HQ (`prores_ks -profile:v 3`, `yuv422p10le`) + `pcm_s16le` in `.mov`, the FCP/Premiere edit-delivery format.
- `chapter --export` — write the resolved marks as an `.ffmetadata` text file at `-o` instead of embedding (hand chapters to an editor or DAW).


## [0.95.0] — 2026-09-22

### Added

- `tempo --at/--dur` — pitch-preserving retempo inside a window only (head/mid/tail `atrim`+`concat`); the rest plays at 1x. Audio-only inputs, as before.
- `broll --position` + `--scale` + `--margin` — PiP mode: the insert scales to a fraction of the frame (default 0.30) and pins to any corner/edge instead of full-screen cutaway. Same `--at`/`--duration`, `--fade`, `--audio` handling.
- `subs --burn --safe` — raises `MarginV` to clear the bottom 20% (or top 15% with `--top`) so burned captions sit above TikTok/Reels UI chrome.


## [0.94.0] — 2026-09-22

### Added

- `extract --gif --loop N` — GIF repeat count passthrough: `-1` plays once, `0`/unset loops forever, `N` loops N times.
- `meme --position top|center|bottom` — move the caption block: `center` stacks both texts mid-screen, `bottom` parks the pair low.
- `solid --text` / `--text-color` / `--font` — centered card text on the generated clip (end-cards, section titles) — one call instead of solid + title.


## [0.93.0] — 2026-09-22

### Added

- `fit --position` — anchor the picture inside the padded frame: top/bottom/left/right/corners, works for `--fit pad` and `--fit blur` (e.g. a bottom bar reserved for captions).
- `mute --at/--dur` — silence only inside a window (`volume=0` gate) instead of dropping the whole track; the rest of the audio and the video pass through copied.
- `boomerang --at/--dur` — forward-backward bounce only the chosen window; head and tail play straight (`--times` still loops the window).


## [0.92.0] — 2026-09-22

### Added

- `audiogram --subs` — burn an .srt's cues along the audiogram bottom strip (≤60 raster PNG overlays, same font pipeline as `--text`).
- `caption --fade` — soft caption in/out: each PNG loops as a 30 fps input with alpha `fade` in/out inside its window.
- `subs --rate` — rescale every cue timestamp by a factor (25↔23.976 fps drift, e.g. `--rate 0.959`).


## [0.91.0] — 2026-09-22

### Added

- `timer --down` — count down to the window end instead of up from `--at`.
- `broll --volume` — linear gain (0..=4) on the insert audio (needs `--audio`).
- `mix --loop` — `-stream_loop -1` on the B input so a short bed repeats under A.


## [0.90.0] — 2026-09-22

### Added

- `loudnorm --target` — platform loudness presets (spotify/youtube −14 LUFS, podcast −16, broadcast −23/LRA 7); explicit `--i`/`--tp`/`--lra` still win.
- `stabilize --edge` — deshake edge fill (blank|original|clamped|mirror).
- `extract --gif --bounce` — palindrome GIF: forward frames + reversed (`split→reverse→concat` before `paletteuse`).


## [0.89.0] — 2026-09-22

### Added

- `gate --at/--dur` — windowed noise gate (same dry/wet graph as denoise/leveler/pitch/dehum/vocal).
- `mix --at/--dur` — the B track enters only inside the window (`volume=between(t,…)` gate on the B feed).
- `waveform --at/--dur` — render just a slice: crops the rendered wave to the window then stretches to `--size`.
- `spectrogram --at/--dur` — render just a slice (`atrim` before `showspectrumpic`).

## [0.88.0] — 2026-09-22

### Added

- `pitch --at/--dur` — windowed pitch shift.
- `dehum --at/--dur` — windowed mains-hum notches.
- `vocal --at/--dur` — windowed karaoke/isolate.


## [0.87.0] — 2026-09-22

### Added

- `timer --box-color C` / `caption --box-color C` — card behind the clock or captions.
- `replace --loop` — loop a short replacement track to fill the video.
- Fix: `--format ms` position math now counts the centisecond field.


## [0.86.0] — 2026-09-22

### Added

- `caption --karaoke` — word-by-word caption reveal inside each cue.
- `denoise --at/--dur` — windowed noise cleanup.
- `leveler --at/--dur` — windowed compression.


## [0.85.0] — 2026-09-22

### Added

- `freeze --reverse SEC` — rewinds the SEC before the hold, then freezes.
- `speed --ramp FROM,TO` — linear speed ramp (whole clip or --at/--dur window).
- `subs --merge FILE` — merge two .srt files into one, cues sorted by start.


## [0.84.0] — 2026-09-22

### Added

- `freeze --ease SEC` — last SEC before the hold plays at half-speed (swoop-in).
- `sheet --time` — timestamp label under every tile.
- `meme --at/--dur` — time-windowed meme captions.


## [0.83.0] — 2026-09-22

### Added

- `extract --gif --at --dur --fps` — 2-pass palette GIF clip straight out of a video.
- `cover --blur` — ambient blurred pad behind the 9:16 cover still.
- `audiogram --progress` — moving progress bar along the bottom edge.


## [0.82.0] — 2026-09-22

### Added

- `title --box COLOR` — filled backplate behind title text (lower-third legibility).
- `censor --strength N` — mosaic block px / blur sigma.
- `chapter --auto MIN_GAP` — auto chapter marks after silences (podcast segments).


## [0.81.0] — 2026-09-22

### Added

- `deinterlace --parity auto|tff|bff` — field-order override for DV/HDV/archival sources.
- `transcode --copy-audio` — keep the original audio bitstream while re-encoding video.
- `meme --outline N` — classic white-on-black-outline meme text.


## [0.80.0] — 2026-09-22

### Added

- Shared `src/color.rs`: every `--color`-family flag now accepts names (`red`, `black`, `gold`, …) as well as `#RGB`/hex — fixes the `0xred` mangling in `solid`/`fit`/gradient paths.
- `hls --single` — byte-range single-`.ts` package (one file to upload).
- `hls --copy` — stream-copy repack (instant when input is already h264/aac).


## [0.79.0] — 2026-09-22

### Added

- `timer --format ms` — mm:ss.cc centisecond timer field for sports/review overlays.
- `autocrop --buffer N` — expand the detected crop box by N px per side, clamped to frame.
- `rough --merge N` — merge keep ranges separated by less than N seconds.


Skill 与 CLI 共用一个 SemVer（`Cargo.toml` + `SKILL.md` 的 `version:`）。格式遵循 [Keep a Changelog](https://keepachangelog.com/)。

## [0.78.0] - 2026-09-22

- `grid --labels "a,b,c"` — raster caption PNGs overlaid at the bottom of each tile (drawtext is absent on some ffmpeg builds, so labels share the raster pipeline).
- `delogo --soft` — generates a PNG mask and uses `removelogo` interpolation instead of the hard delogo box.
- `solid --gradient RRGGBB:RRGGBB` — animated gradient card via the `gradients` lavfi source.

## [0.77.0] - 2026-09-22

- `subs --mux file.srt --lang spa` — muxes a subtitle file in as a selectable stream (mov_text in mp4, srt in mkv) with a language tag; videos stay stream-copied.
- `caption --outline RRGGBB` — strokes each glyph (`render_caption_outlined` in raster) for readability on busy frames.
- `audiogram --font` — custom ttf/otf for the `--text` title card.

## [0.76.0] - 2026-09-22

- `reverb --at/--dur`, `eq --at/--dur` — windowed audio FX via the new shared `engine::audio_window` (dry-duck + wet-delay + amix), extracted from `fx`.
- `loop --from/--to` — loops only the middle section and keeps head/tail once (three concat arms; reencodes since it can't stream-copy).
- Fix: `loop=...:size=0` is a silent no-op — the filter needs an explicit frame/sample buffer. `boomerang --times` was silently emitting one cycle; buffer sizes now derive from probe fps/sample_rate.

## [0.75.0] - 2026-09-22

- `fx --kind` +`echo` (aecho), `lofi` (acrusher + lowpass), `radio` (telephone bandpass + compressor).
- `fx --at/--dur` reworked: ffmpeg 4.4 gives none of the FX filters timeline support, so the window is now a split → duck-dry / delay-wet → amix graph (works for every kind, chorus included).
- `transcode --preset hevc` — libx265 at crf 28 with the `hvc1` tag so QuickTime/Safari play it.
- `gate --preset voice|podcast|studio` — tuned threshold/ratio/attack/release curves.


## [0.74.0] - 2026-09-22

- `fx` — audio FX rack: `--kind tremolo|vibrato|flanger|phaser|chorus` with `--strength` scaling depth and `--at/--dur` enable windows (chorus lacks timeline support and refuses `--at` cleanly).
- `boomerang --times N` — `loop`/`aloop` the fwd+rev cycle N times (duration ×N).

## [0.73.0] - 2026-09-22

- `overlay --angle DEG` — `format=rgba,rotate=a=rad:c=none` pre-chain ahead of any fade/opacity chain (diagonal watermarks).
- `waveform --scale lin|log|sqrt|cbrt` — showwavespic amplitude scale.
- `split --min-silence N` — silence-gap minimum for `--silence` mode (default 0.4s); standalone refuses.

## [0.72.0] - 2026-09-22

- `music --at/--dur` — delayed/trimmed bed window: `atrim` + `adelay=at*1000` before the ducking mix; fades stay relative to the bed.
- `channel --mode invert --side left|right|both` — `aeval='-val(0)|val(1)':c=stereo` polarity flip (pan rejects `-c0`; aeval needs `aformat` to restore the layout for aac).
- `sheet --pad/--margin` — tile spacing/outer margin in px.

## [0.71.0] - 2026-09-22

- `countdown --beep` — one `aevalsrc` source: `sin(2*PI*880*t)*lt(mod(t-at,each),0.12)` gated tick tone, amix'd with input audio (commas must stay inside the quoted expr — aevalsrc splits channels on them).
- `leveler --preset voice|podcast|master` — one-click acompressor curves.
- `spectrogram --color NAME` — showspectrumpic color scheme (validated against the ffmpeg list).

## [0.70.0] - 2026-09-22

- `zoom --out` — zoompan `z='max(factor-on*step,1.0)'`: starts at --factor, settles to 1x (reveal shot).
- `title --shadow N` — raster: blurred darkened copy of the title card offset down-right under the card (soft drop shadow).
- `extract --width` — `-vf scale=w:-2` on the still path (exact-width frames).

## [0.69.0] - 2026-09-22

- `subs --burn --font NAME` — font family in the libass force_style (brand captions).
- `progress --at/--dur` — windowed progress bar via overlay `enable=`.
- `audiogram --position top|center|bottom` — wave band y factor (0.18/0.50/0.62 of frame height).

## [0.68.0] - 2026-09-22

- `vignette --at/--dur` — windowed vignette (`enable='between(...)'` on the angle filter).
- `grade --at/--dur` — windowed grade: `:enable='...'` appended to every filter in the chain (eq/curves/hue/lut3d/colortemperature/colorbalance/noise all accept timeline on ffmpeg 4.4+).
- `broll --audio` — mixes the insert's audio (`atrim`+`adelay`+`amix`) into its window so b-roll sound is heard; requires a video insert with an audio stream.

## [0.67.0] - 2026-09-22

- `bw --at/--dur` — windowed desaturation (`hue=s=0:enable='between(...)'`).
- `sharpen --at/--dur` — windowed `unsharp` (same enable pattern; `--dur` alone refuses).
- `meta --clear` — `-map_metadata -1`, strips every container tag before publishing; refuses to combine with tag flags.

## [0.66.0] - 2026-09-22

- `invert --at/--dur` — windowed negation via `negate=enable='between(...)'` (flashback accents).
- `blur --at/--dur` — windowed `gblur` (same enable pattern; `--dur` alone refuses).
- `title --position` gains corners: `top-left|top-right|bottom-left|bottom-right` (6% margins).

## [0.65.0] - 2026-09-22

- `replace --fade N` — `afade` in/out on the swapped audio (fade-out pinned to the video's end; all three mix/duck/plain paths).
- `audiogram --text` — raster title composited near the top (`render_caption` PNG → second `overlay` stage, `shortest=1` on the looped input).
- `thumb --width` — scale the grabbed still (`scale=W:-2`).

## [0.64.0] — 2026-09-22

- `broll --fade N` — alpha fade in/out on the insert (`format=rgba` + `fade alpha=1` on the shifted B chain; stays rgba through `overlay`).
- `frames --at t1,t2,...` — grab stills at listed timestamps (N `-ss` seek jobs → `stem_NNN.ext`).
- `audiogram --size WxH` — canvas for YouTube (1920x1080) or square (1080x1080); waveform band scales with it.

## [0.63.0] - 2026-09-22

- `split --silence dB` — cuts at silence midpoints (`silencedetect` → `-f segment`; podcast → episode segments).
- `music --fade N` — `afade` in/out on the bed (`-stream_loop` is pinned to the talk's length for the fade-out point).
- `eq --preset voice|podcast|bright|bass` — one-shot curves; each band still overridable (preset only fills bands left at 0).

## [0.62.0] - 2026-09-22

- `subs --burn` style overrides: `--size` (FontSize), `--color` (RRGGBB → ASS `&H00BBGGRR`), `--top` (Alignment 8 vs 2).
- `audiogram --bg` — backdrop color when no `--image` cover (name or hex, default `0x101418`).
- `overlay --opacity` now works on `--image` stills too — `format=rgba,colorchannelmixer=aa=N` pre-chain for subtle watermarks (was `--mode`-only).

## [0.61.0] - 2026-09-22

- `cut --drop "a-b,c-d"` — remove middle sections, keep the rest joined (complement of `--ranges`; same N-trim + concat path).
- `title --outline RRGGBB` — stroke around every glyph (`render_title_outlined`: 16-offset ring blits under the fill) for readable text on busy frames.
- `fit --color RRGGBB` — pad bars in a brand color instead of black (`pad` filter color arg).

## [0.60.0] - 2026-09-22

- `cut --ranges "a-b,c-d"` — keep several ranges joined into one file (N `trim`/`atrim` + `concat`; always re-encodes for frame-exact, aligned pts).
- `solid` — new verb: standalone solid-color clip (`lavfi color`), `--color`/`--size`/`--dur`, optional silent stereo (`--audio=false`). For intro cards, lyric backplates, b-roll spacers.
- `volume --limit dBTP` — `alimiter` brickwall after the gain (linear ceiling from dBTP, `level=false`).

## [0.59.0] - 2026-09-22

- `overlay --fade` — alpha-fades any overlay (`--image` still or `--video`) in/out over its window; stills are `-loop 1` + `shortest=1` so the composite ends with the main stream.
- `subs --shift ±N` — retimes every cue of an .srt (clamped at 0), e.g. "字幕慢了 2 秒" → `subs in.srt --shift 2 -o out.srt`.
- `meta --album/--genre/--date/--track` — podcast & music library tags alongside title/artist/comment.

## [0.58.0] - 2026-09-22

- `title --fade` — fades the title in/out over N s (looped still + `fade alpha=1`; clamps to half the window).
- `grade --hue` — `hue=h=N` rotate (-180..180) for white-balance rescue or color FX.
- `silence --end` — append the `--dur` silence to the tail without doing the math.

## [0.57.0] - 2026-09-22

- `thumb` — single-frame cover grab: `--at 2.5` (fast `-ss` seek) or `--frame N` (exact `select`); writes jpg/png/webp.
- `subs --burn <file.srt|ass>` — libass `subtitles` burn-in with a creator-safe force_style (white text, outline, bottom margin). Extract mode unchanged.
- `split --parts N` — equal-length N-way split (derives `--every` from duration).

## [0.56.0] - 2026-09-22

- `sync` — fix constant A/V offset: `--ms +N` pads the audio start (`adelay`), `--ms -N` trims it (`atrim`); video stream-copied.
- `crop --anchor` — `--aspect` reframes at `center|top|bottom|left|right` instead of always centering (keep faces in 9:16 crops of landscape masters).
- `art` — attach a cover image (`--image`): `mjpeg` art stream + `id3v2`/attached_pic for mp3/m4a/mp4/mov/mkv; refuses containers that can't hold art.

## [0.55.0] - 2026-09-22

- `qa` — measure encode quality loss: `psnr`/`ssim` via `scale2ref` + metric filter, parsed into `extra` (psnr dB, ssim 0..1).
- `conform` — normalize a clip to a shared spec for concat/assembly: `--size WxH` (even-dim fit), `--fps`, `--lufs` one-pass loudnorm (resampled back to 48k after — loudnorm upsamples internally).
- `overlay --mode` — full-frame blend composite (`screen`/`addition`/`multiply`/`lighten`/`darken`/`overlay`/`difference`) with `--opacity` — light leaks, particles, film textures; refuses `--x/--y/--scale` since blending is full-frame.

## [0.54.0] - 2026-09-22

- `timer` — running MM:SS (H:MM:SS past the hour) counter burned into a corner, no drawtext needed: a 60-cell digit sprite + `crop x='mod(floor(t),60)*cell'` driven by `-loop 1` inputs; `--position`, `--at`, `--dur`, `--size`, `--color`, `--font`.
- `mute` — drop the audio stream, everything else stream-copied (`-map 0 -map -0:a -c copy`).
- `hls` — package for web embeds: `dir/index.m3u8` + `seg_*.ts`, `--seg` segment seconds, h264+aac transcode for player compat.

## [0.53.0] - 2026-09-22

- `mix` — sum two audio sources at full level (`amix normalize=0` + `aresample`), `--vol-a`/`--vol-b` linear trims, `--longest` to run to the longer input; keeps input A's video as-is.
- `caption --position top` — burn captions in the upper safe zone (top ~15% social / 10% off).
- `grade --gamma` — mid-tone `eq=gamma` slider (verified on ffmpeg 4.4).

## [0.52.0] - 2026-09-22

- `split --size 9MB` — aim chunks at a byte cap (Discord/WhatsApp uploads) by deriving the even grid from input size; also fixes bare-stem outputs (`-o part.mp4`) failing in `split` and `frames` with a bogus ENOENT after the parts were already written.
- `countdown` — 3-2-1(-GO!) intro overlay (`--from`, `--each`, `--at`, `--go`, styled via `--size`/`--color`/`--font`); rendered by the title rasterizer so no drawtext fontfile wrangling.
- `invert` — negative colors via `negate` (clone-VFX base, flash frames).

## [0.51.0] — 2026-09-22

- `ffkit crossfade`: `acrossfade` blend of two audio files (`--second`, `--dur`)
- `ffkit strip`: drop all metadata + chapters, lossless stream copy
- `ffkit frames`: still dump every `--every` seconds (`stem_%03d.png`, `--width`)

## [0.50.0] — 2026-09-22

## [0.49.0] — 2026-09-22

- `ffkit vocal`: `--mode karaoke` removes center-panned vocals; `isolate` keeps only the center (stereo sources)
- `ffkit remux`: container swap without re-encoding (`-c copy`, faststart on mp4/mov)
- `ffkit meme`: `--top`/`--bottom` caption text burned onto video

## [0.48.0] — 2026-09-22

- `ffkit silence`: insert `--dur` seconds of quiet at `--at` in audio files (video holds stay with `freeze`)
- `grade --preset cinematic|vivid|vintage|soft`: one-click looks applied before the sliders
- `transcode --fps`: retimes h264/webm video too, not only GIF

## [0.47.0] — 2026-09-22

- `ffkit tempo`: audio speed `--factor` 0.5–8, pitch preserved (`atempo` chain; refuses video — use `speed`)
- `ffkit leveler`: voice dynamic-range compressor (`acompressor`)
- `ffkit gate`: noise gate below `--threshold` dB (`agate`)

## [0.46.0] — 2026-09-22

- `ffkit waveform`: audio waveform → PNG (`--size`, `--color`) via `showwavespic`
- `ffkit spectrogram`: audio spectrogram → PNG (`--size`) via `showspectrumpic`
- `ffkit dehum`: notch mains hum `--mains 50|60` + `--harmonics` (`equalizer` Q=12 chain)

## [0.45.0] — 2026-09-22

- `ffkit vdenoise`: spatial video denoise for grainy footage (`--strength` 0.5–30) via `nlmeans`
- `ffkit crop`: crop a `--region x:y:w:h` box or `--aspect W:H` center-reframe
- `title --position bottom`: lower-third placement (joins `top`/`center`)

## [0.44.0] — 2026-09-22

- `ffkit bleep`: censor-tone over a window (`--at`/`--dur`, `--freq`, `--level`) — silences the source and mixes a delayed `sine`
- `censor --at`/`--dur`: mosaic/blur only inside a window
- `grade --warm -1..1`: white-balance warmth via `colortemperature`

## [0.43.0] — 2026-09-22

- `ffkit reverb`: room/hall/cave ambience on a voice (`--size`, `--wet`) via `aecho`
- `audiogram --mode`/`--color`: waveform style (point/line/p2p/cline) and colour
- `delogo --at`/`--dur`: blur the logo box only inside a window
- `meta --rotate 0/90/180/270`: fix the display-rotation flag losslessly (uses `-display_rotation` on ffmpeg ≥7, `rotate` metadata below)

## [0.42.0] — 2026-09-22

- `overlay --at`/`--dur`: windowed logo/overlay (single and `--tile` paths)
- `caption --color`/`--size`: styled caption raster (same knobs as `title`)
- `ffkit subs`: extract embedded subtitle tracks to `.srt`/`.vtt`/`.ass`


## [0.41.0] — 2026-09-22

- `title --size`/`--color`: text styling on the burned-in title
- `ffkit meta`: write `title`/`artist`/`comment` container tags (lossless copy)
- `broll --still --motion kenburns`: animated push on an image cutaway


## [0.40.0] — 2026-09-22

- `ffkit rotate`: `--deg 90/180/270` or `--flip h|v` for mis-oriented phone clips
- `ffkit delogo`: blend out a burned-in logo/watermark box (`--x --y --w --h`)
- `speed --interp`: `minterpolate` blend upsampling for smooth slow-mo (factor < 1)


## [0.39.0] — 2026-09-22

- `ffkit eq`: `--bass`/`--treble`/`--presence` dB shelving on audio
- `zoom --motion kenburns`: animated zoompan push to `--factor`, whole clip or inside `--at/--dur`
- `broll --still`: cut away to a still image (looped over the window)


## [0.38.0] — 2026-09-22

- `ffkit channel`: `--mode dualmono` (one-ear fix), `mono` (fold-down), `swap` (L/R flip)
- `loop --until SEC`: repeat+trim to a target length
- `title --tile N`: text tiled diagonally as a draft watermark
- fix: `overlay --tile` splits the overlay pad for ffmpeg ≤5 (labels consumed once)


## [0.37.0] — 2026-09-22

- `overlay --tile N`: tiled semi-transparent watermark pass (draft protection)
- `caption --shift SEC`: nudge every cue (negative pulls earlier)
- `transcode --preset gif --fps/--width`: gif tuning flags


## [0.36.0] — 2026-09-22

- `split --scenes T`: auto-detect shot changes and cut there (select scene score + segment muxer)
- `ffkit cutsil`: `silenceremove` head+tail dead air (audio-only; video → jumpcut)
- `grid --audio N`: keep one input's audio instead of the amix


## [0.35.0] — 2026-09-22

- `replace --duck`: sidechain-duck the original track under the replacement audio
- `ffkit pitch`: `--semitones` shift with preserved duration (asetrate+atempo)
- `grade --grain`: film-grain `noise=alls:allf=t+u` pass


## [0.34.0] — 2026-09-22

- `ffkit autocrop`: cropdetect scan → crop out letterbox/pillarbox (refuses when nothing detected)
- `ffkit sheet`: `--cols`/`--rows`/`--tile` contact-sheet PNG
- `title --at S`: title/lower-third at any offset, not just t=0


## [0.33.0] — 2026-09-22

- `ffkit boomerang`: forward + reversed replay loop
- `ffkit chapter`: embed `--at T|TITLE` markers via ffmetadata + `-c copy` (lossless)
- `zoom --at S [--dur D]`: punch-zoom only inside a window
- `key --despill`: `despill=type=green` cleanup pass on keyed edges


## [0.32.0] — 2026-09-22

- `ffkit freeze`: mid-clip hold (`--at T --dur D`) or outro hold (`--end D`) via `tpad` clone; the held window's audio is silence
- `ffkit censor`: mosaic/blur a region `--region x:y:w:h` (`--mode pixel|blur`)
- `speed --at S [--dur D]`: speed-ramp only one window (3-segment trim/concat)


## [0.31.0] — 2026-09-22

- `ffkit grid`: N inputs into a `--layout CxR` tile wall via `xstack`; audio mixes when every input has it
- `ffkit progress`: bottom/top fill bar over the duration (`--color`, `--height`, `--edge`)
- `volume --at S [--dur D]`: apply `--db` gain only inside a window (mute a moment)


## [0.30.0] — 2026-09-22

## [0.29.0] — 2026-09-22

## [0.28.0] — 2026-09-22

- `caption --chunk N`: split each cue into ≤N-word groups sharing its span — the chunked-caption look without word timing (burn mode)
- `slideshow --transition <name>`: xfade flavor between stills (`wipe*`, `slide*`, `dissolve`, `radial`, `circleopen`); `--motion kenburns`: zoompan push-in/pull-out per still
- `replace --mix G`: keep the original track under the new one at linear gain G

## [0.27.0] — 2026-09-22

- `ffkit replace`: swap a video's audio track (`--audio`, `--audio-offset`); video stream-copied, new audio padded/trimmed to length
- `ffkit slideshow`: still images → montage (`--per`, `--fade` xfade chain, `--audio` bed, `--size` canvas); silent track written when no bed
- `grade --lut file.cube`: apply a 3D LUT after the slider correction

## [0.26.0] — 2026-09-22

- `ffkit denoise`: `highpass` + voice denoise (`afwtdn` where ffmpeg ≥5.1 ships it, else `afftdn` with raised floor); `--video` adds `hqdn3d` degrain
- `ffkit compress --size 10MB`: two-pass bitrate budget that lands under a platform cap (audio-only inputs single-pass `-b:a`)
- `fit --fit blur`: blurred-pillarbox fill for repurpose; `broll --fit blur` gets the same mode
- `ffkit audiogram`: podcast audio → 1080×1920 `showwaves` video over a cover still (`--image`) or flat colour
- Creator-gap research refreshed in `docs/creator-needs.md` (size caps, podcast→clips, denoise, blur fill)


## [0.25.1] — 2026-09-16

- `music --duck` pins both legs to `aformat=dbl` so Ubuntu/apt ffmpeg can negotiate `sidechaincompress`

## [0.25.0] — 2026-09-15

- `ffkit rough`: list speech islands (audio-only detect), then assemble only those windows — not a full-timeline re-encode

## [0.24.1] — 2026-09-15

- `broll` delays the insert with `setpts` so `--at` plays B-roll from its first frame (short clips no longer freeze)

## [0.24.0] — 2026-09-15

- `ffkit broll --insert --at --duration`: cut away to B-roll; A-roll audio and duration stay

## [0.23.0] — 2026-09-15

- `caption --mode burn` defaults to `--safe social` (above the bottom 20% of the frame)
- Remaining 2026 creator-gap research in `docs/creator-needs.md`

## [0.22.1] — 2026-09-15

- README.md is English by default; Chinese lives in README.zh.md (edit both together)

## [0.22.0] — 2026-09-15

- GitHub Release zips (binary + skill + `install.sh`) on every versioned merge to `main`
- README install path is the Release zip; Source code zip is not the bundle

## [0.21.0] — 2026-09-15

- Pipeline plans take `input` (`$src`), previous output (`$in`), step `label`, and `expect` (sets `verified`)
- Recipe schemes in `references/recipes.md` to adapt after the user agrees

## [0.20.0] — 2026-09-15

- Skill loop is chat → proposed scheme → hands finish the original task (not a verb menu)
- `ffkit pipeline plan.json` runs that scheme (stops on first failure)

## [0.19.0] — 2026-09-15

- `ffkit blur --sigma`: gaussian blur (`gblur`)

## [0.18.0] — 2026-09-15

- `ffkit volume --db`: simple gain (not LUFS)

## [0.17.0] — 2026-09-15

- `ffkit bw`: strip color (`hue=s=0`)

## [0.16.0] — 2026-09-15

- `ffkit vignette`: darken corners for a Reels look

## [0.15.0] — 2026-09-15

- `ffkit sharpen`: unsharp after social re-encode

## [0.14.0] — 2026-09-15

- `ffkit zoom --factor`: center punch-in (talking-head crop)

## [0.13.0] — 2026-09-15

- `ffkit grade`: Reels-style contrast/saturation/brightness pop (`eq`)

## [0.12.0] — 2026-09-15

- `ffkit reverse`: play picture and sound backwards

## [0.11.0] — 2026-09-15

- `ffkit stabilize`: deshake handheld footage

## [0.10.0] — 2026-09-15

- `ffkit loop --times`: lossless concat-repeat for Shorts replay length

## [0.9.0] — 2026-09-15

- `ffkit title --text`: first-second hook card via raster overlay (no libass)

## [0.8.0] — 2026-09-15

- `ffkit fade --in/--out`: video + audio fade

## [0.7.0] — 2026-09-15

- `ffkit cover`: 1080×1920 still for Reels / TikTok / Shorts thumbnails

## [0.6.0] — 2026-09-15

- `ffkit jumpcut`: drop internal silence for talking-head jump cuts

## [0.5.0] — 2026-09-15

- `ffkit music --track`: loop a bed under speech with sidechain ducking

## [0.4.0] — 2026-09-15

- `ffkit speed --factor`: setpts + chained atempo (0.25×–8×, pitch kept)

## [0.3.0] — 2026-09-15

- `caption --mode burn` rasterizes SRT and overlays PNGs; no libass/`drawtext` required

## [0.2.0] — 2026-09-15

- `ffkit deliver`: one-shot 9:16 social export (1080×1920, 30 fps, −14 LUFS, H.264+AAC+faststart)
- Cited creator-needs research in `docs/creator-needs.md`

## [0.1.0] — 2026-09-15

- 首个可安装 skill：probe / cut / concat / fit / extract / overlay / caption / loudnorm / transcode / look / batch / graph / ffmpeg / doctor / install-skill
- `ffkit ffmpeg` 必须 `--because`
- `look --at` 可重复；caption burn 依赖 libass
- `ffkit version` 报告二进制、嵌入 skill、已安装拷贝
